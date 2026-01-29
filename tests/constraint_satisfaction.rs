//! Integration tests verifying that constraint equations are satisfied
//! in the rendered output. These are NOT visual/style tests — they check
//! that the constraint solver actually produces coordinates matching the
//! declared constraints.

use agent_illustrator::{
    layout::{compute, resolve_constrain_statements, LayoutConfig},
    parse,
    template::{resolve_templates, TemplateRegistry},
    LayoutResult,
};

fn compute_layout(source: &str) -> Result<LayoutResult, String> {
    let doc = parse(source).map_err(|e| format!("Parse error: {:?}", e))?;
    let mut registry = TemplateRegistry::new();
    let doc =
        resolve_templates(doc, &mut registry).map_err(|e| format!("Template error: {:?}", e))?;
    let config = LayoutConfig::default();
    let mut result = compute(&doc, &config).map_err(|e| format!("Layout error: {:?}", e))?;
    resolve_constrain_statements(&mut result, &doc, &config)
        .map_err(|e| format!("Constraint error: {:?}", e))?;
    Ok(result)
}

fn get_bounds(result: &LayoutResult, id: &str) -> (f64, f64, f64, f64) {
    let elem = result
        .elements
        .get(id)
        .unwrap_or_else(|| panic!("element '{}' not found", id));
    (elem.bounds.x, elem.bounds.y, elem.bounds.width, elem.bounds.height)
}

const TOLERANCE: f64 = 1.0;

/// Reproduces the feedback-loops regression: two rows stacked in a col,
/// with `constrain a1.left = b1.left` aligning first items across rows.
/// All items in each row should remain at the same Y (row invariant).
#[test]
fn test_cross_row_constraint_preserves_row_alignment() {
    let source = r#"
col main [gap: 40] {
    row top_row [gap: 20] {
        rect a1 [width: 120, height: 50]
        rect a2 [width: 120, height: 50]
        rect a3 [width: 120, height: 50]
        rect a4 [width: 120, height: 50]
    }
    row bot_row [gap: 20] {
        rect b1 [width: 120, height: 50]
        rect b2 [width: 120, height: 50]
        rect b3 [width: 120, height: 50]
        rect b4 [width: 120, height: 50]
    }
}
constrain a1.left = b1.left
"#;

    let result = compute_layout(source).expect("layout should succeed");

    // The constraint should be satisfied
    let (a1_x, _, _, _) = get_bounds(&result, "a1");
    let (b1_x, _, _, _) = get_bounds(&result, "b1");
    assert!(
        (a1_x - b1_x).abs() < TOLERANCE,
        "constrain a1.left = b1.left not satisfied: a1.x={}, b1.x={}",
        a1_x,
        b1_x,
    );

    // Row invariant: all items in top_row should have the same Y
    let (_, a1_y, _, _) = get_bounds(&result, "a1");
    let (_, a2_y, _, _) = get_bounds(&result, "a2");
    let (_, a3_y, _, _) = get_bounds(&result, "a3");
    let (_, a4_y, _, _) = get_bounds(&result, "a4");
    assert!(
        (a1_y - a2_y).abs() < TOLERANCE
            && (a2_y - a3_y).abs() < TOLERANCE
            && (a3_y - a4_y).abs() < TOLERANCE,
        "top_row items should be at the same Y: a1={}, a2={}, a3={}, a4={}",
        a1_y, a2_y, a3_y, a4_y,
    );

    // Row invariant: all items in bot_row should have the same Y
    let (_, b1_y, _, _) = get_bounds(&result, "b1");
    let (_, b2_y, _, _) = get_bounds(&result, "b2");
    let (_, b3_y, _, _) = get_bounds(&result, "b3");
    let (_, b4_y, _, _) = get_bounds(&result, "b4");
    assert!(
        (b1_y - b2_y).abs() < TOLERANCE
            && (b2_y - b3_y).abs() < TOLERANCE
            && (b3_y - b4_y).abs() < TOLERANCE,
        "bot_row items should be at the same Y: b1={}, b2={}, b3={}, b4={}",
        b1_y, b2_y, b3_y, b4_y,
    );
}

