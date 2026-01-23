# Tasks: AIL Format Ergonomics

<!-- Tech Stack Validation: PASSED -->
<!-- Validated against: .specswarm/tech-stack.md -->
<!-- No prohibited technologies found -->
<!-- No new dependencies required -->

## Overview

| Attribute | Value |
|-----------|-------|
| Feature | 004 - AIL Format Ergonomics |
| Branch | `004-ail-format-ergonomics` |
| Total Tasks | 35 |
| Estimated Phases | 11 |

---

## User Story Mapping

| Story | Description | Priority | Phase |
|-------|-------------|----------|-------|
| US1 | Cross-Hierarchy Alignment | P1 | 3-6 |
| US2 | Unified Label Handling | P2 | 7 |
| US3 | Connection Label References | P3 | 8 |
| US4 | Position Relative to Alignment | P4 | 9 |
| US5 | Better Defaults (validation) | P5 | 11 |

---

## Phase 1: Setup & Foundational AST Types

**Goal**: Define all new AST types needed for the feature.

**Checkpoint**: All new types compile, unit tests pass.

### Tasks

- [ ] **T001** [Setup] Verify worktree is clean and tests pass
  - File: N/A (run `cargo test`)
  - Run existing test suite to establish baseline
  - Acceptance: All existing tests pass

- [ ] **T002** [US1] Add `ElementPath` struct to AST
  - File: `src/parser/ast.rs`
  - Add struct with `segments: Vec<Spanned<Identifier>>`
  - Add helper methods: `simple()`, `leaf()`, `is_simple()`, `to_string()`
  - Acceptance: Type compiles, can construct programmatically

- [ ] **T003** [US1] [P] Add `Edge` enum to AST
  - File: `src/parser/ast.rs`
  - Add variants: Left, HorizontalCenter, Right, Top, VerticalCenter, Bottom
  - Add helper methods: `is_horizontal()`, `is_vertical()`, `axis()`
  - Acceptance: Type compiles with all variants

- [ ] **T004** [US1] [P] Add `Axis` enum to AST
  - File: `src/parser/ast.rs`
  - Add variants: Horizontal, Vertical
  - Acceptance: Type compiles

- [ ] **T005** [US1] Add `AlignmentAnchor` struct to AST
  - File: `src/parser/ast.rs`
  - Fields: `element: Spanned<ElementPath>`, `edge: Spanned<Edge>`
  - Acceptance: Type compiles

- [ ] **T006** [US1] Add `AlignmentDecl` struct to AST
  - File: `src/parser/ast.rs`
  - Field: `anchors: Vec<AlignmentAnchor>`
  - Add `is_valid()` and `axis()` methods
  - Acceptance: Type compiles with validation logic

- [ ] **T007** [US1] Extend `Statement` enum with `Alignment` variant
  - File: `src/parser/ast.rs`
  - Add: `Alignment(AlignmentDecl)`
  - Acceptance: Enum compiles with new variant

- [ ] **T008** [US2] Add `Role` to `StyleKey` enum
  - File: `src/parser/ast.rs`
  - Add variant: `Role`
  - Acceptance: StyleKey compiles with Role variant

- [ ] **T009** [US3] Extend `StyleValue` to support identifier references
  - File: `src/parser/ast.rs`
  - Add variant: `Identifier(Identifier)` for `[label: my_shape]` syntax
  - Acceptance: StyleValue compiles with Identifier variant

- [ ] **T010** [Setup] Add unit tests for new AST types
  - File: `src/parser/ast.rs` (test module)
  - Test ElementPath construction and methods
  - Test Edge axis classification
  - Test AlignmentDecl validation
  - Acceptance: All unit tests pass

**Parallel Opportunities**: T003, T004 can run in parallel (different enums)

---

## Phase 2: Lexer Tokens

**Goal**: Add lexer tokens for new keywords.

**Checkpoint**: Lexer tokenizes all new syntax correctly.

### Tasks

- [ ] **T011** [US1] Add alignment-related tokens to lexer
  - File: `src/parser/lexer.rs`
  - Add tokens: `Align`, `Left`, `Right`, `Top`, `Bottom`, `HorizontalCenter`, `VerticalCenter`
  - Acceptance: `align a.left = b.right` tokenizes correctly

