//! `code` blocks: source code (or a diff) as a first-class element.
//!
//! `code c [lang: python, source: "..."]` is expanded, before templates are
//! resolved, into an ordinary generated template: a panel, an optional title
//! bar, and one monospace `rect lineN` per line whose label carries the line
//! number and the highlighted text as label markup. Lines are therefore
//! plain parts (`c.line4`): constraints, anchors and every motion verb work
//! on them, and nothing downstream knows code exists.
//!
//! Colours come from theme tokens (`code-keyword`, `code-string`, ...), which
//! default to colour roles, so a deck restyles all code in one place.

use crate::parser::ast::{Spanned, Statement, StyleValue, TemplateInstance};

/// Inset of a label inside its box (layout's LABEL_INSET).
const INSET: f64 = 8.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    Plain,
    Keyword,
    String,
    Comment,
    Number,
    Function,
    Key,
}

impl Kind {
    fn token(self) -> Option<&'static str> {
        match self {
            Kind::Plain => None,
            Kind::Keyword => Some("code-keyword"),
            Kind::String => Some("code-string"),
            Kind::Comment => Some("code-comment"),
            Kind::Number => Some("code-number"),
            Kind::Function => Some("code-function"),
            Kind::Key => Some("code-key"),
        }
    }
}

/// The shared syntax definitions (Sublime syntaxes bundled with syntect).
fn syntaxes() -> &'static syntect::parsing::SyntaxSet {
    static SET: std::sync::OnceLock<syntect::parsing::SyntaxSet> = std::sync::OnceLock::new();
    SET.get_or_init(syntect::parsing::SyntaxSet::load_defaults_newlines)
}

fn syntax_for(lang: &str) -> Option<&'static syntect::parsing::SyntaxReference> {
    let set = syntaxes();
    let lang = lang.trim().to_ascii_lowercase();
    // TypeScript is close enough to JavaScript for colouring a slide.
    let lang = match lang.as_str() {
        "ts" | "typescript" | "tsx" | "jsx" => "js".to_string(),
        "shell" | "bash" | "zsh" => "sh".to_string(),
        "yml" => "yaml".to_string(),
        other => other.to_string(),
    };
    set.find_syntax_by_token(&lang)
        .or_else(|| set.find_syntax_by_extension(&lang))
}

/// What a stack of syntax scopes means for colouring, innermost first.
fn kind_of(stack: &syntect::parsing::ScopeStack) -> Kind {
    let names: Vec<String> = stack.as_slice().iter().map(|s| s.build_string()).collect();
    let has = |p: &str| names.iter().any(|s| s == p || s.starts_with(&format!("{}.", p)));
    if has("comment") {
        return Kind::Comment;
    }
    // A key is a key even though its innermost scope is a string.
    if has("meta.mapping.key") || has("meta.structure.dictionary.key") || has("support.type.property-name")
        || has("entity.name.tag") || has("entity.other.attribute-name")
    {
        return Kind::Key;
    }
    for s in names.iter().rev() {
        let is = |p: &str| s == p || s.starts_with(&format!("{}.", p));
        // A string's quotes, a variable's `$`: coloured as what they belong to.
        if is("punctuation.definition") {
            continue;
        }
        if is("string.unquoted") {
            return Kind::Plain;
        }
        if is("string") {
            return Kind::String;
        }
        if is("constant.numeric") {
            return Kind::Number;
        }
        if is("constant.language") || is("storage") || is("keyword.operator.logical")
            || is("keyword.operator.word") || is("keyword.control") || is("keyword.other")
            || is("keyword.declaration") || is("keyword.import")
        {
            return Kind::Keyword;
        }
        if is("keyword.operator") || is("punctuation") {
            return Kind::Plain;
        }
        if is("keyword") {
            return Kind::Keyword;
        }
        if is("entity.name.function") || is("support.function") || is("variable.function")
            || is("meta.function-call.generic") || is("variable.other.readwrite.shell")
        {
            return Kind::Function;
        }
    }
    Kind::Plain
}

