//! Friction reported while reworking a deck on v0.2.37: a clip that stayed at
//! bg's old size, a z_order lint between things never on screen together, a
//! hand-written caption that ignored its subject's `appears: later`, and an
//! accent whose own lint suggestion did not clear the warning.
use agent_illustrator::{render_with_config, render_with_lint, RenderConfig};

const CLIP: &str = r#"
rect bg [width: 200, height: 100, fill: role-surface, stroke: role-rule, corner_radius: 8]
rect head [width: 400, height: 30, fill: role-primary, stroke: none, clip: bg]
constrain head.top = bg.top
constrain head.left = bg.left
keyframe "start" { }
keyframe "wide" { transform bg [width: 400, duration: 1] }
"#;

#[test]
fn clip_follows_the_clipped_to_shape_in_the_css_modes() {
    let cfg = RenderConfig { animate: true, ..Default::default() };
    let svg = render_with_config(CLIP, cfg).unwrap();
    assert!(svg.contains("-head { width: 398.5px"), "clip rect resized with bg:\n{svg}");
    let cfg = RenderConfig { animate_css: true, ..Default::default() };
    let svg = render_with_config(CLIP, cfg).unwrap();
    assert!(svg.contains(".kfc-") && svg.contains("-head { animation:"), "clip rect has its own track:\n{svg}");
}

#[test]
fn a_long_form_caption_comes_with_its_subject() {
    let src = r#"
rect a [size: 40, appears: later]
text "hi" cap [caption_of: a, label_position: below]
keyframe "k1" { show a }
"#;
    let cfg = RenderConfig { frame: Some("k1".into()), ..Default::default() };
    let svg = render_with_config(src, cfg).unwrap();
    assert!(svg.contains(">hi<"), "caption shown with a:\n{svg}");
}

#[test]
fn marker_accent_in_error_tone_has_no_badge_to_collide() {
    let src = r#"
rect a [width: 100, height: 40, label: "A"]
rect b [width: 100, height: 40, label: "B"]
a.right -> b.left
keyframe "go" { accent a [tone: error, style: marker] }
"#;
    let (_svg, w) = render_with_lint(src, RenderConfig::new().with_lint(true)).unwrap();
    assert!(w.iter().all(|w| !w.message.contains("accent")), "{:?}", w.iter().map(|w| &w.message).collect::<Vec<_>>());
}