- [ ] **T012** [US1] [P] Add punctuation tokens to lexer
  - File: `src/parser/lexer.rs`
  - Add tokens: `Dot` (`.`), `Equals` (`=`)
  - Acceptance: `a.b = c.d` tokenizes as Ident Dot Ident Equals Ident Dot Ident

- [ ] **T013** [US2] [P] Add role token to lexer
  - File: `src/parser/lexer.rs`
  - Add token: `RoleKeyword` for `role`
  - Acceptance: `role: label` tokenizes correctly

- [ ] **T014** [Setup] Add lexer tests for new tokens
  - File: `src/parser/lexer.rs` (test module)
  - Test all new token patterns
  - Acceptance: All lexer tests pass

**Parallel Opportunities**: T011, T012, T013 can run in parallel (independent token additions)

---

## Phase 3: Parser Grammar - Element Paths

**Goal**: Parse element path syntax (`a.b.c`).

**Checkpoint**: Element paths parse into `ElementPath` AST nodes.

### Tasks

- [ ] **T015** [US1] Implement element path parser
  - File: `src/parser/grammar.rs`
  - Parse: `identifier { "." identifier }`
  - Return: `ElementPath` with segments
  - Acceptance: `group1.item.child` parses to ElementPath with 3 segments

- [ ] **T016** [US1] Add element path parser tests
  - File: `src/parser/grammar.rs` (test module)
  - Test: single segment path
  - Test: multi-segment path
  - Test: path in various contexts
  - Acceptance: All path parsing tests pass

---

## Phase 4: Parser Grammar - Alignment Statements

**Goal**: Parse complete alignment syntax.

**Checkpoint**: `align a.left = b.right = c.left` parses correctly.

### Tasks

- [ ] **T017** [US1] Implement edge parser
  - File: `src/parser/grammar.rs`
  - Parse edge keywords: left, right, horizontal_center, top, bottom, vertical_center
  - Return: `Edge` enum variant
  - Acceptance: All edge keywords parse to correct variants

- [ ] **T018** [US1] Implement alignment anchor parser
  - File: `src/parser/grammar.rs`
  - Parse: `element_path "." edge`
  - Return: `AlignmentAnchor`
  - Acceptance: `mygroup.item.left` parses to anchor

- [ ] **T019** [US1] Implement alignment declaration parser
  - File: `src/parser/grammar.rs`
  - Parse: `"align" anchor { "=" anchor }`
  - Return: `AlignmentDecl` with 2+ anchors
  - Acceptance: `align a.left = b.left = c.left` parses correctly

- [ ] **T020** [US1] Add alignment statement to statement parser
  - File: `src/parser/grammar.rs`
  - Add `alignment_decl` to statement choice
  - Acceptance: Alignment statements parse at document level

- [ ] **T021** [US1] Add alignment parser tests
  - File: `src/parser/grammar.rs` (test module)
  - Test: simple two-element alignment
  - Test: multi-element alignment chain
  - Test: nested path alignment
  - Test: mixed horizontal/vertical (should work, validated later)
  - Acceptance: All alignment parsing tests pass

---

## Phase 5: Parser Grammar - Role and Label Extensions

**Goal**: Parse `role: label` modifier and identifier-based connection labels.

**Checkpoint**: New modifier syntaxes parse correctly.

### Tasks

- [ ] **T022** [US2] Add role modifier parsing
  - File: `src/parser/grammar.rs`
  - Handle `role` as style key
  - Accept keywords (label, content) as values
  - Acceptance: `[role: label]` parses correctly

- [ ] **T023** [US3] Extend label modifier to accept identifiers
  - File: `src/parser/grammar.rs`
  - Allow `[label: identifier]` in addition to `[label: "string"]`
  - Store as `StyleValue::Identifier` vs `StyleValue::String`
  - Acceptance: `a -> b [label: my_label]` parses correctly

