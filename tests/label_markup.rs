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
    assert_eq!(
        wrapped.plain().split_whitespace().collect::<Vec<_>>(),
        "the quick brown fox jumps over the lazy dog"
            .split(' ')
            .collect::<Vec<_>>()
    );
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

use agent_illustrator::render;

#[test]
fn a_multiline_label_renders_one_tspan_per_line() {
    let svg = render(r#"rect card [label: "alpha<br>beta<br>gamma"]"#).expect("renders");
    assert_eq!(svg.matches("<tspan").count(), 6, "svg was: {svg}");
    assert!(svg.contains("alpha") && svg.contains("beta") && svg.contains("gamma"));
}

#[test]
fn a_box_grows_in_height_to_fit_its_lines() {
    let one = render(r#"rect card [label: "alpha"]"#).expect("renders");
    let three = render(r#"rect card [label: "alpha<br>beta<br>gamma"]"#).expect("renders");
    let height_of = |svg: &str| -> f64 {
        let at = svg.find("<rect").expect("a rect");
        let h = svg[at..].find(" height=\"").expect("rect height") + at + 9;
        svg[h..].split('"').next().unwrap().parse().unwrap()
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
    let at = svg.find("<rect").expect("a rect");
    let w = svg[at..].find(" width=\"").expect("rect width") + at + 8;
    let width: f64 = svg[w..].split('"').next().unwrap().parse().unwrap();
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

#[test]
fn an_explicit_width_wraps_the_label_instead_of_overflowing() {
    // The motivating failure: rect card [width: 300, label: "..."] drew the
    // text straight out through both borders.
    let svg = render(
        r#"rect card [width: 200, label: "T <= now confirmed_from <= T < confirmed_until"]"#,
    )
    .expect("renders");
    assert!(
        svg.matches(r#"<tspan x="#).count() >= 2,
        "expected a wrap, svg was: {svg}"
    );
}

#[test]
fn an_explicit_width_and_a_wrapped_label_grows_the_height() {
    let short = render(r#"rect card [width: 200, label: "short"]"#).expect("renders");
    let long = render(
        r#"rect card [width: 200, label: "T <= now confirmed_from <= T < confirmed_until"]"#,
    )
    .expect("renders");
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
