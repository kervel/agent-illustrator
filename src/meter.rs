//! `meter`: a bar of segments that fills up ("cost going up", "load").
//!
//! ```text
//! meter cost [segments: 10, value: 2]          // s1..s10, the first 2 lit
//! keyframe "more" { set cost 6 }               // light the first 6
//! ```
//!
//! Like `table`, it becomes a generated template of ordinary parts: segments
//! `s1..sN` (from 1) in a row (or a column with `grow: up`), and one
//! state per level, `level0..levelN`, so `set cost 6` (or `set cost level6`)
//! animates with the usual state machinery and shows in `--states`.

use crate::parser::ast::{Spanned, Statement, StyleValue, TemplateInstance};

const KEYS: &[&str] = &["segments", "value", "segment_width", "segment_height", "gap", "on", "off", "grow"];

fn arg<'a>(t: &'a TemplateInstance, key: &str) -> Option<&'a StyleValue> {
    t.arguments.iter().find(|(k, _)| k.node.0 == key).map(|(_, v)| &v.node)
}

fn num(v: Option<&StyleValue>) -> Option<f64> {
    match v {
        Some(StyleValue::Number { value, .. }) => Some(*value),
        _ => None,
    }
}

fn word(v: Option<&StyleValue>) -> Option<String> {
    match v {
        Some(StyleValue::String(s)) | Some(StyleValue::Keyword(s)) => Some(s.clone()),
        Some(StyleValue::Identifier(i)) => Some(i.0.clone()),
        Some(StyleValue::Color(c)) => Some(match c {
            crate::parser::ast::ColorValue::Hex(h) => h.clone(),
            crate::parser::ast::ColorValue::Named(n) | crate::parser::ast::ColorValue::PaletteToken(n) => n.clone(),
            other => other.token_string().unwrap_or_default(),
        }),
        _ => None,
    }
}

struct Spec {
    n: usize,
    value: usize,
    w: f64,
    h: f64,
    gap: f64,
    on: String,
    off: String,
    up: bool,
}

fn spec_for(t: &TemplateInstance) -> Result<Spec, String> {
    let name = &t.instance_name.node.0;
    let n = num(arg(t, "segments")).unwrap_or(10.0);
    if n < 1.0 || n.fract() != 0.0 || n > 100.0 {
        return Err(format!("meter {}: segments: a whole number from 1 to 100", name));
    }
    let n = n as usize;
    let value = num(arg(t, "value")).unwrap_or(0.0);
    if value < 0.0 || value.fract() != 0.0 || value as usize > n {
        return Err(format!("meter {}: value: a whole number from 0 to {} (its segments)", name, n));
    }
    let up = match word(arg(t, "grow")).as_deref() {
        None | Some("right") => false,
        Some("up") => true,
        Some(other) => return Err(format!("meter {}: grow: {}: use right or up", name, other)),
    };
    let (dw, dh) = if up { (34.0, 14.0) } else { (14.0, 34.0) };
    Ok(Spec {
        n,
        value: value as usize,
        w: num(arg(t, "segment_width")).unwrap_or(dw),
        h: num(arg(t, "segment_height")).unwrap_or(dh),
        gap: num(arg(t, "gap")).unwrap_or(4.0),
        on: word(arg(t, "on")).unwrap_or_else(|| "role-primary".into()),
        off: word(arg(t, "off")).unwrap_or_else(|| "role-surface-2".into()),
        up,
    })
}

fn template_source(name: &str, s: &Spec) -> String {
    let mut src = format!("template \"{}\" () {{\n", name);
    src.push_str(&format!("    {} segs [gap: {}] {{\n", if s.up { "col" } else { "row" }, s.gap));
    // Up: the first segment is at the bottom.
    let order: Vec<usize> = if s.up { (1..=s.n).rev().collect() } else { (1..=s.n).collect() };
    for i in order {
        let fill = if i <= s.value { &s.on } else { &s.off };
        src.push_str(&format!(
            "        rect s{i} [width: {}, height: {}, fill: {fill}, stroke: none, corner_radius: 3]\n",
            s.w, s.h
        ));
    }
    src.push_str("    }\n");
    for level in 0..=s.n {
        src.push_str(&format!("    state level{level} {{"));
        for i in 1..=s.n {
            let fill = if i <= level { &s.on } else { &s.off };
            src.push_str(&format!(" transform s{i} [fill: {fill}];"));
        }
        src.push_str(" }\n");
    }
    src.push_str("}\n");
    src
}

/// Replace every `meter` instance by an instance of its generated template.
pub fn expand_meters(stmts: Vec<Spanned<Statement>>) -> Result<Vec<Spanned<Statement>>, (std::ops::Range<usize>, String)> {
    if stmts.iter().any(|s| matches!(&s.node, Statement::TemplateDecl(d) if d.name.node.0 == "meter")) {
        return Ok(stmts);
    }
    let mut generated = Vec::new();
    let mut counter = 0usize;
    let out = walk(stmts, &mut generated, &mut counter)?;
    let mut all = generated;
    all.extend(out);
    Ok(all)
}

fn walk(
    stmts: Vec<Spanned<Statement>>,
    generated: &mut Vec<Spanned<Statement>>,
    counter: &mut usize,
) -> Result<Vec<Spanned<Statement>>, (std::ops::Range<usize>, String)> {
    let mut out = Vec::with_capacity(stmts.len());
    for mut st in stmts {
        match &mut st.node {
            Statement::TemplateInstance(t) if t.template_name.node.0 == "meter" => {
                let spec = spec_for(t).map_err(|m| (st.span.clone(), m))?;
                *counter += 1;
                let name = format!("__meter_{}_{}", t.instance_name.node.0, counter);
                let doc = crate::parser::parse(&template_source(&name, &spec))
                    .map_err(|e| (st.span.clone(), format!("meter {}: {:?}", t.instance_name.node.0, e)))?;
                generated.extend(doc.statements);
                t.template_name.node = crate::parser::ast::Identifier::new(name.as_str());
                t.arguments.retain(|(k, _)| !KEYS.contains(&k.node.0.as_str()));
            }
            Statement::Layout(l) => l.children = walk(std::mem::take(&mut l.children), generated, counter)?,
            Statement::Group(g) => g.children = walk(std::mem::take(&mut g.children), generated, counter)?,
            Statement::TemplateDecl(d) => {
                if let Some(body) = d.body.take() {
                    d.body = Some(walk(body, generated, counter)?);
                }
            }
            _ => {}
        }
        out.push(st);
    }
    Ok(out)
}
