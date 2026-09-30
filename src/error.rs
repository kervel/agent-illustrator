//! Error types for parsing and validation

use ariadne::{Color, Label, Report, ReportKind, Source};
use thiserror::Error;

/// Byte range in source text
pub type Span = std::ops::Range<usize>;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("{message}")]
    Syntax {
        span: Span,
        message: String,
        expected: Vec<String>,
    },
}

impl ParseError {
    /// Rewrite the message to say where: `line L, column C`, the source line
    /// with a caret under the problem, and what would have been accepted.
    pub fn locate(self, source: &str) -> ParseError {
        let ParseError::Syntax { span, message, expected } = self;
        let start = span.start.min(source.len());
        let line_no = source[..start].matches('\n').count() + 1;
        let line_start = source[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let line_end = source[start..].find('\n').map(|i| start + i).unwrap_or(source.len());
        let col = source[line_start..start].chars().count() + 1;
        let text = &source[line_start..line_end];
        let width = source[start..span.end.min(line_end).max(start)].chars().count().max(1);
        // Chumsky sometimes reports "end of input" for a token that is plainly
        // there; name what is actually at the caret and what came before it.
        let message = if message.starts_with("Unexpected end of input") && source[start..].trim().len() > 0 {
            let here: String = source[start..]
                .chars()
                .take_while(|c| !c.is_whitespace())
                .take(20)
                .collect();
            let before = source[line_start..start]
                .split_whitespace()
                .last()
                .map(|w| format!(" after '{}'", w))
                .unwrap_or_default();
            let what = match source[line_start..start].split_whitespace().last() {
                Some("to") => " (expected a destination: an element name)".to_string(),
                Some("from") => " (expected where it starts: an element name)".to_string(),
                Some("along") => " (expected a path to move along)".to_string(),
                _ => String::new(),
            };
            format!("Unexpected '{}'{}{}", here, before, what)
        } else {
            message
        };
        // `constrain c.center_y = (a.bottom + b.top) / 2`: no grouping or
        // division in constraints; say what does it.
        let message = if source[start..].starts_with('(') && text.trim_start().starts_with("constrain") {
            format!(
                "{} (constraints have no arithmetic; the middle of a gap is `midpoint(a.bottom, b.top)`, \
                 an offset is `+ 20`)",
                message
            )
        } else {
            message
        };
        let mut shown = expected.clone();
        shown.sort();
        shown.dedup();
        let expected_str = if shown.is_empty() {
            String::new()
        } else if shown.len() > 8 {
            format!("\n  expected one of: {} ...", shown[..8].join(", "))
        } else {
            format!("\n  expected: {}", shown.join(", "))
        };
        let located = format!(
            "line {}, column {}: {}\n  {}\n  {}{}{}",
            line_no,
            col,
            message,
            text,
            " ".repeat(col - 1),
            "^".repeat(width),
            expected_str
        );
        ParseError::Syntax { span, message: located, expected }
    }

    /// Format the error with source context using ariadne
    pub fn format(&self, source: &str, filename: &str) -> String {
        let mut buf = Vec::new();
        match self {
            ParseError::Syntax {
                span,
                message,
                expected,
            } => {
                let expected_str = if expected.is_empty() {
                    String::new()
                } else {
                    format!("\nExpected: {}", expected.join(", "))
                };

                Report::build(ReportKind::Error, filename, span.start)
                    .with_message(message)
                    .with_label(
                        Label::new((filename, span.clone()))
                            .with_message(format!("{}{}", message, expected_str))
                            .with_color(Color::Red),
                    )
                    .finish()
                    .write((filename, Source::from(source)), &mut buf)
                    .unwrap();
            }
        }
        String::from_utf8(buf).unwrap()
    }
}

impl<'a> From<chumsky::error::Rich<'a, crate::parser::lexer::Token>> for ParseError {
    fn from(err: chumsky::error::Rich<'a, crate::parser::lexer::Token>) -> Self {
        use crate::parser::lexer::Token;
        use chumsky::error::RichReason;

        // Check if we found a reserved keyword where an identifier was expected
        let found_token = err.found().cloned();
        let is_reserved_keyword = matches!(
            found_token,
            Some(Token::Left)
                | Some(Token::Right)
                | Some(Token::Top)
                | Some(Token::Bottom)
                | Some(Token::Center)
                | Some(Token::CenterXProp)
                | Some(Token::CenterYProp)
                | Some(Token::HorizontalCenter)
                | Some(Token::VerticalCenter)
        );

        // Format the message based on the reason
        let message = match err.reason() {
            RichReason::ExpectedFound { found, .. } => {
                if is_reserved_keyword {
                    let keyword = match found_token.as_ref().unwrap() {
                        Token::Left => "left",
                        Token::Right => "right",
                        Token::Top => "top",
                        Token::Bottom => "bottom",
                        Token::Center => "center",
                        Token::CenterXProp => "center_x",
                        Token::CenterYProp => "center_y",
                        Token::HorizontalCenter => "horizontal_center",
                        Token::VerticalCenter => "vertical_center",
                        _ => "unknown",
                    };
                    format!(
                        "Cannot use '{}' as a name - it's a reserved keyword for constraints",
                        keyword
                    )
                } else {
                    let found_str = match found {
                        Some(tok) => format_token(tok),
                        None => "end of input".to_string(),
                    };
                    match found.as_ref().and_then(|t| keyword_spelling(t)) {
                        Some(w) => format!(
                            "Unexpected {}. '{}' is a reserved word in AIL: it cannot name an element, \
                             parameter or connection; rename it (e.g. '{}_1' or something more specific)",
                            found_str, w, w
                        ),
                        None => format!("Unexpected {}", found_str),
                    }
                }
            }
            RichReason::Custom(msg) => msg.to_string(),
        };

        // Format expected tokens nicely
        let expected: Vec<String> = err
            .expected()
            .filter_map(|e| {
                match e {
                    chumsky::error::RichPattern::Token(tok) => Some(format_token(tok)),
                    chumsky::error::RichPattern::Label(label) => Some(label.to_string()),
                    chumsky::error::RichPattern::EndOfInput => Some("end of input".to_string()),
                    chumsky::error::RichPattern::Identifier(s) => {
                        Some(format!("identifier '{}'", s))
                    }
                    chumsky::error::RichPattern::Any => Some("any token".to_string()),
                    chumsky::error::RichPattern::SomethingElse => None, // Skip "something else"
                }
            })
            .collect();

        ParseError::Syntax {
            span: err.span().into_range(),
            message,
            expected,
        }
    }
}

/// The source spelling of a keyword token, if it is one.
pub fn keyword_spelling(tok: &crate::parser::lexer::Token) -> Option<&'static str> {
    use crate::parser::lexer::Token;
    Some(match tok {
        Token::Rect => "rect",
        Token::Circle => "circle",
        Token::Ellipse => "ellipse",
        Token::Polygon => "polygon",
        Token::Line => "line",
        Token::Icon => "icon",
        Token::Text => "text",
        Token::Callout => "callout",
        Token::Path => "path",
        Token::Vertex => "vertex",
        Token::LineTo => "line_to",
        Token::ArcTo => "arc_to",
        Token::CurveTo => "curve_to",
        Token::Close => "close",
        Token::Clockwise => "clockwise",
        Token::Cw => "cw",
        Token::Counterclockwise => "counterclockwise",
        Token::Ccw => "ccw",
        Token::Row => "row",
        Token::Col => "col",
        Token::Grid => "grid",
        Token::Stack => "stack",
        Token::Group => "group",
        Token::Label => "label",
        Token::Template => "template",
        Token::From => "from",
        Token::Export => "export",
        Token::Anchor => "anchor",
        Token::Direction => "direction",
        Token::Position => "position",
        Token::Up => "up",
        Token::Down => "down",
        Token::Place => "place",
        Token::RightOf => "right-of",
        Token::LeftOf => "left-of",
        Token::Above => "above",
        Token::Below => "below",
        Token::Inside => "inside",
        Token::Left => "left",
        Token::Right => "right",
        Token::Top => "top",
        Token::Bottom => "bottom",
        Token::HorizontalCenter => "horizontal_center",
        Token::VerticalCenter => "vertical_center",
        Token::Role => "role",
        Token::As => "as",
        Token::Keyframe => "keyframe",
        Token::Show => "show",
        Token::Hide => "hide",
        Token::Transform => "transform",
        Token::Disable => "disable",
        Token::Enable => "enable",
        Token::Constrain => "constrain",
        Token::Midpoint => "midpoint",
        Token::Contains => "contains",
        Token::CenterXProp => "center_x",
        Token::CenterYProp => "center_y",
        Token::Center => "center",
        _ => return None,
    })
}

