//! `each a, i in items { ... }`: the body once per list item.
//!
//! Expansion happens when templates are resolved, because the lists are
//! template parameters (`attrs`) that only have a value per instance.
//!
//! Inside the body, `a` stands for the item and `i` for its 0-based position
//! (`label: a`, `text a`, `"{a}"`, `"{i}"`, or a number such as `width: i`).
//! Every element, anchor, constraint name or connection name the body declares
//! gets the item's 1-based number appended (`row` -> `row_1`, `row_2`, ...),
//! and references to it inside the body follow. Nested blocks append their own
//! number after the outer one (`cell_2_3`), so names never collide.

use std::collections::{HashMap, HashSet};

use crate::parser::ast::{
    AnchorPosition, AnchorReference, ConstrainDecl, ConstraintExpr, EachBlock, ElementPath, Identifier,
    PropertyRef, ShapeType, Span, Spanned, Statement, StyleKey, StyleModifier, StyleValue,
};

use super::names::{head_of, splice_dotted, splice_path};
use super::registry::TemplateError;

type Stmts = Vec<Spanned<Statement>>;

/// What a loop variable stands for in one copy of the body.
#[derive(Clone)]
struct Bound {
    value: StyleValue,
    /// Where the item was written (the list item), for error carets.
    span: Span,
    /// 1-based item number, for messages.
    item: usize,
    /// `each n, i in ...`: `i` is the index, not the item.
    is_index: bool,
    /// The loop header, for messages (`each n in ...`).
    header: String,
}
type Vars = HashMap<String, Bound>;

/// Expand every `each` in `stmts`. `lists` are the values `each x in NAME`
/// can name (a template's parameters); other kinds of value are ignored.
pub fn expand_each(stmts: Stmts, lists: &HashMap<String, StyleValue>) -> Result<Stmts, TemplateError> {
    // Names declared anywhere in this scope (the document, or a template
    // body): a loop variable used as a name must not hide one of them.
    let scope = declared_names(&stmts);
    expand_in(stmts, lists, &scope)
}

fn expand_in(stmts: Stmts, lists: &HashMap<String, StyleValue>, scope: &HashSet<String>) -> Result<Stmts, TemplateError> {
    let mut out = Vec::with_capacity(stmts.len());
    for st in stmts {
        let span = st.span.clone();
        match st.node {
            Statement::Each(e) => out.extend(expand_block(e, lists, scope)?),
            Statement::Layout(mut l) => {
                l.children = expand_in(l.children, lists, scope)?;
                out.push(Spanned::new(Statement::Layout(l), span));
            }
            Statement::Group(mut g) => {
                g.children = expand_in(g.children, lists, scope)?;
                out.push(Spanned::new(Statement::Group(g), span));
            }
            other => out.push(Spanned::new(other, span)),
        }
    }
    Ok(out)
}

fn err(message: String) -> TemplateError {
    TemplateError::Each { message }
}

fn describe(v: &StyleValue) -> String {
    match v {
        StyleValue::String(s) => format!("the string \"{}\"", s),
        StyleValue::Number { value, .. } => format!("the number {}", value),
        StyleValue::Keyword(k) => format!("`{}`", k),
        StyleValue::Identifier(i) => format!("`{}`", i.0),
        StyleValue::Color(_) => "a colour".to_string(),
        StyleValue::Record(_) => "a record".to_string(),
        _ => "something that is not a list".to_string(),
    }
}

