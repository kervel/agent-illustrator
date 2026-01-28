# Tasks: Local/Global Constraint Solver Separation

## Feature Overview
Refactor the constraint solver from a single-pass global system to a two-phase architecture enabling proper template rotation support.

---

## Phase 1: Setup & Regression Baseline

### [X] T001: Create SVG Regression Test Infrastructure
**File**: `tests/svg_regression.rs` (new)
**Goal**: Establish safety net to detect regressions in non-rotated diagrams

Create a test module that:
1. Reads all `.ail` files from `/examples/` directory
2. Renders each to SVG using `render_with_config()`
3. Compares against baseline files in `tests/baseline/`
4. Fails if any byte differs (for non-rotated diagrams)

```rust
#[test]
fn test_svg_regression_all_examples() {
    let examples_dir = Path::new("examples");
    let baseline_dir = Path::new("tests/baseline");

    for entry in fs::read_dir(examples_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension() == Some("ail".as_ref()) {
            let source = fs::read_to_string(&path).unwrap();
            let svg = render(&source).unwrap();

            let baseline_path = baseline_dir.join(path.file_stem().unwrap()).with_extension("svg");
            if baseline_path.exists() {
                let expected = fs::read_to_string(&baseline_path).unwrap();
                assert_eq!(svg, expected, "Regression in {}", path.display());
            }
        }
    }
}
```

**Acceptance**: Test compiles and can be run (will skip if no baselines exist yet)

---

### [X] T002: Capture Baseline SVG Files
**File**: `tests/baseline/` (new directory)
**Goal**: Capture current SVG output as baseline before refactoring

1. Create `tests/baseline/` directory
2. Run each example through the renderer
3. Save SVG output to `tests/baseline/{example_name}.svg`

This can be done via a helper binary or test with `REGENERATE_BASELINE=1` env var:
```rust
#[test]
#[ignore] // Run with: cargo test -- --ignored generate_baselines
fn generate_baselines() {
    let examples_dir = Path::new("examples");
    let baseline_dir = Path::new("tests/baseline");
    fs::create_dir_all(baseline_dir).unwrap();

    for entry in fs::read_dir(examples_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension() == Some("ail".as_ref()) {
            let source = fs::read_to_string(&path).unwrap();
            let svg = render(&source).unwrap();
            let baseline_path = baseline_dir.join(path.file_stem().unwrap()).with_extension("svg");
            fs::write(&baseline_path, svg).unwrap();
        }
    }
}
```

**Acceptance**: Baseline SVG files exist for all examples in `/examples/`

---

## Phase 2: Rotation Transformation Module [P]

### [X] T003: Create transform.rs Module Structure
**File**: `src/layout/transform.rs` (new)
**Goal**: Create the rotation transformation module with type definitions

```rust
//! Rotation transformation utilities for the two-phase constraint solver.
//!
//! This module handles transforming element bounds and anchors after local
//! constraint solving, applying rotation around the template center.

use crate::layout::types::{Anchor, AnchorDirection, AnchorSet, BoundingBox, Point};

/// Represents a 2D rotation transformation around a center point.
#[derive(Debug, Clone, Copy)]
pub struct RotationTransform {
    /// Rotation angle in degrees (clockwise positive, per SVG convention)
    pub angle_degrees: f64,
    /// Center point of rotation
    pub center: Point,
}

impl RotationTransform {
    /// Create a new rotation transform
    pub fn new(angle_degrees: f64, center: Point) -> Self {
        Self { angle_degrees, center }
    }

    /// Check if this is effectively a no-op (0° rotation)
    pub fn is_identity(&self) -> bool {
        self.angle_degrees.abs() < f64::EPSILON
    }
}
```

**Acceptance**: Module compiles, exports `RotationTransform` struct

---

### [X] T004: Implement Point Rotation [P]
**File**: `src/layout/transform.rs`
**Goal**: Implement 2D point rotation around center