/// Highlighted runs for every line of `src`, keeping the parser's state from
/// line to line (a string or comment may span lines). An unknown language
/// comes back as plain text.
pub fn tokenize_lines(lang: &str, src: &str) -> Vec<Vec<(Kind, String)>> {
    use syntect::parsing::{ParseState, ScopeStack};
    let lines: Vec<&str> = src.split('\n').collect();
    let Some(syntax) = syntax_for(lang) else {
        return lines.iter().map(|l| vec![(Kind::Plain, l.to_string())]).collect();
    };
    let set = syntaxes();
    let mut state = ParseState::new(syntax);
    let mut stack = ScopeStack::new();
    let mut out = Vec::with_capacity(lines.len());
    for line in lines {
        let with_nl = format!("{}\n", line);
        let ops = state.parse_line(&with_nl, set).unwrap_or_default();
        let mut runs: Vec<(Kind, String)> = Vec::new();
        let mut push = |k: Kind, t: &str| {
            let t = t.trim_end_matches('\n');
            if t.is_empty() {
                return;
            }
            // `;`, `[`, `=`: never a keyword or a call, whatever the grammar says.
            let k = if matches!(k, Kind::Keyword | Kind::Function) && !t.chars().any(char::is_alphanumeric) {
                Kind::Plain
            } else {
                k
            };
            match runs.last_mut() {
                Some(last) if last.0 == k => last.1.push_str(t),
                _ => runs.push((k, t.to_string())),
            }
        };
        let mut at = 0;
        for (pos, op) in ops {
            if pos > at {
                push(kind_of(&stack), &with_nl[at..pos]);
                at = pos;
            }
            let _ = stack.apply(&op);
        }
        if at < with_nl.len() {
            push(kind_of(&stack), &with_nl[at..]);
        }
        out.push(runs);
    }
    out
}

/// Highlighted runs of one line.
pub fn tokenize(lang: &str, line: &str) -> Vec<(Kind, String)> {
    tokenize_lines(lang, line).into_iter().next().unwrap_or_default()
}

/// Text for label markup: entities escaped, spaces made non-breaking (a
/// line's indentation is its meaning; SVG would collapse it).
pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace(' ', "\u{a0}")
}

/// One line of source as label markup.
pub fn highlight(lang: &str, line: &str) -> String {
    runs_markup(tokenize(lang, line))
}

fn span(token: &str, s: &str) -> String {
    if s.is_empty() {
        String::new()
    } else {
        format!("<span fill={}>{}</span>", token, s)
    }
}

/// The line-number gutter: right-aligned, muted, a little smaller.
fn gutter(n: Option<usize>, width: usize) -> String {
    let text = n.map(|n| n.to_string()).unwrap_or_default();
    let padded = format!("{:>w$}\u{a0}\u{a0}", text, w = width);
    format!("<small>{}</small>", span("code-ln", &escape(&padded)))
}

/// What the generator was asked for.
#[derive(Debug, Clone, Default)]
pub struct CodeSpec {
    pub lang: String,
    pub source: Option<String>,
    pub diff: Option<String>,
    pub title: Option<String>,
    pub line_numbers: bool,
    pub first: usize,
    pub font_size: f64,
    pub line_height: f64,
    /// Minimum width of the code area, in characters.
    pub min_chars: usize,
    /// `marks: "1:role-series-2-soft, 8:role-series-1-soft"`: tinted lines.
    pub marks: Vec<(usize, usize, String)>,
    /// `frame: false`: no panel around the lines (a diff inside a card).
    pub frame: bool,
}

/// A generated line.
struct Line {
    markup: String,
    chars: usize,
    tint: Option<String>,
    /// Part name (`line4`, or an inserted line's `conflict2`).
    name: String,
    /// Inserted: no room and hidden until its `insert` opens it.
    collapsed: bool,
}

/// Lines an `insert` statement adds to a block (`[name: conflict]` names them
/// `conflict1..N` and gives them class `conflict`).
#[derive(Debug, Clone)]
pub struct Insert {
    pub after: usize,
    pub name: String,
    pub source: String,
    pub tint: Option<String>,
}

/// Digits of the widest line number (for the gutter).
fn gutter_width(spec: &CodeSpec) -> usize {
    let n = spec.source.as_deref().map(|s| s.split('\n').count()).unwrap_or(0);
    (spec.first + n.saturating_sub(1)).to_string().len()
}

/// Chars the gutter takes (it is <small>: 0.8 of a glyph per character).
fn gutter_chars(spec: &CodeSpec) -> usize {
    if spec.line_numbers && spec.diff.is_none() {
        ((gutter_width(spec) + 2) as f64 * 0.8).ceil() as usize
    } else {
        0
    }
}

/// The label for a line of this block: gutter (its number, or blank for an
/// inserted line) and the highlighted text. Also what `transform c.line4
/// [source: "..."]` turns into.
pub fn line_label(spec: &CodeSpec, number: Option<usize>, text: &str) -> String {
    let g = if spec.line_numbers && spec.diff.is_none() {
        gutter(number, gutter_width(spec))
    } else {
        String::new()
    };
    format!("{}{}", g, highlight(&spec.lang, text))
}

