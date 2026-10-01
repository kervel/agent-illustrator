//! Macro expansion and selector resolution for motion statements.
//!
//! Runs once templates are resolved, so every element id is known. After this
//! pass every motion statement carries its resolved `targets`, macro calls are
//! gone, and each keyframe's flat `operations` list is regenerated from the
//! tree. A name that no longer exists is a compile error pointing at the
//! statement that uses it.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::layout::LayoutError;
use crate::parser::ast::*;

/// What the selector resolver needs to know about the document.
#[derive(Debug, Default)]
pub struct ElementIndex {
    /// Every element and connection id.
    pub ids: HashSet<String>,
    /// Named children of each container, in declaration order.
    pub children: HashMap<String, Vec<String>>,
    /// Named top-level nodes, in declaration order.
    pub top: Vec<String>,
    /// Elements per class, in declaration order.
    pub by_class: HashMap<String, Vec<String>>,
    /// Ids that are containers (layouts, groups, template instances).
    pub groups: HashSet<String>,
    /// Ids that can be drawn: connections and path shapes.
    pub drawables: HashSet<String>,
    /// How to write each id: the dotted path through its components
    /// (`anna_hist_d4` is `anna.hist.d4`).
    pub display: HashMap<String, String>,
    /// A part folded into its instance -> the instance (`f_val` -> `f`).
    pub aliases: HashMap<String, String>,
}

impl ElementIndex {
    pub fn build(doc: &Document) -> Self {
        let mut idx = ElementIndex::default();
        idx.aliases = doc.aliases.iter().cloned().collect();
        for stmt in &doc.statements {
            if let Some(id) = idx.visit(&stmt.node) {
                idx.top.push(id);
            }
        }
        fn dotted(stmts: &[Spanned<Statement>], scope: Option<(&str, &str)>, out: &mut HashMap<String, String>) {
            for st in stmts {
                // Named connections in a component: `acme.w_front`.
                if let (Statement::Connection(conns), Some((prefix, path))) = (&st.node, scope) {
                    for c in conns {
                        if let Some(n) = &c.name {
                            if let Some(local) = n.node.0.strip_prefix(&format!("{}_", prefix)) {
                                out.insert(n.node.0.clone(), format!("{}.{}", path, local));
                            }
                        }
                    }
                    continue;
                }
                let (id, kids, instance): (Option<String>, &[Spanned<Statement>], bool) = match &st.node {
                    Statement::Group(g) => (g.name.as_ref().map(|n| n.node.0.clone()), &g.children, g.is_template_instance),
                    Statement::Layout(l) => (l.name.as_ref().map(|n| n.node.0.clone()), &l.children, false),
                    Statement::Shape(s) => (
                        s.name.as_ref().map(|n| n.node.0.clone()).or_else(|| match &s.shape_type.node {
                            ShapeType::Path(p) => p.name.as_ref().map(|n| n.node.0.clone()),
                            _ => None,
                        }),
                        &[],
                        false,
                    ),
                    _ => (None, &[], false),
                };
                let shown = match (&id, scope) {
                    (Some(i), Some((prefix, path))) => match i.strip_prefix(&format!("{}_", prefix)) {
                        Some(local) => Some(format!("{}.{}", path, local)),
                        None => Some(i.clone()),
                    },
                    (Some(i), None) => Some(i.clone()),
                    _ => None,
                };
                // An artwork's own drawing (`pg__art`, `lid__self`) is shown
                // as the part it draws.
                let shown = shown.map(|d| {
                    d.strip_suffix(".__art")
                        .or_else(|| d.strip_suffix(".__self"))
                        .or_else(|| d.strip_suffix("._art"))
                        .or_else(|| d.strip_suffix("._self"))
                        .or_else(|| d.strip_suffix("__art"))
                        .or_else(|| d.strip_suffix("__self"))
                        .map(|x| x.trim_end_matches('.').to_string())
                        .unwrap_or(d)
                });
                if let (Some(i), Some(d)) = (&id, &shown) {
                    out.insert(i.clone(), d.clone());
                }
                let inner = if instance {
                    id.as_deref().zip(shown.as_deref())
                } else {
                    scope
                };
                dotted(kids, inner, out);
            }
        }
        dotted(&doc.statements, None, &mut idx.display);
        // `x [caption: "..."]` made `x_caption`: written `x.caption`.
        let caps: Vec<(String, String)> = idx
            .ids
            .iter()
            .filter_map(|id| {
                let subject = id.strip_suffix("_caption")?;
                idx.ids.contains(subject).then(|| (id.clone(), format!("{}.caption", idx.show(subject))))
            })
            .collect();
        idx.display.extend(caps);
        idx
    }

    /// An id as the author writes it.
    pub fn show(&self, id: &str) -> String {
        self.display.get(id).cloned().unwrap_or_else(|| id.to_string())
    }

    fn suggest(&self, name: &str) -> Vec<String> {
        let dotted: HashSet<String> = self.ids.iter().map(|i| self.show(i)).collect();
        let mut out = crate::layout::find_similar(&dotted, name, 2);
        if out.is_empty() {
            out = crate::layout::find_similar(&self.ids, &name.replace('.', "_"), 2)
                .into_iter()
                .map(|i| self.show(&i))
                .collect();
        }
        out
    }

    fn class_of(mods: &[Spanned<StyleModifier>]) -> Vec<String> {
        mods.iter()
            .filter(|m| matches!(m.node.key.node, StyleKey::Class))
            .filter_map(|m| match &m.node.value.node {
                StyleValue::Identifier(i) => Some(i.0.clone()),
                StyleValue::String(s) | StyleValue::Keyword(s) => Some(s.clone()),
                _ => None,
            })
            .flat_map(|s| s.split_whitespace().map(str::to_string).collect::<Vec<_>>())
            .collect()
    }

    fn add(&mut self, id: &str, mods: &[Spanned<StyleModifier>]) {
        self.ids.insert(id.to_string());
        for c in Self::class_of(mods) {
            self.by_class.entry(c).or_default().push(id.to_string());
        }
    }

    /// Record a statement; returns its id if it has one.
    fn visit(&mut self, stmt: &Statement) -> Option<String> {
        match stmt {
            Statement::Shape(s) => {
                let name = s.name.as_ref().map(|n| n.node.0.clone()).or_else(|| {
                    if let ShapeType::Path(p) = &s.shape_type.node {
                        p.name.as_ref().map(|n| n.node.0.clone())
                    } else {
                        None
                    }
                })?;
                self.add(&name, &s.modifiers);
                if matches!(s.shape_type.node, ShapeType::Path(_)) {
                    self.drawables.insert(name.clone());
                }
                Some(name)
            }
            Statement::Layout(l) => {
                let kids: Vec<String> =
                    l.children.iter().filter_map(|c| self.visit(&c.node)).collect();
                let name = l.name.as_ref()?.node.0.clone();
                self.add(&name, &l.modifiers);
                self.groups.insert(name.clone());
                self.children.insert(name.clone(), kids);
                Some(name)
            }
            Statement::Group(g) => {
                let kids: Vec<String> =
                    g.children.iter().filter_map(|c| self.visit(&c.node)).collect();
                let name = g.name.as_ref()?.node.0.clone();
                self.add(&name, &g.modifiers);
                self.groups.insert(name.clone());
                self.children.insert(name.clone(), kids);
                Some(name)
            }
            Statement::Label(inner) => self.visit(inner),
            Statement::Connection(conns) => {
                let mut last = None;
                for c in conns {
                    if let Some(n) = &c.name {
                        self.add(&n.node.0, &c.modifiers);
                        self.drawables.insert(n.node.0.clone());
                        last = Some(n.node.0.clone());
                    }
                }
                last
            }
            _ => None,
        }
    }

    /// `station.label` -> `station_label` (a template child), else as written.
    fn member(&self, dotted: &str) -> String {
        let joined = dotted.replace('.', "_");
        match self.aliases.get(&joined) {
            Some(inst) if !self.ids.contains(&joined) => inst.clone(),
            _ => joined,
        }
    }

    fn resolve(&self, sel: &Spanned<Selector>) -> Result<Vec<String>, LayoutError> {
        let missing = |name: &str| LayoutError::UndefinedIdentifier {
            name: name.to_string(),
            span: sel.span.clone(),
            suggestions: self.suggest(name),
        };
        match &sel.node {
            Selector::Name(n) => {
                let id = self.member(n);
                if self.ids.contains(&id) {
                    Ok(vec![id])
                } else {
                    Err(missing(n))
                }
            }
            Selector::Lines(c, a, b) => {
                let mut out = Vec::new();
                for i in (*a).min(*b)..=(*a).max(*b) {
                    let name = format!("{}.line{}", c, i);
                    let id = self.member(&name);
                    if !self.ids.contains(&id) {
                        return Err(missing(&name));
                    }
                    out.push(id);
                }
                Ok(out)
            }
            Selector::Children(n) => {
                let id = self.member(n);
                match self.children.get(&id) {
                    Some(kids) if !kids.is_empty() => Ok(kids.clone()),
                    Some(_) => Err(LayoutError::validation_error(&format!(
                        "`{}.*` selects nothing: '{}' has no named children",
                        n, id
                    ))),
                    None => Err(missing(n)),
                }
            }
            Selector::Class(c) => match self.by_class.get(c) {
                Some(v) if !v.is_empty() => Ok(v.clone()),
                _ => Err(LayoutError::validation_error(&format!(
                    "`.{}` selects nothing: no element has class '{}'",
                    c, c
                ))),
            },
            Selector::Many(v) => {
                let mut out = Vec::new();
                for x in v {
                    for id in self.resolve(&Spanned::new(x.clone(), sel.span.clone()))? {
                        if !out.contains(&id) {
                            out.push(id);
                        }
                    }
                }
                Ok(out)
            }
            Selector::AllExcept(except) => {
                for e in except {
                    if !self.ids.contains(e) {
                        return Err(missing(e));
                    }
                }
                let except: HashSet<&String> = except.iter().collect();
                let mut out = Vec::new();
                self.all_except(&self.top, &except, &mut out);
                Ok(out)
            }
        }
    }

