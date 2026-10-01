//! Timing by events, beats, named layouts, component states, records,
//! points, one title source, the storyboard (`--states`) and the lints that
//! point at the old spellings.

use agent_illustrator::{render_with_config, render_with_lint, RenderConfig};

fn render(src: &str) -> String {
    render_with_config(src, RenderConfig::new()).unwrap_or_else(|e| panic!("{e}"))
}

fn timeline(src: &str) -> String {
    let mut c = RenderConfig::new();
    c.timeline = true;
    render_with_config(src, c).unwrap_or_else(|e| panic!("{e}"))
}

fn states(src: &str) -> String {
    let mut c = RenderConfig::new();
    c.states = true;
    render_with_config(src, c).unwrap_or_else(|e| panic!("{e}"))
}

fn lint(src: &str) -> Vec<String> {
    let (_, w) = render_with_lint(src, RenderConfig::new().with_lint(true)).expect("renders");
    w.into_iter().map(|w| w.message).collect()
}

fn frame(src: &str, name: &str) -> String {
    let mut c = RenderConfig::new();
    c.frame = Some(name.to_string());
    render_with_config(src, c).unwrap_or_else(|e| panic!("{e}"))
}

fn attr(svg: &str, id: &str, a: &str) -> f64 {
    let at = svg.find(&format!(r#"id="{id}""#)).unwrap_or_else(|| panic!("no {id}"));
    let rest = &svg[at..];
    let k = rest.find(&format!(r#" {a}=""#)).unwrap_or_else(|| panic!("no {a} on {id}"));
    let v = &rest[k + a.len() + 3..];
    v[..v.find('"').unwrap()].parse().unwrap()
}

const LINE: &str = r#"
circle a [size: 20, appears: later]
circle b [size: 20, appears: later]
row r [gap: 200] { rect ra [width: 20, height: 20, fill: none, stroke: none]  rect rb [width: 20, height: 20, fill: none, stroke: none] }
constrain a.center = ra.center
constrain b.center = rb.center
path l [through: [a, b], drawn: 0, extend_start: 100, stroke_width: 4, fill: none]
"#;

#[test]
fn when_a_line_reaches_an_element_its_block_starts_there() {
    let src = format!("{LINE}\nkeyframe \"k\" {{ draw l [duration: 1] \n when l reaches b {{ show b }} \n when l reaches a {{ show a }} }}");
    let t = timeline(&src);
    let start = |id: &str| -> f64 {
        let line = t.lines().find(|l| l.contains(&format!("show {id}"))).unwrap_or_else(|| panic!("{t}"));
        line.trim().split('s').next().unwrap().parse().unwrap()
    };
    assert!(start("a") > 0.0 && start("a") < start("b"), "{t}");
    assert!(lint(&src).is_empty(), "{:?}", lint(&src));
}

#[test]
fn showing_a_station_before_its_line_arrives_is_linted() {
    let src = format!("{LINE}\nkeyframe \"k\" {{ draw l [duration: 1] \n show a, b }}");
    let w = lint(&src);
    assert!(w.iter().any(|m| m.contains("when l reaches b")), "{w:?}");
}

#[test]
fn chained_after_offsets_are_linted() {
    let src = "row r { rect a [appears: later]\n rect b [appears: later]\n rect c [appears: later] }\n\
               keyframe \"k\" { show a\n after 0.2 { show b }\n after 0.3 { show c } }";
    let w = lint(src);
    assert!(w.iter().any(|m| m.contains("`after` offsets in a row (0.2 + 0.3")), "{w:?}");
    let ok = "row r { rect a [appears: later]\n rect b [appears: later] }\n\
              keyframe \"k\" { show a\n when a shown { show b } }";
    assert!(lint(ok).is_empty(), "{:?}", lint(ok));
}

#[test]
fn beats_are_named_and_later_ones_start_after_them() {
    let src = "row r { rect a [appears: later]\n rect b [appears: later] }\n\
               keyframe \"k\" { beat first { show a [duration: 0.4] }\n after first + 0.1 { show b } }";
    let t = timeline(src);
    assert!(t.contains("0.50s"), "b starts when the beat ends, plus 0.1: {t}");
}

const LAYOUTS: &str = r#"
rect stage [width: 800, height: 400, fill: none]
rect f [width: 100, height: 60]
constrain stage.left = 0
constrain stage.top = 0
constrain f.center_x = stage.center_x
constrain f.top = stage.top + 20
layout aside {
    constrain f.left = stage.left + 40
}
keyframe "a" { }
keyframe "b" { use layout aside }
keyframe "c" { use layout default }
"#;

#[test]
fn a_named_layout_replaces_the_base_placement_and_default_restores_it() {
    let x = |k: &str| attr(&frame(LAYOUTS, k), "f", "x");
    assert_eq!(x("a"), 350.0);
    assert_eq!(x("b"), 40.0);
    assert_eq!(x("c"), 350.0);
    let s = states(LAYOUTS);
    assert!(s.contains("layout: aside") && s.contains("layout: default"), "{s}");
    assert!(!s.contains("constraints off"), "the storyboard names the layout, not its constraints: {s}");
}

#[test]
fn an_unknown_layout_is_named() {
    let src = LAYOUTS.replace("use layout aside", "use layout asid");
    let e = render_with_config(&src, RenderConfig::new()).unwrap_err().to_string();
    assert!(e.contains("layout asid"), "{e}");
}

const STATES: &str = r#"
template "check" (says: "check", then_says: "") {
    rect bg [fill: orange, stroke: none, opacity: 0]
    rect txt [fill: none, stroke: none, label: says]
    circle tick [size: 20, appears: later]
    constrain bg contains txt [padding: 4]
    constrain tick.left = bg.right + 6
    constrain tick.center_y = bg.center_y
    state asked { transform bg [opacity: 1] }
    state done { transform txt [label: then_says, swap: fade]; show tick [enter: pop] }
}
check review [says: "Ben asks", then_says: "Approved"]
keyframe "a" { }
keyframe "b" { set review asked }
keyframe "c" { set review done [duration: 0.3] }
keyframe "d" { set review asked }
"#;

#[test]
fn set_enters_a_state_and_leaves_what_other_states_set() {
    let s = states(STATES);
    assert!(s.contains("~ review.bg [opacity: 1]"), "{s}");
    assert!(s.contains(r#"~ review.txt [label: "Approved"]"#), "the parameter fills the state: {s}");
    assert!(s.contains("~ review.bg [opacity: initial]"), "done does not set bg: it goes back: {s}");
    let d = s.split("step 3").nth(1).unwrap();
    assert!(d.contains("- review.tick") && d.contains("review.txt [label: initial]"), "{d}");
    let t = timeline(STATES);
    assert!(t.contains("0.30s  pop     show [enter: pop] review.tick"), "the set's options reach every statement: {t}");
}

#[test]
fn set_names_the_states_a_component_has() {
    let src = STATES.replace("set review done [", "set review don [");
    let e = render_with_config(&src, RenderConfig::new()).unwrap_err().to_string();
    assert!(e.contains("no state `don`") && e.contains("asked, done"), "{e}");
}

#[test]
fn records_fill_starred_instances_row_by_row() {
    let src = r#"
template "f" (name: "x", c: red) { rect r [width: 40, height: 20, fill: c, label: name] }
row files { f k* [items: [{name: "a.py", c: blue}, {name: "b.py", c: green}]] }
"#;
    let svg = render(src);
    assert!(svg.contains("a.py") && svg.contains("b.py"));
    assert!(svg.contains("blue") && svg.contains("green"), "{svg}");
}

#[test]
fn a_point_is_an_invisible_place_with_no_size() {
    let svg = render("rect a [width: 100, height: 50]\npoint hub\nconstrain hub.center = a.center");
    assert_eq!(attr(&svg, "hub", "width"), 0.0);
    assert_eq!(attr(&svg, "hub", "x"), attr(&svg, "a", "x") + 50.0);
}

#[test]
fn the_heading_follows_keyframe_titles() {
    let src = r#"
text "Snapshots" heading [font_size: 30]
motion [title: heading, title_swap: roll]
keyframe "a" [title: "Snapshots"] { }
keyframe "b" [title: "Go back"] { }
keyframe "c" { }
"#;
    let t = timeline(src);
    assert!(t.contains("transform [swap: roll] heading"), "{t}");
    assert_eq!(t.matches("transform [swap: roll] heading").count(), 1, "only where the title changes: {t}");
    assert!(frame(src, "c").contains("Go back"));
}

#[test]
fn appears_and_a_show_in_the_same_frame_is_linted() {
    let src = "rect a [appears: k]\nkeyframe \"k\" { show a [enter: pop] }";
    assert!(lint(src).iter().any(|m| m.contains("comes on twice")), "{:?}", lint(src));
}

#[test]
fn a_swap_to_something_elsewhere_is_linted() {
    let apart = "row r [gap: 100] { rect a [width: 40, height: 20]\n rect b [width: 40, height: 20, appears: later] }\n\
                 keyframe \"k\" { swap a -> b }";
    assert!(lint(apart).iter().any(|m| m.contains("swap a -> b")), "{:?}", lint(apart));
    let together = "rect a [width: 40, height: 20]\nrect b [width: 40, height: 20, appears: later]\n\
                    constrain b.center = a.center\nkeyframe \"k\" { swap a -> b }";
    assert!(lint(together).is_empty(), "{:?}", lint(together));
}

#[test]
fn a_starred_constraint_pairs_instances_by_index() {
    let src = r#"
row a [gap: 50] { rect p* [width: [40, 60, 30], height: 20] }
group b { circle q* [size: [10, 12, 14]] }
constrain q*.center = p*.center
"#;
    let svg = render(src);
    for i in 0..3 {
        let (p, q) = (format!("p{i}"), format!("q{i}"));
        let pc = attr(&svg, &p, "x") + attr(&svg, &p, "width") / 2.0;
        assert!((attr(&svg, &q, "cx") - pc).abs() < 0.5, "{q} sits on {p}: {svg}");
    }
    let e = render_with_config("rect p* [width: [1, 2]]\ncircle q* [size: [1, 2, 3]]\nconstrain q*.center = p*.center", RenderConfig::new())
        .unwrap_err()
        .to_string();
    assert!(e.contains("must match"), "{e}");
}

#[test]
fn a_plain_element_has_states_and_default_undoes_them() {
    let src = r#"
rect status [label: "works", stroke: green, label_fill: green]
state status broken { transform self [label: "broken", stroke: red, label_fill: red, swap: fade] }
keyframe "a" { }
keyframe "b" { set status broken }
keyframe "c" { set status default }
"#;
    let s = states(src);
    assert!(s.contains(r#"~ status [label: "broken", label_fill: red, stroke: red]"#), "{s}");
    assert!(s.contains("~ status [label: initial, label_fill: initial, stroke: initial]"), "{s}");
    assert!(lint(src).is_empty(), "{:?}", lint(src));
}

#[test]
fn the_timeline_prints_when_each_event_happens() {
    let src = "row r { rect a [appears: later]\n rect b [appears: later] }\nkeyframe \"k\" { show a [duration: 0.4]\n when a shown + 0.1 { show b } }";
    let t = timeline(src);
    assert!(t.contains("when a shown + 0.1 (= 0.40s, starts 0.50s)"), "{t}");
}

#[test]
fn an_instance_opacity_dims_the_instance() {
    let src = "template \"t\" (x: 1) { rect a [width: 20, height: 20]\n rect b [width: 20, height: 20] }\nt i [opacity: 0.3]\nkeyframe \"a\" { }\nkeyframe \"k\" { transform i [opacity: 1] }";
    let t = timeline(src);
    assert!(t.contains("i opacity: 0.3 -> 1"), "{t}");
}

#[test]
fn unnamed_dots_in_a_row_are_not_spaced_as_stations_and_pins_hold() {
    let src = r#"
rect stage [width: 1600, height: 900, fill: none, canvas: true]
constrain stage.left = 0
constrain stage.top = 0
template "history" () {
    row dots [gap: 34, align: center] {
        circle d1 [size: 22]
        circle d2 [size: 22]
        circle d3 [size: 22]
    }
    path track [through: [d1, d2, d3], stroke_width: 5, fill: none]
}
template "machine" (title: "Laptop") {
    rect bg [width: 300, height: 150]
    rect head [width: 300, height: 48, label: title]
    history hist
    constrain head.top = bg.top
    constrain head.left = bg.left
    constrain hist.center_x = bg.center_x
    constrain hist.center_y = bg.center_y + 22
}
machine hub [title: "A long title for the server machine"]
constrain hub.bg.center_x = stage.center_x
constrain hub.bg.top = stage.top + 150
"#;
    let w = lint(src);
    assert!(w.is_empty(), "{w:?}");
    let svg = render(src);
    assert_eq!(attr(&svg, "hub_bg", "x"), 650.0, "the author's pin holds");
    assert_eq!(attr(&svg, "hub_hist_d2", "cx") - attr(&svg, "hub_hist_d1", "cx"), 56.0, "the row's gap, not a station's");
}

#[test]
fn a_caption_on_a_link_is_not_a_station_but_the_link_end_is() {
    let base = r#"
rect a [width: 100, height: 40]
rect b [width: 100, height: 40, appears: later]
rect cmd [width: 60, height: 20, appears: later]
constrain a.top = 0
constrain b.top = 300
constrain b.left = a.left
constrain cmd.center_x = a.center_x
constrain cmd.center_y = midpoint(a.bottom, b.top)
a.bottom -> b.top as l [appears: later]
keyframe "k" { show l [enter: draw, duration: 1]
"#;
    let caption = format!("{base} show cmd }}");
    let early = |src: &str| lint(src).into_iter().filter(|m| m.contains("only reaches it")).collect::<Vec<_>>();
    assert!(early(&caption).is_empty(), "{:?}", early(&caption));
    let end = format!("{base} show b }}");
    assert!(lint(&end).iter().any(|m| m.contains("`when l shown { ... }`")), "{:?}", lint(&end));
    let fixed = format!("{base} when l shown {{ show b }} }}");
    assert!(early(&fixed).is_empty(), "{:?}", early(&fixed));
}

#[test]
fn midpoint_takes_edges_for_the_middle_of_a_gap() {
    let src = "rect a [width: 100, height: 40]\nrect b [width: 100, height: 40]\nrect c [width: 60, height: 20]\n\
               constrain a.top = 0\nconstrain b.top = 200\nconstrain b.left = a.left\n\
               constrain c.center_y = midpoint(a.bottom, b.top)";
    assert_eq!(attr(&render(src), "c", "y"), 110.0);
    let e = render_with_config("rect a\nrect c\nconstrain c.center_y = (a.bottom + a.top) / 2", RenderConfig::new())
        .unwrap_err()
        .to_string();
    assert!(e.contains("midpoint(a.bottom, b.top)"), "{e}");
}

#[test]
fn an_instance_can_override_one_of_its_parts() {
    let src = r#"
template "h" () { circle d1 [size: 10]
 circle d2 [size: 10] }
template "m" () { rect bg [width: 100, height: 50]
 h hist }
row r { m a [hist.appears: later]
 m b [bg.fill: red] }
keyframe "k" { }
keyframe "j" { show a.hist }
"#;
    let s = states(src);
    assert!(s.contains("a.hist") && !s.contains("b.hist"), "only a's history enters later: {s}");
    assert!(render(src).contains(r#"id="b_bg""#));
    let svg = render(src);
    let at = svg.find(r#"id="b_bg""#).unwrap();
    assert!(svg[at..at + 300].contains("red"), "b.bg is red");
}

#[test]
fn lint_messages_name_parts_with_dots() {
    let src = "template \"m\" () { rect bg [width: 100, height: 50]\n rect head [width: 100, height: 20] }\nm hub\n\
               rect stage [width: 800, height: 400]\nconstrain hub.bg.center_x = stage.center_x\nconstrain hub.bg.left = 3";
    let w = lint(src);
    assert!(w.iter().any(|m| m.contains("hub.bg.left")) && !w.iter().any(|m| m.contains("hub_bg")), "{w:?}");
}

#[test]
fn a_through_line_runs_under_its_stations_whatever_the_order() {
    let src = "row dots [gap: 30] { circle d1 [size: 20]\n circle d2 [size: 20] }\n\
               path track [through: [d1, d2], stroke_width: 4, fill: none]";
    let svg = render(src);
    assert!(svg.find(r#"id="track""#).unwrap() < svg.find(r#"id="d1""#).unwrap(), "track drawn first");
    let above = src.replace("fill: none]", "fill: none, z_order: 1]");
    let svg = render(&above);
    assert!(svg.find(r#"id="track""#).unwrap() > svg.find(r#"id="d2""#).unwrap(), "z_order: 1 puts it on top");
}

#[test]
fn a_flight_in_arrives_when_it_lands() {
    let src = r#"
row r [gap: 200] { circle a [size: 20]
 circle b [size: 20, appears: later] }
path l [through: [a, b], drawn: a, stroke_width: 4, fill: none]
keyframe "k" { }
keyframe "j" { show b [from: a, duration: slow]
 draw l [to: b, duration: slow] }
"#;
    let early: Vec<String> = lint(src).into_iter().filter(|m| m.contains("only reaches it")).collect();
    assert!(early.is_empty(), "{early:?}");
}

#[test]
fn a_clipped_header_stays_inside_its_frame() {
    let src = "rect bg [width: 300, height: 150, stroke_width: 4, corner_radius: 16]\n\
               rect head [width: 300, height: 48, fill: red, stroke: none, clip: bg]\n\
               constrain head.top = bg.top\nconstrain head.left = bg.left";
    let svg = render(src);
    assert!(svg.contains(r#"<clipPath id="ai-clip-head"><rect x="2" y="2" width="296" height="146" rx="14" ry="14"/>"#), "{svg}");
    assert!(svg.contains(r#"clip-path="url(#ai-clip-head)""#));
    assert!(lint(src).is_empty(), "{:?}", lint(src));
}

#[test]
fn a_column_without_padding_is_flush() {
    let svg = render("col c [gap: 0, padding: 0] { rect a [width: 50, height: 20]\n rect b [width: 50, height: 20] }\nconstrain c.left = 0\nconstrain c.top = 0");
    assert_eq!(attr(&svg, "a", "x"), 0.0);
    assert_eq!(attr(&svg, "a", "y"), 0.0);
}

#[test]
fn a_through_line_stays_above_a_backdrop_declared_before_it() {
    // v0.2.8 sent through-lines to the back of everything: under the slide's
    // opaque stage, so every git scene lost its lines.
    let src = "rect stage [width: 800, height: 400, fill: white, canvas: true]\nconstrain stage.left = 0\nconstrain stage.top = 0\n\
               row dots [gap: 60] { circle a [size: 20]\n circle b [size: 20] }\n\
               path track [through: [a, b], stroke_width: 6, fill: none]";
    let svg = render(src);
    let at = |id: &str| svg.find(&format!(r#"id="{id}""#)).unwrap();
    assert!(at("stage") < at("track"), "the line is painted over the stage");
    assert!(at("track") < at("a"), "and under its stations");
}

#[test]
fn default_reverts_what_a_state_did_through_a_selector() {
    let src = r#"
template "box" () {
    row meter { rect m1 [width: 10, height: 10, fill: green]
      rect m2 [width: 10, height: 10, fill: green, appears: later] }
    state busy { show m2 [enter: grow]; transform meter.* [fill: red] }
}
box api
keyframe "a" { }
keyframe "b" { set api busy }
keyframe "c" { set api default }
"#;
    let s = states(src);
    let c = s.split("step 2").nth(1).unwrap();
    assert!(c.contains("- api.m2"), "{c}");
    assert!(c.contains("api.m1 [fill: initial]"), "the red goes back: {c}");
    let t = timeline(src);
    assert!(t.contains("-> green"), "tweens back to the declared colour: {t}");
}

#[test]
fn a_pivot_is_where_rotation_turns() {
    let src = "rect lid [width: 100, height: 10, pivot: left]\nconstrain lid.left = 50\nconstrain lid.top = 50\n\
               keyframe \"a\" { }\nkeyframe \"b\" { transform lid [rotation: -28] }";
    assert!(frame(src, "b").contains("rotate(-28 50 55)"), "static frames turn about the hinge");
    let mut c = RenderConfig::new();
    c.animate = true;
    let svg = render_with_config(src, c).unwrap();
    assert!(svg.contains("transform-origin:50px 55px"), "and so does the player");
}

#[test]
fn something_shown_and_hidden_within_a_frame_is_not_hidden_throughout() {
    let src = "row r [gap: 40] { rect a [width: 30, height: 30]\n rect b [width: 30, height: 30]\n rect tip [width: 60, height: 20, appears: later] }\n\
               keyframe \"k\" { }\n\
               keyframe \"j\" { show tip\n when tip shown { pulse tip }\n then { hide tip } }";
    let w = lint(src);
    assert!(!w.iter().any(|m| m.contains("hidden the whole keyframe")), "{w:?}");
}

#[test]
fn a_table_is_one_element_with_rows_as_parts() {
    let src = r##"
table orders [columns: ["#", "customer", "status"],
              rows: [["41", "Stroopwafels BV", "shipped"], ["42", "Acme Bikes", "open"]],
              widths: [60, 210, 114], font_size: 18]
keyframe "a" { }
keyframe "b" { highlight orders.row[2] [color: red]
 transform orders.r2c2 [label: "shipped", swap: fade] }
"##;
    let svg = render(src);
    for part in ["orders_bg", "orders_head", "orders_row2", "orders_r1c1"] {
        assert!(svg.contains(&format!(r#"id="{part}""#)), "{part}");
    }
    assert!(svg.contains("Stroopwafels BV"));
    assert_eq!(attr(&svg, "orders_r1c1", "x") - attr(&svg, "orders_r1c0", "x"), 60.0, "cells flush, by width");
    assert!(lint(src).is_empty(), "{:?}", lint(src));
    let s = states(src);
    assert!(s.contains(r#"orders.r2c2 [label: "shipped"]"#), "{s}");
}

#[test]
fn an_explicit_zero_corner_radius_beats_the_stylesheet() {
    let svg = render("rect a [width: 40, height: 20, corner_radius: 0]");
    assert!(svg.contains(r#"style="rx:0px;ry:0px""#), "{svg}");
}

#[test]
fn accent_picks_its_look_from_the_element() {
    let src = "row r [gap: 40] { table t [columns: [\"a\", \"b\"], rows: [[\"1\", \"2\"]], widths: [120, 120]]\n\
               circle dot [size: 24]\nrect panel [width: 200, height: 120] }\n\
               keyframe \"a\" { }\nkeyframe \"b\" { accent t.row[1]\n accent dot\n accent panel [tone: error, hold: step] }";
    let t = timeline(src);
    // A row gets a marker, a dot a ring, a panel an outline (+ "!" badge).
    assert!(t.contains(".aiacm-") && t.contains(".aiacr-") && t.contains(".aiaco-") && t.contains(".aiacb-"), "{t}");
    assert!(t.contains("(loop)"), "hold: step stays while the step is shown: {t}");
    assert!(lint(src).is_empty(), "{:?}", lint(src));
}

#[test]
fn an_accent_on_something_hidden_is_linted() {
    let src = "rect a [width: 40, height: 20, appears: later]\nkeyframe \"k\" { }\nkeyframe \"j\" { accent a }";
    assert!(lint(src).iter().any(|m| m.contains("accent on a")), "{:?}", lint(src));
}

#[test]
fn what_the_first_keyframe_does_later_plays() {
    // v0.2.14 applied every change of frame 0 before it started, so a `set`
    // behind `when` in the first keyframe never animated.
    let src = r#"
template "stage" (what: "build") {
    rect bg [label: what, fill: gray, width: 170, height: 52]
    state passed { transform self [fill: green] }
}
row pipe [gap: 30] { stage a [what: "build"]
 stage b [what: "tests"] }
rect mr [width: 100, height: 30]
keyframe "first" {
    transform mr [fill: blue]
    at 0.2 { show mr [enter: pop] }
    when mr shown { set a passed }
    then { set b passed }
}
keyframe "later" { }
"#;
    let t = timeline(src);
    assert!(t.contains("a (shape) fill: gray -> green  @0.70+0.50"), "{t}");
    assert!(t.contains("b (shape) fill: gray -> green  @1.20+0.50"), "{t}");
    assert!(!t.contains("mr (shape) fill"), "what it does at its start still sets the scene: {t}");
}

#[test]
fn a_held_accent_ends_with_its_step() {
    let src = "rect a [width: 200, height: 100, fill: white]\nkeyframe \"k\" { }\nkeyframe \"j\" { accent a [hold: step] }\nkeyframe \"n\" { }";
    let mut c = RenderConfig::new();
    c.frame = Some("n".into());
    c.at = Some("50%".into());
    let still = render_with_config(src, c).unwrap();
    assert!(still.contains("{ opacity: 0 !important; }") && !still.contains("aiaco-") || still.contains("aiaco-") && still.split("aiaco-").nth(1).unwrap().contains("opacity: 0"), "gone in the next step");
}

#[test]
fn a_bar_gets_a_marker_and_a_box_an_outline() {
    // An outline around a thin bar runs into its neighbours; a marker stays
    // inside. An underline on a table's inner row sat on the next row's border.
    let src = "rect layer [width: 180, height: 44, fill: white, stroke: black]\nkeyframe \"k\" { }\nkeyframe \"j\" { accent layer }";
    let t = timeline(src);
    assert!(t.contains(".aiacm-") && !t.contains(".aiacu-") && !t.contains(".aiaco-"), "{t}");
    let src = "rect card [width: 180, height: 120, fill: white, stroke: black]\nkeyframe \"k\" { }\nkeyframe \"j\" { accent card }";
    assert!(timeline(src).contains(".aiaco-"));
}

#[test]
fn an_artwork_part_turns_about_its_pivot_in_player_and_stills() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let src = "template \"barrier\" from \"assets/barrier.svg\"\nbarrier gate [width: 200, height: 96, arm.pivot: left]\n\
               keyframe \"z\" { }\nkeyframe \"a\" { transform gate.arm [rotation: -80] }";
    let cfg = |f: fn(&mut RenderConfig)| {
        let mut c = RenderConfig::new().with_template_base_path(dir.clone());
        f(&mut c);
        render_with_config(src, c).unwrap()
    };
    let player = cfg(|c| c.animate = true);
    let origin = player.split("gate_arm kf-anim\" style=\"transform-origin:").nth(1).unwrap();
    let (ox, rest) = origin.split_once("px ").unwrap();
    let oy = rest.split("px").next().unwrap();
    let still = cfg(|c| c.frame = Some("a".into()));
    let rot = still.split("id=\"gate_arm\"").nth(1).unwrap();
    let r = rot.split("rotate(").nth(1).unwrap().split(')').next().unwrap();
    let v: Vec<f64> = r.split_whitespace().map(|x| x.parse().unwrap()).collect();
    assert_eq!(v[0], -80.0);
    assert!((v[1] - ox.parse::<f64>().unwrap()).abs() < 0.05 && (v[2] - oy.parse::<f64>().unwrap()).abs() < 0.05,
        "the still turns about the player's origin: rotate({r}) vs origin {ox} {oy}");
    let lint = agent_illustrator::render_with_lint(src, RenderConfig::new().with_template_base_path(dir.clone()).with_lint(true)).unwrap().1;
    assert!(lint.is_empty(), "part overrides are not unknown modifiers: {:?}", lint.iter().map(|w| &w.message).collect::<Vec<_>>());
}

#[test]
fn two_accents_in_different_tones_keep_their_colours() {
    let src = "rect box [width: 200, height: 100, fill: white, stroke: black]\nkeyframe \"a\" { }\n\
               keyframe \"found\" { accent box [tone: error] }\nkeyframe \"retest\" { accent box [tone: ok] }";
    let mut c = RenderConfig::new();
    c.frame = Some("found".into());
    c.at = Some("0.6s".into());
    let still = render_with_config(src, c).unwrap();
    assert!(still.contains("-box-error { opacity: 1") && still.contains("-box-ok { opacity: 0"), "{still}");
}

#[test]
fn a_ghost_copies_what_shows_when_it_flies() {
    // v0.2.18 copied a container with all its children, the ones still to
    // come included.
    // A hidden instance hides its parts too.
    let src = r#"
template "msg" (says: "") { rect bg [width: 120, height: 30, label: says] }
group convo {
    col conv [gap: 8] {
        rect e1 [width: 120, height: 30, label: "first", appears: later]
        msg e2 [says: "second", appears: later]
        msg e3 [says: "later answer", appears: later]
    }
}
rect llm [width: 80, height: 80]
constrain llm.left = convo.right + 100
keyframe "a" { show e1, e2 }
keyframe "b" { fly ghost(convo) to llm
 then { show e3 } }
"#;
    let svg = render(src);
    // The ghost's own group, up to its matching `</g>`.
    let at = svg.find("<g class=\"aighost-").expect("a ghost");
    let (mut depth, mut end) = (0i32, svg.len());
    for (i, _) in svg[at..].match_indices('<') {
        let rest = &svg[at + i..];
        if rest.starts_with("<g") && !rest[..rest.find('>').unwrap()].ends_with('/') {
            depth += 1;
        } else if rest.starts_with("</g>") {
            depth -= 1;
            if depth == 0 {
                end = at + i;
                break;
            }
        }
    }
    let ghost = &svg[at..end];
    assert!(ghost.contains("second"), "{ghost}");
    assert!(!ghost.contains("later answer"), "{ghost}");
}

#[test]
fn a_show_of_something_already_on_screen_is_linted() {
    // A panel missing `appears: later` sat on screen from the first click;
    // its `show` later did nothing, and lint said nothing.
    let src = "rect panel [width: 200, height: 100]\nrect tip [width: 40, height: 20, appears: later]\n\
               keyframe \"a\" { show tip }\nkeyframe \"b\" { show panel [enter: fade] }\n\
               keyframe \"c\" { hide tip\n then { show tip } }";
    let w = lint(src);
    assert!(w.iter().any(|m| m.contains("show panel: it is already on screen")), "{w:?}");
    assert!(!w.iter().any(|m| m.contains("show tip")), "hidden first in the keyframe, or entering: {w:?}");
}

#[test]
fn when_accented_and_when_shown_on_something_already_there() {
    let src = "rect a [width: 40, height: 20]\nrect b [width: 40, height: 20, appears: later]\n\
               keyframe \"k\" { show b }\n\
               keyframe \"j\" { accent b [tone: error]\n when b accented + 0.2 { move b to a }\n when a shown { accent a } }";
    let t = timeline(src);
    assert!(t.contains("b accented"), "{t}");
    assert!(t.contains("a shown"), "already on screen: shown as of the keyframe's start: {t}");
    // Something hidden at the start still needs its show.
    let bad = "rect a [width: 40, height: 20, appears: later]\nkeyframe \"k\" { }\nkeyframe \"j\" { when a shown { accent a } }";
    assert!(render_with_config(bad, RenderConfig::new()).is_err());
}

#[test]
fn a_new_label_takes_the_new_label_colour() {
    // The swapped-in text sits in its own group; the colour reached only
    // the old text, so `[label: "✕", label_fill: role-error]` drew a black ✕.
    let src = "rect c [width: 200, height: 40, label: \"old\", fill: none]\nkeyframe \"a\" { }\n\
               keyframe \"b\" { transform c [label: \"new\", label_fill: red] }";
    let svg = render(src);
    assert!(svg.contains("aitxtv\""), "the variants' group is addressable: {svg}");
    let rule = svg.lines().find(|l| l.contains("> .aitxtv > text") && l.contains("red")).map(str::to_string);
    let player = svg.contains(r#".aitxtv \u003e text","fill"]"#);
    assert!(rule.is_some() || player, "the label colour reaches the new text: {svg}");
}

#[test]
fn a_move_to_something_that_moves_along_is_linted() {
    // The target was placed (through constraints) relative to the mover:
    // the move re-solved it along, and silently went nowhere.
    let src = "rect agent [width: 40, height: 40]\nrect tests [width: 60, height: 40]\npoint at_tests\n\
               rect home_spot [width: 10, height: 10]\n\
               constrain tests.left = agent.right + 200\nconstrain at_tests.center_x = tests.center_x\n\
               constrain at_tests.top = tests.bottom + 20\nconstrain home_spot.top = agent.bottom + 100\n\
               keyframe \"a\" { }\nkeyframe \"b\" { move agent to at_tests }\nkeyframe \"c\" { move agent to home_spot }";
    let w = lint(src);
    assert!(w.iter().any(|m| m.contains("move agent to at_tests ends") && m.contains("moves along with it")), "{w:?}");
    let ok = "rect agent [width: 40, height: 40]\npoint p\nconstrain p.left = 300\nconstrain p.top = 100\n\
              keyframe \"a\" { }\nkeyframe \"b\" { move agent to p }";
    assert!(!lint(ok).iter().any(|m| m.contains("moves along")), "{:?}", lint(ok));
}

#[test]
fn an_accent_mark_that_runs_into_a_neighbour_is_linted() {
    let src = "col ctx [gap: 6] { rect c3 [width: 280, height: 20, fill: gray]\n rect c4 [width: 280, height: 34, fill: gray]\n \
               rect c5 [width: 280, height: 20, fill: gray] }\nkeyframe \"a\" { }\n\
               keyframe \"b\" { accent c4 [style: outline, hold: step] }\nkeyframe \"c\" { accent c4 [tone: error] }";
    let w = lint(src);
    assert!(w.iter().any(|m| m.contains("accent c4 (outline) runs into c3, c5")), "{w:?}");
    assert_eq!(w.iter().filter(|m| m.contains("runs into")).count(), 1, "the marker stays inside: {w:?}");
}

#[test]
fn bracket_parts_work_everywhere() {
    // `c.line[2]` in a constraint, `t.cell[2][2]` (row 2, column 2) anywhere.
    let src = "code c [lang: python, source: \"a = 1\\nb = 2\"]\n\
               table t [columns: [\"n\", \"s\"], rows: [[\"one\", \"\"], [\"two\", \"\"]], widths: [100, 80]]\n\
               rect tag [width: 40, height: 20]\nconstrain t.left = c.right + 40\nconstrain t.top = c.top + 100\n\
               constrain tag.left = c.line[2].right + 10\nconstrain tag.center_y = c.line[2].center_y\n\
               keyframe \"a\" { }\nkeyframe \"b\" { transform t.cell[2][2] [label: \"x\"]\n accent c.lines[1..2] }";
    let st = states(src);
    assert!(st.contains("t.r2c1 [label: \"x\"]"), "{st}");
}

#[test]
fn lists_pair_up_by_position() {
    let src = "row rs [gap: 80] { rect r1 [width: 40, height: 40]  rect r2 [width: 40, height: 40] }\n\
               rect d1 [width: 30, height: 30, appears: later]\nrect d2 [width: 30, height: 30, appears: later]\n\
               point x\npoint y\nconstrain d1.top = 200\nconstrain d2.top = 200\nconstrain d2.left = d1.right + 200\n\
               constrain x.left = 600\nconstrain x.top = 0\nconstrain y.left = 600\nconstrain y.top = 300\n\
               keyframe \"a\" { show d1, d2 [from: r1, r2, stagger: 0.1] }\n\
               keyframe \"b\" { move d1, d2 to y, x }";
    let t = timeline(src);
    assert!(t.contains("d1 -> r1") && t.contains("d2 -> r2"), "{t}");
    assert!(t.contains("move d1 -> y") && t.contains("move d2 -> x"), "{t}");
    assert!(!lint(src).iter().any(|m| m.contains("ends")), "{:?}", lint(src));
    let bad = src.replace("to y, x", "to y, x, y");
    let e = render_with_config(&bad, RenderConfig::new()).unwrap_err().to_string();
    assert!(e.contains("3 destinations for 2"), "{e}");
}

#[test]
fn a_mark_stays_until_unmarked_in_every_output() {
    let src = "table found [columns: [\"finding\", \"\"], rows: [[\"one\", \"✓\"], [\"three\", \"✕\"]], widths: [200, 60]]\n\
               keyframe \"a\" { }\nkeyframe \"b\" { mark found.row[2] [tone: error] }\n\
               keyframe \"c\" { accent found.row[1] }\nkeyframe \"d\" { unmark found.row[2] }";
    let st = states(src);
    assert!(st.contains("found.row2 marked (error)") && st.contains("found.row2 unmarked"), "{st}");
    // Static frames (also --frames-to-dir and --png without --at).
    assert!(!frame(src, "a").contains("aimark"));
    assert!(frame(src, "b").contains("aimark") && frame(src, "c").contains("aimark"));
    assert!(!frame(src, "d").contains("aimark"));
    // The player and the no-JS picture: a settled channel, not a transient.
    let svg = render(src);
    assert!(svg.contains(r#""settled":[["0""#) && svg.contains("aimkm-"), "{svg}");
    assert!(lint(src).is_empty(), "{:?}", lint(src));
}

#[test]
fn a_caption_is_one_modifier_and_comes_with_its_element() {
    let src = "template \"bot\" { circle c [size: 40] }\n\
               row rs [gap: 60] { bot r1 [caption: \"NVIDIA\"]\n bot r2 [caption: \"AMD\", caption_position: above, appears: later] }\n\
               keyframe \"a\" { }\nkeyframe \"b\" { show r2 [enter: pop]\n accent r1.caption }\nkeyframe \"c\" { hide r2 }";
    let st = states(src);
    assert!(st.contains("+ r2, r2.caption") && st.contains("- r2, r2.caption"), "{st}");
    let svg = frame(src, "b");
    assert!(svg.contains(">NVIDIA<") && svg.contains(">AMD<"), "{svg}");
    assert!(!frame(src, "a").contains(">AMD<"), "it follows its element's appears");
    assert!(lint(src).is_empty(), "{:?}", lint(src));
}

#[test]
fn count_makes_identical_elements_and_items_override_some() {
    let src = "col ctx [gap: 6] { rect c* [count: 8, width: 280, height: 20, fill: gray, items: [{}, {}, {}, {fill: orange, height: 34}]] }";
    let svg = render(src);
    assert!(svg.contains(r#"id="c7""#) && !svg.contains(r#"id="c8""#), "c0..c7");
    let c3 = &svg[svg.find(r#"id="c3""#).unwrap()..];
    let c3 = &c3[..c3.find("/>").unwrap()];
    assert!(c3.contains(r#"height="34""#) && c3.contains("orange"), "an item's own value wins: {c3}");
    let bad = "row r { rect c* [count: 3, width: [10, 20], height: 10] }";
    assert!(render_with_config(bad, RenderConfig::new()).is_err());
}

#[test]
fn a_meter_fills_up_by_level() {
    let src = "meter cost [segments: 10, value: 2]\nkeyframe \"a\" { }\nkeyframe \"b\" { set cost 6 }\nkeyframe \"c\" { set cost level9 }";
    let st = states(src);
    assert!(st.contains("cost.s6 [fill: role-primary]") && st.contains("cost.s9 [fill: role-primary]"), "{st}");
    assert!(!st.contains("cost.s7 [fill: role-primary]\n  step 2"), "{st}");
    assert!(lint(src).is_empty(), "{:?}", lint(src));
    let bad = "meter cost [segments: 4, value: 6]";
    assert!(render_with_config(bad, RenderConfig::new()).is_err());
}

#[test]
fn prose_is_a_code_block_in_the_body_font() {
    let src = "code prompt [lang: prose, title: \"prompt.md\", source: \"You are a reviewer.\\nIf a test is missing, send it back.\"]\n\
               keyframe \"a\" { }\nkeyframe \"b\" { accent prompt.line[2] [hold: step] }";
    let svg = render(src);
    let line = &svg[svg.find(r#"id="prompt_line2""#).unwrap()..];
    let label = &line[line.find("<text").unwrap()..line.find("</text>").unwrap()];
    assert!(!label.contains("mono"), "body font: {label}");
    assert!(!label.contains("code-ln"), "no line numbers unless asked: {label}");
    assert!(lint(src).is_empty(), "{:?}", lint(src));
}

#[test]
fn a_status_column_colours_its_marks_and_their_changes() {
    let src = "table j [columns: [\"journey\", \"ok?\"], rows: [[\"log in\", \"✓\"], [\"see date\", \"\"]], widths: [200, 70], status: 2]\n\
               keyframe \"a\" { }\nkeyframe \"b\" { transform j.cell[2][2] [label: \"✕\"] }";
    let st = states(src);
    assert!(st.contains("j.r2c1 [label: \"✕\", label_fill: role-error]"), "{st}");
    assert!(frame(src, "a").contains("role-ok"), "a ✓ cell is green from the start");
    assert!(lint(src).is_empty(), "{:?}", lint(src));
}

#[test]
fn a_scene_part_paints_over_the_next_instance() {
    let tpl = |z: &str| format!(
        "template \"chip\" {{\n rect bg [width: 140, height: 60, fill: white, stroke: black]\n \
         circle badge [size: 30, fill: red, stroke: none, z_order: {z}]\n \
         constrain badge.center_x = bg.right\n constrain badge.center_y = bg.center_y\n}}\n\
         chip k1\nchip k2\nconstrain k1.bg.left = 0\nconstrain k1.bg.top = 0\n\
         constrain k2.bg.left = k1.bg.right\nconstrain k2.bg.top = k1.bg.top\n"
    );
    let scene = render(&tpl("scene(1)"));
    assert!(scene.find(r#"id="k1_badge""#).unwrap() > scene.find(r#"id="k2_bg""#).unwrap(), "lifted over k2");
    let local = tpl("1");
    let svg = render(&local);
    assert!(svg.find(r#"id="k1_badge""#).unwrap() < svg.find(r#"id="k2_bg""#).unwrap(), "local: under k2");
    assert!(lint(&local).iter().any(|m| m.contains("is local to \"k1\"") && m.contains("scene(1)")), "{:?}", lint(&local));
    assert!(!lint(&tpl("scene(1)")).iter().any(|m| m.contains("is local")));
}

#[test]
fn a_layer_change_is_one_switch_at_its_moment() {
    let src = "rect card [width: 120, height: 80, fill: orange, z_order: 1]\nrect server [width: 160, height: 160, fill: blue]\n\
               point spot\nconstrain card.left = 0\nconstrain card.top = 0\nconstrain server.left = 260\nconstrain server.top = 0\n\
               constrain spot.center_x = server.center_x\nconstrain spot.center_y = server.center_y\n\
               keyframe \"a\" { }\nkeyframe \"b\" { move card to spot [z_order: -1, duration: 1] }\n\
               keyframe \"c\" { move card home [z_order: 1, duration: 1] }";
    let t = timeline(src);
    // Unseen: going under just before it reaches the server (here exactly
    // halfway), coming back over just after it has left it.
    assert!(t.contains("card-0 opacity: 0 -> 1  @0.50+0.00"), "{t}");
    assert!(t.contains("card-1 opacity: 0 -> 1  @0.50+0.00"), "{t}");
    // Changing layer while overlapped cannot hide: it pops, and lint says so.
    let pops = src.replace("move card home [z_order: 1, duration: 1]", "transform card [z_order: 1]");
    assert!(lint(&pops).iter().any(|m| m.contains("z_order change on card happens while it overlaps server")), "{:?}", lint(&pops));
    assert!(!lint(&pops.replace("[z_order: 1]", "[z_order: 1, z_at: start]")).iter().any(|m| m.contains("pops")));
    let svg = render(src);
    assert_eq!(svg.matches(r#"id="card""#).count(), 1, "one copy keeps the id");
    assert_eq!(svg.matches("data-zcopy-of=\"card\"").count(), 1, "the other layer's copy");
    // Static frames paint it where its layer is.
    let b = frame(src, "b");
    assert!(b.find(r#"id="card""#).unwrap() < b.find(r#"id="server""#).unwrap());
    let c = frame(src, "c");
    assert!(c.find(r#"id="card""#).unwrap() > c.find(r#"id="server""#).unwrap());
    assert!(!lint(src).iter().any(|m| m.contains("z_order")), "{:?}", lint(src));
}

#[test]
fn a_z_order_that_changes_nothing_is_linted() {
    let src = "rect a [width: 40, height: 40]\nrect b [width: 40, height: 40, z_order: 2]\nconstrain b.left = a.left + 10\nconstrain b.top = a.top + 10";
    assert!(lint(src).iter().any(|m| m.contains("z_order on \"b\" changes nothing")), "{:?}", lint(src));
    let needed = src.replace("rect a [width: 40, height: 40]\nrect b [width: 40, height: 40, z_order: 2]", "rect b [width: 40, height: 40, z_order: 2]\nrect a [width: 40, height: 40]");
    assert!(!lint(&needed).iter().any(|m| m.contains("changes nothing")), "{:?}", lint(&needed));
}

#[test]
fn a_strong_caption_and_a_caption_too_close_to_a_neighbour() {
    let src = "circle r1 [size: 50, caption: \"driver\", caption_style: strong]\nrect chip [width: 120, height: 30]\n\
               constrain r1.center_x = 100\nconstrain r1.top = 0\nconstrain chip.center_x = 100\nconstrain chip.top = r1.bottom + 45";
    let svg = render(src);
    let cap = &svg[svg.find(">driver<").unwrap() - 300..svg.find(">driver<").unwrap()];
    assert!(cap.contains("font-size=\"20\"") && cap.contains("role-ink") && cap.contains("700"), "{cap}");
    assert!(lint(src).iter().any(|m| m.contains("caption \"r1.caption\"") && m.contains("from \"chip\"")), "{:?}", lint(src));
    let roomy = src.replace("r1.bottom + 45", "r1.bottom + 60");
    assert!(!lint(&roomy).iter().any(|m| m.contains("caption \"")), "{:?}", lint(&roomy));
}

#[test]
fn being_covered_after_a_layer_change_is_not_an_overlap() {
    let src = "rect server [width: 260, height: 220, fill: blue, label: \"server\"]\nrect card [width: 160, height: 90, fill: orange, label: \"card\", z_order: 1]\n\
               point spot\nconstrain server.left = 300\nconstrain server.top = 0\nconstrain card.left = 0\nconstrain card.top = 60\n\
               constrain spot.center_x = server.center_x\nconstrain spot.center_y = server.center_y\n\
               keyframe \"a\" { }\nkeyframe \"b\" { move card to spot [z_order: -1] }";
    let w = lint(src);
    assert!(!w.iter().any(|m| m.contains("overlap")), "{w:?}");
    // Without the layer change, the same end state is an overlap.
    let plain = src.replace("move card to spot [z_order: -1]", "move card to spot");
    assert!(lint(&plain).iter().any(|m| m.contains("overlap")), "{:?}", lint(&plain));
}
