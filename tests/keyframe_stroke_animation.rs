//! "A planned thing becomes a confirmed thing" is dashed-outline -> solid.
//! It could not be expressed, and the workaround was a hidden solid twin,
//! which painted over the original's name, which needed a duplicate name
//! element and a second hide list: three elements for one property.
//!
//! And the failure was silent. `unknown-modifier` fires on a *misspelled*
//! key, but `stroke_dasharray` is a recognised StyleKey, so it passed that
//! check and was dropped by a catch-all. An author writing correct syntax got
//! no signal at all.

use agent_illustrator::{render_with_config, render_with_lint, RenderConfig};

const DASHED: &str = r#"
rect item [width: 100, height: 40, stroke_dasharray: "8,4", stroke: foreground-1, fill: none]
constrain item.center_x = 100
constrain item.center_y = 60
keyframe "planned" { }
keyframe "settled" { transform item [stroke_dasharray: "0,0", fill: accent-light] }
"#;

fn frame(source: &str, n: usize) -> String {
    let mut config = RenderConfig::new();
    config.frame = Some(n.to_string());
    render_with_config(source, config).expect("renders")
}

fn warnings(source: &str) -> Vec<String> {
    let (_svg, w) = render_with_lint(source, RenderConfig::new().with_lint(true)).expect("renders");
    w.iter().map(|x| x.message.clone()).collect()
}

#[test]
fn a_keyframe_can_change_stroke_dasharray() {
    let f1 = frame(DASHED, 1);
    let item = f1
        .lines()
        .find(|l| l.contains(r#"id="item""#))
        .expect("item rendered")
        .to_string();
    assert!(
        item.contains(r#"stroke-dasharray="0,0""#),
        "frame 1 should carry the new dash pattern, got: {item}"
    );
}

#[test]
fn frame_zero_keeps_the_authored_dasharray() {
    let f0 = frame(DASHED, 0);
    let item = f0
        .lines()
        .find(|l| l.contains(r#"id="item""#))
        .expect("item rendered")
        .to_string();
    assert!(item.contains(r#"stroke-dasharray="8,4""#), "got: {item}");
}

#[test]
fn a_keyframe_can_change_stroke_width() {
    let source = r#"
rect item [width: 100, height: 40, stroke: foreground-1, stroke_width: 1]
constrain item.center_x = 100
constrain item.center_y = 60
keyframe "a" { }
keyframe "b" { transform item [stroke_width: 6] }
"#;
    let f1 = frame(source, 1);
    let item = f1
        .lines()
        .find(|l| l.contains(r#"id="item""#))
        .expect("item rendered")
        .to_string();
    assert!(item.contains(r#"stroke-width="6""#), "got: {item}");
}

#[test]
fn animating_dasharray_produces_no_warning() {
    let msgs = warnings(DASHED);
    assert!(
        !msgs.iter().any(|m| m.contains("stroke_dasharray")),
        "an animatable key must not warn, got: {msgs:?}"
    );
}

// ── 3b: the silent drop ───────────────────────────────────────────

#[test]
fn a_valid_but_unanimatable_key_is_reported() {
    // The nasty case: correct syntax, recognised key, silently ignored.
    let msgs = warnings(
        r#"
rect item [width: 100, height: 40, fill: accent-light]
constrain item.center_x = 100
constrain item.center_y = 60
keyframe "a" { }
keyframe "b" { transform item [z_order: 5] }
"#,
    );
    assert!(
        msgs.iter()
            .any(|m| m.contains("z_order") && m.contains("animat")),
        "expected a warning that z_order cannot be animated, got: {msgs:?}"
    );
}

#[test]
fn a_misspelled_key_still_reports_unknown_modifier() {
    // The two must stay distinguishable: one is a typo, the other is a real
    // key the renderer cannot animate. Same warning for both would be worse
    // than either.
    let msgs = warnings(
        r#"
rect item [width: 100, height: 40, fill: accent-light]
constrain item.center_x = 100
constrain item.center_y = 60
keyframe "a" { }
keyframe "b" { transform item [bogus_key: 5] }
"#,
    );
    assert!(
        msgs.iter().any(|m| m.contains("bogus_key")),
        "got: {msgs:?}"
    );
    assert!(
        !msgs.iter().any(|m| m.contains("bogus_key") && m.contains("cannot be animated")),
        "a typo is not an unanimatable key: {msgs:?}"
    );
}

#[test]
fn an_animatable_key_never_reports_as_unanimatable() {
    for key in ["fill", "stroke", "opacity", "rotation", "label", "width"] {
        let source = format!(
            r#"
rect item [width: 100, height: 40, fill: accent-light]
constrain item.center_x = 100
constrain item.center_y = 60
keyframe "a" {{ }}
keyframe "b" {{ transform item [{key}: {}] }}
"#,
            if key == "label" {
                "\"x\"".to_string()
            } else if ["fill", "stroke"].contains(&key) {
                "accent-dark".to_string()
            } else {
                "2".to_string()
            }
        );
        let msgs = warnings(&source);
        assert!(
            !msgs.iter().any(|m| m.contains("cannot be animated")),
            "{key} is animatable but warned: {msgs:?}"
        );
    }
}