fn expand_block(e: EachBlock, lists: &HashMap<String, StyleValue>, scope: &HashSet<String>) -> Result<Stmts, TemplateError> {
    let var = e.var.node.0.clone();
    let header = match &e.index {
        Some(ix) => format!("each {}, {} in ...", var, ix.node.0),
        None => format!("each {} in ...", var),
    };
    // `each a in items { a -> hub }` while an element `a` exists: the
    // reader cannot tell which one the arrow means.
    for name in std::iter::once(&var).chain(e.index.as_ref().map(|ix| &ix.node.0)) {
        if scope.contains(name) {
            if let Some(used) = used_as_name(&e.body, name) {
                return Err(err(format!(
                    "{}: `{}` is also the name of an element here, and the body uses `{}` where a name goes ({}); rename the loop variable (each it in ...)",
                    header, name, name, used
                )));
            }
        }
    }
    let items: Vec<Spanned<StyleValue>> = match &e.source.node {
        StyleValue::List(items) => items.clone(),
        StyleValue::Identifier(id) => match lists.get(&id.0) {
            Some(StyleValue::List(items)) => items.clone(),
            Some(other) => {
                return Err(err(format!(
                    "each {} in {}: `{}` is {}, not a list; give it a list such as [\"a\", \"b\"]",
                    var,
                    id.0,
                    id.0,
                    describe(other)
                )))
            }
            None => {
                let mut known: Vec<&str> = lists
                    .iter()
                    .filter(|(_, v)| matches!(v, StyleValue::List(_)))
                    .map(|(k, _)| k.as_str())
                    .collect();
                known.sort();
                let hint = if known.is_empty() {
                    "there is no list parameter here; declare one on the template (e.g. `attrs: []`) or write the list out: [\"a\", \"b\"]".to_string()
                } else {
                    format!("list parameters here: {}", known.join(", "))
                };
                return Err(err(format!("each {} in {}: no list called `{}`; {}", var, id.0, id.0, hint)));
            }
        },
        other => {
            return Err(err(format!(
                "each {} in ...: `each` repeats over a list, but was given {}",
                var,
                describe(other)
            )))
        }
    };

    let declared = declared_names(&e.body);
    let mut out = Vec::new();
    for (k, item) in items.iter().enumerate() {
        let value = match &item.node {
            v @ (StyleValue::String(_)
            | StyleValue::Number { .. }
            | StyleValue::Keyword(_)
            | StyleValue::Identifier(_)) => v.clone(),
            other => {
                return Err(err(format!(
                    "each {} in ...: item {} is {}; items must be strings or numbers",
                    var,
                    k + 1,
                    describe(other)
                )))
            }
        };
        let mut vars: Vars = HashMap::new();
        vars.insert(
            var.clone(),
            Bound { value, span: item.span.clone(), item: k + 1, is_index: false, header: header.clone() },
        );
        if let Some(ix) = &e.index {
            vars.insert(
                ix.node.0.clone(),
                Bound {
                    value: StyleValue::Number { value: k as f64, unit: None },
                    span: ix.span.clone(),
                    item: k + 1,
                    is_index: true,
                    header: header.clone(),
                },
            );
        }
        let renames: HashMap<String, String> =
            declared.iter().map(|n| (n.clone(), format!("{}_{}", n, k + 1))).collect();
        let mut errors = Vec::new();
        let body: Stmts = e
            .body
            .iter()
            .cloned()
            .map(|s| rename_stmt(s, &renames))
            .map(|s| subst_stmt(s, &vars, &mut errors))
            .collect();
        if let Some(first) = errors.into_iter().next() {
            return Err(err(first));
        }
        let mut body = expand_in(body, lists, scope)?;
        crate::parser::join_constraint_parts(&mut body);
        out.extend(body);
    }
    Ok(out)
}

// ---- names declared inside a body ------------------------------------

fn declared_names(stmts: &[Spanned<Statement>]) -> HashSet<String> {
    fn go(stmts: &[Spanned<Statement>], out: &mut HashSet<String>) {
        for st in stmts {
            match &st.node {
                Statement::Shape(s) => {
                    if let Some(n) = &s.name {
                        out.insert(n.node.0.clone());
                    }
                    if let ShapeType::Path(p) = &s.shape_type.node {
                        if let Some(n) = &p.name {
                            out.insert(n.node.0.clone());
                        }
                    }
                }
                Statement::Layout(l) => {
                    if let Some(n) = &l.name {
                        out.insert(n.node.0.clone());
                    }
                    go(&l.children, out);
                }
                Statement::Group(g) => {
                    if let Some(n) = &g.name {
                        out.insert(n.node.0.clone());
                    }
                    go(&g.children, out);
                }
                Statement::TemplateInstance(i) => {
                    out.insert(i.instance_name.node.0.clone());
                }
                Statement::Connection(cs) => {
                    for c in cs {
                        if let Some(n) = &c.name {
                            out.insert(n.node.0.clone());
                        }
                    }
                }
                Statement::Constrain(d) => {
                    if let Some(n) = &d.name {
                        out.insert(n.node.0.clone());
                    }
                }
                Statement::AnchorDecl(a) => {
                    out.insert(a.name.node.0.clone());
                }
                Statement::Each(e) => go(&e.body, out),
                _ => {}
            }
        }
    }
    let mut out = HashSet::new();
    go(stmts, &mut out);
    out
}

// ---- renaming --------------------------------------------------------

