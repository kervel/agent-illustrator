# Tasks: Anchor-Based Constraints

**Feature**: 011-anchor-based-constraints
**Created**: 2026-01-28
**Plan**: [plan.md](plan.md) | **Research**: [research.md](research.md)

---

## Phase 1: AST Extension

### T001: Add AnchorX/AnchorY variants to ConstraintProperty
- [x] Add `AnchorX(String)` variant to `ConstraintProperty` enum
- [x] Add `AnchorY(String)` variant to `ConstraintProperty` enum
- [x] Remove `Copy` from derive, keep `Clone, Debug, PartialEq, Eq`
- [x] Update `from_str()`: add `_x`/`_y` suffix matching AFTER all built-in properties
- [x] Verify: `from_str("drain_x")` → `AnchorX("drain")`
- [x] Verify: `from_str("center_x")` → `CenterX` (NOT `AnchorX("center")`)

**File**: `src/parser/ast.rs:451-489`

**Key logic** — `from_str()` extended with fallback pattern:
```rust
// After all explicit matches for built-in properties:
_ if s.ends_with("_x") => {
    let anchor_name = &s[..s.len() - 2];
    Some(Self::AnchorX(anchor_name.to_string()))
}
_ if s.ends_with("_y") => {
    let anchor_name = &s[..s.len() - 2];
    Some(Self::AnchorY(anchor_name.to_string()))
}
_ => None,
```

---

## Phase 2: Fix Copy Removal Compilation Errors

### T002: Fix collector.rs match blocks
- [x] Change `match prop_ref.property.node` → `match &prop_ref.property.node` at line 454 (`property_to_variable`)
- [x] Add `AnchorX(_) | AnchorY(_)` arms (see Phase 3 for actual logic)
- [x] Fix midpoint handling at lines 376 and 392 (same pattern)
- [x] Add `AnchorX(_) | AnchorY(_)` arms to midpoint match blocks

**File**: `src/layout/collector.rs:376,392,454`

### T003: Fix engine.rs match blocks [P]
- [x] Fix `recompute_custom_anchors` match blocks at lines ~1566-1605 and ~2686-2722
- [x] Add `AnchorX(_) | AnchorY(_)` arms (these are for anchor position resolution, anchor refs may not appear here — add unreachable or skip arm)

**File**: `src/layout/engine.rs:1566-1605,2686-2722`

### T004: Fix types.rs AnchorDirection::from_property [P]
- [x] Change `match prop` → `match prop` (already borrows)
- [x] Add `AnchorX(_) | AnchorY(_) => AnchorDirection::Down` arm (default direction for anchors)

**File**: `src/layout/types.rs:47-55`

### T005: Fix grammar.rs test assertions [P]
- [x] Update any `matches!(x, ConstraintProperty::...)` assertions
- [x] These should work without change if matching on reference

**File**: `src/parser/grammar.rs` (test functions)

### T006: Verify compilation
- [x] Run `cargo check` — must compile with zero errors
- [x] Run `cargo test` — all existing 365+ tests must pass

---

## Phase 3: Anchor Resolution in Constraints

### T007: Implement anchor resolution in collector
- [x] In `property_to_variable()`: handle `AnchorX(name)` → map to `LayoutProperty::X` (position lookup happens in engine)
- [x] In `property_to_variable()`: handle `AnchorY(name)` → map to `LayoutProperty::Y`
- [x] OR: introduce a new approach where anchor refs are resolved to constants before the solver sees them

**File**: `src/layout/collector.rs:449-470`

**Architecture note** (from research.md): Anchor positions are constants, not solver variables.
The collector must convert anchor refs on the right side of constraints into `LayoutConstraint::Fixed`
values. This requires access to `LayoutResult` which the collector doesn't currently have.

**Approach options**:
1. Pass `&LayoutResult` to collector methods (cleanest)
2. Post-process constraints in engine (keeps collector pure)
3. Resolve in `collect_constrain_statements()` in engine.rs where `LayoutResult` is available

### T008: Implement anchor position lookup
- [x] Create helper function: `resolve_anchor_position(result: &LayoutResult, element_id: &str, anchor_name: &str) -> Result<Point, LayoutError>`
- [x] Look up element in `result.elements`
- [x] Look up anchor in `element.anchors.get(anchor_name)`
- [x] Return `anchor.position` or error with available anchor names

