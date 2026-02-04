# Tasks: `--lint` Mode

**Feature**: 010-lint-mode
**Generated**: 2026-02-04
**Total tasks**: 14
**Parallel opportunities**: T003-T005 (3 checks), T006 independent of T003-T005

---

## Phase 1: Infrastructure

### T001: Add `--lint` CLI flag and thread through RenderConfig
**Files**: `src/main.rs`, `src/lib.rs`
**Depends on**: —
**Parallel**: No

1. In `src/main.rs`, add `#[arg(long)] lint: bool` to the `Cli` struct (after `trace`)
2. In `src/lib.rs`, add `pub lint: bool` to `RenderConfig` struct
3. Add `with_lint(self, lint: bool) -> Self` builder method on `RenderConfig`
4. In `main.rs`, pass `cli.lint` to config: `.with_lint(cli.lint)`
5. Change `render_with_config()` return type from `Result<String, RenderError>` to `Result<(String, Vec<layout::lint::LintWarning>), RenderError>`
6. In the pipeline (after `route_connections()`, before debug output), add:
   ```rust
   let lint_warnings = if config.lint {
       layout::lint::check(&result, &doc)
   } else {
       Vec::new()
   };
   ```
7. Return `Ok((svg, lint_warnings))` at the end
8. In `main.rs`, after receiving the result:
   - Print SVG to stdout
   - If lint warnings non-empty: print each to stderr in format `lint: {category}: {message}`, print summary line `lint: N warning(s)`, exit with code 1
   - If lint warnings empty and `--lint` was passed: print `lint: clean` to stderr

**Checkpoint**: `cargo build` succeeds. `--lint` flag accepted. Empty warnings returned.

---

### T002: Create lint engine core types and orchestration
**Files**: `src/layout/lint.rs` (NEW), `src/layout/mod.rs`
**Depends on**: T001
**Parallel**: No

1. Create `src/layout/lint.rs` with:
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

   impl std::fmt::Display for LintCategory { ... }  // "overlap", "containment", "label", "connection"
   ```
2. Add `pub fn check(result: &LayoutResult, doc: &Document) -> Vec<LintWarning>` — calls each checker, returns combined warnings
3. Add `fn element_display_name(elem: &ElementLayout, parent_name: Option<&str>, child_index: usize) -> String` — returns `"name"` for named elements, `<child #N of parent>` for anonymous
4. Stub out the 4 checker functions (empty bodies returning no warnings)
5. Add `pub mod lint;` to `src/layout/mod.rs`

**Checkpoint**: `cargo build` succeeds. `cargo test` passes (no regressions). `--lint` on any file prints `lint: clean`.

---

## Phase 2: Lint Checks (T003, T004, T005 are [P] parallelizable)

### T003: Implement overlap detection (FR2) [P]
**Files**: `src/layout/lint.rs`
**Depends on**: T002
**Parallel**: Yes — independent of T004, T005

Implement `fn check_overlaps(result: &LayoutResult, doc: &Document, warnings: &mut Vec<LintWarning>)`:

1. Build a `HashSet<String>` of all element IDs involved in `contains` constraints — scan `doc.statements` recursively for `ConstraintExpr::Contains`, collect both container IDs and contained element IDs
2. Walk the element tree recursively. For each element with children:
   - For each pair (i, j) where i < j among `element.children`:
     - Skip if either has `styles.opacity` set to `Some(v)` where `v < 1.0`
     - Skip if either element's ID (if named) is in the contains set
     - Skip if one is `ElementType::Shape(ShapeType::Text)` and the other is not
     - If `children[i].bounds.intersects(&children[j].bounds)`:
       - Compute overlap: `overlap_w = min(a.right(), b.right()) - max(a.x, b.x)`, same for height
       - Emit warning: `elements "{name_a}" and "{name_b}" overlap by {overlap_w:.0}x{overlap_h:.0}px`

