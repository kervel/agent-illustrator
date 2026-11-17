//! `--flat-colors`: an SVG with no dependence on CSS custom properties.
//!
//! The renderer writes colours as `var(--role-primary)` so a stylesheet can
//! retheme a diagram, and arrowheads as `context-stroke` so one marker serves
//! every line colour. Hosts that evaluate neither (image viewers, some
//! converters and editors) paint those black. This pass rewrites the finished
//! document:
//!
//! * every `var(--x[, fallback])` in a tag's attributes, in the `<style>`
//!   block and in the motion manifest becomes the value the document's own
//!   root rules give it (later rules win, `@media` blocks are ignored, so the
//!   base/light palette is used). Text content is left alone: a label or code
//!   listing that says `var(--x)` means it.
//! * a marker drawn with `context-stroke` is copied once per line colour that
//!   uses it, with the colour written in, and every line points at its copy.
//!
//! Selectors in the style block are rewritten by the same substitution as the
//! attributes, so a host rule like `[fill="var(--accent-1)"] + .ai-label`
//! still matches the shape it was written for.

use std::collections::HashMap;

/// Rewrite `svg` so it uses literal colours only (see the module docs).
pub fn flatten_colors(svg: &str) -> String {
    let vars: HashMap<String, String> = crate::raster::root_vars(svg)
        .unwrap_or_default()
        .into_iter()
        // A value lands inside a `"`-quoted attribute or a JSON string; CSS
        // takes either quote.
        .map(|(k, v)| (k, v.replace('"', "'")))
        .collect();
    let resolved = resolve_outside_text(svg, &vars);
    flatten_markers(&resolved)
}

fn resolve(s: &str, vars: &HashMap<String, String>) -> String {
    if s.contains("var(") {
        crate::raster::resolve_vars(s, vars, 0)
    } else {
        s.to_string()
    }
}

/// Substitute `var()` in tags and in `<style>`/`<script>` content, not in
/// text nodes.
fn resolve_outside_text(svg: &str, vars: &HashMap<String, String>) -> String {
    let mut out = String::with_capacity(svg.len());
    let mut raw = false; // inside <style> or <script>
    let mut i = 0;
    while i < svg.len() {
        let Some(lt) = svg[i..].find('<').map(|p| p + i) else {
            let rest = &svg[i..];
            out.push_str(&if raw { resolve(rest, vars) } else { rest.to_string() });
            break;
        };
        let text = &svg[i..lt];
        out.push_str(&if raw { resolve(text, vars) } else { text.to_string() });
        let rest = &svg[lt..];
        let (end, is_tag) = if rest.starts_with("<![CDATA[") {
            (rest.find("]]>").map(|p| p + 3).unwrap_or(rest.len()), false)
        } else if rest.starts_with("<!--") {
            (rest.find("-->").map(|p| p + 3).unwrap_or(rest.len()), false)
        } else {
            (tag_end(rest), true)
        };
        let chunk = &rest[..end];
        if is_tag {
            let name: String = chunk[1..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '/' || *c == '-')
                .collect();
            match name.as_str() {
                "style" | "script" if !chunk.ends_with("/>") => raw = true,
                "/style" | "/script" => raw = false,
                _ => {}
            }
            out.push_str(&resolve(chunk, vars));
        } else if raw && chunk.starts_with("<![CDATA[") {
            out.push_str(&resolve(chunk, vars));
        } else {
            out.push_str(chunk);
        }
        i = lt + end;
    }
    out
}

/// Byte length of the tag at the start of `s` (through its `>`), skipping
/// any `>` inside a quoted attribute value.
fn tag_end(s: &str) -> usize {
    let mut quote: Option<char> = None;
    for (k, c) in s.char_indices() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (None, '"') | (None, '\'') => quote = Some(c),
            (None, '>') => return k + 1,
            _ => {}
        }
    }
    s.len()
}

fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let key = format!(" {}=\"", name);
    let start = tag.find(&key)? + key.len();
    let end = tag[start..].find('"')? + start;
    Some(&tag[start..end])
}

/// A short, id-safe spelling of a colour (`#2196f3` -> `2196f3`).
fn slug(colour: &str) -> String {
    let s: String = colour.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    if s.is_empty() {
        "c".to_string()
    } else {
        s
    }
}

