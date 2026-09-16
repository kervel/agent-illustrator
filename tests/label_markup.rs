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
    assert_eq!(lines_of("T < confirmed_until"), vec!["T < confirmed_until"]);
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
