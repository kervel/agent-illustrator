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
        ParseError::Syntax {
            span: err.span().into_range(),
            message: format!("{:?}", err.reason()),
            expected: err.expected().map(|e| format!("{:?}", e)).collect(),
        }
    }
}
