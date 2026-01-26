# Tasks: Feature 009 - Anchor Support for Shape Connections

<!-- Tech Stack Validation: PASSED -->
<!-- Validated against: .specswarm/tech-stack.md -->
<!-- No prohibited technologies found -->
<!-- No new technologies introduced -->

## Summary

| Metric | Value |
|--------|-------|
| Total Tasks | 24 |
| Phases | 7 |
| Parallel Opportunities | 8 task groups |
| Estimated LOC | ~610 |

---

## Phase 1: Foundational - AST Types

**Goal**: Add core data types to AST that all subsequent phases depend on.

**Checkpoint**: New AST types compile and existing tests pass.

### T001: Add AnchorDirection enum to layout types [x] ✓
**File**: `src/layout/types.rs`
**Description**: Add the `AnchorDirection` enum with `Up`, `Down`, `Left`, `Right`, and `Angle(f64)` variants. Include `to_vector()` method that converts direction to a unit vector and `from_property()` that infers direction from ConstraintProperty.

**Acceptance**:
- [x] Enum compiles with all variants
- [x] `to_vector()` returns correct unit vectors (Up=270°, Down=90°, Left=180°, Right=0°)
- [x] `from_property()` maps `.left`→Left, `.right`→Right, `.top`→Up, `.bottom`→Down

---

### T002: Add Anchor and AnchorSet structs to layout types [x] ✓
**File**: `src/layout/types.rs`
**Description**: Add `Anchor` struct with `name: String`, `position: Point`, `direction: AnchorDirection`. Add `AnchorSet` struct with `HashMap<String, Anchor>` and methods: `get()`, `insert()`, `names()`, `simple_shape()`, `path_shape()`, `from_custom()`.

**Depends on**: T001

**Acceptance**:
- [x] `Anchor::new()` constructor works
- [x] `AnchorSet::simple_shape()` creates 4 anchors (top, bottom, left, right)
- [x] `AnchorSet::path_shape()` creates 8 anchors (+ corners)
- [x] `AnchorSet::get()` and `names()` work correctly

---

### T003: Add AnchorReference struct to AST [x] ✓
**File**: `src/parser/ast.rs`
**Description**: Add `AnchorReference` struct with `element: Spanned<Identifier>` and `anchor: Option<Spanned<String>>`. Add helper methods `element_only()` and `with_anchor()`.

**Acceptance**:
- [x] Struct compiles
- [x] `element_only()` creates reference with `anchor: None`
- [x] `with_anchor()` creates reference with `anchor: Some(..)`

---

### T004: Add AnchorDecl and supporting types to AST [x] ✓
**File**: `src/parser/ast.rs`
**Description**: Add types for template anchor declarations:
- `AnchorDecl` with `name`, `position`, `direction` fields
- `AnchorPosition` enum with `PropertyRef` and `Expression` variants
- `AnchorDirectionSpec` enum with `Cardinal` and `Angle` variants
- `CardinalDirection` enum with `Up`, `Down`, `Left`, `Right`

**Acceptance**:
- [x] All types compile
- [x] Types are exported from ast module

---

### T005: Update ConnectionDecl to use AnchorReference [x] ✓
**File**: `src/parser/ast.rs`
**Description**: Change `ConnectionDecl.from` and `ConnectionDecl.to` from `Spanned<Identifier>` to `AnchorReference`. This is a breaking AST change that requires updating all code that creates or inspects ConnectionDecl.

**Depends on**: T003

**Acceptance**:
- [x] ConnectionDecl uses AnchorReference for from/to
- [x] Project compiles (may require stub changes in other files)

---

## Phase 2: Parser - Connection Anchor Syntax

**Goal**: Parse `element.anchor` syntax in connections.

**Checkpoint**: `box_a.right -> box_b.left` parses correctly; existing tests pass.

### T006: Add anchor keyword to lexer [P]
**File**: `src/parser/lexer.rs`
**Description**: Add `Anchor` token for the `anchor` keyword (used in template declarations).

**Acceptance**:
- [ ] `anchor` lexes as `Token::Anchor`
- [ ] Existing tokens unaffected

---

