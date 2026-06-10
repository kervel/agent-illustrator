//! Regression tests for two keyframe animation bugs:
//!
//! Bug 1: `hide` is a no-op on elements that are visible in frame 0.
//!   An element visible in the first keyframe and hidden in a later keyframe
//!   never receives its `kf-<id>` class, so the generated
//!   `.frame-X { .kf-<id> { opacity: 0 } }` rule binds to nothing and the
//!   element stays visible.
//!
//! Bug 2: `show` + `transform` on the same element in one keyframe drops the
//!   transform. Only the opacity (show) is emitted; the stroke/fill/geometry
//!   override is lost.

use agent_illustrator::render;

/// Bug 1: An element visible at frame 0 and hidden later must carry the
/// `kf-<id>` class on a wrapper group so the later frame's
/// `.kf-<id> { opacity: 0 }` rule actually targets a DOM node.
#[test]
fn hide_applies_to_element_visible_in_frame_zero() {
    let src = r#"
rect box1 [width: 100, height: 50, label: "A"]
constrain box1.center_x = 100
constrain box1.center_y = 100

keyframe "idle" {
}

keyframe "gone" {
    hide box1
}
"#;
    let svg = render(src).expect("render should succeed");

    // The CSS rule to hide box1 in the "gone" frame is emitted...
    assert!(
        svg.contains(".kf-box1 { opacity: 0;"),
        "expected hide rule .kf-box1 {{ opacity: 0; }} in CSS, got:\n{}",
        svg
    );

    // ...but it only works if some element actually carries the kf-box1 class.
    // Match a class attribute (not the `.kf-box1` CSS selector) — the wrapper
    // group is `<g class="kf-box1 kf-anim">` (or `kf-hidden kf-box1 kf-anim`).
    assert!(
        svg.contains(r#"class="kf-box1 "#) || svg.contains("kf-hidden kf-box1"),
        "element box1 must carry the kf-box1 class so the hide rule binds, got:\n{}",
        svg
    );
}

/// Bug 2: `show` + `transform` on the same element in one keyframe must emit
/// BOTH the visibility (opacity) and the transform (stroke) overrides.
#[test]
fn show_and_transform_same_element_keeps_transform() {
    let src = r#"
rect tok3 [width: 100, height: 50, label: "T"]
constrain tok3.center_x = 100
constrain tok3.center_y = 100

keyframe "idle" {
    hide tok3
}

keyframe "active" {
    show tok3
    transform tok3 [stroke: accent-1]
}
"#;
    let svg = render(src).expect("render should succeed");

    // The show must still emit the opacity override.
    assert!(
        svg.contains(".kf-tok3 { opacity: 1;"),
        "expected show rule .kf-tok3 {{ opacity: 1; }}, got:\n{}",
        svg
    );

    // The transform on the shown element must NOT be dropped: a stroke
    // override targeting #tok3 must appear in the same frame.
    assert!(
        svg.contains("#tok3 {") && svg.contains("stroke:"),
        "expected transform stroke override #tok3 {{ stroke: ... }} to survive show, got:\n{}",
        svg
    );
}
