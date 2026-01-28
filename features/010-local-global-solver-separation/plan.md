# Implementation Plan: Local/Global Constraint Solver Separation

## Technical Context

- **Language**: Rust
- **Key Dependencies**: kasuari (Cassowary constraint solver)
- **Primary Files**:
  - `src/layout/engine.rs` - Main layout computation, two-phase solving
  - `src/layout/solver.rs` - Constraint solver wrapper
  - `src/layout/types.rs` - LayoutResult, ElementLayout, AnchorSet
  - `src/layout/collector.rs` - Constraint collection from AST
  - `src/renderer/svg.rs` - SVG rendering with rotation transforms
  - `src/template/resolver.rs` - Template expansion and prefixing

## Constitution Check

- [x] No new dependencies required (uses existing kasuari)
- [x] Maintains backward compatibility (FR8)
- [x] Follows existing code patterns (extends two-phase approach)
- [x] Error handling: fail-fast for unsolvable constraints

---

## Phase 1: Preparation & Regression Baseline

### 1.1 Create SVG Regression Test Infrastructure

Add a test that captures current SVG output for all examples to detect regressions.

**File**: `tests/svg_regression.rs` (new)

```rust
// Capture baseline SVG for all examples
// Compare after refactor to ensure byte-identical output for non-rotated diagrams
```

### 1.2 Run Baseline Capture

Execute tests to create baseline `.svg` files in `tests/baseline/` directory.

---

## Phase 2: Constraint Partitioning Infrastructure

### 2.1 Add Template Origin Tracking to Constraints

**File**: `src/layout/types.rs`

Add a new field to track which template instance a constraint belongs to:

```rust
pub struct ConstraintSource {
    pub origin: ConstraintOrigin,
    pub template_instance: Option<String>,  // NEW: None for top-level, Some("r1") for template
    pub description: String,
}
```

### 2.2 Extend Constraint Collection

**File**: `src/layout/collector.rs`

During constraint collection, track the template instance context:

- When collecting constraints from template children, set `template_instance`
- Top-level elements get `template_instance: None`
- Constraints between elements with same `template_instance` are "local"
- Constraints with different/None `template_instance` are "global"

### 2.3 Add Constraint Classification Methods

**File**: `src/layout/engine.rs`

Replace `is_internal_constraint()` and `elements_share_parent_prefix()` with:

```rust
fn classify_constraint(constraint: &LayoutConstraint) -> ConstraintScope {
    match &constraint.source().template_instance {
        Some(instance) => ConstraintScope::Local(instance.clone()),
        None => ConstraintScope::Global,
    }
}

enum ConstraintScope {
    Local(String),  // Template instance name
    Global,
}
```

---

## Phase 3: Rotation Transformation Module

### 3.1 Create Transformation Types

**File**: `src/layout/transform.rs` (new)

```rust
pub struct RotationTransform {
    pub angle_degrees: f64,
    pub center: Point,
}

impl RotationTransform {
    /// Rotate a point around the center
    pub fn transform_point(&self, point: Point) -> Point;

    /// Rotate an anchor (position + direction)
    pub fn transform_anchor(&self, anchor: &Anchor) -> Anchor;

    /// Compute loose bounds after rotation
    pub fn transform_bounds(&self, bounds: &BoundingBox) -> BoundingBox;
}
```

### 3.2 Implement Point Rotation

```rust
pub fn transform_point(&self, point: Point) -> Point {
    let radians = self.angle_degrees.to_radians();
    let cos_a = radians.cos();
    let sin_a = radians.sin();

    let dx = point.x - self.center.x;
    let dy = point.y - self.center.y;

    Point {
        x: self.center.x + dx * cos_a + dy * sin_a,
        y: self.center.y - dx * sin_a + dy * cos_a,
    }
}
```

### 3.3 Implement Anchor Transformation

