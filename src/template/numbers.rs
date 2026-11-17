//! Numbers in a `constrain` written as names: `b.right + gap`, `>= min_w`,
//! `[padding_x: pad]`, `= m.w`. The parser keeps the names
//! (`ConstrainDecl::numbers`); a template parameter or an `each` item fills
//! them in, and a name nothing fills is an error before layout.

use crate::parser::ast::{
    ConstrainDecl, ConstraintExpr, ConstraintProperty, ElementPath, Identifier, NumberName, NumberSlot, PropertyRef,
    Spanned, Statement, StyleValue,
};

/// Fill in every name `value_of` knows (a name it does not know is kept).
/// A number adds to its slot; a name that stands for a side (`after:
/// x.bottom`) in `a.y = after + 4` makes it `a.y = x.bottom + 4`.
pub(crate) fn fill(
    decl: &mut ConstrainDecl,
    value_of: &mut dyn FnMut(&str) -> Option<StyleValue>,
    problems: &mut Vec<String>,
) {
    let mut pending = std::mem::take(&mut decl.numbers);
    let mut keep = Vec::new();
    let mut i = 0;
    while i < pending.len() {
        let n = pending[i].clone();
        i += 1;
        let Some(v) = value_of(&n.name.node) else {
            keep.push(n);
            continue;
        };
        match v {
            StyleValue::Number { value, .. } => add(&mut decl.expr, n.slot, if n.negate { -value } else { value }),
            StyleValue::Identifier(id) if n.slot == NumberSlot::Value && !n.negate => {
                match to_reference(&decl.expr, &id.0, &n.name.node) {
                    Ok(expr) => {
                        decl.expr = expr;
                        // What was added to the value is now the offset.
                        for m in pending.iter_mut().chain(keep.iter_mut()) {
                            if m.slot == NumberSlot::Value {
                                m.slot = NumberSlot::Offset;
                            }
                        }
                    }
                    Err(why) => problems.push(why),
                }
            }
            other => problems.push(format!(
                "`{}{}` in a constraint on `{}`: `{}` is {}, where a number goes",
                if n.negate { "-" } else { "" },
                n.name.node,
                subject(&decl.expr),
                n.name.node,
                describe(&other)
            )),
        }
    }
    decl.numbers = keep;
}

fn add(expr: &mut ConstraintExpr, slot: NumberSlot, v: f64) {
    let plus = |p: &mut Option<f64>| *p = Some(p.unwrap_or(0.0) + v);
    match (slot, expr) {
        (NumberSlot::Offset, ConstraintExpr::EqualWithOffset { offset, .. })
        | (NumberSlot::Offset, ConstraintExpr::Midpoint { offset, .. }) => *offset += v,
        (NumberSlot::Value, ConstraintExpr::Constant { value, .. })
        | (NumberSlot::Value, ConstraintExpr::GreaterOrEqual { value, .. })
        | (NumberSlot::Value, ConstraintExpr::LessOrEqual { value, .. }) => *value += v,
        (NumberSlot::Padding, ConstraintExpr::Contains { padding, .. }) => plus(padding),
        (NumberSlot::PaddingX, ConstraintExpr::Contains { padding_x, .. }) => plus(padding_x),
        (NumberSlot::PaddingY, ConstraintExpr::Contains { padding_y, .. }) => plus(padding_y),
        _ => {}
    }
}

/// `a.y = after + 4` where `after` is `x.bottom`: `a.y = x.bottom + 4`.
fn to_reference(expr: &ConstraintExpr, target: &str, written: &str) -> Result<ConstraintExpr, String> {
    let ConstraintExpr::Constant { left, value } = expr else {
        return Err(format!(
            "`{}` in a constraint on `{}`: `{}` is `{}`, a name, where a number goes",
            written,
            subject(expr),
            written,
            target
        ));
    };
    let Some((element, side)) = target.rsplit_once('.').filter(|(_, s)| super::names::SIDES.contains(s)) else {
        return Err(format!(
            "`{} = {}`: `{}` is `{}`, an element; say which side or centre: `= {}.{}` (or pass a side, `{}.{}`)",
            prop_text(left),
            written,
            written,
            target,
            written,
            spelling(&left.property.node),
            target,
            spelling(&left.property.node)
        ));
    };
    let property = super::names::property_of_side(side, &left.property.node).map_err(|why| format!("`{}`: {}", target, why))?;
    let span = left.element.span.clone();
    let path = ElementPath {
        segments: element.split('.').map(|s| Spanned::new(Identifier::new(s), span.clone())).collect(),
    };
    Ok(ConstraintExpr::EqualWithOffset {
        left: left.clone(),
        right: PropertyRef { element: Spanned::new(path, span.clone()), property: Spanned::new(property, span) },
        offset: *value,
    })
}

