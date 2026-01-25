# Implementation Plan: Curved Paths and Connectors

## Technical Context

| Component | Technology | Notes |
|-----------|------------|-------|
| Language | Rust 2021 edition | Existing codebase |
| Parser | chumsky + logos | Pattern-based grammar |
| Renderer | SVG output | Direct XML generation |
| Layout | Custom constraint solver | BoundingBox-based |

**Dependencies**: No new dependencies required. Uses existing SVG path primitives.

---

## Constitution Check

| Principle | Compliance | Notes |
|-----------|------------|-------|
| 1. Semantic Over Geometric | ✅ Pass | Steering vertices are named references, not coordinates |
| 2. First-Attempt Correctness | ✅ Pass | Default curves provide sensible output without iteration |
| 3. Explicit Over Implicit | ✅ Pass | Via references are explicit; auto-generation is documented |
| 4. Fail Fast, Fail Clearly | ✅ Pass | Invalid references caught at compile time with ariadne |
| 5. Composability | ✅ Pass | Curves compose with existing paths, connectors, constraints |
| 6. Don't Reinvent Wheel | ✅ Pass | Uses SVG native quadratic Bezier (Q/T commands) |

---

## Tech Stack Compliance Report

### ✅ Approved Technologies
- Rust (existing)
- chumsky parser (existing)
- logos lexer (existing)
- ariadne diagnostics (existing)
- SVG output format (existing)

### ➕ New Technologies
*None - this feature uses only existing stack*

### ⚠️ Conflicting Technologies
*None detected*

### ❌ Prohibited Technologies
*None used*

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         Parser Layer                             │
├─────────────────────────────────────────────────────────────────┤
│  lexer.rs: Add `curve_to` token                                 │
│  ast.rs: Add CurveToDecl to PathCommand enum                    │
│  grammar.rs: Parse curve_to command with optional via modifier  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                         Layout Layer                             │
├─────────────────────────────────────────────────────────────────┤
│  routing.rs: Add RoutingMode::Curved variant                    │
│  routing.rs: Extract via references from modifiers              │
│  routing.rs: Compute control points (explicit or default)       │
│  types.rs: Add control_points field to ConnectionLayout         │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                        Renderer Layer                            │
├─────────────────────────────────────────────────────────────────┤
│  path.rs: Add PathSegment::QuadraticTo variant                  │
│  path.rs: Implement Q/T command SVG generation                  │
│  svg.rs: Render curved connections with quadratic segments      │
└─────────────────────────────────────────────────────────────────┘
```

---

## Implementation Phases

### Phase 1: Parser Extensions

**Goal**: Parse `curve_to` command and `[via: ref]` modifier syntax.

**Files Modified**:
- `src/parser/lexer.rs`: Add `CurveTo` token
- `src/parser/ast.rs`: Add `CurveToDecl` struct
- `src/parser/grammar.rs`: Parse curve_to command

**AST Addition**:
```rust
// In PathCommand enum
CurveTo(CurveToDecl),

