//! Line-end styles for connections: `[from_end: triangle, to_end: diamond-filled]`.
//!
//! Every connection has two ends. The operator implies a default (`->` puts an
//! arrow on the `to` end, `<-` on the `from` end, `<->` on both, `--` on
//! neither); `from_end` / `to_end` override it per end, for any operator.

use crate::layout::error::LayoutError;
use crate::parser::ast::{ConnectionDirection, Spanned, StyleKey, StyleModifier, StyleValue};

/// The decoration at one end of a connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnd {
    /// Filled arrowhead (the default head of `->`).
    Arrow,
    /// Open V arrowhead.
    Open,
    /// Hollow triangle (UML inheritance).
    Triangle,
    /// Hollow diamond (UML aggregation).
    Diamond,
    /// Filled diamond (UML composition).
    DiamondFilled,
    /// Crow's foot (ER "many").
    Crow,
    /// Hollow circle.
    Circle,
    /// Plain line end.
    None,
}

impl LineEnd {
    /// Every style, with the word that spells it in source.
    pub const ALL: [(LineEnd, &'static str); 8] = [
        (LineEnd::Arrow, "arrow"),
        (LineEnd::Open, "open"),
        (LineEnd::Triangle, "triangle"),
        (LineEnd::Diamond, "diamond"),
        (LineEnd::DiamondFilled, "diamond-filled"),
        (LineEnd::Crow, "crow"),
        (LineEnd::Circle, "circle"),
        (LineEnd::None, "none"),
    ];

    /// The valid words, for error messages and docs.
    pub fn valid_words() -> String {
        Self::ALL.iter().map(|(_, w)| *w).collect::<Vec<_>>().join(", ")
    }

    /// The source word (`diamond-filled`).
    pub fn word(self) -> &'static str {
        Self::ALL.iter().find(|(e, _)| *e == self).map(|(_, w)| *w).unwrap_or("none")
    }

    /// Parse a source word. `_` is accepted for `-`.
    pub fn parse(word: &str) -> Option<LineEnd> {
        let w = word.replace('_', "-");
        Self::ALL.iter().find(|(_, s)| *s == w).map(|(e, _)| *e)
    }

    /// How far the line stops short of the anchor, in multiples of the stroke
    /// width, so the shape (which sits beyond the line's end) lands on the anchor.
    pub fn pullback(self) -> f64 {
        match self {
            LineEnd::Arrow => 3.6,
            LineEnd::Triangle => 6.5,
            LineEnd::Diamond | LineEnd::DiamondFilled => 6.2,
            LineEnd::Circle => 5.85,
            // The line runs into these: they are drawn on top of it.
            LineEnd::Open | LineEnd::Crow | LineEnd::None => 0.0,
        }
    }

    /// Hollow ends are painted with the background colour inside.
    pub fn is_hollow(self) -> bool {
        matches!(self, LineEnd::Triangle | LineEnd::Diamond | LineEnd::Circle)
    }
}

/// The two ends of a connection, as drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineEnds {
    pub from: LineEnd,
    pub to: LineEnd,
}

impl LineEnds {
    /// What the operator alone implies.
    pub fn from_direction(direction: ConnectionDirection) -> Self {
        let head = |on: bool| if on { LineEnd::Arrow } else { LineEnd::None };
        LineEnds {
            from: head(matches!(
                direction,
                ConnectionDirection::Backward | ConnectionDirection::Bidirectional
            )),
            to: head(matches!(
                direction,
                ConnectionDirection::Forward | ConnectionDirection::Bidirectional
            )),
        }
    }

    /// The operator's default with `from_end` / `to_end` applied. An unknown
    /// value is an error listing the valid ones.
    pub fn resolve(
        direction: ConnectionDirection,
        modifiers: &[Spanned<StyleModifier>],
    ) -> Result<Self, LayoutError> {
        let mut ends = Self::from_direction(direction);
        for m in modifiers {
            let StyleKey::Custom(key) = &m.node.key.node else { continue };
            if key != "from_end" && key != "to_end" {
                continue;
            }
            let word = match &m.node.value.node {
                StyleValue::Keyword(w) | StyleValue::String(w) => w.clone(),
                StyleValue::Identifier(id) => id.0.clone(),
                // A hyphenated word (`diamond-filled`) lexes as a palette token.
                StyleValue::Color(c) => c.token_string().or_else(|| c.concrete_string().map(str::to_string)).unwrap_or_default(),
                _ => String::new(),
            };
            let Some(end) = LineEnd::parse(&word) else {
                return Err(LayoutError::Located {
                    message: format!(
                        "unknown line end '{}' for {} (valid: {})",
                        word,
                        key,
                        LineEnd::valid_words()
                    ),
                    span: m.node.value.span.clone(),
                });
            };
            if key == "from_end" {
                ends.from = end;
            } else {
                ends.to = end;
            }
        }
        Ok(ends)
    }

    /// Both ends plain.
    pub fn none() -> Self {
        LineEnds { from: LineEnd::None, to: LineEnd::None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_word() {
        for (e, w) in LineEnd::ALL {
            assert_eq!(LineEnd::parse(w), Some(e));
        }
        assert_eq!(LineEnd::parse("diamond_filled"), Some(LineEnd::DiamondFilled));
        assert_eq!(LineEnd::parse("zigzag"), None);
    }

    #[test]
    fn operator_defaults() {
        let e = LineEnds::from_direction(ConnectionDirection::Forward);
        assert_eq!((e.from, e.to), (LineEnd::None, LineEnd::Arrow));
        let e = LineEnds::from_direction(ConnectionDirection::Backward);
        assert_eq!((e.from, e.to), (LineEnd::Arrow, LineEnd::None));
        let e = LineEnds::from_direction(ConnectionDirection::Bidirectional);
        assert_eq!((e.from, e.to), (LineEnd::Arrow, LineEnd::Arrow));
        assert_eq!(LineEnds::from_direction(ConnectionDirection::Undirected), LineEnds::none());
    }

    #[test]
    fn hollow_ends_are_the_shapes_with_an_inside() {
        assert!(LineEnd::Triangle.is_hollow() && LineEnd::Diamond.is_hollow() && LineEnd::Circle.is_hollow());
        assert!(!LineEnd::DiamondFilled.is_hollow() && !LineEnd::Arrow.is_hollow());
    }
}