/// Common prefix / suffix lengths (in chars) of two lines.
fn common_ends(a: &[char], b: &[char]) -> (usize, usize) {
    let p = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    let max_s = a.len().min(b.len()) - p;
    let s = a.iter().rev().zip(b.iter().rev()).take(max_s).take_while(|(x, y)| x == y).count();
    (p, s)
}

fn diff_lines(spec: &CodeSpec, diff: &str) -> Vec<Line> {
    let rows: Vec<(char, String)> = diff
        .lines()
        .map(|l| {
            let mut cs = l.chars();
            match cs.next() {
                Some(c @ ('-' | '+' | ' ')) => (c, cs.collect()),
                Some(c) => (' ', std::iter::once(c).chain(cs).collect()),
                None => (' ', String::new()),
            }
        })
        .collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < rows.len() {
        let (sign, text) = &rows[i];
        // A removed line followed by an added one: mark what changed.
        let pair = if *sign == '-' {
            rows.get(i + 1).filter(|(s, _)| *s == '+').map(|(_, t)| t.clone())
        } else {
            None
        };
        let render = |sign: char, text: &str, other: Option<&str>| -> Line {
            let (tint, ink, strong) = match sign {
                '-' => (Some("code-del-bg"), "code-del", "code-del-strong"),
                '+' => (Some("code-add-bg"), "code-add", "code-add-strong"),
                _ => (None, "code-plain", "code-plain"),
            };
            let chars: Vec<char> = text.chars().collect();
            let body = match other {
                Some(o) => {
                    let oc: Vec<char> = o.chars().collect();
                    let (p, s) = common_ends(&chars, &oc);
                    let head: String = chars[..p].iter().collect();
                    let mid: String = chars[p..chars.len() - s].iter().collect();
                    let tail: String = chars[chars.len() - s..].iter().collect();
                    format!(
                        "{}{}{}",
                        span(ink, &escape(&head)),
                        if mid.is_empty() { String::new() } else { format!("<b>{}</b>", span(strong, &escape(&mid))) },
                        span(ink, &escape(&tail))
                    )
                }
                None => span(ink, &escape(text)),
            };
            let sign_s = if sign == ' ' { '\u{a0}' } else { sign };
            Line {
                markup: format!("{}{}", span(strong, &format!("{}\u{a0}", sign_s)), body),
                chars: chars.len() + 2,
                tint: tint.map(str::to_string),
                name: String::new(),
                collapsed: false,
            }
        };
        match pair {
            Some(plus) => {
                out.push(render('-', text, Some(&plus)));
                out.push(render('+', &plus, Some(text)));
                i += 2;
            }
            None => {
                out.push(render(*sign, text, None));
                i += 1;
            }
        }
    }
    let _ = spec;
    for (i, l) in out.iter_mut().enumerate() {
        l.name = format!("line{}", i + 1);
    }
    out
}

fn runs_markup(runs: Vec<(Kind, String)>) -> String {
    runs.into_iter()
        .map(|(k, s)| match k.token() {
            Some(t) => format!("<span fill={}>{}</span>", t, escape(&s)),
            None => escape(&s),
        })
        .collect()
}

fn source_lines(spec: &CodeSpec, source: &str) -> Vec<Line> {
    let source = source.replace('\r', "");
    let lines: Vec<&str> = source.split('\n').collect();
    let mut runs = tokenize_lines(&spec.lang, &source).into_iter();
    let last = spec.first + lines.len().saturating_sub(1);
    let w = last.to_string().len();
    lines
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let g = if spec.line_numbers { gutter(Some(spec.first + i), w) } else { String::new() };
            Line {
                markup: format!("{}{}", g, runs_markup(runs.next().unwrap_or_default())),
                chars: l.chars().count() + gutter_chars(spec),
                tint: None,
                name: format!("line{}", i + 1),
                collapsed: false,
            }
        })
        .collect()
}

/// A string literal in AIL source.
fn lit(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"))
}

