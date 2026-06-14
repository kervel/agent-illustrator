//! Integration tests: --animate-css animates geometry (not just visibility).
use agent_illustrator::{render_with_config, RenderConfig};

const SRC: &str = r#"
rect box [width: 100, height: 50, label: "B"]
rect other [width: 100, height: 50]
rect tok [width: 30, height: 20]
constrain box.center_x = 150
constrain box.center_y = 100
constrain other.center_x = 450
constrain other.center_y = 100
constrain tok.center_x = 300
constrain tok.center_y = 200
box.right -> other.left as feed
keyframe "idle" { hide tok }
keyframe "grow" { show tok; transform box [width: 260] }
keyframe "move" { transform box [dx: 40] }
"#;

fn animate_css(src: &str) -> String {
    let cfg = RenderConfig {
        animate_css: true,
        ..Default::default()
    };
    render_with_config(src, cfg).expect("render")
}

#[test]
fn animate_css_emits_element_transform_keyframes() {
    let svg = animate_css(SRC);
    // The geometry must come from a @keyframes block (the frame-class CSS is inert in
    // --animate-css mode — nothing toggles the frame class without JS).
    assert!(
        svg.contains("@keyframes kf-geo-box"),
        "expected a transform @keyframes (kf-geo-box) for the moved element, got:\n{}",
        svg
    );
}

#[test]
fn animate_css_emits_element_size_keyframes() {
    let svg = animate_css(SRC);
    assert!(
        svg.contains("@keyframes kf-width-box"),
        "expected a width @keyframes (kf-width-box) for the grown element, got:\n{}",
        svg
    );
}

#[test]
fn animate_css_combines_visibility_and_geometry_on_one_selector() {
    // tok hides then shows AND moves. CSS `animation` is one shorthand property, so
    // both must be listed in a SINGLE `.kf-tok { animation: ... }` rule — two separate
    // rules would clobber each other (the second wins), leaving tok invisible.
    let svg = animate_css(
        r#"
rect tok [width: 30, height: 20]
rect target [width: 10, height: 10]
constrain target.center_x = 400
constrain target.center_y = 100
constrain tok.center_x = 100
constrain tok.center_y = 100
keyframe "idle" { hide tok }
keyframe "show" { show tok; transform tok [dx: 50] }
"#,
    );
    let rules: Vec<&str> = svg.lines().filter(|l| l.contains(".kf-tok { animation:")).collect();
    assert_eq!(rules.len(), 1, "expected ONE .kf-tok animation rule, got {}: {:?}", rules.len(), rules);
    assert!(
        rules[0].contains("kf-anim-tok") && rules[0].contains("kf-geo-tok"),
        "the single rule must list BOTH visibility and geometry animations, got: {}",
        rules[0]
    );
}

#[test]
fn animate_css_morphable_connection_d_keyframes() {
    let svg = animate_css(SRC); // feed: straight box.right->other.left, box widens (morphable)
    assert!(
        svg.contains("@keyframes kf-d-feed"),
        "expected d-morph @keyframes (kf-d-feed) for the following connection, got:\n{}",
        svg
    );
}

#[test]
fn animate_css_reshaping_connection_crossfades() {
    let svg = animate_css(r#"
rect box [width: 80, height: 40]
rect other [width: 80, height: 40]
constrain box.center_x = 120
constrain box.center_y = 100
constrain other.center_x = 120
constrain other.center_y = 320
box.bottom -> other.top as link
keyframe "idle" {}
keyframe "shift" { transform box [dx: 220] }
"#);
    // reshaping route → crossfade variant participates in a (step) opacity animation
    assert!(
        svg.contains("conn-link-fshift") && svg.contains("kf-variant-link-shift"),
        "expected crossfade variant animation for reshaping route, got:\n{}",
        svg
    );
}

#[test]
fn animate_css_geometry_is_smooth_visibility_is_step() {
    let svg = animate_css(SRC);
    assert!(
        svg.contains("ease infinite") || svg.contains("ease-in-out infinite"),
        "expected a smooth (ease) geometry animation, got:\n{}",
        svg
    );
    assert!(
        svg.contains("step-end infinite"),
        "visibility should still be step-end, got:\n{}",
        svg
    );
}