- [ ] **T024** [US2/US3] Add modifier extension tests
  - File: `src/parser/grammar.rs` (test module)
  - Test: `[role: label]` parsing
  - Test: `[label: identifier]` vs `[label: "string"]`
  - Test: backward compatibility with existing label syntax
  - Acceptance: All modifier tests pass

---

## Phase 6: Layout Types for Alignment

**Goal**: Add types to represent resolved alignment constraints.

**Checkpoint**: Layout types compile and can represent alignment state.

### Tasks

- [ ] **T025** [US1] Add `ResolvedAlignment` struct
  - File: `src/layout/types.rs`
  - Fields: `elements: Vec<String>`, `edge: Edge`, `span: Span`
  - Add `axis()` method
  - Acceptance: Type compiles

- [ ] **T026** [US1] [P] Add `AlignmentResult` struct
  - File: `src/layout/types.rs`
  - Fields: `elements: Vec<String>`, `coordinate: f64`, `axis: Axis`
  - Acceptance: Type compiles

- [ ] **T027** [US1] Add alignment types unit tests
  - File: `src/layout/types.rs` (test module)
  - Test ResolvedAlignment construction
  - Test axis derivation
  - Acceptance: Unit tests pass

**Parallel Opportunities**: T025, T026 can run in parallel (independent structs)

---

## Phase 7: Element Path Resolution

**Goal**: Resolve element paths to actual element IDs.

**Checkpoint**: `group1.item` resolves to element `item` inside `group1`.

### Tasks

- [ ] **T028** [US1] Implement `resolve_path` function
  - File: `src/layout/engine.rs`
  - Walk through hierarchy to find element
  - Return element ID or error with suggestions
  - Acceptance: Simple and nested paths resolve correctly

- [ ] **T029** [US1] Add path resolution error types
  - File: `src/layout/error.rs`
  - Add: `PathNotFound { path, available, suggestions }`
  - Add: `AnonymousElementReferenced { path }`
  - Acceptance: Error types compile with context

- [ ] **T030** [US1] Add path resolution tests
  - File: `src/layout/engine.rs` (test module) or `tests/`
  - Test: single segment resolution
  - Test: nested path resolution
  - Test: path not found with suggestions
  - Acceptance: All path resolution tests pass

---

## Phase 8: Alignment Resolution Pass

**Goal**: Apply alignment constraints after initial layout.

**Checkpoint**: `align a.left = b.left` correctly aligns elements.

### Tasks

- [ ] **T031** [US1] Implement `apply_alignments` function
  - File: `src/layout/engine.rs`
  - Resolve all element paths
  - Validate axis consistency
  - Compute alignment coordinate from first element
  - Shift all other elements to match
  - Acceptance: Elements align correctly on horizontal and vertical axes

- [ ] **T032** [US1] Implement edge coordinate calculation
  - File: `src/layout/engine.rs`
  - `get_edge_coordinate(element, edge) -> f64`
  - Handle all 6 edge types
  - Acceptance: Coordinates calculated correctly for all edges

- [ ] **T033** [US1] Implement element shifting
  - File: `src/layout/engine.rs`
  - `shift_element(element, delta, axis)`
  - Recursively shift children
  - Acceptance: Elements and their children move correctly

- [ ] **T034** [US1] Integrate alignment pass into layout pipeline
  - File: `src/layout/engine.rs` or `src/layout/mod.rs`
  - Call `apply_alignments` after initial layout, before bounds computation
  - Acceptance: End-to-end DSL with alignments produces correct output

- [ ] **T035** [US1] Add alignment integration tests
  - File: `tests/` (integration tests)
  - Test: horizontal center alignment
  - Test: vertical top alignment
  - Test: multi-element chain alignment
  - Test: cross-group alignment
  - Acceptance: All alignment integration tests pass

---

## Phase 9: Role-Based Label Handling

**Goal**: Support `role: label` modifier for shapes.

**Checkpoint**: `group { text "Title" [role: label] }` positions text as label.

### Tasks

- [ ] **T036** [US2] Implement role detection in layout engine
  - File: `src/layout/engine.rs`
  - Check children for `role: label` modifier
  - Position label shapes above/before group content
  - Acceptance: Labels position correctly in groups

