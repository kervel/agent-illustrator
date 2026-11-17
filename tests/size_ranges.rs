//! `width: 200..`, `width: ..300`, `width: 120..400`: a size constraints may
//! change, within bounds. A plain number stays a fixed size, so a template
//! part keeps its shape unless its author gave it a range.

use agent_illustrator::{render, render_with_lint, RenderConfig};

fn svg(src: &str) -> String {
    render(src).unwrap_or_else(|e| panic!("{e}\n---\n{src}"))
}

fn lint(src: &str) -> Vec<String> {
    let (_svg, w) = render_with_lint(src, RenderConfig::new().with_lint(true)).expect("renders");
    w.iter().map(|x| x.message.clone()).collect()
}

/// `(x, width)` of the rect with this id.
fn rect_x_w(svg: &str, id: &str) -> (f64, f64) {
    let start = svg.find(&format!("id=\"{id}\"")).unwrap_or_else(|| panic!("no {id}"));
    let tag = &svg[start..start + svg[start..].find('>').unwrap()];
    let attr = |name: &str| -> f64 {
        let at = tag.find(&format!(" {name}=\"")).unwrap_or_else(|| panic!("no {name} on {id}: {tag}"));
        let rest = &tag[at + name.len() + 3..];
        rest[..rest.find('"').unwrap()].parse().unwrap()
    };
    (attr("x"), attr("width"))
}

const LANE: &str = r#"
template "lane" (name: "", across: "") {
    rect hd [width: 100, height: 60, label: name]
    rect body [height: 60, width: 200..]
    constrain body.left = hd.right
    constrain body.top = hd.top
    constrain body.right = across.right + 50
}
row cols [gap: 140] {
    rect c1 [size: 10]
    rect c2 [size: 10]
    rect c3 [size: 10]
    rect c4 [size: 10]
}
"#;

