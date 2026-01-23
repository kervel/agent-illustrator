# Implementation Plan: AIL Format Ergonomics

## Overview

| Attribute | Value |
|-----------|-------|
| Feature | 004 - AIL Format Ergonomics |
| Branch | `004-ail-format-ergonomics` |
| Worktree | `.worktrees/004-ail-format-ergonomics` |
| Spec | [spec.md](./spec.md) |
| Research | [research.md](./research.md) |
| Status | Ready for Implementation |

---

## Technical Context

### Language & Framework
- **Language**: Rust (2021 edition)
- **Build**: Cargo
- **Existing deps**: logos, chumsky, ariadne, thiserror, insta, pretty_assertions

### New Dependencies Required
- None (pure Rust implementation)

### Architecture
- **Input**: DSL source text
- **Output**: Extended AST with alignment constraints
- **Modification points**:
  - `src/parser/lexer.rs` - new tokens
  - `src/parser/ast.rs` - new AST types
  - `src/parser/grammar.rs` - new parsing rules
  - `src/layout/engine.rs` - alignment pass
  - `src/layout/types.rs` - alignment types

### Key Files to Modify
```
src/parser/
├── lexer.rs      # Add: align, horizontal_center, vertical_center, role tokens
├── ast.rs        # Add: AlignmentDecl, ElementPath, Edge, extend StyleKey
├── grammar.rs    # Add: alignment parsing, path parsing
src/layout/
├── engine.rs     # Add: alignment resolution pass
├── types.rs      # Add: alignment intermediate types
```

---

## Constitution Check

| Principle | Status | Notes |
|-----------|--------|-------|
| 1. Semantic Over Geometric | Pass | Alignment is semantic (align X to Y), not coordinate-based |
| 2. First-Attempt Correctness | Pass | Better defaults reduce need for iteration |
| 3. Explicit Over Implicit | Pass | Alignment is explicit statement, not inferred |
| 4. Fail Fast, Fail Clearly | Pass | Invalid paths/references produce clear errors |
| 5. Composability | Pass | Alignment composes with existing constraints |
| 6. Don't Reinvent | Pass | Building on existing layout infrastructure |

---

## Tech Stack Compliance Report

### Approved Technologies (already in stack)
- Rust (stable, 2021 edition)
- logos (lexer)
- chumsky (parser)
- thiserror (error handling)
- insta (snapshot testing)

### New Technologies
- None

### Prohibited Check
- No C dependencies ✓
- No unsafe code planned ✓
- No .unwrap() in library code ✓

---

## Implementation Phases

### Phase 1: AST Type Definitions

**Goal**: Define all new AST types needed for the feature.

**Files to modify**:
- `src/parser/ast.rs`

**New Types**:
```rust
/// Dot-separated path to an element
pub struct ElementPath {
    pub segments: Vec<Spanned<Identifier>>,
}

/// Edge of an element for alignment
pub enum Edge {
    // Horizontal
    Left,
    HorizontalCenter,
    Right,
    // Vertical
    Top,
    VerticalCenter,
    Bottom,
}

/// An alignment point: element path + edge
pub struct AlignmentAnchor {
    pub element: Spanned<ElementPath>,
    pub edge: Spanned<Edge>,
}

/// Alignment constraint declaration
pub struct AlignmentDecl {
    pub anchors: Vec<AlignmentAnchor>,  // At least 2 elements
}
```

**Modifications to existing types**:
```rust
// In StyleKey, add:
Role,  // For role: label modifier

// In StyleValue, add:
Identifier(String),  // For label: my_shape reference

// In Statement, add:
Alignment(AlignmentDecl),
```

**Tests**:
- Unit tests for ElementPath construction
- Edge enum coverage

**Acceptance**:
- All types compile
- Can construct alignment AST nodes programmatically

---

### Phase 2: Lexer Tokens

**Goal**: Add lexer tokens for new keywords.

**Files to modify**:
- `src/parser/lexer.rs`

**New Tokens**:
```rust
#[token("align")]
Align,

#[token("left")]
Left,

#[token("right")]
Right,

#[token("horizontal_center")]
HorizontalCenter,

#[token("vertical_center")]
VerticalCenter,

#[token("top")]
Top,

#[token("bottom")]
Bottom,

#[token("role")]
RoleKeyword,

#[token(".")]
Dot,

#[token("=")]
Equals,
```