### T007: Parse element.anchor reference in grammar [P] ✓
**File**: `src/parser/grammar.rs`
**Description**: Modify the connection endpoint parser to accept optional `.anchor_name` suffix. Support patterns:
- `identifier` → AnchorReference with anchor=None
- `identifier.identifier` → AnchorReference with anchor=Some

**Acceptance**:
- [x] `a` parses as AnchorReference(element="a", anchor=None)
- [x] `a.right` parses as AnchorReference(element="a", anchor=Some("right"))
- [x] `a.top_left` parses correctly (underscore in anchor name)

---

### T008: Update connection parsing to use AnchorReference ✓
**File**: `src/parser/grammar.rs`
**Description**: Update the `connection()` parser function to use the new anchor reference parsing from T007. Ensure all connection variants work:
- `a -> b` (both auto-detect)
- `a.right -> b` (source anchor, target auto)
- `a -> b.left` (source auto, target anchor)
- `a.right -> b.left` (both explicit)

**Depends on**: T005, T007

**Acceptance**:
- [x] All four connection variants parse correctly
- [x] Existing connection tests still pass
- [x] Error on invalid syntax like `a. -> b`

---

### T009: Add parser unit tests for anchor references ✓
**File**: `src/parser/grammar.rs` (tests module)
**Description**: Add unit tests for anchor reference parsing:
- Basic anchor reference
- Mixed anchor/non-anchor connections
- Backward compatibility with plain connections
- Error cases (invalid anchor syntax)

**Depends on**: T008

**Acceptance**:
- [x] At least 5 test cases covering happy paths
- [ ] At least 2 test cases for error conditions (partial - need more error tests)

---

## Phase 3: Parser - Template Anchor Declarations

**Goal**: Parse `anchor name [position: element.property]` in templates.

**Checkpoint**: Template with anchor declarations parses correctly.

### T010: Parse anchor declaration statement [P]
**File**: `src/parser/grammar.rs`
**Description**: Add parser for `anchor` statement within templates:
```
anchor name [position: element.property]
anchor name [position: element.property, direction: up/down/left/right]
anchor name [position: element.property + offset, direction: 45]
```

**Depends on**: T004, T006

**Acceptance**:
- [ ] Basic anchor declaration parses
- [ ] Position with property reference parses
- [ ] Position with expression (+ offset) parses
- [ ] Optional direction modifier parses
- [ ] Angle-based direction (numeric) parses

---

### T011: Integrate anchor statement into template parsing
**File**: `src/parser/grammar.rs`
**Description**: Update template body parser to recognize `anchor` statements alongside shape declarations. Store parsed `AnchorDecl` in appropriate location (likely a new field in template AST or as a special Statement variant).

**Depends on**: T010

**Acceptance**:
- [ ] Template with anchor statements parses
- [ ] Multiple anchors in one template work
- [ ] Anchors can be interspersed with shapes

---

### T012: Add parser tests for anchor declarations
**File**: `src/parser/grammar.rs` (tests module)
**Description**: Add unit tests for anchor declaration parsing:
- Basic anchor with property ref
- Anchor with expression offset
- Anchor with cardinal direction
- Anchor with angle direction
- Error cases

**Depends on**: T011

**Acceptance**:
- [ ] At least 4 happy path tests
- [ ] At least 2 error condition tests

---

## Phase 4: Layout - Built-in Anchor Computation

**Goal**: Compute anchors for shapes after layout.

**Checkpoint**: Elements have computed AnchorSet with correct positions.

### T013: Add ResolvedAnchor type ✓
**File**: `src/layout/types.rs`
**Description**: Add `ResolvedAnchor` struct with `position: Point` and `direction: AnchorDirection`. This is used during connection routing.

**Depends on**: T001

**Acceptance**:
- [x] Struct compiles
- [x] Can be created from an Anchor

---

### T014: Add BoundingBox helper methods for anchor positions [P] ✓
**File**: `src/layout/types.rs`
**Description**: Add helper methods to BoundingBox if not already present:
- `top_center()` → Point
- `bottom_center()` → Point
- `left_center()` → Point
- `right_center()` → Point
- `top_left()`, `top_right()`, `bottom_left()`, `bottom_right()` → Point

**Acceptance**:
- [x] All 8 methods return correct points
- [x] Methods work with any bounding box dimensions

---

