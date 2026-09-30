//! PNG output without a browser: `--png`.
//!
//! The SVG this crate writes leans on CSS a rasteriser does not evaluate:
//! colours are `var(--role-ink)`, stills pin their motion with `translate:`
//! and `scale:`, labels take their font from a class rule. So the SVG is
//! first flattened: custom properties are substituted, every rule of its
//! `<style>` is applied to the elements it selects (as attributes, with the
//! CSS cascade: `!important`, specificity, order), and the individual
//! transform properties are folded into a `transform`. What remains is plain
//! SVG, which resvg renders with the fonts bundled in the binary plus any
//! `@font-face` a stylesheet embeds as a data URI. No system font discovery:
//! the same input gives the same picture on every machine.

use std::collections::HashMap;
use std::sync::Arc;

/// Bundled faces: Overpass (sans) and Overpass Mono, SIL OFL 1.1
/// (assets/fonts/OFL.txt).
const SANS: &[u8] = include_bytes!("../assets/fonts/Overpass[wght].ttf");
const MONO: &[u8] = include_bytes!("../assets/fonts/OverpassMono[wght].ttf");
/// Marks the text faces lack (✓ ✕ ★ ⚠ ☐ …): a subset of Noto Sans Symbols 2
/// (OFL), used as the fallback for any character the chosen font has not.
const SYMBOLS: &[u8] = include_bytes!("../assets/fonts/NotoSansSymbols2-subset.ttf");

/// What happened on the way to the picture that the author should know.
pub struct Raster {
    pub png: Vec<u8>,
    pub notes: Vec<String>,
}

pub fn svg_to_png(svg: &str, scale: f32) -> Result<Raster, String> {
    let flat = flatten(svg)?;
    // For debugging the flattening: AIL_PNG_DEBUG_SVG=/tmp/x.svg
    if let Ok(path) = std::env::var("AIL_PNG_DEBUG_SVG") {
        let _ = std::fs::write(path, &flat.svg);
    }
    let mut db = usvg::fontdb::Database::new();
    db.load_font_data(SANS.to_vec());
    db.load_font_data(MONO.to_vec());
    db.load_font_data(SYMBOLS.to_vec());
    db.set_sans_serif_family("Overpass");
    db.set_serif_family("Overpass");
    db.set_monospace_family("Overpass Mono");
    let mut notes = Vec::new();
    for (family, data) in &flat.font_faces {
        // Web fonts come as WOFF/WOFF2: unpack to the plain font first.
        let data = match data.get(0..4) {
            Some(b"wOF2") => wuff::decompress_woff2(data).map_err(|e| format!("{:?}", e)),
            Some(b"wOFF") => wuff::decompress_woff1(data).map_err(|e| format!("{:?}", e)),
            _ => Ok(data.clone()),
        };
        let data = match data {
            Ok(d) => d,
            Err(e) => {
                notes.push(format!("@font-face '{}' could not be unpacked ({}); its text uses the fallback", family, e));
                continue;
            }
        };
        let before: std::collections::HashSet<usvg::fontdb::ID> = db.faces().map(|f| f.id).collect();
        db.load_font_data(data);
        let added: Vec<usvg::fontdb::ID> = db.faces().map(|f| f.id).filter(|id| !before.contains(id)).collect();
        if added.is_empty() {
            notes.push(format!("@font-face '{}' could not be read as a font (TTF, OTF, WOFF and WOFF2 are supported)", family));
        }
        // The stylesheet's name for it wins over the name inside the file
        // (`'Avenir LT Std'` vs `AvenirLTStd-Book`).
        for id in added {
            if let Some(mut info) = db.face(id).cloned() {
                if !info.families.iter().any(|(n, _)| n.eq_ignore_ascii_case(family)) {
                    info.families.insert(0, (family.clone(), usvg::fontdb::Language::English_UnitedStates));
                    db.remove_face(id);
                    db.push_face_info(info);
                }
            }
        }
    }
    // Every family the text asks for first, and whether it is there.
    let mut missing: Vec<String> = flat
        .families
        .iter()
        .filter(|f| !is_generic(f) && !db.faces().any(|face| face.families.iter().any(|(n, _)| n.eq_ignore_ascii_case(f))))
        .cloned()
        .collect();
    missing.sort();
    missing.dedup();
    for f in missing {
        notes.push(format!(
            "font '{}' is not available here (no system fonts are read); the next family in its list, or Overpass, is used. Embed it in the stylesheet as an @font-face data URI to use it",
            f
        ));
    }
    // Characters no available font can draw (they come out as boxes).
    let mut boxes: Vec<char> = flat
        .chars
        .iter()
        .copied()
        .filter(|c| !c.is_whitespace() && !c.is_control())
        .filter(|c| {
            !db.faces().any(|f| {
                db.with_face_data(f.id, |data, index| {
                    ttf_parser::Face::parse(data, index).ok().and_then(|face| face.glyph_index(*c)).is_some()
                })
                .unwrap_or(false)
            })
        })
        .collect();
    boxes.sort();
    boxes.dedup();
    if !boxes.is_empty() {
        notes.push(format!(
            "no available font has {}; they are drawn as boxes. Embed a font that has them as an @font-face data URI",
            boxes.iter().map(|c| format!("'{}' (U+{:04X})", c, *c as u32)).collect::<Vec<_>>().join(", ")
        ));
    }
    if flat.remote_fonts {
        notes.push("the stylesheet @imports fonts from the web; they are not fetched for PNG output".into());
    }
    let opt = usvg::Options { fontdb: Arc::new(db), ..Default::default() };
    let tree = usvg::Tree::from_str(&flat.svg, &opt).map_err(|e| format!("PNG: the flattened SVG does not parse: {}", e))?;
    let size = tree.size().to_int_size().scale_by(scale).ok_or("PNG: empty picture")?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height()).ok_or("PNG: picture too large")?;
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    let png = pixmap.encode_png().map_err(|e| format!("PNG: {}", e))?;
    Ok(Raster { png, notes })
}

