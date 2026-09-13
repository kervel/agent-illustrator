//! Things the linter used to stay quiet about, where silence cost the author
//! a render-and-squint cycle.

use agent_illustrator::{render_with_lint, RenderConfig};

fn lint(source: &str) -> Vec<agent_illustrator::LintWarning> {
    let (_svg, warnings) =
        render_with_lint(source, RenderConfig::new().with_lint(true)).expect("should render");
    warnings
}

fn messages(warnings: &[agent_illustrator::LintWarning]) -> Vec<String> {
    warnings.iter().map(|w| w.message.clone()).collect()
}

#[test]
fn a_shape_landing_on_a_grids_contents_is_reported() {
    // The grid itself paints nothing, so it is not what you collided with —
    // but its children are one level down, where sibling checks never look.
    let source = r#"
grid g [cols: 3, rows: 1, gap: 0, cell_width: 60, cell_height: 40] {
    rect kind_a [at: [0,0], label: "a", fill: accent-light, stroke: accent-dark]
    rect kind_b [at: [0,1], label: "b", fill: accent-light, stroke: accent-dark]
}
rect los [width: 50, height: 30, x: 65, y: 5, fill: secondary-1]
"#;
    let warnings = lint(source);
    assert!(
        messages(&warnings)
            .iter()
            .any(|m| m.contains("\"los\"") && m.contains("\"kind_b\"")),
        "expected los to be reported against the digit it lands on, got: {:?}",
        messages(&warnings)
    );
}

#[test]
fn an_unknown_modifier_is_reported_rather_than_dropped() {
    // An invented key parses fine and is then ignored, so the diagram renders
    // without complaint and the author only notices from the picture.
    let warnings = lint(r#"rect a [width: 100, height: 40, label: "x", fill_label: accent-1]"#);
    assert!(
        messages(&warnings)
            .iter()
            .any(|m| m.contains("fill_label") && m.contains("ignored")),
        "got: {:?}",
        messages(&warnings)
    );
}

#[test]
fn keys_the_engine_really_uses_are_not_reported() {
    let source = r#"
grid g [cols: 2, rows: 1, gap: 4, cell_width: 40, cell_height: 40,
        col_labels: ["a", "b"]] {
    rect [at: [0, 0]]
}
"#;
    let unknown: Vec<String> = messages(&lint(source))
        .into_iter()
        .filter(|m| m.contains("unknown modifier"))
        .collect();
    assert!(unknown.is_empty(), "false positives: {:?}", unknown);
}

#[test]
fn an_unknown_modifier_inside_a_keyframe_is_reported_too() {
    let source = r#"
rect a [width: 40, height: 40]
keyframe "one" {
    transform a [wobble: 3]
}
"#;
    assert!(
        messages(&lint(source))
            .iter()
            .any(|m| m.contains("wobble") && m.contains("keyframe")),
        "got: {:?}",
        messages(&lint(source))
    );
}

#[test]
fn contains_discarding_a_declared_size_is_reported() {
    // `contains` frees both axes, so a rule declared 3px tall comes back as
    // tall as what it contains. Correct, but it must not be silent.
    let source = r#"
grid g [cols: 5, rows: 4, gap: 4, cell_width: 46, cell_height: 52] {
    text "7" [at: [0, 0], align: center]
}
rect streep [height: 3, fill: foreground-1]
constrain streep contains g.cell(1, 1), g.cell(1, 3)
"#;
    assert!(
        messages(&lint(source))
            .iter()
            .any(|m| m.contains("streep") && m.contains("height: 3") && m.contains("contains")),
        "got: {:?}",
        messages(&lint(source))
    );
}

#[test]
fn a_container_that_keeps_its_declared_size_is_not_reported() {
    // The size survived, so there is nothing to warn about.
    let source = r#"
rect a [width: 40, height: 40]
rect b [width: 40, height: 40]
constrain b.x = a.right + 20
constrain b.y = a.y
rect bg [fill: accent-light, opacity: 0.3]
constrain bg contains a, b [padding: 10]
"#;
    let noisy: Vec<String> = messages(&lint(source))
        .into_iter()
        .filter(|m| m.contains("sizes both axes"))
        .collect();
    assert!(noisy.is_empty(), "false positives: {:?}", noisy);
}