fn rn_id(id: &mut Spanned<Identifier>, m: &HashMap<String, String>) {
    if let Some(n) = m.get(&id.node.0) {
        id.node = Identifier::new(n.as_str());
    }
}

/// `x` or `x.part`: the first segment is the element.
fn rn_dotted(s: &str, m: &HashMap<String, String>) -> String {
    let (head, rest) = match s.split_once('.') {
        Some((h, r)) => (h, Some(r)),
        None => (s, None),
    };
    match (m.get(head), rest) {
        (Some(n), Some(r)) => format!("{}.{}", n, r),
        (Some(n), None) => n.clone(),
        _ => s.to_string(),
    }
}

fn rn_path(p: &mut Spanned<ElementPath>, m: &HashMap<String, String>) {
    if let Some(first) = p.node.segments.first_mut() {
        rn_id(first, m);
    }
}

fn rn_prop(p: &mut PropertyRef, m: &HashMap<String, String>) {
    rn_path(&mut p.element, m);
}

fn rn_mods(mods: &mut [Spanned<StyleModifier>], m: &HashMap<String, String>) {
    for md in mods {
        let refers = match &md.node.key.node {
            StyleKey::CaptionOf => true,
            StyleKey::Custom(k) => matches!(k.as_str(), "clip" | "drawn" | "through" | "via"),
            _ => false,
        };
        if !refers {
            continue;
        }
        match &mut md.node.value.node {
            StyleValue::Identifier(id) => id.0 = rn_dotted(&id.0, m),
            StyleValue::IdentifierList(ids) => {
                for id in ids {
                    id.0 = rn_dotted(&id.0, m);
                }
            }
            StyleValue::List(items) => {
                for it in items {
                    if let StyleValue::Identifier(id) = &mut it.node {
                        id.0 = rn_dotted(&id.0, m);
                    }
                }
            }
            _ => {}
        }
    }
}

fn rename_stmt(st: Spanned<Statement>, m: &HashMap<String, String>) -> Spanned<Statement> {
    let span = st.span.clone();
    let node = match st.node {
        Statement::Shape(mut s) => {
            if let Some(n) = &mut s.name {
                rn_id(n, m);
            }
            if let ShapeType::Path(p) = &mut s.shape_type.node {
                if let Some(n) = &mut p.name {
                    rn_id(n, m);
                }
                rn_mods(&mut p.modifiers, m);
            }
            rn_mods(&mut s.modifiers, m);
            Statement::Shape(s)
        }
        Statement::Layout(mut l) => {
            if let Some(n) = &mut l.name {
                rn_id(n, m);
            }
            rn_mods(&mut l.modifiers, m);
            l.children = l.children.into_iter().map(|c| rename_stmt(c, m)).collect();
            Statement::Layout(l)
        }
        Statement::Group(mut g) => {
            if let Some(n) = &mut g.name {
                rn_id(n, m);
            }
            rn_mods(&mut g.modifiers, m);
            g.children = g.children.into_iter().map(|c| rename_stmt(c, m)).collect();
            Statement::Group(g)
        }
        Statement::TemplateInstance(mut i) => {
            rn_id(&mut i.instance_name, m);
            Statement::TemplateInstance(i)
        }
        Statement::Connection(mut cs) => {
            for c in &mut cs {
                rn_id(&mut c.from.element, m);
                rn_id(&mut c.to.element, m);
                if let Some(n) = &mut c.name {
                    rn_id(n, m);
                }
                rn_mods(&mut c.modifiers, m);
            }
            Statement::Connection(cs)
        }
        Statement::Constrain(mut d) => {
            rename_expr(&mut d.expr, m);
            if let Some(n) = &mut d.name {
                rn_id(n, m);
            }
            Statement::Constrain(ConstrainDecl { expr: d.expr, name: d.name })
        }
        Statement::AnchorDecl(mut a) => {
            rn_id(&mut a.name, m);
            match &mut a.position {
                AnchorPosition::PropertyRef(p) => rn_prop(p, m),
                AnchorPosition::PropertyRefWithOffset { prop_ref, .. } => rn_prop(prop_ref, m),
            }
            Statement::AnchorDecl(a)
        }
        Statement::Constraint(mut c) => {
            rn_id(&mut c.subject, m);
            if let Some(a) = &mut c.anchor {
                rn_id(a, m);
            }
            Statement::Constraint(c)
        }
        Statement::Each(mut e) => {
            e.body = e.body.into_iter().map(|c| rename_stmt(c, m)).collect();
            Statement::Each(e)
        }
        other => other,
    };
    Spanned::new(node, span)
}

