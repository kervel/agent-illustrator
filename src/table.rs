//! `table`: a small datagrid as one element.
//!
//! ```text
//! table orders [columns: ["#", "customer", "status"],
//!               rows: [["41", "Stroopwafels BV", "shipped"], ["42", "Acme Bikes", "open"]],
//!               widths: [60, 210, 114], font_size: 18, mono: [0]]
//! ```
//!
//! Like `code`, it becomes a generated template of ordinary parts, so motion,
//! lint and themes treat it like anything else: `bg` (the frame), `head` (the
//! header band) with header cells `h0..`, per row `row1..` (a band to
//! highlight or recolour: `highlight orders.row[2]`) with cells `r1c0..`, and
//! the rules between them. Cells are square, flush and keep their text as
//! written; colours are theme roles.

use crate::parser::ast::{Spanned, Statement, StyleValue, TemplateInstance};

const KEYS: &[&str] = &["columns", "rows", "widths", "font_size", "row_height", "mono", "keys", "status"];

/// The colour a status mark stands for: ✓ ok, ✕ error, ! warn (by the
/// first character, so "✓ delivered" counts too).
pub fn status_colour(text: &str) -> Option<&'static str> {
    match text.trim_start().chars().next()? {
        '✓' | '✔' => Some("role-ok"),
        '✕' | '✗' | '✘' | '✖' | '×' => Some("role-error"),
        '!' | '⚠' => Some("role-warn"),
        _ => None,
    }
}

/// The class a status cell carries, so a later `transform` of its label
/// recolours it too.
pub const STATUS_CLASS: &str = "ail_status";

fn arg<'a>(t: &'a TemplateInstance, key: &str) -> Option<&'a StyleValue> {
    t.arguments.iter().find(|(k, _)| k.node.0 == key).map(|(_, v)| &v.node)
}

fn text(v: &StyleValue) -> String {
    match v {
        StyleValue::String(s) | StyleValue::Keyword(s) => s.clone(),
        StyleValue::Identifier(i) => i.0.clone(),
        StyleValue::Number { value, .. } => format!("{}", value),
        _ => String::new(),
    }
}

fn list(v: Option<&StyleValue>) -> Vec<StyleValue> {
    match v {
        Some(StyleValue::List(items)) => items.iter().map(|x| x.node.clone()).collect(),
        _ => Vec::new(),
    }
}

fn num(v: Option<&StyleValue>) -> Option<f64> {
    match v {
        Some(StyleValue::Number { value, .. }) => Some(*value),
        _ => None,
    }
}

fn lit(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"))
}

struct Spec {
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
    widths: Vec<f64>,
    font_size: f64,
    row_height: f64,
    mono: Vec<usize>,
    /// Status columns (from 0).
    status: Vec<usize>,
}

fn spec_for(t: &TemplateInstance) -> Result<Spec, String> {
    let name = &t.instance_name.node.0;
    let columns: Vec<String> = list(arg(t, "columns")).iter().map(text).collect();
    if columns.is_empty() {
        return Err(format!("table {}: needs `columns: [\"#\", \"customer\", ...]`", name));
    }
    let keys: Vec<String> = list(arg(t, "keys")).iter().map(text).collect();
    let mut rows = Vec::new();
    for (i, r) in list(arg(t, "rows")).iter().enumerate() {
        let cells: Vec<String> = match r {
            StyleValue::List(items) => items.iter().map(|x| text(&x.node)).collect(),
            // A record row names its cells by `keys:` (one per column).
            StyleValue::Record(fields) => {
                if keys.len() != columns.len() {
                    return Err(format!(
                        "table {}: row {} is a record; say which field goes in which column with `keys: [...]` ({} names)",
                        name,
                        i + 1,
                        columns.len()
                    ));
                }
                keys.iter()
                    .map(|k| fields.iter().find(|(f, _)| f.node.0 == *k).map(|(_, v)| text(&v.node)).unwrap_or_default())
                    .collect()
            }
            other => vec![text(other)],
        };
        if cells.len() != columns.len() {
            return Err(format!(
                "table {}: row {} has {} cells for {} columns",
                name,
                i + 1,
                cells.len(),
                columns.len()
            ));
        }
        rows.push(cells);
    }
    let font_size = num(arg(t, "font_size")).unwrap_or(17.0);
    let row_height = num(arg(t, "row_height")).unwrap_or((font_size * 2.4).round());
    let mono: Vec<usize> = list(arg(t, "mono")).iter().filter_map(|v| num(Some(v)).map(|n| n as usize)).collect();
    // `status: [3]` or `status: 3`: columns counted from 1, as `cell[r][c]`.
    let status_given: Vec<f64> = match arg(t, "status") {
        Some(StyleValue::List(_)) => list(arg(t, "status")).iter().filter_map(|v| num(Some(v))).collect(),
        other => num(other).into_iter().collect(),
    };
    let mut status = Vec::new();
    for c in status_given {
        if c < 1.0 || c.fract() != 0.0 || c as usize > columns.len() {
            return Err(format!("table {}: status: {}: a column from 1 to {}", name, c, columns.len()));
        }
        status.push(c as usize - 1);
    }
    let given: Vec<f64> = list(arg(t, "widths")).iter().filter_map(|v| num(Some(v))).collect();
    let widths = (0..columns.len())
        .map(|j| {
            given.get(j).copied().unwrap_or_else(|| {
                // Wide enough for the longest cell of the column.
                let chars = std::iter::once(&columns[j]).chain(rows.iter().map(|r| &r[j])).map(|s| s.chars().count()).max().unwrap_or(1);
                (chars as f64 * 0.6 * font_size + 28.0).ceil()
            })
        })
        .collect();
    Ok(Spec { columns, rows, widths, font_size, row_height, mono, status })
}

