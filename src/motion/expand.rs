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
}

impl ElementIndex {
    pub fn build(doc: &Document) -> Self {
        let mut idx = ElementIndex::default();
        for stmt in &doc.statements {
            if let Some(id) = idx.visit(&stmt.node) {
                idx.top.push(id);
            }
        }
        fn dotted(stmts: &[Spanned<Statement>], scope: Option<(&str, &str)>, out: &mut HashMap<String, String>) {
            for st in stmts {
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
        dotted.replace('.', "_")
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
    inline(doc.statements, ctx, &mut HashSet::new(), &mut out, true)?;
    Ok(Document { statements: out })
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
        MotionVerb::Move { target, to, along } => MotionVerb::Move {
            target: subst_sel(target, env),
            to: to.as_ref().map(|t| subst_str(t, env)),
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
}

impl Expander<'_> {
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
                MotionNode::Stmt(s) => {
                    let s = subst_stmt(s, env);
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
                        let body = self.expand_block(&m.body, &inner, depth + 1)?;
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
            MotionVerb::Show(v)
            | MotionVerb::Hide(v)
            | MotionVerb::Draw(v)
            | MotionVerb::Undraw(v)
            | MotionVerb::Effect { targets: v, .. }
            | MotionVerb::Loop { targets: v, .. } => s.targets = all(v)?,
            MotionVerb::Transform { target, .. } | MotionVerb::Count(target) => {
                s.targets = self.idx.resolve(target)?
            }
            MotionVerb::Move { target, to, along } => {
                s.targets = self.idx.resolve(target)?;
                if let Some(t) = to.as_ref().filter(|t| t.node != "home") {
                    check_name(t)?;
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
    let ex = Expander { idx: &idx, macros: &macros };
    for stmt in &mut doc.statements {
        let Statement::Keyframe(kf) = &mut stmt.node else { continue };
        kf.motion = ex.expand_block(&kf.motion, &HashMap::new(), 0)?;
    }
    apply_appears(&mut doc, &idx)?;
    for stmt in &mut doc.statements {
        let Statement::Keyframe(kf) = &mut stmt.node else { continue };
        kf.operations = operations_of(&kf.motion);
    }
    Ok(doc)
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
        if let (Some(id), Some(f)) = (id, appears_of(mods)) {
            out.push((id, f));
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
        if frame.node == "later" {
            let span = frame.span.clone();
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
                            opts: vec![],
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
    }
    stmt_state_ops(&one, span)
}

/// The internal name of a dotted path, if it names an element.
fn join_path(segments: &[&str], idx: &ElementIndex) -> Option<String> {
    let joined = segments.join("_");
    idx.ids.contains(&joined).then_some(joined)
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
            MotionNode::Then(b) | MotionNode::After(_, b) | MotionNode::At(_, b) => canonical_motion(b, idx),
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
            _ => {}
        }
    }
}
