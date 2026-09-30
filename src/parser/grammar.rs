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
/// `rect line` / `path row [...]`: a reserved word where a name goes. The
/// grammar reads it as a second, unnamed declaration, so the name silently
/// vanishes and every later `line` is a parse error far away from the cause.
fn reserved_word_as_name(input: &str) -> Option<crate::ParseError> {
    use crate::parser::lexer::Token;
    let toks: Vec<(Token, std::ops::Range<usize>)> = crate::parser::lexer::lex(input).collect();
    let declares = |t: &Token| {
        matches!(
            t,
            Token::Rect | Token::Circle | Token::Ellipse | Token::Polygon | Token::Line | Token::Path
                | Token::Callout | Token::Group | Token::Row | Token::Col | Token::Grid | Token::Stack
        )
    };
    for w in toks.windows(2) {
        let ((a, sa), (b, sb)) = (&w[0], &w[1]);
        if !declares(a) || input[sa.end..sb.start].contains('\n') {
            continue;
        }
        if let Some(word) = crate::error::keyword_spelling(b) {
            return Some(crate::ParseError::Syntax {
                span: sb.clone(),
                message: format!(
                    "`{}` is a reserved word and cannot name an element; call it something else \
                     (e.g. `{}_1`, `track`)",
                    word, word
                ),
                expected: vec![],
            });
        }
    }
    None
}

pub fn parse(input: &str) -> Result<Document, Vec<crate::ParseError>> {
    let len = input.len();
    if let Some(e) = reserved_word_as_name(input) {
        return Err(vec![e.locate(input)]);
    }

    // Create a logos lexer and convert to token stream
    let token_iter = crate::parser::lexer::lex(input).map(|(tok, span)| (tok, span.into()));

    // Turn the token iterator into a stream that chumsky can use
    let token_stream = Stream::from_iter(token_iter)
        // Split (Token, SimpleSpan) into token and span parts
        .map((len..len).into(), |(t, s): (_, _)| (t, s));

    document_parser()
        .parse(token_stream)
        .into_result()
        .map_err(|errs| {
            errs.into_iter()
                .map(|e| crate::ParseError::from(e).locate(input))
                .collect::<Vec<_>>()
        })
        .and_then(|doc| {
            expand_starred(doc).map_err(|(span, message)| {
                vec![crate::ParseError::Syntax { span, message, expected: vec![] }.locate(input)]
            })
        })
}

/// `doc d* [fname: ["a", "b", "c"]]` -> d0, d1, d2, each taking the i-th item
/// of every list-valued argument (lists zip by index; other arguments are
/// shared). Works for shapes too: `rect bar* [width: [88, 70, 55]]`.
/// The style key a modifier name means (`font_size` -> FontSize), else a
/// custom key.
pub fn style_key_named(name: &str) -> StyleKey {
    match name {
                "label" => StyleKey::Label,
                "role" => StyleKey::Role,
                "fill" => StyleKey::Fill,
                "stroke" => StyleKey::Stroke,
                "stroke_width" => StyleKey::StrokeWidth,
                "opacity" => StyleKey::Opacity,
                "fill_opacity" => StyleKey::FillOpacity,
                "stroke_opacity" => StyleKey::StrokeOpacity,
                "font_size" => StyleKey::FontSize,
                "class" => StyleKey::Class,
                "gap" => StyleKey::Gap,
                "size" => StyleKey::Size,
                "width" => StyleKey::Width,
                "height" => StyleKey::Height,
                "routing" => StyleKey::Routing,
                "label_position" => StyleKey::LabelPosition,
                "caption_of" => StyleKey::CaptionOf,
                "x" => StyleKey::X,
                "y" => StyleKey::Y,
                "stroke_dasharray" => StyleKey::StrokeDasharray,
                "rotation" => StyleKey::Rotation,
                "label_at" => StyleKey::LabelAt,
                "label_offset" => StyleKey::LabelOffset,
                "z_order" => StyleKey::ZOrder,
                "pointer" => StyleKey::Pointer,
                "dx" => StyleKey::Dx,
                "dy" => StyleKey::Dy,
                "scale" => StyleKey::Scale,
                "align" => StyleKey::Align,
                "label_fill" => StyleKey::LabelFill,
                other => StyleKey::Custom(other.to_string()),
            }
}

fn expand_starred(mut doc: Document) -> Result<Document, (Span, String)> {
    fn per_item(
        mods: &[Spanned<StyleValue>],
        name: &str,
        span: &Span,
    ) -> Result<usize, (Span, String)> {
        let mut n: Option<usize> = None;
        for v in mods {
            if let StyleValue::List(items) = &v.node {
                match n {
                    None => n = Some(items.len()),
                    Some(k) if k != items.len() => {
                        return Err((
                            v.span.clone(),
                            format!(
                                "'{}*': its lists have different lengths ({} and {}); every list makes one element per item, so they must match",
                                name, k, items.len()
                            ),
                        ))
                    }
                    _ => {}
                }
            }
        }
        n.ok_or_else(|| {
            (
                span.clone(),
                format!("'{}*' makes one element per item of a list argument, but it has no list (e.g. [label: [\"a\", \"b\"]])", name),
            )
        })
    }
    /// `items: [{a: 1, b: 2}, ...]`: item i's fields, as extra arguments.
    fn record_fields(values: &[(String, Spanned<StyleValue>)], i: usize) -> Vec<(Spanned<Identifier>, Spanned<StyleValue>)> {
        values
            .iter()
            .filter(|(k, _)| k == "items")
            .filter_map(|(_, v)| match &v.node {
                StyleValue::List(items) => items.get(i).map(|x| x.node.clone()),
                _ => None,
            })
            .flat_map(|r| match r {
                StyleValue::Record(fields) => fields,
                _ => Vec::new(),
            })
            .collect()
    }
    fn pick(v: &Spanned<StyleValue>, i: usize) -> Spanned<StyleValue> {
        match &v.node {
            StyleValue::List(items) => items[i].clone(),
            _ => v.clone(),
        }
    }
    fn walk(stmts: Vec<Spanned<Statement>>, counts: &mut std::collections::HashMap<String, usize>) -> Result<Vec<Spanned<Statement>>, (Span, String)> {
        let mut out = Vec::with_capacity(stmts.len());
        for st in stmts {
            let span = st.span.clone();
            match st.node {
                Statement::Shape(shape) if shape.name.as_ref().is_some_and(|n| n.node.0.ends_with('*')) => {
                    let name = shape.name.as_ref().unwrap();
                    let base = name.node.0.trim_end_matches('*').to_string();
                    let values: Vec<Spanned<StyleValue>> = shape.modifiers.iter().map(|m| m.node.value.clone()).collect();
                    let n = per_item(&values, &base, &name.span)?;
                    counts.insert(base.clone(), n);
                    let named: Vec<(String, Spanned<StyleValue>)> = shape
                        .modifiers
                        .iter()
                        .map(|m| (m.node.key.node.source_name(), m.node.value.clone()))
                        .collect();
                    for i in 0..n {
                        let mut s = shape.clone();
                        s.name = Some(Spanned::new(Identifier::new(format!("{}{}", base, i)), name.span.clone()));
                        s.modifiers.retain(|m| m.node.key.node.source_name() != "items");
                        for m in &mut s.modifiers {
                            m.node.value = pick(&m.node.value, i);
                        }
                        for (k, v) in record_fields(&named, i) {
                            let sp = k.span.clone();
                            s.modifiers.push(Spanned::new(
                                StyleModifier { key: Spanned::new(style_key_named(&k.node.0), sp.clone()), value: v },
                                sp,
                            ));
                        }
                        out.push(Spanned::new(Statement::Shape(s), span.clone()));
                    }
                }
                Statement::TemplateInstance(inst) if inst.instance_name.node.0.ends_with('*') => {
                    let base = inst.instance_name.node.0.trim_end_matches('*').to_string();
                    let values: Vec<Spanned<StyleValue>> = inst.arguments.iter().map(|(_, v)| v.clone()).collect();
                    let n = per_item(&values, &base, &inst.instance_name.span)?;
                    counts.insert(base.clone(), n);
                    let named: Vec<(String, Spanned<StyleValue>)> =
                        inst.arguments.iter().map(|(k, v)| (k.node.0.clone(), v.clone())).collect();
                    for i in 0..n {
                        let mut t = inst.clone();
                        t.instance_name = Spanned::new(Identifier::new(format!("{}{}", base, i)), inst.instance_name.span.clone());
                        t.arguments.retain(|(k, _)| k.node.0 != "items");
                        for (_, v) in &mut t.arguments {
                            *v = pick(v, i);
                        }
                        t.arguments.extend(record_fields(&named, i));
                        out.push(Spanned::new(Statement::TemplateInstance(t), span.clone()));
                    }
                }
                Statement::Layout(mut l) => {
                    l.children = walk(l.children, counts)?;
                    out.push(Spanned::new(Statement::Layout(l), span));
                }
                Statement::Group(mut g) => {
                    g.children = walk(g.children, counts)?;
                    out.push(Spanned::new(Statement::Group(g), span));
                }
                Statement::TemplateDecl(mut t) => {
                    if let Some(body) = t.body.take() {
                        t.body = Some(walk(body, counts)?);
                    }
                    out.push(Spanned::new(Statement::TemplateDecl(t), span));
                }
                other => out.push(Spanned::new(other, span)),
            }
        }
        Ok(out)
    }
    let mut counts = std::collections::HashMap::new();
    doc.statements = walk(doc.statements, &mut counts)?;
    doc.statements = zip_constraints(doc.statements, &counts)?;
    Ok(doc)
}

