//! Integration tests for anchor-based constraints (Feature 011).
//!
//! Tests that constraint expressions can reference anchor positions
//! using the `element.anchor_x` and `element.anchor_y` syntax.

use std::collections::HashMap;

use agent_illustrator::{
    layout::{compute, resolve_constrain_statements, LayoutConfig},
    parse,
    parser::ast::{Statement, StyleValue},
    template::{resolve_templates, TemplateRegistry},
    Document, LayoutResult,
};

/// Extract rotation modifiers from template instances in a document.
fn extract_template_rotations(doc: &Document) -> HashMap<String, f64> {
    let mut rotations = HashMap::new();

    fn visit_statements(
        stmts: &[agent_illustrator::parser::ast::Spanned<Statement>],
        rotations: &mut HashMap<String, f64>,
    ) {
        for stmt in stmts {
            match &stmt.node {
                Statement::TemplateInstance(inst) => {
                    for (key, value) in &inst.arguments {
                        if key.node.0 == "rotation" {
                            if let StyleValue::Number { value: angle, .. } = &value.node {
                                rotations.insert(inst.instance_name.node.0.clone(), *angle);
                            }
                        }
                    }
                }
                Statement::Layout(l) => {
                    visit_statements(&l.children, rotations);
                }
                Statement::Group(g) => {
                    visit_statements(&g.children, rotations);
                }
                _ => {}
            }
        }
    }

    visit_statements(&doc.statements, &mut rotations);
    rotations
}

/// Helper to parse, resolve templates, compute layout, and apply constraints
fn compute_layout(source: &str) -> Result<LayoutResult, String> {
    let doc = parse(source).map_err(|e| format!("Parse error: {:?}", e))?;
    let template_rotations = extract_template_rotations(&doc);

    let mut registry = TemplateRegistry::new();
    let doc =
        resolve_templates(doc, &mut registry).map_err(|e| format!("Template error: {:?}", e))?;
    let config = LayoutConfig::default();
    let mut result = compute(&doc, &config).map_err(|e| format!("Layout error: {:?}", e))?;

    if template_rotations.is_empty() {
        resolve_constrain_statements(&mut result, &doc, &config)
            .map_err(|e| format!("Constraint error: {:?}", e))?;
    } else {
        agent_illustrator::layout::engine::resolve_constrain_statements_two_phase(
            &mut result,
            &doc,
            &config,
            &template_rotations,
        )
        .map_err(|e| format!("Constraint error: {:?}", e))?;
    }

    Ok(result)
}

#[test]
fn test_anchor_constraint_basic_alignment() {
    // Template with a custom anchor offset from center
    let source = r#"
template "component" {
    rect body [width: 40, height: 60]
    rect connector [width: 10, height: 10, x: 20, y: 0]
    anchor output [position: connector.right, direction: right]
}

component c1
rect target [width: 20, height: 20]

// Align target center_x with c1's output anchor x-coordinate
constrain target.center_x = c1.output_x
"#;

    let result = compute_layout(source).expect("Layout should succeed");

    // Get the anchor position
    let c1_output = result
        .elements
        .get("c1")
        .and_then(|e| e.anchors.get("output"))
        .expect("c1 should have output anchor");

    let target = result.elements.get("target").expect("target should exist");
    let target_center_x = target.bounds.x + target.bounds.width / 2.0;

    // Target's center_x should match the output anchor's x position
    assert!(
        (target_center_x - c1_output.position.x).abs() < 1.0,
        "target center_x ({}) should equal c1.output_x ({})",
        target_center_x,
        c1_output.position.x
    );
}

#[test]
fn test_anchor_constraint_y_alignment() {
    let source = r#"
template "component" {
    rect body [width: 40, height: 60]
    rect top_part [width: 10, height: 10, x: 0, y: 0]
    anchor top_conn [position: top_part.top, direction: up]
}

component c1
rect target [width: 20, height: 20]

constrain target.center_y = c1.top_conn_y
"#;

    let result = compute_layout(source).expect("Layout should succeed");

    let c1_top_conn = result
        .elements
        .get("c1")
        .and_then(|e| e.anchors.get("top_conn"))
        .expect("c1 should have top_conn anchor");

    let target = result.elements.get("target").expect("target should exist");
    let target_center_y = target.bounds.y + target.bounds.height / 2.0;

    assert!(
        (target_center_y - c1_top_conn.position.y).abs() < 1.0,
        "target center_y ({}) should equal c1.top_conn_y ({})",
        target_center_y,
        c1_top_conn.position.y
    );
}

