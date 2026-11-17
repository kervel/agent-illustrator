//! Lint measures a rotated shape by the box it covers on the page.
//!
//! `rotation:` on a plain shape (text, rect, ...) is drawn as an SVG rotate
//! about its pivot, but layout keeps the unrotated box. Lint used that box,
//! so a vertical title in a narrow header strip was reported as straddling
//! the strip (its unrotated width sticks out), and a word turned upright was
//! never checked where it really is.

use agent_illustrator::{render_with_lint, RenderConfig};

fn warnings(source: &str) -> Vec<String> {
    let (_svg, w) = render_with_lint(source, RenderConfig::new().with_lint(true)).expect("renders");
    w.iter().map(|x| x.message.clone()).collect()
}

#[test]
fn vertical_text_inside_a_narrow_strip_is_clean() {
    let src = r#"
rect hd [width: 36, height: 130, fill: role-surface-2]
text "Account manager" ttl [rotation: -90, font_size: 13, overlaps: hd]
constrain ttl.center = hd.center
"#;
    let w = warnings(src);
    assert!(
        !w.iter().any(|m| m.contains("ttl")),
        "a vertical title inside its strip must not be reported: {w:?}"
    );
}

#[test]
fn vertical_text_sticking_out_of_a_short_strip_is_reported() {
    // Upright, the title is ~100px tall; the strip is 40px tall.
    let src = r#"
rect hd [width: 60, height: 40, fill: role-surface-2]
text "Account manager" ttl [rotation: -90, font_size: 13, overlaps: hd]
constrain ttl.center = hd.center
"#;
    let w = warnings(src);
    assert!(
        w.iter().any(|m| m.contains("ttl") && m.contains("hd")),
        "a vertical title taller than its strip must be reported: {w:?}"
    );
}

#[test]
fn rotated_label_on_a_rotated_rect_is_clean() {
    // A long rect turned upright with its label: both turn together.
    let src = r#"
rect bar [width: 140, height: 30, rotation: 90, label: "Account manager"]
"#;
    let w = warnings(src);
    assert!(!w.iter().any(|m| m.contains("bar")), "{w:?}");
}

#[test]
fn overlaps_inside_a_template_names_its_own_part() {
    // `overlaps: hd` in a template means this instance's hd, as at top level.
    let src = r#"
template "T" () {
    rect hd [width: 60, height: 60, fill: role-surface-2]
    text "X" ttl [overlaps: hd]
    constrain ttl.center = hd.center
}
T a
"#;
    let w = warnings(src);
    assert!(w.is_empty(), "{w:?}");
}

#[test]
fn overlaps_with_a_list_parameter_names_what_was_passed_in() {
    let src = r#"
rect big [width: 200, height: 80]
template "T" (on: []) { text "X" ttl [overlaps: on] }
T a [on: [big]]
constrain a.center = big.center
"#;
    let w = warnings(src);
    assert!(w.is_empty(), "{w:?}");
}

#[test]
fn a_parameter_value_equal_to_a_part_name_does_not_unscope_the_part() {
    // `overlaps: hd` is the instance's own hd, even when a parameter's value
    // happens to be an outside element also called hd.
    let src = r#"
rect hd [width: 20, height: 20]
template "T" (near: "") {
    rect hd [width: 60, height: 60, fill: role-surface-2]
    text "X" ttl [overlaps: hd]
    constrain ttl.center = hd.center
    constrain hd.left = near.right + 40
}
T a [near: hd]
"#;
    let w = warnings(src);
    assert!(w.is_empty(), "{w:?}");
}

#[test]
fn a_rotated_child_does_not_change_its_columns_box_after_layout() {
    // Layout uses the unturned box; recomputing the column with the turned
    // one made this constraint look violated.
    let src = r#"
col g { rect a [width: 100, height: 20, rotation: 90] }
rect c [width: 30, height: 30]
constrain c.left = g.right + 5
"#;
    let w = warnings(src);
    assert!(!w.iter().any(|m| m.contains("violated")), "{w:?}");
}
