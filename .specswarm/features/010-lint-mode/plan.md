# Implementation Plan: `--lint` Mode

## Technical Context

- **Language**: Rust (edition 2021)
- **No new dependencies** — all checks use existing data structures from the layout engine
- **Key crate**: `kasuari` (Cassowary constraint solver) — already in use
- **Pipeline insertion point**: `src/lib.rs` line ~346, after `route_connections()`, before debug output and SVG rendering
- **Existing utilities**: `BoundingBox::intersects()` already exists in `layout/types.rs`

## Constitution Check

| Principle | Status | Notes |
|-----------|--------|-------|
| Semantic over Geometric | OK | Lint validates semantic intent (contains, connections) |
| First-Attempt Correctness | OK | Lint helps agents achieve correctness faster |
| Explicit over Implicit | OK | Only checks explicitly declared constraints |
| Fail Fast, Fail Clearly | OK | Structured warnings on stderr with categories |
| Composability | OK | Each lint check is independent |
| Don't reinvent | OK | No new dependencies, reuses existing layout data |

## Architecture Overview

```
main.rs                    Add --lint flag to Cli struct
    ↓
lib.rs                     Thread lint flag through RenderConfig
    ↓                      Insert lint pass after route_connections()
layout/lint.rs  (NEW)      Core lint engine — all 4 check categories
    ↓
layout/types.rs            Add line_segment_intersects_bbox() to BoundingBox
                           Add element_display_name() helper
```

One new file (`layout/lint.rs`), small additions to 3 existing files.

## Implementation Phases

### Phase 1: Infrastructure (CLI + plumbing)

**Files**: `src/main.rs`, `src/lib.rs`

1. Add `lint: bool` field to `Cli` struct in `main.rs` (alongside existing `debug`, `trace`)
2. Add `lint: bool` to `RenderConfig` in `lib.rs`, with `with_lint()` builder method
3. Thread `cli.lint` through `RenderConfig::new().with_lint(cli.lint)`
4. After `route_connections()` call in `render_with_config()`, add:
   ```
   if config.lint {
       let warnings = layout::lint::check(&result, &doc);
       // print warnings to stderr, set exit code
   }
   ```
5. Return lint warning count from `render_with_config()` (modify return type to include it, or use a separate channel — simplest: return `(String, Vec<LintWarning>)`)
6. In `main.rs`, if warnings non-empty: print to stderr in FR6 format, exit with code 1

**Exit code design**: Currently `main()` calls `process::exit(1)` on errors. For lint, the SVG is valid output but exit code should be 1 if warnings exist. Change main to: render → print SVG to stdout → print lint warnings to stderr → exit(1) if warnings.

### Phase 2: Lint engine core (`layout/lint.rs`)

**New file**: `src/layout/lint.rs`

Define the lint types and orchestration:

```rust
pub struct LintWarning {
    pub category: LintCategory,
    pub message: String,
}

pub enum LintCategory {
    Overlap,
    Containment,
    Label,
    Connection,
}

pub fn check(result: &LayoutResult, doc: &Document) -> Vec<LintWarning> {
    let mut warnings = Vec::new();
    check_overlaps(result, doc, &mut warnings);
    check_contains(result, doc, &mut warnings);
    check_labels(result, &mut warnings);
    check_connections(result, &mut warnings);
    warnings
}
```

**Element display name helper**: For anonymous elements, walk the element tree to produce a positional path like `<child #3 of group_a>`. Named elements use their ID. Add this as a function in `lint.rs` or on `ElementLayout`.

### Phase 3: FR2 — Overlap detection

**In `layout/lint.rs`**

`check_overlaps()` walks the element tree recursively. For each container's direct children:

1. Collect the set of `contains` target IDs from the document's constraint statements (scan `doc.statements` for `ConstraintExpr::Contains`, collect all element IDs mentioned in any `elements` list, plus the container IDs)
2. For each pair of siblings (i, j) where i < j:
   - Skip if either element has `opacity < 1.0` in its resolved styles
   - Skip if either element's ID is in the `contains` target set
   - Skip if either element is a `Text` shape type and the other is not
   - Check `elem_i.bounds.intersects(&elem_j.bounds)`
   - If true: compute overlap dimensions, emit `LintWarning`

**Overlap dimensions**: `overlap_w = min(a.right(), b.right()) - max(a.x, b.x)`, same for height.

### Phase 4: FR3 — Contains constraint verification

**In `layout/lint.rs`**

`check_contains()` iterates the document's constraint statements:

