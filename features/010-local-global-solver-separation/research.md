# Research: Local/Global Constraint Solver Separation

## Investigation: Path Vertex References

**Question**: Can external constraints reference path vertices (e.g., `mypath.vertex_name.x`)?

**Finding**: No, path vertices cannot be referenced in external constraints.

**Evidence**:
- `PropertyRef` in `src/parser/ast.rs:493` references elements via `ElementPath`, not path vertices
- Path vertices are internal to `PathCommand` declarations and only used during path rendering (`src/renderer/path.rs`)
- The constraint system uses `LayoutVariable` which maps `(element_id, property)` pairs
- Vertex names exist only within the path's local namespace for internal references (e.g., `curve_to via: ctrl_vertex`)

**Implication**: No special handling needed for path vertices in the rotation transformation. Paths as whole elements will have their bounds rotated like any other shape.

---

## Architecture Analysis

### Current Two-Phase Implementation (Hack-Based)

**Location**: `src/layout/engine.rs:1428-1589` (`resolve_constrain_statements()`)

**Pass 1 - Internal Constraints** (lines 1461-1497):
- Uses `is_internal_constraint()` with prefix-based detection
- Separate solver per element's position variables
- Only applies X/Y changes

**Pass 2 - External Constraints** (lines 1499-1582):
- Remaining constraints between different template instances
- Per-property strength management
- Selective solution application

### The Prefix Hack to Remove

**Location**: `src/layout/engine.rs:1767-1779`

```rust
fn elements_share_parent_prefix(a: &str, b: &str) -> bool {
    fn get_prefix(s: &str) -> Option<&str> {
        s.find('_').map(|idx| &s[..idx])
    }
    match (get_prefix(a), get_prefix(b)) {
        (Some(prefix_a), Some(prefix_b)) => prefix_a == prefix_b,
        _ => false,
    }
}
```

**Problems**:
- Relies on naming convention (first underscore = prefix boundary)
- Doesn't handle nesting depth
- Confuses "siblings" with "same template instance"

### Rotation Application (Current)

**Location**: `src/renderer/svg.rs:625-639`

Rotation is SVG transform only - layout operates on pre-rotation coordinates. This causes anchors, constraints, and via points to reference wrong positions.

---

## Key Data Structures

### Constraint System
- `LayoutVariable`: `(element_id: String, property: LayoutProperty)`
- `LayoutConstraint`: Fixed, Suggested, Equal, GreaterOrEqual, LessOrEqual, Midpoint
- `ConstraintOrigin`: UserDefined, LayoutContainer, Intrinsic
- `ConstraintSolver`: Wraps Cassowary (kasuari crate)

### Layout Results
- `ElementLayout`: bounds, styles (including rotation), children, anchors
- `AnchorSet`: collection of `Anchor` (name, position, direction)
- `BoundingBox`: x, y, width, height with computed properties

### Template Resolution
- `ResolutionContext`: tracks name_prefix for hierarchical element IDs
- Elements get prefixed: `{template_instance}_{element_name}`

---

## Transformation Mathematics

### Rotation Matrix (2D)
For angle θ (degrees, clockwise positive per SVG convention):
```
x' = cx + (x - cx) * cos(θ) + (y - cy) * sin(θ)
y' = cy - (x - cx) * sin(θ) + (y - cy) * cos(θ)
```
Where (cx, cy) is the rotation center.

### Anchor Direction Rotation
Cardinal directions map to angles:
- Left: 180°, Right: 0°, Up: -90°, Down: 90°

After rotation by θ:
- New angle = original_angle + θ
- Convert back to nearest cardinal or keep as `Angle(degrees)`

### Loose Bounds Algorithm
1. Get unrotated AABB corners: (x, y), (x+w, y), (x, y+h), (x+w, y+h)
2. Rotate each corner around center
3. Find min/max of rotated corners
4. New AABB: (min_x, min_y, max_x - min_x, max_y - min_y)

---

## Test Strategy

### Regression Testing
- Capture SVG output of all `/examples/*.ail` before refactor
- After refactor, compare byte-for-byte
- Any difference = regression (for non-rotated diagrams)

### New Rotation Tests
1. Single rotated template with connection to anchor
2. External constraint to rotated child element
3. Via point through rotated element center
4. Multiple rotation angles (0°, 45°, 90°, 180°, 270°)
5. Constraint between two rotated templates
