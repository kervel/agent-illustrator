# Implementation Plan: Anchor-Based Constraints

**Feature**: 011-anchor-based-constraints
**Created**: 2026-01-28
**Status**: Planning
**Research**: See [research.md](research.md) for detailed architectural analysis

---

## Technical Context

### Tech Stack (from `.specswarm/tech-stack.md`)

| Technology | Purpose |
|------------|---------|
| Rust (Edition 2021) | Language implementation |
| kasuari | Cassowary constraint solver |
| thiserror | Error type definitions |
| ariadne | Diagnostic messages |

### Relevant Existing Code

| Component | File:Line | Purpose |
|-----------|-----------|---------|
| ConstraintProperty enum | `src/parser/ast.rs:453-469` | Property names parsed from source text |
| from_str() parsing | `src/parser/ast.rs:473-488` | String → ConstraintProperty |
| LayoutProperty enum | `src/layout/solver.rs:21-34` | Kasuari solver variable types |
| property_to_variable() | `src/layout/collector.rs:450-470` | Bridge: ConstraintProperty → LayoutVariable |
| AnchorSet | `src/layout/types.rs:122-260` | Collection of named anchors with positions |
| Anchor | `src/layout/types.rs:98-118` | Named anchor with position and direction |
| ElementLayout.anchors | `src/layout/types.rs:615` | Anchors attached to elements |
| Two-phase solver | `src/layout/engine.rs:2222-2400` | Local→rotation→global constraint solving |
| solve_global() | `src/layout/engine.rs:641-765` | Phase 4: cross-template constraints |
| solve_local() | `src/layout/engine.rs:354-384` | Phase 1: per-template constraints |

### Two-Phase Constraint Solver Architecture (Feature 010)

The constraint solver works in phases (see `resolve_constrain_statements_two_phase`):

```
Phase 1: solve_local()        — Solve constraints within each template instance
Phase 2: apply_rotation()     — Rotate bounds AND anchors for rotated templates
Phase 2b:                      — Handle templates with rotation but no constraints
Phase 3: apply_local_results() — Write local results back to LayoutResult
Phase 4: solve_global()        — Solve cross-template constraints (POST-rotation)
```

**Rotation approach** (commit `4a5e9eb`): SVG uses `<g transform="rotate(...)">` for visual
rotation, but anchor positions are mathematically rotated in the layout engine for correct
connection routing and constraint evaluation.

### Two-Layer Type System

```
AST Layer (parser)                    Solver Layer (layout)
─────────────────                     ───────────────────
ConstraintProperty                    LayoutProperty
  X, Y, Width, Height                  X, Y, Width, Height
  Left, Right, Top, Bottom             CenterX, CenterY
  CenterX, CenterY, Center            Right, Bottom
  AnchorX(String)  ← NEW
  AnchorY(String)  ← NEW

          │ property_to_variable() │
          └─────────┬──────────────┘
                    ↓
            LayoutVariable(element_id, LayoutProperty)
```

**Critical insight**: `AnchorX`/`AnchorY` cannot map to `LayoutProperty` because anchor
positions are pre-computed constants, not derived expressions over solver variables.
They must resolve to **constant values** at constraint collection time.

See [research.md](research.md) for full analysis.

---

## Constitution Check

| Principle | Alignment | Notes |
|-----------|-----------|-------|
| Semantic Over Geometric | ✓ | Users reference anchors by name, not coordinates |
| First-Attempt Correctness | ✓ | Clear `element.anchor_x` syntax, unambiguous |
| Explicit Over Implicit | ✓ | Anchor references are explicit in constraint syntax |
| Fail Fast, Fail Clearly | ✓ | Unknown anchors produce errors with valid anchor list |
| Composability | ✓ | Works with existing constraint expressions |
| Don't Reinvent Wheel | ✓ | Extends existing kasuari-based solver, no new dependencies |

---

## Implementation Phases

### Phase 1: Extend ConstraintProperty Enum

**Goal**: Add anchor coordinate variants to the AST

**Files**: `src/parser/ast.rs`

**Changes**:
1. Add `AnchorX(String)` and `AnchorY(String)` variants to `ConstraintProperty`
2. Remove `Copy` from derive (String prevents Copy), keep `Clone`
3. Update `from_str()` with `_x`/`_y` suffix fallback AFTER all built-in properties

**Key Logic**:
```rust
#[derive(Debug, Clone, PartialEq, Eq)]  // Note: no Copy
pub enum ConstraintProperty {
    // ... existing variants ...
    /// X-coordinate of a named anchor (e.g., "drain" from "drain_x")
    AnchorX(String),
    /// Y-coordinate of a named anchor (e.g., "gate" from "gate_y")
    AnchorY(String),
}

impl ConstraintProperty {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            // ALL built-in properties FIRST (order matters!):
            "x" => Some(Self::X),
            "y" => Some(Self::Y),
            "width" => Some(Self::Width),
            "height" => Some(Self::Height),
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            "top" => Some(Self::Top),
            "bottom" => Some(Self::Bottom),
            "center_x" | "horizontal_center" => Some(Self::CenterX),
            "center_y" | "vertical_center" => Some(Self::CenterY),
            "center" => Some(Self::Center),
            // Anchor fallback — ONLY reached if not a built-in
            _ if s.ends_with("_x") => {
                let anchor_name = &s[..s.len() - 2];
                Some(Self::AnchorX(anchor_name.to_string()))
            }
            _ if s.ends_with("_y") => {
                let anchor_name = &s[..s.len() - 2];
                Some(Self::AnchorY(anchor_name.to_string()))
            }
            _ => None,
        }
    }
}
```

### Phase 2: Fix Copy Removal Fallout

**Goal**: Fix all compilation errors from removing `Copy` trait

