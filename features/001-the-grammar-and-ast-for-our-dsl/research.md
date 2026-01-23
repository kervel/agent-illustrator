# Research: Grammar and AST for the Agent Illustrator DSL

## Research Questions

### RQ-1: Parser Library Selection

**Decision:** Use `logos` for lexing + `chumsky` for parsing

**Rationale:**
- `logos` provides fast, zero-copy lexing with derive macros - minimal boilerplate
- `chumsky` offers excellent error recovery and diagnostic messages (aligns with FR-7 and constitution principle "Fail Fast, Fail Clearly")
- `chumsky` v1.x has good support for building ASTs with spans for error reporting
- Alternative `pest` (PEG) would require separate grammar file; `chumsky` keeps grammar in Rust code which is easier to iterate
- Alternative `nom` is lower-level and requires more boilerplate for good error messages

**Alternatives Considered:**
- `pest` - Good for complex grammars but adds build step, grammar in separate file
- `nom` - Very flexible but verbose, error handling requires significant effort
- `lalrpop` - LALR parser generator, good for complex grammars but heavier weight
- Hand-written recursive descent - Maximum control but highest effort

### RQ-2: Error Reporting Library

**Decision:** Use `ariadne` for error diagnostics

**Rationale:**
- `ariadne` produces beautiful, colorized error messages with source spans
- Integrates well with `chumsky`'s error recovery
- More actively maintained than `miette` for this use case
- Supports multi-line errors and multiple error spans

**Alternatives Considered:**
- `miette` - More feature-rich but heavier, designed for CLI applications
- Custom formatting - Too much effort for marginal benefit

### RQ-3: Grammar Design for Compactness

**Decision:** Use minimal punctuation, optional semicolons, implicit blocks

**Rationale:**
- AI agents pay per-token; verbose syntax wastes context
- Keywords should be short but unambiguous: `rect` vs `rectangle` (prefer short)
- Identifiers without quotes for simple names: `server1` not `"server1"`
- Quoted strings only when needed (spaces, special chars)
- Style blocks use `[]` not `style {}` - more compact

**Design Choices:**
```
# Compact: 15 tokens
rect server1
rect client1 [fill: blue]
server1 -> client1

# Verbose alternative: 25+ tokens
shape rectangle "server1" {}
shape rectangle "client1" { style { fill: "blue" } }
connection from "server1" to "client1" {}
```

### RQ-4: AST Span/Location Strategy

**Decision:** All AST nodes include source spans

**Rationale:**
- Required for meaningful error messages (FR-7)
- Enables future IDE integration (hover info, go-to-definition)
- `chumsky` provides spans naturally during parsing
- Negligible memory overhead

**Implementation:**
```rust
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}
```

### RQ-5: Connection Syntax

**Decision:** Use arrow operators `->`, `<->`, `--` for connections

**Rationale:**
- Familiar from other diagram DSLs (Mermaid, D2)
- Compact: `a -> b` is 3 tokens
- Directional semantics are clear
- Supports bidirectional (`<->`) and undirected (`--`)

**Syntax:**
```
a -> b           # directed
a <-> b          # bidirectional
a -- b           # undirected
a -> b [label: "HTTP"]  # with modifiers
```

## Summary

| Question | Decision | Tech/Approach |
|----------|----------|---------------|
| Parser library | logos + chumsky | Zero-copy lexing, excellent errors |
| Error reporting | ariadne | Beautiful diagnostics |
| Grammar style | Minimal punctuation | Token-efficient for AI |
| AST spans | All nodes have spans | Error reporting support |
| Connection syntax | Arrow operators | Familiar, compact |
