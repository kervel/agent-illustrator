# Implementation Plan: Layout and Render Pipeline

## Overview

| Attribute | Value |
|-----------|-------|
| Feature | 003 - Layout and Render Pipeline |
| Branch | `003-layout-render-pipeline` |
| Spec | [spec.md](./spec.md) |
| Status | Ready for Implementation |

---

## Technical Context

### Language & Framework
- **Language**: Rust (2021 edition)
- **Build**: Cargo
- **Existing deps**: logos, chumsky, ariadne, thiserror, insta, pretty_assertions

### New Dependencies Required
- None (pure Rust implementation, direct SVG string generation)

### Architecture
- **Input**: `Document` AST (from existing parser)
- **Output**: SVG string
- **Intermediate**: `LayoutResult` (positions + routes)

---

## Constitution Check

| Principle | Status | Notes |
|-----------|--------|-------|
| 1. Semantic Over Geometric | Pass | Layout engine handles coordinates, DSL stays semantic |
| 2. First-Attempt Correctness | Pass | Deterministic layout, predictable output |
| 3. Explicit Over Implicit | Pass | Documented defaults in LayoutConfig |
| 4. Fail Fast, Fail Clearly | Pass | LayoutError with spans and suggestions |
| 5. Composability | Pass | Nested layouts, modular pipeline |
| 6. Don't Reinvent | Pass | Custom for simplicity; Graphviz option later |

---

## Tech Stack Compliance Report

### Approved Technologies (already in stack)
- Rust (stable, 2021 edition)
- thiserror (error handling)
- insta (snapshot testing)

### New Technologies
- None (using direct SVG string generation per research.md)

### Prohibited Check
- No C dependencies ✓
- No unsafe code planned ✓
- No .unwrap() in library code ✓

---

## Implementation Phases

### Phase 1: Core Data Structures

**Goal**: Define the intermediate representation types.

**Files to create**:
- `src/layout/mod.rs` - Module root
- `src/layout/types.rs` - BoundingBox, Point, LayoutResult, ElementLayout, ConnectionLayout
- `src/layout/config.rs` - LayoutConfig with defaults
- `src/layout/error.rs` - LayoutError types

**Tests**:
- Unit tests for BoundingBox operations (contains, intersects, union)
- Default config values

**Acceptance**:
- All types compile
- Tests pass

---

### Phase 2: Reference Validation

**Goal**: Validate that all identifier references resolve.

**Files to modify**:
- `src/layout/mod.rs` - Add validate_references()

**Algorithm**:
1. Walk AST, collect all defined identifiers
2. Walk AST, check all referenced identifiers
3. For undefined: compute Levenshtein suggestions
4. Return LayoutError::UndefinedIdentifier with span

**Tests**:
- Valid document passes
- Missing reference detected with span
- Suggestions provided for typos

**Acceptance**:
- `a -> b` where `b` undefined produces clear error
- Error includes source span and "did you mean?" suggestions

---

### Phase 3: Basic Layout Engine

**Goal**: Compute positions for shapes and layout containers.

**Files to create**:
- `src/layout/engine.rs` - Layout computation

**Algorithm**:
```
fn layout_element(stmt, position, config) -> ElementLayout:
    match stmt:
        Shape → compute size from defaults/label, place at position
        Layout(Row) → layout children horizontally, return container bounds
        Layout(Column) → layout children vertically
        Layout(Grid) → layout in grid pattern
        Layout(Stack) → all children at same position
        Group → layout children (default: column), add group wrapper
```

**Tests**:
- Single shape gets default size
- Row arranges children horizontally
- Column arranges children vertically
- Nested layouts compute correctly

**Acceptance**:
- `row { rect a rect b }` produces two rects side by side
- Bounds contain all children

---

### Phase 4: Constraint Resolution

**Goal**: Apply position constraints after initial layout.

**Files to modify**:
- `src/layout/engine.rs` - Add constraint resolution

**Algorithm**:
1. Build constraint graph
2. Detect cycles (error if found)
3. Topological sort
4. Apply constraints in order
5. Detect conflicts (error if found)

**Tests**:
- `place a below b` moves a below b
- Circular constraint detected
- Conflicting constraints detected

**Acceptance**:
- Constraints modify positions correctly
- Conflicts produce clear errors per clarification

---

### Phase 5: Connection Routing

**Goal**: Compute paths between connected elements.