**Files**: `src/layout/collector.rs`, `src/layout/engine.rs`, `src/layout/types.rs`, `src/parser/grammar.rs`

**Pattern**: Change all `match prop_ref.property.node { ... }` to `match &prop_ref.property.node { ... }`

**Locations to fix** (from grep):
1. `collector.rs:376,392` — midpoint property matching
2. `collector.rs:454` — `property_to_variable()` main match
3. `engine.rs:1566-1605` — `recompute_custom_anchors` (old single-phase)
4. `engine.rs:2686-2722` — `recompute_custom_anchors` (two-phase)
5. `types.rs:49-54` — `AnchorDirection::from_property()`
6. `grammar.rs` — test assertions using `matches!()`

**Each match block** must also add `AnchorX(_) | AnchorY(_)` arms.

### Phase 3: Anchor Resolution in Engine

**Goal**: Resolve anchor references to constant values during constraint collection

**Files**: `src/layout/engine.rs`, `src/layout/collector.rs`

**Architecture Decision**: Anchor resolution happens in the engine, NOT in the collector,
because the engine has access to `LayoutResult` (where anchor positions live).

**Approach**: When the engine processes constraints containing anchor references:
1. Look up the element in `LayoutResult`
2. Find the anchor by name in `element.anchors`
3. Extract `anchor.position.x` or `anchor.position.y`
4. Replace the anchor reference with a constant value in the constraint

**In collector** (`property_to_variable`): For `AnchorX`/`AnchorY`, produce a special
`LayoutVariable` that the engine can detect and resolve. Options:
- Use a sentinel `LayoutProperty` (e.g., `LayoutProperty::X` with a marker)
- Or: handle anchor constraints in a separate pass in the engine

**Preferred approach**: In the collector's `property_to_variable()`, anchor properties
map to `LayoutProperty::X` (for AnchorX) or `LayoutProperty::Y` (for AnchorY) with the
element ID set to the full `element_name`. Then in the engine, before adding constraints
to the solver, detect anchor refs and convert them to Fixed constraints using the looked-up
anchor position value.

Actually, the cleanest approach: **modify the collector's `collect_constrain` method** to
accept an optional `LayoutResult` reference. When an anchor ref is encountered on the
right side, resolve it to a constant. This converts:

```
constrain foo.center_x = bar.drain_x
```

Into:

```
LayoutConstraint::Fixed { variable: foo.CenterX, value: 150.0 }
// (where 150.0 is bar.drain anchor's x position)
```

### Phase 4: Error Handling

**Goal**: Clear error messages for anchor-related issues

**New error cases**:
1. "Unknown anchor 'xyz' on element 'foo'. Available anchors: top, bottom, left, right, drain, gate"
2. "Element 'bar' not found" (existing error, but may surface more with anchor refs)

**Files**: `src/error.rs`

### Phase 5: Tests & Validation

**Goal**: Verify correctness including rotation integration

**Test Cases**:
1. **Parser**: `from_str("drain_x")` → `AnchorX("drain")`
2. **Parser**: `from_str("center_x")` → `CenterX` (NOT `AnchorX("center")`)
3. **Parser**: `from_str("left_conn_x")` → `AnchorX("left_conn")`
4. **Integration**: Template with custom anchor, constraint using `element.anchor_x`
5. **Integration**: Two template instances, cross-anchor constraint
6. **Rotation**: Rotated template + anchor constraint → correct post-rotation position
7. **Regression**: `person-rotation.ail` example still renders correctly
8. **Error**: Unknown anchor name → clear error

---

## File Change Summary

| File | Change Type | Description |
|------|-------------|-------------|
| `src/parser/ast.rs` | Modify | Add `AnchorX(String)`, `AnchorY(String)` variants; remove `Copy`; update `from_str()` |
| `src/layout/collector.rs` | Modify | Handle anchor property refs in `property_to_variable()`, anchor resolution in `collect_constrain()` |
| `src/layout/engine.rs` | Modify | Fix `match` blocks for Copy removal; add anchor ref arms |
| `src/layout/types.rs` | Modify | Fix `AnchorDirection::from_property()` for new variants |
| `src/parser/grammar.rs` | Modify | Fix test assertions for Copy removal |
| `src/error.rs` | Possibly modify | Add anchor-specific error variant if needed |
| `examples/mosfet-driver.ail` | Modify | Use anchor-based alignment for flyback diode |

---

## Dependencies

| Dependency | Status | Notes |
|------------|--------|-------|
| Feature 005 (Constraint Solver) | ✓ Complete | Base constraint system |
| Feature 009 (Anchor Support) | ✓ Complete | Anchors on elements, AnchorSet |
| Feature 010 (Local/Global Solver) | ✓ Complete | Two-phase solver, rotation support |

---

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| `Copy` removal cascade | Medium | Systematic `match` → `match &` fix across 6 files |
| Parser conflicts | Low | `_x`/`_y` suffixes checked AFTER all built-in properties |
| Chicken-and-egg: anchors depend on positions | None | Two-phase solver ensures anchors are resolved before global constraints |
| Rotation interaction | None | Phase 4 global constraints use post-rotation anchor positions |
| Performance | None | Anchor lookup is O(1) HashMap, no new solver variables |

---

## Success Criteria

- [ ] `constrain foo.center_x = bar.drain_x` parses correctly
- [ ] `center_x` still parses as `CenterX` (not `AnchorX("center")`)
- [ ] Anchor coordinates resolve from element's anchor set
- [ ] Anchor refs work with rotated template instances (post-rotation coordinates)
- [ ] Clear error for unknown anchor names
- [ ] Existing constraint tests still pass (all 365+ tests)
- [ ] `person-rotation.ail` example still renders correctly
- [ ] MOSFET example updated to use anchor-based alignment
