//! `via:` on a direct connector draws one polyline; `contains` takes per-axis padding.

use agent_illustrator::{render, render_with_lint, RenderConfig};

#[test]
fn direct_connector_with_via_is_one_polyline_with_sharp_corners() {
    let out = render(r#"
rect a [width: 20, height: 20]
rect b [width: 20, height: 20]
constrain b.center_x = a.center_x
constrain b.top = a.bottom + 60
rect w1 [size: 2]
rect w2 [size: 2]
constrain w1.center_x = a.center_x + 50
constrain w1.center_y = a.center_y
constrain w2.center_x = w1.center_x
constrain w2.center_y = b.center_y
a -> b [routing: direct, via: [w1, w2]]"#).unwrap();
    let d = out
        .lines()
        .find(|l| l.contains("ai-connection\""))
        .expect("connection path");
    assert_eq!(d.matches(" L").count(), 3, "start, two corners, end: {d}");
    assert!(!d.contains(" C"), "no Bezier: {d}");
}

#[test]
fn contains_padding_can_differ_per_axis() {
    let out = render(r#"
rect a [width: 20, height: 20]
rect box [fill: none]
constrain box contains a [padding_x: 30, padding_y: 5]"#).unwrap();
    // box is 20+2*30 wide and 20+2*5 tall
    assert!(out.contains(r#"width="80""#) && out.contains(r#"height="30""#), "{out}");
}

#[test]
fn template_parameters_can_name_outside_elements() {
    let out = render(r#"
rect a [width: 20, height: 20]
rect b [width: 20, height: 20]
constrain b.left = a.right + 100
template "link" (src: "", dst: "", what: "") {
  src -> dst [routing: direct, label: what]
}
template "box" (first: "", last: "") {
  rect f [fill: none, overlaps: all]
  constrain f contains first, last [padding_x: 10, padding_y: 10]
}
link l1 [src: a, dst: b, what: "hi"]
box bx [first: a, last: b]"#).unwrap();
    assert!(out.contains(">hi<"), "connection drawn");
    assert!(out.contains(r#"id="bx_f""#));
    // frame spans a (x=0) to b right edge (140) plus 10 on each side
    assert!(out.contains(r#"x="-10""#) && out.contains(r#"width="160""#), "{out}");
}

#[test]
fn overlaps_all_silences_the_overlap_finding() {
    let ail = |extra: &str| format!(r#"
rect a [width: 60, height: 60]
rect cover [width: 30, height: 30{extra}]
constrain cover.center_x = a.center_x
constrain cover.center_y = a.center_y"#);
    let warns = |src: &str| render_with_lint(src, RenderConfig::new().with_lint(true)).map(|(_, w)| w.len()).unwrap_or(99);
    assert!(warns(&ail("")) > 0, "overlap is reported without the marker");
    assert_eq!(warns(&ail(", overlaps: all")), 0);
}