**Tests**:
- Lexer tokenizes all new keywords
- Dot and equals tokens work

**Acceptance**:
- `align a.left = b.right` tokenizes correctly

---

### Phase 3: Parser Grammar Extensions

**Goal**: Parse alignment statements and element paths.

**Files to modify**:
- `src/parser/grammar.rs`

**New Grammar Rules**:
```
element_path = identifier { "." identifier } ;

edge = "left" | "right" | "horizontal_center"
     | "top" | "bottom" | "vertical_center" ;

alignment_anchor = element_path "." edge ;

alignment_decl = "align" alignment_anchor { "=" alignment_anchor } ;
```

**Parser Implementation**:
```rust
// Element path parser
let element_path = identifier
    .separated_by(just(Token::Dot))
    .at_least(1)
    .collect::<Vec<_>>()
    .map(|segments| ElementPath { segments });

// Edge parser
let edge = choice((
    just(Token::Left).to(Edge::Left),
    just(Token::Right).to(Edge::Right),
    // ... etc
));

// Alignment anchor
let alignment_anchor = element_path
    .then_ignore(just(Token::Dot))
    .then(edge);

// Alignment declaration
let alignment_decl = just(Token::Align)
    .ignore_then(alignment_anchor)
    .then(
        just(Token::Equals)
            .ignore_then(alignment_anchor)
            .repeated()
            .at_least(1)
    );
```

**Also update**:
- `role` modifier parsing (role: label)
- `label` modifier to accept identifier (label: my_shape)

**Tests**:
- Parse `align a.left = b.right`
- Parse `align a.top = b.top = c.top`
- Parse `align group1.item.left = group2.other.left`
- Parse `rect foo [role: label]`
- Parse `a -> b [label: my_label]`

**Acceptance**:
- All alignment syntax parses correctly
- AST captures all alignment information

---

### Phase 4: Layout Types for Alignment

**Goal**: Add types to represent resolved alignment constraints.

**Files to modify**:
- `src/layout/types.rs`

**New Types**:
```rust
/// Resolved alignment constraint ready for application
pub struct ResolvedAlignment {
    /// Element identifiers to align
    pub elements: Vec<String>,
    /// Which edge to align
    pub edge: Edge,
    /// Computed alignment coordinate (after first pass)
    pub coordinate: Option<f64>,
}
```

**Tests**:
- Unit tests for ResolvedAlignment

**Acceptance**:
- Types compile and can represent alignment state

---

### Phase 5: Element Path Resolution

**Goal**: Resolve element paths to actual element IDs.

**Files to modify**:
- `src/layout/engine.rs`

**Algorithm**:
```
fn resolve_path(path: &ElementPath, layout: &LayoutResult) -> Result<String, LayoutError>
    if path.segments.len() == 1:
        // Simple case: direct element reference
        return path.segments[0].name
    else:
        // Walk through hierarchy
        current = root
        for segment in path.segments:
            current = find_child(current, segment.name)?
        return current.id
```

**Error Cases**:
- Path segment not found → suggest similar names
- Element is anonymous → can't be referenced

**Tests**:
- Simple path resolution (single segment)
- Nested path resolution
- Path not found error with suggestion
- Path to anonymous element error

**Acceptance**:
- `group1.item` resolves to element `item` inside `group1`
- Clear error messages for invalid paths

---

### Phase 6: Alignment Resolution Pass

**Goal**: Apply alignment constraints after initial layout.

**Files to modify**:
- `src/layout/engine.rs`

**Algorithm**:
```
fn apply_alignments(layout: &mut LayoutResult, alignments: &[AlignmentDecl]) -> Result<(), LayoutError>
    for alignment in alignments:
        // Resolve all element paths
        elements = resolve_paths(alignment.anchors)?

        // Determine edge type (horizontal or vertical)
        edge_type = get_edge_type(alignment.anchors[0].edge)

        // Compute alignment coordinate from first element
        anchor_elem = layout.get_element(elements[0])?
        coord = get_edge_coordinate(anchor_elem, edge_type)

        // Apply to all other elements
        for elem_id in elements[1..]:
            elem = layout.get_element_mut(elem_id)?
            delta = coord - get_edge_coordinate(elem, edge_type)
            shift_element(elem, delta, edge_type)
```