- [ ] **T037** [US2] Add deprecation warning for old label syntax
  - File: `src/parser/grammar.rs` or `src/layout/engine.rs`
  - Emit warning when `label { }` or `label:` syntax used
  - Warning text: "Deprecated: Use `[role: label]` modifier instead"
  - Acceptance: Warning appears for old syntax, doesn't break parsing

- [ ] **T038** [US2] Add role-based label tests
  - File: `tests/` (integration tests)
  - Test: `[role: label]` positions correctly
  - Test: old `label { }` works with warning
  - Test: labels render correctly in SVG
  - Acceptance: All label tests pass

---

## Phase 10: Connection Label References

**Goal**: Allow connection labels to reference shapes.

**Checkpoint**: `a -> b [label: my_label]` uses shape `my_label` as the label.

### Tasks

- [ ] **T039** [US3] Implement connection label shape resolution
  - File: `src/layout/engine.rs`
  - When label is Identifier, look up shape
  - Extract text content from text shapes
  - Apply shape's styles to label
  - Acceptance: Shape references work as connection labels

- [ ] **T040** [US3] Maintain backward compatibility for string labels
  - File: `src/layout/engine.rs`
  - String labels (`[label: "text"]`) continue working
  - Acceptance: Existing connection labels unchanged

- [ ] **T041** [US3] Add connection label reference tests
  - File: `tests/` (integration tests)
  - Test: `text "HTTP" lbl; a -> b [label: lbl]`
  - Test: styled label shape applies styles
  - Test: legacy string labels work
  - Acceptance: All connection label tests pass

---

## Phase 11: Position Offset After Alignment

**Goal**: Make position constraints relative when alignment present.

**Checkpoint**: Position becomes offset from aligned position.

### Tasks

- [ ] **T042** [US4] Track aligned elements
  - File: `src/layout/engine.rs`
  - Maintain set of element IDs that have been aligned
  - Acceptance: Aligned elements tracked correctly

- [ ] **T043** [US4] Implement relative position for aligned elements
  - File: `src/layout/engine.rs`
  - When applying `place` constraint:
    - If element aligned: offset from current (aligned) position
    - If element not aligned: absolute positioning (existing behavior)
  - Acceptance: Position is relative after alignment

- [ ] **T044** [US4] Add position offset tests
  - File: `tests/` (integration tests)
  - Test: `align a.left = b.right; place a [x: 10]` → a is 10px right of b
  - Test: `place a [x: 100]` without alignment → a at x=100
  - Acceptance: All position offset tests pass

---

## Phase 12: Error Enhancement

**Goal**: Polish error messages for new features.

**Checkpoint**: Errors are actionable per constitution principle 4.

### Tasks

- [ ] **T045** [Setup] Add alignment-specific error types
  - File: `src/layout/error.rs`
  - Add: `IncompatibleEdges { edge1, edge2 }`
  - Add: `CircularAlignment { elements }`
  - Acceptance: Error types compile

- [ ] **T046** [Setup] Implement Levenshtein suggestions for path errors
  - File: `src/layout/error.rs` or utility module
  - Suggest similar element names for typos
  - Acceptance: "Did you mean 'header'?" appears for 'headr'

- [ ] **T047** [Setup] Implement circular alignment detection
  - File: `src/layout/engine.rs`
  - Detect when alignment would create circular dependency
  - Produce clear error message
  - Acceptance: Circular alignments produce helpful error

- [ ] **T048** [Setup] Add error message tests
  - File: `tests/` (integration tests)
  - Test: path not found with suggestions
  - Test: incompatible edges error
  - Test: circular alignment error
  - Acceptance: All error tests pass

---

## Phase 13: Refactor Example Files

**Goal**: Update example .ail files to use new features, demonstrating conciseness.

**Checkpoint**: Examples are more concise, visual output unchanged.

### Tasks

- [ ] **T049** [US5] Refactor `examples/railway-topology.ail`
  - File: `examples/railway-topology.ail`
  - Remove redundant style modifiers (rely on defaults)
  - Replace `label { }` with `[role: label]`
  - Add cross-hierarchy alignment where applicable
  - Target: ~30% token reduction
  - Acceptance: Parses without warnings, SVG output equivalent

