//! Stations on a `through:` line: room for their names, `spread: even`,
//! and the lints that report what the engine could not fix. Also the text
//! checks the relabel test asked for (touching text, text cut off).

use agent_illustrator::{render_with_config, render_with_lint, RenderConfig};

const STATIONS: &str = r#"
template "stn" (name: "Station") {
    circle dot [size: 30, fill: white, stroke: blue, stroke_width: 6]
    rect nm [caption_of: dot, label_position: below, label_offset: 8, label: name, font_size: 20, fill: none, stroke: none]
}
path trk [through: [a.dot, b.dot, c.dot, d.dot], stroke: blue, stroke_width: 10, fill: none SPREAD]
stn a [name: "Start"]
stn b [name: "A rather long station name"]
stn c [name: "Another long station"]
stn d [name: "End"]
constrain a.dot.center_x = 0
constrain a.dot.center_y = 100
constrain b.dot.center_y = a.dot.center_y
constrain c.dot.center_y = a.dot.center_y
constrain d.dot.center_y = a.dot.center_y
constrain d.dot.center_x = END
"#;

fn source(spread: bool, end: f64) -> String {
    STATIONS
        .replace(" SPREAD", if spread { ", spread: even" } else { "" })
        .replace("END", &end.to_string())
}

fn cx(svg: &str, id: &str) -> f64 {
    let tag = svg
        .lines()
        .find(|l| l.contains(&format!(r#"id="{id}""#)))
        .unwrap_or_else(|| panic!("{id} not rendered"));
    let at = tag.find(r#" cx=""#).expect("circle") + 5;
    tag[at..].split('"').next().unwrap().parse().unwrap()
}

fn lint(src: &str) -> Vec<String> {
    let (_, w) = render_with_lint(src, RenderConfig::new().with_lint(true)).expect("renders");
    w.into_iter().map(|w| w.message).collect()
}

#[test]
fn free_stations_get_room_for_their_names() {
    let svg = render_with_config(&source(false, 900.0), RenderConfig::new()).expect("renders");
    let (a, b, c, d) = (cx(&svg, "a_dot"), cx(&svg, "b_dot"), cx(&svg, "c_dot"), cx(&svg, "d_dot"));
    assert!(a < b && b < c && c < d, "in line order: {a} {b} {c} {d}");
    assert!(c - b > 200.0, "two long names need more than 200px: {}", c - b);
    assert!(lint(&source(false, 900.0)).is_empty(), "{:?}", lint(&source(false, 900.0)));
}

#[test]
fn spread_even_spaces_a_flat_run_evenly() {
    let svg = render_with_config(&source(true, 900.0), RenderConfig::new()).expect("renders");
    let xs = [cx(&svg, "a_dot"), cx(&svg, "b_dot"), cx(&svg, "c_dot"), cx(&svg, "d_dot")];
    for w in xs.windows(2) {
        assert!((w[1] - w[0] - 300.0).abs() < 1.0, "even steps of 300: {xs:?}");
    }
}

#[test]
fn stations_pinned_too_close_are_reported() {
    let msgs = lint(&source(true, 400.0));
    assert!(
        msgs.iter().any(|m| m.contains("stations") && m.contains("names need")),
        "{msgs:?}"
    );
}

#[test]
fn touching_text_on_one_line_is_reported() {
    let msgs = lint(
        r#"
rect a [fill: none, stroke: none, label: "style.css"]
rect b [fill: none, stroke: none, label: "246 lines"]
constrain b.center_y = a.center_y
constrain b.left = a.right - 20
"#,
    );
    assert!(msgs.iter().any(|m| m.contains("touch")), "{msgs:?}");
}

#[test]
fn text_cut_off_by_a_canvas_is_an_error() {
    let (_, w) = render_with_lint(
        r#"
rect stage [width: 300, height: 200, canvas: true, fill: white]
rect a [width: 40, height: 40, label: "a label that is far far far longer than the stage", label_position: right]
constrain a.right = stage.right - 10
constrain a.top = stage.top + 10
"#,
        RenderConfig::new().with_lint(true),
    )
    .expect("renders");
    assert!(w.iter().any(|w| w.message.contains("cut off")), "{:?}", w.iter().map(|w| &w.message).collect::<Vec<_>>());
}

#[test]
fn unsized_text_renders_at_the_size_it_was_measured_at() {
    let svg = render_with_config(r#"text "Hello" t"#, RenderConfig::new()).expect("renders");
    let root = svg.lines().find(|l| l.starts_with("<svg")).expect("root");
    assert!(root.contains(r#"font-size="14""#), "{root}");
}