1. For each `ConstraintExpr::Contains { container, elements, padding }`:
   - Look up `container` in `result.elements` (by name)
   - For each element in `elements`:
     - Look up element in `result.elements`
     - Check: `container.bounds.x <= element.bounds.x - padding`
     - Check: `container.bounds.right() >= element.bounds.right() + padding`
     - Check: `container.bounds.y <= element.bounds.y - padding`
     - Check: `container.bounds.bottom() >= element.bounds.bottom() + padding`
   - For each failing check, emit a `LintWarning` with direction and overflow amount

### Phase 5: FR4 — Label overlap detection

**In `layout/lint.rs`**

`check_labels()` collects all labels with their estimated bounding boxes, then checks pairs:

1. Walk all elements recursively, collect `(element_id, label_bbox)` for each element that has a label
2. Walk all connections, collect `(connection_desc, label_bbox)` for each connection with a label
3. For label bbox estimation: `BoundingBox { x: label.position.x - width/2, y: label.position.y - height/2, width: text.len() * 7.0, height: 14.0 }` (adjusted for TextAnchor)
4. For each pair of labels:
   - Skip if both belong to the same element
   - Skip if one label's parent element has `opacity < 1.0`
   - Check `label_a_bbox.intersects(&label_b_bbox)`
   - If true: emit `LintWarning` with both label texts

### Phase 6: FR5 — Connection-element intersection

**In `layout/lint.rs` and `layout/types.rs`**

First, add `line_segment_intersects_bbox()` to `BoundingBox` in `types.rs`:
- Takes two `Point`s (segment start, end)
- Returns `bool`
- Implementation: check if segment intersects any of the 4 edges of the bbox, OR if either endpoint is inside the bbox
- Standard line-segment vs AABB algorithm

Then `check_connections()`:
1. Collect all element bounding boxes into a flat list with IDs (excluding elements with `opacity < 1.0`)
2. For each connection in `result.connections`:
   - For each path segment (consecutive pair of waypoints):
     - For each element (excluding source and target of this connection):
       - Check `element.bounds.line_segment_intersects(&segment_start, &segment_end)`
       - If true: emit `LintWarning`
   - Deduplicate: only report each (connection, element) pair once even if multiple segments intersect

### Phase 7: Tests

**Test files**: unit tests in `layout/lint.rs`, integration tests using fixture files

1. **Unit tests** (in `layout/lint.rs` `#[cfg(test)]` module):
   - `test_overlap_detected`: two overlapping sibling rects → warning
   - `test_overlap_skipped_for_opacity`: rect with opacity 0.5 overlapping → no warning
   - `test_overlap_skipped_for_contains_target`: contains target overlapping container → no warning
   - `test_overlap_skipped_for_text_on_shape`: text element on rect → no warning
   - `test_contains_satisfied`: proper contains → no warning
   - `test_contains_violated`: broken contains → warning with direction
   - `test_label_overlap_detected`: two labels at same position → warning
   - `test_connection_through_element`: connection crossing opaque element → warning
   - `test_connection_crossing_zone_bg`: connection crossing opacity < 1.0 → no warning
   - `test_line_segment_intersects_bbox`: geometric primitive correctness

2. **Integration tests** using the fixture files:
   - `tests/lint-fixtures/true-positives.ail` → all 4 warning categories triggered
   - `tests/lint-fixtures/true-negatives.ail` → zero warnings
   - Test the stderr output format matches FR6 specification

3. **Existing test suite**: run full `cargo test` to verify no regressions

### Phase 8: Documentation

1. Update `docs/skill.md` — mention `--lint` in the self-assessment checklist section (agents should run lint before adversarial review)
2. Update `docs/grammar.md` — no changes needed (lint is runtime, not syntax)
3. Add `--lint` to the options table in the skill
4. Update EBNF grammar? — No, `--lint` is a CLI flag, not language syntax

## Dependency Order

```
Phase 1 (infrastructure) → Phase 2 (core types)
                              ↓
                 ┌────────────┼────────────┐
                 ↓            ↓            ↓
             Phase 3      Phase 4      Phase 5
             (overlap)    (contains)   (labels)
                 ↓
             Phase 6 (connections — needs line_segment_intersects from types.rs)
                 ↓
             Phase 7 (tests)
                 ↓
             Phase 8 (docs)
```

Phases 3, 4, 5 are independent and can be implemented in parallel.

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Opacity heuristic suppresses valid warnings | Low | Documented in assumptions; can add `--lint-strict` later |
| Label bbox estimation inaccurate | Low | Known limitation; 7px/char is conservative enough |
| Line-segment intersection edge cases | Medium | Thorough unit tests for degenerate cases (vertical/horizontal lines, zero-length segments) |
| Performance on large diagrams | Low | O(n²) sibling pairs per container, O(c*n) for connections — well under 100ms for 50 elements |