    fn contains_any(&self, id: &str, except: &HashSet<&String>) -> bool {
        self.children
            .get(id)
            .is_some_and(|k| k.iter().any(|c| except.contains(c) || self.contains_any(c, except)))
    }

    fn all_except(&self, nodes: &[String], except: &HashSet<&String>, out: &mut Vec<String>) {
        for n in nodes {
            if except.contains(n) {
                continue;
            }
            if self.contains_any(n, except) {
                if let Some(kids) = self.children.get(n) {
                    self.all_except(kids, except, out);
                }
            } else {
                out.push(n.clone());
            }
        }
    }
}

/// Where imported macro files come from.
pub struct ImportContext<'a> {
    pub base_path: Option<&'a Path>,
}

/// Built-in macro libraries, importable as `import "ail:motion/<name>"`.
fn builtin_library(name: &str) -> Option<&'static str> {
    match name {
        "ail:motion/git" => Some(include_str!("lib/git.ail")),
        _ => None,
    }
}

fn collect_macros(
    doc: &Document,
    ctx: &ImportContext,
    macros: &mut BTreeMap<String, MotionMacroDecl>,
    seen: &mut HashSet<PathBuf>,
) -> Result<(), LayoutError> {
    for stmt in &doc.statements {
        match &stmt.node {
            Statement::MotionMacro(m) => {
                macros.insert(m.name.node.clone(), m.clone());
            }
            Statement::Import(path) => {
                let (source, key) = if let Some(lib) = builtin_library(&path.node) {
                    (lib.to_string(), PathBuf::from(&path.node))
                } else {
                    let p = match ctx.base_path {
                        Some(b) => b.join(&path.node),
                        None => PathBuf::from(&path.node),
                    };
                    let src = std::fs::read_to_string(&p).map_err(|e| {
                        LayoutError::validation_error(&format!(
                            "import \"{}\": cannot read {}: {}",
                            path.node,
                            p.display(),
                            e
                        ))
                    })?;
                    (src, p)
                };
                if !seen.insert(key) {
                    continue;
                }
                let imported = crate::parser::parse(&source).map_err(|errs| {
                    LayoutError::validation_error(&format!(
                        "import \"{}\": {}",
                        path.node,
                        errs.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("; ")
                    ))
                })?;
                collect_macros(&imported, ctx, macros, seen)?;
            }
            _ => {}
        }
    }
    Ok(())
}

