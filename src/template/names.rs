//! Putting one name in place of another at the head of a reference.
//!
//! A name that stands for an element (an `each` loop variable, an
//! element-reference template parameter) is replaced at the head of every
//! reference that starts with it: `n.top` with `n` = `board.led` becomes
//! `board.led.top`. Shared by `each` expansion and template resolution so
//! both splice references the same way.

use crate::parser::ast::{ConstraintProperty, ElementPath, Identifier, Span, Spanned};

/// Words that name a side or the centre of an element: the end of a
/// reference to a point (`box.top`, `box.center`).
pub(crate) const SIDES: &[&str] =
    &["left", "right", "top", "bottom", "center", "center_x", "center_y", "horizontal_center", "vertical_center"];

/// `src.center_x` where `src` stands for `box.top`: a property of the point
/// a side names. The x of a top or bottom is the centre's x, the y of a left
/// or right is the centre's y, a side's own axis is the side itself, and the
/// whole point (`.center`) is the side. `Err` says why there is no such
/// property (`.width` of a side, the y of `center_x`).
pub(crate) fn property_of_side(side: &str, prop: &ConstraintProperty) -> Result<ConstraintProperty, String> {
    use ConstraintProperty as P;
    let s = P::from_str(side).unwrap_or(P::Center);
    let horizontal = matches!(prop, P::Left | P::Right | P::CenterX | P::X);
    let vertical = matches!(prop, P::Top | P::Bottom | P::CenterY | P::Y);
    if horizontal && s != P::CenterY {
        return Ok(s.x_component());
    }
    if vertical && s != P::CenterX {
        return Ok(s.y_component());
    }
    if *prop == P::Center && !matches!(s, P::CenterX | P::CenterY) {
        return Ok(s);
    }
    let what = match prop {
        P::Width => "width".to_string(),
        P::Height => "height".to_string(),
        P::Center => "point".to_string(),
        P::CenterX | P::Left | P::Right | P::X => "x".to_string(),
        P::CenterY | P::Top | P::Bottom | P::Y => "y".to_string(),
        P::Anchor(n) | P::AnchorX(n) | P::AnchorY(n) => format!("anchor `{}`", n),
    };
    Err(format!(
        "`{}` is one side of an element, which has no {}; a side has an x and a y (`.center_x`, `.center_y`), or pass the element itself",
        side, what
    ))
}

/// A property reference whose path ends in a side (`box.top` + `.center_x`,
/// after a parameter or loop variable put `box.top` there): the path without
/// the side, and the side's property. `Err` as for `property_of_side`.
pub(crate) fn fold_side(path: &mut ElementPath, prop: &mut Spanned<ConstraintProperty>) -> Result<(), String> {
    if path.segments.len() < 2 {
        return Ok(());
    }
    let last = path.segments.last().map(|s| s.node.0.clone()).unwrap_or_default();
    if !SIDES.contains(&last.as_str()) {
        return Ok(());
    }
    let element: Vec<&str> = path.segments[..path.segments.len() - 1].iter().map(|s| s.node.0.as_str()).collect();
    let element = element.join(".");
    prop.node = property_of_side(&last, &prop.node).map_err(|why| format!("`{}.{}`: {}", element, last, why))?;
    path.segments.pop();
    Ok(())
}

/// The first segment of a dotted name and the rest (`a.b.c` -> `a`, `b.c`).
pub(crate) fn head_of(name: &str) -> (&str, Option<&str>) {
    match name.split_once('.') {
        Some((h, r)) => (h, Some(r)),
        None => (name, None),
    }
}

/// `head.rest` with the head replaced by `target` (itself maybe dotted).
pub(crate) fn splice_dotted(name: &str, target: &str) -> String {
    match head_of(name).1 {
        Some(rest) => format!("{}.{}", target, rest),
        None => target.to_string(),
    }
}

/// An element path whose first segment is replaced by the segments of
/// `target`; the new segments carry `span` (where the target was written).
pub(crate) fn splice_path(path: &ElementPath, target: &str, span: &Span) -> ElementPath {
    let mut segments: Vec<Spanned<Identifier>> =
        target.split('.').map(|t| Spanned::new(Identifier::new(t), span.clone())).collect();
    segments.extend(path.segments.iter().skip(1).cloned());
    ElementPath { segments }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_side_has_an_x_and_a_y() {
        use ConstraintProperty as P;
        assert_eq!(property_of_side("top", &P::CenterX), Ok(P::CenterX));
        assert_eq!(property_of_side("top", &P::CenterY), Ok(P::Top));
        assert_eq!(property_of_side("right", &P::CenterX), Ok(P::Right));
        assert_eq!(property_of_side("right", &P::Top), Ok(P::CenterY));
        assert_eq!(property_of_side("bottom", &P::Center), Ok(P::Bottom));
        assert!(property_of_side("top", &P::Width).is_err());
        assert!(property_of_side("center_x", &P::CenterY).is_err());
    }

    #[test]
    fn splices_a_dotted_target_into_a_dotted_name() {
        assert_eq!(splice_dotted("n", "a"), "a");
        assert_eq!(splice_dotted("n.top", "board.led"), "board.led.top");
        let p = ElementPath {
            segments: vec![Spanned::new(Identifier::new("n"), 0..1), Spanned::new(Identifier::new("x"), 2..3)],
        };
        let s = splice_path(&p, "board.led", &(10..19));
        let names: Vec<&str> = s.segments.iter().map(|x| x.node.0.as_str()).collect();
        assert_eq!(names, ["board", "led", "x"]);
        assert_eq!(s.segments[0].span, 10..19);
    }
}
