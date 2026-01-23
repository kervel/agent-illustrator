//! Parser implementation using chumsky

use chumsky::input::{Stream, ValueInput};
use chumsky::prelude::*;

use crate::parser::ast::*;
use crate::parser::lexer::Token;

/// Check if an identifier is a symbolic color category
fn is_color_category(ident: &str) -> Option<ColorCategory> {
    match ident {
        "foreground" => Some(ColorCategory::Foreground),
        "background" => Some(ColorCategory::Background),
        "text" => Some(ColorCategory::Text),
        "accent" => Some(ColorCategory::Accent),
        _ => None,
    }
}

/// Check if an identifier is a lightness modifier
fn is_lightness_modifier(ident: &str) -> Option<Lightness> {
    match ident {
        "light" => Some(Lightness::Light),
        "dark" => Some(Lightness::Dark),
        _ => None,
    }
}

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
    // Note: We need to handle both identifiers AND the Label token (since "label" is now a keyword)
    let style_key = choice((
        // Handle the "label" keyword token explicitly
        just(Token::Label).map_with(|_, e| Spanned::new(StyleKey::Label, span_range(&e.span()))),
        // Handle all other style keys as identifiers
        identifier.map(|id| {
            let key = match id.node.as_str() {
                "fill" => StyleKey::Fill,
                "stroke" => StyleKey::Stroke,
                "stroke_width" => StyleKey::StrokeWidth,
                "opacity" => StyleKey::Opacity,
                "font_size" => StyleKey::FontSize,
                "class" => StyleKey::Class,
                "gap" => StyleKey::Gap,
                "size" => StyleKey::Size,
                "width" => StyleKey::Width,
                "height" => StyleKey::Height,
                "routing" => StyleKey::Routing,
                "label_position" => StyleKey::LabelPosition,
                other => StyleKey::Custom(other.to_string()),
            };
            Spanned::new(key, id.span)
        }),
    ));

    // Parse a color category - identifier or "text" keyword (since text is reserved)
    let color_category = choice((
        // "text" keyword token maps to Text category
        just(Token::Text).map_with(|_, e| Spanned::new(Identifier::new("text"), span_range(&e.span()))),
        // Regular identifier
        identifier.clone(),
    ));

    // Parse a symbolic color: category(-variant)?(-lightness)?
    // e.g., foreground, foreground-1, text-dark, accent-2-light
    let symbolic_color = color_category
        .then(
            just(Token::Minus)
                .ignore_then(number.clone())
                .or_not(),
        )
        .then(
            just(Token::Minus)
                .ignore_then(identifier.clone())
                .or_not(),
        )
        .try_map(|((cat_id, variant_num), lightness_id), span| {
            // Check if this is a valid symbolic color category
            if let Some(category) = is_color_category(&cat_id.node.0) {
                let variant = variant_num.map(|n| n.node as u8).filter(|&v| (1..=3).contains(&v));
                let lightness = lightness_id.and_then(|id| is_lightness_modifier(&id.node.0));

                Ok(StyleValue::Color(ColorValue::Symbolic {
                    category,
                    variant,
                    lightness,
                }))
            } else {
                Err(Rich::custom(span, "not a symbolic color"))
            }
        });

    let style_value = choice((
        // Hex colors like #ff0000 or #f00
        select! { Token::HexColor(c) => StyleValue::Color(ColorValue::Hex(c)) }
            .map_with(|v, e| Spanned::new(v, span_range(&e.span()))),
        // Symbolic colors (must come before plain identifiers)
        symbolic_color.map_with(|v, e| Spanned::new(v, span_range(&e.span()))),
        // Numbers (including negative via Minus token)
        just(Token::Minus)
            .or_not()
            .then(number)
            .map_with(|(neg, n), e| {
                let value = if neg.is_some() { -n.node } else { n.node };
                Spanned::new(
                    StyleValue::Number {
                        value,
                        unit: None,
                    },
                    span_range(&e.span()),
                )
            }),
        // Quoted strings
        string_literal.map(|s| Spanned::new(StyleValue::String(s.node), s.span)),
        // Plain identifiers: keywords or named colors
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
            .ignore_then(string_literal.clone())
            .map(|s| ShapeType::Icon { icon_name: s.node }),
        just(Token::Text)
            .ignore_then(string_literal)
            .map(|s| ShapeType::Text { content: s.node }),
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

        // Label declaration: `label { ... }` or `label: <element>`
        // The inner element can be any statement (shape, group, layout, etc.)
        let label_decl = just(Token::Label)
            .ignore_then(
                choice((
                    // Block form: label { text "Foo" [styles] }
                    stmt.clone()
                        .delimited_by(just(Token::BraceOpen), just(Token::BraceClose))
                        .map(|s: Spanned<Statement>| s.node),
                    // Inline form: label: text "Foo" [styles]
                    just(Token::Colon).ignore_then(stmt.clone()).map(|s: Spanned<Statement>| s.node),
                )),
            )
            .map(|inner| Statement::Label(Box::new(inner)));

        // All statements
        choice((
            constraint_decl.clone().map(Statement::Constraint),
            layout_decl.map(Statement::Layout),
            group_decl.map(Statement::Group),
            label_decl,
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

    #[test]
    fn test_parse_text() {
        let doc = parse(r#"text "Hello World" my_label"#).expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::Shape(s) => {
                match &s.shape_type.node {
                    ShapeType::Text { content } => assert_eq!(content, "Hello World"),
                    _ => panic!("Expected text shape"),
                }
                assert_eq!(s.name.as_ref().unwrap().node.as_str(), "my_label");
            }
            _ => panic!("Expected shape"),
        }
    }

    #[test]
    fn test_parse_text_with_modifiers() {
        let doc =
            parse(r#"text "Styled" styled_text [fill: red, font_size: 16]"#).expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::Shape(s) => {
                match &s.shape_type.node {
                    ShapeType::Text { content } => assert_eq!(content, "Styled"),
                    _ => panic!("Expected text shape"),
                }
                assert_eq!(s.modifiers.len(), 2);
            }
            _ => panic!("Expected shape"),
        }
    }

    #[test]
    fn test_parse_label_block_form() {
        // label { text "Foo" } - block form with braces
        let doc = parse(r#"group g { label { text "Foo" } rect a }"#).expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::Group(g) => {
                assert_eq!(g.children.len(), 2);
                // First child should be a Label
                match &g.children[0].node {
                    Statement::Label(inner) => {
                        match inner.as_ref() {
                            Statement::Shape(s) => {
                                assert!(matches!(s.shape_type.node, ShapeType::Text { .. }));
                            }
                            _ => panic!("Expected shape inside label"),
                        }
                    }
                    _ => panic!("Expected label statement"),
                }
            }
            _ => panic!("Expected group"),
        }
    }

    #[test]
    fn test_parse_label_inline_form() {
        // label: text "Foo" - inline form with colon
        let doc = parse(r#"group g { label: text "Bar" rect a }"#).expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::Group(g) => {
                assert_eq!(g.children.len(), 2);
                // First child should be a Label
                match &g.children[0].node {
                    Statement::Label(inner) => {
                        match inner.as_ref() {
                            Statement::Shape(s) => {
                                match &s.shape_type.node {
                                    ShapeType::Text { content } => assert_eq!(content, "Bar"),
                                    _ => panic!("Expected text shape"),
                                }
                            }
                            _ => panic!("Expected shape inside label"),
                        }
                    }
                    _ => panic!("Expected label statement"),
                }
            }
            _ => panic!("Expected group"),
        }
    }

    #[test]
    fn test_parse_label_with_shape() {
        // label { rect foo [fill: red] } - any shape as label
        let doc = parse(r#"group g { label { rect foo [fill: red] } rect a }"#).expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::Group(g) => {
                assert_eq!(g.children.len(), 2);
                // First child should be a Label with a rect inside
                match &g.children[0].node {
                    Statement::Label(inner) => {
                        match inner.as_ref() {
                            Statement::Shape(s) => {
                                assert!(matches!(s.shape_type.node, ShapeType::Rectangle));
                                assert_eq!(s.name.as_ref().unwrap().node.as_str(), "foo");
                                assert_eq!(s.modifiers.len(), 1);
                            }
                            _ => panic!("Expected shape inside label"),
                        }
                    }
                    _ => panic!("Expected label statement"),
                }
            }
            _ => panic!("Expected group"),
        }
    }

    #[test]
    fn test_parse_label_modifier_still_works() {
        // Old [label: "text"] modifier should still work
        let doc = parse(r#"rect foo [label: "Hello"]"#).expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::Shape(s) => {
                assert_eq!(s.modifiers.len(), 1);
                assert!(matches!(s.modifiers[0].node.key.node, StyleKey::Label));
                match &s.modifiers[0].node.value.node {
                    StyleValue::String(text) => assert_eq!(text, "Hello"),
                    _ => panic!("Expected string value"),
                }
            }
            _ => panic!("Expected shape"),
        }
    }

    // ==================== Symbolic Color Tests ====================

    #[test]
    fn test_parse_symbolic_color_foreground() {
        let doc = parse(r#"rect server [fill: foreground-1]"#).expect("Should parse");
        match &doc.statements[0].node {
            Statement::Shape(s) => {
                assert_eq!(s.modifiers.len(), 1);
                match &s.modifiers[0].node.value.node {
                    StyleValue::Color(ColorValue::Symbolic {
                        category,
                        variant,
                        lightness,
                    }) => {
                        assert!(matches!(category, ColorCategory::Foreground));
                        assert_eq!(*variant, Some(1));
                        assert!(lightness.is_none());
                    }
                    other => panic!("Expected symbolic color, got {:?}", other),
                }
            }
            _ => panic!("Expected shape"),
        }
    }

    #[test]
    fn test_parse_symbolic_color_text_dark() {
        let doc = parse(r#"rect server [fill: text-dark]"#).expect("Should parse");
        match &doc.statements[0].node {
            Statement::Shape(s) => {
                match &s.modifiers[0].node.value.node {
                    StyleValue::Color(ColorValue::Symbolic {
                        category,
                        variant,
                        lightness,
                    }) => {
                        assert!(matches!(category, ColorCategory::Text));
                        assert!(variant.is_none());
                        assert!(matches!(lightness, Some(Lightness::Dark)));
                    }
                    other => panic!("Expected symbolic color, got {:?}", other),
                }
            }
            _ => panic!("Expected shape"),
        }
    }

    #[test]
    fn test_parse_symbolic_color_accent_variant_light() {
        let doc = parse(r#"rect server [fill: accent-2-light]"#).expect("Should parse");
        match &doc.statements[0].node {
            Statement::Shape(s) => {
                match &s.modifiers[0].node.value.node {
                    StyleValue::Color(ColorValue::Symbolic {
                        category,
                        variant,
                        lightness,
                    }) => {
                        assert!(matches!(category, ColorCategory::Accent));
                        assert_eq!(*variant, Some(2));
                        assert!(matches!(lightness, Some(Lightness::Light)));
                    }
                    other => panic!("Expected symbolic color, got {:?}", other),
                }
            }
            _ => panic!("Expected shape"),
        }
    }

    #[test]
    fn test_parse_symbolic_color_background_base() {
        let doc = parse(r#"rect server [fill: background]"#).expect("Should parse");
        match &doc.statements[0].node {
            Statement::Shape(s) => {
                match &s.modifiers[0].node.value.node {
                    StyleValue::Color(ColorValue::Symbolic {
                        category,
                        variant,
                        lightness,
                    }) => {
                        assert!(matches!(category, ColorCategory::Background));
                        assert!(variant.is_none());
                        assert!(lightness.is_none());
                    }
                    other => panic!("Expected symbolic color, got {:?}", other),
                }
            }
            _ => panic!("Expected shape"),
        }
    }

    #[test]
    fn test_parse_named_color_passthrough() {
        // Named colors like 'red' should NOT be parsed as symbolic
        let doc = parse(r#"rect server [fill: red]"#).expect("Should parse");
        match &doc.statements[0].node {
            Statement::Shape(s) => {
                match &s.modifiers[0].node.value.node {
                    StyleValue::Keyword(name) => {
                        assert_eq!(name, "red");
                    }
                    other => panic!("Expected keyword for named color, got {:?}", other),
                }
            }
            _ => panic!("Expected shape"),
        }
    }

    #[test]
    fn test_parse_hex_color_passthrough() {
        let doc = parse(r#"rect server [fill: #ff0000]"#).expect("Should parse");
        match &doc.statements[0].node {
            Statement::Shape(s) => {
                match &s.modifiers[0].node.value.node {
                    StyleValue::Color(ColorValue::Hex(hex)) => {
                        assert_eq!(hex, "#ff0000");
                    }
                    other => panic!("Expected hex color, got {:?}", other),
                }
            }
            _ => panic!("Expected shape"),
        }
    }

    #[test]
    fn test_parse_mixed_colors() {
        // Mix of symbolic, named, and hex colors in one document
        let doc = parse(r#"
            rect a [fill: foreground-1]
            rect b [fill: red]
            rect c [fill: #00ff00]
        "#)
        .expect("Should parse");
        assert_eq!(doc.statements.len(), 3);

        // First: symbolic
        match &doc.statements[0].node {
            Statement::Shape(s) => match &s.modifiers[0].node.value.node {
                StyleValue::Color(ColorValue::Symbolic { .. }) => {}
                other => panic!("Expected symbolic, got {:?}", other),
            },
            _ => panic!("Expected shape"),
        }

        // Second: keyword (named color)
        match &doc.statements[1].node {
            Statement::Shape(s) => match &s.modifiers[0].node.value.node {
                StyleValue::Keyword(_) => {}
                other => panic!("Expected keyword, got {:?}", other),
            },
            _ => panic!("Expected shape"),
        }

        // Third: hex
        match &doc.statements[2].node {
            Statement::Shape(s) => match &s.modifiers[0].node.value.node {
                StyleValue::Color(ColorValue::Hex(_)) => {}
                other => panic!("Expected hex, got {:?}", other),
            },
            _ => panic!("Expected shape"),
        }
    }
}