**File**: `src/layout/engine.rs` (new helper function)

### T009: Wire anchor resolution into constraint collection
- [x] In `resolve_constrain_statements_two_phase()` or in a new pass:
  - When a constraint has an anchor ref, resolve it to a constant
  - Convert `Equal { left: foo.CenterX, right: bar.AnchorX("drain") }` into `Fixed { variable: foo.CenterX, value: <drain_x> }`
- [x] Ensure this works for both local and global constraint phases

**File**: `src/layout/engine.rs`

---

## Phase 4: Error Handling

### T010: Add anchor error messages
- [x] Ensure unknown anchor name produces: "Unknown anchor 'xyz' on element 'foo'. Available: top, bottom, left, right, drain, gate"
- [x] Use existing `LayoutError` variants or add new one if needed

**File**: `src/error.rs`, `src/layout/engine.rs`

---

## Phase 5: Tests

### T011: Unit tests for ConstraintProperty::from_str
- [x] `from_str("drain_x")` → `Some(AnchorX("drain"))`
- [x] `from_str("gate_y")` → `Some(AnchorY("gate"))`
- [x] `from_str("left_conn_x")` → `Some(AnchorX("left_conn"))`
- [x] `from_str("center_x")` → `Some(CenterX)` (NOT AnchorX)
- [x] `from_str("center_y")` → `Some(CenterY)` (NOT AnchorY)
- [x] `from_str("left")` → `Some(Left)` (NOT AnchorX with empty name)
- [x] `from_str("x")` → `Some(X)`
- [x] `from_str("y")` → `Some(Y)`
- [x] `from_str("unknown")` → `None`

**File**: `src/parser/ast.rs` (test module)

### T012: Integration test — basic anchor constraint
- [x] Template with custom anchor
- [x] Constraint: `constrain other.center_x = template_instance.anchor_x`
- [x] Verify element moves to correct position

**File**: `tests/anchor_constraints.rs` (new file)

### T013: Integration test — rotation + anchor constraint [P]
- [x] Template with rotation + custom anchor
- [x] Constraint referencing rotated anchor position
- [x] Verify post-rotation coordinates are used

**File**: `tests/anchor_constraints.rs`

### T014: Regression test — person-rotation.ail [P]
- [x] Render `person-rotation.ail` example
- [x] Verify it still produces valid SVG without errors
- [x] Compare against baseline SVG if available

**File**: `tests/svg_regression.rs` (existing test, verify it covers person-rotation)

### T015: Error test — unknown anchor
- [x] Constraint referencing non-existent anchor
- [x] Verify error message is clear and includes available anchors

**File**: `tests/anchor_constraints.rs`

---

## Phase 6: Example Update

### T016: Update MOSFET driver example
- [x] Add anchor-based constraint: `constrain d_flyback.center_x = q_main.drain_x`
- [x] Verify example renders correctly with flyback diode aligned to drain

**File**: `examples/mosfet-driver.ail`

---

## Phase 7: Final Validation

### T017: Full test suite
- [x] `cargo test` — all tests pass
- [x] `cargo clippy` — no new warnings
- [x] `cargo fmt` — formatted

### T018: Update grammar.ebnf if needed [P]
- [x] Check if `features/001-the-grammar-and-ast-for-our-dsl/contracts/grammar.ebnf` needs anchor property syntax

---

## Task Dependencies

```
T001 → T002 ──→ T006 → T007 → T008 → T009 → T010 → T011
       T003 ─╮                                        T012
       T004 ─┤                                        T013
       T005 ─╯                                        T014
                                                      T015
                                                      T016 → T017 → T018
```

**Parallel tasks**: T003/T004/T005 (all fix Copy removal in different files)
**Parallel tasks**: T012/T013/T014/T015 (independent test files)
**Parallel tasks**: T017/T018 (final validation)

---

## Checkpoints

### After Phase 2 (T006): Compilation checkpoint
- `cargo check` passes
- `cargo test` passes (all 365+ existing tests)
- No new functionality yet, just type changes

### After Phase 3 (T009): Feature complete
- Anchor constraints resolve correctly
- Both local and global constraints work with anchors

### After Phase 5 (T015): Test complete
- All new tests pass
- person-rotation.ail renders correctly
- Error messages are clear
