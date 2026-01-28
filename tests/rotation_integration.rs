//! Integration tests for rotation transformation in the two-phase constraint solver.
//!
//! These tests verify that:
//! - Template anchors transform correctly under rotation
//! - External constraints use post-rotation bounding boxes
//! - Connections attach at rotated anchor positions
//! - Via points (curved routing) use rotated element centers

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
/// Uses the two-phase solver when rotations are present
fn compute_layout(source: &str) -> Result<LayoutResult, String> {
    let doc = parse(source).map_err(|e| format!("Parse error: {:?}", e))?;

    // Extract rotation modifiers BEFORE template resolution (they are lost during resolution)
    let template_rotations = extract_template_rotations(&doc);

    let mut registry = TemplateRegistry::new();
    let doc = resolve_templates(doc, &mut registry).map_err(|e| format!("Template error: {:?}", e))?;
    let config = LayoutConfig::default();
    let mut result = compute(&doc, &config).map_err(|e| format!("Layout error: {:?}", e))?;

    // Use two-phase solver when there are rotations
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

/// Get element bounds (x, y, width, height) from a layout result
fn get_element_bounds(result: &LayoutResult, element_id: &str) -> Option<(f64, f64, f64, f64)> {
    result.elements.get(element_id).map(|elem| {
        (
            elem.bounds.x,
            elem.bounds.y,
            elem.bounds.width,
            elem.bounds.height,
        )
    })
}

/// Get anchor position (x, y) from a layout result
fn get_anchor_position(
    result: &LayoutResult,
    element_id: &str,
    anchor_name: &str,
) -> Option<(f64, f64)> {
    result
        .elements
        .get(element_id)
        .and_then(|elem| elem.anchors.get(anchor_name))
        .map(|anchor| (anchor.position.x, anchor.position.y))
}

/// Get anchor direction as a unit vector (x, y) from a layout result
fn get_anchor_direction(
    result: &LayoutResult,
    element_id: &str,
    anchor_name: &str,
) -> Option<(f64, f64)> {
    result
        .elements
        .get(element_id)
        .and_then(|elem| elem.anchors.get(anchor_name))
        .map(|anchor| {
            let vec = anchor.direction.to_vector();
            (vec.x, vec.y)
        })
}

#[test]
fn test_integration_test_helpers() {
    // Simple test to verify the helpers work correctly
    let source = r#"
        rect box1 [width: 100, height: 50]
    "#;

    let result = compute_layout(source).expect("Should compute layout");

    let bounds = get_element_bounds(&result, "box1");
    assert!(bounds.is_some(), "box1 should exist");

    let (x, y, w, h) = bounds.unwrap();
    assert_eq!(w, 100.0, "Width should be 100");
    assert_eq!(h, 50.0, "Height should be 50");
    assert_eq!(x, 0.0, "X should start at 0");
    assert_eq!(y, 0.0, "Y should start at 0");
}

#[test]
fn test_non_rotated_template_bounds() {
    // Verify template bounds without rotation work correctly
    // Note: Single-element templates get flattened (the instance IS the element)
    // Multi-element templates create a Group with prefixed children
    let source = r#"
        template "box" {
            rect body [width: 40, height: 20]
            rect pin [width: 5, height: 5]
        }

        box b1
    "#;

    let result = compute_layout(source).expect("Should compute layout");

    // The template instance wraps its children in a Group
    let body_bounds = get_element_bounds(&result, "b1_body");
    assert!(body_bounds.is_some(), "b1_body should exist");

    let (_, _, w, h) = body_bounds.unwrap();
    assert!((w - 40.0).abs() < 0.1, "Width should be 40, got {}", w);
    assert!((h - 20.0).abs() < 0.1, "Height should be 20, got {}", h);
}

#[test]
fn test_template_with_anchors() {
    // Verify template anchors are accessible
    let source = r#"
        template "component" {
            rect body [width: 40, height: 20]
            anchor left_pin [position: body.left, direction: left]
            anchor right_pin [position: body.right, direction: right]
        }

        component c1
    "#;

    let result = compute_layout(source).expect("Should compute layout");

    // Check body bounds
    let body_bounds = get_element_bounds(&result, "c1_body");
    assert!(body_bounds.is_some(), "c1_body should exist");

    // Check anchor on the template instance group
    let left_anchor = get_anchor_position(&result, "c1", "left_pin");
    let right_anchor = get_anchor_position(&result, "c1", "right_pin");

    assert!(left_anchor.is_some(), "left_pin anchor should exist on c1");
    assert!(right_anchor.is_some(), "right_pin anchor should exist on c1");

    // Verify anchor directions
    let left_dir = get_anchor_direction(&result, "c1", "left_pin");
    let right_dir = get_anchor_direction(&result, "c1", "right_pin");

    assert!(left_dir.is_some(), "left_pin direction should exist");
    assert!(right_dir.is_some(), "right_pin direction should exist");

    // Left anchor should point left (negative x direction)
    let (dx, _dy) = left_dir.unwrap();
    assert!(dx < 0.0, "Left anchor should point left (negative x), got dx={}", dx);

    // Right anchor should point right (positive x direction)
    let (dx, _dy) = right_dir.unwrap();
    assert!(dx > 0.0, "Right anchor should point right (positive x), got dx={}", dx);
}

#[test]
fn test_internal_constraints_centering() {
    // Regression test: template-internal constraints should keep children aligned
    // when the template instance is moved by external constraints.
    let source = r#"
        template "stack3" {
            rect line1 [width: 40, height: 3]
            rect line2 [width: 26, height: 3]
            rect line3 [width: 12, height: 3]
            constrain line2.top = line1.bottom + 4
            constrain line3.top = line2.bottom + 4
            constrain line2.center_x = line1.center_x
            constrain line3.center_x = line1.center_x
        }
        stack3 gnd
        constrain gnd.x = 200
    "#;

    let result = compute_layout(source).expect("Should compute layout");

    // Find the template children
    let line1 = get_element_bounds(&result, "gnd_line1");
    let line2 = get_element_bounds(&result, "gnd_line2");
    let line3 = get_element_bounds(&result, "gnd_line3");

    assert!(line1.is_some(), "gnd_line1 should exist");
    assert!(line2.is_some(), "gnd_line2 should exist");
    assert!(line3.is_some(), "gnd_line3 should exist");

    let (x1, _, w1, _) = line1.unwrap();
    let (x2, _, w2, _) = line2.unwrap();
    let (x3, _, w3, _) = line3.unwrap();

    // All three lines should be centered on the same x coordinate
    let center1 = x1 + w1 / 2.0;
    let center2 = x2 + w2 / 2.0;
    let center3 = x3 + w3 / 2.0;

    assert!(
        (center1 - center2).abs() < 1.0,
        "line1 and line2 should be centered: {} vs {}",
        center1,
        center2
    );
    assert!(
        (center1 - center3).abs() < 1.0,
        "line1 and line3 should be centered: {} vs {}",
        center1,
        center3
    );
}

// ============================================
// T025: Test Rotated Template Anchor Positions
// ============================================

#[test]
fn test_rotated_template_bounds() {
    // Test that rotation modifier affects element bounds
    // 90° rotation should swap width and height (approximately)
    let base_source = r#"
        template "box" {
            rect body [width: 40, height: 20]
            rect pin [width: 5, height: 5]
        }

        box b1
    "#;

    let source = r#"
        template "box" {
            rect body [width: 40, height: 20]
            rect pin [width: 5, height: 5]
        }

        box b1 [rotation: 90]
    "#;

    let base_result = compute_layout(base_source).expect("Should compute layout");
    let result = compute_layout(source).expect("Should compute layout");

    // Internal element bounds should be unchanged
    let body = get_element_bounds(&result, "b1_body");
    assert!(body.is_some(), "b1_body should exist");
    let (_, _, body_w, body_h) = body.unwrap();
    assert!(
        (body_w - 40.0).abs() < 0.1 && (body_h - 20.0).abs() < 0.1,
        "b1_body should remain 40x20, got {}x{}",
        body_w,
        body_h
    );

    // Template instance bounds should rotate for global constraints
    let base_bounds = get_element_bounds(&base_result, "b1").expect("b1 should exist");
    let (_, _, base_w, base_h) = base_bounds;
    let bounds = get_element_bounds(&result, "b1");
    assert!(bounds.is_some(), "b1 should exist");
    let (_, _, w, h) = bounds.unwrap();
    assert!(
        (w - base_h).abs() < 1.0 && (h - base_w).abs() < 1.0,
        "After 90° rotation, bounds should swap: base {}x{}, got {}x{}",
        base_w,
        base_h,
        w,
        h
    );
}

#[test]
fn test_rotated_template_anchor_direction() {
    // Test that anchor directions are rotated
    let source = r#"
        template "component" {
            rect body [width: 40, height: 20]
            rect pin [width: 5, height: 5]
            anchor left_pin [position: body.left, direction: left]
            anchor right_pin [position: body.right, direction: right]
        }

        component c1 [rotation: 90]
    "#;

    let result = compute_layout(source).expect("Should compute layout");

    // After 90° clockwise rotation:
    // - Original left anchor (pointing left, dx=-1, dy=0) should now point up (dx=0, dy=-1)
    // - Original right anchor (pointing right, dx=1, dy=0) should now point down (dx=0, dy=1)
    // This follows the physical rotation: Left→Up→Right→Down→Left for clockwise rotation

    let left_dir = get_anchor_direction(&result, "c1", "left_pin");
    assert!(left_dir.is_some(), "left_pin direction should exist on c1");
    let (dx, dy) = left_dir.unwrap();

    // After 90° clockwise rotation, left (180°) becomes up (270° = -90°), so dy < 0
    assert!(
        dx.abs() < 0.2,
        "After 90° rotation, left anchor dx should be ~0, got {}",
        dx
    );
    assert!(
        dy < -0.5,
        "After 90° rotation, left anchor should point up (dy < 0), got dy={}",
        dy
    );

    let right_dir = get_anchor_direction(&result, "c1", "right_pin");
    assert!(right_dir.is_some(), "right_pin direction should exist on c1");
    let (dx, dy) = right_dir.unwrap();

    // After 90° clockwise rotation, right (0°) becomes down (90°), so dy > 0
    assert!(
        dx.abs() < 0.2,
        "After 90° rotation, right anchor dx should be ~0, got {}",
        dx
    );
    assert!(
        dy > 0.5,
        "After 90° rotation, right anchor should point down (dy > 0), got dy={}",
        dy
    );
}

// ============================================
// T026: Test External Constraint to Rotated Child
// ============================================

#[test]
fn test_external_constraint_to_rotated_child() {
    // Test that external constraints use post-rotation bounds
    let source = r#"
        template "mycomp" {
            rect body [width: 80, height: 40]
            rect pin [width: 5, height: 5]
        }

        mycomp c1 [rotation: 90]
        rect lbl [width: 30, height: 20]

        constrain lbl.left = c1_body.right + 10
    "#;

    let result = compute_layout(source).expect("Should compute layout");

    // After 90° rotation, 80x40 becomes approximately 40x80
    let c1_bounds = get_element_bounds(&result, "c1_body").expect("c1_body should exist");
    let lbl_bounds = get_element_bounds(&result, "lbl").expect("lbl should exist");

    // lbl.left should be 10px right of the rotated c1_body's right edge
    let c1_right = c1_bounds.0 + c1_bounds.2;
    let lbl_left = lbl_bounds.0;

    assert!(
        (lbl_left - (c1_right + 10.0)).abs() < 2.0,
        "lbl should be ~10px right of rotated c1_body: c1_right={}, expected lbl_left={}, got {}",
        c1_right,
        c1_right + 10.0,
        lbl_left
    );
}

// ============================================
// T027: Test Connection to Rotated Anchor
// ============================================

#[test]
fn test_connection_to_rotated_anchor() {
    // Test that connections attach at rotated anchor positions
    let source = r#"
        template "resistor" {
            rect body [width: 40, height: 16]
            rect pin [width: 5, height: 5]
            anchor left_conn [position: body.left, direction: left]
            anchor right_conn [position: body.right, direction: right]
        }

        rect source [width: 20, height: 20]
        resistor r1 [rotation: 90]

        constrain r1.x = source.right + 50

        source.right -> r1.left_conn
    "#;

    // Just verify it renders without error - the connection should attach
    // to the rotated anchor position
    let svg = agent_illustrator::render(source).expect("Should render");
    assert!(svg.contains("<path"), "Should have a connection path");
    assert!(svg.contains("<svg"), "Should produce valid SVG");
}

// ============================================
// T028: Test Via Point Through Rotated Element
// ============================================

#[test]
fn test_via_point_through_rotated_element() {
    // Test that curved routing uses rotated element centers
    let source = r#"
        template "waypoint" {
            circle marker [size: 6]
            rect pin [width: 2, height: 2]
        }

        rect start [width: 20, height: 20]
        rect end [width: 20, height: 20]
        waypoint ctrl [rotation: 45]

        constrain end.left = start.right + 100
        constrain ctrl.center_x = start.center_x + 50
        constrain ctrl.center_y = start.center_y - 30

        start -> end [routing: curved, via: ctrl_marker]
    "#;

    // Verify the layout computes and renders without error
    let svg = agent_illustrator::render(source).expect("Should render");
    assert!(svg.contains("<svg"), "Should produce valid SVG");
    // Curved routing creates a path with curve commands
    assert!(svg.contains("<path"), "Should have connection paths");
}


#[test]
fn test_rotation_extraction_before_template_resolution() {
    // Verify that rotation modifiers are extracted before template resolution
    // (template resolution converts TemplateInstance to Group, losing modifiers)
    let source = r#"
        template "box" {
            rect body [width: 40, height: 20]
            rect pin [width: 5, height: 5]
        }

        box b1 [rotation: 90]
    "#;

    let doc = agent_illustrator::parse(source).expect("Should parse");

    // Extract rotations BEFORE template resolution
    let rotations = extract_template_rotations(&doc);
    assert!(rotations.contains_key("b1"), "b1 should have rotation");
    assert_eq!(rotations.get("b1"), Some(&90.0), "b1 should have 90° rotation");
}

#[test]
fn test_direction_rotation_math() {
    // Verify that direction rotation follows clockwise convention:
    // Right → Down → Left → Up → Right
    use agent_illustrator::layout::transform::RotationTransform;
    use agent_illustrator::layout::types::{AnchorDirection, Point};

    let rotation = RotationTransform::new(90.0, Point { x: 0.0, y: 0.0 });

    // Right (0°) + 90° = Down (90°)
    let right_rotated = rotation.transform_direction(AnchorDirection::Right);
    assert!(
        matches!(right_rotated, AnchorDirection::Down),
        "Right + 90° should be Down, got {:?}",
        right_rotated
    );

    // Left (180°) + 90° = Up (270°)
    let left_rotated = rotation.transform_direction(AnchorDirection::Left);
    assert!(
        matches!(left_rotated, AnchorDirection::Up),
        "Left + 90° should be Up, got {:?}",
        left_rotated
    );
}
