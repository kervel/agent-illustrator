//! SVG file templates with addressable parts.
//!
//! `template "doc" from "doc.svg"`: every drawable element of the file that
//! carries an `id` becomes a part of each instance (`d.fold`, `d.bar3`), laid
//! out where the artwork has it, so it can be constrained, anchored to and
//! animated like anything else. Parts nest as the file nests them (moving
//! `d.bars` carries `d.bar3`). Colours written as CSS variables in the file,
//! `fill="var(--bar, #cbd1da)"`, are instance arguments (`doc d [bar:
//! role-primary]`); theme tokens (`var(--role-ink, #111)`) follow the deck.
//!
//! Each part is cut out of the file as its own fragment (wrapped in copies
//! of its ancestors, so inherited attributes and transforms still apply);
//! the rest of the file is the `art` fragment, which also keeps `<defs>`.
//! Ids are prefixed per instance so gradients and clip paths resolve.

use crate::parser::ast::{
    ConstrainDecl, ConstraintExpr, ConstraintProperty, ElementPath, GroupDecl, Identifier, PropertyRef, ShapeDecl,
    ShapeType, Spanned, Statement, StyleKey, StyleModifier, StyleValue,
};
use std::ops::Range;

const DRAWABLE: &[&str] = &[
    "g", "path", "rect", "circle", "ellipse", "line", "polyline", "polygon", "text", "use", "image",
];
const NOT_DRAWN: &[&str] = &[
    "defs", "clipPath", "mask", "pattern", "symbol", "linearGradient", "radialGradient", "filter", "marker",
];

/// One addressable part: its own drawing (without its sub-parts) and them.
#[derive(Debug, Clone)]
pub struct Part {
    pub id: String,
    /// The colours the part draws with (its own, or inherited in the file);
    /// they move to the part's group, and the element says `inherit`.
    pub fill: Option<String>,
    pub stroke: Option<String>,
    /// The part's own markup, wrapped in its ancestors' attributes.
    pub snippet: String,
    /// The same with its original paint, for measuring its extent.
    pub measure: String,
    pub children: Vec<Part>,
}

#[derive(Debug, Clone)]
pub struct SvgParts {
    /// The `<svg ...>` opening tag of the file.
    pub open_tag: String,
    /// The file with every part cut out (defs stay here).
    pub art: String,
    pub parts: Vec<Part>,
    /// Every id the file defines (to namespace per instance).
    pub ids: Vec<String>,
    /// CSS variables the file reads (`var(--bar, ...)`): instance arguments.
    pub vars: Vec<String>,
}

fn is_part(n: &roxmltree::Node) -> bool {
    n.is_element()
        && n.attribute("id").is_some()
        && DRAWABLE.contains(&n.tag_name().name())
        && !n.ancestors().skip(1).any(|a| a.is_element() && NOT_DRAWN.contains(&a.tag_name().name()))
}

/// The nearest ancestor that is a part (not counting `n`).
fn part_parent<'a, 'i>(n: &roxmltree::Node<'a, 'i>) -> Option<roxmltree::Node<'a, 'i>> {
    n.ancestors().skip(1).find(|a| is_part(a))
}

fn cut(src: &str, whole: Range<usize>, cuts: &[Range<usize>]) -> String {
    let mut out = String::new();
    let mut at = whole.start;
    let mut cuts: Vec<&Range<usize>> = cuts.iter().filter(|c| c.start >= whole.start && c.end <= whole.end).collect();
    cuts.sort_by_key(|c| c.start);
    for c in cuts {
        if c.start >= at {
            out.push_str(&src[at..c.start]);
            at = c.end;
        }
    }
    out.push_str(&src[at..whole.end]);
    out
}