**Edge Coordinate Calculation**:
- `Left` → bounds.x
- `HorizontalCenter` → bounds.x + bounds.width / 2
- `Right` → bounds.x + bounds.width
- `Top` → bounds.y
- `VerticalCenter` → bounds.y + bounds.height / 2
- `Bottom` → bounds.y + bounds.height

**Tests**:
- Horizontal center alignment
- Vertical top alignment
- Multi-element alignment chain
- Nested element alignment

**Acceptance**:
- `align a.left = b.left` aligns element `a` left edge to element `b` left edge
- Nested elements can be aligned across groups

---

### Phase 7: Role-Based Label Handling

**Goal**: Support `role: label` modifier for shapes.

**Files to modify**:
- `src/layout/engine.rs`
- `src/renderer/svg.rs` (if needed)

**Implementation**:
- When processing group children, check for `role: label` modifier
- Position labeled shapes specially (above/before group content)
- Maintain backward compatibility with `label { }` syntax

**Deprecation Warning**:
- When parsing `label { }` or `label:` syntax, emit warning
- Warning includes migration suggestion

**Tests**:
- `group { text "Title" [role: label] rect a }` positions text as label
- `group { label { text "Title" } rect a }` works with deprecation warning

**Acceptance**:
- Both syntaxes work
- Deprecation warning appears for old syntax

---

### Phase 8: Connection Label References

**Goal**: Allow connection labels to reference shapes.

**Files to modify**:
- `src/parser/grammar.rs` - parse identifier as label value
- `src/layout/engine.rs` - resolve shape reference

**Implementation**:
- Parser: `[label: identifier]` parses as StyleValue::Identifier
- Layout: When creating ConnectionLayout, if label is identifier:
  - Look up shape by ID
  - Use shape's text content (if text shape) or ID as label text
  - Apply shape's styles to label

**Tests**:
- `text "HTTP" lbl; a -> b [label: lbl]` uses "HTTP" as label
- `text "Styled" lbl [fill: blue]; a -> b [label: lbl]` applies blue fill to label
- Legacy `[label: "text"]` still works

**Acceptance**:
- Shape references work as connection labels
- Legacy string labels unchanged

---

### Phase 9: Position Offset After Alignment

**Goal**: Make position constraints relative when alignment present.

**Files to modify**:
- `src/layout/engine.rs`

**Implementation**:
- Track which elements have been aligned
- When applying `place` constraint:
  - If element was aligned: offset from aligned position
  - If element not aligned: absolute positioning (current behavior)

**Tests**:
- `align a.left = b.right; place a [x: 10]` → a is 10px right of b's right edge
- `place a [x: 100]` without alignment → a is at x=100

**Acceptance**:
- Position works as offset after alignment
- Absolute positioning unchanged when no alignment

---

### Phase 10: Error Enhancement

**Goal**: Polish error messages for new features.

**Files to modify**:
- `src/layout/error.rs`
- `src/error.rs`

**Enhancements**:
- Path resolution errors: "Element 'foo' not found in group 'bar'. Did you mean 'food'?"
- Alignment errors: "Cannot align horizontal_center with top (incompatible edge types)"
- Circular alignment detection: "Circular alignment: a depends on b, b depends on a"

**Tests**:
- Error messages include context
- Suggestions provided for typos
- Circular dependencies detected

**Acceptance**:
- Errors are actionable per constitution principle 4

---

### Phase 11: Refactor Example Files

**Goal**: Update example .ail files to use new features, demonstrating conciseness gains.

**Files to modify**:
- `examples/railway-topology.ail`
- `examples/railway-junction-direct.ail`
- `examples/label-test.ail`

**Refactoring Targets**:

1. **Remove redundant style modifiers** - rely on better defaults
   ```diff
   - circle e1 [fill: #4169E1, size: 6]
   + circle e1 [size: 6]  // fill defaults to sensible value
   ```

