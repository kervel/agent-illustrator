//! A label rewritten by a keyframe is still a label.
//!
//! Declared labels parse the markup subset; transformed ones did not, so
//! `--animate` emitted the replacement as one unwrapped line with the tags
//! stripped. A four-line card became a single 1200px line in a 400px box, in
//! the published frame only — and `--lint` said clean, because label-overflow
//! measured the declared label and never looked at the variants.
//!
//! The second half is the worse half. A tool that renders something wrong is a
//! bug; a tool that renders something wrong AND reports clean is what lets it
//! reach a browser.

use agent_illustrator::{render_with_config, render_with_lint, RenderConfig};

const CARD: &str = r#"
rect card [width: 200, height: 60, fill: accent-light, stroke: accent-dark,
           label: "title<br><small>second line</small>"]
constrain card.center_x = 150
constrain card.center_y = 60
keyframe "a" { }
keyframe "b" { transform card [label: "retitled<br><small>second line here too</small>"] }
"#;

fn animated(source: &str) -> String {
    let mut config = RenderConfig::new();
    config.animate = true;
    render_with_config(source, config).expect("renders")
}

fn warnings(source: &str) -> Vec<String> {
    let (_svg, w) = render_with_lint(source, RenderConfig::new().with_lint(true)).expect("renders");
    w.iter().map(|x| x.message.clone()).collect()
}

/// The `<text>` element carrying `class`, and its inner markup.
fn text_node<'a>(svg: &'a str, class: &str) -> &'a str {
    // rfind, not find: the class also appears in the <style> block, where
    // there is no <text> before it. The class carries a per-document scope
    // prefix, so match on its suffix.
    let start = svg.rfind(class).unwrap_or_else(|| panic!("no {class} in:\n{svg}"));
    let from = svg[..start].rfind("<text").expect("opening tag");
    let end = svg[from..].find("</text>").expect("closing tag") + from;
    &svg[from..end]
}

#[test]
fn a_rewritten_label_parses_its_markup() {
    let svg = animated(CARD);
    let variant = text_node(&svg, "-card-v0");
    assert!(
        !variant.contains("<br>") && !variant.contains("<small>"),
        "tags must not survive as literal text: {variant}"
    );
    assert!(
        variant.matches("dy=").count() >= 2,
        "the rewritten label should be two lines like the declared one: {variant}"
    );
    assert!(
        variant.contains("font-size="),
        "<small> should still shrink the second line: {variant}"
    );
}

#[test]
fn the_declared_label_is_unchanged() {
    let svg = animated(CARD);
    let base = text_node(&svg, "-card-base");
    assert!(base.matches("dy=").count() >= 2, "got: {base}");
}

#[test]
fn a_static_frame_and_the_animated_variant_agree() {
    // The two paths disagreeing is what let this ship: --frame parsed the
    // markup and was checked, --animate did not and was served.
    let mut config = RenderConfig::new();
    config.frame = Some("b".to_string());
    let static_svg = render_with_config(CARD, config).expect("renders");
    let static_lines = static_svg
        .lines()
        .find(|l| l.contains("ai-label"))
        .map(|l| l.matches("dy=").count())
        .expect("label rendered");

    let svg = animated(CARD);
    let variant = text_node(&svg, "-card-v0");
    assert_eq!(
        variant.matches("dy=").count(),
        static_lines,
        "the same wording must lay out the same way on both paths"
    );
}

// ── the check that was missing ────────────────────────────────────

#[test]
fn a_variant_that_overflows_its_box_is_reported() {
    let msgs = warnings(
        r#"
rect card [width: 200, height: 60, fill: accent-light, stroke: accent-dark, label: "short"]
constrain card.center_x = 150
constrain card.center_y = 60
keyframe "a" { }
keyframe "b" { transform card [label: "a replacement wording far too wide to fit inside that little box"] }
"#,
    );
    assert!(
        msgs.iter().any(|m| m.contains("needs about")),
        "a label three times its box's width in one frame must be reported: {msgs:?}"
    );
}

#[test]
fn a_variant_that_fits_is_not_reported() {
    let msgs = warnings(
        r#"
rect card [width: 300, height: 60, fill: accent-light, stroke: accent-dark, label: "short"]
constrain card.center_x = 200
constrain card.center_y = 60
keyframe "a" { }
keyframe "b" { transform card [label: "also short"] }
"#,
    );
    assert!(
        !msgs.iter().any(|m| m.contains("needs about")),
        "got: {msgs:?}"
    );
}

#[test]
fn the_warning_names_the_frame_and_the_wording() {
    let msgs = warnings(
        r#"
rect card [width: 200, height: 60, fill: accent-light, stroke: accent-dark, label: "short"]
constrain card.center_x = 150
constrain card.center_y = 60
keyframe "a" { }
keyframe "b" { transform card [label: "a replacement wording far too wide to fit inside that little box"] }
"#,
    );
    let m = msgs
        .iter()
        .find(|m| m.contains("needs about"))
        .expect("reported");
    assert!(m.contains("card"), "names the element: {m}");
    assert!(
        m.contains("replacement") || m.contains('b'),
        "names the wording or the frame: {m}"
    );
}