/// The template body for one code block. `name` is the generated template's
/// name; parts are `bg`, `title`, `body`, `line1..N`.
pub fn template_source(name: &str, spec: &CodeSpec, inserts: &[Insert]) -> String {
    let base = match (&spec.diff, &spec.source) {
        (Some(d), _) => diff_lines(spec, d),
        (None, Some(s)) => source_lines(spec, s),
        (None, None) => Vec::new(),
    };
    // Inserted lines go after the line they name, in statement order.
    let mut lines: Vec<Line> = Vec::new();
    let place = |after: usize, lines: &mut Vec<Line>| {
        for ins in inserts.iter().filter(|i| i.after == after) {
            let mut runs = tokenize_lines(&spec.lang, &ins.source).into_iter();
            for (j, text) in ins.source.split('\n').enumerate() {
                let g = if spec.line_numbers && spec.diff.is_none() { gutter(None, gutter_width(spec)) } else { String::new() };
                lines.push(Line {
                    markup: format!("{}{}", g, runs_markup(runs.next().unwrap_or_default())),
                    chars: text.chars().count() + gutter_chars(spec),
                    tint: Some(ins.tint.clone().unwrap_or_else(|| "code-mark-bg".to_string())),
                    name: format!("{}{}", ins.name, j + 1),
                    collapsed: true,
                });
            }
        }
    };
    place(0, &mut lines);
    for (i, mut l) in base.into_iter().enumerate() {
        // Marks count displayed line numbers (a `lines: "3-8"` block starts at 3).
        let shown = spec.first + i;
        if let Some((_, _, c)) = spec.marks.iter().find(|(a, b, _)| (*a..=*b).contains(&shown)) {
            l.tint = Some(c.clone());
        }
        lines.push(l);
        place(i + 1, &mut lines);
    }
    let fs = spec.font_size;
    let chars = lines.iter().map(|l| l.chars).max().unwrap_or(0).max(spec.min_chars);
    let width = (chars as f64 * crate::layout::text::mono_advance() * fs + 2.0 * INSET + 16.0).ceil();
    let mut s = format!("template {} () {{\n", lit(name));
    if spec.frame {
        s.push_str("    rect bg [fill: code-bg, stroke: code-frame, stroke_width: 3, corner_radius: 14]\n");
    } else {
        s.push_str("    rect bg [fill: none, stroke: none]\n");
    }
    // Parts stack by constraints, not in a column: one system, so a line
    // that collapses or opens moves the lines below it and the frame in the
    // same solve.
    let mut stack: Vec<String> = Vec::new();
    if let Some(t) = &spec.title {
        s.push_str(&format!(
            "    rect title [width: {w}, height: {h}, fill: code-title-bg, stroke: none, {shape}\
             label: {l}, font_size: {f}, font_weight: 700, label_fill: code-title, align: start]\n",
            w = width,
            // In a frame, the bar is cut to the frame's rounded inside; on
            // its own, it rounds its own corners.
            shape = if spec.frame { "clip: bg, " } else { "corner_radius: 11, " },
            l = lit(t),
            // The title grows with the code (19px at the default 17).
            f = (fs * 19.0 / 17.0).round(),
            h = (fs * 46.0 / 17.0).round()
        ));
        s.push_str(&format!("    rect title_rule [width: {}, height: 3, fill: code-frame, stroke: none]\n", width));
        stack.push("title".into());
        stack.push("title_rule".into());
    }
    let pad = if spec.frame { 8 } else { 0 };
    s.push_str(&format!("    rect top_pad [width: {}, height: {}, fill: none, stroke: none]\n", width, pad));
    stack.push("top_pad".into());
    for l in &lines {
        s.push_str(&format!(
            "    rect {n} [width: {w}, height: {h}, fill: {f}, stroke: none, label: {l}, font_size: {fs}, \
             font_family: mono, align: start, label_fill: code-plain, code_lang: {lang}{c}{r}]\n",
            n = l.name,
            w = width,
            h = spec.line_height,
            f = l.tint.as_deref().unwrap_or("none"),
            l = lit(&l.markup),
            fs = fs,
            lang = lit(&spec.lang),
            // A diff's rows are pills, as in a review tool.
            r = if spec.diff.is_some() { ", corner_radius: 6" } else { "" },
            // Inserted lines carry their `as:` name as a class: `remove .conflict`.
            c = if l.collapsed {
                format!(", collapsed: true, class: {}", l.name.trim_end_matches(|ch: char| ch.is_ascii_digit()).trim_end_matches('_'))
            } else {
                String::new()
            },
        ));
        stack.push(l.name.clone());
    }
    s.push_str(&format!("    rect bottom_pad [width: {}, height: {}, fill: none, stroke: none]\n", width, pad));
    stack.push("bottom_pad".into());
    let first = &stack[0];
    for w in stack.windows(2) {
        let gap = if spec.diff.is_some() && w[0].starts_with("line") && w[1].starts_with("line") { " + 4" } else { "" };
        s.push_str(&format!("    constrain {}.top = {}.bottom{}\n", w[1], w[0], gap));
        s.push_str(&format!("    constrain {}.left = {}.left\n", w[1], first));
    }
    s.push_str(&format!(
        "    constrain bg.left = {f}.left - 3\n    constrain bg.top = {f}.top - 3\n\
         \x20   constrain bg.right = {f}.right + 3\n    constrain bg.bottom = bottom_pad.bottom + 3\n",
        f = first
    ));
    if spec.title.is_some() && !spec.frame {
        // Square off the title bar's lower corners (only the top is rounded).
        s.push_str(&format!(
            "    rect title_square [width: {}, height: 12, fill: code-title-bg, stroke: none]\n\
             \x20   constrain title_square.left = title.left\n\
             \x20   constrain title_square.bottom = title.bottom\n",
            width
        ));
    }
    s.push_str("}\n");
    s
}

