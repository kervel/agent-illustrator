//! Retest of the round-1 fixes on a real deck: layouts that came out different
//! on every run, a route variant drawn at opacity 0, an overlay on a grid in a
//! template, and a cable that detoured to reach the side of a point.
use agent_illustrator::{render_with_config, render_with_lint, RenderConfig};

fn width_of(svg: &str, id: &str) -> f64 {
    let at = svg.find(&format!(r#"id="{id}""#)).unwrap();
    let rest = &svg[at..];
    let i = rest.find(r#" width=""#).unwrap() + 8;
    rest[i..i + rest[i..].find('"').unwrap()].parse().unwrap()
}

const FOLLOWER: &str = r#"
rect stage [width: 1000, height: 400, fill: none, stroke: none, canvas: true]
constrain stage.left = 0
constrain stage.top = 0
row toks [gap: 14] {
    rect t1 [width: 100, height: 50, label: "a"]
    rect t2 [width: 100, height: 50, label: "b", appears: later]
    rect t3 [width: 100, height: 50, label: "c", appears: later]
}
constrain toks.left = stage.left + 100
constrain toks.center_y = stage.top + 200
rect box [fill: none, stroke: role-ink, stroke_width: 3, z_order: -1]
constrain box contains toks [padding: 20]
rect head [height: 40, fill: role-surface-2, stroke: none, clip: box]
constrain head.left = box.left
constrain head.top = box.top
constrain head.width = box.width
rect other [width: 80, height: 80]
constrain other.left = box.right + 40
constrain other.center_y = box.center_y
keyframe "one" { show box }
keyframe "two" { show t2 [enter: pop] }
"#;

#[test]
fn what_follows_a_growing_contains_frame_is_the_same_every_run() {
    let render = || {
        render_with_config(FOLLOWER, RenderConfig { frame: Some("one".into()), ..Default::default() }).unwrap()
    };
    let first = render();
    assert_eq!(width_of(&first, "box"), 150.0, "only `a` is on screen");
    assert_eq!(width_of(&first, "head"), 150.0, "head follows the frame");
    for _ in 0..40 {
        assert_eq!(render(), first, "same input, same picture");
    }
}

#[test]
fn a_route_variant_is_not_hidden_inside_its_own_wrapper() {
    let src = r#"
rect stage [width: 600, height: 300, fill: none, stroke: none, canvas: true]
constrain stage.left = 0
constrain stage.top = 0
rect a [width: 80, height: 60]
rect b [width: 80, height: 60]
constrain a.left = stage.left + 60
constrain a.center_y = stage.top + 100
constrain b.left = stage.left + 400
constrain b.center_y = stage.top + 100
a.right -> b.left
keyframe "one" { show a }
keyframe "two" { transform b [dy: 120] }
"#;
    let svg = render_with_config(src, RenderConfig { animate: true, ..Default::default() }).unwrap();
    let at = svg.find(r#"<g class="aivar-"#).expect("the route changes shape: a variant exists");
    let variant = &svg[at..at + svg[at..].find("</g>").unwrap()];
    assert!(!variant.contains("opacity=\"0\""), "the wrapper fades it, not the path:\n{variant}");
}

#[test]
fn overlaps_on_a_grid_covers_plain_cells_inside_a_template() {
    let src = r#"
template "head" () {
    grid heat [cols: 2, rows: 1, gap: 8, cell_width: 100, cell_height: 50] {
        rect [at: [0,0], fill: white, stroke: gray]
        rect [at: [0,1], fill: white, stroke: gray]
    }
}
head h
grid wts [cols: 2, rows: 1, gap: 8, cell_width: 100, cell_height: 50, overlaps: h] {
    rect f0 [at: [0,0], fill: steelblue, fill_opacity: 1.0, stroke: none]
    rect f1 [at: [0,1], fill: steelblue, fill_opacity: 0.3, stroke: none]
}
constrain wts.left = h.heat.left
constrain wts.top = h.heat.top
"#;
    let (_s, w) = render_with_lint(src, RenderConfig::new().with_lint(true)).unwrap();
    assert!(w.is_empty(), "{:?}", w.iter().map(|w| &w.message).collect::<Vec<_>>());
}

#[test]
fn a_cable_into_a_point_bends_once() {
    let src = r#"
rect stage [width: 600, height: 400, fill: none, stroke: none, canvas: true]
constrain stage.left = 0
constrain stage.top = 0
template "plug" () { rect body [width: 40, height: 40] anchor plug_top [position: body.top, direction: up] }
plug a
point p
constrain a.left = stage.left + 100
constrain a.top = stage.top + 300
constrain p.center_x = stage.left + 160
constrain p.center_y = stage.top + 240
a.plug_top -> p
"#;
    let svg = render_with_config(src, RenderConfig::default()).unwrap();
    let d = svg.split(r#"class="ai-connection""#).nth(1).and_then(|s| s.split(" d=\"").nth(1)).and_then(|s| s.split('"').next()).unwrap();
    assert_eq!(d.matches('L').count(), 2, "up, then across: one bend, got {d}");
}

#[test]
fn a_strip_gives_each_panel_its_own_clip() {
    let src = r#"
rect frame [width: 200, height: 120, fill: none, stroke: role-ink]
rect bar [width: 700, height: 36, fill: role-primary, stroke: none, clip: frame]
constrain bar.left = frame.left
constrain bar.top = frame.top
keyframe "start" { }
keyframe "grow" { transform frame [width: 600] }
"#;
    let svg = agent_illustrator::render_frames_strip(src, RenderConfig::default(), "grow").unwrap();
    let ids: Vec<&str> = svg.split(r#"<clipPath id=""#).skip(1).map(|s| s.split('"').next().unwrap()).collect();
    assert_eq!(ids.len(), 5, "{ids:?}");
    let mut uniq = ids.clone();
    uniq.sort();
    uniq.dedup();
    assert_eq!(uniq.len(), 5, "one clip per panel: {ids:?}");
}

#[test]
fn a_caption_of_an_artwork_comes_with_it() {
    let src = r#"
template "plug" { rect body [width: 40, height: 40] }
plug p [appears: later]
rect p_nm [caption_of: p, label_position: right, label: "artwork caption", fill: none, stroke: none]
keyframe "start" { }
keyframe "show" { show p }
"#;
    let svg = render_with_config(src, RenderConfig { frame: Some("start".into()), ..Default::default() }).unwrap();
    assert!(!svg.contains("artwork caption"), "hidden with its subject:\n{svg}");
}
