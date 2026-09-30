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
