//! "Put the caption under the box, aligned right" used to be two constraints
//! and a guessed offset. It is now one modifier.

use agent_illustrator::render;

/// Pull the y of the first `<text>` and the geometry of the first `<rect>`.
fn text_y(svg: &str) -> f64 {
    let at = svg.find("<text").expect("a text element");
    let y_at = svg[at..].find(" y=\"").expect("text y") + at + 4;
    svg[y_at..].split('"').next().unwrap().parse().unwrap()
}

fn rect_y(svg: &str) -> f64 {
    let at = svg.find("<rect").expect("a rect element");
    let y_at = svg[at..].find(" y=\"").expect("rect y") + at + 4;
    svg[y_at..].split('"').next().unwrap().parse().unwrap()
}

fn rect_height(svg: &str) -> f64 {
    let at = svg.find("<rect").expect("a rect element");
    let h_at = svg[at..].find(" height=\"").expect("rect height") + at + 9;
    svg[h_at..].split('"').next().unwrap().parse().unwrap()
}

#[test]
fn the_default_is_still_centred_inside() {
    let svg = render(r#"rect b [label: "x", width: 100, height: 40]"#).expect("renders");
    let centre = rect_y(&svg) + rect_height(&svg) / 2.0;
    assert!(
        (text_y(&svg) - centre).abs() < 1.0,
        "{} vs {}",
        text_y(&svg),
        centre
    );
}

#[test]
fn below_puts_the_label_under_the_shape() {
    let svg = render(
        r#"rect b [label: "based_on", width: 100, height: 40, label_position: below]"#,
    )
    .expect("renders");
    assert!(
        text_y(&svg) > rect_y(&svg) + rect_height(&svg),
        "label y {} should be below the box bottom {}",
        text_y(&svg),
        rect_y(&svg) + rect_height(&svg)
    );
}

#[test]
fn above_puts_the_label_over_the_shape() {
    let svg = render(r#"rect b [label: "Zone A", width: 100, height: 40, label_position: above]"#)
        .expect("renders");
    assert!(
        text_y(&svg) < rect_y(&svg),
        "{} vs {}",
        text_y(&svg),
        rect_y(&svg)
    );
}

#[test]
fn align_end_right_aligns_a_label_below() {
    let svg = render(
        r#"rect b [label: "based_on", width: 200, height: 40, label_position: below, align: end]"#,
    )
    .expect("renders");
    assert!(svg.contains(r#"text-anchor="end""#), "svg was: {svg}");
}

#[test]
fn align_start_left_aligns_a_label_above() {
    let svg = render(
        r#"rect b [label: "Zone A", width: 200, height: 40, label_position: above, align: start]"#,
    )
    .expect("renders");
    assert!(svg.contains(r#"text-anchor="start""#), "svg was: {svg}");
}

#[test]
fn label_offset_controls_the_gap() {
    let near = render(
        r#"rect b [label: "x", width: 100, height: 40, label_position: below, label_offset: 4]"#,
    )
    .expect("renders");
    let far = render(
        r#"rect b [label: "x", width: 100, height: 40, label_position: below, label_offset: 40]"#,
    )
    .expect("renders");
    assert!(
        text_y(&far) > text_y(&near),
        "{} vs {}",
        text_y(&far),
        text_y(&near)
    );
}

#[test]
fn an_outside_label_is_inside_the_canvas() {
    // If the label were not folded into the element's bounds it would be
    // clipped off the top of the viewBox.
    let svg = render(r#"rect b [label: "Zone A", width: 100, height: 40, label_position: above]"#)
        .expect("renders");
    let view = svg
        .split("viewBox=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .expect("a viewBox");
    let parts: Vec<f64> = view
        .split_whitespace()
        .map(|p| p.parse().unwrap())
        .collect();
    let (min_y, height) = (parts[1], parts[3]);
    assert!(
        text_y(&svg) >= min_y && text_y(&svg) <= min_y + height,
        "label at y={} escaped the viewBox {view}",
        text_y(&svg)
    );
}

#[test]
fn a_row_reserves_space_for_labels_below_its_children() {
    // gap: should measure from the label, not from the border, or the caption
    // of one box collides with the next.
    let svg = render(
        r#"
row [gap: 10] {
    rect a [label: "first caption", width: 80, height: 40, label_position: below]
    rect b [label: "second caption", width: 80, height: 40, label_position: below]
}
"#,
    )
    .expect("renders");
    assert!(svg.contains("first caption") && svg.contains("second caption"));
}

#[test]
fn left_and_right_place_the_label_beside_the_shape() {
    let right =
        render(r#"rect b [label: "x", width: 100, height: 40, label_position: right]"#)
            .expect("renders");
    assert!(right.contains(r#"text-anchor="start""#), "svg was: {right}");
    let left = render(r#"rect b [label: "x", width: 100, height: 40, label_position: left]"#)
        .expect("renders");
    assert!(left.contains(r#"text-anchor="end""#), "svg was: {left}");
}
