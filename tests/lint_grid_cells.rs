//! Grid cells exist only so `g.cell(r, c)` has something to address. Nothing
//! is drawn for them, so they can never collide — least of all with the child
//! placed inside them.

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
fn a_child_does_not_collide_with_its_own_cell() {
    let source = r#"
grid deling [cols: 5, rows: 4, gap: 4, cell_width: 46, cell_height: 52] {
    rect bd6 [at: [2, 4], fill: accent-light]
    rect [at: [3, 1], fill: accent-light]
    text "7" [at: [0, 0], align: center]
}
"#;
    let warnings = lint(source);
    assert!(
        warnings.is_empty(),
        "a correct grid must lint clean, got: {:?}",
        messages(&warnings)
    );
}

#[test]
fn two_children_in_one_cell_are_still_reported() {
    // The cells are exempt; the children are not.
    let source = r#"
grid deling [cols: 5, rows: 4, gap: 4, cell_width: 46, cell_height: 52] {
    rect one [at: [3, 1], fill: accent-light]
    rect two [at: [3, 1], fill: accent-2]
}
"#;
    let warnings = lint(source);
    assert!(
        messages(&warnings)
            .iter()
            .any(|m| m.contains("\"one\"") && m.contains("\"two\"")),
        "expected the real overlap, got: {:?}",
        messages(&warnings)
    );
}

#[test]
fn generated_cell_ids_do_not_make_a_grid_look_like_a_template() {
    // Cell ids are `{grid}__cell_r_c`, which share the grid's name as a
    // prefix. Counting them as template internals used to switch off sibling
    // overlap checks for every grid whose own children are all anonymous.
    let source = r#"
grid deling [cols: 5, rows: 4, gap: 4, cell_width: 46, cell_height: 52] {
    rect [at: [3, 1], fill: accent-light]
    rect [at: [3, 1], fill: accent-2]
}
"#;
    let warnings = lint(source);
    assert!(
        !warnings.is_empty(),
        "two anonymous rects in one cell overlap and must be reported"
    );
}

#[test]
fn a_rule_drawn_across_a_grid_is_not_a_collision() {
    // A grid renders as a bare <g>: it paints nothing, so a line drawn
    // inside its bounds has not collided with anything. This is the recipe
    // the skill gives for a division rule.
    let source = r#"
grid g [cols: 5, rows: 4, gap: 4, cell_width: 46, cell_height: 52] {
    text "7" [at: [0, 0], align: center]
}
rect rule [height: 3, fill: foreground-1]
constrain rule.left = g.cell(1, 1).left
constrain rule.right = g.cell(1, 3).right
constrain rule.top = g.cell(1, 3).bottom + 4
"#;
    let warnings = lint(source);
    assert!(
        warnings.is_empty(),
        "expected no warnings, got: {:?}",
        messages(&warnings)
    );
}

#[test]
fn a_container_that_paints_something_is_still_checked() {
    // The exemption is for containers that draw nothing. One with a fill is
    // a visible zone, and a shape landing on top of it is worth reporting.
    let source = r#"
col zone [fill: accent-light] {
    rect inner [width: 80, height: 40]
}
rect over [width: 40, height: 20]
constrain over.center_x = zone.center_x
constrain over.center_y = zone.center_y
"#;
    let warnings = lint(source);
    assert!(
        !warnings.is_empty(),
        "a filled container should still report a shape drawn over it"
    );
}

#[test]
fn an_anonymous_child_is_named_by_its_cell() {
    let source = r#"
grid deling [cols: 5, rows: 4, gap: 4, cell_width: 46, cell_height: 52] {
    rect one [at: [3, 1], fill: accent-light]
    rect [at: [3, 1], fill: accent-2]
}
"#;
    let warnings = lint(source);
    assert!(
        messages(&warnings)
            .iter()
            .any(|m| m.contains("cell [3,1] of deling")),
        "an anonymous grid child should be named by its cell, got: {:?}",
        messages(&warnings)
    );
}