fn rename_expr(e: &mut ConstraintExpr, m: &HashMap<String, String>) {
    match e {
        ConstraintExpr::Equal { left, right } | ConstraintExpr::EqualWithOffset { left, right, .. } => {
            rn_prop(left, m);
            rn_prop(right, m);
        }
        ConstraintExpr::Constant { left, .. }
        | ConstraintExpr::GreaterOrEqual { left, .. }
        | ConstraintExpr::LessOrEqual { left, .. } => rn_prop(left, m),
        // Operands keep their dots until the copy is joined (`row.cell`).
        ConstraintExpr::Midpoint { target, a, b, .. } => {
            rn_prop(target, m);
            a.node.0 = rn_dotted(&a.node.0, m);
            b.node.0 = rn_dotted(&b.node.0, m);
        }
        ConstraintExpr::Contains { container, elements, .. } => {
            for id in std::iter::once(container).chain(elements.iter_mut()) {
                id.node.0 = rn_dotted(&id.node.0, m);
            }
        }
    }
}

// ---- substituting the loop variables ---------------------------------

fn interpolate(text: &str, vars: &Vars) -> String {
    let mut out = text.to_string();
    for (k, b) in vars {
        let v = &b.value;
        let needle = format!("{{{}}}", k);
        if out.contains(&needle) {
            let value = match v {
                StyleValue::String(s) | StyleValue::Keyword(s) => s.clone(),
                StyleValue::Number { value, .. } => format!("{}", value),
                StyleValue::Identifier(id) => id.0.clone(),
                _ => continue,
            };
            out = out.replace(&needle, &value);
        }
        // `{i+1}` / `{i-1}`: a number variable plus or minus a whole number
        // (1-based numbering from the 0-based index).
        if let StyleValue::Number { value, .. } = v {
            for sign in ['+', '-'] {
                let prefix = format!("{{{}{}", k, sign);
                while let Some(start) = out.find(&prefix) {
                    let rest = &out[start + prefix.len()..];
                    let Some(end) = rest.find('}') else { break };
                    let Ok(n) = rest[..end].trim().parse::<f64>() else { break };
                    let r = if sign == '+' { value + n } else { value - n };
                    let end_abs = start + prefix.len() + end + 1;
                    out.replace_range(start..end_abs, &format!("{}", r));
                }
            }
        }
    }
    out
}

/// The name a loop variable stands for where a name goes (an endpoint, a
/// constraint, `contains`, an anchor position). `None` when `head` is not a
/// loop variable; an error when it is one whose value is not a name.
fn name_of(head: &str, written: &str, vars: &Vars, errors: &mut Vec<String>) -> Option<(String, Span)> {
    let b = vars.get(head)?;
    let given = match &b.value {
        StyleValue::Identifier(id) => return Some((id.0.clone(), b.span.clone())),
        StyleValue::Keyword(k) => return Some((k.clone(), b.span.clone())),
        _ if b.is_index => format!(
            "`{}` is the item's position (a number), not an element; name the item instead: each it, {} in [a, b] {{ it -> ... }}",
            head, head
        ),
        StyleValue::String(s) => format!(
            "item {} is the string \"{}\"; a name goes without quotes: [{}, ...]",
            b.item, s, s
        ),
        other => format!("item {} is {}, not the name of an element", b.item, describe(other)),
    };
    errors.push(format!("{}: `{}` stands where a name goes, but {}", b.header, written, given));
    None
}

/// A dotted name (`n`, `n.top`) with a loop variable at its head replaced.
fn subst_dotted(id: &mut Spanned<Identifier>, vars: &Vars, errors: &mut Vec<String>) {
    let written = id.node.0.clone();
    if let Some((target, span)) = name_of(head_of(&written).0, &written, vars, errors) {
        id.node = Identifier::new(splice_dotted(&written, &target));
        id.span = span;
    }
}

fn subst_path(p: &mut Spanned<ElementPath>, vars: &Vars, errors: &mut Vec<String>) {
    let Some(first) = p.node.segments.first() else { return };
    let written: Vec<&str> = p.node.segments.iter().map(|s| s.node.0.as_str()).collect();
    let written = written.join(".");
    if let Some((target, span)) = name_of(&first.node.0.clone(), &written, vars, errors) {
        p.node = splice_path(&p.node, &target, &span);
        // The caret of a "no such element" error lands on the list item.
        p.span = span;
    }
}