/// Inline every `import "file.ail"`: the imported file's templates, motion
/// macros and motion defaults become part of this document (shapes and
/// keyframes in it are not). Runs before template resolution, so an imported
/// component is used like a local one.
pub fn inline_imports(doc: Document, ctx: &ImportContext) -> Result<Document, LayoutError> {
    fn inline(
        stmts: Vec<Spanned<Statement>>,
        ctx: &ImportContext,
        seen: &mut HashSet<PathBuf>,
        out: &mut Vec<Spanned<Statement>>,
        top: bool,
    ) -> Result<(), LayoutError> {
        for st in stmts {
            match &st.node {
                Statement::Import(path) => {
                    let (source, key) = if let Some(lib) = builtin_library(&path.node) {
                        (lib.to_string(), PathBuf::from(&path.node))
                    } else {
                        let p = match ctx.base_path {
                            Some(b) => b.join(&path.node),
                            None => PathBuf::from(&path.node),
                        };
                        let src = std::fs::read_to_string(&p).map_err(|e| {
                            LayoutError::validation_error(&format!(
                                "import \"{}\": cannot read {}: {}",
                                path.node,
                                p.display(),
                                e
                            ))
                        })?;
                        (src, p)
                    };
                    if !seen.insert(key) {
                        continue;
                    }
                    let imported = crate::parser::parse(&source).map_err(|errs| {
                        LayoutError::validation_error(&format!(
                            "in import \"{}\": {}",
                            path.node,
                            errs.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("; ")
                        ))
                    })?;
                    inline(imported.statements, ctx, seen, out, false)?;
                }
                Statement::TemplateDecl(_) | Statement::MotionMacro(_) | Statement::MotionDefaults(_) => {
                    out.push(st)
                }
                _ if top => out.push(st),
                _ => {}
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    let aliases = doc.aliases;
    inline(doc.statements, ctx, &mut HashSet::new(), &mut out, true)?;
    Ok(Document { statements: out, aliases })
}

/// Bindings of macro parameters to call arguments.
#[derive(Clone)]
enum Bound {
    Name(String),
    Number(f64),
    Str(String),
}

fn subst_name(name: &str, env: &HashMap<String, Bound>) -> String {
    let (head, rest) = match name.find('.') {
        Some(i) => (&name[..i], &name[i..]),
        None => (name, ""),
    };
    match env.get(head) {
        Some(Bound::Name(n)) => format!("{}{}", n, rest),
        _ => name.to_string(),
    }
}

fn subst_sel(sel: &Spanned<Selector>, env: &HashMap<String, Bound>) -> Spanned<Selector> {
    let node = match &sel.node {
        Selector::Name(n) => Selector::Name(subst_name(n, env)),
        Selector::Children(n) => Selector::Children(subst_name(n, env)),
        Selector::Class(c) => Selector::Class(c.clone()),
        Selector::AllExcept(v) => Selector::AllExcept(v.iter().map(|n| subst_name(n, env)).collect()),
        Selector::Lines(c, a, b) => Selector::Lines(subst_name(c, env), *a, *b),
        Selector::Many(v) => Selector::Many(
            v.iter().map(|x| subst_sel(&Spanned::new(x.clone(), sel.span.clone()), env).node).collect(),
        ),
    };
    Spanned::new(node, sel.span.clone())
}

fn subst_value(v: &Spanned<MotionValue>, env: &HashMap<String, Bound>) -> Spanned<MotionValue> {
    let node = match &v.node {
        MotionValue::Name(n) => match env.get(n.split('.').next().unwrap_or(n)) {
            Some(Bound::Number(x)) if !n.contains('.') => MotionValue::Number(*x),
            Some(Bound::Str(s)) if !n.contains('.') => MotionValue::Str(s.clone()),
            _ => MotionValue::Name(subst_name(n, env)),
        },
        MotionValue::Call(f, args) => {
            MotionValue::Call(f.clone(), args.iter().map(|a| subst_value(a, env)).collect())
        }
        MotionValue::List(items) => MotionValue::List(items.iter().map(|a| subst_value(a, env)).collect()),
        other => other.clone(),
    };
    Spanned::new(node, v.span.clone())
}

fn subst_str(s: &Spanned<String>, env: &HashMap<String, Bound>) -> Spanned<String> {
    Spanned::new(subst_name(&s.node, env), s.span.clone())
}

fn subst_path(p: &PropertyRef, env: &HashMap<String, Bound>) -> PropertyRef {
    let mut p = p.clone();
    for seg in &mut p.element.node.segments {
        if let Some(Bound::Name(n)) = env.get(&seg.node.0) {
            seg.node = Identifier::new(n.as_str());
        }
    }
    p
}

fn subst_constraint(d: &ConstrainDecl, env: &HashMap<String, Bound>) -> ConstrainDecl {
    let mut d = d.clone();
    let id = |i: &mut Spanned<Identifier>| {
        if let Some(Bound::Name(n)) = env.get(&i.node.0) {
            i.node = Identifier::new(n.as_str());
        }
    };
    match &mut d.expr {
        ConstraintExpr::Equal { left, right } | ConstraintExpr::EqualWithOffset { left, right, .. } => {
            *left = subst_path(left, env);
            *right = subst_path(right, env);
        }
        ConstraintExpr::Constant { left, .. }
        | ConstraintExpr::GreaterOrEqual { left, .. }
        | ConstraintExpr::LessOrEqual { left, .. } => *left = subst_path(left, env),
        ConstraintExpr::Midpoint { target, a, b, .. } => {
            *target = subst_path(target, env);
            id(a);
            id(b);
        }
        ConstraintExpr::Contains { container, elements, .. } => {
            id(container);
            elements.iter_mut().for_each(id);
        }
    }
    d
}

fn subst_stmt(s: &MotionStmt, env: &HashMap<String, Bound>) -> MotionStmt {
    let sels = |v: &Vec<Spanned<Selector>>| v.iter().map(|x| subst_sel(x, env)).collect();
    let verb = match &s.verb {
        MotionVerb::Show(v) => MotionVerb::Show(sels(v)),
        MotionVerb::Hide(v) => MotionVerb::Hide(sels(v)),
        MotionVerb::Transform { target, modifiers } => MotionVerb::Transform {
            target: subst_sel(target, env),
            modifiers: modifiers
                .iter()
                .map(|m| {
                    let mut m = m.clone();
                    if let StyleValue::Identifier(id) = &m.node.value.node {
                        match env.get(&id.0) {
                            Some(Bound::Str(s)) => m.node.value.node = StyleValue::String(s.clone()),
                            Some(Bound::Number(x)) => {
                                m.node.value.node = StyleValue::Number { value: *x, unit: None }
                            }
                            Some(Bound::Name(n)) => m.node.value.node = StyleValue::Identifier(Identifier::new(n.as_str())),
                            None => {}
                        }
                    }
                    m
                })
                .collect(),
        },
        MotionVerb::Constrain(d) => MotionVerb::Constrain(subst_constraint(d, env)),
        MotionVerb::Disable(n) => MotionVerb::Disable(n.clone()),
        MotionVerb::Enable(n) => MotionVerb::Enable(n.clone()),
        MotionVerb::Draw(v) => MotionVerb::Draw(sels(v)),
        MotionVerb::Undraw(v) => MotionVerb::Undraw(sels(v)),
        MotionVerb::Fly { subject, from, to } => MotionVerb::Fly {
            subject: match subject {
                FlySubject::Ghost(x) => FlySubject::Ghost(subst_sel(x, env)),
                FlySubject::Proxy(x) => FlySubject::Proxy(subst_sel(x, env)),
            },
            from: from.as_ref().map(|f| subst_str(f, env)),
            to: to.iter().map(|x| subst_sel(x, env)).collect(),
        },
        MotionVerb::Move { target, to, to_list, along } => MotionVerb::Move {
            target: subst_sel(target, env),
            to: to.as_ref().map(|t| subst_str(t, env)),
            to_list: to_list.iter().map(|t| subst_str(t, env)).collect(),
            along: along.as_ref().map(|t| subst_str(t, env)),
        },
        MotionVerb::Effect { name, targets } => MotionVerb::Effect {
            name: name.clone(),
            targets: sels(targets),
        },
        MotionVerb::Loop { targets, effect } => MotionVerb::Loop {
            targets: sels(targets),
            effect: effect.clone(),
        },
        MotionVerb::Count(t) => MotionVerb::Count(subst_sel(t, env)),
        MotionVerb::Insert { code, after } => MotionVerb::Insert { code: code.clone(), after: *after },
        MotionVerb::UseLayout(n) => MotionVerb::UseLayout(n.clone()),
        MotionVerb::SetState { target, state } => MotionVerb::SetState { target: subst_sel(target, env), state: state.clone() },
        MotionVerb::Swap { from, to } => MotionVerb::Swap {
            from: subst_sel(from, env),
            to: subst_sel(to, env),
        },
        MotionVerb::Camera(f) => MotionVerb::Camera(f.as_ref().map(|x| subst_str(x, env))),
        MotionVerb::Call { name, args } => MotionVerb::Call {
            name: name.clone(),
            args: args
                .iter()
                .map(|a| {
                    let node = match &a.node {
                        MotionArg::Name(n) => match env.get(n.as_str()) {
                            Some(Bound::Number(x)) => MotionArg::Number(*x),
                            Some(Bound::Str(s)) => MotionArg::Str(s.clone()),
                            _ => MotionArg::Name(subst_name(n, env)),
                        },
                        other => other.clone(),
                    };
                    Spanned::new(node, a.span.clone())
                })
                .collect(),
        },
    };
    MotionStmt {
        verb,
        opts: s
            .opts
            .iter()
            .map(|o| {
                Spanned::new(
                    MotionOpt { key: o.node.key.clone(), value: subst_value(&o.node.value, env) },
                    o.span.clone(),
                )
            })
            .collect(),
        targets: vec![],
        partners: vec![],
    }
}

struct Expander<'a> {
    idx: &'a ElementIndex,
    macros: &'a BTreeMap<String, MotionMacroDecl>,
    /// Named states of each component instance (`state done { ... }`).
    states: &'a HashMap<String, Vec<(String, Vec<Spanned<MotionNode>>)>>,
}

type States = HashMap<String, Vec<(String, Vec<Spanned<MotionNode>>)>>;

fn collect_states(stmts: &[Spanned<Statement>], out: &mut States) {
    for st in stmts {
        match &st.node {
            Statement::ComponentState { name, body, owner } => {
                out.entry(owner.clone()).or_default().push((name.node.clone(), body.clone()))
            }
            Statement::Group(g) => collect_states(&g.children, out),
            Statement::Layout(l) => collect_states(&l.children, out),
            _ => {}
        }
    }
}

fn strip_states(stmts: &mut Vec<Spanned<Statement>>) {
    stmts.retain(|s| !matches!(s.node, Statement::ComponentState { .. }));
    for st in stmts {
        match &mut st.node {
            Statement::Group(g) => strip_states(&mut g.children),
            Statement::Layout(l) => strip_states(&mut l.children),
            _ => {}
        }
    }
}

/// Every statement in a motion tree, mutably.
fn each_stmt(nodes: &mut [Spanned<MotionNode>], f: &mut dyn FnMut(&mut MotionStmt)) {
    for n in nodes {
        match &mut n.node {
            MotionNode::Stmt(s) => f(s),
            MotionNode::Then(b) | MotionNode::After(_, b) | MotionNode::At(_, b) | MotionNode::When(_, _, b) | MotionNode::Beat(_, b) => {
                each_stmt(b, f)
            }
        }
    }
}

/// What a state sets: (selector, property) for transforms, selector for
/// show/hide (true = shown). Any selector: `meter.*` as much as `bg`.
fn state_effects(nodes: &[Spanned<MotionNode>]) -> (Vec<(Selector, StyleKey)>, Vec<(Selector, bool)>) {
    let mut props = Vec::new();
    let mut vis = Vec::new();
    let mut nodes = nodes.to_vec();
    each_stmt(&mut nodes, &mut |s| match &s.verb {
        MotionVerb::Transform { target, modifiers } => {
            for m in modifiers {
                props.push((target.node.clone(), m.node.key.node.clone()));
            }
        }
        MotionVerb::Show(v) | MotionVerb::Hide(v) => {
            for sel in v {
                vis.push((sel.node.clone(), matches!(s.verb, MotionVerb::Show(_))));
            }
        }
        _ => {}
    });
    (props, vis)
}

impl Expander<'_> {
    /// `x [caption: "..."]` made a part `x_caption`: it comes and goes with x.
    fn follow_captions(&self, s: &mut MotionStmt) {
        let caps: Vec<String> = s
            .targets
            .iter()
            .map(|t| {
                let c = format!("{}_caption", t);
                if self.idx.ids.contains(&c) { c } else { String::new() }
            })
            .collect();
        if caps.iter().all(|c| c.is_empty()) {
            return;
        }
        let items = caps.into_iter().map(|c| Spanned::new(MotionValue::Name(c), 0..0)).collect();
        s.opts.push(Spanned::new(
            MotionOpt { key: Spanned::new(super::CAPTIONS_KEY.to_string(), 0..0), value: Spanned::new(MotionValue::List(items), 0..0) },
            0..0,
        ));
    }

    fn check_arg(
        &self,
        call: &Spanned<String>,
        param: &(Spanned<String>, MotionParamType),
        arg: &Spanned<MotionArg>,
    ) -> Result<Bound, LayoutError> {
        let bad = |what: &str| {
            LayoutError::validation_error(&format!(
                "{}(): parameter '{}' expects {}, got {}",
                call.node,
                param.0.node,
                param.1.name(),
                what
            ))
        };
        let missing = |name: &str| LayoutError::UndefinedIdentifier {
            name: name.to_string(),
            span: arg.span.clone(),
            suggestions: self.idx.suggest(name),
        };
        match (&arg.node, param.1) {
            (MotionArg::Number(n), MotionParamType::Number) => Ok(Bound::Number(*n)),
            (MotionArg::Str(s), MotionParamType::Text) => Ok(Bound::Str(s.clone())),
            (MotionArg::Name(n), MotionParamType::Element) => {
                let id = n.replace('.', "_");
                if self.idx.ids.contains(&id) {
                    Ok(Bound::Name(id))
                } else {
                    Err(missing(n))
                }
            }
            (MotionArg::Name(n), MotionParamType::Group) => {
                let id = n.replace('.', "_");
                if self.idx.groups.contains(&id) {
                    Ok(Bound::Name(id))
                } else if self.idx.ids.contains(&id) {
                    Err(bad(&format!("'{}', which is not a group", n)))
                } else {
                    Err(missing(n))
                }
            }
            (MotionArg::Name(n), MotionParamType::Path) => {
                let id = n.replace('.', "_");
                if self.idx.drawables.contains(&id) {
                    Ok(Bound::Name(id))
                } else if self.idx.ids.contains(&id) {
                    Err(bad(&format!("'{}', which is not a connection or path", n)))
                } else {
                    Err(missing(n))
                }
            }
            (MotionArg::Name(n), MotionParamType::Anchor) => {
                let head = n.split('.').next().unwrap_or(n);
                if self.idx.ids.contains(head) {
                    Ok(Bound::Name(n.clone()))
                } else {
                    Err(missing(head))
                }
            }
            (MotionArg::Name(n), _) => Err(bad(&format!("the name '{}'", n))),
            (MotionArg::Number(n), _) => Err(bad(&format!("the number {}", n))),
            (MotionArg::Str(s), _) => Err(bad(&format!("the text \"{}\"", s))),
        }
    }

    fn expand_block(
        &self,
        nodes: &[Spanned<MotionNode>],
        env: &HashMap<String, Bound>,
        depth: usize,
    ) -> Result<Vec<Spanned<MotionNode>>, LayoutError> {
        if depth > 16 {
            return Err(LayoutError::validation_error(
                "motion macros nest more than 16 deep (a macro calling itself?)",
            ));
        }
        let mut out = Vec::new();
        for n in nodes {
            match &n.node {
                MotionNode::Then(b) => out.push(Spanned::new(
                    MotionNode::Then(self.expand_block(b, env, depth)?),
                    n.span.clone(),
                )),
                MotionNode::At(t, b) => out.push(Spanned::new(
                    MotionNode::At(*t, self.expand_block(b, env, depth)?),
                    n.span.clone(),
                )),
                MotionNode::After(t, b) => out.push(Spanned::new(
                    MotionNode::After(*t, self.expand_block(b, env, depth)?),
                    n.span.clone(),
                )),
                MotionNode::When(ev, off, b) => out.push(Spanned::new(
                    MotionNode::When(self.resolve_event(ev, env)?, *off, self.expand_block(b, env, depth)?),
                    n.span.clone(),
                )),
                MotionNode::Beat(name, b) => out.push(Spanned::new(
                    MotionNode::Beat(name.clone(), self.expand_block(b, env, depth)?),
                    n.span.clone(),
                )),
                MotionNode::Stmt(s) => {
                    let s = subst_stmt(s, env);
                    if let MotionVerb::SetState { target, state } = &s.verb {
                        for id in self.idx.resolve(target)? {
                            let body = self.set_state(&id, state, &s.opts, n.span.clone())?;
                            out.push(Spanned::new(MotionNode::After(0.0, self.expand_block(&body, &HashMap::new(), depth + 1)?), n.span.clone()));
                        }
                        continue;
                    }
                    if let MotionVerb::Call { name, args } = &s.verb {
                        let Some(m) = self.macros.get(&name.node) else {
                            let known: HashSet<String> = self.macros.keys().cloned().collect();
                            return Err(LayoutError::UndefinedIdentifier {
                                name: format!("{}()", name.node),
                                span: name.span.clone(),
                                suggestions: crate::layout::find_similar(&known, &name.node, 2)
                                    .into_iter()
                                    .map(|s| format!("{}()", s))
                                    .collect(),
                            });
                        };
                        if m.params.len() != args.len() {
                            return Err(LayoutError::validation_error(&format!(
                                "{}() takes {} argument(s) ({}), got {}",
                                name.node,
                                m.params.len(),
                                m.params
                                    .iter()
                                    .map(|(p, t)| format!("{}: {}", p.node, t.name()))
                                    .collect::<Vec<_>>()
                                    .join(", "),
                                args.len()
                            )));
                        }
                        let mut inner = HashMap::new();
                        for (p, a) in m.params.iter().zip(args) {
                            inner.insert(p.0.node.clone(), self.check_arg(name, p, a)?);
                        }
                        let body = self.expand_block(&m.body, &inner, depth + 1).map_err(|e| explain_macro_part(e, name))?;
                        // Options on the call (`commit(a, b) [delay: 0.2]`)
                        // are not threaded into the body: a macro is its own
                        // choreography. Wrap to keep the body's beats local.
                        if !s.opts.is_empty() {
                            return Err(LayoutError::validation_error(&format!(
                                "{}(): a macro call takes no [options]; wrap it in `after <t> {{ ... }}` to offset it",
                                name.node
                            )));
                        }
                        out.push(Spanned::new(MotionNode::Then(body), n.span.clone()).into_inline());
                        continue;
                    }
                    let s = self.resolve_stmt(s)?;
                    out.push(Spanned::new(MotionNode::Stmt(s), n.span.clone()));
                }
            }
        }
        Ok(out)
    }

    /// `set review done [opts]`: the state's statements, addressed to the
    /// instance's parts, each with the options; what other states of the
    /// component change and this one does not goes back to how the
    /// template draws it.
    fn set_state(
        &self,
        id: &str,
        state: &Spanned<String>,
        opts: &[Spanned<MotionOpt>],
        span: Span,
    ) -> Result<Vec<Spanned<MotionNode>>, LayoutError> {
        let shown = self.idx.show(id);
        let Some(states) = self.states.get(id) else {
            return Err(LayoutError::Located {
                message: format!("set {} {}: `{}` has no states (declare `state {} {{ ... }}` in its template)", shown, state.node, shown, state.node),
                span: state.span.clone(),
            });
        };
        // `default`: the look as declared (every state's changes undone).
        let empty = Vec::new();
        let found = states.iter().find(|(n, _)| *n == state.node).map(|(_, b)| b);
        let found = found.or((state.node == "default").then_some(&empty));
        let Some(body) = found else {
            let known: HashSet<String> = states.iter().map(|(n, _)| n.clone()).collect();
            let mut names: Vec<&String> = known.iter().collect();
            names.sort();
            return Err(LayoutError::Located {
                message: format!(
                    "set {} {}: no state `{}`; {} has {} (and `default`)",
                    shown,
                    state.node,
                    state.node,
                    shown,
                    names.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
                ),
                span: state.span.clone(),
            });
        };
        // Parts by their name in the template: `bg` is `review_bg`.
        let prefix = format!("{}_", id);
        let mut env: HashMap<String, Bound> = self
            .idx
            .ids
            .iter()
            .filter_map(|i| i.strip_prefix(&prefix).map(|local| (local.to_string(), Bound::Name(i.clone()))))
            .collect();
        env.insert("self".into(), Bound::Name(id.to_string()));
        let (mine, my_vis) = state_effects(body);
        let mut nodes = body.clone();
        for (other, obody) in states.iter().filter(|(n, _)| *n != state.node) {
            let _ = other;
            let (props, vis) = state_effects(obody);
            for (part, key) in props {
                if mine.iter().any(|(p, k)| *p == part && *k == key) {
                    continue;
                }
                let sp = span.clone();
                let modifier = Spanned::new(
                    StyleModifier {
                        key: Spanned::new(key.clone(), sp.clone()),
                        value: Spanned::new(StyleValue::Keyword("initial".into()), sp.clone()),
                    },
                    sp.clone(),
                );
                let exists = nodes.iter_mut().any(|n| match &mut n.node {
                    MotionNode::Stmt(MotionStmt { verb: MotionVerb::Transform { target, modifiers }, .. })
                        if target.node == part =>
                    {
                        if !modifiers.iter().any(|m| m.node.key.node == key) {
                            modifiers.push(modifier.clone());
                        }
                        true
                    }
                    _ => false,
                });
                if !exists {
                    nodes.push(Spanned::new(
                        MotionNode::Stmt(MotionStmt {
                            verb: MotionVerb::Transform { target: Spanned::new(part.clone(), sp.clone()), modifiers: vec![modifier] },
                            opts: vec![],
                            targets: vec![],
                            partners: vec![],
                        }),
                        sp,
                    ));
                }
            }
            for (part, on) in vis {
                if my_vis.iter().any(|(p, _)| *p == part) {
                    continue;
                }
                let sel = vec![Spanned::new(part.clone(), span.clone())];
                let verb = if on { MotionVerb::Hide(sel) } else { MotionVerb::Show(sel) };
                nodes.push(Spanned::new(MotionNode::Stmt(MotionStmt { verb, opts: vec![], targets: vec![], partners: vec![] }), span.clone()));
            }
        }
        // Address the parts, and give every statement the options.
        each_stmt(&mut nodes, &mut |s| {
            *s = subst_stmt(s, &env);
            for o in opts {
                if !s.opts.iter().any(|x| x.node.key.node == o.node.key.node) {
                    s.opts.push(o.clone());
                }
            }
        });
        Ok(nodes)
    }

    /// An event's names, as element ids (macro parameters substituted).
    fn resolve_event(&self, ev: &MotionEvent, env: &HashMap<String, Bound>) -> Result<MotionEvent, LayoutError> {
        let id = |n: &Spanned<String>| -> Result<Spanned<String>, LayoutError> {
            let name = subst_name(&n.node, env);
            let id = self.idx.member(&name);
            if self.idx.ids.contains(&id) {
                Ok(Spanned::new(id, n.span.clone()))
            } else {
                Err(LayoutError::UndefinedIdentifier {
                    name: name.clone(),
                    span: n.span.clone(),
                    suggestions: self.idx.suggest(&name),
                })
            }
        };
        Ok(match ev {
            MotionEvent::Reaches { line, target } => MotionEvent::Reaches { line: id(line)?, target: id(target)? },
            MotionEvent::Arrives(e) => MotionEvent::Arrives(id(e)?),
            MotionEvent::Shown(e) => MotionEvent::Shown(id(e)?),
            MotionEvent::Hidden(e) => MotionEvent::Hidden(id(e)?),
            MotionEvent::Accented(e) => MotionEvent::Accented(id(e)?),
            MotionEvent::BeatEnd(b) => MotionEvent::BeatEnd(b.clone()),
        })
    }

    fn resolve_stmt(&self, mut s: MotionStmt) -> Result<MotionStmt, LayoutError> {
        let all = |v: &Vec<Spanned<Selector>>| -> Result<Vec<String>, LayoutError> {
            let mut out = Vec::new();
            for sel in v {
                out.extend(self.idx.resolve(sel)?);
            }
            Ok(out)
        };
        let check_name = |n: &Spanned<String>| -> Result<String, LayoutError> {
            let id = n.node.replace('.', "_");
            if self.idx.ids.contains(&id) {
                Ok(id)
            } else {
                Err(LayoutError::UndefinedIdentifier {
                    name: n.node.clone(),
                    span: n.span.clone(),
                    suggestions: self.idx.suggest(&n.node),
                })
            }
        };
        match &s.verb {
            MotionVerb::Show(v) => {
                s.targets = all(v)?;
                // `show d1, d2 [from: r1, r2]`: each from its own place.
                if let Some(MotionValue::List(items)) = super::opt(&s.opts, "from").map(|v| &v.node) {
                    let mut srcs = Vec::new();
                    for it in items {
                        let MotionValue::Name(n) = &it.node else {
                            return Err(LayoutError::validation_error("show ... [from: a, b, ...]: list element names"));
                        };
                        srcs.push(check_name(&Spanned::new(n.clone(), it.span.clone()))?);
                    }
                    if srcs.len() != s.targets.len() {
                        return Err(LayoutError::validation_error(&format!(
                            "show ... [from: ...]: {} places for {} things to show; give one each, or one for all",
                            srcs.len(),
                            s.targets.len()
                        )));
                    }
                    s.partners = srcs;
                }
                self.follow_captions(&mut s);
            }
            MotionVerb::Hide(v) => {
                s.targets = all(v)?;
                self.follow_captions(&mut s);
            }
            MotionVerb::Draw(v)
            | MotionVerb::Undraw(v)
            | MotionVerb::Effect { targets: v, .. }
            | MotionVerb::Loop { targets: v, .. } => s.targets = all(v)?,
            MotionVerb::Transform { target, .. } | MotionVerb::Count(target) => {
                s.targets = self.idx.resolve(target)?
            }
            MotionVerb::Move { target, to, to_list, along } => {
                s.targets = self.idx.resolve(target)?;
                if let Some(t) = to.as_ref().filter(|t| t.node != "home") {
                    check_name(t)?;
                }
                // `move a, b, c to x, y, z`: one destination each.
                if !to_list.is_empty() {
                    let dests = to_list.iter().map(check_name).collect::<Result<Vec<_>, _>>()?;
                    if dests.len() != s.targets.len() {
                        return Err(LayoutError::validation_error(&format!(
                            "move ... to {}: {} destinations for {} things to move; give one each, or one for all",
                            to_list.iter().map(|t| t.node.as_str()).collect::<Vec<_>>().join(", "),
                            dests.len(),
                            s.targets.len()
                        )));
                    }
                    s.partners = dests;
                }
                if let Some(p) = along {
                    let id = check_name(p)?;
                    if !self.idx.drawables.contains(&id) {
                        return Err(LayoutError::validation_error(&format!(
                            "move ... along {}: '{}' is not a connection or path",
                            p.node, p.node
                        )));
                    }
                }
            }
            MotionVerb::Fly { subject, from, to } => {
                let (FlySubject::Ghost(x) | FlySubject::Proxy(x)) = subject;
                let subjects = self.idx.resolve(x)?;
                if let Some(f) = from {
                    check_name(f)?;
                }
                let dests = all(to)?;
                // One subject fans out to every destination; several subjects
                // pair up one to one, or all converge on a single destination.
                let (targets, partners) = match (subjects.len(), dests.len()) {
                    (1, n) => (vec![subjects[0].clone(); n], dests),
                    (n, 1) => (subjects, vec![dests[0].clone(); n]),
                    (a, b) if a == b => (subjects, dests),
                    (a, b) => {
                        return Err(LayoutError::validation_error(&format!(
                            "fly: {} subject(s) and {} destination(s); use one subject, one destination, or the same number of each",
                            a, b
                        )))
                    }
                };
                s.targets = targets;
                s.partners = partners;
            }
            MotionVerb::Swap { from, to } => {
                s.targets = self.idx.resolve(from)?;
                s.partners = self.idx.resolve(to)?;
                if s.targets.len() != s.partners.len() {
                    return Err(LayoutError::validation_error(&format!(
                        "swap {} -> {}: {} element(s) on the left, {} on the right; they pair up one to one",
                        from.node.describe(),
                        to.node.describe(),
                        s.targets.len(),
                        s.partners.len()
                    )));
                }
            }
            MotionVerb::Camera(f) => {
                if let Some(f) = f {
                    s.targets = vec![check_name(f)?];
                }
            }
            MotionVerb::Constrain(_) | MotionVerb::Disable(_) | MotionVerb::Enable(_) => {}
            MotionVerb::Call { .. } => unreachable!("calls are expanded before resolution"),
            MotionVerb::UseLayout(_) => unreachable!("layouts are desugared before resolution"),
            MotionVerb::SetState { .. } => unreachable!("states are expanded before resolution"),
            MotionVerb::Insert { code, .. } => {
                return Err(LayoutError::UndefinedIdentifier {
                    name: format!("{} (insert works on a `code` block)", code.node),
                    span: code.span.clone(),
                    suggestions: vec![],
                })
            }
        }
        Ok(s)
    }
}

trait IntoInline {
    fn into_inline(self) -> Self;
}

impl IntoInline for Spanned<MotionNode> {
    /// A macro body is inlined as a nested block that starts where the call
    /// stands (not after what precedes it), so it behaves like the statements
    /// it replaces.
    fn into_inline(self) -> Self {
        match self.node {
            MotionNode::Then(b) => Spanned::new(MotionNode::After(0.0, b), self.span),
            other => Spanned::new(other, self.span),
        }
    }
}

/// Resolve macros and selectors in every keyframe, and regenerate each
/// keyframe's flat state operations from its motion tree.
pub fn expand(mut doc: Document, ctx: &ImportContext) -> Result<Document, LayoutError> {
    // One addressing scheme everywhere: `anna.hist.d4` names the d4 inside
    // the hist inside anna, in constraints and connections as in motion.
    {
        let idx = ElementIndex::build(&doc);
        canonicalize_statements(&mut doc.statements, &idx);
    }
    {
        let idx = ElementIndex::build(&doc);
        desugar_layouts(&mut doc, &idx)?;
    }
    // Component states are motion; the layout never sees them.
    let mut states = States::new();
    collect_states(&doc.statements, &mut states);
    strip_states(&mut doc.statements);
    let has_motion = doc.statements.iter().any(|s| {
        matches!(
            s.node,
            Statement::Keyframe(_) | Statement::MotionMacro(_) | Statement::Import(_)
        )
    });
    if !has_motion {
        return Ok(doc);
    }
    let mut macros = BTreeMap::new();
    collect_macros(&doc, ctx, &mut macros, &mut HashSet::new())?;
    let idx = ElementIndex::build(&doc);
    let ex = Expander { idx: &idx, macros: &macros, states: &states };
    for stmt in &mut doc.statements {
        let Statement::Keyframe(kf) = &mut stmt.node else { continue };
        kf.motion = ex.expand_block(&kf.motion, &HashMap::new(), 0)?;
    }
    titles_follow(&mut doc, &idx)?;
    apply_appears(&mut doc, &idx)?;
    for stmt in &mut doc.statements {
        let Statement::Keyframe(kf) = &mut stmt.node else { continue };
        kf.operations = operations_of(&kf.motion);
    }
    // After the state operations: in the frame state `initial` drops the
    // override; the timed statements get the value to tween to.
    resolve_initial(&mut doc);
    Ok(doc)
}

/// `transform x [fill: initial]`: the value the file declares for x (so
/// timing, tweens and lint see a real value). Undeclared: the default look
/// (opacity 1); otherwise it stays `initial`, which state resets drop.
fn resolve_initial(doc: &mut Document) {
    fn decl<'a>(stmts: &'a [Spanned<Statement>], id: &str) -> Option<(&'a [Spanned<StyleModifier>], Option<&'a str>)> {
        for st in stmts {
            match &st.node {
                Statement::Shape(s) => {
                    let name = s.name.as_ref().map(|n| n.node.0.as_str()).or_else(|| match &s.shape_type.node {
                        ShapeType::Path(p) => p.name.as_ref().map(|n| n.node.0.as_str()),
                        _ => None,
                    });
                    if name == Some(id) {
                        let text = match &s.shape_type.node {
                            ShapeType::Text { content } => Some(content.as_str()),
                            _ => None,
                        };
                        return Some((&s.modifiers, text));
                    }
                }
                Statement::Group(g) => {
                    if g.name.as_ref().is_some_and(|n| n.node.0 == id) {
                        return Some((&g.modifiers, None));
                    }
                    if let Some(d) = decl(&g.children, id) {
                        return Some(d);
                    }
                }
                Statement::Layout(l) => {
                    if l.name.as_ref().is_some_and(|n| n.node.0 == id) {
                        return Some((&l.modifiers, None));
                    }
                    if let Some(d) = decl(&l.children, id) {
                        return Some(d);
                    }
                }
                _ => {}
            }
        }
        None
    }
    let base = doc.statements.clone();
    for st in &mut doc.statements {
        let Statement::Keyframe(kf) = &mut st.node else { continue };
        each_stmt(&mut kf.motion, &mut |s| {
            let MotionVerb::Transform { modifiers, .. } = &mut s.verb else { return };
            if s.targets.is_empty() {
                return;
            }
            for m in modifiers.iter_mut() {
                if !is_initial(&m.node.value.node) {
                    continue;
                }
                let key = &m.node.key.node;
                let declared = |id: &str| {
                    let found = decl(&base, id).and_then(|(mods, text)| {
                        mods.iter()
                            .find(|x| x.node.key.node == *key)
                            .map(|x| x.node.value.node.clone())
                            .or_else(|| (*key == StyleKey::Label).then(|| text.map(|t| StyleValue::String(t.to_string()))).flatten())
                    });
                    found.or_else(|| (*key == StyleKey::Opacity).then(|| StyleValue::Number { value: 1.0, unit: None }))
                };
                // Several targets (`meter.*`): when they all declare the
                // same value, that is the value to go back to.
                let values: Vec<Option<StyleValue>> = s.targets.iter().map(|t| declared(t)).collect();
                if let Some(Some(v)) = values.first() {
                    if values.iter().all(|x| x.as_ref() == Some(v)) {
                        m.node.value.node = v.clone();
                    }
                }
            }
        });
    }
}