```rust
pub fn transform_anchor(&self, anchor: &Anchor) -> Anchor {
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

### 3.4 Implement Loose Bounds

```rust
pub fn transform_bounds(&self, bounds: &BoundingBox) -> BoundingBox {
    // Get four corners
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

---

## Phase 4: Refactor Two-Phase Solver

### 4.1 Create LocalSolverResult Type

**File**: `src/layout/types.rs`

```rust
pub struct LocalSolverResult {
    pub template_instance: String,
    pub element_bounds: HashMap<String, BoundingBox>,
    pub anchors: HashMap<String, AnchorSet>,
    pub rotation: Option<f64>,
}
```

### 4.2 Refactor resolve_constrain_statements()

**File**: `src/layout/engine.rs`

Replace the current implementation with clean phases:

```rust
pub fn resolve_constrain_statements(&mut self, constraints: &[LayoutConstraint]) -> Result<(), LayoutError> {
    // Phase 1: Partition constraints by template instance
    let (local_by_instance, global) = self.partition_constraints(constraints);

    // Phase 2: Solve each template locally
    let mut local_results: HashMap<String, LocalSolverResult> = HashMap::new();
    for (instance, local_constraints) in local_by_instance {
        let result = self.solve_local(instance, local_constraints)?;
        local_results.insert(instance, result);
    }

    // Phase 3: Apply rotation transformations
    for (instance, result) in &mut local_results {
        if let Some(angle) = result.rotation {
            self.apply_rotation(instance, result, angle)?;
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

### 4.3 Implement partition_constraints()

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

### 4.4 Implement solve_local()

```rust
fn solve_local(&self, instance: &str, constraints: Vec<LayoutConstraint>)
    -> Result<LocalSolverResult, LayoutError>
{
    let mut solver = ConstraintSolver::new();

    // Add current bounds as suggestions
    for element in self.get_template_elements(instance) {
        solver.add_element_suggestions(&element)?;
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

    // Build result
    Ok(LocalSolverResult {
        template_instance: instance.to_string(),
        element_bounds: self.extract_bounds_from_solution(&solution, instance),
        anchors: self.extract_anchors(instance),
        rotation: self.get_template_rotation(instance),
    })
}
```

### 4.5 Implement apply_rotation()

```rust
fn apply_rotation(&self, instance: &str, result: &mut LocalSolverResult, angle: f64)
    -> Result<(), LayoutError>
{
    // Compute rotation center from child bounds
    let center = self.compute_template_center(instance, &result.element_bounds);

    let transform = RotationTransform {
        angle_degrees: angle,
        center,
    };

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
```

---

## Phase 5: Remove Prefix Hack

### 5.1 Remove Deprecated Functions

**File**: `src/layout/engine.rs`

Delete:
- `elements_share_parent_prefix()` (lines 1767-1779)
- `is_internal_constraint()` (lines 1745-1763)

### 5.2 Update Callers

Replace any remaining calls to use the new `classify_constraint()` method.

---

## Phase 6: Update Anchor System

### 6.1 Add AnchorSet::transform()

**File**: `src/layout/types.rs`

```rust
impl AnchorSet {
    pub fn transform(&self, rotation: &RotationTransform) -> AnchorSet {
        AnchorSet {
            anchors: self.anchors.iter()
                .map(|(name, anchor)| (name.clone(), rotation.transform_anchor(anchor)))
                .collect(),
        }
    }
}
```

### 6.2 Add AnchorDirection::from_degrees()

```rust
impl AnchorDirection {
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

---

## Phase 7: Testing

### 7.1 Add Rotation Unit Tests

**File**: `src/layout/transform.rs`

- Test point rotation at 0°, 90°, 180°, 270°, 45°
- Test bounds transformation (loose bounds)
- Test anchor direction rotation

### 7.2 Add Integration Tests

**File**: `tests/rotation_integration.rs`

```rust
#[test]
fn test_rotated_template_anchor_position() {
    // Template with anchor, rotated 90°
    // Verify anchor position is transformed correctly
}

#[test]
fn test_external_constraint_to_rotated_child() {
    // constrain label.left = rotated_template_body.right + 10
    // Verify constraint uses post-rotation bounding box
}

#[test]
fn test_connection_to_rotated_anchor() {
    // Connection from element to rotated template anchor
    // Verify connection attaches at rotated position
}

#[test]
fn test_via_point_through_rotated_element() {
    // Curve with via: pointing to element in rotated template
    // Verify curve passes through rotated center
}
```

### 7.3 Run Regression Tests

Verify all examples in `/examples/` produce byte-identical SVG output.

---

## Phase 8: Documentation

### 8.1 Update Code Comments

Add documentation to new types and functions explaining the two-phase architecture.

### 8.2 Add Architecture Note

Document the constraint solving phases in `src/layout/mod.rs` or a new `ARCHITECTURE.md`.

---

## Implementation Order

1. **Phase 1**: Regression baseline (safety net)
2. **Phase 3**: Transformation module (independent, testable)
3. **Phase 2**: Constraint partitioning infrastructure (extends existing)
4. **Phase 4**: Refactor two-phase solver (main change)
5. **Phase 5**: Remove prefix hack (cleanup)
6. **Phase 6**: Anchor system updates (integration)
7. **Phase 7**: Testing (validation)
8. **Phase 8**: Documentation (finalization)

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Regression in non-rotated diagrams | Phase 1 baseline captures current behavior |
| Cassowary solver conflicts | Fail-fast error handling per spec |
| Performance degradation | Profile before/after, optimize if needed |
| Edge cases in rotation math | Comprehensive unit tests for transform module |

---

## Files to Create

- `src/layout/transform.rs` - Rotation transformation logic
- `tests/svg_regression.rs` - Baseline comparison tests
- `tests/rotation_integration.rs` - Rotation feature tests
- `tests/baseline/` - Directory for baseline SVG files

## Files to Modify

- `src/layout/mod.rs` - Export transform module
- `src/layout/types.rs` - Add LocalSolverResult, extend ConstraintSource
- `src/layout/collector.rs` - Track template instance in constraints
- `src/layout/engine.rs` - Refactor resolve_constrain_statements(), remove hack
- `src/layout/solver.rs` - Minor updates for error handling
