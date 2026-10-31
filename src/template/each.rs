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
    AnchorPosition, ConstrainDecl, ConstraintExpr, EachBlock, ElementPath, Identifier, PropertyRef,
    ShapeType, Spanned, Statement, StyleKey, StyleModifier, StyleValue,
};

use super::registry::TemplateError;

type Stmts = Vec<Spanned<Statement>>;
type Vars = HashMap<String, StyleValue>;

/// Expand every `each` in `stmts`. `lists` are the values `each x in NAME`
/// can name (a template's parameters); other kinds of value are ignored.
pub fn expand_each(stmts: Stmts, lists: &HashMap<String, StyleValue>) -> Result<Stmts, TemplateError> {
    let mut out = Vec::with_capacity(stmts.len());
    for st in stmts {
        let span = st.span.clone();
        match st.node {
            Statement::Each(e) => out.extend(expand_block(e, lists)?),
            Statement::Layout(mut l) => {
                l.children = expand_each(l.children, lists)?;
                out.push(Spanned::new(Statement::Layout(l), span));
            }
            Statement::Group(mut g) => {
                g.children = expand_each(g.children, lists)?;
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

fn expand_block(e: EachBlock, lists: &HashMap<String, StyleValue>) -> Result<Stmts, TemplateError> {
    let var = e.var.node.0.clone();
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
        vars.insert(var.clone(), value);
        if let Some(ix) = &e.index {
            vars.insert(ix.node.0.clone(), StyleValue::Number { value: k as f64, unit: None });
        }
        let renames: HashMap<String, String> =
            declared.iter().map(|n| (n.clone(), format!("{}_{}", n, k + 1))).collect();
        let body: Stmts = e
            .body
            .iter()
            .cloned()
            .map(|s| rename_stmt(s, &renames))
            .map(|s| subst_stmt(s, &vars))
            .collect();
        out.extend(expand_each(body, lists)?);
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
        ConstraintExpr::Midpoint { target, a, b, .. } => {
            rn_prop(target, m);
            rn_id(a, m);
            rn_id(b, m);
        }
        ConstraintExpr::Contains { container, elements, .. } => {
            for id in std::iter::once(container).chain(elements.iter_mut()) {
                rn_id(id, m);
            }
        }
    }
}

// ---- substituting the loop variables ---------------------------------

fn interpolate(text: &str, vars: &Vars) -> String {
    let mut out = text.to_string();
    for (k, v) in vars {
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
    }
    out
}

fn subst_value(v: &mut Spanned<StyleValue>, vars: &Vars) {
    match &mut v.node {
        StyleValue::Identifier(id) => {
            if let Some(p) = vars.get(&id.0) {
                v.node = p.clone();
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

fn subst_stmt(st: Spanned<Statement>, vars: &Vars) -> Spanned<Statement> {
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
            l.children = l.children.into_iter().map(|c| subst_stmt(c, vars)).collect();
            Statement::Layout(l)
        }
        Statement::Group(mut g) => {
            subst_mods(&mut g.modifiers, vars);
            g.children = g.children.into_iter().map(|c| subst_stmt(c, vars)).collect();
            Statement::Group(g)
        }
        Statement::Connection(mut cs) => {
            for c in &mut cs {
                subst_mods(&mut c.modifiers, vars);
            }
            Statement::Connection(cs)
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
            e.body = e.body.into_iter().map(|c| subst_stmt(c, &inner)).collect();
            Statement::Each(e)
        }
        other => other,
    };
    Spanned::new(node, span)
}
