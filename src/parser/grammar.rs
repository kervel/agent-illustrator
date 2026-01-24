//! Parser implementation using chumsky

use chumsky::input::{Stream, ValueInput};
use chumsky::prelude::*;

use crate::parser::ast::*;
use crate::parser::lexer::Token;

/// Helper enum for parsing constraint equality expressions
#[derive(Debug, Clone)]
enum ConstraintExprKind {
    Property(PropertyRef),
    PropertyWithOffset(PropertyRef, f64),
    Constant(f64),
}

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
    // Note: We need to handle keyword tokens explicitly since they're not identifiers
    let style_key = choice((
        // Handle the "label" keyword token explicitly
        just(Token::Label).map_with(|_, e| Spanned::new(StyleKey::Label, span_range(&e.span()))),
        // Handle the "role" keyword token explicitly
        just(Token::Role).map_with(|_, e| Spanned::new(StyleKey::Role, span_range(&e.span()))),
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
                "x" => StyleKey::X,
                "y" => StyleKey::Y,
                "stroke_dasharray" => StyleKey::StrokeDasharray,
                "rotation" => StyleKey::Rotation,
                other => StyleKey::Custom(other.to_string()),
            };
            Spanned::new(key, id.span)
        }),
    ));

    // Parse a color category - identifier or "text" keyword (since text is reserved)
    let color_category = choice((
        // "text" keyword token maps to Text category
        just(Token::Text)
            .map_with(|_, e| Spanned::new(Identifier::new("text"), span_range(&e.span()))),
        // Regular identifier
        identifier.clone(),
    ));

    // Parse a symbolic color: category(-variant)?(-lightness)?
    // e.g., foreground, foreground-1, text-dark, accent-2-light
    let symbolic_color = color_category
        .then(just(Token::Minus).ignore_then(number.clone()).or_not())
        .then(just(Token::Minus).ignore_then(identifier.clone()).or_not())
        .try_map(|((cat_id, variant_num), lightness_id), span| {
            // Check if this is a valid symbolic color category
            if let Some(category) = is_color_category(&cat_id.node.0) {
                let variant = variant_num
                    .map(|n| n.node as u8)
                    .filter(|&v| (1..=3).contains(&v));
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
                    StyleValue::Number { value, unit: None },
                    span_range(&e.span()),
                )
            }),
        // Quoted strings
        string_literal.map(|s| Spanned::new(StyleValue::String(s.node), s.span)),
        // Handle "label" keyword as a keyword value (for [role: label])
        just(Token::Label).map_with(|_, e| {
            Spanned::new(
                StyleValue::Keyword("label".to_string()),
                span_range(&e.span()),
            )
        }),
        // Handle edge keywords as keyword values (for [label_position: left], etc.)
        just(Token::Left).map_with(|_, e| {
            Spanned::new(
                StyleValue::Keyword("left".to_string()),
                span_range(&e.span()),
            )
        }),
        just(Token::Right).map_with(|_, e| {
            Spanned::new(
                StyleValue::Keyword("right".to_string()),
                span_range(&e.span()),
            )
        }),
        just(Token::Top).map_with(|_, e| {
            Spanned::new(
                StyleValue::Keyword("top".to_string()),
                span_range(&e.span()),
            )
        }),
        just(Token::Bottom).map_with(|_, e| {
            Spanned::new(
                StyleValue::Keyword("bottom".to_string()),
                span_range(&e.span()),
            )
        }),
        just(Token::HorizontalCenter).map_with(|_, e| {
            Spanned::new(
                StyleValue::Keyword("horizontal_center".to_string()),
                span_range(&e.span()),
            )
        }),
        just(Token::VerticalCenter).map_with(|_, e| {
            Spanned::new(
                StyleValue::Keyword("vertical_center".to_string()),
                span_range(&e.span()),
            )
        }),
        // Center token (can be used in style values like [label_position: center])
        just(Token::Center).map_with(|_, e| {
            Spanned::new(
                StyleValue::Keyword("center".to_string()),
                span_range(&e.span()),
            )
        }),
        // center_x and center_y tokens
        just(Token::CenterXProp).map_with(|_, e| {
            Spanned::new(
                StyleValue::Keyword("center_x".to_string()),
                span_range(&e.span()),
            )
        }),
        just(Token::CenterYProp).map_with(|_, e| {
            Spanned::new(
                StyleValue::Keyword("center_y".to_string()),
                span_range(&e.span()),
            )
        }),
        // Identifiers can be either keyword values OR identifier references
        // Certain common keywords are recognized and stored as Keywords for backward compatibility
        identifier.map(|id| {
            let value = match id.node.as_str() {
                // Common style value keywords (not alignment edges)
                "center" | "direct" | "orthogonal" | "none" | "auto" | "solid" | "dashed"
                | "dotted" | "hidden" | "bold" | "italic" | "normal" | "start" | "middle"
                | "end" => StyleValue::Keyword(id.node.0.clone()),
                // Color keywords
                "red" | "green" | "blue" | "black" | "white" | "gray" | "grey" | "yellow"
                | "orange" | "purple" | "pink" | "cyan" | "magenta" | "transparent" => {
                    StyleValue::Keyword(id.node.0.clone())
                }
                // Everything else is an identifier reference (for [label: my_shape] syntax)
                _ => StyleValue::Identifier(id.node),
            };
            Spanned::new(value, id.span)
        }),
    ));

    let modifier = style_key
        .then_ignore(just(Token::Colon))
        .then(style_value.clone())
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

    // Constraint declaration - supports:
    // - `place a right-of b` - relational positioning
    // - `place a [x: 10]` - position offset only
    // - `place a right-of b [x: 10]` - relational with offset
    let constraint_decl = just(Token::Place)
        .ignore_then(identifier.clone())
        .then(position_relation.then(identifier.clone()).or_not())
        .then(modifier_block.clone().or_not())
        .map(|((subject, rel_anchor), mods)| {
            let (relation, anchor) = match rel_anchor {
                Some((rel, anch)) => (Some(rel), Some(anch)),
                None => (None, None),
            };
            ConstraintDecl {
                subject,
                relation,
                anchor,
                modifiers: mods.unwrap_or_default(),
            }
        });

    // Element path parser: identifier { "." identifier }
    // e.g., "a", "group1.item", "outer.inner.shape"
    let _element_path = identifier
        .clone()
        .separated_by(just(Token::Dot))
        .at_least(1)
        .collect::<Vec<_>>()
        .map_with(|segments, e| Spanned::new(ElementPath { segments }, span_range(&e.span())));

    // ==================== Constraint Parser (Feature 005) ====================

    // Property reference: element_path.property
    // Parse all dot-separated tokens, then split into path and property
    // The last segment must be a valid property, everything before is the element path

    // Helper to parse either an identifier or a keyword that could be a property
    let path_or_prop_segment = choice((
        // Keyword tokens that could appear in property position
        just(Token::CenterXProp)
            .map_with(|_, e| Spanned::new(Identifier::new("center_x"), span_range(&e.span()))),
        just(Token::CenterYProp)
            .map_with(|_, e| Spanned::new(Identifier::new("center_y"), span_range(&e.span()))),
        just(Token::Center)
            .map_with(|_, e| Spanned::new(Identifier::new("center"), span_range(&e.span()))),
        just(Token::Left)
            .map_with(|_, e| Spanned::new(Identifier::new("left"), span_range(&e.span()))),
        just(Token::Right)
            .map_with(|_, e| Spanned::new(Identifier::new("right"), span_range(&e.span()))),
        just(Token::Top)
            .map_with(|_, e| Spanned::new(Identifier::new("top"), span_range(&e.span()))),
        just(Token::Bottom)
            .map_with(|_, e| Spanned::new(Identifier::new("bottom"), span_range(&e.span()))),
        just(Token::HorizontalCenter).map_with(|_, e| {
            Spanned::new(Identifier::new("horizontal_center"), span_range(&e.span()))
        }),
        just(Token::VerticalCenter).map_with(|_, e| {
            Spanned::new(Identifier::new("vertical_center"), span_range(&e.span()))
        }),
        // Regular identifier
        identifier.clone(),
    ));

    let property_ref = path_or_prop_segment
        .clone()
        .separated_by(just(Token::Dot))
        .at_least(2)
        .collect::<Vec<_>>()
        .try_map(|segments, span: SimpleSpan| {
            // Last segment must be a property
            let last = segments.last().unwrap();
            let prop_opt = ConstraintProperty::from_str(last.node.as_str());

            match prop_opt {
                Some(prop) => {
                    let path_segments: Vec<_> = segments[..segments.len() - 1].to_vec();
                    let prop_span = last.span.clone();
                    Ok(PropertyRef {
                        element: Spanned::new(ElementPath { segments: path_segments }, span_range(&span)),
                        property: Spanned::new(prop, prop_span),
                    })
                }
                None => Err(Rich::custom(span, format!("'{}' is not a valid constraint property. Expected one of: x, y, width, height, left, right, top, bottom, center, center_x, center_y", last.node.as_str()))),
            }
        });

    // Parse offset: + number or - number
    let offset = choice((
        just(Token::Plus)
            .ignore_then(number.clone())
            .map(|n| n.node),
        just(Token::Minus)
            .ignore_then(number.clone())
            .map(|n| -n.node),
    ));

    // Constraint expression parsers

    // Midpoint: target.prop = midpoint(a, b) or target.prop = midpoint(a, b) + offset
    let midpoint_expr = property_ref
        .clone()
        .then_ignore(just(Token::Equals))
        .then_ignore(just(Token::Midpoint))
        .then_ignore(just(Token::ParenOpen))
        .then(identifier.clone())
        .then_ignore(just(Token::Comma))
        .then(identifier.clone())
        .then_ignore(just(Token::ParenClose))
        .then(offset.clone().or_not())
        .map(|(((target, a), b), off)| ConstraintExpr::Midpoint {
            target,
            a,
            b,
            offset: off.unwrap_or(0.0),
        });

    // Contains: container contains a, b, c [padding: N]
    let contains_expr = identifier
        .clone()
        .then_ignore(just(Token::Contains))
        .then(
            identifier
                .clone()
                .separated_by(just(Token::Comma))
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .then(modifier_block.clone().or_not())
        .map(|((container, elements), modifiers)| {
            // Extract padding from modifiers if present
            let padding = modifiers.as_ref().and_then(|mods| {
                mods.iter().find_map(|m| {
                    if let StyleKey::Custom(k) = &m.node.key.node {
                        if k == "padding" {
                            if let StyleValue::Number { value, .. } = &m.node.value.node {
                                return Some(*value);
                            }
                        }
                    }
                    None
                })
            });
            ConstraintExpr::Contains {
                container,
                elements,
                padding,
            }
        });

    // Inequality: a.prop >= value or a.prop <= value
    let ge_expr = property_ref
        .clone()
        .then_ignore(just(Token::GreaterOrEqual))
        .then(just(Token::Minus).or_not().then(number.clone()))
        .map(|(left, (neg, n))| {
            let value = if neg.is_some() { -n.node } else { n.node };
            ConstraintExpr::GreaterOrEqual { left, value }
        });

    let le_expr = property_ref
        .clone()
        .then_ignore(just(Token::LessOrEqual))
        .then(just(Token::Minus).or_not().then(number.clone()))
        .map(|(left, (neg, n))| {
            let value = if neg.is_some() { -n.node } else { n.node };
            ConstraintExpr::LessOrEqual { left, value }
        });

    // Equality with property: a.prop = b.prop [+ offset]
    // Or constant: a.prop = value
    let equality_expr = property_ref.clone().then_ignore(just(Token::Equals)).then(
        // Try property ref with optional offset first
        property_ref
            .clone()
            .then(offset.clone().or_not())
            .map(|(right, off)| {
                if let Some(offset) = off {
                    ConstraintExprKind::PropertyWithOffset(right, offset)
                } else {
                    ConstraintExprKind::Property(right)
                }
            })
            // Or just a constant number
            .or(just(Token::Minus)
                .or_not()
                .then(number.clone())
                .map(|(neg, n)| {
                    let value = if neg.is_some() { -n.node } else { n.node };
                    ConstraintExprKind::Constant(value)
                })),
    );

    // Build the final constraint expression from equality
    let equality_constraint = equality_expr.map(|(left, kind)| match kind {
        ConstraintExprKind::Property(right) => ConstraintExpr::Equal { left, right },
        ConstraintExprKind::PropertyWithOffset(right, offset) => ConstraintExpr::EqualWithOffset {
            left,
            right,
            offset,
        },
        ConstraintExprKind::Constant(value) => ConstraintExpr::Constant { left, value },
    });

    // All constraint expressions (order matters - try more specific first)
    let constraint_expr = choice((
        midpoint_expr,
        contains_expr,
        ge_expr,
        le_expr,
        equality_constraint,
    ));

    // Constrain declaration: constrain <expr>
    let constrain_decl = just(Token::Constrain)
        .ignore_then(constraint_expr)
        .map(|expr| ConstrainDecl { expr });

    // ==================== Template Parsing (Feature 005) ====================

    // Export declaration: export name1, name2
    let export_decl = just(Token::Export)
        .ignore_then(
            identifier
                .clone()
                .separated_by(just(Token::Comma))
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .map(|exports| ExportDecl { exports });

    // Parameter definition: name: default_value
    let param_def = identifier
        .clone()
        .then_ignore(just(Token::Colon))
        .then(style_value.clone())
        .map(|(name, default_value)| ParameterDef {
            name,
            default_value,
        });

    // Parameter list: (param1: val1, param2: val2)
    let param_list = param_def
        .separated_by(just(Token::Comma))
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just(Token::ParenOpen), just(Token::ParenClose))
        .or_not()
        .map(|opt| opt.unwrap_or_default());

    // File template: template "name" from "path"
    let file_template = just(Token::Template)
        .ignore_then(string_literal.clone())
        .then_ignore(just(Token::From))
        .then(string_literal.clone())
        .map(|(name, path)| {
            let source_type = if path.node.ends_with(".svg") {
                TemplateSourceType::Svg
            } else {
                TemplateSourceType::Ail
            };
            Statement::TemplateDecl(TemplateDecl {
                name: Spanned::new(Identifier::new(name.node), name.span),
                source_type,
                source_path: Some(path),
                parameters: vec![],
                body: None,
            })
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
            .ignore_then(choice((
                // Block form: label { text "Foo" [styles] }
                stmt.clone()
                    .delimited_by(just(Token::BraceOpen), just(Token::BraceClose))
                    .map(|s: Spanned<Statement>| s.node),
                // Inline form: label: text "Foo" [styles]
                just(Token::Colon)
                    .ignore_then(stmt.clone())
                    .map(|s: Spanned<Statement>| s.node),
            )))
            .map(|inner| Statement::Label(Box::new(inner)));

        // Inline template: template "name" (params) { body }
        let inline_template = just(Token::Template)
            .ignore_then(string_literal.clone())
            .then(param_list.clone())
            .then(
                stmt.clone()
                    .repeated()
                    .collect::<Vec<_>>()
                    .delimited_by(just(Token::BraceOpen), just(Token::BraceClose)),
            )
            .map(|((name, parameters), body)| {
                Statement::TemplateDecl(TemplateDecl {
                    name: Spanned::new(Identifier::new(name.node), name.span),
                    source_type: TemplateSourceType::Inline,
                    source_path: None,
                    parameters,
                    body: Some(body),
                })
            });

        // Template instance: template_name instance_name [args]
        // Note: This needs to be parsed carefully to not conflict with shape_decl
        // We use a special approach where template instances use plain identifiers for
        // both template name and instance name, without a keyword prefix.
        // For now, we support the syntax: identifier identifier [params]
        // where the first identifier is the template name and second is instance name.
        // Template instances will be distinguished from connections by not having ->/<- operators.
        let template_instance = identifier
            .clone()
            .then(identifier.clone())
            .then(modifier_block.clone().or_not())
            .try_map(|((template_name, instance_name), mods), _span| {
                // Convert modifiers to argument list
                let arguments: Vec<(Spanned<Identifier>, Spanned<StyleValue>)> = mods
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|m| {
                        // Convert StyleKey to Identifier
                        let key_str = match &m.node.key.node {
                            StyleKey::Fill => "fill".to_string(),
                            StyleKey::Stroke => "stroke".to_string(),
                            StyleKey::StrokeWidth => "stroke_width".to_string(),
                            StyleKey::Size => "size".to_string(),
                            StyleKey::Width => "width".to_string(),
                            StyleKey::Height => "height".to_string(),
                            StyleKey::Label => "label".to_string(),
                            StyleKey::Custom(s) => s.clone(),
                            _ => return None,
                        };
                        Some((
                            Spanned::new(Identifier::new(key_str), m.node.key.span),
                            m.node.value,
                        ))
                    })
                    .collect();

                Ok(Statement::TemplateInstance(TemplateInstance {
                    template_name,
                    instance_name,
                    arguments,
                }))
            });

        // All statements
        // Note: Order matters! More specific patterns should come first.
        // - constrain_decl before others (starts with 'constrain')
        // - constraint_decl (place) before others
        // - file_template before inline_template (both start with 'template')
        // - inline_template after file_template
        // - export_decl after templates
        // - layout_decl, group_decl, label_decl
        // - connection_decl before template_instance (both start with identifier)
        // - shape_decl before template_instance (rect, circle, etc. are keywords)
        // - template_instance last (identifier identifier pattern is very general)
        choice((
            constrain_decl.clone().map(Statement::Constrain),
            constraint_decl.clone().map(Statement::Constraint),
            file_template.clone(),
            inline_template,
            export_decl.clone().map(Statement::Export),
            layout_decl.map(Statement::Layout),
            group_decl.map(Statement::Group),
            label_decl,
            connection_decl.clone().map(Statement::Connection),
            shape_decl.clone().map(Statement::Shape),
            // Template instance must be last since it matches "identifier identifier"
            // which could conflict with other patterns
            template_instance,
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
                assert!(matches!(
                    c.relation.as_ref().unwrap().node,
                    PositionRelation::RightOf
                ));
                assert_eq!(c.anchor.as_ref().unwrap().node.as_str(), "server");
                assert!(c.modifiers.is_empty());
            }
            _ => panic!("Expected constraint"),
        }
    }

    #[test]
    fn test_parse_constraint_with_offset() {
        let doc = parse("place element [x: 10, y: 20]").expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::Constraint(c) => {
                assert_eq!(c.subject.node.as_str(), "element");
                assert!(c.relation.is_none());
                assert!(c.anchor.is_none());
                assert_eq!(c.modifiers.len(), 2);
            }
            _ => panic!("Expected constraint"),
        }
    }

    #[test]
    fn test_parse_constraint_relational_with_offset() {
        let doc = parse("place a right-of b [x: 10]").expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::Constraint(c) => {
                assert_eq!(c.subject.node.as_str(), "a");
                assert!(matches!(
                    c.relation.as_ref().unwrap().node,
                    PositionRelation::RightOf
                ));
                assert_eq!(c.anchor.as_ref().unwrap().node.as_str(), "b");
                assert_eq!(c.modifiers.len(), 1);
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
                    Statement::Label(inner) => match inner.as_ref() {
                        Statement::Shape(s) => {
                            assert!(matches!(s.shape_type.node, ShapeType::Text { .. }));
                        }
                        _ => panic!("Expected shape inside label"),
                    },
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
                    Statement::Label(inner) => match inner.as_ref() {
                        Statement::Shape(s) => match &s.shape_type.node {
                            ShapeType::Text { content } => assert_eq!(content, "Bar"),
                            _ => panic!("Expected text shape"),
                        },
                        _ => panic!("Expected shape inside label"),
                    },
                    _ => panic!("Expected label statement"),
                }
            }
            _ => panic!("Expected group"),
        }
    }

    #[test]
    fn test_parse_label_with_shape() {
        // label { rect foo [fill: red] } - any shape as label
        let doc =
            parse(r#"group g { label { rect foo [fill: red] } rect a }"#).expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::Group(g) => {
                assert_eq!(g.children.len(), 2);
                // First child should be a Label with a rect inside
                match &g.children[0].node {
                    Statement::Label(inner) => match inner.as_ref() {
                        Statement::Shape(s) => {
                            assert!(matches!(s.shape_type.node, ShapeType::Rectangle));
                            assert_eq!(s.name.as_ref().unwrap().node.as_str(), "foo");
                            assert_eq!(s.modifiers.len(), 1);
                        }
                        _ => panic!("Expected shape inside label"),
                    },
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
            Statement::Shape(s) => match &s.modifiers[0].node.value.node {
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
            },
            _ => panic!("Expected shape"),
        }
    }

    #[test]
    fn test_parse_symbolic_color_accent_variant_light() {
        let doc = parse(r#"rect server [fill: accent-2-light]"#).expect("Should parse");
        match &doc.statements[0].node {
            Statement::Shape(s) => match &s.modifiers[0].node.value.node {
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
            },
            _ => panic!("Expected shape"),
        }
    }

    #[test]
    fn test_parse_symbolic_color_background_base() {
        let doc = parse(r#"rect server [fill: background]"#).expect("Should parse");
        match &doc.statements[0].node {
            Statement::Shape(s) => match &s.modifiers[0].node.value.node {
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
            },
            _ => panic!("Expected shape"),
        }
    }

    #[test]
    fn test_parse_named_color_passthrough() {
        // Named colors like 'red' should NOT be parsed as symbolic
        let doc = parse(r#"rect server [fill: red]"#).expect("Should parse");
        match &doc.statements[0].node {
            Statement::Shape(s) => match &s.modifiers[0].node.value.node {
                StyleValue::Keyword(name) => {
                    assert_eq!(name, "red");
                }
                other => panic!("Expected keyword for named color, got {:?}", other),
            },
            _ => panic!("Expected shape"),
        }
    }

    #[test]
    fn test_parse_hex_color_passthrough() {
        let doc = parse(r#"rect server [fill: #ff0000]"#).expect("Should parse");
        match &doc.statements[0].node {
            Statement::Shape(s) => match &s.modifiers[0].node.value.node {
                StyleValue::Color(ColorValue::Hex(hex)) => {
                    assert_eq!(hex, "#ff0000");
                }
                other => panic!("Expected hex color, got {:?}", other),
            },
            _ => panic!("Expected shape"),
        }
    }

    #[test]
    fn test_parse_mixed_colors() {
        // Mix of symbolic, named, and hex colors in one document
        let doc = parse(
            r#"
            rect a [fill: foreground-1]
            rect b [fill: red]
            rect c [fill: #00ff00]
        "#,
        )
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

    #[test]
    fn test_parse_role_modifier() {
        // Parse [role: label] modifier
        let doc = parse(r#"text "Title" [role: label]"#).expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::Shape(s) => {
                assert_eq!(s.modifiers.len(), 1);
                assert!(matches!(s.modifiers[0].node.key.node, StyleKey::Role));
                match &s.modifiers[0].node.value.node {
                    StyleValue::Keyword(k) => assert_eq!(k, "label"),
                    _ => panic!("Expected keyword value"),
                }
            }
            _ => panic!("Expected shape"),
        }
    }

    #[test]
    fn test_parse_label_identifier_reference() {
        // Parse [label: my_label] where my_label is an identifier reference
        let doc = parse("a -> b [label: my_label]").expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::Connection(c) => {
                assert_eq!(c.modifiers.len(), 1);
                assert!(matches!(c.modifiers[0].node.key.node, StyleKey::Label));
                match &c.modifiers[0].node.value.node {
                    StyleValue::Identifier(id) => assert_eq!(id.as_str(), "my_label"),
                    _ => panic!(
                        "Expected identifier value, got {:?}",
                        c.modifiers[0].node.value.node
                    ),
                }
            }
            _ => panic!("Expected connection"),
        }
    }

    // ==================== Constrain Syntax Tests (Feature 005) ====================

    #[test]
    fn test_parse_constrain_equality() {
        let doc = parse("constrain a.left = b.left").expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::Constrain(c) => match &c.expr {
                ConstraintExpr::Equal { left, right } => {
                    assert_eq!(left.element.node.leaf().as_str(), "a");
                    assert!(matches!(left.property.node, ConstraintProperty::Left));
                    assert_eq!(right.element.node.leaf().as_str(), "b");
                    assert!(matches!(right.property.node, ConstraintProperty::Left));
                }
                other => panic!("Expected Equal, got {:?}", other),
            },
            other => panic!("Expected Constrain, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_constrain_with_offset() {
        let doc = parse("constrain a.left = b.right + 20").expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::Constrain(c) => match &c.expr {
                ConstraintExpr::EqualWithOffset {
                    left,
                    right,
                    offset,
                } => {
                    assert_eq!(left.element.node.leaf().as_str(), "a");
                    assert!(matches!(left.property.node, ConstraintProperty::Left));
                    assert_eq!(right.element.node.leaf().as_str(), "b");
                    assert!(matches!(right.property.node, ConstraintProperty::Right));
                    assert!((offset - 20.0).abs() < 0.001);
                }
                other => panic!("Expected EqualWithOffset, got {:?}", other),
            },
            other => panic!("Expected Constrain, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_constrain_negative_offset() {
        let doc = parse("constrain a.x = b.x - 10").expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::Constrain(c) => match &c.expr {
                ConstraintExpr::EqualWithOffset { offset, .. } => {
                    assert!((offset - (-10.0)).abs() < 0.001);
                }
                other => panic!("Expected EqualWithOffset, got {:?}", other),
            },
            other => panic!("Expected Constrain, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_constrain_constant() {
        let doc = parse("constrain a.width = 100").expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::Constrain(c) => match &c.expr {
                ConstraintExpr::Constant { left, value } => {
                    assert_eq!(left.element.node.leaf().as_str(), "a");
                    assert!(matches!(left.property.node, ConstraintProperty::Width));
                    assert!((value - 100.0).abs() < 0.001);
                }
                other => panic!("Expected Constant, got {:?}", other),
            },
            other => panic!("Expected Constrain, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_constrain_greater_or_equal() {
        let doc = parse("constrain a.width >= 50").expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::Constrain(c) => match &c.expr {
                ConstraintExpr::GreaterOrEqual { left, value } => {
                    assert_eq!(left.element.node.leaf().as_str(), "a");
                    assert!(matches!(left.property.node, ConstraintProperty::Width));
                    assert!((value - 50.0).abs() < 0.001);
                }
                other => panic!("Expected GreaterOrEqual, got {:?}", other),
            },
            other => panic!("Expected Constrain, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_constrain_less_or_equal() {
        let doc = parse("constrain a.height <= 200").expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::Constrain(c) => match &c.expr {
                ConstraintExpr::LessOrEqual { left, value } => {
                    assert_eq!(left.element.node.leaf().as_str(), "a");
                    assert!(matches!(left.property.node, ConstraintProperty::Height));
                    assert!((value - 200.0).abs() < 0.001);
                }
                other => panic!("Expected LessOrEqual, got {:?}", other),
            },
            other => panic!("Expected Constrain, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_constrain_midpoint() {
        let doc = parse("constrain a.center_x = midpoint(b, c)").expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::Constrain(c) => match &c.expr {
                ConstraintExpr::Midpoint {
                    target,
                    a,
                    b,
                    offset,
                } => {
                    assert_eq!(target.element.node.leaf().as_str(), "a");
                    assert!(matches!(target.property.node, ConstraintProperty::CenterX));
                    assert_eq!(a.node.as_str(), "b");
                    assert_eq!(b.node.as_str(), "c");
                    assert_eq!(*offset, 0.0); // No offset specified
                }
                other => panic!("Expected Midpoint, got {:?}", other),
            },
            other => panic!("Expected Constrain, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_constrain_midpoint_with_offset() {
        // Test positive offset
        let doc = parse("constrain a.center_x = midpoint(b, c) + 50").expect("Should parse");
        match &doc.statements[0].node {
            Statement::Constrain(c) => match &c.expr {
                ConstraintExpr::Midpoint { offset, .. } => {
                    assert_eq!(*offset, 50.0);
                }
                other => panic!("Expected Midpoint, got {:?}", other),
            },
            other => panic!("Expected Constrain, got {:?}", other),
        }

        // Test negative offset
        let doc = parse("constrain a.center_x = midpoint(b, c) - 80").expect("Should parse");
        match &doc.statements[0].node {
            Statement::Constrain(c) => match &c.expr {
                ConstraintExpr::Midpoint { offset, .. } => {
                    assert_eq!(*offset, -80.0);
                }
                other => panic!("Expected Midpoint, got {:?}", other),
            },
            other => panic!("Expected Constrain, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_constrain_contains() {
        let doc = parse("constrain container contains a, b, c").expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::Constrain(c) => match &c.expr {
                ConstraintExpr::Contains {
                    container,
                    elements,
                    padding,
                } => {
                    assert_eq!(container.node.as_str(), "container");
                    assert_eq!(elements.len(), 3);
                    assert_eq!(elements[0].node.as_str(), "a");
                    assert_eq!(elements[1].node.as_str(), "b");
                    assert_eq!(elements[2].node.as_str(), "c");
                    assert!(padding.is_none());
                }
                other => panic!("Expected Contains, got {:?}", other),
            },
            other => panic!("Expected Constrain, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_constrain_contains_with_padding() {
        let doc = parse("constrain container contains a, b [padding: 20]").expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::Constrain(c) => match &c.expr {
                ConstraintExpr::Contains {
                    container,
                    elements,
                    padding,
                } => {
                    assert_eq!(container.node.as_str(), "container");
                    assert_eq!(elements.len(), 2);
                    assert!(padding.is_some());
                    assert!((padding.unwrap() - 20.0).abs() < 0.001);
                }
                other => panic!("Expected Contains, got {:?}", other),
            },
            other => panic!("Expected Constrain, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_constrain_center_properties() {
        // Test all center property keywords
        let doc = parse("constrain a.center_x = b.center_y").expect("Should parse");
        match &doc.statements[0].node {
            Statement::Constrain(c) => match &c.expr {
                ConstraintExpr::Equal { left, right } => {
                    assert!(matches!(left.property.node, ConstraintProperty::CenterX));
                    assert!(matches!(right.property.node, ConstraintProperty::CenterY));
                }
                _ => panic!("Expected Equal"),
            },
            _ => panic!("Expected Constrain"),
        }

        // Test "center" property
        let doc2 = parse("constrain a.center = b.center").expect("Should parse");
        match &doc2.statements[0].node {
            Statement::Constrain(c) => match &c.expr {
                ConstraintExpr::Equal { left, right } => {
                    assert!(matches!(left.property.node, ConstraintProperty::Center));
                    assert!(matches!(right.property.node, ConstraintProperty::Center));
                }
                _ => panic!("Expected Equal"),
            },
            _ => panic!("Expected Constrain"),
        }
    }

    #[test]
    fn test_parse_constrain_with_nested_path() {
        let doc = parse("constrain group1.item.left = group2.other.left").expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::Constrain(c) => match &c.expr {
                ConstraintExpr::Equal { left, right } => {
                    // First path: group1.item
                    assert_eq!(left.element.node.segments.len(), 2);
                    assert_eq!(left.element.node.segments[0].node.as_str(), "group1");
                    assert_eq!(left.element.node.segments[1].node.as_str(), "item");
                    // Second path: group2.other
                    assert_eq!(right.element.node.segments.len(), 2);
                    assert_eq!(right.element.node.segments[0].node.as_str(), "group2");
                    assert_eq!(right.element.node.segments[1].node.as_str(), "other");
                }
                other => panic!("Expected Equal, got {:?}", other),
            },
            other => panic!("Expected Constrain, got {:?}", other),
        }
    }

    // ==================== Template Parsing Tests ====================

    #[test]
    fn test_parse_file_template_svg() {
        let doc = parse(r#"template "box" from "icons/box.svg""#).expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::TemplateDecl(t) => {
                assert_eq!(t.name.node.as_str(), "box");
                assert_eq!(t.source_type, TemplateSourceType::Svg);
                assert_eq!(t.source_path.as_ref().unwrap().node, "icons/box.svg");
                assert!(t.body.is_none());
                assert!(t.parameters.is_empty());
            }
            other => panic!("Expected TemplateDecl, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_file_template_ail() {
        let doc = parse(r#"template "component" from "lib/component.ail""#).expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::TemplateDecl(t) => {
                assert_eq!(t.name.node.as_str(), "component");
                assert_eq!(t.source_type, TemplateSourceType::Ail);
                assert_eq!(
                    t.source_path.as_ref().unwrap().node,
                    "lib/component.ail"
                );
            }
            other => panic!("Expected TemplateDecl, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_inline_template() {
        let doc = parse(
            r#"template "server" {
                rect box [fill: blue]
                text "Server" title
            }"#,
        )
        .expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::TemplateDecl(t) => {
                assert_eq!(t.name.node.as_str(), "server");
                assert_eq!(t.source_type, TemplateSourceType::Inline);
                assert!(t.source_path.is_none());
                assert_eq!(t.body.as_ref().unwrap().len(), 2);
            }
            other => panic!("Expected TemplateDecl, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_inline_template_with_params() {
        let doc = parse(
            r#"template "box" (fill: blue, size: 50) {
                rect shape [fill: fill, size: size]
            }"#,
        )
        .expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::TemplateDecl(t) => {
                assert_eq!(t.name.node.as_str(), "box");
                assert_eq!(t.parameters.len(), 2);
                assert_eq!(t.parameters[0].name.node.as_str(), "fill");
                assert_eq!(t.parameters[1].name.node.as_str(), "size");
            }
            other => panic!("Expected TemplateDecl, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_export_declaration() {
        let doc = parse("export port1, port2, port3").expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::Export(e) => {
                assert_eq!(e.exports.len(), 3);
                assert_eq!(e.exports[0].node.as_str(), "port1");
                assert_eq!(e.exports[1].node.as_str(), "port2");
                assert_eq!(e.exports[2].node.as_str(), "port3");
            }
            other => panic!("Expected Export, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_template_instance() {
        let doc = parse("server myserver [fill: red, size: 100]").expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::TemplateInstance(inst) => {
                assert_eq!(inst.template_name.node.as_str(), "server");
                assert_eq!(inst.instance_name.node.as_str(), "myserver");
                assert_eq!(inst.arguments.len(), 2);
            }
            other => panic!("Expected TemplateInstance, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_template_with_export() {
        let doc = parse(
            r#"template "connector" {
                circle port_in
                circle port_out
                export port_in, port_out
            }"#,
        )
        .expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::TemplateDecl(t) => {
                assert_eq!(t.body.as_ref().unwrap().len(), 3);
                // Check last statement is export
                match &t.body.as_ref().unwrap()[2].node {
                    Statement::Export(e) => {
                        assert_eq!(e.exports.len(), 2);
                    }
                    _ => panic!("Expected Export as last statement"),
                }
            }
            other => panic!("Expected TemplateDecl, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_rotation_modifier() {
        let input = r#"rect box [rotation: 45]"#;
        let doc = parse(input).expect("should parse");
        assert_eq!(doc.statements.len(), 1);

        if let Statement::Shape(shape) = &doc.statements[0].node {
            assert_eq!(shape.modifiers.len(), 1);
            assert!(matches!(
                shape.modifiers[0].node.key.node,
                StyleKey::Rotation
            ));
            if let StyleValue::Number { value, .. } = &shape.modifiers[0].node.value.node {
                assert!((value - 45.0).abs() < f64::EPSILON);
            } else {
                panic!("Expected number value");
            }
        } else {
            panic!("Expected shape statement");
        }
    }
}