fn arg<'a>(t: &'a TemplateInstance, key: &str) -> Option<&'a StyleValue> {
    t.arguments.iter().find(|(k, _)| k.node.0 == key).map(|(_, v)| &v.node)
}

fn arg_text(t: &TemplateInstance, key: &str) -> Option<String> {
    match arg(t, key)? {
        StyleValue::String(s) | StyleValue::Keyword(s) => Some(s.clone()),
        StyleValue::Identifier(id) => Some(id.0.clone()),
        StyleValue::Number { value, .. } => Some(format!("{}", value)),
        _ => None,
    }
}

fn arg_num(t: &TemplateInstance, key: &str) -> Option<f64> {
    match arg(t, key)? {
        StyleValue::Number { value, .. } => Some(*value),
        StyleValue::String(s) => s.parse().ok(),
        _ => None,
    }
}

fn arg_bool(t: &TemplateInstance, key: &str) -> Option<bool> {
    match arg(t, key)? {
        StyleValue::Identifier(id) => Some(id.0 == "true"),
        StyleValue::Keyword(k) => Some(k == "true"),
        StyleValue::String(s) => Some(s == "true"),
        _ => None,
    }
}

/// `lines: "3-8"` -> the 1-based inclusive range.
fn line_range(spec: &str) -> Option<(usize, usize)> {
    let (a, b) = spec.split_once('-').unwrap_or((spec, spec));
    Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
}

pub const ARGS: &[&str] = &[
    "lang", "source", "file", "lines", "diff", "title", "line_numbers", "font_size", "line_height", "min_chars",
    "marks", "appears", "frame",
];

/// `"1:role-ok-soft, 3-5:role-warn-soft"` -> (from, to, colour).
fn parse_marks(name: &str, spec: &str) -> Result<Vec<(usize, usize, String)>, String> {
    spec.split(',')
        .filter(|p| !p.trim().is_empty())
        .map(|p| {
            let (lines, colour) = p.split_once(':').ok_or_else(|| {
                format!("code {}: marks are \"line:colour\" or \"from-to:colour\", e.g. \"8:role-warn-soft\"", name)
            })?;
            let (a, b) = line_range(lines).ok_or_else(|| format!("code {}: `{}` is not a line or a range", name, lines))?;
            Ok((a, b, colour.trim().to_string()))
        })
        .collect()
}

/// Build the spec for a `code` instance.
pub fn spec_for(t: &TemplateInstance, base: Option<&std::path::Path>) -> Result<CodeSpec, String> {
    let name = &t.instance_name.node.0;
    for (k, _) in &t.arguments {
        if !ARGS.contains(&k.node.0.as_str()) {
            return Err(format!(
                "code {}: unknown argument `{}` (code takes: {})",
                name,
                k.node.0,
                ARGS.join(", ")
            ));
        }
    }
    let mut source = arg_text(t, "source");
    if let Some(f) = arg_text(t, "file") {
        let path = base.map(|b| b.join(&f)).unwrap_or_else(|| f.clone().into());
        source = Some(
            std::fs::read_to_string(&path).map_err(|e| format!("code {}: cannot read {}: {}", name, path.display(), e))?,
        );
    }
    let mut first = 1;
    if let (Some(src), Some(r)) = (&source, arg_text(t, "lines")) {
        let (a, b) = line_range(&r).ok_or_else(|| format!("code {}: `lines` is \"from-to\", e.g. \"3-8\"", name))?;
        let all: Vec<&str> = src.split('\n').collect();
        let a = a.max(1);
        let b = b.min(all.len());
        source = Some(all[a - 1..b].join("\n"));
        first = a;
    }
    let diff = arg_text(t, "diff");
    if source.is_none() && diff.is_none() {
        return Err(format!("code {}: give it `source: \"...\"`, `file: \"...\"` or `diff: \"...\"`", name));
    }
    let lang = arg_text(t, "lang").unwrap_or_else(|| {
        arg_text(t, "file")
            .and_then(|f| f.rsplit('.').next().map(str::to_string))
            .unwrap_or_default()
    });
    let font_size = arg_num(t, "font_size").unwrap_or(17.0);
    Ok(CodeSpec {
        lang,
        source,
        diff,
        title: arg_text(t, "title"),
        line_numbers: arg_bool(t, "line_numbers").unwrap_or(true),
        first,
        font_size,
        line_height: arg_num(t, "line_height").unwrap_or((font_size * 1.65).round()),
        min_chars: arg_num(t, "min_chars").map(|n| n as usize).unwrap_or(0),
        frame: arg_bool(t, "frame").unwrap_or(true),
        marks: match arg_text(t, "marks") {
            Some(m) => parse_marks(name, &m)?,
            None => Vec::new(),
        },
    })
}

