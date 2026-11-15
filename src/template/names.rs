//! Putting one name in place of another at the head of a reference.
//!
//! A name that stands for an element (an `each` loop variable, an
//! element-reference template parameter) is replaced at the head of every
//! reference that starts with it: `n.top` with `n` = `board.led` becomes
//! `board.led.top`. Shared by `each` expansion and template resolution so
//! both splice references the same way.

use crate::parser::ast::{ElementPath, Identifier, Span, Spanned};

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
