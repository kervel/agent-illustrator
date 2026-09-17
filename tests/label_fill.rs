//! `fill` colours a shape; `label_fill` colours the words written on it.
//! Without the second, a grid whose cell contents carry meaning can only be
//! black, which rules grid out for anything colour-coded.

use agent_illustrator::{render_with_config, RenderConfig};

fn render(source: &str, configure: impl FnOnce(RenderConfig) -> RenderConfig) -> String {
    render_with_config(source, configure(RenderConfig::new())).expect("should render")
}

fn label_line<'a>(svg: &'a str, text: &str) -> &'a str {
    svg.lines()
        .find(|l| l.contains(&format!(">{}<", text)))
        .unwrap_or_else(|| panic!("no label {:?} in:\n{}", text, svg))
}

#[test]
fn a_label_can_be_coloured_independently_of_its_shape() {
    let svg = render(
        r#"rect a [width: 100, height: 40, label: "x", fill: background-1, label_fill: accent-dark]"#,
        |c| c,
    );
    let label = label_line(&svg, "x");
    assert!(
        label.contains(r#"fill="var(--accent-dark)""#),
        "label should carry its own colour: {}",
        label
    );

    let shape = svg
        .lines()
        .find(|l| l.contains("<rect"))
        .expect("shape should render");
    assert!(
        shape.contains(r#"fill="var(--background-1)""#),
        "the shape keeps its own fill: {}",
        shape
    );
}

#[test]
fn without_label_fill_a_label_carries_no_colour_of_its_own() {
    let svg = render(r#"rect a [width: 100, height: 40, label: "x"]"#, |c| c);
    let label = label_line(&svg, "x");
    assert!(
        !label.contains("fill="),
        "unset means the stylesheet decides: {}",
        label
    );
}

#[test]
fn each_grid_cell_can_have_its_own_label_colour() {
    let svg = render(
        r#"
grid g [cols: 2, rows: 1, gap: 4, cell_width: 46, cell_height: 46] {
    rect [at: [0, 0], label: "4", label_fill: accent-dark, stroke: none]
    rect [at: [0, 1], label: "7", label_fill: secondary-dark, stroke: none]
}
"#,
        |c| c,
    );
    assert!(label_line(&svg, "4").contains("var(--accent-dark)"));
    assert!(label_line(&svg, "7").contains("var(--secondary-dark)"));
}

#[test]
fn a_keyframe_can_recolour_a_label() {
    let source = r#"
rect c [width: 60, height: 40, label: "2"]
keyframe "a" {
}
keyframe "b" {
    transform c [label_fill: accent-dark]
}
"#;
    let frame_b = render(source, |mut c| {
        c.frame = Some("b".to_string());
        c
    });
    assert!(
        label_line(&frame_b, "2").contains("var(--accent-dark)"),
        "static frame should apply the new colour"
    );

    // ...and the animated build drives it from the frame CSS.
    let animated = render(source, |mut c| {
        c.animate = true;
        c
    });
    assert!(
        animated.contains("-c text { fill: var(--accent-dark); }"),
        "expected a per-frame rule for the label colour"
    );
}
