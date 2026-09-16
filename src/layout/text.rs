//! Text measurement and the label markup subset.
//!
//! Every place that needs to know how wide a piece of text is goes through
//! this module. Before it there were eight call sites using three different
//! per-character constants, all of them counting UTF-8 bytes — so boxes were
//! sized with one number and lint-checked with a larger one, and any label
//! holding `≤` or `—` measured 3× too wide per symbol.
//!
//! Widths are Helvetica advances (AFM units / 1000), which is a good proxy
//! for the Arial/Helvetica/system-sans stack the default stylesheet asks for.
//! Bold is approximated as a 5% widening rather than a second table.

/// Multiplier from font size to the distance between consecutive baselines.
pub const LINE_HEIGHT_FACTOR: f64 = 1.25;

/// Widening factor applied to bold runs, in place of a second advance table.
const BOLD_FACTOR: f64 = 1.05;

/// Advance for a character with no table entry, in em.
const FALLBACK_ADVANCE: f64 = 0.55;

/// Helvetica advances for the printable ASCII range, U+0020..=U+007E,
/// in AFM units (thousandths of an em).
const ASCII_ADVANCES: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, // space ! " # $ % & ' ( )
    389, 584, 278, 333, 278, 278, // * + , - . /
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, // 0-9
    278, 278, 584, 584, 584, 556, 1015, // : ; < = > ? @
    667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, // A-M
    722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, // N-Z
    278, 278, 278, 469, 556, 333, // [ \ ] ^ _ `
    556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833, // a-m
    556, 556, 556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, // n-z
    334, 260, 334, 584, // { | } ~
];

/// Symbols outside ASCII that agents reach for often enough that a fallback
/// guess is visibly wrong — maths relations, dashes, arrows, quotes.
const SYMBOL_ADVANCES: [(char, u16); 26] = [
    ('—', 1000),
    ('–', 556),
    ('·', 278),
    ('•', 350),
    ('…', 1000),
    ('≤', 549),
    ('≥', 549),
    ('≠', 549),
    ('≈', 549),
    ('±', 584),
    ('×', 584),
    ('÷', 584),
    ('∈', 549),
    ('∞', 713),
    ('°', 400),
    ('→', 987),
    ('←', 987),
    ('↔', 1000),
    ('↑', 987),
    ('↓', 987),
    ('«', 556),
    ('»', 556),
    ('“', 333),
    ('”', 333),
    ('‘', 222),
    ('’', 222),
];

/// Advance width of a single character, in em.
pub fn advance_em(c: char) -> f64 {
    let code = c as u32;
    if (0x20..=0x7E).contains(&code) {
        return ASCII_ADVANCES[(code - 0x20) as usize] as f64 / 1000.0;
    }
    if let Some((_, w)) = SYMBOL_ADVANCES.iter().find(|(sym, _)| *sym == c) {
        return *w as f64 / 1000.0;
    }
    // Latin-1 letters are accented forms of ASCII letters and sit close to
    // the lowercase average.
    if (0xC0..=0xFF).contains(&code) {
        return 0.556;
    }
    FALLBACK_ADVANCE
}

/// One styled span of text within a line.
#[derive(Debug, Clone, PartialEq)]
pub struct TextRun {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    /// Font-size multiplier: 1.0 normally, 0.8 inside `<small>`.
    pub scale: f64,
    /// Per-run colour from `<span fill=…>`, as a CSS colour string.
    pub fill: Option<String>,
}

impl TextRun {
    /// An unstyled run — the common case and the one tests read best.
    pub fn plain(text: &str) -> Self {
        TextRun {
            text: text.to_string(),
            bold: false,
            italic: false,
            scale: 1.0,
            fill: None,
        }
    }

    /// Width of this run at the given base font size.
    pub fn width(&self, font_size: f64) -> f64 {
        let per_char: f64 = self.text.chars().map(advance_em).sum();
        let mut w = per_char * font_size * self.scale;
        if self.bold {
            w *= BOLD_FACTOR;
        }
        w
    }
}

/// A single line: one or more styled runs laid end to end.
pub type TextLine = Vec<TextRun>;

/// A parsed label: lines of styled runs.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RichText {
    pub lines: Vec<TextLine>,
}

impl RichText {
    /// Treat a string as a single unstyled line. Used for text that never
    /// goes through the markup parser (grid gutters, debug labels).
    pub fn from_plain(s: &str) -> Self {
        RichText {
            lines: vec![vec![TextRun::plain(s)]],
        }
    }

    /// The markup stripped back out, for lint messages and keyframe diffs
    /// that compare wording rather than styling. Lines join with a space so
    /// the result reads as one phrase.
    pub fn plain(&self) -> String {
        self.lines
            .iter()
            .map(|line| line.iter().map(|r| r.text.as_str()).collect::<String>())
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn is_empty(&self) -> bool {
        self.lines.iter().all(|l| l.iter().all(|r| r.text.is_empty()))
    }
}

/// Measured extent of a laid-out label.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextMetrics {
    pub width: f64,
    pub height: f64,
    pub line_count: usize,
}

