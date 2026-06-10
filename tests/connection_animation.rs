//! Integration tests: connections follow moving/resized endpoints across keyframes.
use agent_illustrator::render;

const SRC_TRANSFORM: &str = r#"
rect box [width: 100, height: 50]
rect other [width: 100, height: 50]
constrain box.center_x = 150
constrain box.center_y = 100
constrain other.center_x = 450
constrain other.center_y = 100
box.right -> other.left as feed
keyframe "idle" {}
keyframe "grow" { transform box [width: 260] }
"#;

#[test]
fn connection_follows_transformed_endpoint() {
    let svg = render(SRC_TRANSFORM).expect("render");
    // grow frame: feed re-anchors to the box's new right edge → a per-frame d: rule.
    assert!(svg.contains(".conn-feed { d: path("),
        "expected d: morph rule for feed, got:\n{}", svg);
}

#[test]
fn connection_transition_rule_present() {
    let svg = render(SRC_TRANSFORM).expect("render");
    assert!(svg.contains(".ai-connection { transition: d 0.5s ease, opacity 0.5s ease;"),
        "expected .ai-connection transition rule, got:\n{}", svg);
}

#[test]
fn connection_follows_constraint_change() {
    let svg = render(r#"
rect box [width: 100, height: 50]
rect other [width: 100, height: 50]
constrain box.left = 80
constrain box.center_y = 100
constrain box.width = 190 as w0
constrain other.center_x = 450
constrain other.center_y = 100
box.right -> other.left as feed
keyframe "idle" {}
keyframe "grow" { disable w0; constrain box.width = 300 }
"#).expect("render");
    assert!(svg.contains(".conn-feed { d: path("),
        "constraint-driven widen should move the feed connection, got:\n{}", svg);
}

#[test]
fn static_connection_does_not_flicker() {
    // feed connects two static elements; only an unrelated element moves.
    let svg = render(r#"
rect a [width: 60, height: 30]
rect b [width: 60, height: 30]
rect mover [width: 40, height: 20]
constrain a.center_x = 100
constrain a.center_y = 60
constrain b.center_x = 400
constrain b.center_y = 60
constrain mover.center_x = 250
constrain mover.center_y = 200
a.right -> b.left as feed
keyframe "idle" {}
keyframe "go" { transform mover [dx: 50] }
"#).expect("render");
    assert!(!svg.contains(".conn-feed { d:"),
        "static connection must not get a path diff (anti-flicker), got:\n{}", svg);
}

#[test]
fn connection_crossfades_when_route_reshapes() {
    // Vertical stack → orthogonal route is a straight line (2 pts); shifting box
    // horizontally forces a bend (more pts) → not morphable → crossfade variant.
    let svg = render(r#"
rect box [width: 80, height: 40]
rect other [width: 80, height: 40]
constrain box.center_x = 120
constrain box.center_y = 100
constrain other.center_x = 120
constrain other.center_y = 320
box.bottom -> other.top as link
keyframe "idle" {}
keyframe "shift" { transform box [dx: 220] }
"#).expect("render");
    let has_variant = svg.contains("conn-link-fshift");
    let has_morph = svg.contains(".conn-link { d:");
    assert!(has_variant && !has_morph,
        "reshaped route should crossfade (variant + opacity), not morph. variant={} morph={}\n{}",
        has_variant, has_morph, svg);
}