fn attrs_except(n: &roxmltree::Node, skip: &[&str]) -> String {
    n.attributes()
        .filter(|a| !skip.contains(&a.name()))
        .map(|a| {
            let name = match a.namespace() {
                Some(ns) if ns.contains("xlink") => format!("xlink:{}", a.name()),
                _ => a.name().to_string(),
            };
            format!(" {}=\"{}\"", name, a.value().replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;"))
        })
        .collect()
}

/// Wrap a part's markup in its ancestors (below the root), minus their ids,
/// so it draws with what it inherits.
fn wrap_in_ancestors(n: &roxmltree::Node, markup: String) -> String {
    let chain: Vec<roxmltree::Node> = n
        .ancestors()
        .skip(1)
        .filter(|a| a.is_element() && a.parent().is_some_and(|p| p.is_element()))
        .collect();
    let mut s = markup;
    for a in chain {
        // Fill and stroke are carried by the part's own group instead.
        s = format!("<g{}>{}</g>", attrs_except(&a, &["id", "fill", "stroke"]), s);
    }
    s
}

/// Wrap in the ancestors with every attribute but ids (for measuring).
fn wrap_in_ancestors_full(n: &roxmltree::Node, markup: String) -> String {
    let mut s = markup;
    for a in n.ancestors().skip(1).filter(|a| a.is_element() && a.parent().is_some_and(|p| p.is_element())) {
        s = format!("<g{}>{}</g>", attrs_except(&a, &["id"]), s);
    }
    s
}

/// A paint attribute of `n`, or of its nearest ancestor that sets it.
fn effective(n: &roxmltree::Node, attr: &str) -> Option<String> {
    n.ancestors().filter(|a| a.is_element()).find_map(|a| a.attribute(attr).map(str::to_string))
}

/// The element's start tag, rewritten: fill/stroke (when it has any in
/// effect) become `inherit`.
fn inheriting_start_tag(n: &roxmltree::Node, src: &str, fill: bool, stroke: bool) -> (String, usize) {
    // End of the original start tag (quote-aware).
    let start = n.range().start;
    let bytes = src.as_bytes();
    let (mut i, mut quote) = (start, 0u8);
    while i < n.range().end {
        let c = bytes[i];
        if quote != 0 {
            if c == quote {
                quote = 0;
            }
        } else if c == b'"' || c == b'\'' {
            quote = c;
        } else if c == b'>' {
            break;
        }
        i += 1;
    }
    let self_closing = i > 0 && bytes[i - 1] == b'/';
    let tag = n.tag_name().name().to_string();
    let mut attrs = attrs_except(n, &["fill", "stroke"]);
    if fill {
        attrs.push_str(" fill=\"inherit\"");
    }
    if stroke {
        attrs.push_str(" stroke=\"inherit\"");
    }
    (format!("<{}{}{}>", tag, attrs, if self_closing { "/" } else { "" }), i + 1)
}

fn css_vars(src: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut rest = src;
    while let Some(i) = rest.find("var(--") {
        let after = &rest[i + 6..];
        let name: String = after.chars().take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_').collect();
        if !name.is_empty() && !out.contains(&name) {
            out.push(name);
        }
        rest = &after[..];
    }
    out
}

/// Parse a file's parts. `None` when it has none (the file stays one embed).
pub fn analyze(content: &str) -> Option<SvgParts> {
    let opts = roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() };
    let doc = roxmltree::Document::parse_with_options(content, opts).ok()?;
    let root = doc.root_element();
    let parts_nodes: Vec<roxmltree::Node> = root.descendants().filter(|n| is_part(n) && n != &root).collect();
    if parts_nodes.is_empty() {
        return None;
    }
    fn build(n: roxmltree::Node, all: &[roxmltree::Node], src: &str) -> Part {
        let kids: Vec<roxmltree::Node> =
            all.iter().filter(|c| part_parent(c).is_some_and(|p| p == n)).copied().collect();
        let cuts: Vec<Range<usize>> = kids.iter().map(|k| k.range()).collect();
        let (fill, stroke) = (effective(&n, "fill"), effective(&n, "stroke"));
        // A leaf part inherits its paint from its group; a part made of parts
        // keeps its own (a transform recolours its leaves one by one).
        let leaf = kids.is_empty();
        let own = if leaf {
            let (tag, body_start) = inheriting_start_tag(&n, src, fill.is_some(), stroke.is_some());
            format!("{}{}", tag, cut(src, body_start..n.range().end, &cuts))
        } else {
            cut(src, n.range(), &cuts)
        };
        Part {
            id: n.attribute("id").unwrap_or_default().to_string(),
            fill: if leaf { fill } else { None },
            stroke: if leaf { stroke } else { None },
            snippet: wrap_in_ancestors(&n, own),
            measure: wrap_in_ancestors_full(&n, cut(src, n.range(), &cuts)),
            children: kids.into_iter().map(|k| build(k, all, src)).collect(),
        }
    }
    let top: Vec<roxmltree::Node> = parts_nodes.iter().filter(|n| part_parent(n).is_none()).copied().collect();
    let parts = top.iter().map(|n| build(*n, &parts_nodes, content)).collect();
    let top_cuts: Vec<Range<usize>> = top.iter().map(|n| n.range()).collect();
    let art = cut(content, 0..content.len(), &top_cuts);
    let tag_end = content[root.range().start..].find('>').map(|i| root.range().start + i + 1)?;
    let ids = doc
        .descendants()
        .filter_map(|n| n.attribute("id").map(str::to_string))
        .collect();
    Some(SvgParts {
        open_tag: content[root.range().start..tag_end].to_string(),
        art,
        parts,
        ids,
        vars: css_vars(content),
    })
}

