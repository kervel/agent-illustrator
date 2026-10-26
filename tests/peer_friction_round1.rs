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

#[test]
fn fill_opacity_animates_from_its_default() {
    let src = r#"
row r [gap: 20] {
  rect a [size: 60, fill: role-primary]
  rect b [size: 60, fill: role-primary, fill_opacity: 0.5]
}
keyframe "s" { }
keyframe "t" { transform a [fill_opacity: 0.2, duration: 1]
  transform b [fill_opacity: 1, duration: 1] }
"#;
    let svg = render_with_config(src, RenderConfig { animate: true, ..Default::default() }).unwrap();
    assert!(svg.contains(r#"[[0,"1"],[1,"0.2"]]"#), "undeclared base is 1:\n{svg}");
    assert!(svg.contains(r#"[[0,"0.5"],[1,"1"]]"#), "declared base kept:\n{svg}");
    let (_s, w) = render_with_lint(src, RenderConfig::new().with_lint(true)).unwrap();
    assert!(w.iter().all(|w| !w.message.contains("cannot be animated")), "{:?}", w.iter().map(|w| &w.message).collect::<Vec<_>>());
}

#[test]
fn contains_takes_a_path_into_a_component() {
    let src = r#"
template "card" {
  rect x [size: 20]
  rect y [size: 20]
}
card c1
rect frame [fill: none, stroke: role-rule]
constrain frame contains c1.x, c1.y [padding: 10]
"#;
    render_with_config(src, RenderConfig::default()).expect("a dotted path in contains");
}

fn lint_msgs(src: &str) -> Vec<String> {
    let (_s, w) = render_with_lint(src, RenderConfig::new().with_lint(true)).unwrap();
    w.into_iter().map(|w| w.message).collect()
}

#[test]
fn overlaps_on_a_grid_covers_what_is_inside_it() {
    let src = r#"
grid g [cols: 2, gap: 10] {
  rect a [size: 40, fill: role-surface]
  rect b [size: 40, fill: role-surface]
}
rect hl [width: 70, height: 40, fill: none, stroke: role-primary, overlaps: g]
constrain hl.center_x = g.center_x
constrain hl.center_y = g.center_y
"#;
    assert!(lint_msgs(src).is_empty(), "{:?}", lint_msgs(src));
}

#[test]
fn many_cells_hit_by_one_thing_is_one_warning_naming_the_fix() {
    let src = r#"
grid g [cols: 2, gap: 10] {
  rect [size: 40, fill: role-surface]
  rect [size: 40, fill: role-surface]
  rect [size: 40, fill: role-surface]
  rect [size: 40, fill: role-surface]
}
rect hl [width: 70, height: 70, fill: none, stroke: role-primary]
constrain hl.center_x = g.center_x
constrain hl.center_y = g.center_y
"#;
    let m = lint_msgs(src);
    assert_eq!(m.len(), 1, "{m:?}");
    assert!(m[0].contains("overlaps 4 cells") && m[0].contains("`overlaps: g`"), "{m:?}");
}

#[test]
fn grid_cells_are_reachable_through_a_template_instance_and_stay_put() {
    let src = r#"
template "board" {
  grid g [cols: 2, gap: 6] {
    rect [size: 30, fill: role-surface]
    rect [size: 30, fill: role-surface]
    rect [size: 30, fill: role-surface]
    rect [size: 30, fill: role-surface]
  }
  rect hl [width: 36, height: 36, fill: none, stroke: role-primary, overlaps: g]
  constrain hl.center_x = g.cell(1,1).center_x
  constrain hl.center_y = g.cell(1,1).center_y
}
board b1
rect pin [size: 10, fill: role-primary]
constrain pin.center_x = b1.g.cell(0,1).center_x
constrain pin.center_y = b1.g.cell(0,1).top - 14
"#;
    assert!(lint_msgs(src).is_empty(), "{:?}", lint_msgs(src));
    let svg = render_with_config(src, RenderConfig::default()).unwrap();
    // hl is centred on cell (1,1): x 43..79 around 61.
    assert!(svg.contains(r#"id="b1_hl"  x="43" y="43"#) || svg.contains(r#"id="b1_hl" class"#), "{svg}");
    assert!(svg.contains(r#"x="43" y="43" width="36""#), "hl on its cell:\n{svg}");
}

#[test]
fn a_contains_frame_grows_as_hidden_members_appear() {
    let src = r#"
rect ctx [fill: role-surface, stroke: role-ink, stroke_width: 3, corner_radius: 16]
row toks [gap: 14, align: center, overlaps: ctx] {
    rect t1 [width: 100, height: 50, label: "The"]
    rect t2 [width: 100, height: 50, label: "cat"]
    rect t3 [width: 100, height: 50, label: "sat", appears: later]
}
constrain ctx contains toks [padding: 20]
keyframe "a" { }
keyframe "b" { show t3 }
"#;
    let width = |f: &str| {
        let svg = render_with_config(src, RenderConfig { frame: Some(f.into()), ..Default::default() }).unwrap();
        let at = svg.find(r#"id="ctx""#).unwrap();
        let rest = &svg[at..];
        let i = rest.find(r#" width=""#).unwrap() + 8;
        rest[i..i + rest[i..].find('"').unwrap()].parse::<f64>().unwrap()
    };
    let (a, b) = (width("a"), width("b"));
    assert!(b > a + 100.0, "the frame grows with the row: {a} -> {b}");
    assert_eq!(b, 378.0, "all visible: the whole row, as written");
}

#[test]
fn a_hidden_connection_does_not_push_a_visible_one_aside() {
    let src = r#"
row r [gap: 150] {
  rect a [size: 50]
  rect b [size: 50]
}
a.right -> b.left as go [appears: later]
b.left -> a.right as back
keyframe "s" { }
keyframe "t" { show go }
"#;
    let d = |f: &str| {
        let svg = render_with_config(src, RenderConfig { frame: Some(f.into()), ..Default::default() }).unwrap();
        svg.lines().filter(|l| l.contains(" d=\"M2")).map(str::to_string).collect::<Vec<_>>().join("\n")
    };
    assert!(d("s").contains("M205 30 L"), "alone it runs on the centre line:\n{}", d("s"));
    assert!(d("t").contains(" 39 ") || d("t").contains("39"), "with go shown they share lanes:\n{}", d("t"));
}
