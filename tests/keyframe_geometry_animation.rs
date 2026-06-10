//! Integration tests for keyframe position/size animation (geometry channel).
use agent_illustrator::render;

#[test]
fn position_emits_translate_on_wrapper_group() {
    // box moves right+down by a delta in frame "move".
    let src = r#"
rect box [width: 100, height: 50, label: "B"]
constrain box.center_x = 100
constrain box.center_y = 100
keyframe "idle" {}
keyframe "move" { transform box [dx: 60, dy: 40] }
"#;
    let svg = render(src).expect("render ok");
    // position animates as a transform translate on the wrapper, NOT as #box { x: ... }
    assert!(svg.contains(".kf-box { transform: translate(60px, 40px)"),
        "expected translate on .kf-box, got:\n{}", svg);
    assert!(!svg.contains("#box { x:"),
        "position must NOT be emitted as a geometry x prop, got:\n{}", svg);
}

#[test]
fn size_emits_width_height_on_shape() {
    let src = r#"
rect box [width: 100, height: 50]
constrain box.center_x = 100
constrain box.center_y = 100
keyframe "idle" {}
keyframe "grow" { transform box [width: 300, height: 120] }
"#;
    let svg = render(src).expect("render ok");
    assert!(svg.contains("#box {") && svg.contains("width: 300px") && svg.contains("height: 120px"),
        "expected #box width/height on inner shape, got:\n{}", svg);
}

#[test]
fn emits_transition_rules_and_kf_anim_class() {
    let src = r#"
rect box [width: 100, height: 50]
constrain box.center_x = 100
constrain box.center_y = 100
keyframe "idle" {}
keyframe "grow" { transform box [width: 300] }
"#;
    let svg = render(src).expect("render ok");
    assert!(svg.contains(".kf-anim { transition: transform 0.5s ease, opacity 0.5s ease;"),
        "expected .kf-anim transition rule, got:\n{}", svg);
    assert!(svg.contains(".ai-shape { transition:"),
        "expected .ai-shape transition rule, got:\n{}", svg);
    // box is geometry-diffed → gets a wrapper group carrying kf-anim
    assert!(svg.contains("kf-anim"), "wrapper group should carry kf-anim, got:\n{}", svg);
}
