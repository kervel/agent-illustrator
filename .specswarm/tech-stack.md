# Tech Stack: Agent Illustrator

## Overview

| Attribute | Value |
|-----------|-------|
| Project | Agent Illustrator |
| Created | 2026-01-23 |
| Auto-Generated | No (manually configured) |

---

## Core Technologies

### Language & Runtime
- **Rust** (stable channel, latest)
  - Edition: 2021
  - MSRV: 1.75.0 (or latest stable)
  - Purpose: Language implementation, parser, renderer

### Build & Package Management
- **Cargo** (Rust's built-in)
  - Workspace: Consider if splitting into crates (parser, renderer, cli)
  - Features: Use feature flags for optional functionality

---

## Output Format

### Primary Output
- **SVG** (Scalable Vector Graphics)
  - Purpose: Vector-based illustration output
  - Library: Consider `svg` crate or direct XML generation
  - Notes: Optimize for readability (formatted XML, meaningful IDs)

---

## Approved Dependencies

### Parsing & Language
- `nom` or `pest` - Parser combinators / PEG parsing
- `logos` - Lexer generation (fast, zero-copy)
- `chumsky` - Parser combinator library with good error recovery

### SVG Generation
- `svg` - SVG document building
- `resvg` - SVG rendering (for PNG export if needed later)

### Error Handling
- `thiserror` - Derive macros for error types
- `miette` or `ariadne` - Beautiful diagnostic messages

### CLI (if applicable)
- `clap` - Command-line argument parsing
- `indicatif` - Progress bars and spinners

### Testing
- Built-in `#[test]` framework
- `insta` - Snapshot testing for SVG output
- `proptest` - Property-based testing for parser

### Development
- `pretty_assertions` - Better assertion diffs
- `tracing` - Structured logging/diagnostics

---

## Prohibited Patterns

### Dependencies
- No C/C++ dependencies requiring system libraries (pure Rust preferred)
- No unmaintained crates (check last update, issue activity)
- No `unsafe` without documented justification and review

### Code Patterns
- No `.unwrap()` or `.expect()` in library code (tests only)
- No `panic!` for recoverable errors
- No stringly-typed APIs where enums/types would work
- No global mutable state

### Architecture
- No tight coupling between parser and renderer
- No hardcoded magic numbers (use named constants)
- No premature optimization without profiling data

---

## Recommended Project Structure

```
agent-illustrator/
  Cargo.toml
  src/
    lib.rs           # Library root
    parser/          # Language parser
      mod.rs
      lexer.rs
      ast.rs
    renderer/        # SVG renderer
      mod.rs
      layout.rs      # Spatial layout engine
      svg.rs         # SVG output
    primitives/      # Built-in shapes and concepts
      mod.rs
    error.rs         # Error types
  tests/
    integration/     # End-to-end tests
    snapshots/       # SVG snapshot files
  examples/          # Example illustrations
```

---

## Notes

- This is a greenfield project - tech stack will evolve as requirements clarify
- Consider splitting into workspace crates if complexity grows
- Parser and renderer should be decoupled via an AST/IR boundary
- Update this file when adding significant new dependencies

---

*Created: 2026-01-23*
*Last Updated: 2026-01-23*
