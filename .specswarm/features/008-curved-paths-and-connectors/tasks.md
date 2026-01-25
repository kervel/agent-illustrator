# Tasks: Curved Paths and Connectors

<!-- Tech Stack Validation: PASSED -->
<!-- Validated against: .specswarm/tech-stack.md -->
<!-- No prohibited technologies found -->
<!-- No new dependencies required -->

## Overview

| Metric | Value |
|--------|-------|
| Total Tasks | 19 |
| Parallel Opportunities | 8 |
| Estimated LOC | ~440 |

---

## User Stories (from spec.md)

| ID | Story | Priority |
|----|-------|----------|
| US1 | Curved Connectors - Simple `[routing: curved]` with auto-generated control points | P1 |
| US2 | Curved Connectors with Via - Explicit steering vertices `[via: element]` | P1 |
| US3 | Curved Path Shapes - `curve_to` command in path definitions | P2 |
| US4 | Multi-Via Chained Curves - Multiple via points for S-curves | P2 |

---

## Phase 1: Foundational (Parser Infrastructure)

**Goal**: Extend parser to support `curve_to` command and `[via:]` modifier syntax.

These tasks are blocking for all user stories.

### T001: Add `curve_to` token to lexer [Foundational]
**File**: `src/parser/lexer.rs`
**Action**: Add `CurveTo` token variant with `#[token("curve_to")]` attribute
**Acceptance**: Token recognized in lexer output

```rust
// Add to Token enum after existing path tokens
#[token("curve_to")]
CurveTo,
```

---

### T002: Add CurveToDecl struct to AST [Foundational]
**File**: `src/parser/ast.rs`
**Action**: Define `CurveToDecl` struct with target, via, and position fields
**Acceptance**: Struct compiles, follows existing `LineToDecl`/`ArcToDecl` patterns

```rust
/// A curve_to path command - quadratic Bezier segment
#[derive(Debug, Clone, PartialEq)]
pub struct CurveToDecl {
    pub target: Identifier,
    pub via: Option<Identifier>,
    pub position: Option<VertexPosition>,
}
```

---

### T003: Add CurveTo variant to PathCommand enum [Foundational]
**File**: `src/parser/ast.rs`
**Action**: Add `CurveTo(CurveToDecl)` to `PathCommand` enum
**Depends on**: T002
**Acceptance**: Enum variant added, existing match statements updated

---

### T004: Parse curve_to command in grammar [Foundational]
**File**: `src/parser/grammar.rs`
**Action**: Add parser rule for `curve_to target [modifiers]?` following `line_to` pattern
**Depends on**: T001, T003
**Acceptance**: Parses `curve_to foo`, `curve_to foo [via: bar]`, `curve_to foo [via: bar, x: 10]`

---

### T005: Parse via modifier in style modifiers [Foundational]
**File**: `src/parser/grammar.rs`
**Action**: Ensure `via` parses as `StyleKey::Custom("via")` with identifier or identifier list value
**Acceptance**: `[via: ctrl]` and `[via: c1, c2]` parse correctly

---

**CHECKPOINT**: Parser recognizes all new syntax. Run `cargo test` on parser module.

---

## Phase 2: Renderer Infrastructure

**Goal**: Add SVG quadratic Bezier rendering capability.

### T006: Add QuadraticTo variant to PathSegment [P]
**File**: `src/renderer/path.rs`
**Action**: Add `QuadraticTo { control: Point, end: Point }` to `PathSegment` enum
**Acceptance**: Enum variant compiles

```rust
/// Quadratic Bezier curve (SVG Q command)
QuadraticTo {
    control: Point,
    end: Point,
},
```

---

### T007: Add SmoothQuadraticTo variant to PathSegment [P]
**File**: `src/renderer/path.rs`
**Action**: Add `SmoothQuadraticTo(Point)` for SVG T command (smooth continuation)
**Acceptance**: Enum variant compiles

---

### T008: Implement SVG Q/T command generation
**File**: `src/renderer/path.rs`
**Action**: Add match arms in `to_svg_d()` for QuadraticTo and SmoothQuadraticTo
**Depends on**: T006, T007
**Acceptance**: `QuadraticTo { control: (50,20), end: (100,50) }` produces `"Q 50 20, 100 50"`

```rust
PathSegment::QuadraticTo { control, end } => {
    format!("Q {} {}, {} {}", control.x, control.y, end.x, end.y)
}
PathSegment::SmoothQuadraticTo(end) => {
    format!("T {} {}", end.x, end.y)
}
```