/// `motion [title: s.heading]`: the element shows each keyframe's
/// `[title: "..."]`, so a title is written once. Where a keyframe's title
/// differs from what it shows, it swaps (`title_swap:`, fade by default).
fn titles_follow(doc: &mut Document, idx: &ElementIndex) -> Result<(), LayoutError> {
    let mut target: Option<Spanned<String>> = None;
    let mut swap = "fade".to_string();
    for st in &doc.statements {
        if let Statement::MotionDefaults(opts) = &st.node {
            for o in opts {
                match (o.node.key.node.as_str(), &o.node.value.node) {
                    ("title", MotionValue::Name(n)) => target = Some(Spanned::new(n.clone(), o.node.value.span.clone())),
                    ("title_swap", MotionValue::Name(n)) => swap = n.clone(),
                    _ => {}
                }
            }
        }
    }
    let Some(target) = target else { return Ok(()) };
    let id = idx.member(&target.node);
    if !idx.ids.contains(&id) {
        return Err(LayoutError::UndefinedIdentifier {
            name: target.node.clone(),
            span: target.span.clone(),
            suggestions: idx.suggest(&target.node),
        });
    }
    fn text_of(stmts: &[Spanned<Statement>], id: &str) -> Option<String> {
        for st in stmts {
            match &st.node {
                Statement::Shape(s) if s.name.as_ref().is_some_and(|n| n.node.0 == id) => {
                    if let ShapeType::Text { content } = &s.shape_type.node {
                        return Some(content.clone());
                    }
                    return s.modifiers.iter().find_map(|m| match (&m.node.key.node, &m.node.value.node) {
                        (StyleKey::Label, StyleValue::String(t)) => Some(t.clone()),
                        _ => None,
                    });
                }
                Statement::Group(g) => {
                    if let Some(t) = text_of(&g.children, id) {
                        return Some(t);
                    }
                }
                Statement::Layout(l) => {
                    if let Some(t) = text_of(&l.children, id) {
                        return Some(t);
                    }
                }
                _ => {}
            }
        }
        None
    }
    let mut showing = text_of(&doc.statements, &id);
    for st in &mut doc.statements {
        let Statement::Keyframe(kf) = &mut st.node else { continue };
        let Some(title) = kf.title.clone() else { continue };
        if showing.as_deref() == Some(title.as_str()) {
            continue;
        }
        let sp = target.span.clone();
        let stmt = MotionStmt {
            verb: MotionVerb::Transform {
                target: Spanned::new(Selector::Name(id.clone()), sp.clone()),
                modifiers: vec![Spanned::new(
                    StyleModifier {
                        key: Spanned::new(StyleKey::Label, sp.clone()),
                        value: Spanned::new(StyleValue::String(title.clone()), sp.clone()),
                    },
                    sp.clone(),
                )],
            },
            opts: vec![Spanned::new(
                MotionOpt { key: Spanned::new("swap".into(), sp.clone()), value: Spanned::new(MotionValue::Name(swap.clone()), sp.clone()) },
                sp.clone(),
            )],
            targets: vec![id.clone()],
            partners: vec![],
        };
        // Last in the block, at the start: a `then` in the frame does not
        // wait for it.
        kf.motion.push(Spanned::new(MotionNode::At(0.0, vec![Spanned::new(MotionNode::Stmt(stmt), sp.clone())]), sp));
        showing = Some(title);
    }
    Ok(())
}

