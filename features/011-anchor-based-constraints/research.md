# Research: Anchor-Based Constraints

**Feature**: 011-anchor-based-constraints
**Created**: 2026-01-28

---

## Key Discovery: Two-Layer Constraint Architecture

The constraint system has two distinct layers that anchor references must bridge:

### Layer 1: AST (Parser)
- **File**: `src/parser/ast.rs`
- **Type**: `ConstraintProperty` enum — property names parsed from source text
- **Current variants**: `X, Y, Width, Height, Left, Right, Top, Bottom, CenterX, CenterY, Center`
- **Traits**: `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`
- **Parsing**: `ConstraintProperty::from_str(s)` matches string → variant

### Layer 2: Solver (Layout)
- **File**: `src/layout/solver.rs`
- **Type**: `LayoutProperty` enum — kasuari solver variables
- **Variants**: `X, Y, Width, Height, CenterX, CenterY, Right, Bottom`
- **These are the actual variables in the Cassowary solver**

### Bridge: Collector
- **File**: `src/layout/collector.rs`
- **Function**: `property_to_variable()` at line 450
- **Maps**: `ConstraintProperty` → `LayoutVariable(element_id, LayoutProperty)`
- This is where the translation happens

---

## Critical Insight: Anchors Are Not Solver Variables

Standard properties (Left, CenterX, etc.) map to expressions over solver variables:
- `CenterX` → `x + width/2` (derived in `solver.rs:339-343`)
- `Right` → `x + width` (derived in `solver.rs:351-355`)

**Anchor positions cannot be expressed this way.** An anchor like `drain` has a position that depends on the template's internal layout, not on simple bounding box algebra. The anchor's x/y is a **pre-computed constant**, not a derived expression.

### Implication for Constraint Collection

When the collector encounters `constrain foo.center_x = bar.drain_x`:
- **Left side** (`foo.center_x`): maps normally to `LayoutVariable("foo", CenterX)` — a solver variable
- **Right side** (`bar.drain_x`): must resolve to a **constant value** by looking up `bar`'s anchor set for `drain.position.x`

This means anchor references produce **Fixed constraints** (constant values), not **Equal constraints** (variable-to-variable).

Example transformation:
```
constrain foo.center_x = bar.drain_x
// If bar.drain anchor is at position (150, 80):
→ LayoutConstraint::Fixed { variable: foo.CenterX, value: 150.0 }
```

### Consequence: Collector Needs Layout Result Access

Currently `property_to_variable()` only needs the property name and element ID. For anchors, it needs to **look up the current anchor position** from the `LayoutResult`. This means either:

**Option A**: Pass `LayoutResult` to the collector (changes API)
**Option B**: Handle anchor resolution in the engine, not the collector
**Option C**: Two-pass collection: first collect anchor refs, then resolve them

**Decision: Option B** — Handle in the engine (`engine.rs`) where `LayoutResult` is already available. The collector can still produce `LayoutConstraint` variants, but the engine resolves anchor references before passing constraints to the solver.

---

## Two-Phase Solver Integration

The two-phase solver (Feature 010) works as follows:

```
Phase 1: solve_local() — Solve constraints within each template instance
Phase 2: apply_rotation_to_local_result() — Rotate bounds and anchors
Phase 2b: Handle templates with rotation but no constraints
Phase 3: apply_local_results() — Write local results back to LayoutResult
Phase 4: solve_global() — Solve cross-template constraints
```

**Key file**: `src/layout/engine.rs:2222` — `resolve_constrain_statements_two_phase()`

### Where Anchor Constraints Fit

Anchor-based constraints are **always global** because they reference a template instance's anchor from outside that template. They execute in **Phase 4** when:
- Local constraints are already solved
- Rotation is already applied
- Anchor positions reflect post-rotation coordinates

**This means anchor resolution "just works" with rotation** — no special handling needed.

### Rotation Architecture

Rotation is done via **SVG group transforms** (commit `4a5e9eb`), NOT coordinate baking:
- The SVG renderer wraps rotated templates in `<g transform="rotate(...)">`
- But anchor positions ARE rotated in the layout engine (for connection routing)
- So `bar.drain_x` after rotation gives the correct global x-coordinate

See `examples/person-rotation.ail` for the rotation test case.

---

## ConstraintProperty::Copy Removal Impact

Adding `AnchorX(String)` and `AnchorY(String)` variants means `ConstraintProperty` can no longer derive `Copy` (String is not Copy).

### Places that access `ConstraintProperty` via `.node`:

All uses are `match prop_ref.property.node { ... }` pattern matches. These need to change to `match &prop_ref.property.node { ... }` (match by reference).

**Files affected**:
1. `src/layout/collector.rs:376,392,454` — `property_to_variable()` and midpoint handling
2. `src/layout/engine.rs:1566-1605` — anchor position resolution in `recompute_custom_anchors`
3. `src/layout/engine.rs:2686-2722` — same in the two-phase version
4. `src/layout/types.rs:49-54` — `AnchorDirection::from_property()`
5. `src/parser/grammar.rs` — test assertions using `matches!()`

### Fix Pattern

```rust
// Before (Copy):
match prop_ref.property.node {
    ConstraintProperty::Left => ...
}

// After (Clone, no Copy):
match &prop_ref.property.node {
    ConstraintProperty::Left => ...
    ConstraintProperty::AnchorX(name) => ...
    ConstraintProperty::AnchorY(name) => ...
}
```

---

## Implementation Strategy

### Step 1: AST Extension (`src/parser/ast.rs`)
- Add `AnchorX(String)` and `AnchorY(String)` variants
- Change derive from `Copy` to non-Copy (keep `Clone`)
- Update `from_str()` with `_x`/`_y` suffix fallback AFTER built-in properties
- Add unit tests for `from_str()`

### Step 2: Fix All Compilation Errors from Copy Removal
- Update all `match prop_ref.property.node` → `match &prop_ref.property.node`
- Add `AnchorX(_) | AnchorY(_)` arms to match blocks (most can use wildcard/panic for now)
- Files: `collector.rs`, `engine.rs`, `types.rs`

### Step 3: Anchor Resolution in Engine
- In `collect_constrain_statements` or a new helper in `engine.rs`:
  - When a constraint expression references an anchor property, look up the anchor position from `LayoutResult`
  - Convert `AnchorX(name)` → constant x value
  - Convert `AnchorY(name)` → constant y value
- This happens BEFORE constraints are passed to the kasuari solver

### Step 4: Error Handling
- Unknown anchor name → error with valid anchor list
- Anchor on element without anchors → clear error message

### Step 5: Tests
- Parse `drain_x` → `AnchorX("drain")`
- Parse `center_x` → `CenterX` (not `AnchorX("center")`)
- Integration test: template with custom anchor, constraint referencing it
- Integration test with rotation

---

## Design Decision Record

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Anchor refs resolve to constants | Yes | Anchors are pre-computed positions, not solver variables |
| Resolution happens in engine | Yes | Engine has LayoutResult access, collector doesn't |
| Copy trait removed | Yes | String in AnchorX/AnchorY prevents Copy |
| Suffix pattern `_x`/`_y` | Yes | User requested, consistent with `center_x`/`center_y` |
| Built-in properties take precedence | Yes | `center_x` → CenterX, not AnchorX("center") |
| No new LayoutProperty variants | Yes | Anchors resolve to constants, not solver variables |