2. **Replace `label { }` with `role: label`**
   ```diff
   - label {
   -     col micro_label {
   -         text "Micro" micro_lbl [font_size: 20, fill: #333333]
   -     }
   - }
   + text "Micro" micro_lbl [font_size: 20, role: label]
   ```

3. **Use cross-hierarchy alignment** where applicable
   ```diff
   + // Align all level labels
   + align micro_lbl.left = meso_lbl.left = macro_lbl.left
   ```

4. **Simplify connection styling** - use defaults for common patterns
   ```diff
   - e1 -- s1 [stroke: #4169E1, stroke_width: 2, routing: direct]
   + e1 -- s1 [routing: direct]  // stroke defaults
   ```

**Before/After Comparison** (railway-topology.ail):

| Metric | Before | After | Reduction |
|--------|--------|-------|-----------|
| Lines | ~104 | ~70 | ~33% |
| Tokens | ~800 | ~500 | ~38% |
| Explicit modifiers | ~50 | ~20 | ~60% |

**Tests**:
- All refactored examples parse successfully
- Rendered SVG output is visually equivalent (snapshot comparison)
- No deprecation warnings in refactored files

**Acceptance**:
- Examples demonstrate new features
- Token count reduced by at least 30%
- Visual output unchanged or improved

---

## File Structure (Final)

```
src/
├── lib.rs                    # Exports (unchanged)
├── main.rs                   # CLI (unchanged)
├── error.rs                  # Extended with alignment errors
├── parser/
│   ├── mod.rs
│   ├── lexer.rs              # + align, edge, role tokens
│   ├── grammar.rs            # + alignment, path parsing
│   └── ast.rs                # + AlignmentDecl, ElementPath, Edge
├── layout/
│   ├── mod.rs                # + apply_alignments export
│   ├── types.rs              # + ResolvedAlignment
│   ├── config.rs             # (unchanged)
│   ├── engine.rs             # + alignment resolution
│   ├── routing.rs            # (unchanged)
│   └── error.rs              # + alignment errors
└── renderer/
    ├── mod.rs                # (unchanged)
    ├── svg.rs                # (unchanged)
    └── config.rs             # (unchanged)

examples/
├── railway-topology.ail      # Refactored: ~30% fewer tokens
├── railway-junction-direct.ail  # Refactored
└── label-test.ail            # Refactored: demonstrates role: label
```

---

## Testing Strategy

### Unit Tests
- AST type construction
- Edge coordinate calculations
- Path resolution
- Alignment arithmetic

### Integration Tests
- End-to-end DSL → SVG with alignments
- Backward compatibility with existing .ail files
- Deprecation warning output

### Snapshot Tests
- Aligned layouts
- Cross-hierarchy alignment
- Role-based labels

### Test Files
```
tests/
├── alignment_tests.rs        # Alignment-specific tests
├── path_tests.rs             # Element path resolution tests
└── snapshots/
    ├── aligned_headers.svg
    ├── cross_group_align.svg
    ├── role_label.svg
    ├── railway-topology.svg      # Refactored example snapshot
    ├── railway-junction-direct.svg
    └── label-test.svg
```

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Breaking existing .ail files | Extensive backward compatibility testing |
| Circular alignment dependencies | Detection algorithm in Phase 6 |
| Path resolution complexity | Start with simple paths, defer wildcards |
| Performance with many alignments | Profile if issues arise; iterative solver if needed |

---

## Dependencies

```
Phase 1 (AST) → Phase 2 (Lexer) → Phase 3 (Parser) → Phase 4 (Layout Types)
                                        ↓
                                 Phase 5 (Path Resolution) → Phase 6 (Alignment)
                                        ↓
                                 Phase 7 (Role Labels)
                                        ↓
                                 Phase 8 (Connection Labels)
                                        ↓
                                 Phase 9 (Position Offset)
                                        ↓
                                 Phase 10 (Error Enhancement)
                                        ↓
                                 Phase 11 (Refactor Examples)
```

---

## Next Steps

1. Run `/specswarm:tasks` to generate tasks.md
2. Begin Phase 1 implementation
3. Commit after each phase with passing tests

---

*Created: 2026-01-23*