---

**CHECKPOINT**: Renderer can emit Q/T commands. Run `cargo test` on renderer module.

---

## Phase 3: User Story 1 - Basic Curved Connectors [US1]

**Goal**: `a -> b [routing: curved]` produces a smooth curved connection with auto-generated control point.

**Independent Test**: Create `.ail` file with `a -> b [routing: curved]`, verify SVG output contains Q command.

### T009: Add RoutingMode::Curved variant
**File**: `src/layout/routing.rs`
**Action**: Add `Curved` variant to `RoutingMode` enum
**Acceptance**: Enum compiles

---

### T010: Extract "curved" routing mode from modifiers
**File**: `src/layout/routing.rs`
**Action**: Add case in `extract_routing_mode()` to match `"curved"` keyword
**Depends on**: T009
**Acceptance**: `[routing: curved]` returns `RoutingMode::Curved`

---

### T011: Implement default control point calculation
**File**: `src/layout/routing.rs`
**Action**: Create `default_control_point(start: Point, end: Point) -> Point` function
**Acceptance**: Returns point offset perpendicular to chord at 25% of chord length

```rust
fn default_control_point(start: Point, end: Point) -> Point {
    let chord = Point::new(end.x - start.x, end.y - start.y);
    let length = (chord.x * chord.x + chord.y * chord.y).sqrt();
    let midpoint = Point::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);
    // Perpendicular unit vector (rotate 90 degrees)
    let perp = Point::new(-chord.y / length, chord.x / length);
    Point::new(
        midpoint.x + perp.x * length * 0.25,
        midpoint.y + perp.y * length * 0.25,
    )
}
```

---

### T012: Add control_points field to ConnectionLayout
**File**: `src/layout/types.rs`
**Action**: Add `control_points: Vec<Point>` field to `ConnectionLayout` struct
**Acceptance**: Struct compiles, existing code still works (empty vec for non-curved)

---

### T013: Implement curved routing in route_connection
**File**: `src/layout/routing.rs`
**Action**: Add `RoutingMode::Curved` branch that computes path + control point using default
**Depends on**: T009, T010, T011, T012
**Acceptance**: Curved connections have path=[start, end] and control_points=[default_point]

---

### T014: Render curved connections with Q command
**File**: `src/renderer/svg.rs`
**Action**: When rendering ConnectionLayout with non-empty control_points, use QuadraticTo segments
**Depends on**: T008, T012, T013
**Acceptance**: SVG output contains `Q cx cy, ex ey` for curved connections

---

**CHECKPOINT [US1]**: Basic `a -> b [routing: curved]` works end-to-end.

---

## Phase 4: User Story 2 - Via Reference Resolution [US2]

**Goal**: `a -> b [routing: curved, via: my_control]` uses element center as control point.

**Independent Test**: Create `.ail` with steering element, verify curve bends toward it.

### T015: Extract via identifiers from modifiers [P]
**File**: `src/layout/routing.rs`
**Action**: Create `extract_via_references(modifiers) -> Vec<Identifier>` function
**Acceptance**: `[via: c]` returns `[c]`, `[via: c1, c2]` returns `[c1, c2]`

---

### T016: Resolve via references to element positions
**File**: `src/layout/routing.rs`
**Action**: Create `resolve_via_references(via_ids, result) -> Result<Vec<Point>, LayoutError>`
**Depends on**: T015
**Acceptance**: Valid references resolve to element centers; invalid references return error

---

### T017: Integrate via resolution into curved routing
**File**: `src/layout/routing.rs`
**Action**: In curved routing branch, check for via references and use resolved points instead of default
**Depends on**: T013, T016
**Acceptance**: `[via: ctrl]` uses ctrl's center; no via uses default

---

### T018: Add error messages for invalid via references
**File**: `src/layout/routing.rs` or `src/error.rs`
**Action**: Add `LayoutError::SteeringVertexNotFound(String)` variant with ariadne diagnostic
**Depends on**: T016
**Acceptance**: Missing via produces "Steering vertex 'foo' not found" error

---

**CHECKPOINT [US2]**: `a -> b [routing: curved, via: ctrl]` works with proper error handling.

---

## Phase 5: User Story 3 - Path curve_to Command [US3]

**Goal**: `curve_to target [via: ctrl]` works in path definitions.

**Independent Test**: Create path shape with curve_to, verify SVG contains Q command.

