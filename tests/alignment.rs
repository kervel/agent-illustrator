//! `align:` controls where text sits inside its box and how a row's or
//! column's children line up on the cross axis.

use agent_illustrator::{render_with_config, RenderConfig};

fn render(source: &str) -> String {
    render_with_config(source, RenderConfig::new()).expect("should render")
}

/// Pull the `x`/`y`/`text-anchor` of the element with this id out of the SVG.
fn attrs_of(svg: &str, id: &str) -> String {
    svg.lines()
        .find(|line| line.contains(&format!("id=\"{}\"", id)))
        .unwrap_or_else(|| panic!("no element with id {} in:\n{}", id, svg))
        .to_string()
}

fn number_attr(line: &str, name: &str) -> f64 {
    let needle = format!("{}=\"", name);
    let start = line
        .find(&needle)
        .unwrap_or_else(|| panic!("no {} in {}", name, line))
        + needle.len();
    let rest = &line[start..];
    let end = rest.find('"').expect("unterminated attribute");
    rest[..end].parse().expect("numeric attribute")
}

#[test]
fn text_without_align_still_starts_at_the_left_edge() {
    let svg = render(r#"text "hello" t [width: 200]"#);
    let line = attrs_of(&svg, "t");
    assert!(line.contains(r#"text-anchor="start""#), "got: {}", line);
}

#[test]
fn align_places_text_inside_its_own_box() {
    let source = r#"
col c [gap: 10] {
  text "hello" t_left [width: 200, align: start]
  text "hello" t_mid [width: 200, align: center]
  text "hello" t_right [width: 200, align: end]
}
"#;
    let svg = render(source);

    let left = attrs_of(&svg, "t_left");
    let mid = attrs_of(&svg, "t_mid");
    let right = attrs_of(&svg, "t_right");

    assert!(left.contains(r#"text-anchor="start""#), "got: {}", left);
    assert!(mid.contains(r#"text-anchor="middle""#), "got: {}", mid);
    assert!(right.contains(r#"text-anchor="end""#), "got: {}", right);

    // Same 200px box, three anchor points: left edge, center, right edge.
    let x_left = number_attr(&left, "x");
    let x_mid = number_attr(&mid, "x");
    let x_right = number_attr(&right, "x");
    assert!(
        (x_mid - (x_left + 100.0)).abs() < 0.01,
        "center should be 100px into a 200px box: {} vs {}",
        x_mid,
        x_left
    );
    assert!(
        (x_right - (x_left + 200.0)).abs() < 0.01,
        "end should be at the right edge: {} vs {}",
        x_right,
        x_left
    );
}

#[test]
fn captions_of_different_lengths_share_a_left_edge() {
    // The reason align exists: without a fixed box, a centred caption
    // shifts horizontally every time its text changes length.
    let source = r#"
col c [gap: 10, align: center] {
  rect stage [width: 400, height: 60]
  text "short" cap1 [width: 400, align: start]
  text "a considerably longer caption" cap2 [width: 400, align: start]
}
"#;
    let svg = render(source);
    let x1 = number_attr(&attrs_of(&svg, "cap1"), "x");
    let x2 = number_attr(&attrs_of(&svg, "cap2"), "x");
    assert!(
        (x1 - x2).abs() < 0.01,
        "captions should start at the same x, got {} and {}",
        x1,
        x2
    );
}

#[test]
fn a_row_can_align_its_children_on_the_cross_axis() {
    let tall_and_short = |align: &str| {
        let source = format!(
            r#"
row r [gap: 10, align: {}] {{
  rect tall [width: 40, height: 100]
  rect short [width: 40, height: 30]
}}
"#,
            align
        );
        let svg = render(&source);
        let tall = attrs_of(&svg, "tall");
        let short = attrs_of(&svg, "short");
        (
            number_attr(&tall, "y"),
            number_attr(&short, "y"),
        )
    };

    let (tall_y, short_y) = tall_and_short("start");
    assert!((short_y - tall_y).abs() < 0.01, "start: tops should match");

    let (tall_y, short_y) = tall_and_short("center");
    assert!(
        ((short_y + 15.0) - (tall_y + 50.0)).abs() < 0.01,
        "center: centers should match, got {} and {}",
        short_y,
        tall_y
    );

    let (tall_y, short_y) = tall_and_short("end");
    assert!(
        ((short_y + 30.0) - (tall_y + 100.0)).abs() < 0.01,
        "end: bottoms should match, got {} and {}",
        short_y,
        tall_y
    );
}

#[test]
fn a_column_can_align_its_children_on_the_cross_axis() {
    let source = r#"
col c [gap: 10, align: end] {
  rect wide [width: 200, height: 30]
  rect narrow [width: 50, height: 30]
}
"#;
    let svg = render(source);
    let wide = attrs_of(&svg, "wide");
    let narrow = attrs_of(&svg, "narrow");
    let wide_right = number_attr(&wide, "x") + 200.0;
    let narrow_right = number_attr(&narrow, "x") + 50.0;
    assert!(
        (wide_right - narrow_right).abs() < 0.01,
        "right edges should match, got {} and {}",
        wide_right,
        narrow_right
    );
}

#[test]
fn align_moves_a_shape_label_inside_the_shape() {
    let svg = render(r#"rect r [width: 300, height: 40, label: "hi", align: left]"#);
    let label_line = svg
        .lines()
        .find(|l| l.contains(">hi<"))
        .expect("label should render");
    assert!(
        label_line.contains(r#"text-anchor="start""#),
        "got: {}",
        label_line
    );
    // Inset from the border rather than flush against it.
    let x = number_attr(label_line, "x");
    assert!(x > 0.0 && x < 20.0, "expected a small inset, got {}", x);
}
