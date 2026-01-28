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
    let doc =
        resolve_templates(doc, &mut registry).map_err(|e| format!("Template error: {:?}", e))?;
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
    assert!(
        right_anchor.is_some(),
        "right_pin anchor should exist on c1"
    );

    // Verify anchor directions
    let left_dir = get_anchor_direction(&result, "c1", "left_pin");
    let right_dir = get_anchor_direction(&result, "c1", "right_pin");

    assert!(left_dir.is_some(), "left_pin direction should exist");
    assert!(right_dir.is_some(), "right_pin direction should exist");

    // Left anchor should point left (negative x direction)
    let (dx, _dy) = left_dir.unwrap();
    assert!(
        dx < 0.0,
        "Left anchor should point left (negative x), got dx={}",
        dx
    );

    // Right anchor should point right (positive x direction)
    let (dx, _dy) = right_dir.unwrap();
    assert!(
        dx > 0.0,
        "Right anchor should point right (positive x), got dx={}",
        dx
    );
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
    assert!(
        right_dir.is_some(),
        "right_pin direction should exist on c1"
    );
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
    assert_eq!(
        rotations.get("b1"),
        Some(&90.0),
        "b1 should have 90° rotation"
    );
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

/// Helper: rotate a point around a center by angle degrees (CW, SVG convention)
fn rotate_point(x: f64, y: f64, cx: f64, cy: f64, angle_deg: f64) -> (f64, f64) {
    let rad = angle_deg * std::f64::consts::PI / 180.0;
    let dx = x - cx;
    let dy = y - cy;
    let rx = dx * rad.cos() - dy * rad.sin() + cx;
    let ry = dx * rad.sin() + dy * rad.cos() + cy;
    (rx, ry)
}