/// Give every `context-stroke` marker one copy per line colour that uses it.
fn flatten_markers(svg: &str) -> String {
    // The marker defs that paint with the line's colour: id -> (span, text).
    let mut defs: Vec<(String, std::ops::Range<usize>)> = Vec::new();
    let mut from = 0;
    while let Some(p) = svg[from..].find("<marker ").map(|p| p + from) {
        let Some(close) = svg[p..].find("</marker>").map(|q| p + q + "</marker>".len()) else { break };
        let def = &svg[p..close];
        if def.contains("context-stroke") || def.contains("context-fill") {
            if let Some(id) = attr(&svg[p..p + tag_end(&svg[p..])], "id") {
                defs.push((id.to_string(), p..close));
            }
        }
        from = close;
    }
    if defs.is_empty() {
        return svg.to_string();
    }
    let ids: Vec<&str> = defs.iter().map(|(id, _)| id.as_str()).collect();

    // Point each line at a copy for its colour; remember which copies.
    let mut copies: Vec<(String, String)> = Vec::new(); // (marker id, colour)
    let mut out = String::with_capacity(svg.len());
    let mut i = 0;
    while let Some(lt) = svg[i..].find('<').map(|p| p + i) {
        out.push_str(&svg[i..lt]);
        let end = lt + tag_end(&svg[lt..]);
        let mut tag = svg[lt..end].to_string();
        if tag.contains(" marker-start=\"") || tag.contains(" marker-end=\"") || tag.contains(" marker-mid=\"") {
            let colour = attr(&tag, "stroke").filter(|s| !s.is_empty() && *s != "none").unwrap_or("#333").to_string();
            for which in ["marker-start", "marker-mid", "marker-end"] {
                let Some(v) = attr(&tag, which).map(str::to_string) else { continue };
                let Some(id) = v.strip_prefix("url(#").and_then(|r| r.strip_suffix(')')) else { continue };
                if !ids.contains(&id) {
                    continue;
                }
                let copy = format!("{}-{}", id, slug(&colour));
                if !copies.iter().any(|(c, _)| *c == copy) {
                    copies.push((copy.clone(), colour.clone()));
                }
                tag = tag.replacen(&format!(" {}=\"{}\"", which, v), &format!(" {}=\"url(#{})\"", which, copy), 1);
            }
        }
        out.push_str(&tag);
        i = end;
    }
    out.push_str(&svg[i..]);

    // Replace each original def by its coloured copies (in first-use order).
    for (id, _) in &defs {
        let Some(start) = out.find(&format!("<marker id=\"{}\"", id)) else { continue };
        let Some(close) = out[start..].find("</marker>").map(|q| start + q + "</marker>".len()) else { continue };
        let def = out[start..close].to_string();
        let coloured: Vec<String> = copies
            .iter()
            .filter(|(copy, colour)| *copy == format!("{}-{}", id, slug(colour)))
            .map(|(copy, colour)| {
                def.replacen(&format!("id=\"{}\"", id), &format!("id=\"{}\"", copy), 1)
                    .replace("context-stroke", colour)
                    .replace("context-fill", colour)
            })
            .collect();
        out.replace_range(start..close, &coloured.join("\n    "));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_content_keeps_its_words() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><style><![CDATA[:root { --a: #123456; } .x { fill: var(--a); }]]></style><text fill="var(--a)">var(--a)</text></svg>"#;
        let flat = flatten_colors(svg);
        assert!(flat.contains(r##"<text fill="#123456">var(--a)</text>"##), "{flat}");
        assert!(flat.contains(".x { fill: #123456; }"), "{flat}");
    }

    #[test]
    fn markers_get_one_copy_per_line_colour() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><defs><marker id="ai-arrow" orient="auto"><path fill="context-stroke"/></marker></defs><path d="M0 0" stroke="#ff0000" marker-end="url(#ai-arrow)"/><path d="M0 0" stroke="#00ff00" marker-start="url(#ai-arrow)" marker-end="url(#ai-arrow)"/></svg>"##;
        let flat = flatten_colors(svg);
        assert!(!flat.contains("context-stroke"), "{flat}");
        assert!(flat.contains(r##"<marker id="ai-arrow-ff0000" orient="auto"><path fill="#ff0000"/></marker>"##), "{flat}");
        assert!(flat.contains(r##"marker-start="url(#ai-arrow-00ff00)" marker-end="url(#ai-arrow-00ff00)""##), "{flat}");
        assert!(!flat.contains(r#"id="ai-arrow""#), "{flat}");
    }

    #[test]
    fn quoted_values_stay_inside_their_attribute() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><style>:root { --f: "Fira Code", monospace; }</style><text style="font-family:var(--f)">x</text></svg>"#;
        let flat = flatten_colors(svg);
        assert!(flat.contains(r#"style="font-family:'Fira Code', monospace""#), "{flat}");
    }
}
