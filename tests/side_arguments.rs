//! A side of an element as a value: `src: box.top` passed to a template (or
//! held by a record field) means that point wherever the parameter is used:
//! a connection end, a constraint (`src.center_x`), a `midpoint` operand.

use agent_illustrator::render;

fn svg(src: &str) -> String {
    render(src).unwrap_or_else(|e| panic!("{e}\n---\n{src}"))
}

fn render_err(src: &str) -> String {
    match render(src) {
        Ok(_) => panic!("expected an error\n---\n{src}"),
        Err(e) => e.to_string(),
    }
}

/// x, y, width, height of `<rect id="ID" ...>`.
fn rect_of(svg: &str, id: &str) -> (f64, f64, f64, f64) {
    let start = svg.find(&format!("<rect id=\"{id}\"")).unwrap_or_else(|| panic!("no rect {id}"));
    let tag = &svg[start..start + svg[start..].find('>').unwrap()];
    let attr = |k: &str| -> f64 {
        let at = tag.find(&format!(" {k}=\"")).unwrap_or_else(|| panic!("{k} in {tag}")) + k.len() + 3;
        tag[at..at + tag[at..].find('"').unwrap()].parse().unwrap()
    };
    (attr("x"), attr("y"), attr("width"), attr("height"))
}

fn center(r: (f64, f64, f64, f64)) -> (f64, f64) {
    (r.0 + r.2 / 2.0, r.1 + r.3 / 2.0)
}

const STAGE: &str = r#"
template "tag" (at: "") {
  rect dot [width: 6, height: 6]
  constrain dot.center_x = at.center_x
  constrain dot.center_y = at.center_y
}
row r [gap: 80] {
  rect evaluate [width: 100, height: 40]
  rect other [width: 100, height: 40]
}
"#;

#[test]
fn a_side_argument_is_a_point_in_constraints() {
    let out = svg(&format!("{STAGE}\ntag t1 [at: evaluate.top]\ntag t2 [at: evaluate.right]\ntag t3 [at: other.bottom]\n"));
    let ev = rect_of(&out, "evaluate");
    let ot = rect_of(&out, "other");
    assert_eq!(center(rect_of(&out, "t1_dot")), (ev.0 + ev.2 / 2.0, ev.1), "top: centre x, top y");
    assert_eq!(center(rect_of(&out, "t2_dot")), (ev.0 + ev.2, ev.1 + ev.3 / 2.0), "right: right x, centre y");
    assert_eq!(center(rect_of(&out, "t3_dot")), (ot.0 + ot.2 / 2.0, ot.1 + ot.3), "bottom");
}

#[test]
fn the_element_itself_still_works() {
    let out = svg(&format!("{STAGE}\ntag t1 [at: evaluate]\n"));
    assert_eq!(center(rect_of(&out, "t1_dot")), center(rect_of(&out, "evaluate")));
}

#[test]
fn a_side_argument_is_a_connection_end() {
    let src = r#"
template "note" (src: "") {
  rect n [width: 30, height: 20]
  constrain n.left = src.right + 40
  constrain n.top = src.bottom + 20
  src -> n
}
rect box [width: 100, height: 40]
note a [src: box.bottom]
"#;
    let out = svg(src);
    let b = rect_of(&out, "box");
    // The line starts at box's bottom middle.
    let d = out.split("class=\"ai-connection\" d=\"").nth(1).expect("a connection").split('"').next().unwrap();
    let start = format!("M{} {}", b.0 + b.2 / 2.0, b.1 + b.3);
    assert!(d.starts_with(&start), "{d} should start at {start}");
    // `src.right` of a bottom side is its x (the middle).
    let n = rect_of(&out, "a_n");
    assert_eq!(n.0, b.0 + b.2 / 2.0 + 40.0);
    assert_eq!(n.1, b.1 + b.3 + 20.0);
}

#[test]
fn a_side_argument_is_a_midpoint_operand() {
    let src = format!(
        "{STAGE}\n{}\nmid m1 [a: evaluate.right, b: other.left]\n",
        r#"template "mid" (a: "", b: "") {
  rect m [width: 6, height: 6]
  constrain m.center_x = midpoint(a, b)
  constrain m.center_y = a.center_y
}"#
    );
    let out = svg(&src);
    let ev = rect_of(&out, "evaluate");
    let ot = rect_of(&out, "other");
    assert_eq!(center(rect_of(&out, "m1_m")).0, (ev.0 + ev.2 + ot.0) / 2.0);
}

#[test]
fn a_side_and_a_property_written_out() {
    // `box.top.center_x`: the x of box's top, without any parameter.
    let src = r#"
rect box [width: 100, height: 40]
rect d [width: 4, height: 4]
constrain d.center_x = box.top.center_x
constrain d.center_y = box.right.center_y
"#;
    let out = svg(src);
    assert_eq!(center(rect_of(&out, "d")), center(rect_of(&out, "box")));
}

#[test]
fn a_record_field_may_hold_a_side() {
    let src = r#"
row r [gap: 80] {
  rect evaluate [width: 100, height: 40]
  rect other [width: 100, height: 40]
}
let marks = [{at: evaluate.top}, {at: other.left}]
each m in marks {
  rect dot [width: 6, height: 6]
  constrain dot.center_x = m.at.center_x
  constrain dot.center_y = m.at.center_y
}
"#;
    let out = svg(src);
    let ev = rect_of(&out, "evaluate");
    let ot = rect_of(&out, "other");
    assert_eq!(center(rect_of(&out, "dot_1")), (ev.0 + ev.2 / 2.0, ev.1));
    assert_eq!(center(rect_of(&out, "dot_2")), (ot.0, ot.1 + ot.3 / 2.0));
}

#[test]
fn a_side_has_no_width() {
    let src = r#"
template "bar" (at: "") {
  rect b [height: 4]
  constrain b.width = at.width
}
rect box [width: 100, height: 40]
bar x [at: box.top]
"#;
    let err = render_err(src);
    assert!(err.contains("`box.top`") && err.contains("no width"), "{err}");
}

#[test]
fn a_side_cannot_be_contained() {
    let src = r#"
template "frame" (first: "") {
  rect f [fill: none]
  constrain f contains first [padding: 4]
}
rect box [width: 100, height: 40]
frame x [first: box.top]
"#;
    let err = render_err(src);
    assert!(err.contains("contains first") && err.contains("pass the element (`box`)"), "{err}");
}
