//! Parser implementation using chumsky

use chumsky::input::{Stream, ValueInput};
use chumsky::prelude::*;

use crate::parser::ast::*;
use crate::parser::lexer::Token;

// Path parsing helper types imported from AST (Feature 007):
// - PathDecl, PathBody, PathCommand
// - VertexDecl, VertexPosition
// - LineToDecl, ArcToDecl
// - ArcParams, SweepDirection
// All are available via the ast::* glob import above

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
        "secondary" => Some(ColorCategory::Secondary),
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

// ==================== Path Shape Parsers (Feature 007) ====================

/// Parsed modifier value - can be a number, sweep direction, or identifier (Feature 008)
#[derive(Debug, Clone)]
enum ParsedModifierValue {
    Number(f64),
    Sweep(SweepDirection),
    Identifier(Spanned<Identifier>),  // Feature 008: for via references
}

/// Helper struct for parsing arc modifiers within brackets
#[derive(Debug, Clone, Default)]
struct ParsedArcModifiers {
    x: Option<f64>,
    y: Option<f64>,
    radius: Option<f64>,
    bulge: Option<f64>,
    sweep: Option<SweepDirection>,
    via: Option<Spanned<Identifier>>,  // Feature 008: steering vertex reference
}

impl ParsedArcModifiers {
    fn into_position_and_params(self) -> (Option<VertexPosition>, ArcParams) {
        let position = if self.x.is_some() || self.y.is_some() {
            Some(VertexPosition {
                x: self.x,
                y: self.y,
            })
        } else {
            None
        };

        let params = if let Some(radius) = self.radius {
            ArcParams::Radius {
                radius,
                sweep: self.sweep.unwrap_or_default(),
            }
        } else if let Some(bulge) = self.bulge {
            ArcParams::Bulge(bulge)
        } else {
            ArcParams::default()
        };

        (position, params)
    }
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
                // Feature 008: added "curved" for curved routing
                "center" | "direct" | "orthogonal" | "curved" | "none" | "auto" | "solid" | "dashed"
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
    ))
    .boxed(); // Feature 008: boxed() for faster compilation (chumsky trait solving)

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
        .delimited_by(just(Token::BracketOpen), just(Token::BracketClose))
        .boxed(); // boxed() for faster compilation

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
        })
        .boxed(); // boxed() for faster compilation

    // Connection operators
    let connection_op = choice((
        just(Token::ArrowBoth).to(ConnectionDirection::Bidirectional),
        just(Token::Arrow).to(ConnectionDirection::Forward),
        just(Token::ArrowBack).to(ConnectionDirection::Backward),
        just(Token::Dash).to(ConnectionDirection::Undirected),
    ));

    // Anchor name parser: accepts both identifiers and edge keywords (top, bottom, left, right, etc.)
    // This is needed because edge keywords are lexed separately from identifiers
    let anchor_name = choice((
        select! { Token::Ident(s) => s },
        just(Token::Top).to("top".to_string()),
        just(Token::Bottom).to("bottom".to_string()),
        just(Token::Left).to("left".to_string()),
        just(Token::Right).to("right".to_string()),
        just(Token::HorizontalCenter).to("horizontal_center".to_string()),
        just(Token::VerticalCenter).to("vertical_center".to_string()),
    ))
    .map_with(|name, e| Spanned::new(name, span_range(&e.span())));

    // Anchor reference parser: identifier { "." anchor_name }?
    // Parses either:
    //   - `element` -> AnchorReference with anchor=None
    //   - `element.anchor_name` -> AnchorReference with anchor=Some
    let anchor_reference = identifier
        .clone()
        .then(just(Token::Dot).ignore_then(anchor_name).or_not())
        .map(|(element, anchor_opt)| {
            match anchor_opt {
                Some(anchor_name) => AnchorReference::with_anchor(element, anchor_name),
                None => AnchorReference::element_only(element),
            }
        });

    // Connection declaration (supports chained: a -> b -> c [modifiers])
    // Feature 009: Now supports anchor syntax (a.right -> b.left)
    let connection_decl = anchor_reference
        .clone()
        .then(
            connection_op
                .then(anchor_reference.clone())
                .repeated()
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .then(modifier_block.clone().or_not())
        .map(|((first, segments), modifiers)| {
            let modifiers = modifiers.unwrap_or_default();
            let len = segments.len();
            let mut result = Vec::with_capacity(len);
            let mut from = first;
            for (i, (direction, to)) in segments.into_iter().enumerate() {
                let is_last = i == len - 1;
                result.push(ConnectionDecl {
                    from: from.clone(),
                    to: to.clone(),
                    direction,
                    // Only the last segment gets modifiers
                    modifiers: if is_last { modifiers.clone() } else { vec![] },
                });
                from = to;
            }
            result
        })
        .boxed(); // boxed() for faster compilation

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
        })
        .boxed(); // boxed() for faster compilation

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
    ))
    .boxed(); // boxed() for faster compilation (chumsky trait solving)

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

    // ==================== Path Shape Parsers (Feature 007) ====================

    // Parse sweep direction for arcs
    let sweep_direction = choice((
        just(Token::Clockwise).to(SweepDirection::Clockwise),
        just(Token::Cw).to(SweepDirection::Clockwise),
        just(Token::Counterclockwise).to(SweepDirection::Counterclockwise),
        just(Token::Ccw).to(SweepDirection::Counterclockwise),
    ));

    // Parse a single position or arc modifier: x: 10, radius: 5, sweep: clockwise, via: ctrl, etc.
    let path_modifier_spec = choice((
        // Position specs
        just(Token::Ident("x".into()))
            .ignore_then(just(Token::Colon))
            .ignore_then(just(Token::Minus).or_not().then(number.clone()))
            .map(|(neg, n)| {
                let val = if neg.is_some() { -n.node } else { n.node };
                ("x", ParsedModifierValue::Number(val))
            }),
        just(Token::Ident("y".into()))
            .ignore_then(just(Token::Colon))
            .ignore_then(just(Token::Minus).or_not().then(number.clone()))
            .map(|(neg, n)| {
                let val = if neg.is_some() { -n.node } else { n.node };
                ("y", ParsedModifierValue::Number(val))
            }),
        just(Token::Right)
            .ignore_then(just(Token::Colon))
            .ignore_then(number.clone())
            .map(|n| ("right", ParsedModifierValue::Number(n.node))),
        just(Token::Left)
            .ignore_then(just(Token::Colon))
            .ignore_then(number.clone())
            .map(|n| ("left", ParsedModifierValue::Number(n.node))),
        just(Token::Up)
            .ignore_then(just(Token::Colon))
            .ignore_then(number.clone())
            .map(|n| ("up", ParsedModifierValue::Number(n.node))),
        just(Token::Down)
            .ignore_then(just(Token::Colon))
            .ignore_then(number.clone())
            .map(|n| ("down", ParsedModifierValue::Number(n.node))),
        // Arc specs
        just(Token::Ident("radius".into()))
            .ignore_then(just(Token::Colon))
            .ignore_then(number.clone())
            .map(|n| ("radius", ParsedModifierValue::Number(n.node))),
        just(Token::Ident("bulge".into()))
            .ignore_then(just(Token::Colon))
            .ignore_then(just(Token::Minus).or_not().then(number.clone()))
            .map(|(neg, n)| {
                let val = if neg.is_some() { -n.node } else { n.node };
                ("bulge", ParsedModifierValue::Number(val))
            }),
        just(Token::Ident("sweep".into()))
            .ignore_then(just(Token::Colon))
            .ignore_then(sweep_direction.clone())
            .map(|s| ("sweep", ParsedModifierValue::Sweep(s))),
        // Feature 008: via reference for curve steering vertex
        just(Token::Ident("via".into()))
            .ignore_then(just(Token::Colon))
            .ignore_then(identifier.clone())
            .map(|id| ("via", ParsedModifierValue::Identifier(id))),
    ))
    .boxed(); // boxed() for faster compilation (chumsky trait solving)

    // Parse a modifier block for path commands: [x: 10, y: 20] or [radius: 5, sweep: cw, via: ctrl]
    let path_modifier_block = path_modifier_spec
        .separated_by(just(Token::Comma))
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just(Token::BracketOpen), just(Token::BracketClose))
        .map(|specs| {
            let mut mods = ParsedArcModifiers::default();
            for (key, val) in specs {
                match (key, val) {
                    ("x", ParsedModifierValue::Number(n)) => mods.x = Some(n),
                    ("y", ParsedModifierValue::Number(n)) => mods.y = Some(n),
                    ("right", ParsedModifierValue::Number(n)) => mods.x = Some(n),
                    ("left", ParsedModifierValue::Number(n)) => mods.x = Some(-n),
                    ("down", ParsedModifierValue::Number(n)) => mods.y = Some(n),
                    ("up", ParsedModifierValue::Number(n)) => mods.y = Some(-n),
                    ("radius", ParsedModifierValue::Number(n)) => mods.radius = Some(n),
                    ("bulge", ParsedModifierValue::Number(n)) => mods.bulge = Some(n),
                    ("sweep", ParsedModifierValue::Sweep(s)) => mods.sweep = Some(s),
                    ("via", ParsedModifierValue::Identifier(id)) => mods.via = Some(id),
                    _ => {}
                }
            }
            mods
        });

    // Parse: vertex name [position]?
    let vertex_decl = just(Token::Vertex)
        .ignore_then(identifier.clone())
        .then(path_modifier_block.clone().or_not())
        .map_with(|(name, mods), e| {
            let position = mods.and_then(|m| {
                if m.x.is_some() || m.y.is_some() {
                    Some(VertexPosition { x: m.x, y: m.y })
                } else {
                    None
                }
            });
            Spanned::new(
                PathCommand::Vertex(VertexDecl { name, position }),
                span_range(&e.span()),
            )
        });

    // Parse: line_to target [position]?
    let line_to_decl = just(Token::LineTo)
        .ignore_then(identifier.clone())
        .then(path_modifier_block.clone().or_not())
        .map_with(|(target, mods), e| {
            let position = mods.and_then(|m| {
                if m.x.is_some() || m.y.is_some() {
                    Some(VertexPosition { x: m.x, y: m.y })
                } else {
                    None
                }
            });
            Spanned::new(
                PathCommand::LineTo(LineToDecl { target, position }),
                span_range(&e.span()),
            )
        });

    // Parse: arc_to target [position, radius/bulge, sweep]?
    let arc_to_decl = just(Token::ArcTo)
        .ignore_then(identifier.clone())
        .then(path_modifier_block.clone().or_not())
        .map_with(|(target, mods), e| {
            let (position, params) = mods
                .map(|m| m.into_position_and_params())
                .unwrap_or_else(|| (None, ArcParams::default()));
            Spanned::new(
                PathCommand::ArcTo(ArcToDecl {
                    target,
                    position,
                    params,
                }),
                span_range(&e.span()),
            )
        });

    // Parse: curve_to target [via: control, x: 100, y: 50]? (Feature 008)
    let curve_to_decl = just(Token::CurveTo)
        .ignore_then(identifier.clone())
        .then(path_modifier_block.clone().or_not())
        .map_with(|(target, mods), e| {
            let (position, via) = mods
                .map(|m| {
                    let pos = if m.x.is_some() || m.y.is_some() {
                        Some(VertexPosition { x: m.x, y: m.y })
                    } else {
                        None
                    };
                    (pos, m.via)
                })
                .unwrap_or((None, None));
            Spanned::new(
                PathCommand::CurveTo(CurveToDecl {
                    target,
                    via,
                    position,
                }),
                span_range(&e.span()),
            )
        });

    // Parse: close
    let close_decl =
        just(Token::Close).map_with(|_, e| Spanned::new(PathCommand::Close, span_range(&e.span())));

    // Parse path command (vertex | line_to | arc_to | curve_to | close)
    let path_command = choice((vertex_decl, line_to_decl, arc_to_decl, curve_to_decl, close_decl));

    // Parse path body: { commands* }
    let path_body = path_command
        .repeated()
        .collect::<Vec<_>>()
        .delimited_by(just(Token::BraceOpen), just(Token::BraceClose))
        .map(|commands| PathBody { commands });

    // Parse: path "name"? identifier? [modifiers]? { body }
    let path_decl = just(Token::Path)
        .ignore_then(
            select! { Token::String(s) => s }
                .map_with(|s, e| Spanned::new(Identifier::new(s), span_range(&e.span())))
                .or_not(),
        )
        .then(identifier.clone().or_not())
        .then(modifier_block.clone().or_not())
        .then(path_body)
        .map(|(((label, name), mods), body)| {
            // Use label as name if present, otherwise use identifier
            let path_name = label.or(name);
            let path = PathDecl {
                name: path_name,
                body,
                modifiers: mods.clone().unwrap_or_default(),
            };
            ShapeDecl {
                shape_type: Spanned::new(ShapeType::Path(path), 0..0), // Span will be updated
                name: None,                                            // Name is inside PathDecl
                modifiers: mods.unwrap_or_default(),
            }
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
                anchors: vec![],  // Parsed groups don't have custom anchors
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
                            StyleKey::Rotation => "rotation".to_string(),
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

        // Anchor declaration: anchor name [position: element.property, direction: up/down/left/right]
        // (Feature 009 - T010)
        let anchor_direction = choice((
            // Cardinal directions
            just(Token::Up).map(|_| AnchorDirectionSpec::Cardinal(CardinalDirection::Up)),
            just(Token::Down).map(|_| AnchorDirectionSpec::Cardinal(CardinalDirection::Down)),
            just(Token::Left).map(|_| AnchorDirectionSpec::Cardinal(CardinalDirection::Left)),
            just(Token::Right).map(|_| AnchorDirectionSpec::Cardinal(CardinalDirection::Right)),
            // Handle edge keywords as cardinal directions too
            just(Token::Top).map(|_| AnchorDirectionSpec::Cardinal(CardinalDirection::Up)),
            just(Token::Bottom).map(|_| AnchorDirectionSpec::Cardinal(CardinalDirection::Down)),
            // Numeric angle
            number.clone().map(|n| AnchorDirectionSpec::Angle(n.node)),
        ));

        // Parse anchor position: element.property or element.property +/- offset
        let anchor_position = property_ref
            .clone()
            .then(
                choice((
                    just(Token::Plus)
                        .ignore_then(number.clone())
                        .map(|n| n.node),
                    just(Token::Minus)
                        .ignore_then(number.clone())
                        .map(|n| -n.node),
                ))
                .or_not(),
            )
            .map(|(prop_ref, offset)| {
                if let Some(off) = offset {
                    AnchorPosition::PropertyRefWithOffset {
                        prop_ref,
                        offset: off,
                    }
                } else {
                    AnchorPosition::PropertyRef(prop_ref)
                }
            });

        // Parse anchor modifier: position: ..., direction: ...
        let anchor_modifier = choice((
            just(Token::Position)
                .ignore_then(just(Token::Colon))
                .ignore_then(anchor_position)
                .map(|pos| ("position", Some(pos), None)),
            just(Token::Direction)
                .ignore_then(just(Token::Colon))
                .ignore_then(anchor_direction)
                .map(|dir| ("direction", None, Some(dir))),
        ));

        let anchor_decl = just(Token::Anchor)
            .ignore_then(identifier.clone())
            .then(
                anchor_modifier
                    .separated_by(just(Token::Comma))
                    .allow_trailing()
                    .collect::<Vec<_>>()
                    .delimited_by(just(Token::BracketOpen), just(Token::BracketClose)),
            )
            .try_map(|(name, modifiers), span| {
                let mut position: Option<AnchorPosition> = None;
                let mut direction: Option<AnchorDirectionSpec> = None;

                for (_, pos, dir) in modifiers {
                    if pos.is_some() {
                        position = pos;
                    }
                    if dir.is_some() {
                        direction = dir;
                    }
                }

                let pos = position.ok_or_else(|| {
                    Rich::custom(span, "anchor declaration requires 'position' modifier")
                })?;

                Ok(Statement::AnchorDecl(AnchorDecl {
                    name,
                    position: pos,
                    direction,
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
            anchor_decl,  // Feature 009: anchor declarations
            layout_decl.map(Statement::Layout),
            group_decl.map(Statement::Group),
            label_decl,
            connection_decl.clone().map(Statement::Connection),
            // path_decl before shape_decl since 'path' is a keyword (Feature 007)
            path_decl.clone().map(Statement::Shape),
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
            Statement::Connection(conns) => {
                assert_eq!(conns.len(), 1);
                // Feature 009: AnchorReference.element contains the identifier
                assert_eq!(conns[0].from.element.node.as_str(), "a");
                assert_eq!(conns[0].to.element.node.as_str(), "b");
                assert!(conns[0].from.anchor.is_none());
                assert!(conns[0].to.anchor.is_none());
                assert_eq!(conns[0].direction, ConnectionDirection::Forward);
            }
            _ => panic!("Expected connection"),
        }
    }

    #[test]
    fn test_parse_connection_with_anchors() {
        let doc = parse("a.right -> b.left").expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::Connection(conns) => {
                assert_eq!(conns.len(), 1);
                assert_eq!(conns[0].from.element.node.as_str(), "a");
                assert_eq!(conns[0].from.anchor.as_ref().map(|s| s.node.as_str()), Some("right"));
                assert_eq!(conns[0].to.element.node.as_str(), "b");
                assert_eq!(conns[0].to.anchor.as_ref().map(|s| s.node.as_str()), Some("left"));
            }
            _ => panic!("Expected connection"),
        }
    }

    #[test]
    fn test_parse_connection_mixed_anchors() {
        // One with anchor, one without
        let doc = parse("a.top -> b").expect("Should parse");
        match &doc.statements[0].node {
            Statement::Connection(conns) => {
                assert_eq!(conns[0].from.anchor.as_ref().map(|s| s.node.as_str()), Some("top"));
                assert!(conns[0].to.anchor.is_none());
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
            Statement::Connection(conns) => {
                assert_eq!(conns.len(), 1);
                let c = &conns[0];
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
                assert_eq!(t.source_path.as_ref().unwrap().node, "lib/component.ail");
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

    // ==================== Path Shape Parsing Tests (Feature 007) ====================

    #[test]
    fn test_parse_simple_path() {
        let input = r#"
            path "triangle" {
                vertex a
                line_to b [x: 50, y: 0]
                line_to c [x: 25, y: 40]
                close
            }
        "#;
        let doc = parse(input).expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::Shape(s) => {
                match &s.shape_type.node {
                    ShapeType::Path(path) => {
                        assert_eq!(path.name.as_ref().unwrap().node.as_str(), "triangle");
                        assert_eq!(path.body.commands.len(), 4);
                        // First command should be vertex
                        match &path.body.commands[0].node {
                            PathCommand::Vertex(v) => {
                                assert_eq!(v.name.node.as_str(), "a");
                                assert!(v.position.is_none());
                            }
                            other => panic!("Expected Vertex, got {:?}", other),
                        }
                        // Second should be line_to with position
                        match &path.body.commands[1].node {
                            PathCommand::LineTo(lt) => {
                                assert_eq!(lt.target.node.as_str(), "b");
                                let pos = lt.position.as_ref().expect("Should have position");
                                assert_eq!(pos.x, Some(50.0));
                                assert_eq!(pos.y, Some(0.0));
                            }
                            other => panic!("Expected LineTo, got {:?}", other),
                        }
                        // Last should be close
                        assert!(matches!(path.body.commands[3].node, PathCommand::Close));
                    }
                    other => panic!("Expected Path, got {:?}", other),
                }
            }
            other => panic!("Expected Shape, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_path_with_arc() {
        let input = r#"
            path "rounded" {
                vertex a
                arc_to b [x: 50, y: 0, radius: 10]
                line_to c [x: 50, y: 50]
                close
            }
        "#;
        let doc = parse(input).expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::Shape(s) => match &s.shape_type.node {
                ShapeType::Path(path) => {
                    assert_eq!(path.body.commands.len(), 4);
                    // Check arc_to command
                    match &path.body.commands[1].node {
                        PathCommand::ArcTo(arc) => {
                            assert_eq!(arc.target.node.as_str(), "b");
                            let pos = arc.position.as_ref().expect("Should have position");
                            assert_eq!(pos.x, Some(50.0));
                            assert_eq!(pos.y, Some(0.0));
                            match &arc.params {
                                ArcParams::Radius { radius, sweep } => {
                                    assert!((radius - 10.0).abs() < 0.001);
                                    assert!(matches!(sweep, SweepDirection::Clockwise));
                                }
                                other => panic!("Expected Radius params, got {:?}", other),
                            }
                        }
                        other => panic!("Expected ArcTo, got {:?}", other),
                    }
                }
                other => panic!("Expected Path, got {:?}", other),
            },
            other => panic!("Expected Shape, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_path_with_bulge() {
        let input = r#"
            path "curved" {
                vertex a
                arc_to b [x: 50, bulge: 0.3]
                close
            }
        "#;
        let doc = parse(input).expect("Should parse");
        match &doc.statements[0].node {
            Statement::Shape(s) => match &s.shape_type.node {
                ShapeType::Path(path) => match &path.body.commands[1].node {
                    PathCommand::ArcTo(arc) => match &arc.params {
                        ArcParams::Bulge(b) => assert!((b - 0.3).abs() < 0.001),
                        other => panic!("Expected Bulge params, got {:?}", other),
                    },
                    other => panic!("Expected ArcTo, got {:?}", other),
                },
                other => panic!("Expected Path, got {:?}", other),
            },
            other => panic!("Expected Shape, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_path_with_sweep_direction() {
        let input = r#"
            path "arc" {
                vertex a
                arc_to b [x: 50, radius: 20, sweep: counterclockwise]
            }
        "#;
        let doc = parse(input).expect("Should parse");
        match &doc.statements[0].node {
            Statement::Shape(s) => match &s.shape_type.node {
                ShapeType::Path(path) => match &path.body.commands[1].node {
                    PathCommand::ArcTo(arc) => match &arc.params {
                        ArcParams::Radius { sweep, .. } => {
                            assert!(matches!(sweep, SweepDirection::Counterclockwise));
                        }
                        other => panic!("Expected Radius params, got {:?}", other),
                    },
                    other => panic!("Expected ArcTo, got {:?}", other),
                },
                other => panic!("Expected Path, got {:?}", other),
            },
            other => panic!("Expected Shape, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_path_in_layout() {
        let input = r#"
            row {
                path "shape1" { vertex a }
                rect spacer
            }
        "#;
        let doc = parse(input).expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::Layout(l) => {
                assert_eq!(l.children.len(), 2);
                match &l.children[0].node {
                    Statement::Shape(s) => {
                        assert!(matches!(s.shape_type.node, ShapeType::Path(_)));
                    }
                    other => panic!("Expected Shape, got {:?}", other),
                }
            }
            other => panic!("Expected Layout, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_path_with_modifiers() {
        let input = r#"
            path "styled" [fill: blue, stroke: black] {
                vertex a
                vertex b [x: 100, y: 0]
            }
        "#;
        let doc = parse(input).expect("Should parse");
        match &doc.statements[0].node {
            Statement::Shape(s) => {
                assert_eq!(s.modifiers.len(), 2);
                match &s.shape_type.node {
                    ShapeType::Path(path) => {
                        assert_eq!(path.body.commands.len(), 2);
                    }
                    other => panic!("Expected Path, got {:?}", other),
                }
            }
            other => panic!("Expected Shape, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_degenerate_path() {
        // Single vertex path (renders as point)
        // Note: Using "origin" instead of "center" since "center" is a keyword
        let input = r#"path "dot" { vertex origin }"#;
        let doc = parse(input).expect("Should parse");
        match &doc.statements[0].node {
            Statement::Shape(s) => match &s.shape_type.node {
                ShapeType::Path(path) => {
                    assert_eq!(path.name.as_ref().unwrap().node.as_str(), "dot");
                    assert_eq!(path.body.commands.len(), 1);
                }
                other => panic!("Expected Path, got {:?}", other),
            },
            other => panic!("Expected Shape, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_path_with_directional_positions() {
        let input = r#"
            path "arrow" {
                vertex tip [right: 100, down: 25]
                line_to left_edge [left: 60, up: 25]
            }
        "#;
        let doc = parse(input).expect("Should parse");
        match &doc.statements[0].node {
            Statement::Shape(s) => match &s.shape_type.node {
                ShapeType::Path(path) => {
                    // Check tip vertex with right/down (positive x, positive y)
                    match &path.body.commands[0].node {
                        PathCommand::Vertex(v) => {
                            let pos = v.position.as_ref().expect("Should have position");
                            assert_eq!(pos.x, Some(100.0));
                            assert_eq!(pos.y, Some(25.0));
                        }
                        other => panic!("Expected Vertex, got {:?}", other),
                    }
                    // Check left_edge with left/up (negative x, negative y)
                    match &path.body.commands[1].node {
                        PathCommand::LineTo(lt) => {
                            let pos = lt.position.as_ref().expect("Should have position");
                            assert_eq!(pos.x, Some(-60.0));
                            assert_eq!(pos.y, Some(-25.0));
                        }
                        other => panic!("Expected LineTo, got {:?}", other),
                    }
                }
                other => panic!("Expected Path, got {:?}", other),
            },
            other => panic!("Expected Shape, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_path_with_identifier_name() {
        // Path with identifier instead of string name
        let input = r#"path my_shape { vertex a close }"#;
        let doc = parse(input).expect("Should parse");
        match &doc.statements[0].node {
            Statement::Shape(s) => match &s.shape_type.node {
                ShapeType::Path(path) => {
                    assert_eq!(path.name.as_ref().unwrap().node.as_str(), "my_shape");
                }
                other => panic!("Expected Path, got {:?}", other),
            },
            other => panic!("Expected Shape, got {:?}", other),
        }
    }

    // ==================== Anchor Declaration Tests (Feature 009 - T012) ====================

    #[test]
    fn test_parse_anchor_basic() {
        let input = r#"anchor input [position: body.left]"#;
        let doc = parse(input).expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::AnchorDecl(a) => {
                assert_eq!(a.name.node.as_str(), "input");
                match &a.position {
                    AnchorPosition::PropertyRef(pr) => {
                        assert_eq!(pr.element.node.segments[0].node.as_str(), "body");
                        assert!(matches!(pr.property.node, ConstraintProperty::Left));
                    }
                    _ => panic!("Expected PropertyRef"),
                }
                assert!(a.direction.is_none());
            }
            other => panic!("Expected AnchorDecl, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_anchor_with_direction() {
        let input = r#"anchor output [position: body.right, direction: right]"#;
        let doc = parse(input).expect("Should parse");
        match &doc.statements[0].node {
            Statement::AnchorDecl(a) => {
                assert_eq!(a.name.node.as_str(), "output");
                assert!(matches!(
                    a.direction,
                    Some(AnchorDirectionSpec::Cardinal(CardinalDirection::Right))
                ));
            }
            other => panic!("Expected AnchorDecl, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_anchor_with_angle_direction() {
        let input = r#"anchor diagonal [position: body.top, direction: 45]"#;
        let doc = parse(input).expect("Should parse");
        match &doc.statements[0].node {
            Statement::AnchorDecl(a) => {
                match &a.direction {
                    Some(AnchorDirectionSpec::Angle(angle)) => {
                        assert_eq!(*angle, 45.0);
                    }
                    other => panic!("Expected Angle direction, got {:?}", other),
                }
            }
            other => panic!("Expected AnchorDecl, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_anchor_with_up_down_directions() {
        // Test that up/down keywords work for anchor direction
        let input_up = r#"anchor top_port [position: header.top, direction: up]"#;
        let doc = parse(input_up).expect("Should parse up");
        match &doc.statements[0].node {
            Statement::AnchorDecl(a) => {
                assert!(matches!(
                    a.direction,
                    Some(AnchorDirectionSpec::Cardinal(CardinalDirection::Up))
                ));
            }
            _ => panic!("Expected AnchorDecl"),
        }

        let input_down = r#"anchor bottom_port [position: footer.bottom, direction: down]"#;
        let doc = parse(input_down).expect("Should parse down");
        match &doc.statements[0].node {
            Statement::AnchorDecl(a) => {
                assert!(matches!(
                    a.direction,
                    Some(AnchorDirectionSpec::Cardinal(CardinalDirection::Down))
                ));
            }
            _ => panic!("Expected AnchorDecl"),
        }
    }

    #[test]
    fn test_parse_anchor_missing_position_error() {
        // Anchor without position should fail
        let input = r#"anchor invalid [direction: left]"#;
        let result = parse(input);
        assert!(result.is_err(), "Should fail without position");
    }

    #[test]
    fn test_parse_anchor_in_template() {
        let input = r#"
            template "server" {
                rect body [width: 100, height: 60]
                anchor input [position: body.left, direction: left]
                anchor output [position: body.right, direction: right]
            }
        "#;
        let doc = parse(input).expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::TemplateDecl(t) => {
                assert_eq!(t.body.as_ref().unwrap().len(), 3);
                // Check that anchors are parsed correctly inside template
                let mut anchor_count = 0;
                for stmt in t.body.as_ref().unwrap() {
                    if matches!(stmt.node, Statement::AnchorDecl(_)) {
                        anchor_count += 1;
                    }
                }
                assert_eq!(anchor_count, 2);
            }
            other => panic!("Expected TemplateDecl, got {:?}", other),
        }
    }
}
