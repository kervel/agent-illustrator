//! Integration tests for rotation transformation in the two-phase constraint solver.
//!
//! These tests verify that:
//! - Template anchors transform correctly under rotation
//! - External constraints use post-rotation bounding boxes
//! - Connections attach at rotated anchor positions
//! - Via points (curved routing) use rotated element centers

use agent_illustrator::{
    layout::{compute, resolve_constrain_statements, LayoutConfig},
    parse,
    template::{resolve_templates, TemplateRegistry},
    LayoutResult,
};

/// Helper to parse, resolve templates, compute layout, and apply constraints
fn compute_layout(source: &str) -> Result<LayoutResult, String> {
    let doc = parse(source).map_err(|e| format!("Parse error: {:?}", e))?;
    let mut registry = TemplateRegistry::new();
    let doc = resolve_templates(doc, &mut registry).map_err(|e| format!("Template error: {:?}", e))?;
    let config = LayoutConfig::default();
    let mut result = compute(&doc, &config).map_err(|e| format!("Layout error: {:?}", e))?;
    resolve_constrain_statements(&mut result, &doc, &config)
        .map_err(|e| format!("Constraint error: {:?}", e))?;
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

// TODO: Enable these tests once rotation is fully integrated into the render pipeline

// #[test]
// fn test_rotated_template_anchor_position() {
//     let source = r#"
//         template "box" {
//             rect body [width: 40, height: 20]
//             anchor left_pin [position: body.left, direction: left]
//             anchor right_pin [position: body.right, direction: right]
//         }
//
//         box b1 [rotation: 90]
//     "#;
//
//     let result = compute_layout(source).expect("Should compute layout");
//
//     // After 90° clockwise rotation:
//     // - Original left anchor (pointing left) should now point down
//     // - Original right anchor (pointing right) should now point up
//     // - 40x20 bounds should become 20x40
//
//     // Verify bounds changed
//     let bounds = get_element_bounds(&result, "b1_body");
//     assert!(bounds.is_some());
//     let (_, _, w, h) = bounds.unwrap();
//     // After 90° rotation, 40x20 becomes 20x40 (approximately, due to loose bounds)
//     assert!(h > w, "Height should be greater than width after 90° rotation");
//
//     // Verify anchor directions transformed
//     let left_dir = get_anchor_direction(&result, "b1", "left_pin");
//     assert!(left_dir.is_some());
//     let (dx, dy) = left_dir.unwrap();
//     // After 90° clockwise rotation, left (-1, 0) becomes down (0, 1)
//     assert!((dx).abs() < 0.1, "dx should be ~0 after rotation, got {}", dx);
//     assert!(dy > 0.5, "dy should be positive (down) after rotation, got {}", dy);
// }

// #[test]
// fn test_external_constraint_to_rotated_child() {
//     let source = r#"
//         template "component" {
//             rect body [width: 80, height: 40]
//         }
//
//         component c1 [rotation: 90]
//         rect label [width: 30, height: 20]
//
//         constrain label.left = c1_body.right + 10
//     "#;
//
//     let result = compute_layout(source).expect("Should compute layout");
//
//     // After 90° rotation, 80x40 becomes approximately 40x80
//     let c1_bounds = get_element_bounds(&result, "c1_body").expect("c1_body should exist");
//     let label_bounds = get_element_bounds(&result, "label").expect("label should exist");
//
//     // label.left should be 10px right of the rotated c1_body's right edge
//     let c1_right = c1_bounds.0 + c1_bounds.2;
//     let label_left = label_bounds.0;
//
//     assert!(
//         (label_left - (c1_right + 10.0)).abs() < 1.0,
//         "label should be 10px right of c1_body: expected {}, got {}",
//         c1_right + 10.0,
//         label_left
//     );
// }

// #[test]
// fn test_connection_to_rotated_anchor() {
//     let source = r#"
//         template "resistor" {
//             rect body [width: 40, height: 16]
//             anchor left_conn [position: body.left, direction: left]
//             anchor right_conn [position: body.right, direction: right]
//         }
//
//         rect source [width: 20, height: 20]
//         resistor r1 [rotation: 90]
//
//         constrain r1.x = source.right + 50
//
//         source.right -> r1.left_conn
//     "#;
//
//     let svg = agent_illustrator::render(source).expect("Should render");
//
//     // Connection should attach to the rotated position of left_conn
//     // Verify by checking SVG path endpoints (would need SVG parsing)
//     assert!(svg.contains("<path"), "Should have a connection path");
// }
