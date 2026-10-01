//! `caption: "..."` on any element: a caption part, `x.caption`.
//!
//! ```text
//! agent_art r1 [width: 60, height: 60, caption: "NVIDIA", caption_position: below]
//! ```
//!
//! becomes the long form agents used to write out every time:
//!
//! ```text
//! rect r1_caption [caption_of: r1, label_position: below, label_offset: 8, label: "NVIDIA",
//!                  font_size: 18, font_weight: 600, label_fill: role-muted, fill: none, stroke: none]
//! ```
//!
//! The caption is declared at the root of the scope the element is in (a
//! caption inside a `row` would take a slot in it), copies the element's
//! `appears`, and `show x` / `hide x` bring it along (see motion expand).

use crate::parser::ast::{Spanned, Statement, StyleKey, StyleModifier, StyleValue};

const POSITIONS: &[&str] = &["above", "below", "left", "right"];

fn lit(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"))
}

fn text(v: &StyleValue) -> Option<String> {
    match v {
        StyleValue::String(s) => Some(s.clone()),
        StyleValue::Keyword(s) => Some(s.clone()),
        StyleValue::Identifier(i) => Some(i.0.clone()),
        _ => None,
    }
}

type Err = (std::ops::Range<usize>, String);

/// What a `caption:` asks for: (subject, text, position, appears).
struct Want {
    subject: String,
    text: String,
    position: String,
    appears: Option<String>,
    span: std::ops::Range<usize>,
}

fn from_modifiers(name: &str, mods: &mut Vec<Spanned<StyleModifier>>, span: &std::ops::Range<usize>) -> Result<Option<Want>, Err> {
    let key = |m: &Spanned<StyleModifier>, k: &str| matches!(&m.node.key.node, StyleKey::Custom(c) if c == k);
    let Some(i) = mods.iter().position(|m| key(m, "caption")) else {
        if let Some(m) = mods.iter().find(|m| key(m, "caption_position")) {
            return Err((m.span.clone(), format!("{}: `caption_position` without a `caption: \"...\"`", name)));
        }
        return Ok(None);
    };
    let m = mods.remove(i);
    let words = text(&m.node.value.node).ok_or_else(|| (m.span.clone(), format!("{}: caption: give the words in quotes", name)))?;
    let position = match mods.iter().position(|m| key(m, "caption_position")) {
        Some(j) => {
            let p = mods.remove(j);
            let v = text(&p.node.value.node).unwrap_or_default();
            if !POSITIONS.contains(&v.as_str()) {
                return Err((p.span.clone(), format!("{}: caption_position: {}: use above, below, left or right", name, v)));
            }
            v
        }
        None => "below".to_string(),
    };
    let appears = mods.iter().find(|m| key(m, "appears")).and_then(|m| text(&m.node.value.node));
    Ok(Some(Want { subject: name.to_string(), text: words, position, appears, span: span.clone() }))
}

fn from_arguments(name: &str, args: &mut Vec<(Spanned<crate::parser::ast::Identifier>, Spanned<StyleValue>)>, span: &std::ops::Range<usize>) -> Result<Option<Want>, Err> {
    let Some(i) = args.iter().position(|(k, _)| k.node.0 == "caption") else {
        return Ok(None);
    };
    let (_, v) = args.remove(i);
    let words = text(&v.node).ok_or_else(|| (v.span.clone(), format!("{}: caption: give the words in quotes", name)))?;
    let position = match args.iter().position(|(k, _)| k.node.0 == "caption_position") {
        Some(j) => {
            let (_, p) = args.remove(j);
            let v = text(&p.node).unwrap_or_default();
            if !POSITIONS.contains(&v.as_str()) {
                return Err((p.span.clone(), format!("{}: caption_position: {}: use above, below, left or right", name, v)));
            }
            v
        }
        None => "below".to_string(),
    };
    let appears = args.iter().find(|(k, _)| k.node.0 == "appears").and_then(|(_, v)| text(&v.node));
    Ok(Some(Want { subject: name.to_string(), text: words, position, appears, span: span.clone() }))
}

fn source(w: &Want) -> String {
    format!(
        "rect {s}_caption [caption_of: {s}, label_position: {p}, label_offset: 8, label: {t}, font_size: 18, \
         font_weight: 600, label_fill: role-muted, fill: none, stroke: none{a}]\n",
        s = w.subject,
        p = w.position,
        t = lit(&w.text),
        a = w.appears.as_ref().map(|a| format!(", appears: {}", a)).unwrap_or_default()
    )
}

/// Collect the captions of one statement list (and the layouts in it).
fn walk(stmts: &mut [Spanned<Statement>], out: &mut Vec<Want>) -> Result<(), Err> {
    for st in stmts.iter_mut() {
        let span = st.span.clone();
        match &mut st.node {
            Statement::Shape(sh) => {
                if let Some(name) = sh.name.as_ref().map(|n| n.node.0.clone()) {
                    out.extend(from_modifiers(&name, &mut sh.modifiers, &span)?);
                }
            }
            Statement::TemplateInstance(t) => {
                let name = t.instance_name.node.0.clone();
                out.extend(from_arguments(&name, &mut t.arguments, &span)?);
            }
            Statement::Group(g) => {
                if let Some(name) = g.name.as_ref().map(|n| n.node.0.clone()) {
                    out.extend(from_modifiers(&name, &mut g.modifiers, &span)?);
                }
                walk(&mut g.children, out)?;
            }
            Statement::Layout(l) => {
                if let Some(name) = l.name.as_ref().map(|n| n.node.0.clone()) {
                    out.extend(from_modifiers(&name, &mut l.modifiers, &span)?);
                }
                walk(&mut l.children, out)?;
            }
            // A template body is a scope of its own.
            Statement::TemplateDecl(d) => {
                if let Some(body) = d.body.take() {
                    d.body = Some(expand_scope(body)?);
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn expand_scope(mut stmts: Vec<Spanned<Statement>>) -> Result<Vec<Spanned<Statement>>, Err> {
    let mut wants = Vec::new();
    walk(&mut stmts, &mut wants)?;
    for w in wants {
        let doc = crate::parser::parse(&source(&w)).map_err(|e| (w.span.clone(), format!("caption of {}: {:?}", w.subject, e)))?;
        stmts.extend(doc.statements.into_iter().map(|mut s| {
            s.span = w.span.clone();
            s
        }));
    }
    Ok(stmts)
}

/// Turn every `caption: "..."` into its caption part.
pub fn expand_captions(stmts: Vec<Spanned<Statement>>) -> Result<Vec<Spanned<Statement>>, Err> {
    expand_scope(stmts)
}