/// Width of a plain string at a given font size.
pub fn measure_str(text: &str, font_size: f64) -> f64 {
    TextRun::plain(text).width(font_size)
}

/// Extent of a set of lines: widest line by width, line count by line height.
pub fn measure_runs(lines: &[TextLine], font_size: f64) -> TextMetrics {
    let width = lines
        .iter()
        .map(|line| line.iter().map(|r| r.width(font_size)).sum::<f64>())
        .fold(0.0, f64::max);
    let line_count = lines.len().max(1);
    TextMetrics {
        width,
        height: line_count as f64 * font_size * LINE_HEIGHT_FACTOR,
        line_count,
    }
}

/// Tags the label markup subset understands. Anything else that looks like a
/// tag is left as literal text and reported by the linter.
pub const SUPPORTED_TAGS: &[&str] = &["br", "b", "i", "small", "span"];

/// A malformed *recognised* tag. Unrecognised angle brackets are not errors —
/// they are text.
#[derive(Debug, Clone, PartialEq)]
pub struct MarkupError {
    pub message: String,
    /// Byte offset into the label string where the problem starts.
    pub offset: usize,
}

/// Style state carried down through nested tags.
#[derive(Clone)]
struct MarkupStyle {
    bold: bool,
    italic: bool,
    scale: f64,
    fill: Option<String>,
}

impl MarkupStyle {
    fn root() -> Self {
        MarkupStyle {
            bold: false,
            italic: false,
            scale: 1.0,
            fill: None,
        }
    }
}

/// Flush the pending characters as a run under the current style.
fn flush_run(buf: &mut String, style: &MarkupStyle, lines: &mut Vec<TextLine>) {
    if buf.is_empty() {
        return;
    }
    lines.last_mut().expect("always one line").push(TextRun {
        text: std::mem::take(buf),
        bold: style.bold,
        italic: style.italic,
        scale: style.scale,
        fill: style.fill.clone(),
    });
}

/// Parse the label markup subset into lines of styled runs.
///
/// `<` only begins a tag when a recognised tag name follows it; otherwise it
/// is literal text, so `"T < confirmed_until"` and `"Vec<String>"` survive.
pub fn parse_markup(src: &str) -> Result<RichText, MarkupError> {
    let chars: Vec<char> = src.chars().collect();
    let mut lines: Vec<TextLine> = vec![Vec::new()];
    let mut stack: Vec<(String, MarkupStyle)> = Vec::new();
    let mut style = MarkupStyle::root();
    let mut buf = String::new();
    let mut i = 0usize;

    while i < chars.len() {
        let c = chars[i];

        if c == '\n' {
            flush_run(&mut buf, &style, &mut lines);
            lines.push(Vec::new());
            i += 1;
            continue;
        }

        if c == '&' {
            if let Some((entity, len)) = read_entity(&chars[i..]) {
                buf.push(entity);
                i += len;
                continue;
            }
            buf.push(c);
            i += 1;
            continue;
        }

        if c != '<' {
            buf.push(c);
            i += 1;
            continue;
        }

        // `<` — a tag only if a recognised name follows, else literal.
        match read_tag(&chars[i..]) {
            None => {
                buf.push('<');
                i += 1;
            }
            Some(tag) => {
                flush_run(&mut buf, &style, &mut lines);
                let offset = i;
                i += tag.consumed;

                if tag.closing {
                    match stack.pop() {
                        Some((open, prev)) if open == tag.name => style = prev,
                        _ => {
                            return Err(MarkupError {
                                message: format!(
                                    "</{}> closes a tag that was never opened",
                                    tag.name
                                ),
                                offset,
                            })
                        }
                    }
                    continue;
                }

                match tag.name.as_str() {
                    "br" => lines.push(Vec::new()),
                    "b" => {
                        stack.push(("b".into(), style.clone()));
                        style.bold = true;
                    }
                    "i" => {
                        stack.push(("i".into(), style.clone()));
                        style.italic = true;
                    }
                    "small" => {
                        stack.push(("small".into(), style.clone()));
                        style.scale *= 0.8;
                    }
                    "span" => {
                        let fill = tag.fill.ok_or_else(|| MarkupError {
                            message: "<span> needs a fill, e.g. <span fill=accent-dark>"
                                .to_string(),
                            offset,
                        })?;
                        stack.push(("span".into(), style.clone()));
                        style.fill = Some(fill);
                    }
                    other => unreachable!("read_tag only yields supported names, got {other}"),
                }
            }
        }
    }

    flush_run(&mut buf, &style, &mut lines);

    if let Some((open, _)) = stack.last() {
        return Err(MarkupError {
            message: format!("<{open}> is never closed; add </{open}>"),
            offset: src.len(),
        });
    }

    Ok(RichText { lines })
}

