//! `grid [gap_x: N, gap_y: M]`: the gap between columns and between rows
//! apart; `gap` sets both. And `at:` on a template instance in a grid.

use agent_illustrator::{render, render_with_lint, RenderConfig};

fn svg(src: &str) -> String {
    render(src).unwrap_or_else(|e| panic!("{e}\n---\n{src}"))
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

const CELLS: &str = r#"
    rect a [width: 50, height: 20]
    rect b [width: 50, height: 20]
    rect c [width: 50, height: 20]
    rect d [width: 50, height: 20]
"#;

#[test]
fn gap_x_spaces_columns_and_gap_y_spaces_rows() {
    let out = svg(&format!("grid g [cols: 2, gap_x: 40, gap_y: 6] {{ {CELLS} }}"));
    let (ax, ay, aw, ah) = rect_of(&out, "a");
    let (bx, by, _, _) = rect_of(&out, "b");
    let (cx, cy, _, _) = rect_of(&out, "c");
    assert_eq!(bx - (ax + aw), 40.0, "columns 40 apart");
    assert_eq!(by, ay, "a and b share a row");
    assert_eq!(cy - (ay + ah), 6.0, "rows 6 apart");
    assert_eq!(cx, ax, "a and c share a column");
}

#[test]
fn gap_sets_both_and_one_axis_overrides_it() {
    let both = svg(&format!("grid g [cols: 2, gap: 12] {{ {CELLS} }}"));
    let (ax, ay, aw, ah) = rect_of(&both, "a");
    assert_eq!(rect_of(&both, "b").0 - (ax + aw), 12.0);
    assert_eq!(rect_of(&both, "c").1 - (ay + ah), 12.0);

    let one = svg(&format!("grid g [cols: 2, gap: 12, gap_y: 30] {{ {CELLS} }}"));
    let (ax, ay, aw, ah) = rect_of(&one, "a");
    assert_eq!(rect_of(&one, "b").0 - (ax + aw), 12.0, "gap_x falls back to gap");
    assert_eq!(rect_of(&one, "c").1 - (ay + ah), 30.0);
}

#[test]
fn gap_x_and_gap_y_are_known_modifiers() {
    let (_, warnings) =
        render_with_lint(&format!("grid g [cols: 2, gap_x: 40, gap_y: 6] {{ {CELLS} }}"), RenderConfig::new().with_lint(true))
            .expect("render");
    let unknown: Vec<_> = warnings.iter().filter(|w| w.message.contains("gap_")).collect();
    assert!(unknown.is_empty(), "{unknown:?}");
}

/// Regression: `at:` on a template instance was dropped (the instance went
/// to the next free cell), for a one-part template and for a multi-part one.
#[test]
fn at_places_a_template_instance_in_its_cell() {
    let src = r#"
template "one" (t: "") { rect x [width: 40, height: 20, label: t] }
template "card" (t: "") {
  rect body [width: 60, height: 30, label: t]
  rect tag [width: 20, height: 10]
  constrain tag.left = body.right + 4
  constrain tag.top = body.top
}
grid g [cols: 2, gap: 20] {
  one o1 [t: "a"]
  rect big [width: 140, height: 80]
  one o3 [t: "c", at: [1, 1]]
  card c4 [t: "d", at: [2, 1]]
}
"#;
    let out = svg(src);
    let (big_x, _, big_w, _) = rect_of(&out, "big");
    let (o1x, _, _, _) = rect_of(&out, "o1");
    let (o3x, o3y, _, _) = rect_of(&out, "o3");
    let (c4x, c4y, _, _) = rect_of(&out, "c4_body");
    assert!(o3x >= big_x && o3x < big_x + big_w, "o3 in column 1: x {o3x}, column {big_x}..{}", big_x + big_w);
    assert!(o1x < big_x, "o1 stays in column 0");
    assert!(c4x >= big_x && c4x < big_x + big_w, "c4 in column 1: x {c4x}");
    assert!(c4y > o3y, "c4 in row 2, below o3 in row 1");
}