**Checkpoint**: `cargo run -- --lint tests/lint-fixtures/true-positives.ail` reports overlap for solid_box_a/solid_box_b. `true-negatives.ail` reports zero overlap warnings.

---

### T004: Implement contains verification (FR3) [P]
**Files**: `src/layout/lint.rs`
**Depends on**: T002
**Parallel**: Yes — independent of T003, T005

Implement `fn check_contains(result: &LayoutResult, doc: &Document, warnings: &mut Vec<LintWarning>)`:

1. Walk `doc.statements` recursively, find all `ConstraintExpr::Contains { container, elements, padding }`
2. For each:
   - Look up container bounds via `result.get_element_by_name(&container.node.0)`
   - `pad = padding.unwrap_or(0.0)`
   - For each element in `elements`:
     - Look up element bounds via `result.get_element_by_name(&elem.node.0)`
     - Check 4 edges:
       - Left: `container.x > elem.x - pad` → overflow on left
       - Right: `container.right() < elem.right() + pad` → overflow on right
       - Top: `container.y > elem.y - pad` → overflow on top
       - Bottom: `container.bottom() < elem.bottom() + pad` → overflow on bottom
     - For each failing edge, emit warning: `element "{elem}" extends {amount:.0}px past {direction} edge of container "{container}"`

**Checkpoint**: `true-positives.ail` reports containment violation for `extra` past right edge of `container_box`. `true-negatives.ail` reports zero containment warnings.

---

### T005: Implement label overlap detection (FR4) [P]
**Files**: `src/layout/lint.rs`
**Depends on**: T002
**Parallel**: Yes — independent of T003, T004

Implement `fn check_labels(result: &LayoutResult, warnings: &mut Vec<LintWarning>)`:

