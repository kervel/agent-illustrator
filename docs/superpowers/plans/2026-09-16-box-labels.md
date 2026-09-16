# Box Labels Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make "text centred in a box / above it / below it", multi-line labels, and text that wraps instead of overflowing expressible in one modifier each, so an agent never computes a label offset.

**Architecture:** A new `src/layout/text.rs` owns all text measurement and a small `<tspan>`-targeted markup subset; every existing measurement site is rewired to it. `LabelLayout` gains a parsed `RichText` alongside its plain-text flattening, the engine learns to place a label outside its shape, and the renderer emits one `<tspan>` per line. The linter is then trimmed so the warnings that remain name their own fix.

**Tech Stack:** Rust 2021, `cargo test`, no new dependencies. Rendering to SVG via `src/renderer/svg.rs` (string building, not a DOM library).

**Spec:** `docs/superpowers/specs/2026-09-16-box-labels-design.md`

## Global Constraints

- **Zero new grammar keywords.** `label`, `label_position`, `label_offset`, `align`, `font_size` all already parse (`src/parser/grammar.rs:140-175`). No lexer or grammar changes in this plan.
- **No new dependencies.** The advance table is a static Rust array; no font loading.
- Measure `chars()`, never `str::len()`. `str::len()` counts UTF-8 bytes and every current site gets `≤`, `—`, `·` wrong by 3× per character.
- Line height is `1.25 × font_size` everywhere.
- A `<` in a label that does not begin a recognised tag is **literal text**. `"T < confirmed_until"` must keep working.
- The existing SVG regression test compares structure, not bytes (`tests/svg_regression.rs`); example SVGs are expected to change and are re-rendered by `bash examples/render-all.sh`.
- Never `git add -A` in this repo — the working tree carries untracked scratch files. Stage explicit paths only.
- Run the full suite with `cargo test` before every commit.

## File Structure

| File | Responsibility |
|---|---|
| `src/layout/text.rs` (new) | Advance table, `measure_str`/`measure_runs`, markup parser, greedy wrap. The single source of truth for "how wide is this text". |
| `src/layout/mod.rs` | Register `pub mod text;`. |
| `src/layout/types.rs` | `LabelLayout` gains `rich` + `line_height`; `expand_bounds_for_label` handles multi-line and outside positions. |
| `src/layout/engine.rs` | `compute_shape_size` measures via `text.rs`; new `label_position:` placement; wrapping at an explicit width. |
| `src/renderer/svg.rs` | `add_text_with_classes` emits one `<tspan>` per line and one nested `<tspan>` per styled run. |
| `src/layout/lint.rs` | Containment epsilon, duplicate-warning merge, two new rules. |
| `src/layout/keyframe.rs`, `src/layout/routing.rs` | Rewired to `text.rs` measurement. |
| `tests/text_measurement.rs` (new) | Unit tests for the advance table and `measure_*`. |
| `tests/label_markup.rs` (new) | Markup parsing + wrapping + multi-line rendering. |
| `tests/label_position.rs` (new) | `label_position` × `align` placement and bounds. |
| `tests/lint_label_guidance.rs` (new) | The two motivating failures as fixtures. |
| `examples/card-labels.ail` (new) | Canonical reference figure for the feature. |

---

### Task 1: Text measurement module

**Files:**
- Create: `src/layout/text.rs`
- Modify: `src/layout/mod.rs:6-15`
- Test: `tests/text_measurement.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `agent_illustrator::layout::text::{TextRun, TextLine, RichText, TextMetrics, measure_str, measure_runs, advance_em, LINE_HEIGHT_FACTOR}`.
  - `pub fn advance_em(c: char) -> f64`
  - `pub fn measure_str(text: &str, font_size: f64) -> f64`
  - `pub fn measure_runs(lines: &[TextLine], font_size: f64) -> TextMetrics`
  - `pub struct TextMetrics { pub width: f64, pub height: f64, pub line_count: usize }`
  - `pub struct TextRun { pub text: String, pub bold: bool, pub italic: bool, pub scale: f64, pub fill: Option<String> }`
  - `pub type TextLine = Vec<TextRun>;`
  - `pub struct RichText { pub lines: Vec<TextLine> }` with `pub fn plain(&self) -> String` and `pub fn from_plain(s: &str) -> RichText`
  - `pub const LINE_HEIGHT_FACTOR: f64 = 1.25;`

- [ ] **Step 1: Write the failing test**

Create `tests/text_measurement.rs`:

```rust
//! The single measurement path. Before this module there were eight call
//! sites with three different constants, all counting UTF-8 bytes.

use agent_illustrator::layout::text::{
    advance_em, measure_runs, measure_str, RichText, TextRun, LINE_HEIGHT_FACTOR,
};

#[test]
fn ascii_advances_come_from_the_table_not_a_flat_constant() {
    // A lowercase 'l' is narrow and an 'M' is wide; a flat per-character
    // constant cannot tell them apart, which is how boxes ended up
    // mis-sized for both short and long words.
    assert!(advance_em('l') < 0.30, "'l' should be narrow, got {}", advance_em('l'));
    assert!(advance_em('M') > 0.75, "'M' should be wide, got {}", advance_em('M'));
    assert!(advance_em(' ') > 0.0, "space must have width");
}

#[test]
fn multibyte_characters_count_as_one_character() {
    // "≤" is 3 bytes. The old `str::len()` sites measured it as three
    // characters, inflating every box holding a maths symbol or em dash.
    let one_le = measure_str("≤", 14.0);
    let three_ascii = measure_str("mmm", 14.0);
    assert!(
        one_le < three_ascii,
        "a single ≤ ({one_le}) must not measure wider than three m's ({three_ascii})"
    );
}

#[test]
fn unlisted_characters_fall_back_rather_than_measuring_zero() {
    // A CJK or emoji character has no table entry; zero width would let it
    // silently overflow its box.
    assert!(advance_em('漢') > 0.0);
}

#[test]
fn measuring_a_string_scales_linearly_with_font_size() {
    let at_14 = measure_str("hello world", 14.0);
    let at_28 = measure_str("hello world", 28.0);
    assert!((at_28 - at_14 * 2.0).abs() < 0.001, "{at_14} vs {at_28}");
}

#[test]
fn runs_measure_to_the_widest_line_and_stack_in_height() {
    let rich = RichText {
        lines: vec![
            vec![TextRun::plain("short")],
            vec![TextRun::plain("a much longer line")],
        ],
    };
    let m = measure_runs(&rich.lines, 14.0);
    assert_eq!(m.line_count, 2);
    assert!((m.width - measure_str("a much longer line", 14.0)).abs() < 0.001);
    assert!((m.height - 2.0 * 14.0 * LINE_HEIGHT_FACTOR).abs() < 0.001);
}

#[test]
fn bold_runs_measure_wider_than_normal_ones() {
    let normal = measure_runs(&[vec![TextRun::plain("changeset")]], 14.0);
    let bold = measure_runs(
        &[vec![TextRun { bold: true, ..TextRun::plain("changeset") }]],
        14.0,
    );
    assert!(bold.width > normal.width, "{} vs {}", bold.width, normal.width);
}

