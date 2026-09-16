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