// New struct
pub struct CurveToDecl {
    pub target: Identifier,
    pub via: Option<Identifier>,      // Optional steering vertex reference
    pub position: Option<VertexPosition>,
}
```

**Grammar Pattern**:
```
curve_to := "curve_to" identifier ("[" modifiers "]")?
modifiers := modifier ("," modifier)*
modifier := "via" ":" identifier | position_modifier
```

**Tests**:
- Parse `curve_to target`
- Parse `curve_to target [via: control]`
- Parse `curve_to target [via: control, x: 10]`
- Error on `curve_to` without target

---

### Phase 2: Curved Connector Routing

**Goal**: Add `[routing: curved]` support with via references.

**Files Modified**:
- `src/layout/routing.rs`: Add `RoutingMode::Curved`
- `src/layout/routing.rs`: Via reference extraction and resolution
- `src/layout/types.rs`: Add control points to `ConnectionLayout`

**Routing Mode Addition**:
```rust
pub enum RoutingMode {
    Direct,
    #[default]
    Orthogonal,
    Curved,  // NEW
}
```

**Control Point Computation**:
```rust
fn compute_curved_route(
    from: &BoundingBox,
    to: &BoundingBox,
    via_points: Vec<Point>,  // From resolved via references
) -> (Vec<Point>, Vec<Point>)  // (path_points, control_points)
```

**Default Curve Calculation**:
```rust
fn default_control_point(start: Point, end: Point) -> Point {
    let chord = end - start;
    let midpoint = (start + end) * 0.5;
    let perpendicular = Point::new(-chord.y, chord.x).normalized();
    midpoint + perpendicular * (chord.length() * 0.25)
}
```

**Tests**:
- `a -> b [routing: curved]` produces curved connection
- `a -> b [routing: curved, via: c]` uses element c as control point
- `a -> b [routing: curved, via: c, d]` chains quadratics
- Error on invalid via reference

---

### Phase 3: Path curve_to Rendering

**Goal**: Render curve_to commands as SVG quadratic Bezier.

**Files Modified**:
- `src/renderer/path.rs`: Add `QuadraticTo` segment type
- `src/renderer/path.rs`: Implement SVG Q/T generation
- `src/renderer/svg.rs`: Handle curved connections

**New PathSegment Variant**:
```rust
pub enum PathSegment {
    MoveTo(Point),
    LineTo(Point),
    ArcTo { ... },
    QuadraticTo {          // NEW
        control: Point,
        end: Point,
    },
    SmoothQuadraticTo(Point),  // NEW - SVG T command
    Close,
}
```

**SVG Generation**:
```rust
PathSegment::QuadraticTo { control, end } => {
    format!("Q {} {}, {} {}", control.x, control.y, end.x, end.y)
}
PathSegment::SmoothQuadraticTo(end) => {
    format!("T {} {}", end.x, end.y)
}
```

**Tests**:
- Single curve_to renders as Q command
- Multiple curve_to in sequence uses T for smooth joins
- Mixed line_to and curve_to renders correctly

---

### Phase 4: Via Reference Resolution

**Goal**: Resolve via references to element positions with proper error handling.

**Files Modified**:
- `src/layout/routing.rs`: Via reference resolution during layout
- `src/layout/engine.rs`: Integration with element lookup

**Resolution Pattern** (following existing label reference pattern):
```rust
fn resolve_via_references(
    modifiers: &[Spanned<StyleModifier>],
    result: &LayoutResult,
) -> Result<Vec<Point>, LayoutError> {
    let via_refs = extract_via_identifiers(modifiers);
    via_refs.iter().map(|id| {
        result.get_element_by_name(&id.0)
            .ok_or_else(|| LayoutError::ElementNotFound(id.clone()))
            .map(|elem| elem.bounds.center())
    }).collect()
}
```

**Error Cases**:
- `ElementNotFound`: "Steering vertex 'foo' not found"
- `NoPosition`: "Steering vertex 'foo' has no resolved position"

**Tests**:
- Valid via reference resolves to element center
- Missing via reference produces clear error
- Via reference to unpositioned element errors

---

### Phase 5: Integration and Grammar Update

**Goal**: Update grammar.ebnf and integration tests.

**Files Modified**:
- `features/001-the-grammar-and-ast-for-our-dsl/contracts/grammar.ebnf`
- `tests/` - Add integration tests
- `examples/` - Add example files

**Grammar EBNF Update**:
```ebnf
path_command := vertex_decl | line_to_decl | arc_to_decl | curve_to_decl | close_decl

curve_to_decl := "curve_to" identifier ("[" curve_modifiers "]")?
curve_modifiers := curve_modifier ("," curve_modifier)*
curve_modifier := "via" ":" identifier | position_modifier

connection_modifier := ... | "routing" ":" routing_mode | "via" ":" via_list
routing_mode := "direct" | "orthogonal" | "curved"
via_list := identifier ("," identifier)*
```

**Example Files**:
- `examples/curved-connector.ail`: Basic curved connection demo
- `examples/spline-path.ail`: Multi-segment curved path
- `examples/mixed-routing.ail`: Combined straight, orthogonal, curved

---

## Test Strategy

### Unit Tests
| Component | Test Coverage |
|-----------|---------------|
| Parser | curve_to syntax, via modifier parsing, error cases |
| Routing | Control point calculation, via resolution, error handling |
| Renderer | Q/T command generation, segment sequencing |

### Integration Tests
| Test | Description |
|------|-------------|
| `curved_connector_basic` | Simple `a -> b [routing: curved]` |
| `curved_connector_via` | With explicit via point |
| `curved_connector_multi_via` | Chained quadratics |
| `path_curve_to` | curve_to command in path shape |
| `mixed_path_commands` | line_to + curve_to + arc_to |

### Snapshot Tests
- SVG output for each example file
- Regression detection for curve rendering

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| SVG T command browser inconsistency | Low | Medium | Test across browsers; fallback to explicit Q |
| Control point at element center not ideal | Medium | Low | Allow future refinement; default works for most cases |
| Circular via references | Low | Medium | Detect cycles during reference resolution |

---

## Estimated Complexity

| Phase | Scope | Files | LOC Estimate |
|-------|-------|-------|--------------|
| Phase 1: Parser | Small | 3 | ~80 |
| Phase 2: Routing | Medium | 2 | ~150 |
| Phase 3: Rendering | Small | 2 | ~60 |
| Phase 4: Resolution | Medium | 2 | ~100 |
| Phase 5: Integration | Small | 3+ | ~50 |
| **Total** | | **12** | **~440** |

---

## Success Metrics

1. **Parser**: All curve_to and via syntax variants parse correctly
2. **Routing**: Curved connections render with smooth curves
3. **Rendering**: SVG output validates and displays correctly in browsers
4. **Errors**: Invalid via references produce clear, actionable messages
5. **Grammar**: EBNF updated and in sync with implementation
