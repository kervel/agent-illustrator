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

/// Rotated template instances with custom anchors and cross-instance constraints.
/// Verifies that:
/// 1. `constrain p90.left = p0.right + 120` places p90 to the right of p0
/// 2. `constrain p90.vertical_center = p0.vertical_center` aligns centers vertically
/// 3. Rotated instances have correct relative positioning
#[test]
fn test_rotated_template_cross_instance_constraints() {
    let source = r#"
template "box" {
    col [gap: 4] {
        rect head [width: 20, height: 20]
        rect body [width: 30, height: 20]
    }
    constrain head.center_x = body.center_x
    anchor top_point [position: head.top - 2, direction: up]
    anchor bot_point [position: body.bottom + 2, direction: down]
}

box p0
box p90 [rotation: 90]
box p180 [rotation: 180]

constrain p90.left = p0.right + 80
constrain p180.left = p90.right + 80
constrain p90.vertical_center = p0.vertical_center
constrain p180.vertical_center = p0.vertical_center
"#;

    let result = compute_layout(source).expect("layout should succeed");

    let get_cy = |id: &str| -> f64 {
        let elem = result.elements.get(id).unwrap_or_else(|| panic!("{} not found", id));
        elem.bounds.y + elem.bounds.height / 2.0
    };
    let get_left = |id: &str| -> f64 {
        result.elements.get(id).unwrap_or_else(|| panic!("{} not found", id)).bounds.x
    };
    let get_right = |id: &str| -> f64 {
        let elem = result.elements.get(id).unwrap_or_else(|| panic!("{} not found", id));
        elem.bounds.x + elem.bounds.width
    };

    // p90.left = p0.right + 80
    let p0_right = get_right("p0");
    let p90_left = get_left("p90");
    assert!(
        (p90_left - (p0_right + 80.0)).abs() < TOLERANCE,
        "p90.left should be p0.right + 80: p90.left={}, p0.right+80={}",
        p90_left, p0_right + 80.0,
    );

    // p180.left = p90.right + 80
    let p90_right = get_right("p90");
    let p180_left = get_left("p180");
    assert!(
        (p180_left - (p90_right + 80.0)).abs() < TOLERANCE,
        "p180.left should be p90.right + 80: p180.left={}, p90.right+80={}",
        p180_left, p90_right + 80.0,
    );

    // Vertical center alignment
    let p0_cy = get_cy("p0");
    let p90_cy = get_cy("p90");
    let p180_cy = get_cy("p180");
    assert!(
        (p90_cy - p0_cy).abs() < TOLERANCE,
        "p90.vertical_center should equal p0.vertical_center: p90={}, p0={}",
        p90_cy, p0_cy,
    );
    assert!(
        (p180_cy - p0_cy).abs() < TOLERANCE,
        "p180.vertical_center should equal p0.vertical_center: p180={}, p0={}",
        p180_cy, p0_cy,
    );
}