**Files to create**:
- `src/layout/routing.rs` - Connection routing

**Algorithm**:
1. For each connection:
   a. Get source and target bounds
   b. Determine attachment edges (based on relative position)
   c. Create orthogonal path (L-shape or direct)
   d. Store path in ConnectionLayout

**Tests**:
- Horizontal connection (left-to-right)
- Vertical connection (top-to-bottom)
- Diagonal needs L-route

**Acceptance**:
- Connections have valid paths
- No paths through element centers

---

### Phase 6: SVG Renderer

**Goal**: Generate SVG from LayoutResult.

**Files to create**:
- `src/renderer/mod.rs` - Module root
- `src/renderer/svg.rs` - SVG generation
- `src/renderer/config.rs` - SvgConfig

**Components**:
- SvgBuilder - accumulates elements
- Shape renderers (rect, circle, ellipse, polygon)
- Connection renderer with arrow markers
- Label renderer
- ViewBox computation

**Tests**:
- Single rect produces valid SVG
- Connection has marker-end
- CSS classes applied correctly

**Acceptance**:
- Valid SVG output
- Human-readable formatting
- Semantic CSS classes per tech-stack.md

---

### Phase 7: Pipeline Integration

**Goal**: Single entry point from DSL to SVG.

**Files to modify**:
- `src/lib.rs` - Export render functions
- `src/main.rs` - CLI usage

**API**:
```rust
pub fn render(source: &str) -> Result<String, RenderError>;
pub fn render_with_config(source: &str, config: RenderConfig) -> Result<String, RenderError>;
```

**Tests**:
- End-to-end integration tests
- Snapshot tests for example diagrams

**Acceptance**:
- Single function call produces SVG
- Errors have full context

---

### Phase 8: Error Enhancement

**Goal**: Polish error messages with suggestions.

**Files to modify**:
- `src/layout/error.rs` - Enhance messages
- `src/error.rs` - Unify error chain

**Enhancements**:
- Levenshtein suggestions for typos
- Context from parser spans
- ariadne-style rendering

**Tests**:
- Error messages include source snippets
- Suggestions are relevant

**Acceptance**:
- Errors are actionable per constitution principle 4

---

## File Structure (Final)

```
src/
├── lib.rs                    # Exports: parse, render, Document
├── main.rs                   # CLI (hello world → real usage)
├── error.rs                  # RenderError unifying parse/layout/render
├── parser/
│   ├── mod.rs
│   ├── lexer.rs
│   ├── grammar.rs
│   └── ast.rs
├── layout/
│   ├── mod.rs                # pub use, validate_references, compute
│   ├── types.rs              # BoundingBox, Point, LayoutResult, etc.
│   ├── config.rs             # LayoutConfig
│   ├── engine.rs             # Layout algorithm
│   ├── routing.rs            # Connection routing
│   └── error.rs              # LayoutError
└── renderer/
    ├── mod.rs                # pub use, render_svg
    ├── svg.rs                # SvgBuilder, element renderers
    └── config.rs             # SvgConfig
```

---

## Testing Strategy

### Unit Tests
- BoundingBox operations
- Routing edge cases
- Config defaults

### Integration Tests
- End-to-end DSL → SVG
- Error scenarios

### Snapshot Tests
- Example diagrams (visual regression)
- Error message formatting

### Test Files
```
tests/
├── integration_tests.rs      # Existing + new render tests
├── layout_tests.rs           # Layout-specific tests
└── snapshots/
    ├── simple_rect.svg
    ├── row_layout.svg
    ├── nested_layout.svg
    └── connections.svg
```

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Complex constraint conflicts | Start simple, defer edge cases |
| Connection routing complexity | Orthogonal-only for v1 |
| SVG browser compatibility | Test in Chrome, Firefox, Safari |
| Performance with 50 elements | Profile if issues arise |

---

## Dependencies

```
Phase 1 (types) → Phase 2 (validation) → Phase 3 (layout) → Phase 4 (constraints)
                                                        ↓
                                               Phase 5 (routing) → Phase 6 (renderer) → Phase 7 (integration)
                                                                                                    ↓
                                                                                           Phase 8 (errors)
```

---

## Next Steps

1. Run `/specswarm:tasks` to generate tasks.md
2. Begin Phase 1 implementation
3. Commit after each phase with passing tests

---

*Created: 2026-01-23*