fn is_generic(f: &str) -> bool {
    matches!(
        f.to_ascii_lowercase().as_str(),
        "sans-serif" | "serif" | "monospace" | "system-ui" | "ui-monospace" | "ui-sans-serif" | "cursive" | "fantasy"
    )
}

struct Flat {
    svg: String,
    font_faces: Vec<(String, Vec<u8>)>,
    /// The first family of every font-family list in use.
    families: Vec<String>,
    /// Every character of the text.
    chars: std::collections::BTreeSet<char>,
    remote_fonts: bool,
}

// ---------------------------------------------------------------- CSS

struct Rule {
    selectors: Vec<Selector>,
    decls: Vec<(String, String, bool)>,
    order: usize,
}

/// A compound selector chain, rightmost last: `.a > text` is
/// [(Descendant?, [.a]), (Child, [text])].
#[derive(Clone)]
struct Selector {
    parts: Vec<(Comb, Vec<Simple>)>,
    spec: (u32, u32, u32),
}

#[derive(Clone, Copy, PartialEq)]
enum Comb {
    Descendant,
    Child,
}

#[derive(Clone)]
enum Simple {
    Tag(String),
    Class(String),
    Id(String),
    Any,
    Root,
}

fn strip_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(i) = rest.find("/*") {
        out.push_str(&rest[..i]);
        match rest[i + 2..].find("*/") {
            Some(j) => rest = &rest[i + 2 + j + 2..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

fn parse_selector(s: &str) -> Option<Selector> {
    let s = s.trim();
    if s.is_empty() || s.contains(':') && !s.contains(":root") || s.contains('[') || s.contains('+') || s.contains('~') {
        return None;
    }
    let mut parts = Vec::new();
    let mut comb = Comb::Descendant;
    let spaced = s.replace('>', " > ");
    for tok in spaced.split_whitespace() {
        if tok == ">" {
            comb = Comb::Child;
            continue;
        }
        let mut simples = Vec::new();
        let mut cur = String::new();
        let mut kind = 't';
        let flush = |kind: char, cur: &mut String, simples: &mut Vec<Simple>| {
            if cur.is_empty() {
                return;
            }
            simples.push(match kind {
                '.' => Simple::Class(cur.clone()),
                '#' => Simple::Id(cur.clone()),
                ':' => Simple::Root,
                _ if cur == "*" => Simple::Any,
                _ => Simple::Tag(cur.clone()),
            });
            cur.clear();
        };
        for ch in tok.chars() {
            if ch == '.' || ch == '#' || ch == ':' {
                flush(kind, &mut cur, &mut simples);
                kind = ch;
            } else {
                cur.push(ch);
            }
        }
        flush(kind, &mut cur, &mut simples);
        parts.push((comb, simples));
        comb = Comb::Descendant;
    }
    let mut spec = (0, 0, 0);
    for (_, ss) in &parts {
        for x in ss {
            match x {
                Simple::Id(_) => spec.0 += 1,
                Simple::Class(_) | Simple::Root => spec.1 += 1,
                Simple::Tag(_) => spec.2 += 1,
                Simple::Any => {}
            }
        }
    }
    Some(Selector { parts, spec })
}

fn parse_decls(block: &str) -> Vec<(String, String, bool)> {
    // `d: path("M0 0 L1 1")` holds no `;`, but a data URI might: split on
    // `;` outside parentheses and quotes.
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut cur = String::new();
    let mut push = |cur: &mut String| {
        if let Some((k, v)) = cur.split_once(':') {
            let v = v.trim();
            let (v, imp) = match v.strip_suffix("!important") {
                Some(x) => (x.trim().to_string(), true),
                None => (v.to_string(), false),
            };
            out.push((k.trim().to_ascii_lowercase(), v, imp));
        }
        cur.clear();
    };
    for ch in block.chars() {
        match (quote, ch) {
            (Some(q), c) if c == q => quote = None,
            (None, '"') | (None, '\'') => quote = Some(ch),
            (None, '(') => depth += 1,
            (None, ')') => depth -= 1,
            (None, ';') if depth == 0 => {
                push(&mut cur);
                continue;
            }
            _ => {}
        }
        cur.push(ch);
    }
    push(&mut cur);
    out
}

/// Top-level rules and @font-face blocks; @keyframes and @media are skipped
/// (a still has no running animation).
fn parse_css(css: &str, rules: &mut Vec<Rule>, faces: &mut Vec<Vec<(String, String, bool)>>, remote: &mut bool) {
    let css = strip_comments(css);
    let b = css.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let Some(open) = css[i..].find('{').map(|x| x + i) else { break };
        let head = css[i..open].trim().to_string();
        // Matching close brace (nested for @media/@keyframes).
        let mut depth = 0;
        let mut j = open;
        while j < b.len() {
            match b[j] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            j += 1;
        }
        let body = &css[open + 1..j.min(b.len())];
        // Statements before the rule (`@import url(...);`).
        let head = match head.rfind(';') {
            Some(k) => {
                if head[..k].contains("@import") {
                    *remote = true;
                }
                head[k + 1..].trim().to_string()
            }
            None => head,
        };
        if head.starts_with("@font-face") {
            faces.push(parse_decls(body));
        } else if !head.starts_with('@') {
            let selectors: Vec<Selector> = head.split(',').filter_map(parse_selector).collect();
            if !selectors.is_empty() {
                let order = rules.len();
                rules.push(Rule { selectors, decls: parse_decls(body), order });
            }
        }
        i = j + 1;
    }
    if css.contains("@import") && css.contains("fonts.googleapis") {
        *remote = true;
    }
}

/// `var(--x, fallback)` -> its value, recursively.
fn resolve_vars(v: &str, vars: &HashMap<String, String>, depth: usize) -> String {
    if depth > 8 || !v.contains("var(") {
        return v.to_string();
    }
    let mut out = String::new();
    let mut rest = v;
    while let Some(i) = rest.find("var(") {
        out.push_str(&rest[..i]);
        let inner_start = i + 4;
        let mut d = 1;
        let mut j = inner_start;
        let bytes = rest.as_bytes();
        while j < bytes.len() && d > 0 {
            match bytes[j] {
                b'(' => d += 1,
                b')' => d -= 1,
                _ => {}
            }
            j += 1;
        }
        let inner = &rest[inner_start..j.saturating_sub(1)];
        let (name, fallback) = match inner.split_once(',') {
            Some((n, f)) => (n.trim(), Some(f.trim())),
            None => (inner.trim(), None),
        };
        let val = vars.get(name).map(|s| s.as_str()).or(fallback).unwrap_or("");
        out.push_str(&resolve_vars(val, vars, depth + 1));
        rest = &rest[j..];
    }
    out.push_str(rest);
    out
}

// ---------------------------------------------------------------- matching

fn matches_simple(n: roxmltree::Node, s: &Simple) -> bool {
    match s {
        Simple::Any => true,
        Simple::Root => n.parent_element().is_none(),
        Simple::Tag(t) => n.tag_name().name() == t,
        Simple::Id(id) => n.attribute("id") == Some(id.as_str()),
        Simple::Class(c) => n.attribute("class").is_some_and(|cl| cl.split_whitespace().any(|x| x == c)),
    }
}

fn matches_compound(n: roxmltree::Node, ss: &[Simple]) -> bool {
    ss.iter().all(|s| matches_simple(n, s))
}

fn matches(n: roxmltree::Node, sel: &Selector) -> bool {
    fn rec(n: roxmltree::Node, parts: &[(Comb, Vec<Simple>)]) -> bool {
        let Some(((_, last), rest)) = parts.split_last().map(|(l, r)| (l, r)) else { return true };
        if !matches_compound(n, last) {
            return false;
        }
        if rest.is_empty() {
            return true;
        }
        let comb = parts[parts.len() - 1].0;
        match comb {
            Comb::Child => n.parent_element().is_some_and(|p| rec(p, rest)),
            Comb::Descendant => {
                let mut cur = n.parent_element();
                while let Some(p) = cur {
                    if rec(p, rest) {
                        return true;
                    }
                    cur = p.parent_element();
                }
                false
            }
        }
    }
    rec(n, &sel.parts)
}

// ---------------------------------------------------------------- flatten

const PRESENTATION: &[&str] = &[
    "fill", "stroke", "stroke-width", "opacity", "fill-opacity", "stroke-opacity", "stroke-dasharray",
    "stroke-dashoffset", "stroke-linecap", "stroke-linejoin", "font-family", "font-size", "font-weight",
    "font-style", "text-anchor", "dominant-baseline", "display", "visibility", "clip-path", "mask",
    "marker-start", "marker-mid", "marker-end", "rx", "ry", "width", "height", "letter-spacing",
    "text-decoration", "paint-order",
];

fn num(s: &str) -> f64 {
    s.trim().trim_end_matches("px").trim_end_matches("deg").parse().unwrap_or(0.0)
}

fn flatten(svg: &str) -> Result<Flat, String> {
    let doc = roxmltree::Document::parse(svg).map_err(|e| format!("PNG: the SVG does not parse: {}", e))?;
    let mut rules = Vec::new();
    let mut faces_raw = Vec::new();
    let mut remote = false;
    for n in doc.descendants().filter(|n| n.tag_name().name() == "style") {
        let text: String = n.children().filter_map(|c| c.text()).collect();
        parse_css(&text, &mut rules, &mut faces_raw, &mut remote);
    }
    // Custom properties from rules that apply to the root.
    let root = doc.root_element();
    let mut vars: HashMap<String, String> = HashMap::new();
    let mut root_rules: Vec<&Rule> = rules.iter().filter(|r| r.selectors.iter().any(|s| matches(root, s))).collect();
    root_rules.sort_by_key(|r| r.order);
    for r in root_rules {
        for (k, v, _) in &r.decls {
            if k.starts_with("--") {
                vars.insert(k.clone(), v.clone());
            }
        }
    }
    let font_faces: Vec<(String, Vec<u8>)> = faces_raw
        .iter()
        .filter_map(|decls| {
            let family = decls.iter().find(|(k, _, _)| k == "font-family")?.1.trim_matches(|c| c == '"' || c == '\'').to_string();
            let src = &decls.iter().find(|(k, _, _)| k == "src")?.1;
            let start = src.find("base64,")? + 7;
            let end = src[start..].find(|c: char| c == ')' || c == '"' || c == '\'').map(|e| start + e).unwrap_or(src.len());
            use base64::Engine;
            let data = base64::engine::general_purpose::STANDARD.decode(src[start..end].trim()).ok()?;
            Some((family, data))
        })
        .collect();
    if faces_raw.iter().any(|d| d.iter().any(|(k, v, _)| k == "src" && v.contains("url(") && !v.contains("data:"))) {
        remote = true;
    }

    let mut families = Vec::new();
    let mut out = String::with_capacity(svg.len());
    write_node(root, &rules, &vars, &mut out, &mut families);
    let chars = doc
        .descendants()
        .filter(|n| n.is_text() && n.ancestors().any(|a| a.tag_name().name() == "text"))
        .flat_map(|n| n.text().unwrap_or("").chars().collect::<Vec<_>>())
        .collect();
    Ok(Flat { svg: out, font_faces, families, chars, remote_fonts: remote })
}

/// Length of an SVG path (M/L/H/V/C/Q/Z, absolute or relative; curves
/// sampled, arcs by their chord).
fn path_length(d: &str) -> f64 {
    let mut toks: Vec<String> = Vec::new();
    let mut cur = String::new();
    for ch in d.chars() {
        if ch.is_ascii_alphabetic() && ch != 'e' && ch != 'E' {
            if !cur.is_empty() {
                toks.push(std::mem::take(&mut cur));
            }
            toks.push(ch.to_string());
        } else if ch == ',' || ch.is_whitespace() || (ch == '-' && !cur.is_empty() && !cur.ends_with('e')) {
            if !cur.is_empty() {
                toks.push(std::mem::take(&mut cur));
            }
            if ch == '-' {
                cur.push(ch);
            }
        } else {
            cur.push(ch);
        }
    }
    if !cur.is_empty() {
        toks.push(cur);
    }
    let (mut x, mut y, mut sx, mut sy) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    let mut len = 0.0;
    let mut i = 0;
    let mut cmd = 'M';
    let n = |t: &Vec<String>, i: usize| t.get(i).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
    let dist = |a: (f64, f64), b: (f64, f64)| ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
    while i < toks.len() {
        if toks[i].len() == 1 && toks[i].chars().next().unwrap().is_ascii_alphabetic() {
            cmd = toks[i].chars().next().unwrap();
            i += 1;
            if cmd == 'Z' || cmd == 'z' {
                len += dist((x, y), (sx, sy));
                x = sx;
                y = sy;
                continue;
            }
        }
        let rel = cmd.is_ascii_lowercase();
        let (bx, by) = if rel { (x, y) } else { (0.0, 0.0) };
        match cmd.to_ascii_uppercase() {
            'M' => {
                x = bx + n(&toks, i);
                y = by + n(&toks, i + 1);
                sx = x;
                sy = y;
                i += 2;
                cmd = if rel { 'l' } else { 'L' };
            }
            'L' => {
                let p = (bx + n(&toks, i), by + n(&toks, i + 1));
                len += dist((x, y), p);
                (x, y) = p;
                i += 2;
            }
            'H' => {
                let p = (if rel { x } else { 0.0 } + n(&toks, i), y);
                len += dist((x, y), p);
                (x, y) = p;
                i += 1;
            }
            'V' => {
                let p = (x, if rel { y } else { 0.0 } + n(&toks, i));
                len += dist((x, y), p);
                (x, y) = p;
                i += 1;
            }
            'C' | 'Q' => {
                let k = if cmd.to_ascii_uppercase() == 'C' { 3 } else { 2 };
                let pts: Vec<(f64, f64)> = (0..k).map(|j| (bx + n(&toks, i + 2 * j), by + n(&toks, i + 2 * j + 1))).collect();
                let p0 = (x, y);
                let mut prev = p0;
                for s in 1..=24 {
                    let t = s as f64 / 24.0;
                    let q = if k == 3 {
                        let u = 1.0 - t;
                        (
                            u * u * u * p0.0 + 3.0 * u * u * t * pts[0].0 + 3.0 * u * t * t * pts[1].0 + t * t * t * pts[2].0,
                            u * u * u * p0.1 + 3.0 * u * u * t * pts[0].1 + 3.0 * u * t * t * pts[1].1 + t * t * t * pts[2].1,
                        )
                    } else {
                        let u = 1.0 - t;
                        (
                            u * u * p0.0 + 2.0 * u * t * pts[0].0 + t * t * pts[1].0,
                            u * u * p0.1 + 2.0 * u * t * pts[0].1 + t * t * pts[1].1,
                        )
                    };
                    len += dist(prev, q);
                    prev = q;
                }
                (x, y) = pts[k - 1];
                i += 2 * k;
            }
            'A' => {
                let p = (bx + n(&toks, i + 5), by + n(&toks, i + 6));
                len += dist((x, y), p);
                (x, y) = p;
                i += 7;
            }
            _ => i += 1,
        }
    }
    len
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn write_node(n: roxmltree::Node, rules: &[Rule], vars: &HashMap<String, String>, out: &mut String, families: &mut Vec<String>) {
    if n.is_text() {
        out.push_str(&escape(n.text().unwrap_or("")));
        return;
    }
    if !n.is_element() {
        return;
    }
    let name = n.tag_name().name();
    if name == "style" || name == "script" {
        return;
    }
    // The cascade: (important, specificity, order) decides; inline style
    // beats rules of the same importance.
    let mut winners: HashMap<String, ((bool, (u32, u32, u32), usize), String)> = HashMap::new();
    let mut consider = |k: &str, v: &str, key: (bool, (u32, u32, u32), usize)| {
        if k.starts_with("--") {
            return;
        }
        match winners.get(k) {
            Some((old, _)) if *old > key => {}
            _ => {
                winners.insert(k.to_string(), (key, v.to_string()));
            }
        }
    };
    for r in rules {
        let Some(spec) = r.selectors.iter().filter(|s| matches(n, s)).map(|s| s.spec).max() else { continue };
        for (k, v, imp) in &r.decls {
            consider(k, v, (*imp, spec, r.order));
        }
    }
    if let Some(style) = n.attribute("style") {
        for (k, v, imp) in parse_decls(style) {
            consider(&k, &v, (imp, (1000, 0, 0), usize::MAX));
        }
    }
    let mut attrs: Vec<(String, String)> = Vec::new();
    for a in n.attributes() {
        if a.name() == "style" {
            continue;
        }
        let key = match a.namespace() {
            Some("http://www.w3.org/1999/xlink") => format!("xlink:{}", a.name()),
            Some("http://www.w3.org/XML/1998/namespace") => format!("xml:{}", a.name()),
            _ => a.name().to_string(),
        };
        attrs.push((key, resolve_vars(a.value(), vars, 0)));
    }
    let set = |attrs: &mut Vec<(String, String)>, k: &str, v: String| {
        attrs.retain(|(a, _)| a != k);
        attrs.push((k.to_string(), v));
    };
    let get = |k: &str| winners.get(k).map(|(_, v)| resolve_vars(v, vars, 0));
    for p in PRESENTATION {
        if let Some(v) = get(p) {
            let v = v.trim().to_string();
            if *p == "rx" || *p == "ry" || *p == "width" || *p == "height" {
                set(&mut attrs, p, v.trim_end_matches("px").to_string());
            } else {
                set(&mut attrs, p, v);
            }
        }
    }
    if let Some(d) = get("d") {
        if let Some(inner) = d.trim().strip_prefix("path(").and_then(|x| x.strip_suffix(')')) {
            set(&mut attrs, "d", inner.trim().trim_matches(|c| c == '"' || c == '\'').to_string());
        }
    }
    // Individual transform properties, around transform-origin, before the
    // element's own transform.
    let translate = get("translate");
    let rotate = get("rotate");
    let scale = get("scale");
    if translate.is_some() || rotate.is_some() || scale.is_some() {
        let origin = get("transform-origin").unwrap_or_default();
        let o: Vec<f64> = origin.split_whitespace().map(num).collect();
        let (ox, oy) = (o.first().copied().unwrap_or(0.0), o.get(1).copied().unwrap_or(0.0));
        let mut t = format!("translate({} {})", ox, oy);
        if let Some(tr) = translate.filter(|v| v.trim() != "none") {
            let v: Vec<f64> = tr.split_whitespace().map(num).collect();
            t.push_str(&format!(" translate({} {})", v.first().copied().unwrap_or(0.0), v.get(1).copied().unwrap_or(0.0)));
        }
        if let Some(r) = rotate.filter(|v| v.trim() != "none") {
            t.push_str(&format!(" rotate({})", num(&r)));
        }
        if let Some(sc) = scale.filter(|v| v.trim() != "none") {
            let v: Vec<f64> = sc.split_whitespace().map(num).collect();
            let sx = v.first().copied().unwrap_or(1.0);
            t.push_str(&format!(" scale({} {})", sx, v.get(1).copied().unwrap_or(sx)));
        }
        t.push_str(&format!(" translate({} {})", -ox, -oy));
        let own = attrs.iter().find(|(k, _)| k == "transform").map(|(_, v)| v.clone());
        if let Some(own) = own {
            t.push(' ');
            t.push_str(&own);
        }
        set(&mut attrs, "transform", t);
    }
    // `pathLength="1"` (a drawn line's mask): dashes are fractions of the
    // path. Rasterisers measure in user units, so scale them to the real
    // length.
    if let Some(pl) = attrs.iter().find(|(k, _)| k == "pathLength").map(|(_, v)| num(v)) {
        let d = attrs.iter().find(|(k, _)| k == "d").map(|(_, v)| v.clone()).unwrap_or_default();
        let len = path_length(&d);
        if pl > 0.0 && len > 0.0 {
            let f = len / pl;
            for key in ["stroke-dasharray", "stroke-dashoffset"] {
                if let Some(v) = attrs.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone()) {
                    let scaled: Vec<String> = v
                        .split(|c: char| c == ',' || c.is_whitespace())
                        .filter(|x| !x.is_empty())
                        .map(|x| format!("{}", num(x) * f))
                        .collect();
                    set(&mut attrs, key, scaled.join(" "));
                }
            }
        }
        attrs.retain(|(k, _)| k != "pathLength");
    }
    if let Some((_, f)) = attrs.iter().find(|(k, _)| k == "font-family") {
        if let Some(first) = f.split(',').next() {
            families.push(first.trim().trim_matches(|c| c == '"' || c == '\'').to_string());
        }
    }
    out.push('<');
    out.push_str(name);
    if n.parent_element().is_none() {
        attrs.retain(|(k, _)| !k.starts_with("xmlns"));
        out.push_str(r#" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink""#);
    }
    for (k, v) in &attrs {
        out.push_str(&format!(r#" {}="{}""#, k, escape(v)));
    }
    out.push('>');
    for c in n.children() {
        write_node(c, rules, vars, out, families);
    }
    out.push_str(&format!("</{}>", name));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_lengths() {
        assert!((path_length("M0 0 L3 4") - 5.0).abs() < 1e-9);
        assert!((path_length("M0 0 h10 v10 Z") - (20.0 + 200f64.sqrt())).abs() < 1e-9);
        assert!((path_length("M0 0 C0 0 10 0 10 0") - 10.0).abs() < 0.01);
    }

    #[test]
    fn vars_resolve_with_fallbacks_and_nesting() {
        let mut v = HashMap::new();
        v.insert("--a".to_string(), "var(--b)".to_string());
        v.insert("--b".to_string(), "#123456".to_string());
        assert_eq!(resolve_vars("var(--a)", &v, 0), "#123456");
        assert_eq!(resolve_vars("var(--missing, red)", &v, 0), "red");
    }

    #[test]
    fn rules_become_attributes_and_transforms() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><style>
            :root { --c: #ff0000; }
            .x { fill: var(--c); translate: 5px 0px; transform-origin: 1px 1px; }
            .x > text { fill: blue !important; }
        </style><g class="x"><rect width="2" height="2"/><text fill="green">t</text></g></svg>"#;
        let f = flatten(svg).unwrap();
        assert!(f.svg.contains(r##"fill="#ff0000""##), "{}", f.svg);
        assert!(f.svg.contains("translate(5 0)"), "{}", f.svg);
        assert!(f.svg.contains(r#"<text fill="blue""#), "{}", f.svg);
    }
}
