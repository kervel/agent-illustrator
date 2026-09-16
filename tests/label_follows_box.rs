//! A label's point is derived from its element's box. The solver can move a
//! box, resize it, or both; the label goes with it in every case.
//!
//! Before this, the resize path wrote `bounds.width` and nothing else, so a
//! box sized by left+right constraints drew its label at the centre the box
//! had *before* it was stretched.

use agent_illustrator::{render, RenderConfig};

/// The x of the first `ai-label` text element in the output.
fn label_x(source: &str) -> f64 {
    let svg = render(source).expect("renders");
    let marker = r#"<text class="ai-label" x=""#;
    let start = svg.find(marker).expect("a label was rendered") + marker.len();
    let rest = &svg[start..];
    let end = rest.find('"').expect("x attribute closes");
    rest[..end].parse().expect("x is a number")
}

fn label_y(source: &str) -> f64 {
    let svg = render(source).expect("renders");
    let marker = r#"<text class="ai-label" "#;
    let start = svg.find(marker).expect("a label was rendered");
    let rest = &svg[start..];
    let ymark = rest.find(r#"y=""#).expect("y attribute") + 3;
    let tail = &rest[ymark..];
    let end = tail.find('"').expect("y closes");
    tail[..end].parse().expect("y is a number")
}

#[test]
fn a_label_centres_in_a_box_the_solver_widened() {
    // The motivating failure. 100..400 -> centre 250.
    let x = label_x(
        r#"
rect a [height: 40, label: "A"]
constrain a.left = 100
constrain a.right = 400
"#,
    );
    assert!(
        (x - 250.0).abs() < 0.5,
        "label should sit at the solved centre 250, got {x}"
    );
}

#[test]
fn a_label_centres_in_a_box_the_solver_heightened() {
    // Same defect on the other axis: 100..300 -> centre 200.
    let y = label_y(
        r#"
rect a [width: 80, label: "A"]
constrain a.top = 100
constrain a.bottom = 300
"#,
    );
    assert!(
        (y - 200.0).abs() < 0.5,
        "label should sit at the solved middle 200, got {y}"
    );
}

#[test]
fn a_label_follows_a_box_that_is_only_moved() {
    // Regression guard: the shift path already worked and must keep working.
    let x = label_x(
        r#"
rect a [width: 100, height: 40, label: "A"]
constrain a.center_x = 500
constrain a.center_y = 300
"#,
    );
    assert!((x - 500.0).abs() < 0.5, "expected 500, got {x}");
}

#[test]
fn a_label_follows_a_box_that_is_moved_and_resized() {
    // 200..600 -> centre 400.
    let x = label_x(
        r#"
rect a [height: 40, label: "A"]
constrain a.left = 200
constrain a.right = 600
"#,
    );
    assert!((x - 400.0).abs() < 0.5, "expected 400, got {x}");
}

#[test]
fn an_outside_label_stays_under_a_resized_box() {
    // `label_position: below` with `align: end` lines up on the box's right
    // edge, so it must track the *solved* right edge, not the authored one.
    let x = label_x(
        r#"
rect a [height: 40, label: "cap", label_position: below, align: end]
constrain a.left = 100
constrain a.right = 400
"#,
    );
    assert!(
        (x - 400.0).abs() < 0.5,
        "an end-aligned caption should sit on the solved right edge 400, got {x}"
    );
}

#[test]
fn the_reported_bar_case_renders_its_name_in_the_middle() {
    // friction.txt P0-1(3), reduced: a version bar spanning two tick marks.
    let source = r#"
rect t_start [width: 1, height: 1, fill: none, stroke: none]
constrain t_start.center_x = 100
rect t_now [width: 1, height: 1, fill: none, stroke: none]
constrain t_now.center_x = 400
rect v1 [height: 56, label: "V1"]
constrain v1.left  = t_start.center_x
constrain v1.right = t_now.center_x + 70
"#;
    // 100..470 -> centre 285.
    let x = label_x(source);
    assert!(
        (x - 285.0).abs() < 0.5,
        "the bar's name should be centred at 285, got {x}"
    );
}

#[test]
fn lint_is_quiet_about_a_label_that_now_sits_where_it_belongs() {
    // The stale position used to push the label out of its own box, which the
    // straddle rule then reported. Fixing the position must silence that.
    let (_svg, warnings) = agent_illustrator::render_with_lint(
        r#"
rect a [height: 40, label: "A"]
constrain a.left = 100
constrain a.right = 400
"#,
        RenderConfig::new().with_lint(true),
    )
    .expect("renders");
    let straddles: Vec<_> = warnings
        .iter()
        .filter(|w| w.message.contains("straddle"))
        .collect();
    assert!(straddles.is_empty(), "got: {straddles:?}");
}
