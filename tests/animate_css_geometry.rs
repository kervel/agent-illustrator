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
    let mut cfg = RenderConfig::default();
    cfg.animate_css = true;
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