/// Elements that declare their entrance: `[appears: go_back]`.
fn collect_appears(stmts: &[Spanned<Statement>], out: &mut Vec<(String, Spanned<String>)>) {
    for st in stmts {
        let (id, mods, kids): (Option<String>, &[Spanned<StyleModifier>], &[Spanned<Statement>]) = match &st.node {
            Statement::Shape(s) => (
                s.name.as_ref().map(|n| n.node.0.clone()).or_else(|| match &s.shape_type.node {
                    ShapeType::Path(p) => p.name.as_ref().map(|n| n.node.0.clone()),
                    _ => None,
                }),
                &s.modifiers,
                &[],
            ),
            Statement::Group(g) => (g.name.as_ref().map(|n| n.node.0.clone()), &g.modifiers, &g.children),
            Statement::Layout(l) => (l.name.as_ref().map(|n| n.node.0.clone()), &l.modifiers, &l.children),
            Statement::Connection(conns) => {
                for c in conns {
                    if let Some(n) = &c.name {
                        if let Some(f) = appears_of(&c.modifiers) {
                            out.push((n.node.0.clone(), f));
                        }
                    }
                }
                continue;
            }
            _ => continue,
        };
        let collapsed = mods.iter().any(|m| {
            matches!(&m.node.key.node, StyleKey::Custom(k) if k == "collapsed")
                && matches!(&m.node.value.node, StyleValue::Identifier(i) if i.0 == "true")
        });
        if let Some(id) = id {
            if collapsed {
                // `collapsed: true`: takes no room and is hidden until a
                // `show [enter: expand]` (or an `insert`) opens it.
                out.push((id, Spanned::new("later:collapsed".to_string(), st.span.clone())));
            } else if let Some(f) = appears_of(mods) {
                out.push((id, f));
            }
        }
        collect_appears(kids, out);
    }
}

