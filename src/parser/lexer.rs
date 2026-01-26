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

    // Path shape keywords (Feature 007)
    #[token("path")]
    Path,
    #[token("vertex")]
    Vertex,
    #[token("line_to")]
    LineTo,
    #[token("arc_to")]
    ArcTo,
    #[token("curve_to")]
    CurveTo,
    #[token("close")]
    Close,

    // Sweep direction keywords (Feature 007)
    #[token("clockwise")]
    Clockwise,
    #[token("cw")]
    Cw,
    #[token("counterclockwise")]
    Counterclockwise,
    #[token("ccw")]
    Ccw,

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

    // Template keywords
    #[token("template")]
    Template,
    #[token("from")]
    From,
    #[token("export")]
    Export,
    #[token("anchor")]
    Anchor,
    #[token("direction")]
    Direction,
    #[token("position")]
    Position,
    // Cardinal direction keywords for anchor declarations
    #[token("up")]
    Up,
    #[token("down")]
    Down,

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

    // Edge keywords (used in constraints)
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

    // Constraint keywords
    #[token("constrain")]
    Constrain,
    #[token("midpoint")]
    Midpoint,
    #[token("contains")]
    Contains,

    // Constraint property keywords
    #[token("center_x")]
    CenterXProp,
    #[token("center_y")]
    CenterYProp,
    #[token("center")]
    Center,

    // Connection operators (order matters - longer patterns first)
    #[token("<->")]
    ArrowBoth,
    #[token("->")]
    Arrow,
    #[token("<-")]
    ArrowBack,
    #[token("--")]
    Dash,

    // Single minus sign (for symbolic colors like foreground-1)
    #[token("-")]
    Minus,

    // Plus sign (for constraint offsets like a.left = b.right + 20)
    #[token("+")]
    Plus,

    // Delimiters
    #[token("{")]
    BraceOpen,
    #[token("}")]
    BraceClose,
    #[token("[")]
    BracketOpen,
    #[token("]")]
    BracketClose,
    #[token("(")]
    ParenOpen,
    #[token(")")]
    ParenClose,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[token(".")]
    Dot,
    // Comparison operators (longer first)
    #[token(">=")]
    GreaterOrEqual,
    #[token("<=")]
    LessOrEqual,
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

    #[regex(r"[0-9]+(\.[0-9]+)?", |lex| lex.slice().parse::<f64>().ok())]
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
                Token::Minus,
                Token::Number(10.0)
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
    fn test_edge_keywords() {
        let tokens: Vec<_> = lex("left right top bottom horizontal_center vertical_center")
            .map(|(t, _)| t)
            .collect();
        assert_eq!(
            tokens,
            vec![
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
    fn test_role_keyword() {
        let tokens: Vec<_> = lex("role: label").map(|(t, _)| t).collect();
        assert_eq!(tokens, vec![Token::Role, Token::Colon, Token::Label]);
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

    #[test]
    fn test_constrain_keywords() {
        let tokens: Vec<_> = lex("constrain midpoint contains").map(|(t, _)| t).collect();
        assert_eq!(
            tokens,
            vec![Token::Constrain, Token::Midpoint, Token::Contains]
        );
    }

    #[test]
    fn test_comparison_operators() {
        let tokens: Vec<_> = lex(">= <=").map(|(t, _)| t).collect();
        assert_eq!(tokens, vec![Token::GreaterOrEqual, Token::LessOrEqual]);
    }

    #[test]
    fn test_constraint_property_keywords() {
        let tokens: Vec<_> = lex("center center_x center_y").map(|(t, _)| t).collect();
        assert_eq!(
            tokens,
            vec![Token::Center, Token::CenterXProp, Token::CenterYProp]
        );
    }

    #[test]
    fn test_parentheses() {
        let tokens: Vec<_> = lex("midpoint(a, b)").map(|(t, _)| t).collect();
        assert_eq!(
            tokens,
            vec![
                Token::Midpoint,
                Token::ParenOpen,
                Token::Ident("a".to_string()),
                Token::Comma,
                Token::Ident("b".to_string()),
                Token::ParenClose,
            ]
        );
    }

    #[test]
    fn test_plus_for_offset() {
        let tokens: Vec<_> = lex("a.left = b.right + 20").map(|(t, _)| t).collect();
        assert_eq!(
            tokens,
            vec![
                Token::Ident("a".to_string()),
                Token::Dot,
                Token::Left,
                Token::Equals,
                Token::Ident("b".to_string()),
                Token::Dot,
                Token::Right,
                Token::Plus,
                Token::Number(20.0),
            ]
        );
    }

    #[test]
    fn test_template_keywords() {
        let tokens: Vec<_> = lex("template from export").map(|(t, _)| t).collect();
        assert_eq!(tokens, vec![Token::Template, Token::From, Token::Export]);
    }

    #[test]
    fn test_template_declaration() {
        let input = r#"template "box" from "icons/box.svg""#;
        let tokens: Vec<_> = lex(input).map(|(t, _)| t).collect();
        assert_eq!(
            tokens,
            vec![
                Token::Template,
                Token::String("box".to_string()),
                Token::From,
                Token::String("icons/box.svg".to_string()),
            ]
        );
    }

    #[test]
    fn test_export_declaration() {
        let input = "export port1, port2";
        let tokens: Vec<_> = lex(input).map(|(t, _)| t).collect();
        assert_eq!(
            tokens,
            vec![
                Token::Export,
                Token::Ident("port1".to_string()),
                Token::Comma,
                Token::Ident("port2".to_string()),
            ]
        );
    }

    // Path shape tests (Feature 007)
    #[test]
    fn test_path_keywords() {
        let tokens: Vec<_> = lex("path vertex line_to arc_to close")
            .map(|(t, _)| t)
            .collect();
        assert_eq!(
            tokens,
            vec![
                Token::Path,
                Token::Vertex,
                Token::LineTo,
                Token::ArcTo,
                Token::Close
            ]
        );
    }

    #[test]
    fn test_sweep_direction_keywords() {
        let tokens: Vec<_> = lex("clockwise cw counterclockwise ccw")
            .map(|(t, _)| t)
            .collect();
        assert_eq!(
            tokens,
            vec![
                Token::Clockwise,
                Token::Cw,
                Token::Counterclockwise,
                Token::Ccw
            ]
        );
    }

    #[test]
    fn test_path_example() {
        let input = r#"path "arrow" { vertex start line_to tip close }"#;
        let tokens: Vec<_> = lex(input).map(|(t, _)| t).collect();
        assert!(tokens.contains(&Token::Path));
        assert!(tokens.contains(&Token::Vertex));
        assert!(tokens.contains(&Token::LineTo));
        assert!(tokens.contains(&Token::Close));
    }

    // Anchor keyword tests (Feature 009)
    #[test]
    fn test_anchor_keywords() {
        let tokens: Vec<_> = lex("anchor direction position up down")
            .map(|(t, _)| t)
            .collect();
        assert_eq!(
            tokens,
            vec![Token::Anchor, Token::Direction, Token::Position, Token::Up, Token::Down]
        );
    }
}
