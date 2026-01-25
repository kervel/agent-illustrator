# Research: Curved Paths and Connectors

## Decision Summary

| Topic | Decision | Rationale |
|-------|----------|-----------|
| SVG Curve Primitive | Quadratic Bezier (Q command) | Maps directly to single-control-point "steering vertex" concept |
| Smooth Joins | Use SVG T command for chained curves | Automatic tangent continuity without explicit computation |
| Via Reference Resolution | Use existing StyleValue::Identifier pattern | Consistent with label reference mechanism |
| Default Control Point | Perpendicular offset at 25% of chord length | Provides gentle, predictable curves |

---

## Research Findings

### 1. SVG Quadratic Bezier Curves

**Decision**: Use SVG `Q` command (quadratic Bezier) for curve rendering.

**Rationale**:
- Single control point per segment maps perfectly to "steering vertex" concept
- Native browser support - no calculation overhead
- Simpler than cubic Bezier (two control points) while sufficient for illustration use cases

**SVG Syntax**:
```
Q cx cy, ex ey   // Quadratic Bezier: control point (cx,cy), endpoint (ex,ey)
T ex ey          // Smooth quadratic: auto-calculates control point for tangent continuity
```

**Alternatives Considered**:
- Cubic Bezier (C command): More control but requires two control points per segment - overkill
- Arcs (A command): Already implemented; less intuitive for general curves
- Catmull-Rom splines: Would require custom implementation; SVG doesn't support natively

---

### 2. Chained Curve Segments (Multiple Via Points)

**Decision**: Use SVG `T` command for smooth continuation between chained quadratic segments.

**Rationale**:
- `T` automatically mirrors the previous control point about the current endpoint
- Guarantees G1 continuity (smooth tangent) at join points
- Zero additional computation needed - browser handles it

**Pattern for `[via: v1, v2, v3]`**:
```
M start.x start.y          // Move to start
Q v1.x v1.y, junction1     // First segment to intermediate point
T junction2                 // Smooth continuation through junction2
T end.x end.y              // Smooth continuation to end
```

**Alternative Considered**:
- Explicit control point calculation for each segment: More complex, same result

---

### 3. Reference Resolution Pattern

**Decision**: Use existing `StyleValue::Identifier` pattern from label references.

**Rationale**: The codebase already has a proven pattern for resolving element references:

```rust
// Existing pattern in routing.rs (lines 371-406)
match &modifier.node.value.node {
    StyleValue::Identifier(id) => {
        result.get_element_by_name(&id.0).map(|element| {
            // Get element's center point
            element.bounds.center()
        })
    }
    _ => None,
}
```

**For `[via: control_vertex]`**:
- Parse as `StyleValue::Identifier(id)`
- Resolve in layout phase via `result.get_element_by_name(&id.0)`
- Extract center point from resolved element's `BoundingBox`
- Error if element not found (compile error per spec)

---

### 4. Default Control Point Calculation

**Decision**: Perpendicular offset at 25% of chord length.

**Rationale**:
- Creates gentle, visually pleasing curves
- Consistent with assumption #3 in spec (25-30% range)
- Simple vector math: `offset = perpendicular_unit * chord_length * 0.25`

**Algorithm**:
```rust
fn default_control_point(start: Point, end: Point) -> Point {
    let chord = end - start;
    let chord_length = chord.length();
    let perpendicular = Point::new(-chord.y, chord.x).normalized();
    let midpoint = Point::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);
    midpoint + perpendicular * (chord_length * 0.25)
}
```

**Direction Convention**: Offset in positive perpendicular direction (counterclockwise from chord vector). This creates consistent "outward" curves.

---

### 5. Integration Architecture

**Parser Layer** (`src/parser/`):
- Add `CurveTo` token to lexer
- Add `CurveToDecl` variant to `PathCommand` enum
- Parse `curve_to target [via: ref, position]?` syntax

**Layout Layer** (`src/layout/`):
- Add `RoutingMode::Curved` variant
- Extract `via` references from modifiers
- Resolve via element positions during connection routing
- Compute control points (explicit or default)

**Renderer Layer** (`src/renderer/`):
- Add `PathSegment::QuadraticTo` variant
- Implement SVG Q/T command generation
- Handle single and chained quadratic segments

---

### 6. Error Handling

**Compile-Time Errors** (per spec clarifications):

| Error | Detection Point | Message |
|-------|-----------------|---------|
| Missing via reference | Layout phase (element resolution) | "Steering vertex 'foo' not found" |
| Unpositioned via | Layout phase (bounds access) | "Steering vertex 'foo' has no resolved position" |
| Empty via list | Parser | "Expected element reference after 'via:'" |

**Implementation**: Use existing `ariadne` diagnostic system for consistent error formatting.

---

## Technical Constraints

### From Constitution
- ✅ Semantic over geometric: Steering vertices are semantic references, not coordinates
- ✅ First-attempt correctness: Default curves provide sensible output
- ✅ Explicit over implicit: Via references are explicit; defaults are documented
- ✅ Fail fast: Invalid references caught at compile time

### From Tech Stack
- ✅ Pure Rust: No new dependencies required
- ✅ No unsafe: Standard vector math operations
- ✅ Uses existing patterns: `StyleValue::Identifier`, `PathSegment` enum

---

## Open Questions Resolved

| Question | Resolution |
|----------|------------|
| Quadratic vs Cubic? | Quadratic - sufficient and simpler |
| How to chain segments? | SVG T command for automatic smoothing |
| Default curve direction? | Positive perpendicular (counterclockwise from chord) |
| Error handling? | Compile-time errors using ariadne diagnostics |