/// `constrain k*.center = d*.center`: one constraint per index (k0 on d0,
/// k1 on d1, ...), for elements declared with `name*`.
fn zip_constraints(stmts: Vec<Spanned<Statement>>, counts: &std::collections::HashMap<String, usize>) -> Result<Vec<Spanned<Statement>>, (Span, String)> {
    fn starred(p: &ElementPath) -> Vec<String> {
        p.segments.iter().filter_map(|s| s.node.0.strip_suffix('*').map(str::to_string)).collect()
    }
    fn paths(e: &ConstraintExpr) -> Vec<&ElementPath> {
        match e {
            ConstraintExpr::Equal { left, right } | ConstraintExpr::EqualWithOffset { left, right, .. } => {
                vec![&left.element.node, &right.element.node]
            }
            ConstraintExpr::Constant { left, .. } | ConstraintExpr::GreaterOrEqual { left, .. } | ConstraintExpr::LessOrEqual { left, .. } => {
                vec![&left.element.node]
            }
            ConstraintExpr::Midpoint { target, .. } => vec![&target.element.node],
            ConstraintExpr::Contains { .. } => vec![],
        }
    }
    fn nth(e: &mut ConstraintExpr, i: usize) {
        let fix = |p: &mut ElementPath| {
            for s in &mut p.segments {
                if let Some(b) = s.node.0.strip_suffix('*') {
                    s.node = Identifier::new(format!("{}{}", b, i));
                }
            }
        };
        match e {
            ConstraintExpr::Equal { left, right } | ConstraintExpr::EqualWithOffset { left, right, .. } => {
                fix(&mut left.element.node);
                fix(&mut right.element.node);
            }
            ConstraintExpr::Constant { left, .. } | ConstraintExpr::GreaterOrEqual { left, .. } | ConstraintExpr::LessOrEqual { left, .. } => {
                fix(&mut left.element.node)
            }
            ConstraintExpr::Midpoint { target, .. } => fix(&mut target.element.node),
            ConstraintExpr::Contains { .. } => {}
        }
    }
    let mut out = Vec::with_capacity(stmts.len());
    for st in stmts {
        let span = st.span.clone();
        match st.node {
            Statement::Constrain(d) => {
                let names: Vec<String> = paths(&d.expr).into_iter().flat_map(starred).collect();
                if names.is_empty() {
                    out.push(Spanned::new(Statement::Constrain(d), span));
                    continue;
                }
                let mut n: Option<(usize, &str)> = None;
                for name in &names {
                    let Some(&k) = counts.get(name) else {
                        return Err((span, format!("'{}*' in a constraint names the elements of a `{}*` declaration, and there is none", name, name)));
                    };
                    match n {
                        Some((m, other)) if m != k => {
                            return Err((
                                span,
                                format!("'{}*' has {} elements and '{}*' has {}: a constraint over both pairs them by index, so they must match", other, m, name, k),
                            ))
                        }
                        _ => n = Some((k, name)),
                    }
                }
                for i in 0..n.unwrap().0 {
                    let mut c = d.clone();
                    nth(&mut c.expr, i);
                    if let Some(nm) = &mut c.name {
                        nm.node = Identifier::new(format!("{}_{}", nm.node.0, i));
                    }
                    out.push(Spanned::new(Statement::Constrain(c), span.clone()));
                }
            }
            Statement::Layout(mut l) => {
                l.children = zip_constraints(l.children, counts)?;
                out.push(Spanned::new(Statement::Layout(l), span));
            }
            Statement::Group(mut g) => {
                g.children = zip_constraints(g.children, counts)?;
                out.push(Spanned::new(Statement::Group(g), span));
            }
            Statement::TemplateDecl(mut t) => {
                if let Some(body) = t.body.take() {
                    t.body = Some(zip_constraints(body, counts)?);
                }
                out.push(Spanned::new(Statement::TemplateDecl(t), span));
            }
            other => out.push(Spanned::new(other, span)),
        }
    }
    Ok(out)
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
    Identifier(Spanned<Identifier>), // Feature 008: for via references
}