fn describe(v: &StyleValue) -> String {
    match v {
        StyleValue::String(s) => format!("the string \"{}\"", s),
        StyleValue::Keyword(k) => format!("the word `{}`", k),
        StyleValue::Identifier(i) => format!("the name `{}`", i.0),
        StyleValue::List(_) | StyleValue::IdentifierList(_) => "a list".to_string(),
        StyleValue::Record(_) => "a record".to_string(),
        StyleValue::Range { .. } => "a range".to_string(),
        _ => "not a number".to_string(),
    }
}

/// How a property is written (`center_x`, `gate_y`).
pub(crate) fn spelling(p: &ConstraintProperty) -> String {
    use ConstraintProperty as P;
    match p {
        P::X => "x".into(),
        P::Y => "y".into(),
        P::Width => "width".into(),
        P::Height => "height".into(),
        P::Left => "left".into(),
        P::Right => "right".into(),
        P::Top => "top".into(),
        P::Bottom => "bottom".into(),
        P::CenterX => "center_x".into(),
        P::CenterY => "center_y".into(),
        P::Center => "center".into(),
        P::AnchorX(n) => format!("{}_x", n),
        P::AnchorY(n) => format!("{}_y", n),
        P::Anchor(n) => n.clone(),
    }
}

fn prop_text(p: &PropertyRef) -> String {
    format!("{}.{}", p.element.node, spelling(&p.property.node))
}

/// What a constraint is about, for messages (`a.left`, `bg contains ...`).
fn subject(expr: &ConstraintExpr) -> String {
    match expr {
        ConstraintExpr::Equal { left, .. }
        | ConstraintExpr::EqualWithOffset { left, .. }
        | ConstraintExpr::Constant { left, .. }
        | ConstraintExpr::GreaterOrEqual { left, .. }
        | ConstraintExpr::LessOrEqual { left, .. } => prop_text(left),
        ConstraintExpr::Midpoint { target, .. } => prop_text(target),
        ConstraintExpr::Contains { container, .. } => format!("{} contains ...", container.node.0.replace('_', ".")),
    }
}

/// The message for a name that nothing filled in. `known` are the names
/// that could have (a template's parameters), for a did-you-mean.
pub(crate) fn unfilled(decl: &ConstrainDecl, n: &NumberName, known: &[String]) -> String {
    let close = known
        .iter()
        .filter(|k| crate::layout::levenshtein_distance(k, &n.name.node) <= 2)
        .min_by_key(|k| crate::layout::levenshtein_distance(k, &n.name.node));
    let hint = match close {
        Some(c) => format!("; did you mean `{}`?", c),
        None if !known.is_empty() => format!("; the template's parameters are {}", known.join(", ")),
        None => String::new(),
    };
    let element_hint = if n.slot == NumberSlot::Value && !n.name.node.contains('.') {
        if let ConstraintExpr::Constant { left, .. } = &decl.expr {
            format!(
                ". If `{}` is an element, say which side: `= {}.{}`",
                n.name.node,
                n.name.node,
                spelling(&left.property.node)
            )
        } else {
            String::new()
        }
    } else {
        String::new()
    };
    format!(
        "`{}` in a constraint on `{}`: nothing gives `{}` a value here; a number in a constraint may be a template parameter (`template \"t\" ({}: 20)`) or an `each` variable or field (`m.{}`){}{}",
        n.name.node,
        subject(&decl.expr),
        n.name.node,
        n.name.node,
        n.name.node,
        hint,
        element_hint
    )
}

/// The first name left unfilled anywhere in `stmts` (resolved templates),
/// as a message.
pub(crate) fn first_unfilled(stmts: &[Spanned<Statement>]) -> Option<String> {
    fn decl(d: &ConstrainDecl) -> Option<String> {
        d.numbers.first().map(|n| unfilled(d, n, &[]))
    }
    for st in stmts {
        let found = match &st.node {
            Statement::Constrain(d) => decl(d),
            Statement::NamedLayout { constraints, .. } => constraints.iter().find_map(|c| decl(&c.node)),
            Statement::Layout(l) => first_unfilled(&l.children),
            Statement::Group(g) => first_unfilled(&g.children),
            Statement::Label(inner) => first_unfilled(&[Spanned::new((**inner).clone(), st.span.clone())]),
            Statement::Keyframe(k) => k.operations.iter().find_map(|o| match &o.node {
                crate::parser::ast::KeyframeOp::Constrain(d) => decl(d),
                _ => None,
            }),
            _ => None,
        };
        if found.is_some() {
            return found;
        }
    }
    None
}