fn subst_prop(p: &mut PropertyRef, vars: &Vars, errors: &mut Vec<String>) {
    subst_path(&mut p.element, vars, errors);
}

/// `n.top -> hub`: the endpoint's element is the head, the rest its anchor.
fn subst_endpoint(r: &mut AnchorReference, vars: &Vars, errors: &mut Vec<String>) {
    let written = match &r.anchor {
        Some(a) => format!("{}.{}", r.element.node.0, a.node),
        None => r.element.node.0.clone(),
    };
    let Some((target, span)) = name_of(&r.element.node.0.clone(), &written, vars, errors) else { return };
    let (head, rest) = head_of(&target);
    if let Some(rest) = rest {
        // `c: board.led` -> element `board`, anchor `led[.top]`
        let anchor = match &r.anchor {
            Some(a) => format!("{}.{}", rest, a.node),
            None => rest.to_string(),
        };
        let aspan = r.anchor.as_ref().map(|a| a.span.clone()).unwrap_or_else(|| span.clone());
        r.anchor = Some(Spanned::new(anchor, aspan));
    }
    r.element = Spanned::new(Identifier::new(head), span);
}

fn subst_expr(e: &mut ConstraintExpr, vars: &Vars, errors: &mut Vec<String>) {
    match e {
        ConstraintExpr::Equal { left, right } | ConstraintExpr::EqualWithOffset { left, right, .. } => {
            subst_prop(left, vars, errors);
            subst_prop(right, vars, errors);
        }
        ConstraintExpr::Constant { left, .. }
        | ConstraintExpr::GreaterOrEqual { left, .. }
        | ConstraintExpr::LessOrEqual { left, .. } => subst_prop(left, vars, errors),
        ConstraintExpr::Midpoint { target, a, b, .. } => {
            subst_prop(target, vars, errors);
            subst_dotted(a, vars, errors);
            subst_dotted(b, vars, errors);
        }
        ConstraintExpr::Contains { container, elements, .. } => {
            for id in std::iter::once(container).chain(elements.iter_mut()) {
                subst_dotted(id, vars, errors);
            }
        }
    }
}

/// The name a loop variable holds, if it holds one.
fn bound_name(vars: &Vars, head: &str) -> Option<String> {
    vars.get(head).and_then(|b| match &b.value {
        StyleValue::Identifier(t) => Some(t.0.clone()),
        StyleValue::Keyword(k) => Some(k.clone()),
        _ => None,
    })
}

fn subst_value(v: &mut Spanned<StyleValue>, vars: &Vars) {
    match &mut v.node {
        StyleValue::Identifier(id) => {
            if let Some(b) = vars.get(&id.0) {
                v.node = b.value.clone();
            } else if let (head, Some(_)) = head_of(&id.0) {
                // `via: [n.dot]`, `position: n.top`: a name with a part.
                if let Some(target) = bound_name(vars, head) {
                    id.0 = splice_dotted(&id.0, &target);
                }
            }
        }
        StyleValue::IdentifierList(ids) => {
            for id in ids {
                if let Some(target) = bound_name(vars, head_of(&id.0).0) {
                    id.0 = splice_dotted(&id.0, &target);
                }
            }
        }
        StyleValue::String(s) if s.contains('{') => *s = interpolate(s, vars),
        StyleValue::List(items) => {
            for it in items {
                subst_value(it, vars);
            }
        }
        _ => {}
    }
}

fn subst_mods(mods: &mut [Spanned<StyleModifier>], vars: &Vars) {
    for m in mods {
        subst_value(&mut m.node.value, vars);
    }
}