#[test]
fn small_runs_measure_narrower_and_do_not_change_line_height() {
    let normal = measure_runs(&[vec![TextRun::plain("inherits")]], 14.0);
    let small = measure_runs(
        &[vec![TextRun { scale: 0.8, ..TextRun::plain("inherits") }]],
        14.0,
    );
    assert!(small.width < normal.width);
    assert!((small.height - normal.height).abs() < 0.001);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test text_measurement`
Expected: FAIL — `unresolved import agent_illustrator::layout::text`.

- [ ] **Step 3: Write the implementation**

Create `src/layout/text.rs`:

```rust
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
    ('—', 1000), ('–', 556), ('·', 278), ('•', 350), ('…', 1000),
    ('≤', 549), ('≥', 549), ('≠', 549), ('≈', 549), ('±', 584),
    ('×', 584), ('÷', 584), ('∈', 549), ('∞', 713), ('°', 400),
    ('→', 987), ('←', 987), ('↔', 1000), ('↑', 987), ('↓', 987),
    ('«', 556), ('»', 556), ('“', 333), ('”', 333), ('‘', 222), ('’', 222),
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
```

Register the module in `src/layout/mod.rs`, in the existing alphabetical `pub mod` block (after `pub mod solver;`):

```rust
pub mod text;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test text_measurement`
Expected: PASS, 7 tests.

Then `cargo test` — expected: everything still passes, because nothing consumes the new module yet.

- [ ] **Step 5: Commit**

```bash
git add src/layout/text.rs src/layout/mod.rs tests/text_measurement.rs
git commit -m "feat(text): single text measurement module with a real advance table

Eight call sites measured text with three different per-character
constants, all counting UTF-8 bytes. Boxes were sized with 8.0px/char
and lint-checked with 8.4, so lint reported overflow on text that fits.

Not wired up yet."
```

---

### Task 2: Label markup parser

**Files:**
- Modify: `src/layout/text.rs` (append)
- Test: `tests/label_markup.rs`

**Interfaces:**
- Consumes: `TextRun`, `TextLine`, `RichText` from Task 1.
- Produces:
  - `pub fn parse_markup(s: &str) -> Result<RichText, MarkupError>`
  - `pub struct MarkupError { pub message: String, pub offset: usize }`
  - `pub const SUPPORTED_TAGS: &[&str] = &["br", "b", "i", "small", "span"];`

- [ ] **Step 1: Write the failing test**

Create `tests/label_markup.rs`:

```rust
//! The label markup subset: enough to write a card in one modifier,
//! small enough to measure ourselves.

use agent_illustrator::layout::text::{parse_markup, RichText};

fn lines_of(src: &str) -> Vec<String> {
    parse_markup(src)
        .expect("should parse")
        .lines
        .iter()
        .map(|l| l.iter().map(|r| r.text.as_str()).collect::<String>())
        .collect()
}

#[test]
fn plain_text_is_one_line() {
    assert_eq!(lines_of("Cache"), vec!["Cache"]);
}

#[test]
fn br_splits_lines() {
    assert_eq!(lines_of("a<br>b<br/>c"), vec!["a", "b", "c"]);
}

#[test]
fn a_literal_newline_is_a_line_break() {
    // An LLM writing prose reaches for \n before it reaches for <br>.
    assert_eq!(lines_of("a\nb"), vec!["a", "b"]);
}

#[test]
fn bold_italic_and_small_mark_their_runs() {
    let rich = parse_markup("<b>title</b> <i>note</i> <small>fine</small>").unwrap();
    let runs = &rich.lines[0];
    let bold = runs.iter().find(|r| r.text == "title").unwrap();
    let italic = runs.iter().find(|r| r.text == "note").unwrap();
    let small = runs.iter().find(|r| r.text == "fine").unwrap();
    assert!(bold.bold && !bold.italic);
    assert!(italic.italic && !italic.bold);
    assert!((small.scale - 0.8).abs() < 0.001);
}

#[test]
fn tags_nest() {
    let rich = parse_markup("<b><i>both</i></b>").unwrap();
    let run = &rich.lines[0][0];
    assert!(run.bold && run.italic);
}

#[test]
fn span_carries_a_fill_colour() {
    let rich = parse_markup(r#"<span fill=accent-dark>x</span>"#).unwrap();
    assert_eq!(rich.lines[0][0].fill.as_deref(), Some("accent-dark"));
    let quoted = parse_markup(r#"<span fill="accent-dark">x</span>"#).unwrap();
    assert_eq!(quoted.lines[0][0].fill.as_deref(), Some("accent-dark"));
}

#[test]
fn a_less_than_sign_is_literal_text() {
    // The motivating diagram is full of these: "T < confirmed_until".
    // Treating a bare < as markup would break real labels.
    assert_eq!(
        lines_of("T < confirmed_until"),
        vec!["T < confirmed_until"]
    );
    assert_eq!(lines_of("a <= b"), vec!["a <= b"]);
    assert_eq!(lines_of("Vec<String>"), vec!["Vec<String>"]);
}

#[test]
fn an_unclosed_recognised_tag_is_an_error() {
    // Silently dropping it would render plausible-but-wrong output, which is
    // the failure mode this whole feature exists to remove.
    let err = parse_markup("<b>title").unwrap_err();
    assert!(err.message.contains("<b>"), "got: {}", err.message);
}

#[test]
fn a_span_without_a_fill_is_an_error() {
    let err = parse_markup("<span>x</span>").unwrap_err();
    assert!(err.message.contains("fill"), "got: {}", err.message);
}

#[test]
fn entities_are_unescaped() {
    assert_eq!(lines_of("a &lt; b &amp; c"), vec!["a < b & c"]);
}

#[test]
fn plain_flattening_drops_the_markup() {
    let rich = parse_markup("<b>changeset</b><br><small>v2</small>").unwrap();
    assert_eq!(rich.plain(), "changeset v2");
}

#[test]
fn round_trips_through_from_plain() {
    assert_eq!(RichText::from_plain("x").plain(), "x");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test label_markup`
Expected: FAIL — `cannot find function parse_markup`.

- [ ] **Step 3: Write the implementation**

Append to `src/layout/text.rs`:

```rust
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
        MarkupStyle { bold: false, italic: false, scale: 1.0, fill: None }
    }
}

/// Parse the label markup subset into lines of styled runs.
///
/// `<` only begins a tag when a recognised tag name follows it; otherwise it
/// is literal text, so `"T < confirmed_until"` and `"Vec<String>"` survive.
pub fn parse_markup(src: &str) -> Result<RichText, MarkupError> {
    let bytes: Vec<char> = src.chars().collect();
    let mut lines: Vec<TextLine> = vec![Vec::new()];
    let mut stack: Vec<(String, MarkupStyle)> = Vec::new();
    let mut style = MarkupStyle::root();
    let mut buf = String::new();
    let mut i = 0usize;

    // Flush the pending characters as a run under the current style.
    fn flush(buf: &mut String, style: &MarkupStyle, lines: &mut Vec<TextLine>) {
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

    while i < bytes.len() {
        let c = bytes[i];

        if c == '\n' {
            flush(&mut buf, &style, &mut lines);
            lines.push(Vec::new());
            i += 1;
            continue;
        }

        if c == '&' {
            if let Some((entity, len)) = read_entity(&bytes[i..]) {
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
        match read_tag(&bytes[i..]) {
            None => {
                buf.push('<');
                i += 1;
            }
            Some(tag) => {
                flush(&mut buf, &style, &mut lines);
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

    flush(&mut buf, &style, &mut lines);

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
    let name: String = chars[name_start..i].iter().collect::<String>().to_ascii_lowercase();
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

    Some(ParsedTag { name, closing, fill, consumed: i + 1 })
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test label_markup`
Expected: PASS, 12 tests.

Then `cargo test` — expected: all green, nothing consumes the parser yet.

- [ ] **Step 5: Commit**

```bash
git add src/layout/text.rs tests/label_markup.rs
git commit -m "feat(text): label markup subset (<br> <b> <i> <small> <span fill>)

Parses to styled runs, so we keep measurement in our own hands. A bare
'<' stays literal — 'T < confirmed_until' and 'Vec<String>' are real
labels. A malformed recognised tag is an error rather than silently
dropped output.

Not wired up yet."
```

---

### Task 3: Greedy word wrap

**Files:**
- Modify: `src/layout/text.rs` (append)
- Test: `tests/label_markup.rs` (append)

**Interfaces:**
- Consumes: `RichText`, `TextLine`, `measure_runs` from Tasks 1–2.
- Produces: `pub fn wrap(rich: &RichText, font_size: f64, max_width: f64) -> RichText`

- [ ] **Step 1: Write the failing test**

Append to `tests/label_markup.rs`:

```rust
use agent_illustrator::layout::text::{measure_runs, wrap};

#[test]
fn wrapping_breaks_at_spaces_to_fit_the_width() {
    let rich = parse_markup("the quick brown fox jumps over the lazy dog").unwrap();
    let wrapped = wrap(&rich, 14.0, 120.0);
    assert!(wrapped.lines.len() > 1, "expected several lines");
    let m = measure_runs(&wrapped.lines, 14.0);
    assert!(m.width <= 120.0, "wrapped width {} exceeds 120", m.width);
}

#[test]
fn wrapping_preserves_the_words() {
    let rich = parse_markup("the quick brown fox jumps over the lazy dog").unwrap();
    let wrapped = wrap(&rich, 14.0, 120.0);
    assert_eq!(wrapped.plain().split_whitespace().collect::<Vec<_>>(),
               "the quick brown fox jumps over the lazy dog".split(' ').collect::<Vec<_>>());
}

#[test]
fn an_explicit_break_survives_wrapping() {
    let rich = parse_markup("alpha<br>beta").unwrap();
    let wrapped = wrap(&rich, 14.0, 9999.0);
    assert_eq!(wrapped.lines.len(), 2);
}

#[test]
fn a_single_unbreakable_word_overflows_rather_than_being_hyphenated() {
    // Breaking mid-word needs hyphenation rules we do not have; lint reports
    // the overflow instead.
    let rich = parse_markup("supercalifragilistic").unwrap();
    let wrapped = wrap(&rich, 14.0, 20.0);
    assert_eq!(wrapped.lines.len(), 1);
}

#[test]
fn wrapping_keeps_run_styling_across_the_break() {
    let rich = parse_markup("<b>alpha beta gamma delta epsilon</b>").unwrap();
    let wrapped = wrap(&rich, 14.0, 60.0);
    assert!(wrapped.lines.len() > 1);
    assert!(
        wrapped.lines.iter().all(|l| l.iter().all(|r| r.bold)),
        "bold should survive the wrap"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test label_markup`
Expected: FAIL — `cannot find function wrap`.

- [ ] **Step 3: Write the implementation**

Append to `src/layout/text.rs`:

```rust
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
                let styled = TextRun { text: piece.clone(), ..run.clone() };
                let w = styled.width(font_size);

                if !current.is_empty() && current_width + w > max_width {
                    out.push(std::mem::take(&mut current));
                    current_width = 0.0;
                    // The word starts a fresh line, so drop its leading space.
                    let trimmed = TextRun {
                        text: piece.trim_start().to_string(),
                        ..run.clone()
                    };
                    current_width += trimmed.width(font_size);
                    current.push(trimmed);
                } else {
                    current_width += w;
                    current.push(styled);
                }
            }
        }

        out.push(current);
    }

    RichText { lines: out }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test label_markup`
Expected: PASS, 17 tests. Then `cargo test` — all green.

- [ ] **Step 5: Commit**

```bash
git add src/layout/text.rs tests/label_markup.rs
git commit -m "feat(text): greedy word wrap that keeps run styling

A word wider than the line overflows rather than hyphenating; lint
reports it by name."
```

---

### Task 4: Rewire every measurement site

**Files:**
- Modify: `src/layout/engine.rs:1030-1035` (label min width), `src/layout/engine.rs:1055` (text shape), `src/layout/engine.rs:1966`, `src/layout/engine.rs:2105`, `src/layout/engine.rs:2166` (grid gutters)
- Modify: `src/layout/types.rs:1055-1057` (`estimate_label_width`)
- Modify: `src/layout/lint.rs:979` (`estimate_label_bbox`)
- Modify: `src/layout/keyframe.rs:515`
- Modify: `src/layout/routing.rs:1097`
- Test: `tests/text_measurement.rs` (append)

**Interfaces:**
- Consumes: `measure_str` from Task 1.
- Produces: no new public API. After this task, `src/layout/` and `src/renderer/` contain **zero** occurrences of `.len() as f64 *` for text.

- [ ] **Step 1: Write the failing test**

Append to `tests/text_measurement.rs`:

```rust
use agent_illustrator::{render_with_lint, RenderConfig};

#[test]
fn a_label_that_fits_its_box_is_not_reported_as_overflowing() {
    // The box was sized at 8.0px/char and checked at 8.4px/char, so labels
    // that fit were reported as straddling. One measurer, one answer.
    let source = r#"rect card [label: "temporal_mode = correction"]"#;
    let (_svg, warnings) =
        render_with_lint(source, RenderConfig::new().with_lint(true)).expect("renders");
    assert!(
        warnings.is_empty(),
        "an auto-sized box must fit its own label, got: {:?}",
        warnings.iter().map(|w| w.message.clone()).collect::<Vec<_>>()
    );
}

#[test]
fn a_label_full_of_maths_symbols_does_not_inflate_its_box() {
    // Each ≤ is 3 UTF-8 bytes; the old sites measured it as 3 characters.
    let narrow = render_str(r#"rect a [label: "T <= now"]"#);
    let symbolic = render_str(r#"rect a [label: "T ≤ now"]"#);
    let width_of = |svg: &str| -> f64 {
        let at = svg.find("width=\"").expect("a width attribute");
        svg[at + 7..]
            .split('"')
            .next()
            .unwrap()
            .parse()
            .unwrap_or(0.0)
    };
    assert!(
        width_of(&symbolic) <= width_of(&narrow) + 1.0,
        "≤ should not measure wider than <=: {} vs {}",
        width_of(&symbolic),
        width_of(&narrow)
    );
}

fn render_str(source: &str) -> String {
    agent_illustrator::render(source).expect("renders")
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test text_measurement`
Expected: FAIL on at least `a_label_that_fits_its_box_is_not_reported_as_overflowing`.

- [ ] **Step 3: Replace each site**

In `src/layout/engine.rs`, add near the other imports:

```rust
use crate::layout::text::{measure_str, LINE_HEIGHT_FACTOR};
```

Replace the label minimum width (currently `engine.rs:1030-1035`):

```rust
    // Minimum width needed to fit the label, measured the same way lint
    // checks it.
    let label_min_width = extract_label(&shape.modifiers).map(|text| {
        let font_size = extract_font_size(&shape.modifiers).unwrap_or(14.0);
        measure_str(&text, font_size) + 2.0 * LABEL_INSET
    });
```

Replace the text-shape estimate (currently `engine.rs:1055`):

```rust
            let estimated_width = measure_str(content, font_size);
            (estimated_width.max(20.0), font_size)
```

Replace the three grid gutter sites (`engine.rs:1966`, `:2105`, `:2166`), each of the form `s.len() as f64 * GRID_LABEL_FONT * 0.6`:

```rust
    let w = measure_str(text, GRID_LABEL_FONT);
```

```rust
                .map(|s| measure_str(s, GRID_LABEL_FONT))
```

```rust
            let tw = measure_str(text, GRID_LABEL_FONT);
```

In `src/layout/types.rs`, replace `estimate_label_width` (`:1054-1057`):

```rust
/// Estimate the width of a text label.
fn estimate_label_width(text: &str) -> f64 {
    crate::layout::text::measure_str(text, 14.0)
}
```

In `src/layout/lint.rs`, replace the width line in `estimate_label_bbox` (`:979`):

```rust
    let width = crate::layout::text::measure_str(&label.text, font_size);
```

In `src/layout/keyframe.rs`, replace `:515`:

```rust
    crate::layout::text::measure_str(text, font_size).max(20.0)
```

In `src/layout/routing.rs`, replace `:1097`:

```rust
                let width = crate::layout::text::measure_str(&label.text, 14.0) + PADDING * 2.0;
```

Leave `CHAR_WIDTH` defined only if other code still uses it; otherwise delete the constant so it cannot drift back.

- [ ] **Step 4: Run the tests and re-render examples**

Run: `cargo test`
Expected: PASS. Some layout assertions may hold hard-coded pixel widths that assumed 8.0px/char — update those assertions to the new measured value (the new number is the correct one; the test was pinning the old estimator, not a behaviour). Do **not** loosen an assertion to a range to make it pass.

Then re-render every example, because auto-sized boxes change size:

Run: `bash examples/render-all.sh`
Run: `cargo test --test svg_regression`
Expected: PASS — the regression test compares structure, not bytes.

Verify no byte-length text measurement survives:

Run: `grep -rn "len() as f64" src/layout src/renderer`
Expected: no hits on text measurement.

- [ ] **Step 5: Commit**

```bash
git add src/layout/engine.rs src/layout/types.rs src/layout/lint.rs \
        src/layout/keyframe.rs src/layout/routing.rs \
        tests/text_measurement.rs examples/
git commit -m "fix(text): measure text in one place, by character not by byte

Sizing used 8.0px/char, lint used 8.4, bounds used 7.0, so lint reported
overflow on labels that fit. All eight sites now call measure_str.

Example SVGs re-rendered: auto-sized boxes change slightly."
```

---

### Task 5: Multi-line labels end to end

**Files:**
- Modify: `src/layout/types.rs:858-864` (`LabelLayout`), `src/layout/types.rs:1060-1086` (`expand_bounds_for_label`)
- Modify: `src/layout/engine.rs:900-975` (label construction), `engine.rs:1028-1040` (shape sizing)
- Modify: `src/renderer/svg.rs:404-442` (`add_text_with_classes`)
- Test: `tests/label_markup.rs` (append)

**Interfaces:**
- Consumes: `parse_markup`, `measure_runs`, `RichText`, `LINE_HEIGHT_FACTOR`.
- Produces: `LabelLayout` gains `pub rich: RichText` and `pub font_size: f64`. `LabelLayout.text` stays as the plain-text flattening (`rich.plain()`) so lint messages and keyframe diffs are unchanged.

- [ ] **Step 1: Write the failing test**

Append to `tests/label_markup.rs`:

```rust
use agent_illustrator::render;

#[test]
fn a_multiline_label_renders_one_tspan_per_line() {
    let svg = render(r#"rect card [label: "alpha<br>beta<br>gamma"]"#).expect("renders");
    assert_eq!(svg.matches("<tspan").count(), 3, "svg was: {svg}");
    assert!(svg.contains("alpha") && svg.contains("beta") && svg.contains("gamma"));
}

#[test]
fn a_box_grows_in_height_to_fit_its_lines() {
    let one = render(r#"rect card [label: "alpha"]"#).expect("renders");
    let three = render(r#"rect card [label: "alpha<br>beta<br>gamma"]"#).expect("renders");
    let height_of = |svg: &str| -> f64 {
        let at = svg.find("height=\"").expect("a height attribute");
        svg[at + 8..].split('"').next().unwrap().parse().unwrap_or(0.0)
    };
    assert!(
        height_of(&three) > height_of(&one),
        "three lines ({}) should be taller than one ({})",
        height_of(&three),
        height_of(&one)
    );
}

#[test]
fn a_box_grows_in_width_to_fit_its_longest_line() {
    let svg = render(r#"rect card [label: "a<br>a much longer second line"]"#).expect("renders");
    assert!(svg.contains("<tspan"));
    // The rect must be at least as wide as the long line.
    let width = svg[svg.find("width=\"").unwrap() + 7..]
        .split('"').next().unwrap().parse::<f64>().unwrap();
    assert!(width > 100.0, "expected the box to widen, got {width}");
}

#[test]
fn bold_runs_reach_the_svg() {
    let svg = render(r#"rect card [label: "<b>title</b>"]"#).expect("renders");
    assert!(svg.contains("font-weight"), "svg was: {svg}");
}

#[test]
fn a_span_fill_reaches_the_svg() {
    let svg = render(r#"rect card [label: "<span fill=red>x</span>"]"#).expect("renders");
    assert!(svg.contains("red"), "svg was: {svg}");
}

#[test]
fn malformed_markup_is_a_render_error_not_silent_output() {
    let err = render(r#"rect card [label: "<b>title"]"#);
    assert!(err.is_err(), "unclosed <b> should not render");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test label_markup`
Expected: FAIL — no `<tspan>` in the output.

- [ ] **Step 3: Write the implementation**

In `src/layout/types.rs`, extend `LabelLayout`:

```rust
pub struct LabelLayout {
    /// The wording with markup stripped. Lint messages and keyframe diffs
    /// compare wording, not styling, so they keep reading this.
    pub text: String,
    /// The parsed lines of styled runs, as rendered.
    pub rich: crate::layout::text::RichText,
    /// Base font size the runs were measured at.
    pub font_size: f64,
    pub position: Point,
    pub anchor: TextAnchor,
    /// Optional styles for the label (used when referencing a styled element)
    pub styles: Option<ResolvedStyles>,
}
```

Fix the other two construction sites the compiler will flag (`routing.rs:1394`, `lint.rs:3117`) by adding:

```rust
            rich: crate::layout::text::RichText::from_plain(&text),
            font_size: 14.0,
```

In `types.rs`, make `expand_bounds_for_label` use the real metrics:

```rust
fn expand_bounds_for_label(bounds: BoundingBox, label: &LabelLayout) -> BoundingBox {
    let metrics = crate::layout::text::measure_runs(&label.rich.lines, label.font_size);
    let (estimated_width, estimated_height) = (metrics.width, metrics.height);

    // ... the existing anchor match and union, unchanged below this point ...
```

In `src/layout/engine.rs`, parse the label once and carry it through. Replace the `extract_label(...).map(|text| { ... })` label construction (around `:900-975`) so it starts:

```rust
    let label = match extract_label(&shape.modifiers) {
        None => None,
        Some(raw) => {
            let font_size = extract_font_size(&shape.modifiers).unwrap_or(14.0);
            let rich = crate::layout::text::parse_markup(&raw).map_err(|e| {
                LayoutError::invalid_label(shape.shape_type.span.clone(), e.message)
            })?;
            // ... existing position/anchor computation, unchanged ...
            Some(LabelLayout {
                text: rich.plain(),
                rich,
                font_size,
                position: Point::new(label_x, label_y),
                anchor,
                styles: None,
            })
        }
    };
```

Add the error constructor in `src/layout/error.rs`, following the pattern of the constructors already there:

```rust
    pub fn invalid_label(span: std::ops::Range<usize>, message: String) -> Self {
        LayoutError::new(format!("invalid label markup: {message}"), span)
    }
```

In `compute_shape_size`, size from the parsed lines rather than the raw string:

```rust
    let label_metrics = extract_label(&shape.modifiers).and_then(|text| {
        let font_size = extract_font_size(&shape.modifiers).unwrap_or(14.0);
        crate::layout::text::parse_markup(&text)
            .ok()
            .map(|rich| crate::layout::text::measure_runs(&rich.lines, font_size))
    });
    let label_min_width = label_metrics.map(|m| m.width + 2.0 * LABEL_INSET);
    let label_min_height = label_metrics.map(|m| m.height + 2.0 * LABEL_INSET);
```

and apply the height the same way the width is applied, right after the existing `final_width` block:

```rust
    let mut final_height = match (height, label_min_height) {
        (Some(h), _) => h,
        (None, Some(min)) => default_height.max(min),
        (None, None) => default_height,
    };
```

In `src/renderer/svg.rs`, replace the body of `add_text_with_classes` so it takes the rich text. Change its signature to accept `&RichText` plus the base font size, update the two call sites in `render_element`/connection rendering to pass `&label.rich` and `label.font_size`, and emit:

```rust
        let lines = &rich.lines;
        let line_height = font_size * crate::layout::text::LINE_HEIGHT_FACTOR;
        // Vertically centre n lines about y: the first baseline sits above it.
        let first_dy = -((lines.len() as f64 - 1.0) / 2.0) * line_height;

        let mut body = String::new();
        for (i, line) in lines.iter().enumerate() {
            let dy = if i == 0 { first_dy } else { line_height };
            body.push_str(&format!(r#"<tspan x="{x}" dy="{dy}">"#));
            for run in line {
                let mut attrs = String::new();
                if run.bold {
                    attrs.push_str(r#" font-weight="bold""#);
                }
                if run.italic {
                    attrs.push_str(r#" font-style="italic""#);
                }
                if (run.scale - 1.0).abs() > f64::EPSILON {
                    attrs.push_str(&format!(r#" font-size="{}""#, font_size * run.scale));
                }
                if let Some(fill) = &run.fill {
                    attrs.push_str(&format!(r#" fill="{}""#, escape_xml(fill)));
                }
                body.push_str(&format!("<tspan{}>{}</tspan>", attrs, escape_xml(&run.text)));
            }
            body.push_str("</tspan>");
        }

        self.elements.push(format!(
            r#"{}<text class="{}label{}" x="{}" y="{}" text-anchor="{}" dominant-baseline="middle"{}>{}</text>"#,
            self.indent_str(), prefix, extra, x, y, anchor_str, styles, body
        ));
```

Keep `add_text(text, …)` as a thin wrapper that calls `RichText::from_plain(text)`, so debug rects and grid gutters need no changes.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test label_markup`
Expected: PASS, 23 tests.

Run: `cargo test`
Expected: PASS. Re-render examples (`bash examples/render-all.sh`) and re-run `cargo test --test svg_regression`.

Eyeball one render:

```bash
printf 'rect card [label: "<b>changeset from INES</b><br>temporal_mode = correction<br><small>V2 inherits its label</small>"]\n' \
  | ./target/debug/agent-illustrator --lint > /tmp/card.svg
```

Expected: three visually distinct lines inside a box that grew to hold them; no lint warnings.

- [ ] **Step 5: Commit**

```bash
git add src/layout/types.rs src/layout/engine.rs src/layout/error.rs \
        src/layout/routing.rs src/layout/lint.rs src/renderer/svg.rs \
        tests/label_markup.rs examples/
git commit -m "feat(label): multi-line labels rendered as tspans

A three-line card is one modifier instead of three text elements and
three constraint pairs. The box grows in both dimensions to hold its
lines."
```

---

### Task 6: `label_position:` on shapes

**Files:**
- Modify: `src/layout/engine.rs:955-975` (label placement)
- Modify: `src/layout/types.rs` (`expand_bounds_for_label` already handles the union; verify outside positions)
- Test: `tests/label_position.rs`

**Interfaces:**
- Consumes: `LabelLayout` (Task 5), `parse_align` (`types.rs:586`).
- Produces: `enum ShapeLabelPosition { Inside, Above, Below, Left, Right }` in `engine.rs` with `fn extract_label_position(modifiers) -> ShapeLabelPosition`, defaulting to `Inside`.

- [ ] **Step 1: Write the failing test**

Create `tests/label_position.rs`:

```rust
//! "Put the caption under the box, aligned right" used to be two constraints
//! and a guessed offset. It is now one modifier.

use agent_illustrator::render;

/// Pull the y of the first `<text>` and of the first `<rect>` out of an SVG.
fn text_y(svg: &str) -> f64 {
    let at = svg.find("<text").expect("a text element");
    let y_at = svg[at..].find(" y=\"").expect("text y") + at + 4;
    svg[y_at..].split('"').next().unwrap().parse().unwrap()
}

fn rect_y(svg: &str) -> f64 {
    let at = svg.find("<rect").expect("a rect element");
    let y_at = svg[at..].find(" y=\"").expect("rect y") + at + 4;
    svg[y_at..].split('"').next().unwrap().parse().unwrap()
}

fn rect_height(svg: &str) -> f64 {
    let at = svg.find("<rect").expect("a rect element");
    let h_at = svg[at..].find(" height=\"").expect("rect height") + at + 9;
    svg[h_at..].split('"').next().unwrap().parse().unwrap()
}

#[test]
fn the_default_is_still_centred_inside() {
    let svg = render(r#"rect b [label: "x", width: 100, height: 40]"#).expect("renders");
    let centre = rect_y(&svg) + rect_height(&svg) / 2.0;
    assert!((text_y(&svg) - centre).abs() < 1.0, "{} vs {}", text_y(&svg), centre);
}

#[test]
fn below_puts_the_label_under_the_shape() {
    let svg = render(
        r#"rect b [label: "based_on", width: 100, height: 40, label_position: below]"#,
    )
    .expect("renders");
    assert!(
        text_y(&svg) > rect_y(&svg) + rect_height(&svg),
        "label y {} should be below the box bottom {}",
        text_y(&svg),
        rect_y(&svg) + rect_height(&svg)
    );
}

#[test]
fn above_puts_the_label_over_the_shape() {
    let svg = render(
        r#"rect b [label: "Zone A", width: 100, height: 40, label_position: above]"#,
    )
    .expect("renders");
    assert!(text_y(&svg) < rect_y(&svg), "{} vs {}", text_y(&svg), rect_y(&svg));
}

#[test]
fn align_end_right_aligns_a_label_below() {
    let svg = render(
        r#"rect b [label: "based_on", width: 200, height: 40, label_position: below, align: end]"#,
    )
    .expect("renders");
    assert!(svg.contains(r#"text-anchor="end""#), "svg was: {svg}");
}

#[test]
fn align_start_left_aligns_a_label_above() {
    let svg = render(
        r#"rect b [label: "Zone A", width: 200, height: 40, label_position: above, align: start]"#,
    )
    .expect("renders");
    assert!(svg.contains(r#"text-anchor="start""#), "svg was: {svg}");
}

#[test]
fn label_offset_controls_the_gap() {
    let near = render(
        r#"rect b [label: "x", width: 100, height: 40, label_position: below, label_offset: 4]"#,
    ).expect("renders");
    let far = render(
        r#"rect b [label: "x", width: 100, height: 40, label_position: below, label_offset: 40]"#,
    ).expect("renders");
    assert!(text_y(&far) > text_y(&near), "{} vs {}", text_y(&far), text_y(&near));
}

#[test]
fn an_outside_label_is_inside_the_canvas() {
    // If the label were not folded into the element's bounds it would be
    // clipped off the top of the viewBox.
    let svg = render(
        r#"rect b [label: "Zone A", width: 100, height: 40, label_position: above]"#,
    )
    .expect("renders");
    assert!(text_y(&svg) >= 0.0, "label escaped the canvas at y={}", text_y(&svg));
}

#[test]
fn a_row_reserves_space_for_labels_below_its_children() {
    // gap: should measure from the label, not from the border, or the caption
    // of one box collides with the next.
    let svg = render(
        r#"
row [gap: 10] {
    rect a [label: "first caption", width: 80, height: 40, label_position: below]
    rect b [label: "second caption", width: 80, height: 40, label_position: below]
}
"#,
    )
    .expect("renders");
    assert!(svg.contains("first caption") && svg.contains("second caption"));
}

#[test]
fn left_and_right_place_the_label_beside_the_shape() {
    let right = render(
        r#"rect b [label: "x", width: 100, height: 40, label_position: right]"#,
    ).expect("renders");
    assert!(right.contains(r#"text-anchor="start""#), "svg was: {right}");
    let left = render(
        r#"rect b [label: "x", width: 100, height: 40, label_position: left]"#,
    ).expect("renders");
    assert!(left.contains(r#"text-anchor="end""#), "svg was: {left}");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test label_position`
Expected: FAIL — `below_puts_the_label_under_the_shape`, because `label_position` is currently ignored on shapes.

- [ ] **Step 3: Write the implementation**

In `src/layout/engine.rs`, add the enum and extractors:

```rust
/// Where a shape's label sits relative to the shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShapeLabelPosition {
    Inside,
    Above,
    Below,
    Left,
    Right,
}

/// Default gap between a shape's edge and a label placed outside it.
const LABEL_OUTSIDE_OFFSET: f64 = 6.0;

fn extract_label_position(modifiers: &[Spanned<StyleModifier>]) -> ShapeLabelPosition {
    modifiers
        .iter()
        .find_map(|m| {
            if !matches!(m.node.key.node, StyleKey::LabelPosition) {
                return None;
            }
            let word = match &m.node.value.node {
                StyleValue::Keyword(k) => k.as_str(),
                StyleValue::Identifier(id) => id.0.as_str(),
                StyleValue::String(s) => s.as_str(),
                _ => return None,
            };
            match word {
                "inside" => Some(ShapeLabelPosition::Inside),
                "above" => Some(ShapeLabelPosition::Above),
                "below" => Some(ShapeLabelPosition::Below),
                "left" => Some(ShapeLabelPosition::Left),
                "right" => Some(ShapeLabelPosition::Right),
                _ => None,
            }
        })
        .unwrap_or(ShapeLabelPosition::Inside)
}

fn extract_label_offset(modifiers: &[Spanned<StyleModifier>]) -> f64 {
    modifiers
        .iter()
        .find_map(|m| {
            if matches!(m.node.key.node, StyleKey::LabelOffset) {
                match &m.node.value.node {
                    StyleValue::Number { value, .. } => Some(*value),
                    _ => None,
                }
            } else {
                None
            }
        })
        .unwrap_or(LABEL_OUTSIDE_OFFSET)
}
```

In the label construction block, after the existing inside-placement computation produces `(label_x, label_y, anchor)`, branch on the position. Replace the current `extract_align` override block with:

```rust
        let align = extract_align(&shape.modifiers);
        let offset = extract_label_offset(&shape.modifiers);
        let metrics = crate::layout::text::measure_runs(&rich.lines, font_size);

        let (label_x, label_y, anchor) = match extract_label_position(&shape.modifiers) {
            ShapeLabelPosition::Inside => {
                // An explicit `align:` overrides the horizontal placement,
                // keeping a small inset so edge-aligned text does not touch
                // the border.
                match align {
                    Some(TextAnchor::Start) => {
                        (position.x + LABEL_INSET, label_y, TextAnchor::Start)
                    }
                    Some(TextAnchor::End) => (
                        position.x + width - LABEL_INSET,
                        label_y,
                        TextAnchor::End,
                    ),
                    Some(TextAnchor::Middle) => {
                        (position.x + width / 2.0, label_y, TextAnchor::Middle)
                    }
                    None => (label_x, label_y, anchor),
                }
            }
            // Above/below: `align` picks the horizontal edge to align to.
            // The y is the vertical centre of the label block, because the
            // renderer centres n lines about it.
            ShapeLabelPosition::Above => {
                let y = position.y - offset - metrics.height / 2.0;
                match align.unwrap_or(TextAnchor::Middle) {
                    TextAnchor::Start => (position.x, y, TextAnchor::Start),
                    TextAnchor::Middle => (position.x + width / 2.0, y, TextAnchor::Middle),
                    TextAnchor::End => (position.x + width, y, TextAnchor::End),
                }
            }
            ShapeLabelPosition::Below => {
                let y = position.y + height + offset + metrics.height / 2.0;
                match align.unwrap_or(TextAnchor::Middle) {
                    TextAnchor::Start => (position.x, y, TextAnchor::Start),
                    TextAnchor::Middle => (position.x + width / 2.0, y, TextAnchor::Middle),
                    TextAnchor::End => (position.x + width, y, TextAnchor::End),
                }
            }
            // Left/right: `align` picks the vertical edge. parse_align already
            // maps top -> Start and bottom -> End.
            ShapeLabelPosition::Left => {
                let y = match align.unwrap_or(TextAnchor::Middle) {
                    TextAnchor::Start => position.y + metrics.height / 2.0,
                    TextAnchor::Middle => position.y + height / 2.0,
                    TextAnchor::End => position.y + height - metrics.height / 2.0,
                };
                (position.x - offset, y, TextAnchor::End)
            }
            ShapeLabelPosition::Right => {
                let y = match align.unwrap_or(TextAnchor::Middle) {
                    TextAnchor::Start => position.y + metrics.height / 2.0,
                    TextAnchor::Middle => position.y + height / 2.0,
                    TextAnchor::End => position.y + height - metrics.height / 2.0,
                };
                (position.x + width + offset, y, TextAnchor::Start)
            }
        };
```

`expand_bounds_for_label` (Task 5) already unions the label box into the element bounds using the anchor and the real metrics, which is what makes the canvas, `row`/`col` gaps, and `contains` account for an outside label. Confirm the vertical union uses the metrics height rather than the old hard-coded `14.0`, and that it centres about `position.y` rather than treating it as a baseline:

```rust
    let label_top = label.position.y - estimated_height / 2.0;
    let label_bottom = label.position.y + estimated_height / 2.0;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test label_position`
Expected: PASS, 9 tests.

Run: `cargo test` — expected PASS. Re-render examples and re-run `svg_regression`.

- [ ] **Step 5: Commit**

```bash
git add src/layout/engine.rs src/layout/types.rs tests/label_position.rs examples/
git commit -m "feat(label): label_position inside|above|below|left|right on shapes

'Caption under the box, aligned right' becomes one modifier instead of
two constraints and a guessed offset. Outside labels join the element's
bounds, so rows reserve space for them and contains surrounds them.

No new grammar keywords: label_position, label_offset and align all
already parsed."
```

---

### Task 7: An explicit width wraps instead of overflowing

**Files:**
- Modify: `src/layout/engine.rs` (`compute_shape_size`, label construction)
- Test: `tests/label_markup.rs` (append)

**Interfaces:**
- Consumes: `wrap` (Task 3), the sizing path (Task 5).
- Produces: no new public API.

- [ ] **Step 1: Write the failing test**

Append to `tests/label_markup.rs`:

```rust
#[test]
fn an_explicit_width_wraps_the_label_instead_of_overflowing() {
    // The motivating failure: rect card [width: 300, label: "..."] drew the
    // text straight out through both borders.
    let svg = render(
        r#"rect card [width: 200, label: "T <= now confirmed_from <= T < confirmed_until"]"#,
    )
    .expect("renders");
    assert!(svg.matches("<tspan").count() >= 2, "expected a wrap, svg was: {svg}");
}

#[test]
fn an_explicit_width_and_a_wrapped_label_grows_the_height() {
    let short = render(r#"rect card [width: 200, label: "short"]"#).expect("renders");
    let long = render(
        r#"rect card [width: 200, label: "T <= now confirmed_from <= T < confirmed_until"]"#,
    ).expect("renders");
    let h = |svg: &str| -> f64 {
        let at = svg.find("<rect").unwrap();
        let ha = svg[at..].find(" height=\"").unwrap() + at + 9;
        svg[ha..].split('"').next().unwrap().parse().unwrap()
    };
    assert!(h(&long) > h(&short), "{} vs {}", h(&long), h(&short));
}

#[test]
fn a_fully_explicit_box_keeps_its_size_and_lint_reports_the_overflow() {
    use agent_illustrator::{render_with_lint, RenderConfig};
    let source =
        r#"rect card [width: 120, height: 30, label: "a very long label that cannot possibly fit"]"#;
    let (_svg, warnings) =
        render_with_lint(source, RenderConfig::new().with_lint(true)).expect("renders");
    assert!(
        warnings.iter().any(|w| w.message.contains("card")),
        "expected an overflow warning naming the box, got: {:?}",
        warnings.iter().map(|w| w.message.clone()).collect::<Vec<_>>()
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test label_markup`
Expected: FAIL — only one `<tspan>`; the label overflows.

- [ ] **Step 3: Write the implementation**

In `src/layout/engine.rs`, add a helper that produces the label as it will actually be laid out, and use it in both `compute_shape_size` and the label construction so the two never disagree:

```rust
/// The label of a shape, parsed and — when the shape has an explicit width —
/// wrapped to fit inside it.
///
/// Sizing and rendering both go through this, so a box is never sized for one
/// set of lines and drawn with another.
fn resolve_shape_label(
    shape: &ShapeDecl,
) -> Result<Option<(crate::layout::text::RichText, f64)>, LayoutError> {
    let Some(raw) = extract_label(&shape.modifiers) else {
        return Ok(None);
    };
    let font_size = extract_font_size(&shape.modifiers).unwrap_or(14.0);
    let rich = crate::layout::text::parse_markup(&raw)
        .map_err(|e| LayoutError::invalid_label(shape.shape_type.span.clone(), e.message))?;

    // An explicit width is a promise about the box, so the text wraps to it
    // rather than running out through the borders.
    let rich = match extract_width_modifier(&shape.modifiers) {
        Some(w) => crate::layout::text::wrap(&rich, font_size, w - 2.0 * LABEL_INSET),
        None => rich,
    };
    Ok(Some((rich, font_size)))
}
```

Change `compute_shape_size` to take the resolved label rather than re-reading the modifiers:

```rust
fn compute_shape_size(
    shape: &ShapeDecl,
    config: &LayoutConfig,
    label: Option<&(crate::layout::text::RichText, f64)>,
) -> (f64, f64) {
```

and inside it replace the `label_metrics` block from Task 5 with:

```rust
    let label_metrics =
        label.map(|(rich, font_size)| crate::layout::text::measure_runs(&rich.lines, *font_size));
```

Keep the early return for `(Some(w), Some(h))` — a fully explicit box stays exactly as written, and lint reports the overflow. Height growth applies when `height` is absent, exactly as in Task 5.

At the shape-layout call site, resolve once and pass it to both:

```rust
    let resolved_label = resolve_shape_label(shape)?;
    let (width, height) = compute_shape_size(shape, config, resolved_label.as_ref());
```

and have the label construction use `resolved_label` instead of calling `parse_markup` again.

Update the other `compute_shape_size` call sites the compiler flags to pass `None` where they have no label in hand, or to resolve it the same way.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test label_markup`
Expected: PASS, 26 tests.

Run: `cargo test` — expected PASS. Re-render examples, re-run `svg_regression`.

Reproduce the original failure to confirm it is gone:

```bash
printf 'rect card [width: 300, label: "T <= now confirmed_from <= T < confirmed_until"]\n' \
  | ./target/debug/agent-illustrator --lint > /tmp/wrap.svg
```

Expected: the text wraps inside the 300px box; no warnings.

- [ ] **Step 5: Commit**

```bash
git add src/layout/engine.rs tests/label_markup.rs examples/
git commit -m "feat(label): an explicit width wraps the label, not overflows it

rect card [width: 300, label: \"...\"] used to draw straight out through
both borders, because the label minimum was only applied when width was
absent. It now wraps to the width and grows in height.

Sizing and rendering share one resolve_shape_label, so a box is never
sized for one set of lines and drawn with another."
```

---

### Task 8: Make the linter worth reading

**Files:**
- Modify: `src/layout/lint.rs` — `check_contains_in_stmts` (containment epsilon), `dedup_warnings:266`, plus two new checks
- Test: `tests/lint_label_guidance.rs`

**Interfaces:**
- Consumes: `SUPPORTED_TAGS` (Task 2), `LintWarning`, `LintCategory`.
- Produces: no new public API; new warning messages only.

- [ ] **Step 1: Write the failing test**

Create `tests/lint_label_guidance.rs`:

```rust
//! Agents ignore the linter because it warns on correct code and describes
//! two facts four times. Each warning here must be rare and must name its fix.

use agent_illustrator::{render_with_lint, RenderConfig};

fn lint(source: &str) -> Vec<String> {
    let (_svg, warnings) =
        render_with_lint(source, RenderConfig::new().with_lint(true)).expect("renders");
    warnings.iter().map(|w| w.message.clone()).collect()
}

#[test]
fn the_contains_idiom_produces_no_warnings() {
    // This reported `extends 0px past left edge` — a float-rounding
    // violation on correct code, which is how agents learned to ignore lint.
    let source = r#"
group diagram {
  rect card [fill: accent-light, stroke: accent-dark, stroke_width: 2]
  col lines [gap: 6] {
    text "changeset from INES" t1 [font_size: 14]
    text "temporal_mode = correction" t2 [font_size: 12]
  }
}
constrain lines.center_x = 300
constrain lines.center_y = 100
constrain card contains lines [padding: 14]
"#;
    assert!(lint(source).is_empty(), "got: {:?}", lint(source));
}

#[test]
fn text_positioned_inside_a_box_is_told_to_use_a_label() {
    // The motivating failure, and the warning that replaces four others.
    let source = r#"
rect card [width: 300, height: 100, fill: accent-light, stroke: accent-dark]
text "temporal_mode = correction" t2 [font_size: 12]
constrain t2.center_x = card.center_x
constrain t2.center_y = card.center_y
"#;
    let messages = lint(source);
    assert!(
        messages.iter().any(|m| m.contains("card") && m.contains("label:")),
        "expected a warning naming the fix, got: {messages:?}"
    );
}

#[test]
fn a_pair_is_reported_once_not_as_overlap_and_straddle() {
    let source = r#"
rect card [width: 100, height: 40, fill: accent-light, stroke: accent-dark]
text "a label far too wide for that little box" t [font_size: 12]
constrain t.center_x = card.center_x
constrain t.center_y = card.center_y
"#;
    let messages = lint(source);
    let mentioning_both = messages
        .iter()
        .filter(|m| m.contains("\"card\"") && m.contains("\"t\""))
        .count();
    assert_eq!(
        mentioning_both, 1,
        "the same geometry should be reported once, got: {messages:?}"
    );
}

#[test]
fn unsupported_markup_in_a_label_is_reported() {
    let messages = lint(r#"rect a [label: "<bold>x</bold>"]"#);
    assert!(
        messages.iter().any(|m| m.contains("<bold>") && m.contains("<b>")),
        "expected the supported set to be listed, got: {messages:?}"
    );
}

#[test]
fn a_less_than_sign_in_a_label_is_not_reported_as_markup() {
    assert!(
        lint(r#"rect a [label: "T < confirmed_until"]"#).is_empty(),
        "got: {:?}",
        lint(r#"rect a [label: "T < confirmed_until"]"#)
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test lint_label_guidance`
Expected: FAIL on all five.

- [ ] **Step 3: Write the implementation**

**(a) Containment epsilon.** In `src/layout/lint.rs`, in the containment check inside `check_contains_in_stmts`, introduce and apply a tolerance so sub-pixel solver residue is not a violation:

```rust
/// Containment violations below this are solver float residue, not a mistake.
const CONTAINMENT_EPSILON: f64 = 0.5;
```

and guard each of the four edge comparisons with `if overshoot > CONTAINMENT_EPSILON`.

**(b) Merge duplicates.** Extend `dedup_warnings` (`:266`) so that when the same unordered element pair yields both an `Overlap` and a `Label` warning, only the more specific `Label` one survives:

```rust
fn dedup_warnings(warnings: &mut Vec<LintWarning>) {
    // ... existing exact-duplicate removal ...

    // The same geometry reported twice — once as an overlap, once as a label
    // straddle — reads as two problems. Keep the label one: it names the text.
    let labelled: std::collections::HashSet<(String, String)> = warnings
        .iter()
        .filter(|w| matches!(w.category, LintCategory::Label))
        .filter_map(|w| w.pair.clone())
        .collect();
    warnings.retain(|w| {
        !(matches!(w.category, LintCategory::Overlap)
            && w.pair.as_ref().is_some_and(|p| labelled.contains(p)))
    });
}
```

This needs the element pair to be recoverable. Add `pub pair: Option<(String, String)>` to `LintWarning` (`:21`), defaulting to `None`, and populate it in `overlap_warning` (`:553`) and in the label-straddle warning, storing the two names sorted so the pair is order-independent.

**(c) Text-inside-a-box guidance.** Add a check, called from `check` (`:125`):

```rust
/// A `text` element positioned inside a filled shape is almost always a label
/// that was hand-placed. Say so, and say what to write instead — agents act on
/// a warning that contains the replacement syntax far more often than on one
/// that only describes the symptom.
fn check_hand_placed_labels(
    result: &LayoutResult,
    doc: &Document,
    warnings: &mut Vec<LintWarning>,
) {
    let contains = collect_contains_ids(doc);
    let texts = collect_visible_elements(result)
        .into_iter()
        .filter(|e| is_text_shape(e))
        .collect::<Vec<_>>();

    for shape in collect_opaque_elements(result) {
        for text in &texts {
            if contains.wraps(shape.id.as_deref(), text.id.as_deref()) {
                continue; // a deliberate contains relationship
            }
            if !shape.bounds.contains_box(&text.bounds) {
                continue;
            }
            let (Some(shape_id), Some(text_id)) = (&shape.id, &text.id) else {
                continue;
            };
            warnings.push(LintWarning {
                category: LintCategory::Label,
                message: format!(
                    "text \"{text_id}\" sits inside \"{shape_id}\" — write \
                     {shape_id} [label: \"…\"] instead of positioning it \
                     (use <br> for more lines, label_position: below for a caption)"
                ),
                pair: Some(sorted_pair(shape_id, text_id)),
                ..LintWarning::default()
            });
        }
    }
}
```

Add `BoundingBox::contains_box` in `src/layout/types.rs` if it does not already exist:

```rust
    /// True when `other` lies entirely within this box.
    pub fn contains_box(&self, other: &BoundingBox) -> bool {
        other.x >= self.x
            && other.y >= self.y
            && other.x + other.width <= self.x + self.width
            && other.y + other.height <= self.y + self.height
    }
```

**(d) Unsupported markup.** Add a check that scans each label's raw source for `<name>` where `name` is alphabetic and not in `SUPPORTED_TAGS`:

```rust
/// `<bold>` parses as literal text and renders as the characters `<bold>`.
/// That is a plausible-looking wrong picture, which is exactly what this
/// feature exists to prevent.
fn check_label_markup(doc: &Document, warnings: &mut Vec<LintWarning>) {
    for (owner, raw) in collect_raw_labels(doc) {
        for tag in unrecognised_tags(&raw) {
            warnings.push(LintWarning {
                category: LintCategory::Label,
                message: format!(
                    "label on \"{owner}\" contains unsupported markup <{tag}>; \
                     supported: <br> <b> <i> <small> <span fill=…>"
                ),
                ..LintWarning::default()
            });
        }
    }
}

/// Tag-shaped runs in a label that the markup parser does not recognise.
/// A bare `<` with no `>` after it, or `<` followed by a non-letter, is
/// ordinary text like "a < b" and is not reported.
fn unrecognised_tags(raw: &str) -> Vec<String> {
    let chars: Vec<char> = raw.chars().collect();
    let mut found = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '<' {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        if chars.get(j) == Some(&'/') {
            j += 1;
        }
        let start = j;
        while j < chars.len() && chars[j].is_ascii_alphabetic() {
            j += 1;
        }
        let name: String = chars[start..j].iter().collect();
        if !name.is_empty() && chars.get(j) == Some(&'>')
            && !crate::layout::text::SUPPORTED_TAGS.contains(&name.to_ascii_lowercase().as_str())
        {
            found.push(name);
        }
        i += 1;
    }
    found
}
```

`collect_raw_labels(doc)` walks the AST collecting `(owner_name, label_string)` for every `StyleKey::Label` modifier, mirroring the existing `collect_contains_ids_from_stmts` recursion (`:437`).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test lint_label_guidance`
Expected: PASS, 5 tests.

Run: `cargo test`
Expected: PASS. If `tests/lint_integration.rs` or `tests/lint_silent_traps.rs` asserted on a now-merged overlap warning, update the assertion to the surviving message — do not re-add the duplicate.

Re-run the original reproduction:

```bash
printf 'group d {\n rect card [width: 300, height: 100, fill: accent-light, stroke: accent-dark]\n text "temporal_mode = correction" t2 [font_size: 12]\n}\nconstrain t2.center_x = card.center_x\nconstrain t2.center_y = card.center_y\n' \
  | ./target/debug/agent-illustrator --lint > /dev/null
```

Expected: one warning, naming `card [label: "…"]`.

- [ ] **Step 5: Commit**

```bash
git add src/layout/lint.rs src/layout/types.rs tests/lint_label_guidance.rs
git commit -m "fix(lint): fewer warnings, each naming its own fix

- a 0px containment violation on correct code is float residue, not a
  mistake: apply a 0.5px epsilon
- the same pair reported as both overlap and label straddle collapses
  to the label one, which names the text
- text hand-placed inside a box now says 'write card [label: ...]'
- unsupported markup like <bold> is reported with the supported set"
```

---

### Task 9: Documentation

**Files:**
- Modify: `docs/grammar.md` (STYLE MODIFIERS section, ~line 96-125)
- Modify: `docs/skill.md` (near line 26, the existing do-this/not-that table)
- Modify: `docs/examples.md` (the architecture example, ~line 280-330)
- Test: manual — `cargo run -- --grammar`, `cargo run -- --skill`

**Interfaces:**
- Consumes: the syntax settled in Tasks 5–7.
- Produces: nothing code-facing.

- [ ] **Step 1: Update `docs/grammar.md`**

Under STYLE MODIFIERS, replace the `label:` and `align:` lines with:

```
    label: "text"           Words on the shape. Accepts inline markup:
                            <br> line break (a literal \n works too)
                            <b>…</b> bold, <i>…</i> italic
                            <small>…</small> smaller
                            <span fill=accent-dark>…</span> coloured run
                            A bare `<` is literal text, so "a < b" is fine.
    label_position: <where> inside (default) | above | below | left | right
    label_offset: <number>  Gap from the shape's edge for an outside label
                            (default 6)
    align: start|center|end Inside: where the text sits in its box.
                            above/below: which edge it aligns to.
                            left/right: top|center|bottom (same spellings).
    label_fill: <color>     Colour of the label text
```

and add to the EXAMPLES section:

```
Labels:
    rect b [label: "Cache"]                              // centred inside
    rect b [label: "Cache", align: end]                  // inside, right-aligned
    rect b [label: "based_on", label_position: below, align: end]
    rect card [label: "<b>title</b><br>second line<br><small>note</small>"]
    rect card [width: 200, label: "a long sentence that wraps to the width"]
```

- [ ] **Step 2: Update `docs/skill.md`**

Insert into the existing table near line 26:

```
| Text centred in a box | `rect b [label: "x"]` | a separate `text` element plus constraints |
| Several lines in a box | `label: "a<br>b<br>c"` | one `text` element per line |
| A caption under a box | `[label: "x", label_position: below]` | `constrain cap.top = b.bottom + 20` |
| A title above, left-aligned | `[label: "x", label_position: above, align: start]` | an absolute `center_y` |
| Text that must fit a fixed box | give the box a `width:`; the label wraps | guessing where to break it |
```

Also correct the note at `docs/skill.md:369`, which currently says `text name [label: "content"]` does not create text — it still does not, and the new markup does not change that. Leave the line, but append: "A shape's `label:` now takes `<br>`/`<b>`/`<i>`/`<small>`/`<span fill=>`."

- [ ] **Step 3: Update `docs/examples.md`**

In the architecture example (~line 280-330), replace the separate zone-label text elements and their absolute constraints:

```
    rect prod_bg [width: 550, height: 200, fill: accent-light, stroke: accent-dark, stroke_width: 2, opacity: 0.25]
    text "Production Zone" prod_label [font_size: 14, fill: accent-dark]
...
constrain prod_label.center_x = 300
constrain prod_label.center_y = 80
```

with:

```
    rect prod_bg [width: 550, height: 200, fill: accent-light, stroke: accent-dark, stroke_width: 2, opacity: 0.25,
                  label: "Production Zone", label_position: above, align: start, label_fill: accent-dark]
```

Do the same for `data_label`. Add a sentence under the example: "Zone titles ride on their background rect — `label_position: above` keeps the label out of the box and still inside the canvas, so there is no offset to guess."

- [ ] **Step 4: Verify the docs render**

Run: `cargo run -- --grammar | head -140`
Run: `cargo run -- --skill | head -40`
Expected: the new rows appear; no formatting breakage.

- [ ] **Step 5: Commit**

```bash
git add docs/grammar.md docs/skill.md docs/examples.md
git commit -m "docs: label markup, label_position, and wrapping

Rewrites the architecture example off absolute label coordinates — it
was teaching the pattern this feature exists to remove."
```

---

### Task 10: Adapt the shipped examples

**Files:**
- Modify: `examples/architecture.ail:39-45,71,133`
- Modify: `examples/gallic-wars-timeline.ail:40-68`
- Create: `examples/card-labels.ail`
- Modify: `examples/render-all.sh`
- Test: `bash examples/render-all.sh` + `--lint` on each

**Interfaces:**
- Consumes: everything from Tasks 5–7.
- Produces: `examples/card-labels.ail` as the canonical reference agents copy.

- [ ] **Step 1: Write `examples/card-labels.ail`**

```
// Boxes with labels: inside, outside, multi-line, wrapped.
// This is the reference for "put text on a box" — no offsets to guess.

col sheet [gap: 36] {
    text "Labels on boxes" heading [font_size: 18, fill: foreground-1]

    // --- where the label sits -------------------------------------------
    row placements [gap: 28] {
        rect p_in    [width: 120, height: 54, fill: accent-light, stroke: accent-dark, stroke_width: 2,
                      label: "inside"]
        rect p_above [width: 120, height: 54, fill: accent-light, stroke: accent-dark, stroke_width: 2,
                      label: "above", label_position: above]
        rect p_below [width: 120, height: 54, fill: accent-light, stroke: accent-dark, stroke_width: 2,
                      label: "below", label_position: below]
        rect p_left  [width: 120, height: 54, fill: accent-light, stroke: accent-dark, stroke_width: 2,
                      label: "left", label_position: left]
        rect p_right [width: 120, height: 54, fill: accent-light, stroke: accent-dark, stroke_width: 2,
                      label: "right", label_position: right]
    }

    // --- align picks the edge --------------------------------------------
    row alignments [gap: 28] {
        rect a_start [width: 160, height: 54, fill: secondary-light, stroke: secondary-dark, stroke_width: 2,
                      label: "below, start", label_position: below, align: start]
        rect a_mid   [width: 160, height: 54, fill: secondary-light, stroke: secondary-dark, stroke_width: 2,
                      label: "below, center", label_position: below, align: center]
        rect a_end   [width: 160, height: 54, fill: secondary-light, stroke: secondary-dark, stroke_width: 2,
                      label: "below, end", label_position: below, align: end]
    }

    // --- a multi-line card is one modifier --------------------------------
    row cards [gap: 28] {
        rect card [fill: accent-light, stroke: accent-dark, stroke_width: 2,
                   label: "<b>changeset from INES</b><br>temporal_mode = correction<br><small>V2 inherits its label from V1</small>"]

        // An explicit width is a promise: the label wraps to it.
        rect wrapped [width: 240, fill: accent-light, stroke: accent-dark, stroke_width: 2,
                      label: "T <= now confirmed_from <= T < confirmed_until, so the event is active"]
    }
}
```

- [ ] **Step 2: Render it and check it is clean**

Run:
```bash
./target/debug/agent-illustrator examples/card-labels.ail --lint > examples/card-labels.svg
```
Expected: zero lint warnings, and an SVG where every label is clear of its neighbours. If the linter reports anything, the example is wrong — fix the example, not the linter.

Register it in `examples/render-all.sh` alongside the other entries (follow the existing loop or list format in that file).

- [ ] **Step 3: Convert `examples/architecture.ail`**

Replace the three zone labels. Where the file currently has (around `:39-45`):

```
    text "Application Layer" app_label [font_size: 12, fill: accent-dark]
```

fold each into its background rect as `label: "Application Layer", label_position: above, align: start, label_fill: accent-dark`, and delete the matching `constrain app_label.…` lines. Do the same for `async_label` and `data_label`.

Replace `postgis_label` (`:71`) and its `constrain postgis_label.top = 748` (`:133`) with `label_position: below` on the `postgres` element — this is the exact "caption under the box with a guessed offset" the feature replaces.

Run:
```bash
./target/debug/agent-illustrator examples/architecture.ail --lint > examples/architecture.svg
```
Expected: no more warnings than before the change; visually the labels sit where they did.

- [ ] **Step 4: Convert `examples/gallic-wars-timeline.ail`**

Each event is currently three `text` elements (e.g. `:40-42`):

```
    text "58 BC" t58 [font_size: 11, fill: accent-dark]
    text "Defeats Helvetii" t58a [font_size: 11, fill: text-1]
    text "& Ariovistus" t58b [font_size: 11, fill: text-2]
```

Collapse each into one label on the event's box:

```
    label: "<b>58 BC</b><br>Defeats Helvetii<br><small>& Ariovistus</small>"
```

Delete the now-unused `text` elements and every constraint that positioned them. Repeat for the 55 BC, 57 BC, 52 BC and 50 BC events. The 52 BC event uses `text-light`/`accent-light` colours — carry those over with `<span fill=…>` or the element's `label_fill:`.

Run:
```bash
./target/debug/agent-illustrator examples/gallic-wars-timeline.ail --lint > examples/gallic-wars-timeline.svg
```
Expected: clean, and visually equivalent to the previous render.

- [ ] **Step 5: Re-render everything and commit**

Run: `bash examples/render-all.sh`
Run: `cargo test`
Expected: PASS, including `svg_regression`.

Confirm the conversion actually removed work:

Run: `git diff --stat examples/architecture.ail examples/gallic-wars-timeline.ail`
Expected: a net reduction in lines — 12 `text` elements and their constraints gone from the timeline, 4 and ~6 from the architecture example.

```bash
git add examples/card-labels.ail examples/card-labels.svg \
        examples/architecture.ail examples/architecture.svg \
        examples/gallic-wars-timeline.ail examples/gallic-wars-timeline.svg \
        examples/render-all.sh
git commit -m "examples: model the label idiom instead of hand-placed text

Agents copy examples more readily than they read docs, and these two
were teaching 'constrain postgis_label.top = 748'. The timeline loses
12 text elements and their constraints; the architecture example loses
its guessed label offsets.

Adds card-labels.ail as the reference figure for the feature."
```

---

## Self-Review

**Spec coverage:**

| Spec section | Task |
|---|---|
| 0. Unify text measurement | 1, 4 |
| A. `label_position:` on shapes | 6 |
| B. Inline markup in `label:` | 2, 5 |
| C. Explicit width wraps | 3, 7 |
| D. Linter changes (5 items) | 8 |
| E. Documentation | 9 |
| F. Adapt shipped examples | 10 |
| Backward compatibility (examples re-rendered) | 4, 5, 6, 7, 10 |
| Testing (unit / layout / lint / regression) | every task's step 4 |

No gaps.

**Type consistency:** `measure_str`, `measure_runs`, `advance_em`, `TextRun::plain`, `TextRun::width`, `RichText::plain`, `RichText::from_plain`, `parse_markup`, `wrap`, `SUPPORTED_TAGS`, `MarkupError`, `LINE_HEIGHT_FACTOR` are defined in Tasks 1–3 and used with those exact names and signatures in Tasks 4–8. `LabelLayout` gains `rich` and `font_size` in Task 5 and is read with those names in Tasks 6–8. `ShapeLabelPosition` and `extract_label_offset` are defined and used only in Task 6. `BoundingBox::contains_box` is added in Task 8 where it is used.

**Known risk:** Task 4 changes the measured width of every auto-sized box, so existing tests that pin a pixel value will fail. The instruction there is explicit — update the assertion to the new correct value, never loosen it to a range.