/// Replace every `code` instance (when no user template is called `code`)
/// with an instance of a template generated for it, and prepend those
/// templates. Keyframes are rewritten too: `insert` becomes a `show [enter:
/// expand]` of the lines it adds, and `transform c.line4 [source: "..."]`
/// becomes the highlighted label. Runs before template resolution.
pub fn expand_code_blocks(
    stmts: Vec<Spanned<Statement>>,
    base: Option<&std::path::Path>,
) -> Result<Vec<Spanned<Statement>>, (std::ops::Range<usize>, String)> {
    let user_code = stmts.iter().any(|s| matches!(&s.node, Statement::TemplateDecl(d) if d.name.node.0 == "code"));
    if user_code {
        return Ok(stmts);
    }
    let mut specs: std::collections::BTreeMap<String, CodeSpec> = Default::default();
    collect_specs(&stmts, base, &mut specs)?;
    if specs.is_empty() {
        return Ok(stmts);
    }
    let mut inserts: std::collections::BTreeMap<String, Vec<Insert>> = Default::default();
    let mut stmts = stmts;
    // `appears:` on a code block acts on the block (the generated instance
    // has no modifiers of its own to carry it).
    let mut appears: Vec<(String, String, std::ops::Range<usize>)> = Vec::new();
    collect_code_appears(&stmts, &mut appears);
    for (name, when, span) in appears {
        let names: Vec<String> = stmts
            .iter()
            .filter_map(|s| match &s.node {
                Statement::Keyframe(k) => Some(k.name.node.clone()),
                _ => None,
            })
            .collect();
        let fi = if when == "later" {
            None
        } else {
            Some(names.iter().position(|n| *n == when).ok_or_else(|| {
                (span.clone(), format!("code {}: appears: \"{}\" is not a keyframe", name, when))
            })?)
        };
        let stmt = |show: bool| {
            use crate::parser::ast::{MotionNode, MotionStmt, MotionVerb, Selector};
            let sel = vec![Spanned::new(Selector::Name(name.clone()), span.clone())];
            Spanned::new(
                MotionNode::Stmt(MotionStmt {
                    verb: if show { MotionVerb::Show(sel) } else { MotionVerb::Hide(sel) },
                    opts: vec![],
                    targets: vec![],
                    partners: vec![],
                }),
                span.clone(),
            )
        };
        let mut k = 0;
        for st in &mut stmts {
            let Statement::Keyframe(kf) = &mut st.node else { continue };
            if k == 0 && fi != Some(0) {
                kf.motion.insert(0, stmt(false));
            }
            if Some(k) == fi && k > 0 {
                kf.motion.insert(0, stmt(true));
            }
            k += 1;
        }
    }
    for st in &mut stmts {
        if let Statement::Keyframe(k) = &mut st.node {
            rewrite_motion(&mut k.motion, &specs, &mut inserts)?;
        }
    }
    let mut generated = Vec::new();
    let out = rewrite(stmts, &specs, &inserts, &mut generated)?;
    let mut all = generated;
    all.extend(out);
    Ok(all)
}

fn collect_code_appears(stmts: &[Spanned<Statement>], out: &mut Vec<(String, String, std::ops::Range<usize>)>) {
    for s in stmts {
        match &s.node {
            Statement::TemplateInstance(t) if t.template_name.node.0 == "code" => {
                if let Some(w) = arg_text(t, "appears") {
                    out.push((t.instance_name.node.0.clone(), w, s.span.clone()));
                }
            }
            Statement::Layout(l) => collect_code_appears(&l.children, out),
            Statement::Group(g) => collect_code_appears(&g.children, out),
            _ => {}
        }
    }
}

fn collect_specs(
    stmts: &[Spanned<Statement>],
    base: Option<&std::path::Path>,
    out: &mut std::collections::BTreeMap<String, CodeSpec>,
) -> Result<(), (std::ops::Range<usize>, String)> {
    for s in stmts {
        match &s.node {
            Statement::TemplateInstance(t) if t.template_name.node.0 == "code" => {
                let spec = spec_for(t, base).map_err(|m| (s.span.clone(), m))?;
                out.insert(t.instance_name.node.0.clone(), spec);
            }
            Statement::Layout(l) => collect_specs(&l.children, base, out)?,
            Statement::Group(g) => collect_specs(&g.children, base, out)?,
            _ => {}
        }
    }
    Ok(())
}