### T015: Implement compute_anchors() in layout engine ✓
**File**: `src/layout/engine.rs`
**Description**: Add function to compute anchors for all elements after layout resolution:
```rust
fn compute_anchors(elements: &mut HashMap<String, ElementLayout>) {
    for (_, elem) in elements.iter_mut() {
        elem.anchors = match elem.shape_type {
            ShapeType::Rect | ShapeType::Ellipse | ShapeType::Circle =>
                AnchorSet::simple_shape(&elem.bounds),
            ShapeType::Path =>
                AnchorSet::path_shape(&elem.bounds),
            // containers get simple_shape anchors
            _ => AnchorSet::simple_shape(&elem.bounds),
        };
    }
}
```

**Depends on**: T002, T014

**Implementation Note**: Instead of a separate compute_anchors function, anchors are computed inline during element creation in layout_shape(), layout_container(), and layout_group().

**Acceptance**:
- [x] Rect elements get 4 anchors
- [x] Path elements get 8 anchors
- [x] Container elements get 4 anchors
- [x] Anchor positions match bounding box edges

---

### T016: Integrate anchor computation into layout pipeline ✓
**File**: `src/layout/engine.rs`
**Description**: Call `compute_anchors()` at the appropriate point in the layout pipeline - after bounding boxes are finalized but before connection routing.

**Depends on**: T015

**Implementation Note**: Anchors are computed inline during element creation, so no separate integration step was needed.

**Acceptance**:
- [x] Anchors are computed for all elements
- [x] Anchors have correct positions based on final layout
- [x] Existing layout tests pass

---

## Phase 5: Routing - Anchor-Based Connection Endpoints

**Goal**: Use anchor positions and directions for connection routing.

**Checkpoint**: Connections with anchors route to correct positions with perpendicular approach.

### T017: Add resolve_anchor() function ✓
**File**: `src/layout/routing.rs`
**Description**: Add function to resolve an AnchorReference to a ResolvedAnchor:
```rust
fn resolve_anchor(
    anchor_ref: &AnchorReference,
    elements: &HashMap<String, ElementLayout>,
) -> Result<ResolvedAnchor, LayoutError>
```
When anchor is None, return center with auto-computed direction.
When anchor is Some, look up in element's AnchorSet.

**Depends on**: T013, T016

**Acceptance**:
- [x] Returns correct position for explicit anchor
- [x] Returns center for auto-detect (anchor=None)
- [x] Returns error with valid anchor list for invalid anchor name

---

### T018: Add InvalidAnchor error variant ✓
**File**: `src/layout/error.rs` (or wherever LayoutError is defined)
**Description**: Add error variant for invalid anchor references:
```rust
InvalidAnchor {
    element: String,
    anchor: String,
    valid_anchors: Vec<String>,
}
```
Implement Display to show helpful error message with suggestions.

**Acceptance**:
- [x] Error variant exists
- [x] Display shows element name, invalid anchor, and valid options

---

### T019: Update route_connection() to use anchors ✓
**File**: `src/layout/routing.rs`
**Description**: Update `route_connection()` signature and implementation to accept optional `ResolvedAnchor` for start and end. When anchors are provided:
- Use anchor position as start/end point
- For orthogonal routing: ensure first/last segment aligns with anchor direction
- For curved routing: use direction as hint for control point
- For direct routing: straight line between anchor positions

**Implementation Note**: Added `route_connection_with_anchors()` function that accepts optional anchors.

**Depends on**: T017

**Acceptance**:
- [x] Connections with explicit anchors use anchor positions
- [x] Orthogonal routing starts/ends perpendicular to anchor
- [x] Connections without anchors use existing auto-detect behavior

---

### T020: Integrate anchor resolution into connection routing pipeline ✓
**File**: `src/layout/routing.rs` (in route_connections function)
**Description**: Update the connection routing code to:
1. Extract AnchorReference from ConnectionDecl
2. Call resolve_anchor() for source and target
3. Pass resolved anchors to route_connection()

**Depends on**: T019

**Acceptance**:
- [x] Anchor-based connections route correctly
- [x] Mixed connections (one anchor, one auto) work
- [x] Plain connections (no anchors) unchanged

---

## Phase 6: Template Anchor Resolution

**Goal**: Template-defined anchors work on instances.

**Checkpoint**: `server.input` resolves to template-defined anchor position.

