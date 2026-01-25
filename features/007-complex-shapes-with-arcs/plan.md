# Implementation Plan: Complex Shapes with Arcs and Curves

## Feature Summary

Extend the Agent Illustrator DSL with a `path` shape type for defining custom shapes with straight and curved segments.

## Technical Context

| Aspect | Choice | Rationale |
|--------|--------|-----------|
| Language | Rust (2021 edition) | Existing codebase |
| Parser | Chumsky | Existing parser infrastructure |
| AST location | `src/parser/ast.rs` | Extend existing AST types |
| Grammar doc | `features/001.../contracts/grammar.ebnf` | Keep in sync per constitution |

## Constitution Check

| Principle | Status | Notes |
|-----------|--------|-------|
| 1. Semantic Over Geometric | ✅ Pass | Named vertices, relative positions |
| 2. First-Attempt Correctness | ✅ Pass | Bulge factor intuitive for LLMs |
| 3. Explicit Over Implicit | ✅ Pass | Positions must be specified |
| 4. Fail Fast, Fail Clearly | ✅ Pass | Invalid arcs produce clear errors |
| 5. Composability | ✅ Pass | Paths work in layouts, connections |
| 6. Don't reinvent wheel | ✅ Pass | Using existing modifier system |

## Tech Stack Compliance Report

### ✅ Approved Technologies (already in stack)
- Rust (core language)
- Chumsky (parser)
- Existing AST/modifier infrastructure

### ➕ New Technologies
None required - this feature extends existing parser/AST infrastructure.

### ❌ Prohibited Patterns Check
- No unsafe code required
- No new dependencies needed
- No stringly-typed APIs (using proper enums)

## Implementation Phases

### Phase 1: AST Types (Foundation)

**Goal:** Define all new AST types for path shapes.

**Files to modify:**
- `src/parser/ast.rs` - Add path-related types

**Deliverables:**
1. `PathDecl` struct
2. `PathBody` struct
3. `PathCommand` enum (Vertex, LineTo, ArcTo, Close, CloseArc)
4. `VertexDecl`, `VertexPosition` structs
5. `LineToDecl`, `ArcToDecl` structs
6. `ArcParams` enum (Radius, Bulge)
7. `SweepDirection` enum
8. Extended `ShapeType::Path` variant
9. Extended `StyleKey` variants (Radius, Bulge, Sweep, Rounded, directional)

**Tests:**
- Unit tests for new type construction
- Serialization round-trip tests (if applicable)

### Phase 2: Lexer Extension

**Goal:** Add new tokens for path keywords.

**Files to modify:**
- `src/parser/lexer.rs` - Add path keyword tokens

**New Tokens:**
- `path` (keyword)
- `vertex` (keyword)
- `line_to` (keyword)
- `arc_to` (keyword)
- `close` (keyword)
- `clockwise`, `counterclockwise`, `cw`, `ccw` (keywords)
- `rounded`, `radius`, `bulge`, `sweep` (modifier keys)
- `right`, `left`, `up`, `down` (directional position keys)

**Tests:**
- Lexer tests for each new keyword
- Lexer tests for combinations with identifiers

### Phase 3: Parser Implementation

**Goal:** Parse path declarations into AST.

**Files to modify:**
- `src/parser/grammar.rs` - Add path parsing rules

**Parser Rules:**
1. `path_decl` - Top-level path shape parser
2. `path_block` - Parse `{ ... }` with path commands
3. `path_command` - Dispatch to vertex/line_to/arc_to/close
4. `vertex_decl` - Parse `vertex name [position]`
5. `position_block` - Parse position modifiers
6. `line_to_decl` - Parse `line_to target [position]`
7. `arc_to_decl` - Parse `arc_to target [arc_params]`
8. `arc_params` - Parse radius/bulge/sweep
9. `close_decl` - Parse `close` or `close_arc`

**Integration:**
- Add `path_decl` to `shape_decl` parser
- Ensure path works in layout blocks

**Tests:**
- Parse simple path (triangle)
- Parse path with arcs (rounded rectangle)
- Parse path with bulge
- Parse path in layout container
- Parse degenerate paths (1-2 vertices)
- Error tests: invalid arc params, missing position

### Phase 4: Grammar Documentation

**Goal:** Update canonical grammar.ebnf.

**Files to modify:**
- `features/001-the-grammar-and-ast-for-our-dsl/contracts/grammar.ebnf`

**Updates:**
1. Add `path_decl` to `statement` production
2. Add all path-related productions
3. Add examples in comment section
4. Bump grammar version

### Phase 5: Semantic Validation (Optional - Stretch)

**Goal:** Add post-parse validation for path semantics.

**Validations:**
1. First vertex defaults to origin
2. Non-first vertices need positions
3. Arc radius validity check
4. Duplicate vertex name warning

**Note:** This phase may be deferred if time-constrained; basic parsing should work without it.

## Dependency Graph

```
Phase 1 (AST Types)
      ↓
Phase 2 (Lexer)
      ↓
Phase 3 (Parser) ←──┐
      ↓             │
Phase 4 (Grammar)   │
      ↓             │
Phase 5 (Validation)┘
```

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Parser conflicts with existing keywords | Low | Medium | Keyword list is isolated |
| Arc math complexity | Medium | Low | Well-documented algorithms, defer rendering |
| Layout integration issues | Low | Medium | Paths use same bounding box as other shapes |
| LLM confusion with syntax | Medium | High | Provide clear examples, use intuitive keywords |

## Out of Scope

Per spec, this feature does NOT include:
- SVG rendering of paths (separate layout/render work)
- Layout algorithm for vertex positioning
- Bezier curves (arcs only)
- Animation/morphing

## Success Metrics

1. All parser tests pass
2. Example paths parse correctly
3. Grammar.ebnf updated and consistent
4. No regressions in existing shape parsing

## Estimated Complexity

| Phase | Effort | Reason |
|-------|--------|--------|
| Phase 1 | Low | Straightforward type definitions |
| Phase 2 | Low | Small set of new tokens |
| Phase 3 | Medium | Core parsing logic, multiple rules |
| Phase 4 | Low | Documentation update |
| Phase 5 | Medium | Validation logic, optional |

**Total:** Medium complexity feature - primarily parser extension work.