fn opt_str(opts: &[Spanned<crate::parser::ast::MotionOpt>], key: &str) -> Option<String> {
    use crate::parser::ast::MotionValue;
    opts.iter().find(|o| o.node.key.node == key).and_then(|o| match &o.node.value.node {
        MotionValue::Str(s) | MotionValue::Name(s) => Some(s.clone()),
        _ => None,
    })
}

/// `c.line4` / `c.conflict2` -> (code block, part).
fn split_part<'a>(name: &'a str, specs: &std::collections::BTreeMap<String, CodeSpec>) -> Option<(&'a str, &'a str)> {
    let (code, part) = name.rsplit_once('.').or_else(|| name.rsplit_once('_'))?;
    specs.contains_key(code).then_some((code, part))
}

fn rewrite_motion(
    nodes: &mut [Spanned<crate::parser::ast::MotionNode>],
    specs: &std::collections::BTreeMap<String, CodeSpec>,
    inserts: &mut std::collections::BTreeMap<String, Vec<Insert>>,
) -> Result<(), (std::ops::Range<usize>, String)> {
    use crate::parser::ast::{MotionNode, MotionOpt, MotionValue, MotionVerb, Selector, StyleKey};
    for n in nodes {
        let span = n.span.clone();
        match &mut n.node {
            MotionNode::Then(b) | MotionNode::After(_, b) | MotionNode::At(_, b) | MotionNode::When(_, _, b) | MotionNode::Beat(_, b) => {
                rewrite_motion(b, specs, inserts)?
            }
            MotionNode::Stmt(st) => match &mut st.verb {
                MotionVerb::Insert { code, after } => {
                    let Some(spec) = specs.get(&code.node) else {
                        return Err((code.span.clone(), format!("insert: `{}` is not a code block", code.node)));
                    };
                    let source = opt_str(&st.opts, "source")
                        .ok_or_else(|| (span.clone(), "insert: give the new lines as `[source: \"...\"]`".to_string()))?;
                    let count = spec.source.as_deref().map(|s| s.split('\n').count()).unwrap_or(0);
                    if *after > count {
                        return Err((span.clone(), format!("insert: `{}` has {} lines, no line {}", code.node, count, after)));
                    }
                    let list = inserts.entry(code.node.clone()).or_default();
                    let name = opt_str(&st.opts, "name").unwrap_or_else(|| format!("ins{}_", list.len() + 1));
                    let tint = opt_str(&st.opts, "tint");
                    let n_lines = source.split('\n').count();
                    list.push(Insert { after: *after, name: name.clone(), source, tint });
                    let sels = (1..=n_lines)
                        .map(|j| Spanned::new(Selector::Name(format!("{}.{}{}", code.node, name, j)), code.span.clone()))
                        .collect();
                    st.opts.retain(|o| !matches!(o.node.key.node.as_str(), "source" | "name" | "tint"));
                    if !st.opts.iter().any(|o| o.node.key.node == "enter") {
                        st.opts.push(Spanned::new(
                            MotionOpt {
                                key: Spanned::new("enter".to_string(), span.clone()),
                                value: Spanned::new(MotionValue::Name("expand".into()), span.clone()),
                            },
                            span.clone(),
                        ));
                    }
                    st.verb = MotionVerb::Show(sels);
                }
                MotionVerb::Transform { target, modifiers } => {
                    let Selector::Name(t) = &target.node else { continue };
                    let Some((code, part)) = split_part(t, specs) else { continue };
                    let spec = &specs[code];
                    let number = part.strip_prefix("line").and_then(|n| n.parse::<usize>().ok()).map(|k| spec.first + k - 1);
                    for m in modifiers.iter_mut() {
                        if matches!(&m.node.key.node, StyleKey::Custom(k) if k == "source") {
                            let text = match &m.node.value.node {
                                StyleValue::String(s) => s.clone(),
                                _ => continue,
                            };
                            m.node.key.node = StyleKey::Label;
                            m.node.value.node = StyleValue::String(line_label(spec, number, &text));
                        }
                    }
                }
                _ => {}
            },
        }
    }
    Ok(())
}