fn appears_of(mods: &[Spanned<StyleModifier>]) -> Option<Spanned<String>> {
    mods.iter().find_map(|m| match (&m.node.key.node, &m.node.value.node) {
        (StyleKey::Custom(k), StyleValue::Identifier(v)) if k == "appears" => {
            Some(Spanned::new(v.0.clone(), m.node.value.span.clone()))
        }
        (StyleKey::Custom(k), StyleValue::String(v) | StyleValue::Keyword(v)) if k == "appears" => {
            Some(Spanned::new(v.clone(), m.node.value.span.clone()))
        }
        _ => None,
    })
}

fn shows(nodes: &[Spanned<MotionNode>], id: &str) -> bool {
    let mut found = false;
    super::for_each_stmt(nodes, &mut |n| {
        if let MotionNode::Stmt(s) = &n.node {
            let entering = matches!(s.verb, MotionVerb::Show(_))
                || matches!(s.verb, MotionVerb::Swap { .. }) && s.partners.iter().any(|p| p == id);
            if entering && (s.targets.iter().any(|t| t == id) || s.partners.iter().any(|t| t == id)) {
                found = true;
            }
        }
    });
    found
}

/// Turn `appears:` into a hide before that keyframe and a show in it (unless
/// the keyframe already shows it, with whatever entrance it chose).
fn apply_appears(doc: &mut Document, idx: &ElementIndex) -> Result<(), LayoutError> {
    let mut decls = Vec::new();
    collect_appears(&doc.statements, &mut decls);
    if decls.is_empty() {
        return Ok(());
    }
    let names: Vec<String> = doc
        .statements
        .iter()
        .filter_map(|s| match &s.node {
            Statement::Keyframe(k) => Some(k.name.node.clone()),
            _ => None,
        })
        .collect();
    for (id, frame) in decls {
        // `appears: later`: hidden from the start; a statement brings it on.
        if frame.node == "later" || frame.node == "later:collapsed" {
            let span = frame.span.clone();
            let opts = if frame.node == "later:collapsed" {
                vec![Spanned::new(
                    MotionOpt { key: Spanned::new("exit".to_string(), span.clone()), value: Spanned::new(MotionValue::Name("collapse".into()), span.clone()) },
                    span.clone(),
                )]
            } else {
                vec![]
            };
            if let Some(Statement::Keyframe(kf)) = doc
                .statements
                .iter_mut()
                .map(|s| &mut s.node)
                .find(|s| matches!(s, Statement::Keyframe(_)))
            {
                kf.motion.insert(
                    0,
                    Spanned::new(
                        MotionNode::Stmt(MotionStmt {
                            verb: MotionVerb::Hide(vec![Spanned::new(Selector::Name(id.clone()), span.clone())]),
                            opts,
                            targets: vec![id.clone()],
                            partners: vec![],
                        }),
                        span,
                    ),
                );
            }
            continue;
        }
        let Some(fi) = names.iter().position(|n| n == &frame.node) else {
            let known: HashSet<String> = names.iter().cloned().collect();
            return Err(LayoutError::UndefinedIdentifier {
                name: format!("keyframe \"{}\" (in appears: on '{}')", frame.node, idx.show(&id)),
                span: frame.span.clone(),
                suggestions: crate::layout::find_similar(&known, &frame.node, 2),
            });
        };
        let span = frame.span.clone();
        let stmt = |verb: MotionVerb| {
            Spanned::new(
                MotionNode::Stmt(MotionStmt { verb, opts: vec![], targets: vec![id.clone()], partners: vec![] }),
                span.clone(),
            )
        };
        let sel = || vec![Spanned::new(Selector::Name(id.clone()), span.clone())];
        let mut k = 0;
        for st in doc.statements.iter_mut() {
            let Statement::Keyframe(kf) = &mut st.node else { continue };
            if k == 0 && fi > 0 {
                kf.motion.insert(0, stmt(MotionVerb::Hide(sel())));
            }
            if k == fi && !shows(&kf.motion, &id) {
                kf.motion.insert(0, stmt(MotionVerb::Show(sel())));
            }
            k += 1;
        }
    }
    Ok(())
}