/// Format a token for human-readable error messages
fn format_token(tok: &crate::parser::lexer::Token) -> String {
    use crate::parser::lexer::Token;
    match tok {
        Token::Ident(s) => format!("identifier '{}'", s),
        Token::String(s) => format!("string \"{}\"", s),
        Token::Number(n) => format!("number {}", n),
        Token::HexColor(c) => format!("color {}", c),
        Token::Arrow => "'->'".to_string(),
        Token::ArrowBack => "'<-'".to_string(),
        Token::ArrowBoth => "'<->'".to_string(),
        Token::Dash => "'--'".to_string(),
        Token::BraceOpen => "'{'".to_string(),
        Token::BraceClose => "'}'".to_string(),
        Token::BracketOpen => "'['".to_string(),
        Token::BracketClose => "']'".to_string(),
        Token::Comma => "','".to_string(),
        Token::Colon => "':'".to_string(),
        // Reserved keywords
        Token::Left => "keyword 'left'".to_string(),
        Token::Right => "keyword 'right'".to_string(),
        Token::Top => "keyword 'top'".to_string(),
        Token::Bottom => "keyword 'bottom'".to_string(),
        Token::Center => "keyword 'center'".to_string(),
        Token::CenterXProp => "keyword 'center_x'".to_string(),
        Token::CenterYProp => "keyword 'center_y'".to_string(),
        // Shape keywords
        Token::Rect => "keyword 'rect'".to_string(),
        Token::Circle => "keyword 'circle'".to_string(),
        Token::Ellipse => "keyword 'ellipse'".to_string(),
        Token::Path => "keyword 'path'".to_string(),
        Token::Text => "keyword 'text'".to_string(),
        // Layout keywords
        Token::Row => "keyword 'row'".to_string(),
        Token::Col => "keyword 'col'".to_string(),
        Token::Group => "keyword 'group'".to_string(),
        Token::Percent => "'%'".to_string(),
        Token::Star => "'*'".to_string(),
        Token::Semicolon => "';'".to_string(),
        Token::Dot => "'.'".to_string(),
        Token::ParenOpen => "'('".to_string(),
        Token::ParenClose => "')'".to_string(),
        Token::Equals => "'='".to_string(),
        Token::Plus => "'+'".to_string(),
        // Other
        other => match keyword_spelling(other) {
            Some(w) => format!("keyword '{}'", w),
            None => format!("{:?}", other),
        },
    }
}
