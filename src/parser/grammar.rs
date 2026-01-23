//! Parser implementation using chumsky

use chumsky::input::{Stream, ValueInput};
use chumsky::prelude::*;

use crate::parser::ast::*;
use crate::parser::lexer::Token;

/// Parse DSL source code into an AST
pub fn parse(input: &str) -> Result<Document, Vec<crate::ParseError>> {
    let len = input.len();

    // Create a logos lexer and convert to token stream
    let token_iter = crate::parser::lexer::lex(input).map(|(tok, span)| (tok, span.into()));

    // Turn the token iterator into a stream that chumsky can use
    let token_stream = Stream::from_iter(token_iter)
        // Split (Token, SimpleSpan) into token and span parts
        .map((len..len).into(), |(t, s): (_, _)| (t, s));

    document_parser()
        .parse(token_stream)
        .into_result()
        .map_err(|errs| errs.into_iter().map(|e| e.into()).collect())
}

/// Helper to extract span range from chumsky's MapExtra
fn span_range(e: &impl chumsky::span::Span<Offset = usize>) -> std::ops::Range<usize> {
    e.start()..e.end()
}

fn document_parser<'a, I>() -> impl Parser<'a, I, Document, extra::Err<Rich<'a, Token>>> + Clone
where
    I: ValueInput<'a, Token = Token, Span = SimpleSpan>,
{
    // Basic token parsers
    let identifier = select! {
        Token::Ident(s) => Identifier::new(s),
    }
    .map_with(|id, e| Spanned::new(id, span_range(&e.span())));

    let string_literal = select! {
        Token::String(s) => s,
    }
    .map_with(|s, e| Spanned::new(s, span_range(&e.span())));

    let number = select! {
        Token::Number(n) => n,
    }
    .map_with(|n, e| Spanned::new(n, span_range(&e.span())));

    // Style key/value parsers
    let style_key = identifier.map(|id| {
        let key = match id.node.as_str() {
            "fill" => StyleKey::Fill,
            "stroke" => StyleKey::Stroke,
            "stroke_width" => StyleKey::StrokeWidth,
            "opacity" => StyleKey::Opacity,
            "label" => StyleKey::Label,
            "font_size" => StyleKey::FontSize,
            "class" => StyleKey::Class,
            "gap" => StyleKey::Gap,
            "size" => StyleKey::Size,
            "width" => StyleKey::Width,
            "height" => StyleKey::Height,
            "routing" => StyleKey::Routing,
            other => StyleKey::Custom(other.to_string()),
        };
        Spanned::new(key, id.span)
    });

    let style_value = choice((
        select! { Token::HexColor(c) => StyleValue::Color(c) }
            .map_with(|v, e| Spanned::new(v, span_range(&e.span()))),
        number.map(|n| {
            Spanned::new(
                StyleValue::Number {
                    value: n.node,
                    unit: None,
                },
                n.span,
            )
        }),
        string_literal.map(|s| Spanned::new(StyleValue::String(s.node), s.span)),
        identifier.map(|id| Spanned::new(StyleValue::Keyword(id.node.0), id.span)),
    ));

    let modifier = style_key
        .then_ignore(just(Token::Colon))
        .then(style_value)
        .map_with(|(key, value), e| {
            Spanned::new(StyleModifier { key, value }, span_range(&e.span()))
        });

    let modifier_block = modifier
        .separated_by(just(Token::Comma))
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just(Token::BracketOpen), just(Token::BracketClose));

    // Shape type parser
    let shape_type = choice((
        just(Token::Rect).to(ShapeType::Rectangle),
        just(Token::Circle).to(ShapeType::Circle),
        just(Token::Ellipse).to(ShapeType::Ellipse),
        just(Token::Polygon).to(ShapeType::Polygon),
        just(Token::Line).to(ShapeType::Line),
        just(Token::Icon)
            .ignore_then(string_literal)
            .map(|s| ShapeType::Icon { icon_name: s.node }),
    ))
    .map_with(|st, e| Spanned::new(st, span_range(&e.span())));

    // Shape declaration
    let shape_decl = shape_type
        .then(identifier.or_not())
        .then(modifier_block.clone().or_not())
        .map(|((shape_type, name), modifiers)| ShapeDecl {
            shape_type,
            name,
            modifiers: modifiers.unwrap_or_default(),
        });

    // Connection operators
    let connection_op = choice((
        just(Token::ArrowBoth).to(ConnectionDirection::Bidirectional),
        just(Token::Arrow).to(ConnectionDirection::Forward),
        just(Token::ArrowBack).to(ConnectionDirection::Backward),
        just(Token::Dash).to(ConnectionDirection::Undirected),
    ));

    // Connection declaration
    let connection_decl = identifier
        .then(connection_op)
        .then(identifier)
        .then(modifier_block.clone().or_not())
        .map(|(((from, direction), to), modifiers)| ConnectionDecl {
            from,
            to,
            direction,
            modifiers: modifiers.unwrap_or_default(),
        });

    // Layout type
    let layout_type = choice((
        just(Token::Row).to(LayoutType::Row),
        just(Token::Col).to(LayoutType::Column),
        just(Token::Grid).to(LayoutType::Grid),
        just(Token::Stack).to(LayoutType::Stack),
    ))
    .map_with(|lt, e| Spanned::new(lt, span_range(&e.span())));

    // Position relation
    let position_relation = choice((
        just(Token::RightOf).to(PositionRelation::RightOf),
        just(Token::LeftOf).to(PositionRelation::LeftOf),
        just(Token::Above).to(PositionRelation::Above),
        just(Token::Below).to(PositionRelation::Below),
        just(Token::Inside).to(PositionRelation::Inside),
    ))
    .map_with(|rel, e| Spanned::new(rel, span_range(&e.span())));

    // Constraint declaration
    let constraint_decl = just(Token::Place)
        .ignore_then(identifier)
        .then(position_relation)
        .then(identifier)
        .map(|((subject, relation), anchor)| ConstraintDecl {
            subject,
            relation,
            anchor,
        });

    // Recursive statement parser
    let statement = recursive(|stmt| {
        // Layout declaration with children
        let layout_decl = layout_type
            .clone()
            .then(identifier.or_not())
            .then(modifier_block.clone().or_not())
            .then(
                stmt.clone()
                    .repeated()
                    .collect::<Vec<_>>()
                    .delimited_by(just(Token::BraceOpen), just(Token::BraceClose)),
            )
            .map(|(((layout_type, name), modifiers), children)| LayoutDecl {
                layout_type,
                name,
                children,
                modifiers: modifiers.unwrap_or_default(),
            });

        // Group declaration with children
        let group_decl = just(Token::Group)
            .ignore_then(identifier.or_not())
            .then(modifier_block.clone().or_not())
            .then(
                stmt.clone()
                    .repeated()
                    .collect::<Vec<_>>()
                    .delimited_by(just(Token::BraceOpen), just(Token::BraceClose)),
            )
            .map(|((name, modifiers), children)| GroupDecl {
                name,
                children,
                modifiers: modifiers.unwrap_or_default(),
            });

        // All statements
        choice((
            constraint_decl.clone().map(Statement::Constraint),
            layout_decl.map(Statement::Layout),
            group_decl.map(Statement::Group),
            connection_decl.clone().map(Statement::Connection),
            shape_decl.clone().map(Statement::Shape),
        ))
        .map_with(|s, e| Spanned::new(s, span_range(&e.span())))
        .boxed()
    });

    // Document is a list of statements
    statement
        .repeated()
        .collect()
        .then_ignore(end())
        .map(|statements| Document { statements })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_shape() {
        let doc = parse("rect server").expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::Shape(s) => {
                assert!(matches!(s.shape_type.node, ShapeType::Rectangle));
                assert_eq!(s.name.as_ref().unwrap().node.as_str(), "server");
            }
            _ => panic!("Expected shape"),
        }
    }

    #[test]
    fn test_parse_shape_with_modifiers() {
        let doc = parse("circle db [fill: blue, stroke: #ff0000]").expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::Shape(s) => {
                assert_eq!(s.modifiers.len(), 2);
            }
            _ => panic!("Expected shape"),
        }
    }

    #[test]
    fn test_parse_connection() {
        let doc = parse("a -> b").expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::Connection(c) => {
                assert_eq!(c.from.node.as_str(), "a");
                assert_eq!(c.to.node.as_str(), "b");
                assert_eq!(c.direction, ConnectionDirection::Forward);
            }
            _ => panic!("Expected connection"),
        }
    }

    #[test]
    fn test_parse_layout() {
        let doc = parse("row { rect a rect b }").expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::Layout(l) => {
                assert!(matches!(l.layout_type.node, LayoutType::Row));
                assert_eq!(l.children.len(), 2);
            }
            _ => panic!("Expected layout"),
        }
    }

    #[test]
    fn test_parse_group() {
        let doc = parse("group datacenter { rect server1 rect server2 }").expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::Group(g) => {
                assert_eq!(g.name.as_ref().unwrap().node.as_str(), "datacenter");
                assert_eq!(g.children.len(), 2);
            }
            _ => panic!("Expected group"),
        }
    }

    #[test]
    fn test_parse_constraint() {
        let doc = parse("place client right-of server").expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::Constraint(c) => {
                assert_eq!(c.subject.node.as_str(), "client");
                assert!(matches!(c.relation.node, PositionRelation::RightOf));
                assert_eq!(c.anchor.node.as_str(), "server");
            }
            _ => panic!("Expected constraint"),
        }
    }

    #[test]
    fn test_parse_icon() {
        let doc = parse(r#"icon "server" myserver"#).expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::Shape(s) => {
                match &s.shape_type.node {
                    ShapeType::Icon { icon_name } => assert_eq!(icon_name, "server"),
                    _ => panic!("Expected icon"),
                }
                assert_eq!(s.name.as_ref().unwrap().node.as_str(), "myserver");
            }
            _ => panic!("Expected shape"),
        }
    }

    #[test]
    fn test_parse_nested() {
        let input = r#"
            group datacenter {
                col {
                    group rack1 {
                        rect server1
                    }
                }
            }
        "#;
        let doc = parse(input).expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
    }
}