/// Load the actual person-rotation.ail example and verify key constraints.
/// This exercises templates with path geometry, custom anchors, rotation,
/// and cross-instance positioning constraints.
#[test]
fn test_person_rotation_cross_instance_alignment() {
    let source = std::fs::read_to_string("examples/person-rotation.ail")
        .expect("person-rotation.ail should exist");

    let result = compute_layout(&source).expect("layout should succeed");

    let get_cy = |id: &str| -> f64 {
        let elem = result.elements.get(id).unwrap_or_else(|| panic!("{} not found", id));
        elem.bounds.y + elem.bounds.height / 2.0
    };
    let get_left = |id: &str| -> f64 {
        result.elements.get(id).unwrap_or_else(|| panic!("{} not found", id)).bounds.x
    };
    let get_right = |id: &str| -> f64 {
        let elem = result.elements.get(id).unwrap_or_else(|| panic!("{} not found", id));
        elem.bounds.x + elem.bounds.width
    };

    // Row 1: constrain p90.vertical_center = p0.vertical_center
    let p0_cy = get_cy("p0");
    let p90_cy = get_cy("p90");
    let p180_cy = get_cy("p180");
    assert!(
        (p90_cy - p0_cy).abs() < TOLERANCE,
        "p90.vertical_center should equal p0: p90={}, p0={}",
        p90_cy, p0_cy,
    );
    assert!(
        (p180_cy - p0_cy).abs() < TOLERANCE,
        "p180.vertical_center should equal p0: p180={}, p0={}",
        p180_cy, p0_cy,
    );

    // constrain p90.left = p0.right + 120
    let p0_right = get_right("p0");
    let p90_left = get_left("p90");
    assert!(
        (p90_left - (p0_right + 120.0)).abs() < TOLERANCE,
        "p90.left should be p0.right + 120: p90.left={}, expected={}",
        p90_left, p0_right + 120.0,
    );

    // constrain p180.left = p90.right + 120
    let p90_right = get_right("p90");
    let p180_left = get_left("p180");
    assert!(
        (p180_left - (p90_right + 120.0)).abs() < TOLERANCE,
        "p180.left should be p90.right + 120: p180.left={}, expected={}",
        p180_left, p90_right + 120.0,
    );

    // Row 2: constrain p45.vertical_center = p270.vertical_center
    let p270_cy = get_cy("p270");
    let p45_cy = get_cy("p45");
    let p135_cy = get_cy("p135");
    assert!(
        (p45_cy - p270_cy).abs() < TOLERANCE,
        "p45.vertical_center should equal p270: p45={}, p270={}",
        p45_cy, p270_cy,
    );
    assert!(
        (p135_cy - p270_cy).abs() < TOLERANCE,
        "p135.vertical_center should equal p270: p135={}, p270={}",
        p135_cy, p270_cy,
    );

    // Row 2 should be below Row 1: constrain p270.top = p0.bottom + 140
    let p0_bottom = {
        let elem = result.elements.get("p0").unwrap();
        elem.bounds.y + elem.bounds.height
    };
    let p270_top = result.elements.get("p270").unwrap().bounds.y;
    assert!(
        (p270_top - (p0_bottom + 140.0)).abs() < TOLERANCE,
        "p270.top should be p0.bottom + 140: p270.top={}, expected={}",
        p270_top, p0_bottom + 140.0,
    );
}

/// Load the feedback-loops.ail example and verify:
/// 1. Row alignment: all boxes in human_loop at the same Y
/// 2. Row alignment: all boxes in agent_loop at the same Y
/// 3. Cross-row constraint: assign.left = task.left
#[test]
fn test_feedback_loops_row_alignment_and_constraints() {
    let source = std::fs::read_to_string("examples/feedback-loops.ail")
        .expect("feedback-loops.ail should exist");

    let result = compute_layout(&source).expect("layout should succeed");

    // Human loop row: assign, tune, spot, evaluate should share Y
    let (_, assign_y, _, _) = get_bounds(&result, "assign");
    let (_, tune_y, _, _) = get_bounds(&result, "tune");
    let (_, spot_y, _, _) = get_bounds(&result, "spot");
    let (_, evaluate_y, _, _) = get_bounds(&result, "evaluate");
    assert!(
        (assign_y - tune_y).abs() < TOLERANCE
            && (tune_y - spot_y).abs() < TOLERANCE
            && (spot_y - evaluate_y).abs() < TOLERANCE,
        "human_loop items should share Y: assign={}, tune={}, spot={}, evaluate={}",
        assign_y, tune_y, spot_y, evaluate_y,
    );

    // Agent loop row: task, execute, check, result should share Y
    let (_, task_y, _, _) = get_bounds(&result, "task");
    let (_, execute_y, _, _) = get_bounds(&result, "execute");
    let (_, check_y, _, _) = get_bounds(&result, "check");
    let (_, result_y, _, _) = get_bounds(&result, "result");
    assert!(
        (task_y - execute_y).abs() < TOLERANCE
            && (execute_y - check_y).abs() < TOLERANCE
            && (check_y - result_y).abs() < TOLERANCE,
        "agent_loop items should share Y: task={}, execute={}, check={}, result={}",
        task_y, execute_y, check_y, result_y,
    );

    // Cross-row constraint: constrain assign.left = task.left
    let (assign_x, _, _, _) = get_bounds(&result, "assign");
    let (task_x, _, _, _) = get_bounds(&result, "task");
    assert!(
        (assign_x - task_x).abs() < TOLERANCE,
        "assign.left should equal task.left: assign={}, task={}",
        assign_x, task_x,
    );

    // Agent loop should be below human loop
    assert!(
        task_y > assign_y + 40.0,
        "agent_loop should be below human_loop: task.y={}, assign.y={}",
        task_y, assign_y,
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