/// The flat state operations of an expanded motion tree, in source order.
pub fn operations_of(nodes: &[Spanned<MotionNode>]) -> Vec<Spanned<KeyframeOp>> {
    let mut out = Vec::new();
    super::for_each_stmt(nodes, &mut |n| {
        let MotionNode::Stmt(s) = &n.node else { return };
        for op in stmt_state_ops(s, &n.span) {
            out.push(Spanned::new(op, n.span.clone()));
        }
    });
    out
}

/// State operations of one resolved statement (all its targets).
pub fn stmt_state_ops(s: &MotionStmt, span: &Span) -> Vec<KeyframeOp> {
    let ids = |v: &[String]| {
        v.iter()
            .map(|t| Spanned::new(Identifier::new(t.as_str()), span.clone()))
            .collect::<Vec<_>>()
    };
    super::stmt_ops(&s.verb, &s.opts, &ids(&s.targets), &ids(&s.partners))
}

/// State operations of one statement for a single target (index `i`), used
/// by the compiler to attribute changes per staggered target.
pub fn atom_state_ops(s: &MotionStmt, i: usize, span: &Span) -> Vec<KeyframeOp> {
    let mut one = s.clone();
    if i < s.targets.len() {
        one.targets = vec![s.targets[i].clone()];
        if i < s.partners.len() {
            one.partners = vec![s.partners[i].clone()];
        }
        // Its own caption only.
        for o in one.opts.iter_mut().filter(|o| o.node.key.node == super::CAPTIONS_KEY) {
            if let MotionValue::List(items) = &o.node.value.node {
                let mine: Vec<_> = items.get(i).cloned().into_iter().collect();
                o.node.value.node = MotionValue::List(mine);
            }
        }
    }
    stmt_state_ops(&one, span)
}

/// The internal name of a dotted path, if it names an element.
fn join_path(segments: &[&str], idx: &ElementIndex) -> Option<String> {
    let joined = segments.join("_");
    if idx.ids.contains(&joined) {
        return Some(joined);
    }
    // A one-shape template is drawn as its instance: `f.val` is `f`.
    idx.aliases.get(&joined).cloned()
}

fn canonical_path(path: &mut Spanned<ElementPath>, idx: &ElementIndex) {
    if path.node.segments.len() < 2 {
        return;
    }
    let segs: Vec<&str> = path.node.segments.iter().map(|s| s.node.0.as_str()).collect();
    if let Some(joined) = join_path(&segs, idx) {
        let span = path.span.clone();
        path.node = ElementPath::simple(Identifier::new(joined), span);
    } else if !idx.ids.contains(*segs.last().unwrap()) {
        // Neither `a.b` nor a bare `b` exists: report the path as written,
        // not its last word.
        let span = path.span.clone();
        path.node = ElementPath::simple(Identifier::new(segs.join(".")), span);
    }
}

const BUILTIN_ANCHORS: &[&str] = &[
    "top", "bottom", "left", "right", "center", "horizontal_center", "vertical_center",
];

fn canonical_ref(r: &mut AnchorReference, idx: &ElementIndex) {
    let Some(anchor) = r.anchor.clone() else { return };
    let mut segs: Vec<String> = vec![r.element.node.0.clone()];
    segs.extend(anchor.node.split('.').map(str::to_string));
    // Longest prefix that names an element; the rest (if any) is its anchor.
    for k in (1..=segs.len()).rev() {
        let refs: Vec<&str> = segs[..k].iter().map(String::as_str).collect();
        let Some(joined) = join_path(&refs, idx) else { continue };
        let rest = segs[k..].join(".");
        // `box.top` stays an anchor even if a `box_top` element exists.
        if k == segs.len() && segs.len() == 2 && BUILTIN_ANCHORS.contains(&segs[1].as_str()) {
            return;
        }
        if k == 1 {
            return;
        }
        r.element = Spanned::new(Identifier::new(joined), r.element.span.clone());
        r.anchor = if rest.is_empty() { None } else { Some(Spanned::new(rest, anchor.span.clone())) };
        return;
    }
}

fn canonical_constraint(d: &mut ConstrainDecl, idx: &ElementIndex) {
    match &mut d.expr {
        ConstraintExpr::Equal { left, right } | ConstraintExpr::EqualWithOffset { left, right, .. } => {
            canonical_path(&mut left.element, idx);
            canonical_path(&mut right.element, idx);
        }
        ConstraintExpr::Constant { left, .. }
        | ConstraintExpr::GreaterOrEqual { left, .. }
        | ConstraintExpr::LessOrEqual { left, .. } => canonical_path(&mut left.element, idx),
        ConstraintExpr::Midpoint { target, .. } => canonical_path(&mut target.element, idx),
        ConstraintExpr::Contains { .. } => {}
    }
}

fn canonical_motion(nodes: &mut [Spanned<MotionNode>], idx: &ElementIndex) {
    for n in nodes {
        match &mut n.node {
            MotionNode::Stmt(MotionStmt { verb: MotionVerb::Constrain(d), .. }) => canonical_constraint(d, idx),
            MotionNode::Then(b) | MotionNode::After(_, b) | MotionNode::At(_, b) | MotionNode::When(_, _, b) | MotionNode::Beat(_, b) => {
                canonical_motion(b, idx)
            }
            _ => {}
        }
    }
}

fn canonicalize_statements(stmts: &mut [Spanned<Statement>], idx: &ElementIndex) {
    for st in stmts {
        match &mut st.node {
            Statement::Constrain(d) => canonical_constraint(d, idx),
            Statement::Connection(conns) => {
                for c in conns {
                    canonical_ref(&mut c.from, idx);
                    canonical_ref(&mut c.to, idx);
                }
            }
            Statement::Group(g) => canonicalize_statements(&mut g.children, idx),
            Statement::Layout(l) => canonicalize_statements(&mut l.children, idx),
            Statement::Keyframe(k) => {
                canonical_motion(&mut k.motion, idx);
                for op in &mut k.operations {
                    if let KeyframeOp::Constrain(d) = &mut op.node {
                        canonical_constraint(d, idx);
                    }
                }
            }
            Statement::MotionMacro(m) => canonical_motion(&mut m.body, idx),
            Statement::NamedLayout { constraints, .. } => {
                for d in constraints {
                    canonical_constraint(&mut d.node, idx);
                }
            }
            _ => {}
        }
    }
}

/// Which way a constraint places its element: overlapping classes on the
/// same element mean a layout's constraint replaces the base one.
fn constraint_axes(expr: &ConstraintExpr) -> Option<(String, &'static [&'static str])> {
    let pr = match expr {
        ConstraintExpr::Equal { left, .. }
        | ConstraintExpr::EqualWithOffset { left, .. }
        | ConstraintExpr::Constant { left, .. }
        | ConstraintExpr::GreaterOrEqual { left, .. }
        | ConstraintExpr::LessOrEqual { left, .. } => left,
        ConstraintExpr::Midpoint { target, .. } => target,
        ConstraintExpr::Contains { .. } => return None,
    };
    let axes: &'static [&'static str] = match &pr.property.node {
        ConstraintProperty::X
        | ConstraintProperty::Left
        | ConstraintProperty::Right
        | ConstraintProperty::CenterX
        | ConstraintProperty::AnchorX(_) => &["x"],
        ConstraintProperty::Y
        | ConstraintProperty::Top
        | ConstraintProperty::Bottom
        | ConstraintProperty::CenterY
        | ConstraintProperty::AnchorY(_) => &["y"],
        ConstraintProperty::Center | ConstraintProperty::Anchor(_) => &["x", "y"],
        ConstraintProperty::Width => &["width"],
        ConstraintProperty::Height => &["height"],
    };
    Some((pr.element.node.to_string(), axes))
}