/// The drawn extent of some markup, in the file's coordinates.
fn extent(open_tag: &str, defs: &str, markup: &str) -> Option<(f64, f64, f64, f64)> {
    // The measuring parser knows no CSS variables: use their fallbacks.
    let svg = with_fallbacks(&format!("{}{}{}</svg>", open_tag, defs, markup));
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).ok()?;
    let b = tree.root().abs_stroke_bounding_box();
    let (x, y, w, h) = (b.x() as f64, b.y() as f64, b.width() as f64, b.height() as f64);
    (w > 0.0 && h > 0.0 && x.is_finite() && y.is_finite()).then_some((x, y, w, h))
}

/// Every `var(--x, fallback)` replaced by its fallback (`#000` without one).
fn with_fallbacks(src: &str) -> String {
    let mut s = src.to_string();
    while let Some(i) = s.find("var(--") {
        let after = &s[i + 4..];
        let (mut depth, mut comma, mut end) = (1, None, None);
        for (j, c) in after.char_indices() {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(j);
                        break;
                    }
                }
                ',' if depth == 1 && comma.is_none() => comma = Some(j),
                _ => {}
            }
        }
        let Some(end) = end else { break };
        let value = match comma {
            Some(c) => after[c + 1..end].trim().to_string(),
            None => "#000".to_string(),
        };
        s = format!("{}{}{}", &s[..i], value, &after[end + 1..]);
    }
    s
}

/// `defs` of the file (for measuring a fragment that uses a gradient).
fn defs_of(content: &str) -> String {
    let opts = roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() };
    let Ok(doc) = roxmltree::Document::parse_with_options(content, opts) else { return String::new() };
    doc.descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "defs")
        .map(|n| content[n.range()].to_string())
        .collect()
}

/// Prefix every id the file defines, and every reference to one.
fn namespace(markup: &str, ids: &[String], ns: &str) -> String {
    let mut s = markup.to_string();
    for id in ids {
        s = s
            .replace(&format!("id=\"{}\"", id), &format!("id=\"{}{}\"", ns, id))
            .replace(&format!("url(#{})", id), &format!("url(#{}{})", ns, id))
            .replace(&format!("href=\"#{}\"", id), &format!("href=\"#{}{}\"", ns, id));
    }
    s
}