1. Collect all labels into a `Vec<(String, BoundingBox, Option<f64>)>` (owner description, estimated bbox, parent opacity):
   - Walk elements recursively: for each with `label: Some(lbl)`:
     - Estimate width: `lbl.text.len() as f64 * 7.0`
     - Height: `14.0` (or `lbl.styles.font_size` if available)
     - Bbox centered on `lbl.position` (adjust for `TextAnchor`)
     - Parent opacity: `elem.styles.opacity`
   - Walk `result.connections`: for each with `label: Some(lbl)`:
     - Same estimation, owner = `"{from}→{to}"`
     - Parent opacity: None (connections don't have opacity relevant here)
2. For each pair (i, j) where i < j:
   - Skip if same owner element
   - Skip if either parent has `opacity < 1.0`
   - If `label_i.bbox.intersects(&label_j.bbox)`:
     - Emit warning: `labels "{text_a}" and "{text_b}" overlap`

**Checkpoint**: `true-positives.ail` reports label overlap for "Temperature"/"Pressure". `true-negatives.ail` reports zero label warnings.

---

### T006: Add line-segment-vs-bbox geometric primitive
**Files**: `src/layout/types.rs`
**Depends on**: T002
**Parallel**: Yes — independent of T003, T004, T005

Add to `impl BoundingBox`:
```rust
pub fn line_segment_intersects(&self, p1: &Point, p2: &Point) -> bool
```

Implementation — a segment intersects a bbox if:
- Either endpoint is inside the bbox (`self.contains(p1) || self.contains(p2)`), OR
- The segment crosses any of the 4 edges of the bbox

For edge intersection, use parametric line-segment intersection. Each bbox edge is itself a segment. Two segments intersect if their parametric parameters both fall in [0, 1].

Handle degenerate cases:
- Zero-length segment (point): use `self.contains(p1)`
- Axis-aligned segments: avoid division by zero in parametric calculation

Add unit tests in `types.rs` `#[cfg(test)]`:
- `test_segment_inside`: both endpoints inside → true
- `test_segment_outside`: both endpoints outside, no crossing → false
- `test_segment_crosses`: enters from left, exits right → true
- `test_segment_touches_corner`: passes exactly through corner → true
- `test_segment_parallel_to_edge`: runs along top edge → true or false (document choice)
- `test_zero_length_segment_inside`: point inside → true
- `test_zero_length_segment_outside`: point outside → false

**Checkpoint**: all unit tests pass.

---

### T007: Implement connection-element intersection (FR5)
**Files**: `src/layout/lint.rs`
**Depends on**: T006
**Parallel**: No (needs T006)

Implement `fn check_connections(result: &LayoutResult, warnings: &mut Vec<LintWarning>)`:

1. Collect all opaque elements into a flat list: walk element tree, collect `(id_or_path, BoundingBox)` for elements where `opacity` is `None` or `Some(1.0)`. Exclude `Text` shape types.
2. For each connection in `result.connections`:
   - Get source and target IDs (`connection.from_id`, `connection.to_id`)
   - For each path segment (consecutive pair in `connection.path`):
     - For each opaque element (excluding source and target by ID):
       - If `element.bounds.line_segment_intersects(&seg_start, &seg_end)`:
         - Record this (connection, element) pair
   - Deduplicate: only emit one warning per (connection, element) pair
   - For each pair: emit warning: `connection {from}→{to} crosses element "{element}"`

**Checkpoint**: `true-positives.ail` reports connection crossing for src→dst through blocker. `true-negatives.ail` reports zero connection warnings.

---

## Phase 3: Testing

### T008: Integration tests with fixture files [P]
**Files**: `tests/lint_integration.rs` (NEW)
**Depends on**: T003, T004, T005, T007
**Parallel**: Yes — with T009

Create `tests/lint_integration.rs`:

1. `test_true_positives_all_categories`: parse+render `tests/lint-fixtures/true-positives.ail` with lint enabled, assert warnings contain at least one of each category (overlap, containment, label, connection)
2. `test_true_negatives_clean`: parse+render `tests/lint-fixtures/true-negatives.ail` with lint enabled, assert zero warnings
3. `test_lint_disabled_no_warnings`: render any file without `--lint`, assert no warnings returned
4. `test_lint_output_format`: verify warning strings match `lint: {category}: {description}` pattern
5. `test_lint_summary_line`: verify `lint: N warning(s)` or `lint: clean` format

**Checkpoint**: `cargo test --test lint_integration` passes.

---

### T009: Unit tests for each lint check [P]
**Files**: `src/layout/lint.rs` (add `#[cfg(test)]` module)
**Depends on**: T003, T004, T005, T007
**Parallel**: Yes — with T008

Add `#[cfg(test)] mod tests` in `lint.rs`. Build `LayoutResult` and `Document` fixtures programmatically (no file parsing needed):

1. **Overlap tests**:
   - `test_overlap_detected`: two rects, full opacity, overlapping → 1 warning
   - `test_overlap_skipped_opacity`: one rect opacity 0.5 → 0 warnings
   - `test_overlap_skipped_contains`: both in contains set → 0 warnings
   - `test_overlap_skipped_text_on_shape`: text on rect → 0 warnings
   - `test_no_overlap_different_containers`: rects in different groups → 0 warnings

2. **Contains tests**:
   - `test_contains_satisfied`: container wraps element → 0 warnings
   - `test_contains_violated_right`: element extends past right → 1 warning mentioning "right"
   - `test_contains_violated_multiple_edges`: element extends in two directions → 2 warnings

3. **Label tests**:
   - `test_label_overlap`: two labels at same position → 1 warning
   - `test_label_no_overlap`: labels far apart → 0 warnings
   - `test_label_skip_opacity_parent`: label on opacity<1 element → 0 warnings

4. **Connection tests**:
   - `test_connection_crosses_element`: path through opaque rect → 1 warning
   - `test_connection_skips_zone`: path through opacity<1 → 0 warnings
   - `test_connection_skips_endpoints`: path starts/ends at element → 0 warnings

5. **Display name tests**:
   - `test_named_element_display`: returns `"foo"`
   - `test_anonymous_element_display`: returns `<child #2 of group_a>`

**Checkpoint**: `cargo test` all pass including new tests.

---

## Phase 4: Documentation

### T010: Update skill doc with --lint flag
**Files**: `docs/skill.md`
**Depends on**: T007
**Parallel**: [P] with T011

1. Add `--lint` to the options summary near the top of the file (if there's a flags table)
2. In the Self-Assessment Checklist section, add a step before the adversarial review: "Run `agent-illustrator --lint diagram.ail` and fix all warnings before proceeding to adversarial review"
3. In the Adversarial Review section, add a note: "If `--lint` is available, run it first. It catches mechanical defects (overlaps, containment violations, connection crossings) instantly and deterministically. The adversarial review subagent should then focus only on subjective layout quality."

**Checkpoint**: docs read correctly, no syntax errors.

---

### T011: Update TODO.md — mark --lint as in progress [P]
**Files**: `TODO.md`
**Depends on**: T001
**Parallel**: [P] with T010

Mark the `--lint` item in TODO.md as implemented (or remove it / move to completed section).

**Checkpoint**: TODO.md reflects current state.

---

## Phase 5: Final validation

### T012: Full regression test suite
**Files**: —
**Depends on**: T008, T009
**Parallel**: No

1. Run `cargo test` — all existing tests must pass (269 unit, 84 integration, etc.)
2. Run `cargo clippy` — no new warnings
3. Run `cargo fmt --check` — properly formatted
4. Verify `--lint` on the IoT edge architecture example (`/tmp/na1/iot-edge-architecture.ail`) produces zero false positives

**Checkpoint**: clean test suite, clean clippy, zero false positives on real-world diagram.

---

### T013: Verify exit code behavior
**Files**: —
**Depends on**: T012
**Parallel**: No

1. `agent-illustrator --lint tests/lint-fixtures/true-positives.ail > /dev/null 2>/dev/null; echo $?` → should be `1`
2. `agent-illustrator --lint tests/lint-fixtures/true-negatives.ail > /dev/null 2>/dev/null; echo $?` → should be `0`
3. `agent-illustrator tests/lint-fixtures/true-positives.ail > /dev/null 2>/dev/null; echo $?` → should be `0` (no lint = no warnings = success)
4. Verify SVG is still produced on stdout even when lint returns warnings

**Checkpoint**: exit codes correct in all 4 cases.

---

### T014: Copy IoT fixture for regression
**Files**: `tests/lint-fixtures/iot-edge-architecture.ail` (NEW)
**Depends on**: T012
**Parallel**: No

Copy `/tmp/na1/iot-edge-architecture.ail` to `tests/lint-fixtures/` and add an integration test that verifies it produces zero lint warnings. This is the canonical real-world false-positive regression test.

**Checkpoint**: `cargo test` includes IoT fixture test, passes.

---

## Dependency Graph

```
T001 (CLI flag)
  ↓
T002 (core types)
  ↓
  ├── T003 (overlap)     [P]
  ├── T004 (contains)    [P]
  ├── T005 (labels)      [P]
  └── T006 (segment primitive) [P]
        ↓
      T007 (connections)
        ↓
  ┌─────┴─────┐
T008 (integ)  T009 (unit)  [P]
  └─────┬─────┘
      T010 (skill doc) [P]
      T011 (TODO.md)   [P]
        ↓
      T012 (regression suite)
        ↓
      T013 (exit codes)
        ↓
      T014 (IoT fixture)
```

## Parallel Execution Examples

**Batch 1** (sequential): T001 → T002
**Batch 2** (parallel): T003 + T004 + T005 + T006
**Batch 3** (sequential): T007
**Batch 4** (parallel): T008 + T009
**Batch 5** (parallel): T010 + T011
**Batch 6** (sequential): T012 → T013 → T014