Add to `RotationTransform` impl:
```rust
/// Rotate a point around the center using standard 2D rotation matrix.
/// Uses SVG convention: clockwise positive angles.
pub fn transform_point(&self, point: Point) -> Point {
    if self.is_identity() {
        return point;
    }

    let radians = self.angle_degrees.to_radians();
    let cos_a = radians.cos();
    let sin_a = radians.sin();

    let dx = point.x - self.center.x;
    let dy = point.y - self.center.y;

    // SVG uses clockwise positive, so rotation matrix is:
    // [cos  sin] [dx]   Note: +sin for clockwise
    // [-sin cos] [dy]
    Point {
        x: self.center.x + dx * cos_a + dy * sin_a,
        y: self.center.y - dx * sin_a + dy * cos_a,
    }
}
```

**Acceptance**: Unit tests pass for 0°, 90°, 180°, 270°, 45° rotations

---

### [X] T005: Implement Loose Bounds Transformation [P]
**File**: `src/layout/transform.rs`
**Goal**: Implement axis-aligned bounding box transformation using loose bounds algorithm

```rust
/// Transform a bounding box using the "loose bounds" algorithm.
///
/// Rather than computing mathematically tight bounds (complex for curves),
/// we rotate the 4 corners of the original AABB and take the AABB of
/// those rotated corners. This matches CSS/SVG transform behavior.
pub fn transform_bounds(&self, bounds: &BoundingBox) -> BoundingBox {
    if self.is_identity() {
        return bounds.clone();
    }

    // Get four corners of the original AABB
    let corners = [
        Point { x: bounds.x, y: bounds.y },
        Point { x: bounds.x + bounds.width, y: bounds.y },
        Point { x: bounds.x, y: bounds.y + bounds.height },
        Point { x: bounds.x + bounds.width, y: bounds.y + bounds.height },
    ];

    // Rotate all corners
    let rotated: Vec<Point> = corners.iter()
        .map(|p| self.transform_point(*p))
        .collect();

    // Find AABB of rotated corners
    let min_x = rotated.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
    let max_x = rotated.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
    let min_y = rotated.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
    let max_y = rotated.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);

    BoundingBox {
        x: min_x,
        y: min_y,
        width: max_x - min_x,
        height: max_y - min_y,
    }
}
```

**Acceptance**: Unit tests verify loose bounds for various angles (90° swaps width/height, 45° expands box)

---

### [X] T006: Implement Anchor Direction Transformation [P]
**File**: `src/layout/types.rs`
**Goal**: Add `from_degrees()` method to AnchorDirection for direction rotation

```rust
impl AnchorDirection {
    /// Create an AnchorDirection from an angle in degrees.
    /// Snaps to cardinal directions if within 1 degree tolerance.
    pub fn from_degrees(degrees: f64) -> Self {
        // Normalize to 0-360
        let normalized = ((degrees % 360.0) + 360.0) % 360.0;

        // Snap to cardinal if close (within 1 degree)
        if (normalized - 0.0).abs() < 1.0 || (normalized - 360.0).abs() < 1.0 {
            AnchorDirection::Right
        } else if (normalized - 90.0).abs() < 1.0 {
            AnchorDirection::Down
        } else if (normalized - 180.0).abs() < 1.0 {
            AnchorDirection::Left
        } else if (normalized - 270.0).abs() < 1.0 {
            AnchorDirection::Up
        } else {
            AnchorDirection::Angle(normalized)
        }
    }
}
```

**Acceptance**: `from_degrees(90.0)` returns `Down`, `from_degrees(45.0)` returns `Angle(45.0)`

---

### [X] T007: Implement Anchor Transformation [P]
**File**: `src/layout/transform.rs`
**Goal**: Implement full anchor transformation (position + direction)

```rust
/// Transform an anchor's position and direction.
/// Position is rotated around the center, direction angle is adjusted.
pub fn transform_anchor(&self, anchor: &Anchor) -> Anchor {
    if self.is_identity() {
        return anchor.clone();
    }

    Anchor {
        name: anchor.name.clone(),
        position: self.transform_point(anchor.position),
        direction: self.transform_direction(anchor.direction),
    }
}

fn transform_direction(&self, dir: AnchorDirection) -> AnchorDirection {
    let original_angle = dir.to_degrees();
    let new_angle = original_angle + self.angle_degrees;
    AnchorDirection::from_degrees(new_angle)
}
```