/// Replace `var(--name, fallback)` with an instance's value for `name`.
fn apply_vars(markup: &str, values: &[(String, String)]) -> String {
    let mut s = markup.to_string();
    for (name, value) in values {
        let needle = format!("var(--{}", name);
        let mut out = String::new();
        let mut rest = s.as_str();
        while let Some(i) = rest.find(&needle) {
            let after = &rest[i + needle.len()..];
            // Only this exact name (`--bar`, not `--bars`).
            if after.starts_with(|c: char| c.is_alphanumeric() || c == '-' || c == '_') {
                out.push_str(&rest[..i + needle.len()]);
                rest = after;
                continue;
            }
            let mut depth = 1;
            let mut end = None;
            for (j, c) in after.char_indices() {
                match c {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            end = Some(j + 1);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let Some(end) = end else { break };
            out.push_str(&rest[..i]);
            out.push_str(value);
            rest = &after[end..];
        }
        out.push_str(rest);
        s = out;
    }
    s
}

fn modifier(key: StyleKey, value: StyleValue, span: &Range<usize>) -> Spanned<StyleModifier> {
    Spanned::new(
        StyleModifier { key: Spanned::new(key, span.clone()), value: Spanned::new(value, span.clone()) },
        span.clone(),
    )
}

fn num(v: f64) -> StyleValue {
    StyleValue::Number { value: v, unit: None }
}

fn pin(name: &str, prop: ConstraintProperty, to: &str, to_prop: ConstraintProperty, offset: f64, span: &Range<usize>) -> Spanned<Statement> {
    let r = |n: &str, p: ConstraintProperty| PropertyRef {
        element: Spanned::new(ElementPath::simple(Identifier::new(n), span.clone()), span.clone()),
        property: Spanned::new(p, span.clone()),
    };
    Spanned::new(
        Statement::Constrain(ConstrainDecl {
            expr: ConstraintExpr::EqualWithOffset { left: r(name, prop), right: r(to, to_prop), offset },
            name: None,
        }),
        span.clone(),
    )
}

/// The statements for one instance of a file template with parts.
///
/// `origin` is the artwork's trimmed box (the instance's frame of
/// reference); `scale` resizes the whole instance; `values` are the CSS
/// variables the instance sets.
#[allow(clippy::too_many_arguments)]
pub fn instance_statements(
    parts: &SvgParts,
    content: &str,
    instance: &str,
    origin: (f64, f64, f64, f64),
    scale: f64,
    values: &[(String, String)],
    group_modifiers: Vec<Spanned<StyleModifier>>,
    span: &Range<usize>,
) -> Vec<Spanned<Statement>> {
    let ns = format!("{}__", instance);
    let defs = defs_of(content);
    let finish = |m: &str| apply_vars(&namespace(m, &parts.ids, &ns), values);
    let (ox, oy, ow, oh) = origin;
    // Generated names (`__`): never an author's part, shown as their owner.
    let art_name = format!("{}__art", instance);
    let embed = |name: &str, markup: &str, ext: (f64, f64, f64, f64), paint: (&Option<String>, &Option<String>)| -> Spanned<Statement> {
        let (x, y, w, h) = ext;
        let mut modifiers = vec![modifier(StyleKey::Width, num(w * scale), span), modifier(StyleKey::Height, num(h * scale), span)];
        for (key, v) in [(StyleKey::Fill, paint.0), (StyleKey::Stroke, paint.1)] {
            if let Some(v) = v {
                modifiers.push(modifier(key, StyleValue::Color(crate::parser::ast::ColorValue::Named(finish(v))), span));
            }
        }
        Spanned::new(
            Statement::Shape(ShapeDecl {
                shape_type: Spanned::new(
                    ShapeType::SvgEmbed {
                        content: format!("{}{}</svg>", parts.open_tag, finish(markup)),
                        intrinsic_width: Some(w),
                        intrinsic_height: Some(h),
                        offset_x: x,
                        offset_y: y,
                    },
                    span.clone(),
                ),
                name: Some(Spanned::new(Identifier::new(name), span.clone())),
                modifiers,
            }),
            span.clone(),
        )
    };
    let mut body = vec![embed(&art_name, &strip_open(&parts.art, &parts.open_tag), (ox, oy, ow, oh), (&None, &None))];
    fn add(
        p: &Part,
        instance: &str,
        art: &str,
        open_tag: &str,
        defs: &str,
        origin: (f64, f64),
        scale: f64,
        embed: &dyn Fn(&str, &str, (f64, f64, f64, f64), (&Option<String>, &Option<String>)) -> Spanned<Statement>,
        span: &Range<usize>,
    ) -> Option<Spanned<Statement>> {
        let name = format!("{}_{}", instance, p.id);
        let place = |n: &str, (x, y): (f64, f64)| {
            vec![
                pin(n, ConstraintProperty::Left, art, ConstraintProperty::Left, (x - origin.0) * scale, span),
                pin(n, ConstraintProperty::Top, art, ConstraintProperty::Top, (y - origin.1) * scale, span),
            ]
        };
        let own = extent(open_tag, defs, &p.measure);
        if p.children.is_empty() {
            let ext = own?;
            // The part and its pins; the caller files the pins at top level.
            let mut v = vec![embed(&name, &p.snippet, ext, (&p.fill, &p.stroke))];
            v.extend(place(&name, (ext.0, ext.1)));
            return Some(Spanned::new(
                Statement::Group(GroupDecl {
                    name: None,
                    children: v,
                    modifiers: vec![],
                    anchors: vec![],
                    is_template_instance: false,
                }),
                span.clone(),
            ));
        }
        // A part with parts inside: a group of its own drawing and theirs.
        let mut kids = Vec::new();
        if let Some(ext) = own {
            let self_name = format!("{}__self", name);
            kids.push(embed(&self_name, &p.snippet, ext, (&None, &None)));
            kids.extend(place(&self_name, (ext.0, ext.1)));
        }
        for c in &p.children {
            if let Some(s) = add(c, instance, art, open_tag, defs, origin, scale, embed, span) {
                kids.push(s);
            }
        }
        Some(Spanned::new(
            Statement::Group(GroupDecl {
                name: Some(Spanned::new(Identifier::new(&name), span.clone())),
                children: kids,
                modifiers: vec![],
                anchors: vec![],
                is_template_instance: false,
            }),
            span.clone(),
        ))
    }
    for p in &parts.parts {
        if let Some(s) = add(p, instance, &art_name, &parts.open_tag, &defs, (ox, oy), scale, &embed, span) {
            body.push(s);
        }
    }
    let body = flatten_leaf_wrappers(body);
    vec![Spanned::new(
        Statement::Group(GroupDecl {
            name: Some(Spanned::new(Identifier::new(instance), span.clone())),
            children: body,
            modifiers: group_modifiers,
            anchors: vec![],
            is_template_instance: true,
        }),
        span.clone(),
    )]
}

/// A leaf part was returned wrapped in an unnamed group with its pins, so
/// the recursion could return one statement; splice those back in place.
fn flatten_leaf_wrappers(stmts: Vec<Spanned<Statement>>) -> Vec<Spanned<Statement>> {
    let mut out = Vec::new();
    for s in stmts {
        match s.node {
            Statement::Group(g) if g.name.is_none() => out.extend(flatten_leaf_wrappers(g.children)),
            Statement::Group(mut g) => {
                g.children = flatten_leaf_wrappers(g.children);
                out.push(Spanned::new(Statement::Group(g), s.span));
            }
            other => out.push(Spanned::new(other, s.span)),
        }
    }
    out
}

/// The file's markup after its `<svg ...>` tag, without the closing tag.
fn strip_open(full: &str, open_tag: &str) -> String {
    let start = full.find(open_tag).map(|i| i + open_tag.len()).unwrap_or(0);
    let end = full.rfind("</svg>").unwrap_or(full.len());
    full[start..end.max(start)].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 120">
  <defs><linearGradient id="g"><stop offset="0" stop-color="red"/></linearGradient></defs>
  <rect id="sheet" x="10" y="10" width="80" height="100" fill="url(#g)"/>
  <g id="bars" stroke-width="6" stroke="var(--bar, #ccc)">
    <line id="bar1" x1="20" y1="30" x2="70" y2="30"/>
    <line id="bar2" x1="20" y1="50" x2="60" y2="50"/>
  </g>
</svg>"#;

    #[test]
    fn finds_parts_and_nesting() {
        let p = analyze(DOC).expect("parts");
        assert_eq!(p.parts.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(), vec!["sheet", "bars"]);
        let bars = &p.parts[1];
        assert_eq!(bars.children.len(), 2);
        // A bar keeps the stroke width it inherits from its group.
        assert!(bars.children[0].snippet.contains("stroke-width=\"6\""), "{}", bars.children[0].snippet);
        assert!(p.vars.contains(&"bar".to_string()));
        assert!(!p.art.contains("sheet") && p.art.contains("linearGradient"));
    }

    #[test]
    fn vars_and_ids_are_per_instance() {
        let s = apply_vars(&namespace("fill=\"url(#g)\" stroke=\"var(--bar, #ccc)\"", &["g".into()], "d0__"),
            &[("bar".into(), "var(--role-primary)".into())]);
        assert_eq!(s, "fill=\"url(#d0__g)\" stroke=\"var(--role-primary)\"");
    }
}
