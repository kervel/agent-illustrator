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

use super::names::head_of;
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
    let by = e.key.as_ref().map(|k| format!(" by {}", k.node.0)).unwrap_or_default();
    let header = match &e.index {
        Some(ix) => format!("each {}, {}{} in ...", var, ix.node.0, by),
        None => format!("each {}{} in ...", var, by),
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

    // A name whose index this block binds (`rect lane[l]`, `rect p[m.id]`)
    // is already one per item: it keeps that name. Every other declared name
    // gets the item's key: the `by` field of each record (`each r by id`),
    // else its position (x -> x_1, x_2, ...).
    let index_vars: Vec<&str> = std::iter::once(var.as_str()).chain(e.index.as_ref().map(|ix| ix.node.0.as_str())).collect();
    let declared: HashSet<String> =
        declared_names(&e.body).into_iter().filter(|n| !indexed_by(n, &index_vars)).collect();
    let keys = item_keys(&items, e.key.as_ref().map(|k| k.node.0.as_str()), &header)?;
    // declared name -> the item that made it, to catch two items making one name
    let mut made: HashMap<String, usize> = HashMap::new();
    let mut out = Vec::new();
    let mut problems: Vec<Problem> = Vec::new();
    for (k, item) in items.iter().enumerate() {
        let value = match &item.node {
            v @ (StyleValue::String(_)
            | StyleValue::Number { .. }
            | StyleValue::Keyword(_)
            | StyleValue::Identifier(_)
            | StyleValue::Record(_)) => v.clone(),
            other => {
                return Err(err(format!(
                    "each {} in ...: item {} is {}; items are strings, numbers, names or records ({{from: a, label: \"x\"}})",
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
            declared.iter().map(|n| (n.clone(), format!("{}_{}", n, keys[k]))).collect();
        let body: Stmts = e
            .body
            .iter()
            .cloned()
            .map(|s| rename_stmt(s, &renames))
            .map(|s| subst_stmt(s, &vars, &mut problems))
            .collect();
        if !problems.is_empty() {
            // Keep going: a field missing from some items is reported for all of them.
            continue;
        }
        let mut names: Vec<String> = declared_names(&body).into_iter().collect();
        names.sort();
        for n in names {
            if let Some(first) = made.insert(n.clone(), k + 1) {
                if first != k + 1 {
                    problems.push(Problem::Msg(format!(
                        "{}: items {} and {} both make an element called `{}`; a name like p[m.id] must differ per item",
                        header, first, k + 1, n
                    )));
                }
            }
        }
        if !problems.is_empty() {
            continue;
        }
        let mut body = expand_in(body, lists, scope)?;
        crate::parser::join_constraint_parts(&mut body);
        out.extend(body);
    }
    if !problems.is_empty() {
        return Err(err(report(problems, &items)));
    }
    Ok(out)
}

/// `lane_{l}`, `p_{m.id}`, `p_{i+1}`: a name with an index that one of `vars`
/// binds (the indexed-name pass writes `x[e]` as `x_{e}`).
fn indexed_by(name: &str, vars: &[&str]) -> bool {
    let mut rest = name;
    while let Some(start) = rest.find("_{") {
        let after = &rest[start + 2..];
        let Some(end) = after.find('}') else { return false };
        let expr = &after[..end];
        let head = expr.split(['.', '+', '-']).next().unwrap_or("");
        if vars.contains(&head) {
            return true;
        }
        rest = &after[end + 1..];
    }
    false
}

/// What each item's copies are called: with `by FIELD`, that field of every
/// record (`each r by id in [{id: login, ...}]` -> `p_login`); otherwise
/// their positions counted from 1.
fn item_keys(items: &[Spanned<StyleValue>], by: Option<&str>, header: &str) -> Result<Vec<String>, TemplateError> {
    let Some(field) = by else {
        return Ok((1..=items.len()).map(|k| k.to_string()).collect());
    };
    let mut keys: Vec<String> = Vec::with_capacity(items.len());
    for (k, it) in items.iter().enumerate() {
        let value = match &it.node {
            StyleValue::Record(fields) => fields.iter().find(|(f, _)| f.node.0 == field).map(|(_, v)| v.node.clone()),
            _ => {
                return Err(err(format!(
                    "{}: item {} is not a record; `by {}` names each copy by a field, so every item needs one: {{{}: a, ...}}",
                    header,
                    k + 1,
                    field,
                    field
                )))
            }
        };
        let Some(value) = value else {
            return Err(err(format!(
                "{}: item {} has no `{}`; `by {}` names each copy by it, so every item needs one",
                header,
                k + 1,
                field,
                field
            )));
        };
        let key = match value {
            StyleValue::Identifier(id) => Some(id.0),
            StyleValue::Keyword(w) => Some(w),
            StyleValue::Number { value, .. } if value >= 0.0 && value.fract() == 0.0 => Some(format!("{}", value as u64)),
            _ => None,
        }
        .filter(|w| !w.is_empty() && w.chars().all(|c| c.is_alphanumeric() || c == '_'));
        let Some(key) = key else {
            return Err(err(format!(
                "{}: item {}: `{}` must be a plain word (letters, digits, _) without quotes, because it becomes part of a name ({{{}: am}} makes x_am)",
                header,
                k + 1,
                field,
                field
            )));
        };
        if let Some(first) = keys.iter().position(|x| *x == key) {
            return Err(err(format!(
                "{}: items {} and {} both have {} `{}`; it names their copies, so each must be different",
                header,
                first + 1,
                k + 1,
                field,
                key
            )));
        }
        keys.push(key);
    }
    Ok(keys)
}

/// Something wrong with how the body uses the loop variables.
enum Problem {
    Msg(String),
    /// `m.f` where item `item` (1-based) has no field `f`.
    Missing { header: String, written: String, field: String, item: usize },
}

/// The fields of a record item.
fn field_names(item: &StyleValue) -> Vec<String> {
    match item {
        StyleValue::Record(fields) => fields.iter().map(|(k, _)| k.node.0.clone()).collect(),
        _ => Vec::new(),
    }
}

/// One message for everything that went wrong, every item considered.
fn report(problems: Vec<Problem>, items: &[Spanned<StyleValue>]) -> String {
    let mut lines: Vec<String> = Vec::new();
    // field -> (header, written, items missing it), in first-seen order
    let mut missing: Vec<(String, String, String, Vec<usize>)> = Vec::new();
    for p in problems {
        match p {
            Problem::Msg(m) => {
                if !lines.contains(&m) {
                    lines.push(m);
                }
            }
            Problem::Missing { header, written, field, item } => {
                match missing.iter_mut().find(|(_, w, f, _)| *w == written && *f == field) {
                    Some((_, _, _, its)) => {
                        if !its.contains(&item) {
                            its.push(item)
                        }
                    }
                    None => missing.push((header, written, field, vec![item])),
                }
            }
        }
    }
    let records = items.iter().filter(|i| matches!(i.node, StyleValue::Record(_))).count();
    for (header, written, field, its) in missing {
        if its.len() == records {
            // No item has it: a misspelt field.
            let first = its[0];
            let fields = field_names(&items[first - 1].node);
            let close = fields
                .iter()
                .filter(|f| crate::layout::levenshtein_distance(f, &field) <= 2)
                .min_by_key(|f| crate::layout::levenshtein_distance(f, &field));
            let hint = close.map(|c| format!("; did you mean `{}`?", c)).unwrap_or_default();
            lines.push(format!(
                "{}: `{}`: no item has a field `{}` (item {} has: {}){}",
                header,
                written,
                field,
                first,
                fields.join(", "),
                hint
            ));
        } else {
            let names: Vec<String> = its.iter().map(|i| i.to_string()).collect();
            let which = if its.len() == 1 {
                format!("item {} has", names[0])
            } else {
                let (last, rest) = names.split_last().unwrap();
                format!("items {} and {} have", rest.join(", "), last)
            };
            lines.push(format!(
                "{}: `{}`: {} no field `{}`, which the other items have; a field cannot be left out, so add `{}: ...` there",
                header, written, which, field, field
            ));
        }
    }
    lines.join("\n")
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

/// What `written` (`m`, `m.from`, `m.from.top`, `n.top`) means when its head
/// is a loop variable: the value, the rest of the name after it, where the
/// value was written, and how to call it in a message. `None` when the head
/// is not a loop variable, or on a problem (pushed).
fn lookup(written: &str, vars: &Vars, problems: &mut Vec<Problem>) -> Option<(StyleValue, Option<String>, Span, String)> {
    let (head, rest) = head_of(written);
    let b = vars.get(head)?;
    match &b.value {
        StyleValue::Record(fields) => {
            let names: Vec<String> = fields.iter().map(|(k, _)| k.node.0.clone()).collect();
            let Some(rest) = rest else {
                problems.push(Problem::Msg(format!(
                    "{}: `{}` is a whole record; pick a field: {}",
                    b.header,
                    head,
                    names.iter().map(|n| format!("{}.{}", head, n)).collect::<Vec<_>>().join(", ")
                )));
                return None;
            };
            let (field, after) = head_of(rest);
            match fields.iter().find(|(k, _)| k.node.0 == field) {
                Some((_, v)) => Some((
                    v.node.clone(),
                    after.map(str::to_string),
                    v.span.clone(),
                    format!("`{}.{}` (item {})", head, field, b.item),
                )),
                None => {
                    problems.push(Problem::Missing {
                        header: b.header.clone(),
                        written: format!("{}.{}", head, field),
                        field: field.to_string(),
                        item: b.item,
                    });
                    None
                }
            }
        }
        v => {
            let what = if b.is_index { format!("`{}`", head) } else { format!("item {}", b.item) };
            Some((v.clone(), rest.map(str::to_string), b.span.clone(), what))
        }
    }
}

/// The name `written` stands for where an element name goes, with its span
/// (the list item, or the record field, so a typo is shown where it is).
fn name_at(written: &str, vars: &Vars, problems: &mut Vec<Problem>) -> Option<(String, Span)> {
    let head = head_of(written).0;
    let b = vars.get(head)?;
    let (value, rest, span, what) = lookup(written, vars, problems)?;
    let name = match &value {
        StyleValue::Identifier(id) => id.0.clone(),
        StyleValue::Keyword(k) => k.clone(),
        _ => {
            let given = if b.is_index {
                format!(
                    "`{}` is the item's position (a number), not an element; name the item instead: each it, {} in [a, b] {{ it -> ... }}",
                    head, head
                )
            } else if let StyleValue::String(s) = &value {
                format!("{} is the string \"{}\"; a name goes without quotes: {}", what, s, s)
            } else {
                format!("{} is {}, not the name of an element", what, describe(&value))
            };
            problems.push(Problem::Msg(format!(
                "{}: `{}` stands where a name goes, but {}",
                b.header, written, given
            )));
            return None;
        }
    };
    let full = match rest {
        Some(r) => format!("{}.{}", name, r),
        None => name,
    };
    Some((full, span))
}

/// The text a value shows inside a string, if it has one.
fn as_text(v: &StyleValue) -> Option<String> {
    match v {
        StyleValue::String(s) | StyleValue::Keyword(s) => Some(s.clone()),
        StyleValue::Number { value, .. } => Some(format!("{}", value)),
        StyleValue::Identifier(id) => Some(id.0.clone()),
        // A bare hyphenated word (`diamond-filled`, `accent-1`) parses as a
        // colour token; inside a string it is the word as written.
        StyleValue::Color(c) => c.token_string().or_else(|| c.concrete_string().map(str::to_string)),
        _ => None,
    }
}

/// `{m}`, `{m.label}`, `{i}`, `{i+1}`, `{i-1}` filled in; any other `{...}`
/// stays as written (a template parameter, or text lint reports).
fn interpolate(text: &str, vars: &Vars, problems: &mut Vec<Problem>) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find('}') else {
            out.push_str(&rest[start..]);
            return out;
        };
        let expr = after[..end].trim();
        match fill(expr, vars, problems) {
            Some(t) => out.push_str(&t),
            None => out.push_str(&rest[start..start + end + 2]),
        }
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

fn fill(expr: &str, vars: &Vars, problems: &mut Vec<Problem>) -> Option<String> {
    // `{i+1}` / `{i-1}`: a number variable plus or minus a whole number.
    if let Some(at) = expr.find(['+', '-']) {
        let (name, n) = (expr[..at].trim(), expr[at + 1..].trim());
        let StyleValue::Number { value, .. } = vars.get(name)?.value else { return None };
        let n: f64 = n.parse().ok()?;
        return Some(format!("{}", if expr.as_bytes()[at] == b'+' { value + n } else { value - n }));
    }
    vars.get(head_of(expr).0)?;
    let (value, rest, _, what) = lookup(expr, vars, problems)?;
    if let Some(r) = rest {
        // `{m.from.top}`: no text for a part of an element
        problems.push(Problem::Msg(format!("`{{{}}}`: {} has no part `{}` to show as text", expr, what, r)));
        return None;
    }
    let text = as_text(&value);
    if text.is_none() {
        problems.push(Problem::Msg(format!(
            "`{{{}}}`: {} is {}, which has no text to show inside a string; give it a string value",
            expr,
            what,
            match &value {
                StyleValue::List(_) | StyleValue::IdentifierList(_) => "a list".to_string(),
                StyleValue::Call { name, .. } => format!("`{}(...)`", name),
                other => describe(other),
            }
        )));
    }
    text
}

/// A name as the author wrote it: `p_{i+1}` -> `p[i+1]`.
fn as_written(name: &str) -> String {
    name.replace("_{", "[").replace('}', "]")
}

/// `p_{i+1}`, `api.p_{m.row}`, `p_{m}`: the indices that this block's
/// variables bind, filled in (`p_3`, `api.p_2`, `p_query`). An index whose
/// variable is not bound here stays as written, for an inner `each`. Runs
/// before a name is split at its dots (`{m.row}` has one).
fn fill_index(name: &str, vars: &Vars, problems: &mut Vec<Problem>) -> String {
    fill_index_in(name, name, vars, problems)
}

/// `fill_index` for one part of a longer name (`p_{m.r}` of `api.p_{m.r}`),
/// which messages show whole.
fn fill_index_in(name: &str, whole: &str, vars: &Vars, problems: &mut Vec<Problem>) -> String {
    if !name.contains("_{") {
        return name.to_string();
    }
    let mut out = String::new();
    let mut rest = name;
    while let Some(start) = rest.find("_{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find('}') else {
            out.push_str(&rest[start..]);
            return out;
        };
        let expr = &after[..end];
        match index_value(expr, whole, vars, problems) {
            Some(k) => {
                out.push('_');
                out.push_str(&k);
            }
            None => out.push_str(&rest[start..start + end + 3]),
        }
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

/// The text one index stands for (`i+1` -> `3`, `m.row` -> `2`, `m.id` ->
/// `query`), or `None` when its variable is not bound here or on a problem.
fn index_value(expr: &str, name: &str, vars: &Vars, problems: &mut Vec<Problem>) -> Option<String> {
    let written = as_written(name);
    let (value, what, header) = if let Some(at) = expr.find(['+', '-']) {
        let var = &expr[..at];
        let b = vars.get(var)?;
        let n: f64 = expr[at + 1..].parse().ok()?;
        let StyleValue::Number { value, .. } = b.value else {
            problems.push(Problem::Msg(format!(
                "{}: `{}`: `{}` is {} (item {}), so `{}` is not a number; arithmetic in an index needs a number such as the position: each m, i in ... {{ p[i+1] }}",
                b.header, written, var, describe(&b.value), b.item, expr
            )));
            return None;
        };
        let v = if expr.as_bytes()[at] == b'+' { value + n } else { value - n };
        let what = if b.is_index { format!("`{}` (item {})", expr, b.item) } else { format!("`{}` for item {}", expr, b.item) };
        (StyleValue::Number { value: v, unit: None }, what, b.header.clone())
    } else {
        let b = vars.get(head_of(expr).0)?;
        let header = b.header.clone();
        let (value, rest, _, what) = lookup(expr, vars, problems)?;
        if rest.is_some() {
            problems.push(Problem::Msg(format!(
                "{}: `{}`: {} is {}, which cannot be an index; an index is a whole number or a name",
                header, written, what, describe(&value)
            )));
            return None;
        }
        (value, what, header)
    };
    let bad = |problems: &mut Vec<Problem>, why: String| {
        problems.push(Problem::Msg(format!("{}: `{}`: {}", header, written, why)));
        None
    };
    match &value {
        StyleValue::Number { value: v, .. } => {
            if v.fract() != 0.0 {
                bad(problems, format!("{} is {}; an index is a whole number, counted from 1", what, v))
            } else if *v < 0.0 {
                bad(
                    problems,
                    format!("{} is {}; copies are numbered from 1, so an index is never negative", what, v),
                )
            } else {
                Some(format!("{}", *v as u64))
            }
        }
        StyleValue::Identifier(id) if !id.0.is_empty() && id.0.chars().all(|c| c.is_alphanumeric() || c == '_') => {
            Some(id.0.clone())
        }
        StyleValue::Keyword(k) if k.chars().all(|c| c.is_alphanumeric() || c == '_') => Some(k.clone()),
        StyleValue::String(s) => bad(
            problems,
            format!("{} is the string \"{}\"; an index is a whole number or a name (letters, digits, _), written without quotes", what, s),
        ),
        other => bad(
            problems,
            format!("{} is {}; an index is a whole number or a name (letters, digits, _)", what, describe(other)),
        ),
    }
}

/// A dotted name (`n`, `n.top`, `m.from`) with a loop variable at its head replaced.
fn subst_dotted(id: &mut Spanned<Identifier>, vars: &Vars, problems: &mut Vec<Problem>) {
    if id.node.0.contains("_{") {
        id.node = Identifier::new(fill_index(&id.node.0, vars, problems));
    }
    if let Some((full, span)) = name_at(&id.node.0.clone(), vars, problems) {
        id.node = Identifier::new(full);
        id.span = span;
    }
}

fn subst_path(p: &mut Spanned<ElementPath>, vars: &Vars, problems: &mut Vec<Problem>) {
    if p.node.segments.is_empty() {
        return;
    }
    let whole: Vec<&str> = p.node.segments.iter().map(|s| s.node.0.as_str()).collect();
    let whole = whole.join(".");
    for seg in &mut p.node.segments {
        if seg.node.0.contains("_{") {
            seg.node = Identifier::new(fill_index_in(&seg.node.0, &whole, vars, problems));
        }
    }
    let written: Vec<&str> = p.node.segments.iter().map(|s| s.node.0.as_str()).collect();
    let written = written.join(".");
    if let Some((full, span)) = name_at(&written, vars, problems) {
        p.node = ElementPath {
            segments: full.split('.').map(|t| Spanned::new(Identifier::new(t), span.clone())).collect(),
        };
        // The caret of a "no such element" error lands on the list item.
        p.span = span;
    }
}

fn subst_prop(p: &mut PropertyRef, vars: &Vars, problems: &mut Vec<Problem>) {
    subst_path(&mut p.element, vars, problems);
}

/// `n.top -> hub`, `m.from -> m.to`: the head is the element, the rest its anchor.
fn subst_endpoint(r: &mut AnchorReference, vars: &Vars, problems: &mut Vec<Problem>) {
    let whole = match &r.anchor {
        Some(a) => format!("{}.{}", r.element.node.0, a.node),
        None => r.element.node.0.clone(),
    };
    if r.element.node.0.contains("_{") {
        r.element.node = Identifier::new(fill_index_in(&r.element.node.0, &whole, vars, problems));
    }
    if let Some(a) = &mut r.anchor {
        if a.node.contains("_{") {
            a.node = fill_index_in(&a.node, &whole, vars, problems);
        }
    }
    let written = match &r.anchor {
        Some(a) => format!("{}.{}", r.element.node.0, a.node),
        None => r.element.node.0.clone(),
    };
    let Some((full, span)) = name_at(&written, vars, problems) else { return };
    let (head, rest) = head_of(&full);
    r.anchor = rest.map(|rest| {
        let aspan = r.anchor.as_ref().map(|a| a.span.clone()).unwrap_or_else(|| span.clone());
        Spanned::new(rest.to_string(), aspan)
    });
    r.element = Spanned::new(Identifier::new(head), span);
}

fn subst_expr(e: &mut ConstraintExpr, vars: &Vars, problems: &mut Vec<Problem>) {
    match e {
        ConstraintExpr::Equal { left, right } | ConstraintExpr::EqualWithOffset { left, right, .. } => {
            subst_prop(left, vars, problems);
            subst_prop(right, vars, problems);
        }
        ConstraintExpr::Constant { left, .. }
        | ConstraintExpr::GreaterOrEqual { left, .. }
        | ConstraintExpr::LessOrEqual { left, .. } => subst_prop(left, vars, problems),
        ConstraintExpr::Midpoint { target, a, b, .. } => {
            subst_prop(target, vars, problems);
            subst_dotted(a, vars, problems);
            subst_dotted(b, vars, problems);
        }
        ConstraintExpr::Contains { container, elements, .. } => {
            for id in std::iter::once(container).chain(elements.iter_mut()) {
                subst_dotted(id, vars, problems);
            }
        }
    }
}

fn subst_value(v: &mut Spanned<StyleValue>, vars: &Vars, problems: &mut Vec<Problem>) {
    match &mut v.node {
        StyleValue::Identifier(id) => {
            if id.0.contains("_{") {
                id.0 = fill_index(&id.0, vars, problems);
            }
            let written = id.0.clone();
            if !vars.contains_key(head_of(&written).0) {
                return;
            }
            match lookup(&written, vars, problems) {
                // `label: m`, `label: m.label`, `fill: m.tint`: the value itself
                Some((value, None, _, _)) => v.node = value,
                // `position: n.top`, `via: [m.from.dot]`: a part of the element it names
                Some((StyleValue::Identifier(t), Some(r), _, _)) => id.0 = format!("{}.{}", t.0, r),
                Some((StyleValue::Keyword(t), Some(r), _, _)) => id.0 = format!("{}.{}", t, r),
                Some((other, Some(r), _, what)) => problems.push(Problem::Msg(format!(
                    "`{}`: {} is {}, which has no part `{}`",
                    written,
                    what,
                    describe(&other),
                    r
                ))),
                None => {}
            }
        }
        StyleValue::IdentifierList(ids) => {
            for id in ids {
                if id.0.contains("_{") {
                    id.0 = fill_index(&id.0, vars, problems);
                }
                if let Some((full, _)) = name_at(&id.0.clone(), vars, problems) {
                    id.0 = full;
                }
            }
        }
        StyleValue::String(s) if s.contains('{') => *s = interpolate(s, vars, problems),
        StyleValue::List(items) => {
            for it in items {
                subst_value(it, vars, problems);
            }
        }
        _ => {}
    }
}

fn subst_mods(mods: &mut [Spanned<StyleModifier>], vars: &Vars, problems: &mut Vec<Problem>) {
    for m in mods {
        subst_value(&mut m.node.value, vars, problems);
    }
}

fn subst_stmt(st: Spanned<Statement>, vars: &Vars, problems: &mut Vec<Problem>) -> Spanned<Statement> {
    let span = st.span.clone();
    let node = match st.node {
        Statement::Shape(mut s) => {
            if let Some(n) = &mut s.name {
                if n.node.0.contains("_{") {
                    n.node = Identifier::new(fill_index(&n.node.0, vars, problems));
                }
            }
            if let ShapeType::Text { content } = &mut s.shape_type.node {
                *content = interpolate(content, vars, problems);
            }
            if let ShapeType::Path(p) = &mut s.shape_type.node {
                subst_mods(&mut p.modifiers, vars, problems);
            }
            subst_mods(&mut s.modifiers, vars, problems);
            Statement::Shape(s)
        }
        Statement::Layout(mut l) => {
            subst_mods(&mut l.modifiers, vars, problems);
            l.children = l.children.into_iter().map(|c| subst_stmt(c, vars, problems)).collect();
            Statement::Layout(l)
        }
        Statement::Group(mut g) => {
            subst_mods(&mut g.modifiers, vars, problems);
            g.children = g.children.into_iter().map(|c| subst_stmt(c, vars, problems)).collect();
            Statement::Group(g)
        }
        Statement::Connection(mut cs) => {
            for c in &mut cs {
                subst_endpoint(&mut c.from, vars, problems);
                subst_endpoint(&mut c.to, vars, problems);
                subst_mods(&mut c.modifiers, vars, problems);
            }
            Statement::Connection(cs)
        }
        Statement::Constrain(mut d) => {
            subst_expr(&mut d.expr, vars, problems);
            Statement::Constrain(d)
        }
        Statement::Constraint(mut c) => {
            subst_dotted(&mut c.subject, vars, problems);
            if let Some(a) = &mut c.anchor {
                subst_dotted(a, vars, problems);
            }
            subst_mods(&mut c.modifiers, vars, problems);
            Statement::Constraint(c)
        }
        Statement::AnchorDecl(mut a) => {
            match &mut a.position {
                AnchorPosition::PropertyRef(p) => subst_prop(p, vars, problems),
                AnchorPosition::PropertyRefWithOffset { prop_ref, .. } => subst_prop(prop_ref, vars, problems),
            }
            Statement::AnchorDecl(a)
        }
        Statement::TemplateInstance(mut i) => {
            for (_, v) in &mut i.arguments {
                subst_value(v, vars, problems);
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
            e.body = e.body.into_iter().map(|c| subst_stmt(c, &inner, problems)).collect();
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
