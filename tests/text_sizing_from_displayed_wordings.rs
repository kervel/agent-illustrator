//! A box is sized for the wordings it actually shows.
//!
//! friction.txt P0-1(b): the declared wording sized the box even when the
//! first keyframe overrode it, so the box reserved space for a string drawn in
//! no frame at all. That slack is what makes a centre-constrained text element
//! render off-centre — start-anchored glyphs land on the box's left edge, and
//! the extra width all lands on one side.

use agent_illustrator::{render_with_config, RenderConfig};

fn frame(source: &str, n: usize) -> String {
    let mut config = RenderConfig::new();
    config.frame = Some(n.to_string());
    render_with_config(source, config).expect("renders")
}

/// Total width of the rendered document — a proxy for how wide the text box
/// was sized, now that centred text sits at the same x whatever its box.
fn doc_width(svg: &str) -> f64 {
    let marker = r#"viewBox="#;
    let start = svg.find(marker).expect("viewBox") + marker.len() + 1;
    let rest = &svg[start..];
    let end = rest.find('"').unwrap();
    rest[..end]
        .split_whitespace()
        .nth(2)
        .expect("viewBox width")
        .parse()
        .unwrap()
}

fn text_x(svg: &str, id: &str) -> f64 {
    let marker = format!(r#"<text id="{id}""#);
    let start = svg.find(&marker).expect("text rendered");
    let rest = &svg[start..];
    let xi = rest.find(r#" x=""#).expect("x attr") + 4;
    let tail = &rest[xi..];
    let end = tail.find('"').unwrap();
    tail[..end].parse().unwrap()
}

#[test]
fn a_declared_wording_shown_in_no_frame_does_not_size_the_box() {
    // The reported case. Every keyframe overrides the label, so the declared
    // string is drawn nowhere and must not reserve ~445px.
    let source = r#"
text "THIS DECLARED WORDING IS NEVER DISPLAYED IN ANY FRAME" b [font_size: 14]
constrain b.center_x = 300
keyframe "idle" { transform b [label: "short one"] }
keyframe "two"  { transform b [label: "short two"] }
"#;
    let x = text_x(&frame(source, 0), "b");
    // "short one" is ~65px, so a box sized to it centres near 300 - 65/2.
    assert!(
        x > 250.0,
        "the box should be sized for the wording on screen, got x={x}"
    );
}

#[test]
fn a_declared_wording_that_frame_zero_shows_still_sizes_the_box() {
    // The first keyframe leaves the label alone, so the declared wording IS
    // displayed and must keep its space.
    let source = r#"
text "a considerably longer declared wording here" b [font_size: 14]
constrain b.center_x = 300
keyframe "idle" { }
keyframe "two"  { transform b [label: "short"] }
"#;
    let svg = frame(source, 0);
    assert!(
        svg.contains("a considerably longer declared wording here"),
        "frame 0 draws the declared wording"
    );
    // The box must still be wide enough to hold the declared wording.
    assert!(
        doc_width(&svg) > 250.0,
        "the declared wording must still reserve its space, got {}",
        doc_width(&svg)
    );
}

#[test]
fn every_displayed_wording_still_fits() {
    // Shrinking must not clip a later, longer wording.
    let source = r#"
text "tiny" b [font_size: 14]
constrain b.center_x = 300
keyframe "one" { transform b [label: "short"] }
keyframe "two" { transform b [label: "a much longer wording than the first"] }
"#;
    let f1 = frame(source, 1);
    assert!(f1.contains("a much longer wording than the first"));
    // Shrinking to displayed wordings must not clip the longest of them.
    assert!(
        doc_width(&f1) > 250.0,
        "the box must still hold the longest wording, got {}",
        doc_width(&f1)
    );
}

// ── 5b: the anchor default ────────────────────────────────────────

#[test]
fn text_centres_itself_when_its_box_is_wider_than_its_wording() {
    // 5a removes the slack wherever the wordings are similar in length. It
    // cannot remove it where they genuinely differ, and nothing stops an
    // author constraining a box wider than its text. So the default is the
    // guard rail: with slack and no explicit align, centre the glyphs.
    let source = r#"
text "tiny" b [font_size: 14]
constrain b.center_x = 300
keyframe "one" { transform b [label: "tiny"] }
keyframe "two" { transform b [label: "a much longer wording than the first"] }
"#;
    let x = text_x(&frame(source, 0), "b");
    assert!(
        (x - 300.0).abs() < 1.0,
        "a text element with slack should centre on its constrained centre, got {x}"
    );
}

#[test]
fn an_explicit_align_still_wins() {
    let source = r#"
text "tiny" b [font_size: 14, align: start]
constrain b.center_x = 300
keyframe "one" { transform b [label: "tiny"] }
keyframe "two" { transform b [label: "a much longer wording than the first"] }
"#;
    let x = text_x(&frame(source, 0), "b");
    assert!(x < 250.0, "align: start must still start-anchor, got {x}");
}

#[test]
fn a_plain_text_element_is_unchanged() {
    // No constraints, no keyframes, no slack — must render exactly as before.
    let svg = agent_illustrator::render(r#"text "hello" t [font_size: 14]"#).expect("renders");
    assert!(
        svg.contains(r#"text-anchor="start""#),
        "a box that fits its text keeps the historical anchor: {svg}"
    );
}