/// Helper struct for parsing arc modifiers within brackets
#[derive(Debug, Clone, Default)]
struct ParsedArcModifiers {
    x: Option<f64>,
    y: Option<f64>,
    radius: Option<f64>,
    bulge: Option<f64>,
    sweep: Option<SweepDirection>,
    large_arc: Option<bool>,
    via: Option<Spanned<Identifier>>, // Feature 008: steering vertex reference
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
                large_arc: self.large_arc.unwrap_or(false),
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
            let key = style_key_named(id.node.as_str());
            Spanned::new(key, id.span)
        }),
    ));

    // Parse a color category - identifier or "text" keyword (since text is reserved)
    let color_category = choice((
        // "text" keyword token maps to Text category
        just(Token::Text)
            .map_with(|_, e| Spanned::new(Identifier::new("text"), span_range(&e.span()))),
        // Regular identifier
        identifier,
    ));

    // Parse a symbolic color: category(-variant)?(-lightness)?
    // e.g., foreground, foreground-1, text-dark, accent-2-light
    let symbolic_color = color_category
        .then(just(Token::Minus).ignore_then(number).or_not())
        .then(just(Token::Minus).ignore_then(identifier).or_not())
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

    // Any other hyphenated word, e.g. `status-success`. A stylesheet is an
    // open map of tokens while the categories above are a closed set, so
    // without this the palette can ship colours the language cannot name.
    // Validation rejects a token no stylesheet defines, so this widens what
    // can be *spelled*, not what is accepted.
    // `role` is also a keyword (`[role: label]`); as a token head it names
    // a colour role (`role-primary`).
    let token_head = choice((
        just(Token::Role).map_with(|_, e| Spanned::new(Identifier::new("role"), span_range(&e.span()))),
        identifier,
    ));
    let palette_token = token_head
        .then(
            just(Token::Minus)
                .ignore_then(choice((
                    identifier.map(|i: Spanned<Identifier>| i.node.0),
                    number.map(|n: Spanned<f64>| format!("{}", n.node as i64)),
                )))
                .repeated()
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .map(|(head, tail)| {
            let mut token = head.node.0;
            for part in tail {
                token.push('-');
                token.push_str(&part);
            }
            StyleValue::Color(ColorValue::PaletteToken(token))
        });

    let value_atom = choice((
        // Hex colors like #ff0000 or #f00
        select! { Token::HexColor(c) => StyleValue::Color(ColorValue::Hex(c)) }
            .map_with(|v, e| Spanned::new(v, span_range(&e.span()))),
        // Symbolic colors (must come before plain identifiers)
        symbolic_color.map_with(|v, e| Spanned::new(v, span_range(&e.span()))),
        // Any other hyphenated palette token, after the structured ones.
        palette_token.map_with(|v, e| Spanned::new(v, span_range(&e.span()))),
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
        // above / below / inside keyword values (used by `label_position:`).
        // These are already lexer tokens for constraint relations; this only
        // lets them be read as style values too.
        just(Token::Above).map_with(|_, e| {
            Spanned::new(
                StyleValue::Keyword("above".to_string()),
                span_range(&e.span()),
            )
        }),
        just(Token::Below).map_with(|_, e| {
            Spanned::new(
                StyleValue::Keyword("below".to_string()),
                span_range(&e.span()),
            )
        }),
        just(Token::Inside).map_with(|_, e| {
            Spanned::new(
                StyleValue::Keyword("inside".to_string()),
                span_range(&e.span()),
            )
        }),
        // up / down keyword values (used by `pointer: up|down` on callouts)
        just(Token::Up).map_with(|_, e| {
            Spanned::new(StyleValue::Keyword("up".to_string()), span_range(&e.span()))
        }),
        just(Token::Down).map_with(|_, e| {
            Spanned::new(
                StyleValue::Keyword("down".to_string()),
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
        // `grid` is a reserved token but is also a valid pattern-fill name in
        // style values (`fill: grid`); accept it as a keyword value here.
        just(Token::Grid).map_with(|_, e| {
            Spanned::new(
                StyleValue::Keyword("grid".to_string()),
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
                "center" | "direct" | "orthogonal" | "curved" | "none" | "auto" | "solid"
                | "dashed" | "dotted" | "hidden" | "bold" | "italic" | "normal" | "start"
                | "middle" | "end" => StyleValue::Keyword(id.node.0.clone()),
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

    // Bracketed list value: `[a, b, c]` (numbers or strings), e.g. at: [1,0],
    // col_labels: ["a","b"]. Lists contain atoms only (no nested lists).
    // `[st1.dot, st2.dot]`: a dotted name is one identifier in a list.
    let dotted_atom = identifier
        .then(just(Token::Dot).ignore_then(identifier).repeated().at_least(1).collect::<Vec<_>>())
        .map_with(|(head, rest), e| {
            let mut n = head.node.0;
            for r in rest {
                n.push('.');
                n.push_str(&r.node.0);
            }
            Spanned::new(StyleValue::Identifier(Identifier::new(n)), span_range(&e.span()))
        });
    // `{fname: "login.py", c1: role-primary}`: a record, one item of `items:`.
    let record_key = choice((
        identifier,
        just(Token::Label).map_with(|_, e| Spanned::new(Identifier::new("label"), span_range(&e.span()))),
        just(Token::Text).map_with(|_, e| Spanned::new(Identifier::new("text"), span_range(&e.span()))),
    ));
    let record = record_key
        .then_ignore(just(Token::Colon))
        .then(choice((dotted_atom.clone(), value_atom.clone())))
        .separated_by(just(Token::Comma))
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just(Token::BraceOpen), just(Token::BraceClose))
        .map_with(|fields, e| Spanned::new(StyleValue::Record(fields), span_range(&e.span())));
    let value_list = choice((record, dotted_atom, value_atom.clone()))
        .separated_by(just(Token::Comma))
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just(Token::BracketOpen), just(Token::BracketClose))
        .map_with(|items, e| Spanned::new(StyleValue::List(items), span_range(&e.span())))
        .boxed();

    // Function-style values for pattern/gradient fills, e.g. `hatch(accent-1)`,
    // `gradient(blue, white, 90)`. Name + arity are validated here so that
    // style resolution can stay infallible. A bare `hatch` (no parens) is NOT
    // matched here — it falls through to `value_atom` as an identifier and is
    // interpreted as a default-colored pattern during resolution.
    // The function name is an identifier, except `grid` which is a reserved
    // token (grid-layout keyword) so we accept it explicitly here.
    let call_head = choice((
        identifier.map(|id| Spanned::new(id.node.0.clone(), id.span)),
        just(Token::Grid)
            .map_with(|_, e| Spanned::new("grid".to_string(), span_range(&e.span()))),
    ));
    let call_value = call_head
        .then(
            value_atom
                .clone()
                .separated_by(just(Token::Comma))
                .allow_trailing()
                .collect::<Vec<_>>()
                .delimited_by(just(Token::ParenOpen), just(Token::ParenClose)),
        )
        .try_map(|(name, args), span| {
            let n = name.node.as_str();
            let argc = args.len();
            let ok = match n {
                "hatch" | "cross_hatch" | "dots" | "grid" => argc <= 2,
                "gradient" => argc == 2 || argc == 3,
                "radial_gradient" => argc == 2,
                _ => {
                    return Err(Rich::custom(
                        span,
                        format!(
                            "unknown fill function `{}`; expected one of: \
                             hatch, cross_hatch, dots, grid, gradient, radial_gradient",
                            n
                        ),
                    ))
                }
            };
            if !ok {
                return Err(Rich::custom(
                    span,
                    format!(
                        "`{}` got {} argument(s); patterns take 0-2 colors, \
                         gradient takes 2 colors + optional angle, \
                         radial_gradient takes 2 colors",
                        n, argc
                    ),
                ));
            }
            Ok(StyleValue::Call {
                name: name.node.clone(),
                args,
            })
        })
        .map_with(|v, e| Spanned::new(v, span_range(&e.span())))
        .boxed();

    // `drawn: 60%`, and a dotted name (`drawn: st.dot`) as one identifier.
    let percent_value = just(Token::Minus)
        .or_not()
        .then(number)
        .then_ignore(just(Token::Percent))
        .map_with(|(neg, n), e| {
            let v = if neg.is_some() { -n.node } else { n.node };
            Spanned::new(StyleValue::Number { value: v, unit: Some("%".to_string()) }, span_range(&e.span()))
        });
    let dotted_value = identifier
        .then(just(Token::Dot).ignore_then(identifier).repeated().at_least(1).collect::<Vec<_>>())
        .map_with(|(head, rest), e| {
            let mut n = head.node.0;
            for r in rest {
                n.push('.');
                n.push_str(&r.node.0);
            }
            Spanned::new(StyleValue::Identifier(Identifier::new(n)), span_range(&e.span()))
        });
    let style_value = choice((value_list, call_value, percent_value, dotted_value, value_atom.clone())).boxed();

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
            .ignore_then(string_literal)
            .map(|s| ShapeType::Icon { icon_name: s.node }),
        just(Token::Text)
            .ignore_then(string_literal)
            .map(|s| ShapeType::Text { content: s.node }),
        // Callout: pointer direction comes from the `pointer:` modifier (default down)
        just(Token::Callout).to(ShapeType::Callout {
            pointer: PointerDir::Down,
        }),
    ))
    .map_with(|st, e| Spanned::new(st, span_range(&e.span())));

    // `name` or `name*` (one element per item of its list-valued modifiers).
    let starred_name = identifier
        .then(just(Token::Star).or_not())
        .map(|(id, star)| match star {
            Some(_) => Spanned::new(Identifier::new(format!("{}*", id.node.0)), id.span),
            None => id,
        })
        .boxed();

    // Shape declaration
    let shape_decl = shape_type
        .then(starred_name.clone().or_not())
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

    // Grid cell as a connection endpoint: `grid.cell(row, col)[.anchor]`.
    let cell_anchor_reference = identifier
        .then_ignore(just(Token::Dot))
        .then_ignore(just(Token::Ident("cell".to_string())))
        .then_ignore(just(Token::ParenOpen))
        .then(number)
        .then_ignore(just(Token::Comma))
        .then(number)
        .then_ignore(just(Token::ParenClose))
        .then(just(Token::Dot).ignore_then(anchor_name.clone()).or_not())
        .map(|(((grid, row), col), anch)| {
            let cell_id = grid_cell_id(&grid.node.0, row.node as usize, col.node as usize);
            let id = Spanned::new(Identifier::new(cell_id), grid.span.clone());
            match anch {
                Some(a) => AnchorReference::with_anchor(id, a),
                None => AnchorReference::element_only(id),
            }
        });

    // Anchor reference parser: identifier { "." anchor_name }?
    // Parses either:
    //   - `element` -> AnchorReference with anchor=None
    //   - `element.anchor_name` -> AnchorReference with anchor=Some
    // `a.b.c` (a nested component, maybe with an anchor) keeps the tail in
    // `anchor`, dotted; `motion::expand` splits it once element names are known.
    let plain_anchor_reference = identifier
        .then(
            just(Token::Dot)
                .ignore_then(anchor_name.clone())
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(|(element, rest)| {
            if rest.is_empty() {
                AnchorReference::element_only(element)
            } else {
                let span = rest[0].span.start..rest[rest.len() - 1].span.end;
                let joined = rest.iter().map(|r| r.node.clone()).collect::<Vec<_>>().join(".");
                AnchorReference::with_anchor(element, Spanned::new(joined, span))
            }
        });

    let anchor_reference = choice((cell_anchor_reference, plain_anchor_reference)).boxed();

    // Connection declaration (supports chained: a -> b -> c [modifiers])
    // Feature 009: Now supports anchor syntax (a.right -> b.left)
    // Feature 011: Now supports named connections (a -> b as name [modifiers])
    let connection_name = just(Token::As).ignore_then(identifier);

    let connection_decl = anchor_reference
        .clone()
        .then(
            connection_op
                .then(anchor_reference.clone())
                .repeated()
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .then(connection_name.or_not())
        .then(modifier_block.clone().or_not())
        .map(|(((first, segments), name), modifiers)| {
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
                    // Only the last segment gets modifiers and name
                    modifiers: if is_last { modifiers.clone() } else { vec![] },
                    name: if is_last { name.clone() } else { None },
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
        .ignore_then(identifier)
        .then(position_relation.then(identifier).or_not())
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
        .separated_by(just(Token::Dot))
        .at_least(1)
        .collect::<Vec<_>>()
        .map_with(|segments, e| Spanned::new(ElementPath { segments }, span_range(&e.span())));

    // `c.dot` as one name (a component's part): `c_dot`.
    let joined_path = identifier
        .then(just(Token::Dot).ignore_then(identifier).repeated().collect::<Vec<_>>())
        .map(|(head, rest)| {
            let mut n = head.node.0.clone();
            let span = head.span.clone();
            for r in rest {
                n.push('_');
                n.push_str(&r.node.0);
            }
            Spanned::new(Identifier::new(n), span)
        })
        .boxed();

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
        // Regular identifier; `k*` (every instance of a starred
        // declaration, zipped by index with the other side's)
        identifier.then(just(Token::Star).or_not()).map(|(id, star)| match star {
            Some(_) => Spanned::new(Identifier::new(format!("{}*", id.node.0)), id.span),
            None => id,
        }),
    ));

    // Grid cell reference in a constraint: `grid.cell(row, col)[.property]`.
    // Desugars to a property ref on the reference-only cell element.
    let cell_property_ref = identifier
        .then_ignore(just(Token::Dot))
        .then_ignore(just(Token::Ident("cell".to_string())))
        .then_ignore(just(Token::ParenOpen))
        .then(number)
        .then_ignore(just(Token::Comma))
        .then(number)
        .then_ignore(just(Token::ParenClose))
        .then(just(Token::Dot).ignore_then(path_or_prop_segment.clone()).or_not())
        .try_map(|(((grid, row), col), prop_seg), span: SimpleSpan| {
            let cell_id = grid_cell_id(&grid.node.0, row.node as usize, col.node as usize);
            let property = match prop_seg {
                Some(seg) => ConstraintProperty::from_str(seg.node.as_str())
                    .ok_or_else(|| Rich::custom(span, "invalid grid cell property"))?,
                None => ConstraintProperty::Center,
            };
            Ok(PropertyRef {
                element: Spanned::new(
                    ElementPath {
                        segments: vec![Spanned::new(Identifier::new(cell_id), span_range(&span))],
                    },
                    span_range(&span),
                ),
                property: Spanned::new(property, span_range(&span)),
            })
        });

    let plain_property_ref = path_or_prop_segment
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

    // Cell references must be tried first (the plain ref would choke on `cell(`).
    let property_ref = choice((cell_property_ref, plain_property_ref)).boxed();

    // Parse offset: + number or - number
    let offset = choice((
        just(Token::Plus).ignore_then(number).map(|n| n.node),
        just(Token::Minus).ignore_then(number).map(|n| -n.node),
    ));

    // Constraint expression parsers

    // Midpoint: target.prop = midpoint(a, b) or target.prop = midpoint(a, b) + offset
    let midpoint_expr = property_ref
        .clone()
        .then_ignore(just(Token::Equals))
        .then_ignore(just(Token::Midpoint))
        .then_ignore(just(Token::ParenOpen))
        .then(joined_path.clone())
        .then_ignore(just(Token::Comma))
        .then(joined_path.clone())
        .then_ignore(just(Token::ParenClose))
        .then(offset.clone().or_not())
        .map(|(((target, a), b), off)| ConstraintExpr::Midpoint {
            target,
            a,
            b,
            offset: off.unwrap_or(0.0),
        });

    // A `contains` element may be a plain identifier or a grid cell reference
    // `grid.cell(row, col)` (desugared to the cell's synthetic id). The plain
    // identifier parser stops at the dot, so the cell form is tried first.
    let cell_as_identifier = identifier
        .then_ignore(just(Token::Dot))
        .then_ignore(just(Token::Ident("cell".to_string())))
        .then_ignore(just(Token::ParenOpen))
        .then(number)
        .then_ignore(just(Token::Comma))
        .then(number)
        .then_ignore(just(Token::ParenClose))
        .map(|((grid, row), col)| {
            let cell_id = grid_cell_id(&grid.node.0, row.node as usize, col.node as usize);
            Spanned::new(Identifier::new(cell_id), grid.span.clone())
        });
    let contains_element = choice((cell_as_identifier, identifier));

    // Contains: container contains a, b, c [padding: N]
    let contains_expr = identifier
        .then_ignore(just(Token::Contains))
        .then(
            contains_element
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
        .then(just(Token::Minus).or_not().then(number))
        .map(|(left, (neg, n))| {
            let value = if neg.is_some() { -n.node } else { n.node };
            ConstraintExpr::GreaterOrEqual { left, value }
        });

    let le_expr = property_ref
        .clone()
        .then_ignore(just(Token::LessOrEqual))
        .then(just(Token::Minus).or_not().then(number))
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
            .or(just(Token::Minus).or_not().then(number).map(|(neg, n)| {
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
        .then(just(Token::As).ignore_then(identifier).or_not())
        .map(|(expr, name)| ConstrainDecl { expr, name });

    // ==================== Template Parsing (Feature 005) ====================

    // Export declaration: export name1, name2
    let export_decl = just(Token::Export)
        .ignore_then(
            identifier
                .separated_by(just(Token::Comma))
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .map(|exports| ExportDecl { exports });

    // Parameter definition: name: default_value
    let param_def = identifier
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
        .ignore_then(string_literal)
        .then_ignore(just(Token::From))
        .then(string_literal)
        .map(|(name, path)| {
            let path_lower = path.node.to_lowercase();
            let source_type = if path_lower.ends_with(".svg") {
                TemplateSourceType::Svg
            } else if path_lower.ends_with(".png")
                || path_lower.ends_with(".jpg")
                || path_lower.ends_with(".jpeg")
                || path_lower.ends_with(".gif")
                || path_lower.ends_with(".webp")
                || path_lower.ends_with(".bmp")
            {
                TemplateSourceType::Raster
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
            .ignore_then(just(Token::Minus).or_not().then(number))
            .map(|(neg, n)| {
                let val = if neg.is_some() { -n.node } else { n.node };
                ("x", ParsedModifierValue::Number(val))
            }),
        just(Token::Ident("y".into()))
            .ignore_then(just(Token::Colon))
            .ignore_then(just(Token::Minus).or_not().then(number))
            .map(|(neg, n)| {
                let val = if neg.is_some() { -n.node } else { n.node };
                ("y", ParsedModifierValue::Number(val))
            }),
        just(Token::Right)
            .ignore_then(just(Token::Colon))
            .ignore_then(number)
            .map(|n| ("right", ParsedModifierValue::Number(n.node))),
        just(Token::Left)
            .ignore_then(just(Token::Colon))
            .ignore_then(number)
            .map(|n| ("left", ParsedModifierValue::Number(n.node))),
        just(Token::Up)
            .ignore_then(just(Token::Colon))
            .ignore_then(number)
            .map(|n| ("up", ParsedModifierValue::Number(n.node))),
        just(Token::Down)
            .ignore_then(just(Token::Colon))
            .ignore_then(number)
            .map(|n| ("down", ParsedModifierValue::Number(n.node))),
        // Arc specs
        just(Token::Ident("radius".into()))
            .ignore_then(just(Token::Colon))
            .ignore_then(number)
            .map(|n| ("radius", ParsedModifierValue::Number(n.node))),
        just(Token::Ident("bulge".into()))
            .ignore_then(just(Token::Colon))
            .ignore_then(just(Token::Minus).or_not().then(number))
            .map(|(neg, n)| {
                let val = if neg.is_some() { -n.node } else { n.node };
                ("bulge", ParsedModifierValue::Number(val))
            }),
        just(Token::Ident("sweep".into()))
            .ignore_then(just(Token::Colon))
            .ignore_then(sweep_direction.clone())
            .map(|s| ("sweep", ParsedModifierValue::Sweep(s))),
        just(Token::Ident("large_arc".into()))
            .ignore_then(just(Token::Colon))
            .ignore_then(choice((
                just(Token::Ident("true".into())).to(1.0),
                just(Token::Ident("false".into())).to(0.0),
            )))
            .map(|v| ("large_arc", ParsedModifierValue::Number(v))),
        // Feature 008: via reference for curve steering vertex
        just(Token::Ident("via".into()))
            .ignore_then(just(Token::Colon))
            .ignore_then(identifier)
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
                    ("large_arc", ParsedModifierValue::Number(n)) => {
                        mods.large_arc = Some(n != 0.0)
                    }
                    ("via", ParsedModifierValue::Identifier(id)) => mods.via = Some(id),
                    _ => {}
                }
            }
            mods
        });

    // Parse: vertex name [position]?
    let vertex_decl = just(Token::Vertex)
        .ignore_then(identifier)
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
        .ignore_then(identifier)
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
        .ignore_then(identifier)
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
        .ignore_then(identifier)
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
    let path_command = choice((
        vertex_decl,
        line_to_decl,
        arc_to_decl,
        curve_to_decl,
        close_decl,
    ));

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
        .then(identifier.or_not())
        .then(modifier_block.clone().or_not())
        // `path track [through: [a, b]]` has no body: it is routed after layout.
        .then(path_body.or_not().map(|b| b.unwrap_or(PathBody { commands: vec![] })))
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
                anchors: vec![], // Parsed groups don't have custom anchors
                is_template_instance: false,
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
            .ignore_then(string_literal)
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
            .then(starred_name.clone())
            .then(modifier_block.clone().or_not())
            .try_map(|((template_name, instance_name), mods), _span| {
                // Convert modifiers to argument list
                let arguments: Vec<(Spanned<Identifier>, Spanned<StyleValue>)> = mods
                    .unwrap_or_default()
                    .into_iter()
                    .map(|m| {
                        // Every key: one left out here was silently dropped
                        // (`code c [font_size: 24]` did nothing).
                        (
                            Spanned::new(Identifier::new(m.node.key.node.source_name()), m.node.key.span),
                            m.node.value,
                        )
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
            number.map(|n| AnchorDirectionSpec::Angle(n.node)),
        ));

        // Parse anchor position: element.property or element.property +/- offset
        let anchor_position = property_ref
            .clone()
            .then(
                choice((
                    just(Token::Plus).ignore_then(number).map(|n| n.node),
                    just(Token::Minus).ignore_then(number).map(|n| -n.node),
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
            .ignore_then(identifier)
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

        // ==================== Motion (keyframe bodies) ====================
        // Verbs are contextual: `draw`, `fly`, `then`, ... are ordinary
        // identifiers everywhere else, so no existing name breaks.
        fn kw<'a, I>(name: &'static str) -> impl Parser<'a, I, (), extra::Err<Rich<'a, Token>>> + Clone
        where
            I: ValueInput<'a, Token = Token, Span = SimpleSpan>,
        {
            any()
                .filter(move |t: &Token| matches!(t, Token::Ident(s) if s == name))
                .ignored()
                .labelled(name)
        }

        // A dotted name: `box`, `station.label`
        let dotted_name = identifier
            .separated_by(just(Token::Dot))
            .at_least(1)
            .collect::<Vec<_>>()
            .map_with(|parts, e| {
                let joined = parts.iter().map(|p| p.node.0.clone()).collect::<Vec<_>>().join(".");
                Spanned::new(joined, span_range(&e.span()))
            })
            .boxed();

        let selector = choice((
            kw("all")
                .ignore_then(kw("except"))
                .ignore_then(
                    identifier
                        .separated_by(just(Token::Comma))
                        .at_least(1)
                        .collect::<Vec<_>>(),
                )
                .map(|ids| Selector::AllExcept(ids.into_iter().map(|i| i.node.0).collect())),
            just(Token::Dot)
                .ignore_then(identifier)
                .map(|id| Selector::Class(id.node.0)),
            identifier
                .then(
                    just(Token::Dot)
                        .ignore_then(choice((
                            just(Token::Star).to(None),
                            identifier.map(Some),
                            // `c.line[4]`: `line` is a keyword elsewhere.
                            just(Token::Line).map_with(|_, e| {
                                Some(Spanned::new(Identifier::new("line"), span_range(&e.span())))
                            }),
                        )))
                        .repeated()
                        .collect::<Vec<_>>(),
                )
                // `c.line[4]`, `c.lines[8..12]`: lines of a code block.
                .then(
                    number
                        .then(just(Token::Dot).then(just(Token::Dot)).ignore_then(number).or_not())
                        .delimited_by(just(Token::BracketOpen), just(Token::BracketClose))
                        .or_not(),
                )
                .try_map(|((head, rest), index), span| {
                    if let Some((a, b)) = index {
                        let parts: Vec<String> = std::iter::once(head.node.0.clone())
                            .chain(rest.iter().filter_map(|p| p.as_ref().map(|i| i.node.0.clone())))
                            .collect();
                        let last = parts.last().cloned().unwrap_or_default();
                        if parts.len() < 2 || (last != "line" && last != "lines") {
                            return Err(Rich::custom(
                                span,
                                "an index goes on a code block's lines: `c.line[4]` or `c.lines[8..12]`",
                            ));
                        }
                        let code = parts[..parts.len() - 1].join(".");
                        let a = a.node as usize;
                        return Ok(match b {
                            Some(b) => Selector::Lines(code, a, b.node as usize),
                            None => Selector::Name(format!("{}.line{}", code, a)),
                        });
                    }
                    let mut name = head.node.0;
                    let mut children = false;
                    for (i, part) in rest.iter().enumerate() {
                        match part {
                            Some(id) => {
                                name.push('.');
                                name.push_str(&id.node.0);
                            }
                            None => {
                                if i + 1 != rest.len() {
                                    return Err(Rich::custom(span, "`.*` must end a selector"));
                                }
                                children = true;
                            }
                        }
                    }
                    Ok(if children { Selector::Children(name) } else { Selector::Name(name) })
                }),
        ))
        .map_with(|sel, e| Spanned::new(sel, span_range(&e.span())))
        .boxed();

        let selector_list = selector
            .clone()
            .separated_by(just(Token::Comma))
            .at_least(1)
            .collect::<Vec<_>>()
            .boxed();

        // Motion option values
        let motion_value = recursive(|mv| {
            let num = just(Token::Minus)
                .or_not()
                .then(number)
                .map(|(neg, n)| if neg.is_some() { -n.node } else { n.node });
            choice((
                just(Token::Vertex)
                    .ignore_then(number)
                    .map(|n| MotionValue::Vertex(n.node as usize)),
                num.then_ignore(just(Token::Percent))
                    .map(|n| MotionValue::Percent(n / 100.0)),
                identifier
                    .then(
                        mv.separated_by(just(Token::Comma))
                            .allow_trailing()
                            .collect::<Vec<_>>()
                            .delimited_by(just(Token::ParenOpen), just(Token::ParenClose)),
                    )
                    .map(|(name, args)| MotionValue::Call(name.node.0, args)),
                // `to: station.dot`
                identifier
                    .then(just(Token::Dot).ignore_then(identifier).repeated().at_least(1).collect::<Vec<_>>())
                    .map(|(head, rest)| {
                        let mut n = head.node.0;
                        for r in rest {
                            n.push('.');
                            n.push_str(&r.node.0);
                        }
                        MotionValue::Name(n)
                    }),
                value_atom.clone().map(|v| match v.node {
                    StyleValue::Number { value, .. } => MotionValue::Number(value),
                    StyleValue::String(s) => MotionValue::Str(s),
                    StyleValue::Identifier(id) => MotionValue::Name(id.0),
                    StyleValue::Keyword(k) => MotionValue::Name(k),
                    other => MotionValue::Style(other),
                }),
            ))
            .map_with(|v, e| Spanned::new(v, span_range(&e.span())))
        })
        .boxed();

        let motion_key = choice((
            identifier.map(|id| Spanned::new(id.node.0, id.span)),
            just(Token::From).map_with(|_, e| Spanned::new("from".to_string(), span_range(&e.span()))),
            just(Token::Label).map_with(|_, e| Spanned::new("label".to_string(), span_range(&e.span()))),
            just(Token::Direction).map_with(|_, e| Spanned::new("direction".to_string(), span_range(&e.span()))),
        ));

        let motion_opt = motion_key
            .then_ignore(just(Token::Colon))
            .then(motion_value.clone())
            .map_with(|(key, value), e| Spanned::new(MotionOpt { key, value }, span_range(&e.span())))
            .boxed();

        let motion_opts = motion_opt
            .clone()
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(just(Token::BracketOpen), just(Token::BracketClose))
            .boxed();

        let opts_or_none = motion_opts.clone().or_not().map(|o| o.unwrap_or_default());

        let motion_arg = choice((
            string_literal.map(|s| Spanned::new(MotionArg::Str(s.node), s.span)),
            just(Token::Minus)
                .or_not()
                .then(number)
                .map_with(|(neg, n), e| {
                    let v = if neg.is_some() { -n.node } else { n.node };
                    Spanned::new(MotionArg::Number(v), span_range(&e.span()))
                }),
            dotted_name.clone().map(|n| Spanned::new(MotionArg::Name(n.node), n.span)),
        ));

        let motion_block = recursive(|block| {
            let stmt = |verb: MotionVerb, opts: Vec<Spanned<MotionOpt>>| {
                MotionNode::Stmt(MotionStmt { verb, opts, targets: vec![], partners: vec![] })
            };

            let show = just(Token::Show)
                .ignore_then(selector_list.clone())
                .then(opts_or_none.clone())
                .map(move |(t, o)| stmt(MotionVerb::Show(t), o));
            let hide = just(Token::Hide)
                .ignore_then(selector_list.clone())
                .then(opts_or_none.clone())
                .map(move |(t, o)| stmt(MotionVerb::Hide(t), o));
            // `remove c.lines[8..12]`: hide and close up the room they took.
            let remove = kw("remove")
                .ignore_then(selector_list.clone())
                .then(opts_or_none.clone())
                .map_with(move |(t, mut o), e| {
                    if !o.iter().any(|x: &Spanned<MotionOpt>| x.node.key.node == "exit") {
                        let sp = span_range(&e.span());
                        o.push(Spanned::new(
                            MotionOpt {
                                key: Spanned::new("exit".to_string(), sp.clone()),
                                value: Spanned::new(MotionValue::Name("collapse".into()), sp.clone()),
                            },
                            sp,
                        ));
                    }
                    stmt(MotionVerb::Hide(t), o)
                });
            // `insert c after line 7 [source: "...", as: conflict]`
            let insert = kw("insert")
                .ignore_then(dotted_name.clone().labelled("the code block to insert into"))
                .then_ignore(kw("after").labelled("`after line <n>`"))
                .then_ignore(just(Token::Line).labelled("`line <n>`"))
                .then(number.labelled("a line number"))
                .then(opts_or_none.clone())
                .map(move |((code, after), o)| stmt(MotionVerb::Insert { code, after: after.node as usize }, o));
            let transform = just(Token::Transform)
                .ignore_then(selector.clone())
                .then(modifier_block.clone())
                .map(move |(target, modifiers)| {
                    // Timing keys ride along in the same brackets; split them
                    // off so the state keys stay exactly what they were.
                    let (timing, state): (Vec<_>, Vec<_>) = modifiers.into_iter().partition(|m| {
                        matches!(&m.node.key.node, StyleKey::Custom(k) if crate::motion::is_timing_key(k))
                    });
                    let opts = timing
                        .into_iter()
                        .map(|m| {
                            let StyleKey::Custom(k) = &m.node.key.node else { unreachable!() };
                            let value = match &m.node.value.node {
                                StyleValue::Number { value, .. } => MotionValue::Number(*value),
                                StyleValue::String(s) => MotionValue::Str(s.clone()),
                                StyleValue::Identifier(id) => MotionValue::Name(id.0.clone()),
                                StyleValue::Keyword(k) => MotionValue::Name(k.clone()),
                                other => MotionValue::Style(other.clone()),
                            };
                            Spanned::new(
                                MotionOpt {
                                    key: Spanned::new(k.clone(), m.node.key.span.clone()),
                                    value: Spanned::new(value, m.node.value.span.clone()),
                                },
                                m.span.clone(),
                            )
                        })
                        .collect();
                    stmt(MotionVerb::Transform { target, modifiers: state }, opts)
                });
            let constrain = constrain_decl
                .clone()
                .map(move |d| stmt(MotionVerb::Constrain(d), vec![]));
            let disable = just(Token::Disable)
                .ignore_then(identifier.separated_by(just(Token::Comma)).at_least(1).collect::<Vec<_>>())
                .map(move |n| stmt(MotionVerb::Disable(n), vec![]));
            let enable = just(Token::Enable)
                .ignore_then(identifier.separated_by(just(Token::Comma)).at_least(1).collect::<Vec<_>>())
                .map(move |n| stmt(MotionVerb::Enable(n), vec![]));
            let draw = kw("draw")
                .ignore_then(selector_list.clone())
                .then(opts_or_none.clone())
                .map(move |(t, o)| stmt(MotionVerb::Draw(t), o));
            let undraw = kw("undraw")
                .ignore_then(selector_list.clone())
                .then(opts_or_none.clone())
                .map(move |(t, o)| stmt(MotionVerb::Undraw(t), o));
            let fly_subject = choice((
                kw("ghost")
                    .ignore_then(selector.clone().delimited_by(just(Token::ParenOpen), just(Token::ParenClose)))
                    .map(FlySubject::Ghost),
                selector.clone().map(FlySubject::Proxy),
            ));
            let fly = kw("fly")
                .ignore_then(fly_subject)
                .then(just(Token::From).ignore_then(dotted_name.clone()).or_not())
                .then_ignore(kw("to").labelled("`to` and a destination"))
                .then(selector_list.clone().labelled("a destination after 'to'"))
                .then(opts_or_none.clone())
                .map(move |(((subject, from), to), o)| stmt(MotionVerb::Fly { subject, from, to }, o));
            let mv = kw("move")
                .ignore_then(selector.clone())
                .then(choice((
                    kw("home").map_with(|_, e| (Some(Spanned::new("home".to_string(), span_range(&e.span()))), None)),
                    kw("to").ignore_then(dotted_name.clone().labelled("where to move it (an element, or `home`)")).map(|n| (Some(n), None)),
                    kw("along").ignore_then(dotted_name.clone().labelled("a path to move along")).map(|n| (None, Some(n))),
                )).labelled("`to <element>`, `home` or `along <path>` after `move <element>`"))
                .then(opts_or_none.clone())
                .map(move |((target, (to, along)), o)| stmt(MotionVerb::Move { target, to, along }, o));
            let effect_name = any()
                .filter(|t: &Token| {
                    matches!(t, Token::Ident(s) if crate::motion::EFFECTS.contains(&s.as_str()))
                })
                .map_with(|t, e| match t {
                    Token::Ident(s) => Spanned::new(s, span_range(&e.span())),
                    _ => unreachable!(),
                });
            let effect = effect_name
                .clone()
                .then(selector_list.clone())
                .then(opts_or_none.clone())
                .map(move |((name, targets), o)| stmt(MotionVerb::Effect { name, targets }, o));
            let lp = kw("loop")
                .ignore_then(selector_list.clone())
                .then(
                    effect_name
                        .clone()
                        .then(just(Token::Comma).ignore_then(motion_opt.clone()).repeated().collect::<Vec<_>>())
                        .delimited_by(just(Token::BracketOpen), just(Token::BracketClose)),
                )
                .map(move |(targets, (effect, o))| stmt(MotionVerb::Loop { targets, effect }, o));
            let count = kw("count")
                .ignore_then(selector.clone())
                .then(opts_or_none.clone())
                .map(move |(t, o)| stmt(MotionVerb::Count(t), o));
            let swap = kw("swap")
                .ignore_then(selector.clone())
                .then_ignore(just(Token::Arrow))
                .then(selector.clone())
                .then(opts_or_none.clone())
                .map(move |((from, to), o)| stmt(MotionVerb::Swap { from, to }, o));
            let camera = kw("camera")
                .ignore_then(choice((
                    kw("focus").ignore_then(dotted_name.clone()).map(Some),
                    kw("reset").to(None),
                )))
                .then(opts_or_none.clone())
                .map(move |(f, o)| stmt(MotionVerb::Camera(f), o));
            let call = identifier
                .then(
                    motion_arg
                        .clone()
                        .separated_by(just(Token::Comma))
                        .allow_trailing()
                        .collect::<Vec<_>>()
                        .delimited_by(just(Token::ParenOpen), just(Token::ParenClose)),
                )
                .then(opts_or_none.clone())
                .map(move |((name, args), o)| {
                    stmt(
                        MotionVerb::Call { name: Spanned::new(name.node.0, name.span), args },
                        o,
                    )
                });
            let then_beat = kw("then")
                .ignore_then(block.clone().delimited_by(just(Token::BraceOpen), just(Token::BraceClose)))
                .map(MotionNode::Then);
            // A small nudge after an event: `+ 0.2`, `- 0.1`.
            let nudge = choice((just(Token::Plus).to(1.0), just(Token::Minus).to(-1.0)))
                .then(number)
                .map(|(sign, n)| sign * n.node)
                .or_not()
                .map(|v| v.unwrap_or(0.0));
            let body = block.clone().delimited_by(just(Token::BraceOpen), just(Token::BraceClose));
            let after_beat = kw("after")
                .ignore_then(number)
                .then(body.clone())
                .map(|(n, b)| MotionNode::After(n.node, b));
            // `after tick + 0.2 { }`: after a named beat ends.
            let after_named = kw("after")
                .ignore_then(identifier)
                .then(nudge.clone())
                .then(body.clone())
                .map(|((name, off), b)| {
                    MotionNode::When(MotionEvent::BeatEnd(Spanned::new(name.node.0, name.span)), off, b)
                });
            // `when branch reaches l1.dot { }`, `when anna arrives + 0.1 { }`,
            // `when mr shown { }`, `when card hidden { }`
            let event = choice((
                dotted_name
                    .clone()
                    .then_ignore(kw("reaches"))
                    .then(dotted_name.clone().labelled("what the line reaches"))
                    .map(|(line, target)| MotionEvent::Reaches { line, target }),
                dotted_name.clone().then_ignore(kw("arrives")).map(MotionEvent::Arrives),
                dotted_name.clone().then_ignore(kw("shown")).map(MotionEvent::Shown),
                dotted_name.clone().then_ignore(kw("hidden")).map(MotionEvent::Hidden),
            ))
            .labelled("an event: `<line> reaches <element>`, `<element> arrives`, `<element> shown` or `<element> hidden`");
            let when_beat = kw("when")
                .ignore_then(event)
                .then(nudge.clone())
                .then(body.clone())
                .map(|((ev, off), b)| MotionNode::When(ev, off, b));
            let use_layout = kw("use")
                .ignore_then(kw("layout"))
                .ignore_then(identifier.labelled("a layout name (or `default`)"))
                .then(opts_or_none.clone())
                .map(move |(name, o)| stmt(MotionVerb::UseLayout(Spanned::new(name.node.0, name.span)), o));
            let set_state = kw("set")
                .ignore_then(selector.clone())
                .then(identifier.labelled("a state of the component's template"))
                .then(opts_or_none.clone())
                .map(move |((t, st), o)| stmt(MotionVerb::SetState { target: t, state: Spanned::new(st.node.0, st.span) }, o));
            let named_beat = kw("beat")
                .ignore_then(identifier)
                .then(body.clone())
                .map(|(name, b)| MotionNode::Beat(name.node.0, b));
            let at_beat = kw("at")
                .ignore_then(number)
                .then(block.clone().delimited_by(just(Token::BraceOpen), just(Token::BraceClose)))
                .map(|(n, b)| MotionNode::At(n.node, b));

            choice((
                then_beat,
                after_beat,
                after_named,
                when_beat,
                named_beat,
                use_layout,
                set_state,
                at_beat,
                show,
                hide,
                remove,
                insert,
                transform,
                constrain,
                disable,
                enable,
                draw,
                undraw,
                fly,
                mv,
                lp,
                count,
                swap,
                camera,
                effect,
                call,
            ))
            .map_with(|n, e| Spanned::new(n, span_range(&e.span())))
            .then_ignore(just(Token::Semicolon).or_not())
            .repeated()
            .collect::<Vec<_>>()
        })
        .boxed();

        // `[no_resolve]`, `[auto]`, `[auto, after: 0.6]`
        // `[title: "...", note: "..."]`: host metadata (a deck's header, notes).
        #[derive(Clone)]
        enum FlagVal {
            None,
            Num(f64),
            Str(String),
        }
        let keyframe_flag = identifier
            .then(
                just(Token::Colon)
                    .ignore_then(choice((
                        number.map(|n| FlagVal::Num(n.node)),
                        string_literal.map(|s| FlagVal::Str(s.node)),
                    )))
                    .or_not()
                    .map(|v| v.unwrap_or(FlagVal::None)),
            )
            .try_map(|(id, val), span| match (id.node.0.as_str(), val) {
                ("no_resolve", FlagVal::None) => Ok(("no_resolve", 0.0, None)),
                ("auto", FlagVal::None) => Ok(("auto", 0.0, None)),
                ("after", FlagVal::Num(v)) => Ok(("after", v, None)),
                ("title", FlagVal::Str(s)) => Ok(("title", 0.0, Some(s))),
                ("note", FlagVal::Str(s)) => Ok(("note", 0.0, Some(s))),
                (other, _) => Err(Rich::custom(
                    span,
                    format!(
                        "unknown keyframe flag '{}': expected no_resolve, auto, after: <seconds>, title: \"...\", note: \"...\"",
                        other
                    ),
                )),
            });
        let keyframe_flags = keyframe_flag
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(just(Token::BracketOpen), just(Token::BracketClose));

        let keyframe_decl = just(Token::Keyframe)
            .ignore_then(string_literal)
            .then(keyframe_flags.or_not())
            .then(motion_block.clone().delimited_by(just(Token::BraceOpen), just(Token::BraceClose)))
            .map(|((name, flags), motion)| {
                let flags = flags.unwrap_or_default();
                let no_resolve = flags.iter().any(|(k, _, _)| *k == "no_resolve");
                let auto_on = flags.iter().any(|(k, _, _)| *k == "auto");
                let after = flags.iter().find(|(k, _, _)| *k == "after").map(|(_, v, _)| *v);
                let auto = if auto_on || after.is_some() { Some(after.unwrap_or(0.0)) } else { None };
                let text = |key: &str| flags.iter().find(|(k, _, _)| *k == key).and_then(|(_, _, s)| s.clone());
                let (title, note) = (text("title"), text("note"));
                let operations = crate::motion::naive_operations(&motion);
                KeyframeDecl { name, operations, no_resolve, motion, auto, title, note }
            });

        // `motion commit(folder: element, station) { ... }`
        let motion_param = identifier
            .then(
                just(Token::Colon)
                    .ignore_then(choice((
                        identifier,
                        just(Token::Text).map_with(|_, e| Spanned::new(Identifier::new("text"), span_range(&e.span()))),
                        just(Token::Path).map_with(|_, e| Spanned::new(Identifier::new("path"), span_range(&e.span()))),
                        just(Token::Group).map_with(|_, e| Spanned::new(Identifier::new("group"), span_range(&e.span()))),
                        just(Token::Anchor).map_with(|_, e| Spanned::new(Identifier::new("anchor"), span_range(&e.span()))),
                    )))
                    .or_not(),
            )
            .try_map(|(name, ty), span| {
                let ty = match ty {
                    None => MotionParamType::Element,
                    Some(t) => MotionParamType::parse(&t.node.0).ok_or_else(|| {
                        Rich::custom(
                            span,
                            format!(
                                "unknown parameter type '{}': expected element, group, path, anchor, number or text",
                                t.node.0
                            ),
                        )
                    })?,
                };
                Ok((Spanned::new(name.node.0, name.span), ty))
            });
        let motion_macro = kw("motion")
            .ignore_then(identifier)
            .then(
                motion_param
                    .separated_by(just(Token::Comma))
                    .allow_trailing()
                    .collect::<Vec<_>>()
                    .delimited_by(just(Token::ParenOpen), just(Token::ParenClose)),
            )
            .then(motion_block.clone().delimited_by(just(Token::BraceOpen), just(Token::BraceClose)))
            .map(|((name, params), body)| {
                Statement::MotionMacro(MotionMacroDecl {
                    name: Spanned::new(name.node.0, name.span),
                    params,
                    body,
                })
            });
        let motion_defaults = kw("motion")
            .ignore_then(motion_opts.clone())
            .map(Statement::MotionDefaults);
        let import_stmt = kw("import")
            .ignore_then(string_literal)
            .map(Statement::Import);

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
        // Top-level `disable a_home, b_home`: release a pin written elsewhere.
        // Same spelling as the keyframe op, so a composing file does not have
        // to learn a second syntax for the same idea.
        let disable_stmt = just(Token::Disable)
            .ignore_then(
                identifier
                    .separated_by(just(Token::Comma))
                    .at_least(1)
                    .collect::<Vec<_>>(),
            )
            .map(Statement::DisableConstraint);

        // `layout beside { constrain ...; constrain ... }`
        let named_layout = identifier
            .filter(|i: &Spanned<Identifier>| i.node.0 == "layout")
            .ignore_then(identifier)
            .then(
                constrain_decl
                    .clone()
                    .map_with(|c, e| Spanned::new(c, span_range(&e.span())))
                    .then_ignore(just(Token::Semicolon).or_not())
                    .repeated()
                    .collect::<Vec<_>>()
                    .delimited_by(just(Token::BraceOpen), just(Token::BraceClose)),
            )
            .map(|(name, constraints)| Statement::NamedLayout { name: Spanned::new(name.node.0, name.span), constraints });
        // `state done { ... }` (in a template body)
        // `state done { ... }` in a template (owned by each instance), or
        // `state status broken { ... }` for one element anywhere.
        let component_state = identifier
            .filter(|i: &Spanned<Identifier>| i.node.0 == "state")
            .ignore_then(identifier)
            .then(identifier.or_not())
            .then(motion_block.clone().delimited_by(just(Token::BraceOpen), just(Token::BraceClose)))
            .map(|((first, second), body)| match second {
                Some(name) => Statement::ComponentState { name: Spanned::new(name.node.0, name.span), body, owner: first.node.0 },
                None => Statement::ComponentState { name: Spanned::new(first.node.0, first.span), body, owner: String::new() },
            });
        choice((
            disable_stmt,
            named_layout,
            component_state,
            constrain_decl.clone().map(Statement::Constrain),
            constraint_decl.clone().map(Statement::Constraint),
            keyframe_decl.map(Statement::Keyframe), // Feature 011: before templates
            motion_macro,
            motion_defaults,
            import_stmt,
            file_template.clone(),
            inline_template,
            export_decl.clone().map(Statement::Export),
            anchor_decl, // Feature 009: anchor declarations
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
        .map(|statements| Document { statements, aliases: Vec::new() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_fill_hatch_no_args() {
        // `fill: hatch` parses as a bare identifier (Call requires parens).
        parse("rect a [fill: hatch]").expect("Should parse");
    }

    #[test]
    fn parse_fill_hatch_with_color() {
        parse("rect a [fill: hatch(accent-1)]").expect("Should parse");
    }

    #[test]
    fn parse_fill_gradient_three_args() {
        parse("rect a [fill: gradient(blue, white, 90)]").expect("Should parse");
    }

    #[test]
    fn parse_fill_radial_gradient() {
        parse("rect a [fill: radial_gradient(white, accent-1)]").expect("Should parse");
    }

    #[test]
    fn parse_fill_grid_with_color() {
        // `grid` is a reserved token but must work as a pattern-fill function.
        parse("rect a [fill: grid(foreground-2)]").expect("Should parse");
    }

    #[test]
    fn parse_fill_bare_grid() {
        parse("rect a [fill: grid]").expect("Should parse");
    }

    #[test]
    fn parse_fill_unknown_function_errors() {
        assert!(parse("rect a [fill: bogus(blue, white)]").is_err());
    }

    #[test]
    fn parse_fill_gradient_one_stop_errors() {
        assert!(parse("rect a [fill: gradient(blue)]").is_err());
    }

    #[test]
    fn parse_fill_pattern_too_many_args_errors() {
        assert!(parse("rect a [fill: hatch(a, b, c)]").is_err());
    }

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
                assert_eq!(
                    conns[0].from.anchor.as_ref().map(|s| s.node.as_str()),
                    Some("right")
                );
                assert_eq!(conns[0].to.element.node.as_str(), "b");
                assert_eq!(
                    conns[0].to.anchor.as_ref().map(|s| s.node.as_str()),
                    Some("left")
                );
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
                assert_eq!(
                    conns[0].from.anchor.as_ref().map(|s| s.node.as_str()),
                    Some("top")
                );
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
    fn test_parse_file_template_raster_png() {
        let doc = parse(r#"template "photo" from "images/person.png""#).expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::TemplateDecl(t) => {
                assert_eq!(t.name.node.as_str(), "photo");
                assert_eq!(t.source_type, TemplateSourceType::Raster);
                assert_eq!(t.source_path.as_ref().unwrap().node, "images/person.png");
            }
            other => panic!("Expected TemplateDecl, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_file_template_raster_jpg() {
        let doc = parse(r#"template "photo" from "images/person.JPG""#).expect("Should parse");
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].node {
            Statement::TemplateDecl(t) => {
                assert_eq!(t.name.node.as_str(), "photo");
                assert_eq!(t.source_type, TemplateSourceType::Raster);
            }
            other => panic!("Expected TemplateDecl, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_file_template_raster_webp() {
        let doc = parse(r#"template "icon" from "assets/icon.webp""#).expect("Should parse");
        match &doc.statements[0].node {
            Statement::TemplateDecl(t) => {
                assert_eq!(t.source_type, TemplateSourceType::Raster);
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
                                ArcParams::Radius { radius, sweep, .. } => {
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
            Statement::AnchorDecl(a) => match &a.direction {
                Some(AnchorDirectionSpec::Angle(angle)) => {
                    assert_eq!(*angle, 45.0);
                }
                other => panic!("Expected Angle direction, got {:?}", other),
            },
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

    #[test]
    fn parses_keyframe_constraint_ops() {
        use crate::parser::ast::{Statement, KeyframeOp};
        let src = r#"
rect a [width: 10, height: 10]
constrain a.center_x = 10 as a_home
keyframe "k" {
    disable a_home
    constrain a.center_x = 200
    enable a_home
}
"#;
        let doc = parse(src).expect("parse ok");
        let kf = doc.statements.iter().find_map(|s| match &s.node {
            Statement::Keyframe(k) => Some(k), _ => None,
        }).expect("keyframe");
        let has_disable = kf.operations.iter().any(|o| matches!(&o.node, KeyframeOp::Disable(n) if n.iter().any(|x| x.node.0 == "a_home")));
        let has_enable  = kf.operations.iter().any(|o| matches!(&o.node, KeyframeOp::Enable(n) if n.iter().any(|x| x.node.0 == "a_home")));
        let has_constr  = kf.operations.iter().any(|o| matches!(&o.node, KeyframeOp::Constrain(_)));
        assert!(has_disable, "disable parsed");
        assert!(has_enable, "enable parsed");
        assert!(has_constr, "keyframe-scoped constrain parsed");
    }

    #[test]
    fn parses_named_constraint() {
        use crate::parser::ast::Statement;
        let src = r#"
rect a [width: 10, height: 10]
constrain a.center_x = 50 as a_home
"#;
        let doc = parse(src).expect("parse ok");
        let named = doc.statements.iter().any(|s| matches!(
            &s.node, Statement::Constrain(c) if c.name.as_ref().map(|n| n.node.0.as_str()) == Some("a_home")
        ));
        assert!(named, "constraint should be named a_home");
    }

    #[test]
    fn parses_dx_dy_scale_transform_keys() {
        use crate::parser::ast::{Statement, KeyframeOp, StyleKey};
        let src = r#"
rect box [width: 10, height: 10]
keyframe "k" { transform box [dx: 5, dy: -3, scale: 2] }
"#;
        let doc = parse(src).expect("parse ok");
        let kf = doc.statements.iter().find_map(|s| match &s.node {
            Statement::Keyframe(k) => Some(k),
            _ => None,
        }).expect("keyframe present");
        let modifiers = kf.operations.iter().find_map(|op| match &op.node {
            KeyframeOp::Transform { modifiers, .. } => Some(modifiers),
            _ => None,
        }).expect("transform op");
        let keys: Vec<&StyleKey> = modifiers.iter().map(|m| &m.node.key.node).collect();
        assert!(keys.contains(&&StyleKey::Dx), "dx should map to StyleKey::Dx, got {:?}", keys);
        assert!(keys.contains(&&StyleKey::Dy), "dy should map to StyleKey::Dy");
        assert!(keys.contains(&&StyleKey::Scale), "scale should map to StyleKey::Scale");
    }
}