#[test]
fn test_anchor_constraint_with_offset() {
    let source = r#"
template "component" {
    rect body [width: 40, height: 60]
    rect connector [width: 10, height: 10, x: 20, y: 0]
    anchor output [position: connector.right, direction: right]
}

component c1
rect target [width: 20, height: 20]

constrain target.center_x = c1.output_x + 30
"#;

    let result = compute_layout(source).expect("Layout should succeed");

    let c1_output = result
        .elements
        .get("c1")
        .and_then(|e| e.anchors.get("output"))
        .expect("c1 should have output anchor");

    let target = result.elements.get("target").expect("target should exist");
    let target_center_x = target.bounds.x + target.bounds.width / 2.0;

    assert!(
        (target_center_x - (c1_output.position.x + 30.0)).abs() < 1.0,
        "target center_x ({}) should equal c1.output_x + 30 ({})",
        target_center_x,
        c1_output.position.x + 30.0
    );
}

#[test]
fn test_anchor_constraint_unknown_anchor_error() {
    let source = r#"
template "component" {
    rect body [width: 40, height: 60]
    anchor output [position: body.right, direction: right]
}

component c1
rect target [width: 20, height: 20]

constrain target.center_x = c1.nonexistent_x
"#;

    let result = compute_layout(source);
    assert!(result.is_err(), "Should fail for unknown anchor");
    let err = result.unwrap_err();
    assert!(
        err.contains("nonexistent") || err.contains("Unknown anchor"),
        "Error should mention the unknown anchor name, got: {}",
        err
    );
}

#[test]
fn test_existing_constraints_still_work() {
    // Regression: standard constraints (no anchors) must still work
    let source = r#"
rect a [width: 50, height: 50]
rect b [width: 50, height: 50]
constrain a.center_x = b.center_x
constrain a.bottom = b.top - 10
"#;

    let result = compute_layout(source).expect("Standard constraints should still work");

    let a = result.elements.get("a").expect("a should exist");
    let b = result.elements.get("b").expect("b should exist");

    let a_center_x = a.bounds.x + a.bounds.width / 2.0;
    let b_center_x = b.bounds.x + b.bounds.width / 2.0;

    assert!(
        (a_center_x - b_center_x).abs() < 1.0,
        "a and b should be horizontally centered"
    );
}

#[test]
fn test_anchor_constraint_with_rotation() {
    // Template with a custom anchor, instantiated with 90° rotation.
    // The anchor position should be post-rotation.
    let source = r#"
template "component" {
    rect body [width: 40, height: 20]
    anchor right_conn [position: body.right, direction: right]
}

component c1 [rotation: 90]
rect target [width: 10, height: 10]

// Align target with the rotated right_conn anchor
constrain target.center_x = c1.right_conn_x
constrain target.center_y = c1.right_conn_y
"#;

    let result = compute_layout(source).expect("Rotation + anchor constraint should succeed");

    let c1_right_conn = result
        .elements
        .get("c1")
        .and_then(|e| e.anchors.get("right_conn"))
        .expect("c1 should have right_conn anchor");

    let target = result.elements.get("target").expect("target should exist");
    let target_cx = target.bounds.x + target.bounds.width / 2.0;
    let target_cy = target.bounds.y + target.bounds.height / 2.0;

    // After 90° rotation, right_conn should be rotated to a new position
    // Target should be aligned with that post-rotation position
    assert!(
        (target_cx - c1_right_conn.position.x).abs() < 1.0,
        "target center_x ({}) should equal c1.right_conn_x ({})",
        target_cx,
        c1_right_conn.position.x
    );
    assert!(
        (target_cy - c1_right_conn.position.y).abs() < 1.0,
        "target center_y ({}) should equal c1.right_conn_y ({})",
        target_cy,
        c1_right_conn.position.y
    );
}

#[test]
fn test_mosfet_driver_renders() {
    // Verify the MOSFET driver example renders without errors
    let source = std::fs::read_to_string("examples/mosfet-driver.ail");
    if let Ok(source) = source {
        let result = agent_illustrator::render(&source);
        assert!(
            result.is_ok(),
            "mosfet-driver.ail should render: {:?}",
            result.err()
        );
    }
}
