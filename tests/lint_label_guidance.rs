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
        messages
            .iter()
            .any(|m| m.contains("card") && m.contains("label:")),
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
        messages
            .iter()
            .any(|m| m.contains("<bold>") && m.contains("<b>")),
        "expected the supported set to be listed, got: {messages:?}"
    );
}

#[test]
fn a_less_than_sign_in_a_label_is_not_reported_as_markup() {
    let messages = lint(r#"rect a [label: "T < confirmed_until"]"#);
    assert!(messages.is_empty(), "got: {messages:?}");
}

#[test]
fn an_outside_label_cannot_overflow_its_box() {
    // A 10px switch cannot hold its own name, which is why the label goes
    // outside. Once it is outside it is drawn on the background and is not
    // trying to fit in the box, so reporting overflow is wrong.
    let messages = lint(
        r#"circle j [size: 10, fill: accent-dark, label: "Switch 1", label_position: above]"#,
    );
    assert!(
        !messages.iter().any(|m| m.contains("overflow")),
        "got: {messages:?}"
    );
}

#[test]
fn an_outside_label_does_not_inherit_the_shape_fill_for_contrast() {
    let messages = lint(
        r#"circle j [size: 10, fill: #228B22, label: "Switch 1", label_position: below]"#,
    );
    assert!(
        !messages.iter().any(|m| m.contains("dark fill")),
        "got: {messages:?}"
    );
}

#[test]
fn an_inside_label_still_reports_overflow_and_contrast() {
    let messages = lint(r#"circle j [size: 10, fill: #228B22, label: "Switch 1"]"#);
    assert!(
        messages.iter().any(|m| m.contains("overflow")),
        "inside labels must still report overflow: {messages:?}"
    );
    assert!(
        messages.iter().any(|m| m.contains("dark fill")),
        "inside labels must still report contrast: {messages:?}"
    );
}

#[test]
fn a_barely_visible_fill_is_not_treated_as_dark() {
    // `fill: X, fill_opacity: 0.10` composites to a very pale box. Reporting
    // it as a dark fill is what drives a stylesheet to pick light label text,
    // which then lands near-white on near-white.
    let messages = lint(
        r#"rect a [width: 120, height: 40, fill: #1a1a1a, fill_opacity: 0.10, label: "swatch"]"#,
    );
    assert!(
        !messages.iter().any(|m| m.contains("dark fill")),
        "a 10% fill is not dark: {messages:?}"
    );
}

#[test]
fn a_solid_dark_fill_is_still_reported() {
    let messages =
        lint(r#"rect a [width: 120, height: 40, fill: #1a1a1a, label: "swatch"]"#);
    assert!(
        messages.iter().any(|m| m.contains("dark fill")),
        "got: {messages:?}"
    );
}

#[test]
fn a_mostly_opaque_dark_fill_is_still_reported() {
    // The exemption is for washes, not for a fill that is merely not solid.
    let messages = lint(
        r#"rect a [width: 120, height: 40, fill: #1a1a1a, fill_opacity: 0.9, label: "swatch"]"#,
    );
    assert!(
        messages.iter().any(|m| m.contains("dark fill")),
        "got: {messages:?}"
    );
}
