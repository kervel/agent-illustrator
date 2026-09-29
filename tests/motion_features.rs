//! First-class motion: compiler, timeline, sampler and the authoring
//! features that came with it (nested components, dotted paths, appears,
//! canvas, through-paths).

use agent_illustrator::{render_with_config, RenderConfig};

fn cfg() -> RenderConfig {
    let mut c = RenderConfig::new();
    c.custom_css = Some(include_str!("../examples/motion/git-deck.css").to_string());
    c
}

fn timeline(src: &str) -> String {
    let mut c = cfg();
    c.timeline = true;
    render_with_config(src, c).expect("timeline")
}

#[test]
fn pushpull_nested_components_and_fan_out() {
    // Written by a first-time author: a machine with a history inside it.
    let src = include_str!("motion/pushpull.ail");
    let t = timeline(src);
    assert!(t.contains("fly hub.hist.d4 -> anna.hist.d4"), "{}", t);
    assert!(t.contains("fly hub.hist.d4 -> chris.hist.d4"), "{}", t);
    assert!(t.contains("draw [to: ben.hist.d4] ben.hist.h_track"), "{}", t);
    let svg = render_with_config(src, cfg()).expect("render");
    assert!(svg.contains("class=\"ail-motion\""));
}

#[test]
fn nested_component_is_placed_by_its_parent() {
    let src = r#"
template "dots" () {
    row r [gap: 10] { circle a [size: 10]  circle b [size: 10] }
}
template "box" () {
    rect bg [width: 200, height: 100]
    dots d
    constrain d.center_x = bg.center_x
    constrain d.center_y = bg.center_y
}
box one
constrain one.left = 0
constrain one.top = 0
"#;
    let svg = render_with_config(src, RenderConfig::new()).expect("render");
    // bg centre is (100, 50); the two dots straddle it.
    assert!(svg.contains(r#"id="one_d_a""#), "{}", svg);
    let cx = |id: &str| -> f64 {
        let at = svg.find(&format!("id=\"{}\"", id)).unwrap();
        let rest = &svg[at..];
        let v = rest.split("cx=\"").nth(1).unwrap().split('"').next().unwrap();
        v.parse().unwrap()
    };
    let mid = (cx("one_d_a") + cx("one_d_b")) / 2.0;
    assert!((mid - 100.0).abs() < 1.0, "dots centred on bg, got mid {}", mid);
}

#[test]
fn renamed_element_is_a_located_error_with_suggestions() {
    let src = include_str!("motion/pushpull.ail")
        .replace("show anna.hist.d4, chris.hist.d4", "show anna.hist.d5, chris.hist.d4");
    let err = render_with_config(&src, cfg()).unwrap_err().to_string();
    assert!(err.contains("undefined identifier 'anna.hist.d5'"), "{}", err);
    assert!(err.contains("line "), "{}", err);
    assert!(err.contains("did you mean: anna.hist.d"), "suggests dotted names: {}", err);
}

#[test]
fn reserved_word_parse_error_says_so() {
    let err = render_with_config("template \"t\" (row: 1) { rect a }", RenderConfig::new())
        .unwrap_err()
        .to_string();
    assert!(err.contains("reserved word"), "{}", err);
    assert!(err.contains("line 1"), "{}", err);
}

#[test]
fn backward_compatible_story_gets_default_motion() {
    let src = include_str!("../examples/agentic-loop-story.ail");
    let mut c = cfg().with_template_base_path(std::path::PathBuf::from("examples"));
    c.timeline = true;
    let t = render_with_config(src, c).expect("timeline");
    assert!(t.contains("show speech_bubble"), "{}", t);
    assert!(t.contains("speech_bubble translate: 0px 12px -> 0px 0px"), "default rise: {}", t);
}

#[test]
fn stagger_and_beats_place_statements_in_time() {
    let t = timeline(
        r#"
row r [gap: 10] { rect a  rect b  rect c }
keyframe "zero" { hide a, b, c }
keyframe "one" {
    show r.* [stagger: 0.1]
    then { hide a }
    after 0.05 { hide b }
}
"#,
    );
    assert!(t.contains("0.00s  0.50s  settle  show a"), "{}", t);
    assert!(t.contains("0.10s  0.50s  settle  show b"), "{}", t);
    assert!(t.contains("0.20s  0.50s  settle  show c"), "{}", t);
    // `then` starts when the stagger ends (0.2 + 0.5); `after` 0.05 later.
    assert!(t.contains("0.70s  0.25s  settle  hide a"), "{}", t);
    assert!(t.contains("0.75s  0.25s  settle  hide b"), "{}", t);
}

#[test]
fn appears_declares_an_entrance() {
    let t = timeline(
        r#"
rect a
rect b [appears: second]
keyframe "first" { show a }
keyframe "second" { }
"#,
    );
    assert!(t.contains("frame 1 \"second\""), "{}", t);
    let second = t.split("frame 1").nth(1).unwrap();
    assert!(second.contains("show b"), "b enters in second: {}", t);
}

#[test]
fn count_rolls_digits_and_samples_natively() {
    let src = r#"
text "€ 0" total [font_size: 40]
keyframe "a" {}
keyframe "b" { count total [to: 14900, format: "€ {:,}", duration: 1, ease: linear] }
"#;
    let mut c = RenderConfig::new();
    c.frame = Some("b".into());
    c.at = Some("50%".into());
    let svg = render_with_config(src, c).expect("render");
    assert!(svg.contains("€ 7,450"), "half way through a linear count: {}", svg);
}

#[test]
fn canvas_fixes_the_viewbox() {
    let svg = render_with_config(
        r#"
rect stage [width: 800, height: 450, canvas: true]
constrain stage.left = 0
constrain stage.top = 0
rect a [width: 100, height: 50]
constrain a.left = stage.left + 10
constrain a.top = stage.top + 10
"#,
        RenderConfig::new(),
    )
    .expect("render");
    assert!(svg.contains(r#"viewBox="0 0 800 450""#), "{}", svg);
}

#[test]
fn draw_to_station_resolves_a_fraction() {
    let t = timeline(
        r#"
circle a [size: 20]
circle b [size: 20]
circle c [size: 20]
path line_ab [through: [a, c], drawn: 0, stroke: accent-1, stroke_width: 6, fill: none]
constrain a.center_x = 0
constrain a.center_y = 0
constrain b.center_x = 100
constrain b.center_y = 0
constrain c.center_x = 400
constrain c.center_y = 0
keyframe "k" { draw line_ab [to: b] }
"#,
    );
    // b sits at a quarter of the way along: dash offset 1 - 0.25.
    assert!(t.contains("stroke-dashoffset: 1.001 -> 0.75"), "{}", t);
}

#[test]
fn metro_routing_uses_45_degree_runs() {
    let svg = render_with_config(
        r#"
circle a [size: 10]
circle b [size: 10]
path br [through: [a, b], routing: metro, stroke: accent-1, fill: none]
constrain a.center_x = 0
constrain a.center_y = 100
constrain b.center_x = 300
constrain b.center_y = 0
"#,
        RenderConfig::new(),
    )
    .expect("render");
    // Leaves a at 45 degrees (100 up over 100 across), then runs flat.
    assert!(svg.contains("M0.00 100.00 L100.00 0.00 L300.00 0.00"), "{}", svg);
}
