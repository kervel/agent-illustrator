//! `via:` on a direct connector draws one polyline; `contains` takes per-axis padding.

use agent_illustrator::render;

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