fn template_source(name: &str, s: &Spec) -> String {
    let w: f64 = s.widths.iter().sum();
    let rh = s.row_height;
    let h = rh * (s.rows.len() + 1) as f64;
    let fs = s.font_size;
    let mut src = format!("template {} () {{\n", lit(name));
    src.push_str(&format!(
        "    rect bg [width: {w}, height: {h}, fill: role-surface, stroke: role-rule, stroke_width: 2, corner_radius: 0]\n"
    ));
    src.push_str(&format!(
        "    rect head [width: {w}, height: {rh}, fill: role-surface-2, stroke: none, corner_radius: 0, clip: bg]\n"
    ));
    src.push_str("    constrain head.left = bg.left\n    constrain head.top = bg.top\n");
    let font = |j: usize| if s.mono.contains(&j) { ", font_family: mono" } else { "" };
    for (j, c) in s.columns.iter().enumerate() {
        src.push_str(&format!(
            "    rect h{j} [width: {}, height: {rh}, fill: none, stroke: none, corner_radius: 0, label: {}, align: start, \
             font_size: {fs}, font_weight: 700, label_fill: role-ink{}]\n",
            s.widths[j],
            lit(c),
            font(j)
        ));
        src.push_str(&format!("    constrain h{j}.top = bg.top\n"));
        if j == 0 {
            src.push_str("    constrain h0.left = bg.left\n");
        } else {
            src.push_str(&format!("    constrain h{j}.left = h{}.right\n", j - 1));
        }
    }
    for (i, row) in s.rows.iter().enumerate() {
        let r = i + 1;
        src.push_str(&format!(
            "    rect row{r} [width: {w}, height: {rh}, fill: none, stroke: none, corner_radius: 0]\n\
             \x20   constrain row{r}.left = bg.left\n    constrain row{r}.top = bg.top + {}\n",
            rh * r as f64
        ));
        src.push_str(&format!(
            "    rect rule{r} [width: {w}, height: 1, fill: role-rule, stroke: none, corner_radius: 0]\n\
             \x20   constrain rule{r}.left = bg.left\n    constrain rule{r}.top = row{r}.top\n"
        ));
        for (j, cell) in row.iter().enumerate() {
            let (ink, extra) = if s.status.contains(&j) {
                (status_colour(cell).unwrap_or("role-ink"), format!(", font_weight: 700, class: {}", STATUS_CLASS))
            } else {
                ("role-ink", String::new())
            };
            src.push_str(&format!(
                "    rect r{r}c{j} [width: {}, height: {rh}, fill: none, stroke: none, corner_radius: 0, label: {}, \
                 align: start, font_size: {fs}, label_fill: {ink}{}{extra}]\n    constrain r{r}c{j}.top = row{r}.top\n",
                s.widths[j],
                lit(cell),
                font(j)
            ));
            if j == 0 {
                src.push_str(&format!("    constrain r{r}c0.left = bg.left\n"));
            } else {
                src.push_str(&format!("    constrain r{r}c{j}.left = r{r}c{}.right\n", j - 1));
            }
        }
    }
    // Column rules, under the text.
    let mut x = 0.0;
    for j in 1..s.columns.len() {
        x += s.widths[j - 1];
        src.push_str(&format!(
            "    rect col{j} [width: 1, height: {h}, fill: role-rule, stroke: none, corner_radius: 0]\n\
             \x20   constrain col{j}.left = bg.left + {x}\n    constrain col{j}.top = bg.top\n"
        ));
    }
    src.push_str("}\n");
    src
}

/// Replace every `table` instance by an instance of its generated template.
pub fn expand_tables(stmts: Vec<Spanned<Statement>>) -> Result<Vec<Spanned<Statement>>, (std::ops::Range<usize>, String)> {
    if stmts.iter().any(|s| matches!(&s.node, Statement::TemplateDecl(d) if d.name.node.0 == "table")) {
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
            Statement::TemplateInstance(t) if t.template_name.node.0 == "table" => {
                let spec = spec_for(t).map_err(|m| (st.span.clone(), m))?;
                *counter += 1;
                let name = format!("__table_{}_{}", t.instance_name.node.0, counter);
                let doc = crate::parser::parse(&template_source(&name, &spec))
                    .map_err(|e| (st.span.clone(), format!("table {}: {:?}", t.instance_name.node.0, e)))?;
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