struct ParsedTag {
    name: String,
    closing: bool,
    /// `fill=` value on a `<span>`.
    fill: Option<String>,
    /// Characters consumed, including the angle brackets.
    consumed: usize,
}

/// Read a recognised tag starting at `chars[0] == '<'`. Returns `None` when
/// what follows is not a supported tag name, so the `<` stays literal.
fn read_tag(chars: &[char]) -> Option<ParsedTag> {
    debug_assert_eq!(chars[0], '<');
    let mut i = 1;
    let closing = chars.get(i) == Some(&'/');
    if closing {
        i += 1;
    }

    let name_start = i;
    while i < chars.len() && chars[i].is_ascii_alphabetic() {
        i += 1;
    }
    let name: String = chars[name_start..i]
        .iter()
        .collect::<String>()
        .to_ascii_lowercase();
    if !SUPPORTED_TAGS.contains(&name.as_str()) {
        return None;
    }

    // Attributes: only `fill=` on `<span>`, bare or quoted.
    let mut fill = None;
    while i < chars.len() && chars[i] != '>' {
        if chars[i].is_whitespace() || chars[i] == '/' {
            i += 1;
            continue;
        }
        let attr_start = i;
        while i < chars.len() && chars[i] != '=' && chars[i] != '>' {
            i += 1;
        }
        let attr: String = chars[attr_start..i].iter().collect();
        if chars.get(i) != Some(&'=') {
            continue;
        }
        i += 1; // '='
        let quote = matches!(chars.get(i), Some('"') | Some('\''));
        if quote {
            i += 1;
        }
        let val_start = i;
        while i < chars.len()
            && chars[i] != '>'
            && !(quote && (chars[i] == '"' || chars[i] == '\''))
            && !(!quote && chars[i].is_whitespace())
        {
            i += 1;
        }
        let value: String = chars[val_start..i].iter().collect();
        if quote {
            i += 1;
        }
        if attr.eq_ignore_ascii_case("fill") {
            fill = Some(value);
        }
    }

    if chars.get(i) != Some(&'>') {
        return None; // never closed the bracket — treat `<` as literal
    }

    Some(ParsedTag {
        name,
        closing,
        fill,
        consumed: i + 1,
    })
}

/// Read an XML entity at `chars[0] == '&'`, returning the character and the
/// number of characters consumed.
fn read_entity(chars: &[char]) -> Option<(char, usize)> {
    for (name, ch) in [("lt;", '<'), ("gt;", '>'), ("amp;", '&'), ("quot;", '"')] {
        let n = name.chars().count();
        if chars.len() > n && chars[1..=n].iter().collect::<String>() == name {
            return Some((ch, n + 1));
        }
    }
    None
}

/// Greedily wrap each line to `max_width`, breaking only at spaces.
///
/// A word wider than the whole line is left to overflow: breaking mid-word
/// needs hyphenation rules we do not have, and `--lint` reports the overflow
/// with the shape's name, which is more useful than a silent mid-word break.
pub fn wrap(rich: &RichText, font_size: f64, max_width: f64) -> RichText {
    let mut out: Vec<TextLine> = Vec::new();

    for source_line in &rich.lines {
        let mut current: TextLine = Vec::new();
        let mut current_width = 0.0f64;

        for run in source_line {
            // Split into words, keeping the spaces attached to the word that
            // follows so that re-joining a line reproduces the original text.
            for (idx, word) in run.text.split(' ').enumerate() {
                let piece = if idx == 0 {
                    word.to_string()
                } else {
                    format!(" {word}")
                };
                if piece.is_empty() {
                    continue;
                }
                let styled = TextRun {
                    text: piece.clone(),
                    ..run.clone()
                };
                let w = styled.width(font_size);

                if !current.is_empty() && current_width + w > max_width {
                    out.push(coalesce(std::mem::take(&mut current)));
                    // The word starts a fresh line, so drop its leading space.
                    let trimmed = TextRun {
                        text: piece.trim_start().to_string(),
                        ..run.clone()
                    };
                    current_width = trimmed.width(font_size);
                    current.push(trimmed);
                } else {
                    current_width += w;
                    current.push(styled);
                }
            }
        }

        out.push(coalesce(current));
    }

    RichText { lines: out }
}

/// Merge neighbouring runs that share their styling.
///
/// Wrapping splits a line word by word to measure it; without this every word
/// would reach the renderer as its own `<tspan>`.
fn coalesce(line: TextLine) -> TextLine {
    let mut out: TextLine = Vec::new();
    for run in line {
        match out.last_mut() {
            Some(prev)
                if prev.bold == run.bold
                    && prev.italic == run.italic
                    && (prev.scale - run.scale).abs() < f64::EPSILON
                    && prev.fill == run.fill =>
            {
                prev.text.push_str(&run.text);
            }
            _ => out.push(run),
        }
    }
    out
}
