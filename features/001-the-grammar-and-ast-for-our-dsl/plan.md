# Implementation Plan: Grammar and AST for the Agent Illustrator DSL

## Technical Context

| Aspect | Choice | Rationale |
|--------|--------|-----------|
| Language | Rust (Edition 2021) | Per tech-stack.md |
| Lexer | `logos` | Fast, zero-copy, derive macros |
| Parser | `chumsky` v1.x | Excellent error recovery, spans |
| Error Display | `ariadne` | Beautiful diagnostics |
| Testing | `insta` + standard `#[test]` | Snapshot testing for AST output |

## Constitution Check

| Principle | Status | Evidence |
|-----------|--------|----------|
| 1. Semantic Over Geometric | ✅ PASS | Grammar describes shapes/relations, not coordinates |
| 2. First-Attempt Correctness | ✅ PASS | Simple syntax, good error messages |
| 3. Explicit Over Implicit | ✅ PASS | All constructs explicitly declared |
| 4. Fail Fast, Fail Clearly | ✅ PASS | ariadne diagnostics, span tracking |
| 5. Composability | ✅ PASS | Nesting, layout containers compose |
| 6. Don't Reinvent Wheel | ✅ PASS | Using logos, chumsky, ariadne |

## Tech Stack Compliance Report

### ✅ Approved Technologies (already in stack)
- `logos` - Lexer generation (in Parsing & Language)
- `chumsky` - Parser combinators (in Parsing & Language)
- `thiserror` - Error types (in Error Handling)
- `insta` - Snapshot testing (in Testing)

### ➕ New Technologies (auto-added)
- **ariadne** v0.4+
  - Purpose: Error diagnostic formatting
  - No conflicts detected
  - Added to: Error Handling section
  - Note: Listed as alternative to miette, now selected

### ⚠️ Conflicting Technologies
None detected.

### ❌ Prohibited Technologies
None used.

## Implementation Phases

### Phase 1: Project Structure & Dependencies

**Goal:** Set up Cargo project with dependencies and module structure.

**Files to create/modify:**
- `Cargo.toml` - Add dependencies
- `src/lib.rs` - Library root with module declarations
- `src/parser/mod.rs` - Parser module
- `src/parser/ast.rs` - AST type definitions
- `src/parser/lexer.rs` - Token definitions
- `src/error.rs` - Error types

**Dependencies to add:**
```toml
[dependencies]
logos = "0.14"
chumsky = "1.0.0-alpha.7"
ariadne = "0.4"
thiserror = "1.0"

[dev-dependencies]
insta = "1.39"
pretty_assertions = "1.4"
```

### Phase 2: Lexer Implementation

**Goal:** Define tokens using `logos` derive macro.

**Token types needed:**
- Keywords: `rect`, `circle`, `ellipse`, `line`, `polygon`, `icon`, `row`, `col`, `grid`, `stack`, `group`, `place`
- Position relations: `right-of`, `left-of`, `above`, `below`, `inside`
- Operators: `->`, `<-`, `<->`, `--`
- Delimiters: `{`, `}`, `[`, `]`, `,`, `:`
- Literals: identifiers, strings, numbers, colors
- Comments: `//...` and `/*...*/`
- Whitespace (skipped)

**Test cases:**
- Tokenize simple shape declarations
- Tokenize connections with all arrow types
- Handle comments (line and block)
- Handle string escapes
- Error on invalid tokens

### Phase 3: AST Type Definitions

**Goal:** Define all AST types per data-model.md.

**Types to implement:**
- `Spanned<T>` wrapper
- `Document` root
- `Statement` enum
- `ShapeDecl`, `ShapeType`
- `ConnectionDecl`, `ConnectionDirection`
- `LayoutDecl`, `LayoutType`
- `GroupDecl`
- `ConstraintDecl`, `PositionRelation`
- `StyleModifier`, `StyleKey`, `StyleValue`
- `Identifier`

**Traits to derive:**
- `Debug`, `Clone`, `PartialEq` for all types
- Consider `serde::Serialize` for test output (optional)

### Phase 4: Parser Implementation

**Goal:** Parse tokens into AST using `chumsky`.

**Parser functions (bottom-up):**
1. `identifier()` - Parse identifier token
2. `string_literal()` - Parse quoted string
3. `number()` - Parse numeric values
4. `style_value()` - Parse color/number/string/keyword
5. `modifier()` - Parse `key: value`
6. `modifier_block()` - Parse `[mod1, mod2]`
7. `shape_decl()` - Parse shape declarations
8. `connection_decl()` - Parse connections
9. `constraint_decl()` - Parse position constraints
10. `block()` - Parse `{ statements }`
11. `layout_decl()` - Parse layout containers
12. `group_decl()` - Parse groups
13. `statement()` - Choose between statement types
14. `document()` - Parse complete document

**Error recovery strategy:**
- Use `chumsky`'s built-in recovery
- Recover at statement boundaries
- Collect multiple errors

### Phase 5: Error Formatting

**Goal:** Format parse errors using `ariadne`.

**Error types:**
- `ParseError` - Wrapper for chumsky errors
- Include span, expected tokens, found token
- Implement `Display` using ariadne

**Error messages should:**
- Show source line with error span highlighted
- List expected tokens
- Suggest fixes when obvious (e.g., missing `}`)

### Phase 6: Testing & Validation

**Goal:** Comprehensive test coverage.

**Test categories:**
1. **Lexer unit tests** - Token recognition
2. **Parser unit tests** - Individual constructs
3. **Integration tests** - Complete documents
4. **Error message tests** - Verify error quality
5. **Snapshot tests** - AST structure verification

**Example test documents:**
```
// test_simple.ail
rect server
rect client [fill: blue]
server -> client
```

```
// test_layout.ail
row {
  rect a
  rect b
  rect c
}
```

```
// test_nested.ail
group datacenter {
  col {
    group rack1 {
      rect server1
      rect server2
    }
    group rack2 {
      rect server3
    }
  }
}
```

## File Structure After Implementation

```
src/
  lib.rs              # pub mod parser; pub mod error;
  error.rs            # ParseError, ErrorKind
  parser/
    mod.rs            # pub mod ast; pub mod lexer; pub fn parse()
    ast.rs            # All AST types
    lexer.rs          # Token enum with logos derive
    parser.rs         # chumsky parser implementation

tests/
  lexer_tests.rs      # Lexer unit tests
  parser_tests.rs     # Parser unit tests
  integration_tests.rs # Full document parsing
  snapshots/          # insta snapshots

examples/
  simple.ail          # Example DSL files
  layout.ail
  complex.ail
```

## Success Criteria Verification

| Criterion | Verification Method |
|-----------|---------------------|
| First-Attempt Correctness | Manual testing with AI-generated examples |
| Parse Speed <100ms | Benchmark test with 1000 elements |
| Error Clarity 90% | Review error messages, user feedback |
| Round-trip Completeness | Parse → AST → (future) print test |
| Compactness | Token count comparison vs alternatives |

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| chumsky v1 is alpha | Pin to specific version, monitor releases |
| Grammar ambiguity | Start simple, add features incrementally |
| Error recovery complexity | Use chumsky's built-in recovery first |
| Constraint syntax may change | Mark as experimental, design for flexibility |

## Definition of Done

- [ ] All lexer tokens implemented and tested
- [ ] All AST types defined per data-model.md
- [ ] Parser handles all grammar constructs
- [ ] Error messages include spans and suggestions
- [ ] Unit tests for all parser functions
- [ ] Integration tests for complete documents
- [ ] Snapshot tests for AST verification
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo fmt` applied
- [ ] Documentation comments on public APIs