fn subst_stmt(st: Spanned<Statement>, vars: &Vars, errors: &mut Vec<String>) -> Spanned<Statement> {
    let span = st.span.clone();
    let node = match st.node {
        Statement::Shape(mut s) => {
            if let ShapeType::Text { content } = &mut s.shape_type.node {
                *content = interpolate(content, vars);
            }
            if let ShapeType::Path(p) = &mut s.shape_type.node {
                subst_mods(&mut p.modifiers, vars);
            }
            subst_mods(&mut s.modifiers, vars);
            Statement::Shape(s)
        }
        Statement::Layout(mut l) => {
            subst_mods(&mut l.modifiers, vars);
            l.children = l.children.into_iter().map(|c| subst_stmt(c, vars, errors)).collect();
            Statement::Layout(l)
        }
        Statement::Group(mut g) => {
            subst_mods(&mut g.modifiers, vars);
            g.children = g.children.into_iter().map(|c| subst_stmt(c, vars, errors)).collect();
            Statement::Group(g)
        }
        Statement::Connection(mut cs) => {
            for c in &mut cs {
                subst_endpoint(&mut c.from, vars, errors);
                subst_endpoint(&mut c.to, vars, errors);
                subst_mods(&mut c.modifiers, vars);
            }
            Statement::Connection(cs)
        }
        Statement::Constrain(mut d) => {
            subst_expr(&mut d.expr, vars, errors);
            Statement::Constrain(d)
        }
        Statement::Constraint(mut c) => {
            subst_dotted(&mut c.subject, vars, errors);
            if let Some(a) = &mut c.anchor {
                subst_dotted(a, vars, errors);
            }
            subst_mods(&mut c.modifiers, vars);
            Statement::Constraint(c)
        }
        Statement::AnchorDecl(mut a) => {
            match &mut a.position {
                AnchorPosition::PropertyRef(p) => subst_prop(p, vars, errors),
                AnchorPosition::PropertyRefWithOffset { prop_ref, .. } => subst_prop(prop_ref, vars, errors),
            }
            Statement::AnchorDecl(a)
        }
        Statement::TemplateInstance(mut i) => {
            for (_, v) in &mut i.arguments {
                subst_value(v, vars);
            }
            Statement::TemplateInstance(i)
        }
        Statement::Each(mut e) => {
            // The inner block's own names shadow the outer ones.
            let mut inner = vars.clone();
            inner.remove(&e.var.node.0);
            if let Some(ix) = &e.index {
                inner.remove(&ix.node.0);
            }
            e.body = e.body.into_iter().map(|c| subst_stmt(c, &inner, errors)).collect();
            Statement::Each(e)
        }
        other => other,
    };
    Spanned::new(node, span)
}

// ---- a loop variable that is also an element name ---------------------

/// Where the body uses `name` as an element name, if anywhere (for the
/// message): a connection endpoint, a constraint, `contains`, an anchor
/// position. Nested blocks that bind `name` again are skipped.
fn used_as_name(stmts: &[Spanned<Statement>], name: &str) -> Option<String> {
    let is = |s: &str| head_of(s).0 == name;
    let in_prop = |p: &PropertyRef| p.element.node.segments.first().is_some_and(|f| f.node.0 == name);
    for st in stmts {
        let hit = match &st.node {
            Statement::Connection(cs) => cs.iter().find_map(|c| {
                [&c.from, &c.to].into_iter().find(|r| r.element.node.0 == name).map(|_| {
                    format!("{} -> {}", c.from.element.node.0, c.to.element.node.0)
                })
            }),
            Statement::Constrain(d) => {
                let hit = match &d.expr {
                    ConstraintExpr::Equal { left, right } | ConstraintExpr::EqualWithOffset { left, right, .. } => {
                        in_prop(left) || in_prop(right)
                    }
                    ConstraintExpr::Constant { left, .. }
                    | ConstraintExpr::GreaterOrEqual { left, .. }
                    | ConstraintExpr::LessOrEqual { left, .. } => in_prop(left),
                    ConstraintExpr::Midpoint { target, a, b, .. } => in_prop(target) || is(&a.node.0) || is(&b.node.0),
                    ConstraintExpr::Contains { container, elements, .. } => {
                        is(&container.node.0) || elements.iter().any(|e| is(&e.node.0))
                    }
                };
                hit.then(|| format!("constrain ... {} ...", name))
            }
            Statement::Constraint(c) => (is(&c.subject.node.0) || c.anchor.as_ref().is_some_and(|a| is(&a.node.0)))
                .then(|| format!("place {} ...", name)),
            Statement::AnchorDecl(a) => {
                let p = match &a.position {
                    AnchorPosition::PropertyRef(p) => p,
                    AnchorPosition::PropertyRefWithOffset { prop_ref, .. } => prop_ref,
                };
                in_prop(p).then(|| format!("anchor {} [position: {}...]", a.name.node.0, name))
            }
            Statement::Layout(l) => used_as_name(&l.children, name),
            Statement::Group(g) => used_as_name(&g.children, name),
            Statement::Each(e) if e.var.node.0 != name && e.index.as_ref().is_none_or(|i| i.node.0 != name) => {
                used_as_name(&e.body, name)
            }
            _ => None,
        };
        if hit.is_some() {
            return hit;
        }
    }
    None
}
