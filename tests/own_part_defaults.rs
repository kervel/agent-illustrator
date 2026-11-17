//! A template parameter whose default names one of the template's own parts
//! (`until: rows`) means that part of each instance (`cl_rows`), so a caller
//! can leave it out. A value the caller passes is a name from outside, as
//! for every element-reference parameter.

use agent_illustrator::{render, render_with_lint, RenderConfig};

fn svg(src: &str) -> String {
    render(src).unwrap_or_else(|e| panic!("{e}\n---\n{src}"))
}

/// The y where the connection whose class ends in `-{name}` stops.
fn end_y(svg: &str, name: &str) -> f64 {
    let at = svg.find(&format!("-{name}\" d=\"")).unwrap_or_else(|| panic!("no connection {name}"));
    let rest = &svg[at + name.len() + 6..];
    let d = &rest[..rest.find('"').unwrap()];
    d.rsplit(' ').next().unwrap().parse().unwrap()
}

/// The y of the bottom edge of the rect with this id.
fn bottom(svg: &str, id: &str) -> f64 {
    let start = svg.find(&format!("id=\"{id}\"")).unwrap_or_else(|| panic!("no {id}"));
    let tag = &svg[start..start + svg[start..].find('>').unwrap()];
    let attr = |name: &str| -> f64 {
        let at = tag.find(&format!(" {name}=\"")).unwrap_or_else(|| panic!("no {name} on {id}: {tag}"));
        let rest = &tag[at + name.len() + 3..];
        rest[..rest.find('"').unwrap()].parse().unwrap()
    };
    attr("y") + attr("height")
}

const LIFE: &str = r#"
template "life" (ticks: [], until: rows) {
  col l [gap: 20] {
    rect hd [width: 60, height: 30]
    col rows [gap: 0] {
      each t in ticks { rect p [width: 2, height: 40, stroke: none, fill: none] }
    }
  }
  hd.bottom -- until.bottom as lifeline [routing: direct]
}
"#;

#[test]
fn a_default_naming_an_own_part_means_this_instances_part() {
    let out = svg(&format!(
        "{LIFE}
row r [gap: 40, align: top] {{
  life a [ticks: [1, 2, 3]]
  life b [ticks: [1]]
}}"
    ));
    let (a, b) = (end_y(&out, "a_lifeline"), end_y(&out, "b_lifeline"));
    assert!((a - b - 80.0).abs() < 0.5, "a ends at its own 3 rows, b at its 1 row: {a} vs {b}");
}

#[test]
fn a_value_the_caller_passes_still_wins() {
    let out = svg(&format!(
        "{LIFE}
row r [gap: 40, align: top] {{
  life a [ticks: [1, 2, 3]]
  life b [ticks: [1, 2, 3], until: b.p_1]
}}"
    ));
    let (a, b) = (end_y(&out, "a_lifeline"), end_y(&out, "b_lifeline"));
    assert!(a > b + 80.0, "b ends at its first row, a at all three: {a} vs {b}");
    assert!((b - bottom(&out, "b_p_1")).abs() < 0.5, "b ends at the bottom of b.p_1: {b}");
}

#[test]
fn a_nested_instance_reaches_its_own_part() {
    let out = svg(&format!(
        "{LIFE}
template \"pair\" () {{
  row r [gap: 40, align: top] {{
    life x [ticks: [1, 2]]
    life y [ticks: [1]]
  }}
}}
pair s"
    ));
    let (x, y) = (end_y(&out, "s_x_lifeline"), end_y(&out, "s_y_lifeline"));
    assert!((x - y - 40.0).abs() < 0.5, "{x} vs {y}");
}

#[test]
fn a_default_that_names_no_part_is_left_alone() {
    // `tint: role-primary` is a colour, not a part of the template.
    let src = r#"
template "chip" (tint: role-primary) {
  rect body [size: 20, fill: tint]
}
chip c
"#;
    let (out, warnings) = render_with_lint(src, RenderConfig::new().with_lint(true)).expect("renders");
    assert!(out.contains("role-primary"), "{out}");
    assert!(warnings.is_empty(), "{:?}", warnings.iter().map(|w| &w.message).collect::<Vec<_>>());
}