- [ ] **T050** [US5] [P] Refactor `examples/railway-junction-direct.ail`
  - File: `examples/railway-junction-direct.ail`
  - Apply same refactoring patterns
  - Acceptance: Parses without warnings, SVG output equivalent

- [ ] **T051** [US5] [P] Refactor `examples/label-test.ail`
  - File: `examples/label-test.ail`
  - Demonstrate `[role: label]` syntax
  - Remove old `label { }` syntax
  - Acceptance: Parses without warnings, demonstrates new features

- [ ] **T052** [US5] Update snapshot tests for refactored examples
  - File: `tests/snapshots/`
  - Update or add snapshots for refactored examples
  - Verify visual equivalence
  - Acceptance: Snapshot tests pass

- [ ] **T053** [US5] Verify token count reduction
  - File: N/A (manual or script verification)
  - Count tokens before/after for railway-topology.ail
  - Document reduction percentage
  - Acceptance: At least 30% reduction achieved

**Parallel Opportunities**: T049, T050, T051 can run in parallel (independent files)

---

## Final Checkpoint

- [ ] **T054** [Setup] Run full test suite
  - All existing tests pass (backward compatibility)
  - All new tests pass
  - No deprecation warnings in refactored examples
  - Acceptance: `cargo test` passes

- [ ] **T055** [Setup] Update grammar.ebnf (documentation)
  - File: `features/001-the-grammar-and-ast-for-our-dsl/contracts/grammar.ebnf`
  - Add alignment syntax
  - Add element path syntax
  - Add role modifier
  - Acceptance: Grammar documentation matches implementation

---

## Dependencies Graph

```
Phase 1 (T001-T010: AST Types)
    ↓
Phase 2 (T011-T014: Lexer Tokens)
    ↓
Phase 3 (T015-T016: Path Parser)
    ↓
Phase 4 (T017-T021: Alignment Parser)
    ↓
Phase 5 (T022-T024: Role/Label Parser)
    ↓
Phase 6 (T025-T027: Layout Types)
    ↓
Phase 7 (T028-T030: Path Resolution)
    ↓
Phase 8 (T031-T035: Alignment Resolution)
    ↓
Phase 9 (T036-T038: Role Labels)
    ↓
Phase 10 (T039-T041: Connection Labels)
    ↓
Phase 11 (T042-T044: Position Offset)
    ↓
Phase 12 (T045-T048: Error Enhancement)
    ↓
Phase 13 (T049-T055: Examples & Polish)
```

---

## Parallel Execution Examples

### Phase 1 Parallelization
```bash
# These can run in parallel (different types)
T003 (Edge enum) || T004 (Axis enum)
```

### Phase 2 Parallelization
```bash
# These can run in parallel (independent token additions)
T011 (alignment tokens) || T012 (punctuation tokens) || T013 (role token)
```

### Phase 6 Parallelization
```bash
# These can run in parallel (independent structs)
T025 (ResolvedAlignment) || T026 (AlignmentResult)
```

### Phase 13 Parallelization
```bash
# These can run in parallel (independent files)
T049 (railway-topology) || T050 (railway-junction) || T051 (label-test)
```

---

## Implementation Strategy

### MVP Scope (Phases 1-8)
Complete core alignment functionality:
- AST types for alignment
- Parser for alignment syntax
- Layout engine alignment pass

This enables the primary use case: cross-hierarchy element alignment.

### Incremental Delivery
1. **After Phase 8**: Cross-hierarchy alignment works (US1 complete)
2. **After Phase 9**: Unified labels work (US2 complete)
3. **After Phase 10**: Connection label references work (US3 complete)
4. **After Phase 11**: Position offsets work (US4 complete)
5. **After Phase 13**: Examples demonstrate all features (US5 complete)

### Commit Strategy
Commit after each phase with passing tests:
- `feat(parser): add alignment AST types`
- `feat(parser): add alignment lexer tokens`
- `feat(parser): implement alignment parser`
- `feat(layout): add alignment resolution`
- etc.

---

*Generated: 2026-01-23*