#[test]
fn test_rotated_anchor_positions_match_svg_transform() {
    // Verify that the mathematically transformed anchor positions used for routing
    // match where the SVG transform visually places the anchor.
    // This is critical: connections are drawn in global coords, elements are
    // rendered with SVG group transforms.
    let source = r#"
        template "box" {
            rect body [width: 40, height: 20, fill: #4a6fa5]
            anchor left_conn [position: body.left, direction: left]
            anchor right_conn [position: body.right, direction: right]
            anchor top_conn [position: body.top, direction: up]
            anchor bottom_conn [position: body.bottom, direction: down]
        }

        box a
        box b [rotation: 90]

        constrain b.left = a.right + 80
        constrain b.vertical_center = a.vertical_center
    "#;

    let result = compute_layout(source).expect("Should compute layout");

    // Get a's anchor (no rotation - straightforward)
    let a_right =
        get_anchor_position(&result, "a", "right_conn").expect("a.right_conn should exist");

    // Get b's anchors (rotated 90°)
    let b_left = get_anchor_position(&result, "b", "left_conn").expect("b.left_conn should exist");
    let b_right =
        get_anchor_position(&result, "b", "right_conn").expect("b.right_conn should exist");
    let b_top = get_anchor_position(&result, "b", "top_conn").expect("b.top_conn should exist");
    let b_bottom =
        get_anchor_position(&result, "b", "bottom_conn").expect("b.bottom_conn should exist");

    // After 90° CW rotation of a 40x20 box:
    // - left_conn (was at left edge, mid height) should move to top center
    // - right_conn (was at right edge) should move to bottom center
    // - top_conn (was at top center) should move to right middle
    // - bottom_conn (was at bottom center) should move to left middle

    // Verify relative positions: left_conn should be above right_conn after 90° rotation
    assert!(
        b_left.1 < b_right.1,
        "After 90° rotation, left_conn ({:.1},{:.1}) should be above right_conn ({:.1},{:.1})",
        b_left.0,
        b_left.1,
        b_right.0,
        b_right.1
    );

    // top_conn should be to the right of bottom_conn after 90° rotation
    assert!(
        b_top.0 > b_bottom.0,
        "After 90° rotation, top_conn ({:.1},{:.1}) should be right of bottom_conn ({:.1},{:.1})",
        b_top.0,
        b_top.1,
        b_bottom.0,
        b_bottom.1
    );

    // Verify the computed anchor positions match what SVG transform would produce.
    // Get b's group bounds center (SVG rotation center)
    let b_bounds = get_element_bounds(&result, "b").expect("b should exist");
    let b_cx = b_bounds.0 + b_bounds.2 / 2.0;
    let b_cy = b_bounds.1 + b_bounds.3 / 2.0;

    // Get b_body's local bounds (pre-rotation)
    let b_body = get_element_bounds(&result, "b_body").expect("b_body should exist");
    let body_left_x = b_body.0;
    let body_center_y = b_body.1 + b_body.3 / 2.0;
    let body_right_x = b_body.0 + b_body.2;
    let body_center_x = b_body.0 + b_body.2 / 2.0;
    let body_top_y = b_body.1;

    // The SVG transform rotates around (b_cx, b_cy)
    // The anchor positions should match the rotated local positions
    let (expected_left_x, expected_left_y) =
        rotate_point(body_left_x, body_center_y, b_cx, b_cy, 90.0);
    let (expected_right_x, expected_right_y) =
        rotate_point(body_right_x, body_center_y, b_cx, b_cy, 90.0);
    let (expected_top_x, expected_top_y) =
        rotate_point(body_center_x, body_top_y, b_cx, b_cy, 90.0);

    let tol = 1.0; // 1px tolerance
    assert!(
        (b_left.0 - expected_left_x).abs() < tol && (b_left.1 - expected_left_y).abs() < tol,
        "left_conn: expected ({:.1},{:.1}), got ({:.1},{:.1})",
        expected_left_x,
        expected_left_y,
        b_left.0,
        b_left.1
    );
    assert!(
        (b_right.0 - expected_right_x).abs() < tol && (b_right.1 - expected_right_y).abs() < tol,
        "right_conn: expected ({:.1},{:.1}), got ({:.1},{:.1})",
        expected_right_x,
        expected_right_y,
        b_right.0,
        b_right.1
    );
    assert!(
        (b_top.0 - expected_top_x).abs() < tol && (b_top.1 - expected_top_y).abs() < tol,
        "top_conn: expected ({:.1},{:.1}), got ({:.1},{:.1})",
        expected_top_x,
        expected_top_y,
        b_top.0,
        b_top.1
    );
}

#[test]
fn test_rotated_anchor_directions() {
    // Verify anchor directions transform correctly for all 4 cardinal rotations
    let source = r#"
        template "component" {
            rect body [width: 40, height: 20]
            anchor left_conn [position: body.left, direction: left]
            anchor right_conn [position: body.right, direction: right]
            anchor top_conn [position: body.top, direction: up]
            anchor bottom_conn [position: body.bottom, direction: down]
        }

        component c90 [rotation: 90]
        component c180 [rotation: 180]
        component c270 [rotation: 270]

        constrain c180.left = c90.right + 40
        constrain c270.left = c180.right + 40
        constrain c180.vertical_center = c90.vertical_center
        constrain c270.vertical_center = c90.vertical_center
    "#;

    let result = compute_layout(source).expect("Should compute layout");

    // 90° CW: Right→Down, Down→Left, Left→Up, Up→Right
    let (dx, dy) = get_anchor_direction(&result, "c90", "right_conn").unwrap();
    assert!(
        dy > 0.5,
        "right_conn at 90°: expected Down, got ({:.2},{:.2})",
        dx,
        dy
    );

    let (dx, dy) = get_anchor_direction(&result, "c90", "left_conn").unwrap();
    assert!(
        dy < -0.5,
        "left_conn at 90°: expected Up, got ({:.2},{:.2})",
        dx,
        dy
    );

    let (dx, dy) = get_anchor_direction(&result, "c90", "top_conn").unwrap();
    assert!(
        dx > 0.5,
        "top_conn at 90°: expected Right, got ({:.2},{:.2})",
        dx,
        dy
    );

    let (dx, dy) = get_anchor_direction(&result, "c90", "bottom_conn").unwrap();
    assert!(
        dx < -0.5,
        "bottom_conn at 90°: expected Left, got ({:.2},{:.2})",
        dx,
        dy
    );

    // 180°: Right→Left, Left→Right, Up→Down, Down→Up
    let (dx, _) = get_anchor_direction(&result, "c180", "right_conn").unwrap();
    assert!(
        dx < -0.5,
        "right_conn at 180°: expected Left, got dx={:.2}",
        dx
    );

    let (dx, _) = get_anchor_direction(&result, "c180", "left_conn").unwrap();
    assert!(
        dx > 0.5,
        "left_conn at 180°: expected Right, got dx={:.2}",
        dx
    );

    // 270°: Right→Up, Left→Down, Up→Left, Down→Right
    let (_, dy) = get_anchor_direction(&result, "c270", "right_conn").unwrap();
    assert!(
        dy < -0.5,
        "right_conn at 270°: expected Up, got dy={:.2}",
        dy
    );

    let (_, dy) = get_anchor_direction(&result, "c270", "left_conn").unwrap();
    assert!(
        dy > 0.5,
        "left_conn at 270°: expected Down, got dy={:.2}",
        dy
    );
}

#[test]
fn test_person_rotation_renders_all_angles() {
    // Verify the full person-rotation example computes without errors
    // and all expected elements exist with reasonable bounds
    let source = std::fs::read_to_string("examples/person-rotation.ail")
        .expect("Should read person-rotation.ail");
    let result = compute_layout(&source).expect("Should compute layout");

    // All six persons should exist
    for name in &["p0", "p90", "p180", "p270", "p45", "p135"] {
        assert!(
            result.elements.contains_key(*name),
            "Element {} should exist",
            name
        );
    }

    // Row 1: p0, p90, p180 should be horizontally ordered
    let p0 = get_element_bounds(&result, "p0").unwrap();
    let p90 = get_element_bounds(&result, "p90").unwrap();
    let p180 = get_element_bounds(&result, "p180").unwrap();
    assert!(p0.0 < p90.0, "p0 should be left of p90");
    assert!(p90.0 < p180.0, "p90 should be left of p180");

    // Row 2: p270, p45, p135 should be horizontally ordered
    let p270 = get_element_bounds(&result, "p270").unwrap();
    let p45 = get_element_bounds(&result, "p45").unwrap();
    let p135 = get_element_bounds(&result, "p135").unwrap();
    assert!(p270.0 < p45.0, "p270 should be left of p45");
    assert!(p45.0 < p135.0, "p45 should be left of p135");

    // Row 2 should be below row 1
    assert!(p270.1 > p0.1 + p0.3, "Row 2 should be below Row 1");

    // All persons should have anchors
    for name in &["p0", "p90", "p180", "p270", "p45", "p135"] {
        let crown = get_anchor_position(&result, name, "crown");
        let feet = get_anchor_position(&result, name, "feet");
        let hand_left = get_anchor_position(&result, name, "hand_left");
        let hand_right = get_anchor_position(&result, name, "hand_right");
        assert!(crown.is_some(), "{}.crown should exist", name);
        assert!(feet.is_some(), "{}.feet should exist", name);
        assert!(hand_left.is_some(), "{}.hand_left should exist", name);
        assert!(hand_right.is_some(), "{}.hand_right should exist", name);
    }

    // Verify rotated persons have transformed anchor positions
    // p90 (90° CW): crown (was up) should now be to the right
    let p90_crown = get_anchor_position(&result, "p90", "crown").unwrap();
    let p90_feet = get_anchor_position(&result, "p90", "feet").unwrap();
    assert!(
        p90_crown.0 > p90_feet.0,
        "p90 crown ({:.1},{:.1}) should be right of feet ({:.1},{:.1}) after 90° rotation",
        p90_crown.0,
        p90_crown.1,
        p90_feet.0,
        p90_feet.1
    );

    // p180 (180°): crown should be below feet (upside down)
    let p180_crown = get_anchor_position(&result, "p180", "crown").unwrap();
    let p180_feet = get_anchor_position(&result, "p180", "feet").unwrap();
    assert!(
        p180_crown.1 > p180_feet.1,
        "p180 crown ({:.1},{:.1}) should be below feet ({:.1},{:.1}) after 180° rotation",
        p180_crown.0,
        p180_crown.1,
        p180_feet.0,
        p180_feet.1
    );
}