### T019: Resolve curve_to commands in path rendering
**File**: `src/renderer/path.rs`
**Action**: In `resolve_path()`, handle `PathCommand::CurveTo` - resolve via reference or compute default, emit QuadraticTo segment
**Depends on**: T004, T006, T016
**Acceptance**: Path with curve_to renders correctly with Q command

---

**CHECKPOINT [US3]**: Path shapes with `curve_to` work end-to-end.

---

## Phase 6: User Story 4 - Multi-Via Chained Curves [US4]

**Goal**: `[via: v1, v2]` creates smooth S-curve with chained quadratics.

**Independent Test**: Create connection with two via points, verify smooth S-curve.

### T020: Handle multiple via points in curved routing
**File**: `src/layout/routing.rs`
**Action**: When multiple via points resolved, create intermediate path points and chain Q segments
**Depends on**: T017
**Acceptance**: Two via points create path=[start, mid1, mid2, end] with appropriate control points

---

### T021: Use SmoothQuadraticTo for chained segments
**File**: `src/renderer/svg.rs`
**Action**: When rendering chained curves, use Q for first segment, T for subsequent smooth joins
**Depends on**: T007, T020
**Acceptance**: Multi-via curves render as `Q ... T ... T ...` for smooth continuity

---

**CHECKPOINT [US4]**: Multi-point S-curves work with smooth joins.

---

## Phase 7: Polish & Integration

**Goal**: Update grammar documentation and add example files.

### T022: Update grammar.ebnf with curve syntax [P]
**File**: `.specswarm/features/001-the-grammar-and-ast-for-our-dsl/contracts/grammar.ebnf`
**Action**: Add `curve_to_decl` rule and `curved` routing mode
**Acceptance**: EBNF is valid and matches implementation

---

### T023: Create curved-connector.ail example [P]
**File**: `examples/curved-connector.ail`
**Action**: Demo file showing basic curved connections
**Acceptance**: File parses and renders correctly

---

### T024: Create spline-path.ail example [P]
**File**: `examples/spline-path.ail`
**Action**: Demo file showing curve_to in path shapes
**Acceptance**: File parses and renders correctly

---

**FINAL CHECKPOINT**: All examples render, `cargo test` passes, grammar in sync.

---

## Dependencies Graph

```
T001 ─┬─→ T004 ─→ T005
T002 ─┤
T003 ─┘

T006 ─┬─→ T008 ─→ T014
T007 ─┘

T009 → T010 → T011 → T013 → T014 → [US1 Complete]

T015 → T016 → T017 → T018 → [US2 Complete]
                ↓
              T019 → [US3 Complete]
                ↓
         T020 → T021 → [US4 Complete]

T022, T023, T024 → [Polish Complete]
```

---

## Parallel Execution Opportunities

### Batch 1 (Foundation)
```
T001 ──┐
T002 ──┼──→ T003 → T004 → T005
       │
T006 ──┼──→ T008
T007 ──┘
```

### Batch 2 (US1 + US2 partial)
```
T009 → T010 → T011 ──┐
                     ├──→ T013 → T014
T012 ────────────────┘

T015 [P] (can run alongside T009-T012)
```

### Batch 3 (US2 + US3)
```
T016 → T017 → T018 → [US2]
         ↓
       T019 → [US3]
```

### Batch 4 (US4 + Polish)
```
T020 → T021 → [US4]

T022 [P]
T023 [P]
T024 [P]
```

---

## Implementation Strategy

**MVP (Minimum Viable Product)**: Complete US1 + US2 (Phases 1-4)
- Basic `[routing: curved]` with auto-generated and explicit control points
- Core value delivered: smooth curved connectors

**Full Feature**: Add US3 + US4 (Phases 5-6)
- `curve_to` in path shapes
- Multi-point S-curves

**Polish**: Phase 7
- Documentation and examples

---

## Task Summary by User Story

| Story | Tasks | Parallel | Dependencies |
|-------|-------|----------|--------------|
| Foundational | T001-T005 | 2 | None |
| Renderer | T006-T008 | 2 | None |
| US1: Basic Curved | T009-T014 | 1 | Foundational, Renderer |
| US2: Via References | T015-T018 | 1 | US1 |
| US3: Path curve_to | T019 | 0 | US2 |
| US4: Multi-Via | T020-T021 | 0 | US3 |
| Polish | T022-T024 | 3 | US4 |
