//! Lexer for the Agent Illustrator DSL using logos

use logos::Logos;

/// Byte range in source text
pub type Span = std::ops::Range<usize>;

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\n\r]+")]
pub enum Token {
    // Shape keywords
    #[token("rect")]
    Rect,
    #[token("circle")]
    Circle,
    #[token("ellipse")]
    Ellipse,
    #[token("polygon")]
    Polygon,
    #[token("line")]
    Line,
    #[token("icon")]
    Icon,
    #[token("text")]
    Text,

    // Layout keywords
    #[token("row")]
    Row,
    #[token("col")]
    Col,
    #[token("grid")]
    Grid,
    #[token("stack")]
    Stack,
    #[token("group")]
    Group,
    #[token("label")]
    Label,

    // Constraint keywords
    #[token("place")]
    Place,
    #[token("right-of")]
    RightOf,
    #[token("left-of")]
    LeftOf,
    #[token("above")]
    Above,
    #[token("below")]
    Below,
    #[token("inside")]
    Inside,

    // Alignment keywords
    #[token("align")]
    Align,
    #[token("left")]
    Left,
    #[token("right")]
    Right,
    #[token("top")]
    Top,
    #[token("bottom")]
    Bottom,
    #[token("horizontal_center")]
    HorizontalCenter,
    #[token("vertical_center")]
    VerticalCenter,

    // Role keyword
    #[token("role")]
    Role,

    // Connection operators (order matters - longer patterns first)
    #[token("<->")]
    ArrowBoth,
    #[token("->")]
    Arrow,
    #[token("<-")]
    ArrowBack,
    #[token("--")]
    Dash,

    // Delimiters
    #[token("{")]
    BraceOpen,
    #[token("}")]
    BraceClose,
    #[token("[")]
    BracketOpen,
    #[token("]")]
    BracketClose,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[token(".")]
    Dot,
    #[token("=")]
    Equals,

    // Literals - identifiers must come after keywords
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string(), priority = 1)]
    Ident(String),

    #[regex(r#""([^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        s[1..s.len()-1].to_string()
    })]
    String(String),

    #[regex(r"-?[0-9]+(\.[0-9]+)?", |lex| lex.slice().parse::<f64>().ok())]
    Number(f64),

    #[regex(r"#[0-9a-fA-F]{3,6}", |lex| lex.slice().to_string())]
    HexColor(String),

    // Comments (skip)
    #[regex(r"//[^\n]*", logos::skip)]
    LineComment,

    #[regex(r"/\*([^*]|\*[^/])*\*/", logos::skip)]
    BlockComment,
}