fn rewrite(
    stmts: Vec<Spanned<Statement>>,
    specs: &std::collections::BTreeMap<String, CodeSpec>,
    inserts: &std::collections::BTreeMap<String, Vec<Insert>>,
    generated: &mut Vec<Spanned<Statement>>,
) -> Result<Vec<Spanned<Statement>>, (std::ops::Range<usize>, String)> {
    let mut out = Vec::with_capacity(stmts.len());
    for mut s in stmts {
        match &mut s.node {
            Statement::TemplateInstance(t) if t.template_name.node.0 == "code" => {
                let name = t.instance_name.node.0.clone();
                let spec = &specs[&name];
                let tname = format!("__code_{}", name);
                let src = template_source(&tname, spec, inserts.get(&name).map(Vec::as_slice).unwrap_or(&[]));
                let doc = crate::parser::parse(&src)
                    .map_err(|e| (s.span.clone(), format!("code {}: internal error: {:?}", name, e)))?;
                generated.extend(doc.statements);
                t.template_name.node.0 = tname;
                t.arguments.clear();
            }
            Statement::Layout(l) => l.children = rewrite(std::mem::take(&mut l.children), specs, inserts, generated)?,
            Statement::Group(g) => g.children = rewrite(std::mem::take(&mut g.children), specs, inserts, generated)?,
            _ => {}
        }
        out.push(s);
    }
    Ok(out)
}

/// `point hub`: a position and nothing else (a zero-size, invisible anchor for
/// constraints, moves and flights), instead of a 1x1 rect with no fill and no
/// stroke. A user template called `point` takes precedence.
pub fn expand_points(stmts: Vec<Spanned<Statement>>) -> Vec<Spanned<Statement>> {
    let user = stmts.iter().any(|s| matches!(&s.node, Statement::TemplateDecl(d) if d.name.node.0 == "point"));
    if user {
        return stmts;
    }
    fn walk(stmts: Vec<Spanned<Statement>>) -> Vec<Spanned<Statement>> {
        stmts
            .into_iter()
            .map(|mut s| {
                match &mut s.node {
                    Statement::TemplateInstance(t) if t.template_name.node.0 == "point" => {
                        use crate::parser::ast::{ShapeDecl, ShapeType, StyleKey, StyleModifier};
                        let sp = s.span.clone();
                        let m = |k: StyleKey, v: StyleValue| {
                            Spanned::new(
                                StyleModifier { key: Spanned::new(k, sp.clone()), value: Spanned::new(v, sp.clone()) },
                                sp.clone(),
                            )
                        };
                        let none = || StyleValue::Keyword("none".into());
                        let mut modifiers = vec![
                            m(StyleKey::Width, StyleValue::Number { value: 0.0, unit: None }),
                            m(StyleKey::Height, StyleValue::Number { value: 0.0, unit: None }),
                            m(StyleKey::Fill, none()),
                            m(StyleKey::Stroke, none()),
                            m(StyleKey::Class, StyleValue::Identifier(crate::parser::ast::Identifier::new("ai-point"))),
                        ];
                        // Anything else written on it (appears:, ...) is kept.
                        for (k, v) in std::mem::take(&mut t.arguments) {
                            modifiers.push(m(StyleKey::Custom(k.node.0), v.node));
                        }
                        let name = t.instance_name.clone();
                        s.node = Statement::Shape(ShapeDecl {
                            shape_type: Spanned::new(ShapeType::Rectangle, sp.clone()),
                            name: Some(name),
                            modifiers,
                        });
                    }
                    Statement::Layout(l) => l.children = walk(std::mem::take(&mut l.children)),
                    Statement::Group(g) => g.children = walk(std::mem::take(&mut g.children)),
                    // A point is a part like any other inside a component.
                    Statement::TemplateDecl(d) => {
                        if let Some(body) = d.body.take() {
                            d.body = Some(walk(body));
                        }
                    }
                    _ => {}
                }
                s
            })
            .collect()
    }
    walk(stmts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_keywords_strings_and_calls() {
        let t = tokenize("python", "def total(cart, coupon=None):  # sum");
        assert_eq!(t[0], (Kind::Keyword, "def".into()));
        assert!(t.contains(&(Kind::Function, "total".into())));
        assert!(t.contains(&(Kind::Keyword, "None".into())));
        assert_eq!(t.last().unwrap().0, Kind::Comment);
    }

    #[test]
    fn json_keys_are_keys() {
        let t = tokenize("json", r#"{"name": "cart", "qty": 2}"#);
        assert!(t.iter().any(|(k, s)| *k == Kind::Key && s.contains("name")), "{t:?}");
        assert!(t.iter().any(|(k, s)| *k == Kind::String && s.contains("cart")), "{t:?}");
        assert!(t.contains(&(Kind::Number, "2".into())));
    }

    #[test]
    fn markup_escapes_and_keeps_indentation() {
        let m = highlight("python", "    if a < b:");
        assert!(m.starts_with("\u{a0}\u{a0}\u{a0}\u{a0}"));
        assert!(m.contains("&lt;"));
    }
}
