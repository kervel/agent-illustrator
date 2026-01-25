//! Error types for parsing and validation

use ariadne::{Color, Label, Report, ReportKind, Source};
use thiserror::Error;

/// Byte range in source text
pub type Span = std::ops::Range<usize>;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Parse error at {span:?}: {message}")]
    Syntax {
        span: Span,
        message: String,
        expected: Vec<String>,
    },
}

impl ParseError {
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
                    format!("Unexpected {}", found_str)
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
                    chumsky::error::RichPattern::Identifier(s) => Some(format!("identifier '{}'", s)),
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

/// Format a token for human-readable error messages
fn format_token(tok: &crate::parser::lexer::Token) -> String {
    use crate::parser::lexer::Token;
    match tok {
        Token::Ident(s) => format!("identifier '{}'", s),
        Token::String(s) => format!("string \"{}\"", s),
        Token::Number(n) => format!("number {}", n),
        Token::HexColor(c) => format!("color {}", c),
        Token::Arrow => "'->'"  .to_string(),
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
        // Other
        _ => format!("{:?}", tok),
    }
}