/// Lex input string into tokens with spans
pub fn lex(input: &str) -> impl Iterator<Item = (Token, Span)> + '_ {
    Token::lexer(input)
        .spanned()
        .filter_map(|(tok, span)| tok.ok().map(|t| (t, span)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shape_keywords() {
        let tokens: Vec<_> = lex("rect circle ellipse text").map(|(t, _)| t).collect();
        assert_eq!(
            tokens,
            vec![Token::Rect, Token::Circle, Token::Ellipse, Token::Text]
        );
    }

    #[test]
    fn test_connection_operators() {
        let tokens: Vec<_> = lex("-> <- <-> --").map(|(t, _)| t).collect();
        assert_eq!(
            tokens,
            vec![
                Token::Arrow,
                Token::ArrowBack,
                Token::ArrowBoth,
                Token::Dash
            ]
        );
    }

    #[test]
    fn test_identifiers_and_strings() {
        let tokens: Vec<_> = lex(r#"server "my name""#).map(|(t, _)| t).collect();
        assert_eq!(
            tokens,
            vec![
                Token::Ident("server".to_string()),
                Token::String("my name".to_string())
            ]
        );
    }

    #[test]
    fn test_comments_skipped() {
        let tokens: Vec<_> = lex("rect // comment\ncircle").map(|(t, _)| t).collect();
        assert_eq!(tokens, vec![Token::Rect, Token::Circle]);
    }

    #[test]
    fn test_block_comments_skipped() {
        let tokens: Vec<_> = lex("rect /* block comment */ circle")
            .map(|(t, _)| t)
            .collect();
        assert_eq!(tokens, vec![Token::Rect, Token::Circle]);
    }

    #[test]
    fn test_numbers() {
        let tokens: Vec<_> = lex("42 3.14 -10").map(|(t, _)| t).collect();
        assert_eq!(
            tokens,
            vec![
                Token::Number(42.0),
                Token::Number(3.14),
                Token::Number(-10.0)
            ]
        );
    }

    #[test]
    fn test_hex_colors() {
        let tokens: Vec<_> = lex("#fff #ff0000").map(|(t, _)| t).collect();
        assert_eq!(
            tokens,
            vec![
                Token::HexColor("#fff".to_string()),
                Token::HexColor("#ff0000".to_string())
            ]
        );
    }

    #[test]
    fn test_layout_keywords() {
        let tokens: Vec<_> = lex("row col grid stack group").map(|(t, _)| t).collect();
        assert_eq!(
            tokens,
            vec![
                Token::Row,
                Token::Col,
                Token::Grid,
                Token::Stack,
                Token::Group
            ]
        );
    }

    #[test]
    fn test_constraint_keywords() {
        let tokens: Vec<_> = lex("place right-of left-of above below inside")
            .map(|(t, _)| t)
            .collect();
        assert_eq!(
            tokens,
            vec![
                Token::Place,
                Token::RightOf,
                Token::LeftOf,
                Token::Above,
                Token::Below,
                Token::Inside
            ]
        );
    }

    #[test]
    fn test_delimiters() {
        let tokens: Vec<_> = lex("{ } [ ] , :").map(|(t, _)| t).collect();
        assert_eq!(
            tokens,
            vec![
                Token::BraceOpen,
                Token::BraceClose,
                Token::BracketOpen,
                Token::BracketClose,
                Token::Comma,
                Token::Colon
            ]
        );
    }

    #[test]
    fn test_complete_example() {
        let input = r#"
            rect server [fill: #ff0000]
            circle db
            server -> db [label: "query"]
        "#;
        let tokens: Vec<_> = lex(input).map(|(t, _)| t).collect();
        assert_eq!(
            tokens,
            vec![
                Token::Rect,
                Token::Ident("server".to_string()),
                Token::BracketOpen,
                Token::Ident("fill".to_string()),
                Token::Colon,
                Token::HexColor("#ff0000".to_string()),
                Token::BracketClose,
                Token::Circle,
                Token::Ident("db".to_string()),
                Token::Ident("server".to_string()),
                Token::Arrow,
                Token::Ident("db".to_string()),
                Token::BracketOpen,
                Token::Label, // "label" is now a keyword token
                Token::Colon,
                Token::String("query".to_string()),
                Token::BracketClose,
            ]
        );
    }

    #[test]
    fn test_label_keyword() {
        let tokens: Vec<_> = lex("label").map(|(t, _)| t).collect();
        assert_eq!(tokens, vec![Token::Label]);
    }

    #[test]
    fn test_alignment_keywords() {
        let tokens: Vec<_> = lex("align left right top bottom horizontal_center vertical_center")
            .map(|(t, _)| t)
            .collect();
        assert_eq!(
            tokens,
            vec![
                Token::Align,
                Token::Left,
                Token::Right,
                Token::Top,
                Token::Bottom,
                Token::HorizontalCenter,
                Token::VerticalCenter,
            ]
        );
    }

    #[test]
    fn test_alignment_syntax() {
        let tokens: Vec<_> = lex("align a.left = b.right").map(|(t, _)| t).collect();
        assert_eq!(
            tokens,
            vec![
                Token::Align,
                Token::Ident("a".to_string()),
                Token::Dot,
                Token::Left,
                Token::Equals,
                Token::Ident("b".to_string()),
                Token::Dot,
                Token::Right,
            ]
        );
    }

    #[test]
    fn test_role_keyword() {
        let tokens: Vec<_> = lex("role: label").map(|(t, _)| t).collect();
        assert_eq!(
            tokens,
            vec![Token::Role, Token::Colon, Token::Label]
        );
    }

    #[test]
    fn test_dot_and_equals() {
        let tokens: Vec<_> = lex("a.b = c.d").map(|(t, _)| t).collect();
        assert_eq!(
            tokens,
            vec![
                Token::Ident("a".to_string()),
                Token::Dot,
                Token::Ident("b".to_string()),
                Token::Equals,
                Token::Ident("c".to_string()),
                Token::Dot,
                Token::Ident("d".to_string()),
            ]
        );
    }
}