/// `layout beside { constrain ... }` + `use layout beside`: the layout's
/// constraints are named `layout_beside_<i>`; the base constraints they
/// replace (same element, same axis) form the implicit `default` layout.
/// `use layout X` becomes, at one moment: disable every other layout's
/// constraints (and the base ones X replaces), enable and add X's.
fn desugar_layouts(doc: &mut Document, idx: &ElementIndex) -> Result<(), LayoutError> {
    let mut layouts: Vec<(String, Vec<ConstrainDecl>)> = Vec::new();
    doc.statements.retain(|st| {
        if let Statement::NamedLayout { name, constraints } = &st.node {
            let decls = constraints
                .iter()
                .enumerate()
                .map(|(i, d)| ConstrainDecl {
                    expr: d.node.expr.clone(),
                    name: Some(Spanned::new(Identifier::new(format!("layout_{}_{}", name.node, i)), d.span.clone())),
                })
                .collect();
            layouts.push((name.node.clone(), decls));
            false
        } else {
            true
        }
    });
    let mut seen = HashSet::new();
    for (n, _) in &layouts {
        if n == "default" {
            return Err(LayoutError::validation_error(
                "`layout default` is the layout the file declares outside layout blocks; name yours differently",
            ));
        }
        if !seen.insert(n.clone()) {
            return Err(LayoutError::validation_error(&format!("layout `{}` is declared twice", n)));
        }
    }
    let used = doc.statements.iter().any(|s| match &s.node {
        Statement::Keyframe(k) => uses_layout(&k.motion),
        Statement::MotionMacro(m) => uses_layout(&m.body),
        _ => false,
    });
    if layouts.is_empty() && !used {
        return Ok(());
    }
    // Base constraints some layout replaces: named (auto-named if need be).
    let placed: Vec<(String, &'static [&'static str])> =
        layouts.iter().flat_map(|(_, ds)| ds.iter().filter_map(|d| constraint_axes(&d.expr))).collect();
    let mut defaults = Vec::new();
    fn walk(
        stmts: &mut [Spanned<Statement>],
        placed: &[(String, &'static [&'static str])],
        idx: &ElementIndex,
        out: &mut Vec<(String, String, &'static [&'static str])>,
    ) {
        for st in stmts {
            match &mut st.node {
                Statement::Constrain(d) => {
                    let Some((el, axes)) = constraint_axes(&d.expr) else { continue };
                    // How a component holds its own parts together (a frame
                    // around its title) is not where it is placed.
                    if constraint_others(&d.expr).iter().any(|o| same_component(&el, o, idx)) {
                        continue;
                    }
                    if placed.iter().any(|(e, a)| *e == el && a.iter().any(|x| axes.contains(x))) {
                        let name = d
                            .name
                            .get_or_insert_with(|| {
                                Spanned::new(Identifier::new(format!("layout_default_{}", out.len())), st.span.clone())
                            })
                            .node
                            .0
                            .clone();
                        out.push((name, el, axes));
                    }
                }
                Statement::Group(g) => walk(&mut g.children, placed, idx, out),
                Statement::Layout(l) => walk(&mut l.children, placed, idx, out),
                _ => {}
            }
        }
    }
    walk(&mut doc.statements, &placed, idx, &mut defaults);
    let ctx = LayoutSet { layouts, defaults };
    for st in &mut doc.statements {
        match &mut st.node {
            Statement::Keyframe(k) => ctx.rewrite(&mut k.motion)?,
            Statement::MotionMacro(m) => ctx.rewrite(&mut m.body)?,
            _ => {}
        }
    }
    Ok(())
}

/// The elements a constraint places its element against.
fn constraint_others(expr: &ConstraintExpr) -> Vec<String> {
    match expr {
        ConstraintExpr::Equal { right, .. } | ConstraintExpr::EqualWithOffset { right, .. } => {
            vec![right.element.node.to_string()]
        }
        ConstraintExpr::Midpoint { a, b, .. } => vec![a.node.0.clone(), b.node.0.clone()],
        _ => vec![],
    }
}

/// Both parts of one component (`merged_bg`, `merged_title` in `merged`).
fn same_component(a: &str, b: &str, idx: &ElementIndex) -> bool {
    idx.groups.iter().any(|g| {
        let p = format!("{}_", g);
        (a == g || a.starts_with(&p)) && (b == g || b.starts_with(&p))
    })
}

fn uses_layout(nodes: &[Spanned<MotionNode>]) -> bool {
    nodes.iter().any(|n| match &n.node {
        MotionNode::Stmt(s) => matches!(s.verb, MotionVerb::UseLayout(_)),
        MotionNode::Then(b) | MotionNode::After(_, b) | MotionNode::At(_, b) | MotionNode::When(_, _, b) | MotionNode::Beat(_, b) => {
            uses_layout(b)
        }
    })
}

struct LayoutSet {
    layouts: Vec<(String, Vec<ConstrainDecl>)>,
    /// Base constraints some layout replaces: name, element, axes.
    defaults: Vec<(String, String, &'static [&'static str])>,
}

impl LayoutSet {
    fn rewrite(&self, nodes: &mut [Spanned<MotionNode>]) -> Result<(), LayoutError> {
        for n in nodes {
            match &mut n.node {
                MotionNode::Stmt(s) => {
                    let MotionVerb::UseLayout(name) = &s.verb else { continue };
                    let stmts = self.use_layout(name, &s.opts)?;
                    n.node = MotionNode::After(0.0, stmts.into_iter().map(|x| Spanned::new(MotionNode::Stmt(x), n.span.clone())).collect());
                }
                MotionNode::Then(b) | MotionNode::After(_, b) | MotionNode::At(_, b) | MotionNode::When(_, _, b) | MotionNode::Beat(_, b) => {
                    self.rewrite(b)?
                }
            }
        }
        Ok(())
    }

    fn use_layout(&self, name: &Spanned<String>, opts: &[Spanned<MotionOpt>]) -> Result<Vec<MotionStmt>, LayoutError> {
        let id = |s: &str| Spanned::new(Identifier::new(s), name.span.clone());
        let names = |ds: &[ConstrainDecl]| -> Vec<String> { ds.iter().filter_map(|d| d.name.as_ref().map(|n| n.node.0.clone())).collect() };
        // Tagged with the layout, so the timeline says `use layout beside`.
        let mut opts = opts.to_vec();
        opts.push(Spanned::new(
            MotionOpt {
                key: Spanned::new("layout".into(), name.span.clone()),
                value: Spanned::new(MotionValue::Name(name.node.clone()), name.span.clone()),
            },
            name.span.clone(),
        ));
        let stmt = |verb| MotionStmt { verb, opts: opts.clone(), targets: vec![], partners: vec![] };
        let chosen = if name.node == "default" {
            None
        } else {
            match self.layouts.iter().find(|(n, _)| *n == name.node) {
                Some(l) => Some(l),
                None => {
                    let mut known: HashSet<String> = self.layouts.iter().map(|(n, _)| n.clone()).collect();
                    known.insert("default".into());
                    return Err(LayoutError::UndefinedIdentifier {
                        name: format!("layout {}", name.node),
                        span: name.span.clone(),
                        suggestions: crate::layout::find_similar(&known, &name.node, 2),
                    });
                }
            }
        };
        let mut off: Vec<String> = self
            .layouts
            .iter()
            .filter(|(n, _)| Some(n) != chosen.map(|(c, _)| c))
            .flat_map(|(_, ds)| names(ds))
            .collect();
        let mut out = Vec::new();
        match chosen {
            Some((_, ds)) => {
                // The base constraints this layout replaces.
                let axes: Vec<_> = ds.iter().filter_map(|d| constraint_axes(&d.expr)).collect();
                off.extend(
                    self.defaults
                        .iter()
                        .filter(|(_, el, a)| axes.iter().any(|(e, x)| e == el && x.iter().any(|k| a.contains(k))))
                        .map(|(n, _, _)| n.clone()),
                );
                out.push(stmt(MotionVerb::Disable(off.iter().map(|s| id(s)).collect())));
                out.push(stmt(MotionVerb::Enable(names(ds).iter().map(|s| id(s)).collect())));
                for d in ds {
                    out.push(stmt(MotionVerb::Constrain(d.clone())));
                }
            }
            None => {
                out.push(stmt(MotionVerb::Disable(off.iter().map(|s| id(s)).collect())));
                out.push(stmt(MotionVerb::Enable(self.defaults.iter().map(|(s, _, _)| id(s)).collect())));
            }
        }
        out.retain(|s| !matches!(&s.verb, MotionVerb::Disable(v) | MotionVerb::Enable(v) if v.is_empty()));
        Ok(out)
    }
}

/// `commit(folder, st, track)` where `st` has no `dot`: the body fails on
/// `st.dot`, which the author never wrote. Say which macro wanted which part
/// of which argument, at the call.
fn explain_macro_part(e: LayoutError, name: &Spanned<String>) -> LayoutError {
    // Already explained by a macro this one calls: point at this call (the
    // outermost is the one the author wrote) and say how it got there.
    if let LayoutError::Located { message, .. } = &e {
        if message.contains("() uses a part `") {
            return LayoutError::Located {
                message: format!("{}() -> {}", name.node, message),
                span: name.span.clone(),
            };
        }
        return e;
    }
    let LayoutError::UndefinedIdentifier { name: missing, .. } = &e else { return e };
    let Some((head, part)) = missing.rsplit_once('.') else { return e };
    let hint = if ["snapshot", "change", "commit"].contains(&name.node.as_str()) {
        " (the templates in `ail:motion/git` have the parts its macros use: git_station, git_file)"
    } else {
        ""
    };
    LayoutError::Located {
        message: format!(
            "{}() uses a part `{}` of `{}`, and `{}` has none; pass an element that has one{}",
            name.node, part, head, head, hint
        ),
        span: name.span.clone(),
    }
}