#[test]
fn a_ranged_part_stretches_to_an_outside_edge() {
    let out = svg(&format!(
        "{LANE}
lane a [name: \"A\", across: cols]
constrain cols.left = a.hd.right + 30
constrain cols.top = a.hd.bottom + 100
"
    ));
    let (hx, hw) = rect_x_w(&out, "a_hd");
    let (bx, bw) = rect_x_w(&out, "a_body");
    let (c4x, c4w) = rect_x_w(&out, "c4");
    assert!((bx - (hx + hw)).abs() < 0.5, "body stays against its header: hd ends {} body starts {bx}", hx + hw);
    // the row has a little padding past its last child
    assert!(bx + bw > c4x + c4w + 40.0, "body reaches past the columns: ends {} c4 ends {}", bx + bw, c4x + c4w);
    assert!(bw > 200.0, "{bw}");
}

#[test]
fn a_fixed_part_still_moves_its_whole_instance() {
    // Without a range nothing changes: the part keeps its width and the
    // instance slides as one piece.
    let out = svg(r#"
template "L" () {
    rect a [width: 50, height: 50]
    rect b [width: 80, height: 50]
    constrain b.left = a.right
    constrain b.top = a.top
}
L x
rect c [size: 10]
constrain c.left = 400
constrain x.b.right = c.right
"#);
    let (ax, _) = rect_x_w(&out, "x_a");
    let (bx, bw) = rect_x_w(&out, "x_b");
    assert_eq!(bw, 80.0);
    assert!((bx - (ax + 50.0)).abs() < 0.5);
    assert!((bx + bw - 410.0).abs() < 0.5, "{}", bx + bw);
}

#[test]
fn the_natural_size_is_kept_within_the_range() {
    let wide = svg("rect a [width: 200.., height: 40]\n");
    assert_eq!(rect_x_w(&wide, "a").1, 200.0);
    let narrow = svg("rect a [width: ..50, height: 40]\n");
    assert_eq!(rect_x_w(&narrow, "a").1, 50.0);
}

#[test]
fn an_upper_bound_stops_a_stretch_and_lint_says_so() {
    let src = r#"
rect a [width: 60, height: 40]
rect b [width: 100..200, height: 40]
rect c [size: 10]
constrain b.left = a.right
constrain b.top = a.top
constrain c.left = a.right + 500
constrain b.right = c.right
"#;
    let out = svg(src);
    assert_eq!(rect_x_w(&out, "b").1, 200.0);
    let w = lint(src);
    assert!(w.iter().any(|m| m.contains("`b` stopped at the top of its width range (100..200)")), "{w:?}");
}

#[test]
fn a_long_label_wraps_at_the_upper_bound() {
    let out = svg("rect a [width: ..120, label: \"a fairly long label that does not fit on one line\"]\n");
    assert!(rect_x_w(&out, "a").1 <= 120.0);
    assert!(out.matches("<tspan").count() >= 2, "label should wrap:\n{out}");
}

#[test]
fn a_range_needs_a_bound() {
    let e = render("rect a [width: ..]\n").unwrap_err().to_string();
    assert!(e.contains("at least one bound"), "{e}");
}

#[test]
fn a_range_on_anything_but_width_or_height_is_an_error() {
    let e = render("rect a [font_size: 10..20]\n").unwrap_err().to_string();
    assert!(e.contains("width") && e.contains("height"), "{e}");
}

#[test]
fn a_range_on_size_is_an_error_for_now() {
    let e = render("circle a [size: 10..20]\n").unwrap_err().to_string();
    assert!(e.contains("size"), "{e}");
}

#[test]
fn a_reversed_range_is_an_error() {
    let e = render("rect a [width: 300..100]\n").unwrap_err().to_string();
    assert!(e.contains("300") && e.contains("100"), "{e}");
}

#[test]
fn lint_points_a_rigid_template_part_to_a_range() {
    // The part can only move its instance, and the instance also decides
    // where `c` is: no move satisfies both.
    let w = lint(r#"
template "L" () {
    rect a [width: 50, height: 50]
    rect b [height: 50]
    constrain b.left = a.right
    constrain b.top = a.top
}
L x
rect c [size: 10]
constrain c.left = x.a.right + 300
constrain x.b.right = c.right
"#);
    assert!(w.iter().any(|m| m.contains("part of a template") && m.contains("width: 200..")), "{w:?}");
}

#[test]
fn a_far_edge_out_of_reach_moves_a_ranged_shape_as_it_would_a_fixed_one() {
    // Grows to its upper bound, then moves: no unsatisfiable error.
    let out = svg("rect a [width: 100..200, height: 20]\nconstrain a.right = 300\n");
    assert_eq!(rect_x_w(&out, "a"), (100.0, 200.0));
    let out = svg("rect a [width: 100.., height: 20]\nconstrain a.right = 50\n");
    assert_eq!(rect_x_w(&out, "a"), (-50.0, 100.0));
    // Within reach it grows and stays put.
    let out = svg("rect a [width: 100..200, height: 20]\nconstrain a.right = 150\n");
    assert_eq!(rect_x_w(&out, "a"), (0.0, 150.0));
}

#[test]
fn a_range_on_the_other_axis_is_not_blamed() {
    let w = lint(
        "rect b [size: 10]\nrect r [width: 100.., height: 30]\nconstrain b.top = 100\nconstrain r.top = 10\nconstrain r.top = b.bottom + 50\n",
    );
    assert!(w.iter().any(|m| m.contains("violated")), "{w:?}");
    assert!(!w.iter().any(|m| m.contains("range")), "{w:?}");
}

#[test]
fn a_position_conflict_on_a_template_part_does_not_suggest_a_range() {
    let w = lint(
        "template \"Q\" () { rect q [size: 20] }\nQ x\nrect z [size: 10]\nconstrain z.top = 200\nconstrain x.q.top = 10\nconstrain x.q.top = z.bottom + 100\n",
    );
    assert!(w.iter().any(|m| m.contains("violated")), "{w:?}");
    assert!(!w.iter().any(|m| m.contains("size range")), "{w:?}");
}

#[test]
fn a_row_member_stretched_over_its_neighbour_is_reported() {
    let w = lint(r#"
row r {
    rect a [width: 100.., height: 30]
    rect b [width: 50, height: 30]
}
rect c [width: 400, height: 20]
constrain c.top = r.bottom + 20
constrain a.right = c.right
"#);
    assert!(w.iter().any(|m| m.contains("\"a\" and \"b\" overlap")), "{w:?}");
}

#[test]
fn a_negative_bound_is_an_error_that_says_so() {
    let e = render("rect a [width: -10..20]\n").unwrap_err().to_string();
    assert!(e.contains("never negative"), "{e}");
}