/// Reproduces the railway topology regression: constraints inside deeply
/// nested containers (col > group > stack > col > row) reference elements
/// from a sibling group. Matches the actual nesting depth of the railway example.
#[test]
fn test_deep_cross_group_constraint_alignment() {
    let source = r#"
col diagram {
    group micro {
        col tracks [gap: 30] {
            row trackA [gap: 80] {
                circle a1 [size: 6]
                circle j1 [size: 6]
                circle a2 [size: 6]
            }
            constrain j1.center_x = midpoint(a1, a2)
        }
    }

    group meso {
        stack meso_content {
            col meso_tracks [gap: 30] {
                row mtrackA [gap: 80] {
                    circle ma1 [size: 6]
                    circle mj1 [size: 6]
                    circle ma2 [size: 6]
                }
                // Cross-group: align meso elements to micro elements
                constrain mj1.center_x = j1.center_x
                constrain ma1.center_x = a1.center_x
                constrain ma2.center_x = a2.center_x
            }
        }
    }
}
"#;

    let result = compute_layout(source).expect("layout should succeed");

    let get_cx = |id: &str| -> f64 {
        let elem = result.elements.get(id).unwrap_or_else(|| panic!("{} not found", id));
        elem.bounds.x + elem.bounds.width / 2.0
    };

    let a1_cx = get_cx("a1");
    let j1_cx = get_cx("j1");
    let a2_cx = get_cx("a2");
    let ma1_cx = get_cx("ma1");
    let mj1_cx = get_cx("mj1");
    let ma2_cx = get_cx("ma2");

    assert!(
        (ma1_cx - a1_cx).abs() < TOLERANCE,
        "ma1.center_x should equal a1.center_x: ma1={}, a1={}",
        ma1_cx, a1_cx,
    );
    assert!(
        (mj1_cx - j1_cx).abs() < TOLERANCE,
        "mj1.center_x should equal j1.center_x: mj1={}, j1={}",
        mj1_cx, j1_cx,
    );
    assert!(
        (ma2_cx - a2_cx).abs() < TOLERANCE,
        "ma2.center_x should equal a2.center_x: ma2={}, a2={}",
        ma2_cx, a2_cx,
    );
}

/// Load the actual railway-topology.ail example and verify key constraints.
#[test]
fn test_railway_topology_cross_level_alignment() {
    let source = std::fs::read_to_string("examples/railway-topology.ail")
        .expect("railway-topology.ail should exist");

    let result = compute_layout(&source).expect("layout should succeed");

    let get_cx = |id: &str| -> f64 {
        let elem = result.elements.get(id).unwrap_or_else(|| panic!("{} not found", id));
        elem.bounds.x + elem.bounds.width / 2.0
    };

    // Micro-level junction positions
    let jb1_cx = get_cx("jB1");
    let jb2_cx = get_cx("jB2");

    // Meso-level junction positions (should match micro)
    let mjb1_cx = get_cx("mjB1");
    let mjb2_cx = get_cx("mjB2");

    // constrain mjB1.center_x = jB1.center_x
    assert!(
        (mjb1_cx - jb1_cx).abs() < TOLERANCE,
        "mjB1.center_x should equal jB1.center_x: mjB1={}, jB1={}",
        mjb1_cx, jb1_cx,
    );

    // constrain mjB2.center_x = jB2.center_x
    assert!(
        (mjb2_cx - jb2_cx).abs() < TOLERANCE,
        "mjB2.center_x should equal jB2.center_x: mjB2={}, jB2={}",
        mjb2_cx, jb2_cx,
    );

    // Endpoint alignment: constrain ma1.center_x = a1.center_x
    let a1_cx = get_cx("a1");
    let ma1_cx = get_cx("ma1");
    assert!(
        (ma1_cx - a1_cx).abs() < TOLERANCE,
        "ma1.center_x should equal a1.center_x: ma1={}, a1={}",
        ma1_cx, a1_cx,
    );
}

/// Constraint-based layout: explicit positioning should be exactly satisfied.
#[test]
fn test_explicit_position_constraints_satisfied() {
    let source = r#"
group diagram {
    rect a [width: 100, height: 50]
    rect b [width: 100, height: 50]
    rect c [width: 100, height: 50]
}
constrain a.center_x = 200
constrain a.center_y = 100
constrain b.center_x = 400
constrain b.center_y = 100
constrain c.center_x = 300
constrain c.center_y = 250
"#;

    let result = compute_layout(source).expect("layout should succeed");

    let (ax, ay, aw, ah) = get_bounds(&result, "a");
    assert!(
        ((ax + aw / 2.0) - 200.0).abs() < TOLERANCE,
        "a.center_x should be 200, got {}",
        ax + aw / 2.0,
    );
    assert!(
        ((ay + ah / 2.0) - 100.0).abs() < TOLERANCE,
        "a.center_y should be 100, got {}",
        ay + ah / 2.0,
    );

    let (bx, by, bw, bh) = get_bounds(&result, "b");
    assert!(
        ((bx + bw / 2.0) - 400.0).abs() < TOLERANCE,
        "b.center_x should be 400, got {}",
        bx + bw / 2.0,
    );
    assert!(
        ((by + bh / 2.0) - 100.0).abs() < TOLERANCE,
        "b.center_y should be 100, got {}",
        by + bh / 2.0,
    );

    let (cx, cy, cw, ch) = get_bounds(&result, "c");
    assert!(
        ((cx + cw / 2.0) - 300.0).abs() < TOLERANCE,
        "c.center_x should be 300, got {}",
        cx + cw / 2.0,
    );
    assert!(
        ((cy + ch / 2.0) - 250.0).abs() < TOLERANCE,
        "c.center_y should be 250, got {}",
        cy + ch / 2.0,
    );
}
