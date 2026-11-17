//! Numbers in `constrain` written as names: a template's number parameters
//! (`+ gap`, `= w`, `>= min_w`, `[padding_x: pad]`) and an `each` item's
//! numbers (`+ m.dy`, `= m.w`, `+ i`).

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

const PAIR: &str = r#"
template "pair" (gap: 10, pad: 6, w: 40) {
  rect a [height: 20]
  rect b [width: 30, height: 20]
  constrain a.width = w
  constrain b.left = a.right + gap
  constrain b.top = a.top
  rect bg [fill: none]
  constrain bg contains a, b [padding_x: pad, padding_y: 2]
}
"#;

#[test]
fn template_parameters_feed_offsets_values_and_padding() {
    let out = svg(&format!("{PAIR}\npair p [gap: 30]\npair q [gap: 5, pad: 20, w: 80]\nconstrain q.top = p.bottom + 20\n"));
    for (inst, gap, pad, w) in [("p", 30.0, 6.0, 40.0), ("q", 5.0, 20.0, 80.0)] {
        let a = rect_of(&out, &format!("{inst}_a"));
        let b = rect_of(&out, &format!("{inst}_b"));
        let bg = rect_of(&out, &format!("{inst}_bg"));
        assert_eq!(a.2, w, "{inst}: width = w");
        assert_eq!(b.0 - (a.0 + a.2), gap, "{inst}: + gap");
        assert_eq!(a.0 - bg.0, pad, "{inst}: padding_x: pad");
        assert_eq!(a.1 - bg.1, 2.0, "{inst}: padding_y stays a number");
    }
}

#[test]
fn defaults_are_numbers_too_and_minus_subtracts() {
    let src = r#"
template "t" (gap: 12, lo: 50) {
  rect a [width: 20, height: 20]
  rect b [width: 20.., height: 20]
  constrain b.right = a.left - gap
  constrain b.width >= lo
  constrain b.top = a.top
}
t x
"#;
    let out = svg(src);
    let a = rect_of(&out, "x_a");
    let b = rect_of(&out, "x_b");
    assert_eq!(a.0 - (b.0 + b.2), 12.0, "- gap");
    assert!(b.2 >= 50.0, ">= lo: {}", b.2);
}

#[test]
fn each_fields_and_the_index_feed_constraints() {
    let src = r#"
rect top_bar [width: 200, height: 10]
let bars = [{w: 50, dy: 10}, {w: 90, dy: 30}]
each r, i in bars {
  rect bar [height: 8]
  constrain bar.width = r.w
  constrain bar.left = top_bar.left + i
  constrain bar.top = top_bar.bottom + r.dy
}
each w in [15, 25] {
  rect sq [height: 5]
  constrain sq.width = w
  constrain sq.top = top_bar.bottom + 60
  constrain sq.left = top_bar.left + w
}
"#;
    let out = svg(src);
    let t = rect_of(&out, "top_bar");
    let b1 = rect_of(&out, "bar_1");
    let b2 = rect_of(&out, "bar_2");
    assert_eq!((b1.2, b2.2), (50.0, 90.0), "= r.w");
    assert_eq!((b1.0 - t.0, b2.0 - t.0), (0.0, 1.0), "+ i");
    assert_eq!((b1.1 - (t.1 + t.3), b2.1 - (t.1 + t.3)), (10.0, 30.0), "+ r.dy");
    assert_eq!(rect_of(&out, "sq_2").2, 25.0, "= w over plain numbers");
    assert_eq!(rect_of(&out, "sq_2").0 - t.0, 25.0);
}

#[test]
fn a_name_given_a_side_makes_a_reference() {
    // `= after + 4` with `after: box.bottom`: 4 below box's bottom.
    let src = r#"
template "rule" (after: "") {
  rect r [width: 20, height: 2]
  constrain r.center_y = after + 4
}
rect box [width: 100, height: 40]
rule x [after: box.bottom]
"#;
    let out = svg(src);
    let b = rect_of(&out, "box");
    let r = rect_of(&out, "x_r");
    assert_eq!(r.1 + r.3 / 2.0, b.1 + b.3 + 4.0);
}

#[test]
fn the_sequence_frame_takes_its_padding_as_a_parameter() {
    let ex = std::fs::read_to_string("examples/sequence-diagram.ail").unwrap();
    assert!(ex.contains("[padding_x: pad, padding_y: 0]"), "the frame template reads `pad`");
    let out = svg(&ex);
    let f = rect_of(&out, "loop_f_box");
    let left = ex.contains("pad: 110");
    assert!(left && f.2 > 220.0, "loop frame is padded 110 on each side: width {}", f.2);
}

#[test]
fn a_name_nothing_fills_is_an_error() {
    let err = render_err("rect a\nrect b\nconstrain b.left = a.right + gap\n");
    assert!(err.contains("nothing gives `gap` a value") && err.contains("template parameter"), "{err}");

    let err = render_err("rect a\nrect b\nconstrain b.left = a\n");
    assert!(err.contains("If `a` is an element, say which side: `= a.left`"), "{err}");
}

#[test]
fn a_misspelt_parameter_suggests_the_right_one() {
    let src = "template \"t\" (gap: 10) { rect a\nrect b\nconstrain b.left = a.right + gpa }\nt x\n";
    let err = render_err(src);
    assert!(err.contains("`gpa` in a constraint on `b.left`") && err.contains("did you mean `gap`?"), "{err}");
}

#[test]
fn a_parameter_that_is_not_a_number_is_an_error() {
    let src = "template \"t\" (gap: \"\") { rect a\nrect b\nconstrain b.left = a.right + gap }\nt x\n";
    let err = render_err(src);
    assert!(err.contains("`gap` is the string \"\", where a number goes"), "{err}");

    let src = r#"
template "rule" (after: "") {
  rect r [width: 20, height: 2]
  constrain r.center_y = after + 4
}
rect box
rule x [after: box]
"#;
    let err = render_err(src);
    assert!(err.contains("`after` is `box`, an element; say which side"), "{err}");
}

#[test]
fn a_misspelt_field_is_named() {
    let err = render_err("let bars = [{w: 50}]\neach r in bars { rect bar\nconstrain bar.width = r.wd }\n");
    assert!(err.contains("no item has a field `wd`") && err.contains("did you mean `w`?"), "{err}");
}
