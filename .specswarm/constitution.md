# Project Constitution: Agent Illustrator

## Mission Statement

Create a declarative illustration language that bridges the gap between structured diagram DSLs and low-level graphics - enabling AI agents to describe *what* to draw semantically, while the renderer handles *how* to position and render it.

## Core Principles

### 1. Semantic Over Geometric
The language describes meaning and relationships, not coordinates. Layout and positioning are the renderer's responsibility, not the user's burden.
However, it should be possible to position elements explicitly (eg a user should be able to say 'move this to the right')

### 2. First-Attempt Correctness
AI agents should produce correct, readable illustrations without iteration or correction. The language must be predictable and unambiguous.
AI agents are often bad at translating abstract shapes to coordinates. you cannot ask an agent 'draw a penguin shape' and expect it to work.

### 3. Explicit Over Implicit
Favor explicit declarations over implicit behavior. When the language makes assumptions, document them clearly.

### 4. Fail Fast, Fail Clearly
Invalid input should produce clear, actionable error messages. Never silently degrade or produce unexpected output.

### 5. Composability
Primitives should combine naturally. Complex illustrations emerge from simple, well-defined building blocks.

### 6. Do not reinvent the wheel
If there is already something we can use as a component, bring it in. For instance, if we could use graphviz just for placement (not rendering) then its maybe better than making our own placement engine. This is to achieve results fast. We can always replace an external tool or lib with our own impl later.

## Coding Standards

### Rust-Specific Guidelines
- Use `cargo fmt` and `cargo clippy` before every commit
- Prefer `Result<T, E>` over panics for recoverable errors
- Use `thiserror` for error type definitions
- Document public APIs with rustdoc comments
- Write unit tests alongside implementation code

### Error Handling
- All user-facing errors must include context and suggestions
- Internal errors should preserve the error chain
- Never unwrap in library code; reserve `.unwrap()` for tests only

### Testing Philosophy
- Test behavior, not implementation
- Parser tests should cover edge cases and error messages
- Renderer tests should use snapshot testing for visual output
- Integration tests verify end-to-end language-to-SVG pipeline

## Decision Framework

When making architectural decisions:
1. Does it make the language easier for LLMs to use correctly?
2. Does it maintain semantic clarity over geometric precision?
3. Does it compose well with existing primitives?
4. Can it fail clearly if misused?

## Out of Scope

Per project goals, we explicitly do NOT:
- Replace Mermaid/D2/PlantUML for structured diagrams
- Provide pixel-perfect artistic control
- Support animation or interactivity
- Compete with design tools (Figma, Illustrator)

---

*Created: 2026-01-23*
*Last Updated: 2026-01-23*