### T021: Store anchor declarations in Template
**File**: `src/template/registry.rs`
**Description**: Add `anchors: Vec<AnchorDecl>` field to Template struct. Update template registration to store parsed anchor declarations.

**Depends on**: T011

**Acceptance**:
- [ ] Template struct has anchors field
- [ ] Anchor declarations stored during registration

---

### T022: Resolve template anchors during expansion
**File**: `src/template/resolver.rs`
**Description**: During template instantiation, resolve anchor declarations:
1. For each AnchorDecl, compute position based on internal element
2. Infer direction from position property (or use explicit direction)
3. Create Anchor and add to instance's AnchorSet
4. Merge with built-in anchors (custom anchors take precedence)

**Depends on**: T021, T016

**Acceptance**:
- [ ] Template instance has custom anchors
- [ ] Anchor positions computed from internal elements
- [ ] Directions inferred correctly
- [ ] Built-in anchors still accessible

---

### T023: Add template anchor tests
**File**: `tests/` (integration tests)
**Description**: Add integration tests for template anchors:
- Template with custom anchor definitions
- Instance anchor resolution
- Connection to template anchor
- Error for invalid template anchor reference

**Depends on**: T022

**Acceptance**:
- [ ] Template with anchors compiles and renders
- [ ] Connections to template anchors work
- [ ] Error messages for invalid anchors are helpful

---

## Phase 7: Integration and Polish

**Goal**: Complete feature with examples and error handling.

**Checkpoint**: All tests pass, examples work, feature complete.

### T024: Update feedback-loops example and add anchors-demo [P]
**File**: `examples/feedback-loops.ail`, `examples/anchors-demo.ail`
**Description**:
1. Update feedback-loops.ail to use anchor syntax for loop-back connections
2. Create new anchors-demo.ail with:
   - Basic anchor connections between rects
   - Template with custom anchors
   - Various routing modes with anchors

**Depends on**: T020, T022

**Acceptance**:
- [ ] feedback-loops.ail uses anchors where beneficial
- [ ] anchors-demo.ail demonstrates all anchor features
- [ ] Both examples render correctly

---

## Dependency Graph

```
T001 ─┬─► T002 ───► T015 ───► T016 ───► T020 ───► T024
      │                                    │
      └─► T013 ───────────────► T017 ──────┘
                                  │
T003 ───► T005 ───► T008 ───► T009      │
                      ▲                  │
T006 ─┬─► T007 ──────┘                   │
      │                                  │
      └─► T010 ───► T011 ───► T012      │
                      │                  │
                      └─► T021 ──► T022 ─┤
                                    │    │
T014 ──────────────────────────────►│    │
                                    │    │
T018 ──────────────────────────► T019 ──►│
                                         │
                              T023 ◄─────┘
```

---

## Parallel Execution Opportunities

### Group 1: Foundation Types (T001-T004)
```bash
# T001 first, then T002-T004 can parallelize
T001 → [T002, T003, T004] in parallel
```

### Group 2: Parser Work (T006-T012)
```bash
# After T004-T005
[T006, T007] in parallel → T008 → T009
T006 → T010 → T011 → T012
```

### Group 3: Layout Work (T013-T016)
```bash
# After T001-T002
[T013, T014] in parallel → T015 → T016
```

### Group 4: Routing Work (T017-T020)
```bash
T017 + T018 → T019 → T020
```

### Group 5: Template Work (T021-T023)
```bash
T021 → T022 → T023
```

### Group 6: Final Integration (T024)
```bash
T024 (after T020, T022)
```

---

## Implementation Strategy

### MVP (Minimum Viable Product)
Complete Phases 1-5 (T001-T020) for:
- Built-in anchors on all shapes
- Connection syntax with anchors
- Direction-aware routing

### Full Feature
Add Phase 6 (T021-T023) for:
- Template custom anchors

### Polish
Add Phase 7 (T024) for:
- Examples and documentation

---

## Success Criteria

| Criterion | Tasks |
|-----------|-------|
| Backward compatibility | T008, T009, T020 |
| Parser completeness | T009, T012 |
| Anchor computation | T015, T016 |
| Routing integration | T019, T020 |
| Template support | T022, T023 |
| Examples | T024 |