**Acceptance**: Anchor at `direction: Left` rotated 90° becomes `direction: Up`

---

### [X] T008: Implement AnchorSet Transformation [P]
**File**: `src/layout/types.rs`
**Goal**: Add `transform()` method to AnchorSet

```rust
impl AnchorSet {
    /// Transform all anchors in this set using the given rotation.
    pub fn transform(&self, rotation: &RotationTransform) -> AnchorSet {
        AnchorSet {
            anchors: self.anchors.iter()
                .map(|(name, anchor)| (name.clone(), rotation.transform_anchor(anchor)))
                .collect(),
        }
    }
}
```

**Acceptance**: AnchorSet with 4 cardinal anchors transforms correctly under 90° rotation

---

### [X] T009: Export Transform Module
**File**: `src/layout/mod.rs`
**Goal**: Export the transform module from the layout crate

Add:
```rust
pub mod transform;
pub use transform::RotationTransform;
```

**Acceptance**: `use crate::layout::RotationTransform;` works from other modules

---

### [X] T010: Add Transform Unit Tests
**File**: `src/layout/transform.rs`
**Goal**: Comprehensive unit tests for transformation logic

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_rotation() {
        let t = RotationTransform::new(0.0, Point { x: 50.0, y: 50.0 });
        let p = Point { x: 100.0, y: 0.0 };
        let result = t.transform_point(p);
        assert!((result.x - p.x).abs() < 0.001);
        assert!((result.y - p.y).abs() < 0.001);
    }

    #[test]
    fn test_90_degree_rotation() {
        let t = RotationTransform::new(90.0, Point { x: 0.0, y: 0.0 });
        let p = Point { x: 1.0, y: 0.0 };
        let result = t.transform_point(p);
        assert!((result.x - 0.0).abs() < 0.001);
        assert!((result.y - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_loose_bounds_90_degrees() {
        // 100x50 box rotated 90° becomes 50x100
        let t = RotationTransform::new(90.0, Point { x: 50.0, y: 25.0 });
        let bounds = BoundingBox { x: 0.0, y: 0.0, width: 100.0, height: 50.0 };
        let result = t.transform_bounds(&bounds);
        assert!((result.width - 50.0).abs() < 0.001);
        assert!((result.height - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_direction_rotation() {
        assert!(matches!(AnchorDirection::from_degrees(0.0), AnchorDirection::Right));
        assert!(matches!(AnchorDirection::from_degrees(90.0), AnchorDirection::Down));
        assert!(matches!(AnchorDirection::from_degrees(180.0), AnchorDirection::Left));
        assert!(matches!(AnchorDirection::from_degrees(270.0), AnchorDirection::Up));
        assert!(matches!(AnchorDirection::from_degrees(45.0), AnchorDirection::Angle(_)));
    }
}
```

**Acceptance**: All unit tests pass

---

## Phase 3: Constraint Partitioning Infrastructure

### [X] T011: Add Template Instance Tracking to ConstraintSource
**File**: `src/layout/types.rs` (or `src/layout/solver.rs`)
**Goal**: Extend ConstraintSource to track template instance origin

Locate the `ConstraintSource` struct and add:
```rust
pub struct ConstraintSource {
    pub origin: ConstraintOrigin,
    pub template_instance: Option<String>,  // NEW: None for top-level, Some("r1") for template
    pub description: String,
}
```

Update any constructors/builders for ConstraintSource to accept the new field (default to `None` for backward compatibility).

**Acceptance**: Code compiles with new field, existing tests pass

---

### [X] T012: Add ConstraintScope Enum
**File**: `src/layout/engine.rs`
**Goal**: Define clean abstraction for constraint classification

```rust
/// Classification of a constraint's scope for two-phase solving.
#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintScope {
    /// Constraint is internal to a specific template instance
    Local(String),
    /// Constraint spans multiple templates or involves top-level elements
    Global,
}
```

**Acceptance**: Enum defined and usable

---

### [X] T013: Implement classify_constraint()
**File**: `src/layout/engine.rs`
**Goal**: Replace prefix-based detection with clean classification

```rust
/// Classify a constraint as Local (within one template) or Global (cross-template).
fn classify_constraint(&self, constraint: &LayoutConstraint) -> ConstraintScope {
    // Get template instances for all variables in this constraint
    let instances: HashSet<Option<&str>> = self.get_constraint_template_instances(constraint);

    // If all variables belong to the same template instance, it's local
    if instances.len() == 1 {
        if let Some(Some(instance)) = instances.iter().next() {
            return ConstraintScope::Local(instance.to_string());
        }
    }

    // Mixed templates or top-level = global
    ConstraintScope::Global
}

fn get_constraint_template_instances(&self, constraint: &LayoutConstraint) -> HashSet<Option<&str>> {
    // Extract element IDs from constraint, look up their template instance
    // This requires tracking template membership during constraint collection
    todo!("Implement based on constraint structure")
}
```

**Acceptance**: Method correctly classifies local vs global constraints

---

### [X] T014: Track Template Instance During Constraint Collection
**File**: `src/layout/collector.rs`
**Goal**: Set template_instance field when collecting constraints

When visiting template children, pass the template instance name through:
- Constraints between `r1_body` and `r1_pin` should get `template_instance: Some("r1")`
- Constraints involving top-level elements get `template_instance: None`

This may require threading context through the collector or post-processing based on element prefixes.

**Acceptance**: Constraints have correct `template_instance` values set

---

### [X] T015: Add LocalSolverResult Type
**File**: `src/layout/types.rs`
**Goal**: Define result type for local solving phase

```rust
use std::collections::HashMap;

/// Result of solving constraints within a single template instance.
#[derive(Debug, Clone)]
pub struct LocalSolverResult {
    /// The template instance name (e.g., "r1", "resistor_1")
    pub template_instance: String,
    /// Solved bounds for each child element
    pub element_bounds: HashMap<String, BoundingBox>,
    /// Anchors for each child element
    pub anchors: HashMap<String, AnchorSet>,
    /// Rotation angle if this template is rotated
    pub rotation: Option<f64>,
}
```

**Acceptance**: Type defined and usable

---

## Phase 4: Refactor Two-Phase Solver

### [X] T016: Implement partition_constraints()
**File**: `src/layout/engine.rs`
**Goal**: Partition constraints into local (per-template) and global sets

```rust
fn partition_constraints(&self, constraints: &[LayoutConstraint])
    -> (HashMap<String, Vec<LayoutConstraint>>, Vec<LayoutConstraint>)
{
    let mut local_by_instance: HashMap<String, Vec<LayoutConstraint>> = HashMap::new();
    let mut global: Vec<LayoutConstraint> = Vec::new();

    for constraint in constraints {
        match self.classify_constraint(constraint) {
            ConstraintScope::Local(instance) => {
                local_by_instance.entry(instance)
                    .or_default()
                    .push(constraint.clone());
            }
            ConstraintScope::Global => {
                global.push(constraint.clone());
            }
        }
    }

    (local_by_instance, global)
}
```

**Acceptance**: Constraints correctly partitioned, local constraints grouped by template

---

### [X] T017: Implement solve_local()
**File**: `src/layout/engine.rs`
**Goal**: Solve constraints for a single template instance in isolation

```rust
fn solve_local(&self, instance: &str, constraints: Vec<LayoutConstraint>)
    -> Result<LocalSolverResult, LayoutError>
{
    let mut solver = ConstraintSolver::new();

    // Get all elements belonging to this template instance
    let elements = self.get_template_elements(instance);

    // Add current bounds as suggestions
    for element in &elements {
        self.add_element_suggestions(&mut solver, element)?;
    }

    // Add local constraints
    for constraint in &constraints {
        if let Err(e) = solver.add_constraint(constraint.clone()) {
            return Err(LayoutError::UnsolvableConstraint {
                template: instance.to_string(),
                details: e.to_string(),
            });
        }
    }

    let solution = solver.solve()?;

    // Extract results
    Ok(LocalSolverResult {
        template_instance: instance.to_string(),
        element_bounds: self.extract_bounds_from_solution(&solution, instance),
        anchors: self.extract_anchors_for_instance(instance),
        rotation: self.get_template_rotation(instance),
    })
}
```

**Acceptance**: Local solving works in isolation, fail-fast on unsolvable constraints

---

### [X] T018: Implement apply_rotation()
**File**: `src/layout/engine.rs`
**Goal**: Apply rotation transformation to local solver results

```rust
fn apply_rotation(&self, result: &mut LocalSolverResult, angle: f64)
    -> Result<(), LayoutError>
{
    // Compute rotation center from combined child bounds
    let center = self.compute_template_center(&result.element_bounds);

    let transform = RotationTransform::new(angle, center);

    // Transform all element bounds
    for bounds in result.element_bounds.values_mut() {
        *bounds = transform.transform_bounds(bounds);
    }

    // Transform all anchors
    for anchor_set in result.anchors.values_mut() {
        *anchor_set = anchor_set.transform(&transform);
    }

    Ok(())
}

fn compute_template_center(&self, bounds_map: &HashMap<String, BoundingBox>) -> Point {
    // Compute combined AABB of all children
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for bounds in bounds_map.values() {
        min_x = min_x.min(bounds.x);
        min_y = min_y.min(bounds.y);
        max_x = max_x.max(bounds.x + bounds.width);
        max_y = max_y.max(bounds.y + bounds.height);
    }

    Point {
        x: (min_x + max_x) / 2.0,
        y: (min_y + max_y) / 2.0,
    }
}
```

**Acceptance**: Rotated template bounds and anchors are transformed correctly

---

### [X] T019: Implement apply_local_results()
**File**: `src/layout/engine.rs`
**Goal**: Apply local solver results back to the main layout

```rust
fn apply_local_results(&mut self, results: &HashMap<String, LocalSolverResult>)
    -> Result<(), LayoutError>
{
    for (instance, result) in results {
        for (element_id, bounds) in &result.element_bounds {
            if let Some(element) = self.result.elements.get_mut(element_id) {
                element.bounds = bounds.clone();
            }
        }

        for (element_id, anchor_set) in &result.anchors {
            if let Some(element) = self.result.elements.get_mut(element_id) {
                element.anchors = anchor_set.clone();
            }
        }
    }

    Ok(())
}
```

**Acceptance**: Layout reflects local solving results

---

### [X] T020: Implement solve_global()
**File**: `src/layout/engine.rs`
**Goal**: Solve global constraints using post-rotation bounds

```rust
fn solve_global(&mut self, constraints: &[LayoutConstraint]) -> Result<(), LayoutError> {
    if constraints.is_empty() {
        return Ok(());
    }

    let mut solver = ConstraintSolver::new();

    // Add all element suggestions (now with rotated bounds)
    for element in self.result.elements.values() {
        self.add_element_suggestions(&mut solver, element)?;
    }

    // Add global constraints
    for constraint in constraints {
        solver.add_constraint(constraint.clone())?;
    }

    let solution = solver.solve()?;

    // Apply solution to affected elements
    self.apply_solution(&solution)?;

    Ok(())
}
```

**Acceptance**: Global constraints correctly position templates relative to each other

---

### [X] T021: Refactor resolve_constrain_statements() Main Function
**File**: `src/layout/engine.rs`
**Goal**: Replace current implementation with clean two-phase architecture

Replace the existing `resolve_constrain_statements()` with:

```rust
pub fn resolve_constrain_statements(&mut self, constraints: &[LayoutConstraint]) -> Result<(), LayoutError> {
    // Phase 1: Partition constraints by template instance
    let (local_by_instance, global) = self.partition_constraints(constraints);

    // Phase 2: Solve each template locally
    let mut local_results: HashMap<String, LocalSolverResult> = HashMap::new();
    for (instance, local_constraints) in local_by_instance {
        let result = self.solve_local(&instance, local_constraints)?;
        local_results.insert(instance, result);
    }

    // Phase 3: Apply rotation transformations
    for result in local_results.values_mut() {
        if let Some(angle) = result.rotation {
            if angle.abs() > f64::EPSILON {
                self.apply_rotation(result, angle)?;
            }
        }
    }

    // Phase 4: Apply local results to layout
    self.apply_local_results(&local_results)?;

    // Phase 5: Solve global constraints
    self.solve_global(&global)?;

    // Phase 6: Recompute anchors from final bounds
    self.recompute_all_anchors()?;

    Ok(())
}
```

**Acceptance**: Two-phase solving works end-to-end, existing tests pass

---

## Phase 5: Remove Prefix Hack

### [X] T022: Remove elements_share_parent_prefix()
**File**: `src/layout/engine.rs`
**Goal**: Delete the deprecated prefix-based hack

Remove:
- `fn elements_share_parent_prefix(a: &str, b: &str) -> bool` (lines ~1767-1779)
- `fn is_internal_constraint(...)` (lines ~1745-1763)

**Acceptance**: Functions removed, code compiles

---

### [X] T023: Remove Any Remaining Prefix Hack Callers
**File**: `src/layout/engine.rs`
**Goal**: Ensure no code still references the removed functions

Search for and remove any remaining calls to:
- `elements_share_parent_prefix`
- `is_internal_constraint`

Replace with calls to `classify_constraint()` where needed.

**Acceptance**: No references to removed functions, all tests pass

---

## Phase 6: Integration Testing

### [X] T024: Create Rotation Integration Test File
**File**: `tests/rotation_integration.rs` (new)
**Goal**: Set up integration test infrastructure for rotation scenarios

```rust
use agent_illustrator::render;

fn render_and_get_element_bounds(source: &str, element_id: &str) -> Option<(f64, f64, f64, f64)> {
    // Helper to render and extract bounds for testing
    todo!()
}

fn render_and_get_anchor_position(source: &str, element_id: &str, anchor_name: &str) -> Option<(f64, f64)> {
    // Helper to render and extract anchor position
    todo!()
}
```

**Acceptance**: Test file compiles with helper functions

---

### [X] T025: Test Rotated Template Anchor Positions
**File**: `tests/rotation_integration.rs`
**Goal**: Verify anchors transform correctly under rotation

```rust
#[test]
fn test_rotated_template_anchor_position() {
    let source = r#"
        template "box" {
            rect body [width: 40, height: 20]
            anchor left_pin [position: body.left, direction: left]
            anchor right_pin [position: body.right, direction: right]
        }

        box b1 [rotation: 90]
    "#;

    // After 90° rotation:
    // - Original left anchor (pointing left) should now point up
    // - Positions should be rotated around center
    let svg = render(source).unwrap();

    // Verify anchor positions and directions are transformed
    // (Check connection attachment points in SVG)
}
```

**Acceptance**: Test passes, anchors at correct rotated positions

---

### [X] T026: Test External Constraint to Rotated Child
**File**: `tests/rotation_integration.rs`
**Goal**: Verify constraints use post-rotation bounding boxes

```rust
#[test]
fn test_external_constraint_to_rotated_child() {
    let source = r#"
        template "component" {
            rect body [width: 80, height: 40]
            export body
        }

        component c1 [rotation: 90]
        rect label [width: 30, height: 20]

        constrain label.left = c1_body.right + 10
    "#;

    // After 90° rotation, 80x40 becomes 40x80
    // label.left should be 10px right of the rotated bounding box's right edge
    let svg = render(source).unwrap();

    // Verify label position respects rotated bounds
}
```

**Acceptance**: Test passes, constraint uses rotated bounding box

---

### [X] T027: Test Connection to Rotated Anchor
**File**: `tests/rotation_integration.rs`
**Goal**: Verify connections attach at rotated anchor positions

```rust
#[test]
fn test_connection_to_rotated_anchor() {
    let source = r#"
        template "resistor" {
            rect body [width: 40, height: 16]
            anchor left_conn [position: body.left, direction: left]
            anchor right_conn [position: body.right, direction: right]
        }

        rect source [width: 20, height: 20]
        resistor r1 [rotation: 90]

        source.right -> r1.left_conn
    "#;

    // Connection should attach to the rotated position of left_conn
    let svg = render(source).unwrap();

    // Verify connection endpoint matches rotated anchor position
}
```

**Acceptance**: Test passes, connection attaches at correct rotated position

---

### [X] T028: Test Via Point Through Rotated Element
**File**: `tests/rotation_integration.rs`
**Goal**: Verify curved routing uses rotated element centers

```rust
#[test]
fn test_via_point_through_rotated_element() {
    let source = r#"
        template "waypoint" {
            circle marker [size: 6]
        }

        rect start [width: 20, height: 20]
        rect end [width: 20, height: 20]
        waypoint ctrl [rotation: 45]

        constrain end.left = start.right + 100
        constrain ctrl.center_x = start.center_x + 50
        constrain ctrl.center_y = start.center_y - 30

        start -> end [routing: curved, via: ctrl_marker]
    "#;

    // Curve should pass through rotated position of ctrl_marker
    let svg = render(source).unwrap();

    // Verify curve control point is at rotated marker center
}
```

**Acceptance**: Test passes, curve routes through rotated via point

---

### [X] T029: Run Full Regression Suite
**Goal**: Verify all examples produce byte-identical SVG output

```bash
cargo test test_svg_regression_all_examples
```

**Acceptance**: All baseline comparisons pass (0 regressions)

---

## Phase 7: Documentation & Cleanup

### [X] T030: Add Module Documentation
**File**: `src/layout/transform.rs`
**Goal**: Document the transformation module's purpose and usage

Add comprehensive module-level documentation explaining:
- The two-phase solver architecture
- Why loose bounds are used
- The rotation convention (clockwise positive)
- How anchors are transformed

**Acceptance**: `cargo doc` generates clear documentation

---

### [X] T031: Update Engine Module Documentation
**File**: `src/layout/engine.rs`
**Goal**: Document the refactored constraint solving architecture

Add comments to `resolve_constrain_statements()` explaining:
- Phase 1: Partition constraints
- Phase 2: Local solving per template
- Phase 3: Rotation transformation
- Phase 4: Apply local results
- Phase 5: Global solving
- Phase 6: Anchor recomputation

**Acceptance**: Code is well-documented for future maintainers

---

## Summary

| Phase | Tasks | Description |
|-------|-------|-------------|
| 1: Setup | T001-T002 | Regression baseline |
| 2: Transform | T003-T010 | Rotation transformation module [P] |
| 3: Partition | T011-T015 | Constraint classification infrastructure |
| 4: Solver | T016-T021 | Two-phase solver refactor |
| 5: Cleanup | T022-T023 | Remove prefix hack |
| 6: Testing | T024-T029 | Integration tests |
| 7: Docs | T030-T031 | Documentation |

**Total Tasks**: 31
**Parallelizable**: T003-T010 (transform module tasks can largely run in parallel)

---

## Dependencies

```
T001 ← T002 (baseline infrastructure before capture)
T003 ← T004, T005, T006, T007, T008 (module structure before implementations)
T009 ← T003-T008 (export after implementations)
T010 ← T004-T008 (tests after implementations)
T011 ← T012 (type before enum)
T012 ← T013 (enum before classification)
T013 ← T014 (classification before collection tracking)
T015 ← T016-T021 (result type before solver refactor)
T016-T021 are sequential (solver refactor phases)
T021 ← T022-T023 (refactor complete before hack removal)
T024 ← T025-T028 (test infrastructure before tests)
T029 ← T001, T002, T021 (regression after baseline and refactor)
T030-T031 can run after T021
```

---

## Checkpoint

After T029, verify:
- [ ] All existing tests pass
- [ ] All examples produce byte-identical SVG (regression test)
- [ ] New rotation tests pass
- [ ] `elements_share_parent_prefix()` is removed
- [ ] Code compiles without warnings
