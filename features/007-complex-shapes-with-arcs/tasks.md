# Tasks: Complex Shapes with Arcs and Curves

<!-- Tech Stack Validation: PASSED -->
<!-- Validated against: .specswarm/tech-stack.md -->
<!-- No prohibited technologies found -->
<!-- No new dependencies required -->

## Feature Summary

Extend the Agent Illustrator DSL with a `path` shape type for defining custom shapes with straight and curved segments.

## Task Overview

| Phase | Description | Task Count |
|-------|-------------|------------|
| 1 | Foundational - AST Types | 4 |
| 2 | Lexer Extension | 2 |
| 3 | Parser Implementation | 6 |
| 4 | Grammar Documentation | 2 |
| 5 | Integration & Polish | 2 |
| **Total** | | **16** |

---

## Phase 1: Foundational - AST Types

**Goal:** Define all new AST types for path shapes before any other work can begin.

**Files:** `src/parser/ast.rs`

**Checkpoint:** All new types compile, existing tests pass.

---

### T001: Add SweepDirection and ArcParams types

**File:** `src/parser/ast.rs`

**Description:**
Add the arc parameter types at the end of the file, before the `#[cfg(test)]` block:

```rust
// ============================================
// Path Shape Types (Feature 007)
// ============================================

/// Arc sweep direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SweepDirection {
    #[default]
    Clockwise,
    Counterclockwise,
}

/// Arc curve parameters
#[derive(Debug, Clone, PartialEq)]
pub enum ArcParams {
    /// Radius-based arc: `[radius: 20, sweep: clockwise]`
    Radius {
        radius: f64,
        sweep: SweepDirection,
    },
    /// Bulge-based arc: `[bulge: 0.3]`
    Bulge(f64),
}

impl Default for ArcParams {
    fn default() -> Self {
        ArcParams::Bulge(0.414) // tan(π/8) - gentle quarter-circle
    }
}
```

**Acceptance:** Code compiles, `cargo test` passes.

---

### T002: Add VertexDecl and VertexPosition types

**File:** `src/parser/ast.rs`

**Description:**
Add vertex-related types after T001's additions:

```rust
/// Vertex position specification
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VertexPosition {
    /// X offset from origin
    pub x: Option<f64>,
    /// Y offset from origin
    pub y: Option<f64>,
}

/// Vertex declaration
#[derive(Debug, Clone, PartialEq)]
pub struct VertexDecl {
    /// Vertex name (required for referencing)
    pub name: Spanned<Identifier>,
    /// Optional position (relative to shape origin)
    pub position: Option<VertexPosition>,
}
```

**Acceptance:** Code compiles, `cargo test` passes.

---

### T003: Add LineToDecl, ArcToDecl, PathCommand, PathBody types

**File:** `src/parser/ast.rs`

**Description:**
Add segment and path body types after T002's additions:

```rust
/// Line segment declaration
#[derive(Debug, Clone, PartialEq)]
pub struct LineToDecl {
    /// Target vertex (existing or implicit)
    pub target: Spanned<Identifier>,
    /// Optional position for implicit vertex creation
    pub position: Option<VertexPosition>,
}

/// Arc segment declaration
#[derive(Debug, Clone, PartialEq)]
pub struct ArcToDecl {
    /// Target vertex (existing or implicit)
    pub target: Spanned<Identifier>,
    /// Optional position for implicit vertex creation
    pub position: Option<VertexPosition>,
    /// Arc parameters (radius, bulge, sweep)
    pub params: ArcParams,
}

/// Commands that can appear inside a path block
#[derive(Debug, Clone, PartialEq)]
pub enum PathCommand {
    /// Explicit vertex declaration: `vertex name [position]`
    Vertex(VertexDecl),
    /// Straight line segment: `line_to target [position]`
    LineTo(LineToDecl),
    /// Arc segment: `arc_to target [arc_params]`
    ArcTo(ArcToDecl),
    /// Close path with straight line: `close`
    Close,
    /// Close path with arc: `close_arc [arc_params]`
    CloseArc(ArcParams),
}

/// The body of a path shape
#[derive(Debug, Clone, PartialEq)]
pub struct PathBody {
    /// Sequence of path commands (vertices, segments, close)
    pub commands: Vec<Spanned<PathCommand>>,
}
```

**Acceptance:** Code compiles, `cargo test` passes.

---

### T004: Add PathDecl and extend ShapeType enum [P]

**File:** `src/parser/ast.rs`

**Description:**

1. Add `PathDecl` struct after T003's additions:

```rust
/// Path shape declaration
#[derive(Debug, Clone, PartialEq)]
pub struct PathDecl {
    /// Shape name (optional)
    pub name: Option<Spanned<Identifier>>,
    /// Path body: vertices and segments
    pub body: PathBody,
    /// Style modifiers (fill, stroke, etc.)
    pub modifiers: Vec<Spanned<StyleModifier>>,
}
```

2. Extend the `ShapeType` enum by adding a new variant:

```rust
pub enum ShapeType {
    Rectangle,
    Circle,
    Ellipse,
    Line,
    Polygon,
    Icon { icon_name: String },
    Text { content: String },
    SvgEmbed { ... },
    /// Custom path shape (Feature 007)
    Path(PathDecl),
}
```

**Acceptance:** Code compiles, `cargo test` passes. May need to update pattern matches in other files.

---

## Phase 2: Lexer Extension

**Goal:** Add new tokens for path keywords.

**Files:** `src/parser/lexer.rs`

**Checkpoint:** All new tokens lex correctly, existing tests pass.

---

### T005: Add path keyword tokens

**File:** `src/parser/lexer.rs`

**Description:**
Add new token variants to the `Token` enum. Add in the shape keywords section after `Text`:

```rust
// Path shape keywords (Feature 007)
#[token("path")]
Path,
#[token("vertex")]
Vertex,
#[token("line_to")]
LineTo,
#[token("arc_to")]
ArcTo,
#[token("close")]
Close,
```

And add sweep direction keywords (can go near edge keywords):

```rust
// Sweep direction keywords (Feature 007)
#[token("clockwise")]
Clockwise,
#[token("cw")]
Cw,
#[token("counterclockwise")]
Counterclockwise,
#[token("ccw")]
Ccw,
```

**Acceptance:** Code compiles.

---

### T006: Add lexer tests for path tokens [P]

**File:** `src/parser/lexer.rs`

**Description:**
Add test functions in the `#[cfg(test)]` module:

```rust
#[test]
fn test_path_keywords() {
    let tokens: Vec<_> = lex("path vertex line_to arc_to close")
        .map(|(t, _)| t)
        .collect();
    assert_eq!(
        tokens,
        vec![Token::Path, Token::Vertex, Token::LineTo, Token::ArcTo, Token::Close]
    );
}

#[test]
fn test_sweep_direction_keywords() {
    let tokens: Vec<_> = lex("clockwise cw counterclockwise ccw")
        .map(|(t, _)| t)
        .collect();
    assert_eq!(
        tokens,
        vec![Token::Clockwise, Token::Cw, Token::Counterclockwise, Token::Ccw]
    );
}

#[test]
fn test_path_example() {
    let input = r#"path "arrow" { vertex start line_to tip [x: 10] close }"#;
    let tokens: Vec<_> = lex(input).map(|(t, _)| t).collect();
    assert!(tokens.contains(&Token::Path));
    assert!(tokens.contains(&Token::Vertex));
    assert!(tokens.contains(&Token::LineTo));
    assert!(tokens.contains(&Token::Close));
}
```

**Acceptance:** `cargo test lexer` passes with new tests.

---

## Phase 3: Parser Implementation

**Goal:** Parse path declarations into AST.

**Files:** `src/parser/grammar.rs`

**Checkpoint:** Path declarations parse correctly, error messages are clear.

---

### T007: Add path parser helper imports and setup

**File:** `src/parser/grammar.rs`

**Description:**
Add imports for the new AST types at the top of the file. Locate the existing imports section and add:

```rust
use crate::parser::ast::{
    // ... existing imports ...
    PathDecl, PathBody, PathCommand, VertexDecl, VertexPosition,
    LineToDecl, ArcToDecl, ArcParams, SweepDirection,
};
```

Also import the new Token variants if not using `Token::*`.

**Acceptance:** Code compiles.

---

### T008: Implement vertex_position parser

**File:** `src/parser/grammar.rs`

**Description:**
Create a parser for vertex positions that handles both `x:, y:` and directional `right:, down:` syntax:

```rust
/// Parse vertex position modifiers: [x: 10, y: 20] or [right: 30, down: 15]
fn vertex_position() -> impl Parser<Token, VertexPosition, Error = Simple<Token>> + Clone {
    // Parse position specs inside brackets
    let x_spec = just(Token::Ident("x".into()))
        .ignore_then(just(Token::Colon))
        .ignore_then(number())
        .map(|v| ("x", v));

    let y_spec = just(Token::Ident("y".into()))
        .ignore_then(just(Token::Colon))
        .ignore_then(number())
        .map(|v| ("y", v));

    let right_spec = just(Token::Right)
        .ignore_then(just(Token::Colon))
        .ignore_then(number())
        .map(|v| ("right", v));

    let down_spec = just(Token::Ident("down".into()))
        .ignore_then(just(Token::Colon))
        .ignore_then(number())
        .map(|v| ("down", v));

    // Add left, up similarly...

    let spec = choice((x_spec, y_spec, right_spec, down_spec));

    spec.separated_by(just(Token::Comma))
        .allow_trailing()
        .delimited_by(just(Token::BracketOpen), just(Token::BracketClose))
        .map(|specs| {
            let mut pos = VertexPosition::default();
            for (key, val) in specs {
                match key {
                    "x" => pos.x = Some(val),
                    "y" => pos.y = Some(val),
                    "right" => pos.x = Some(val),
                    "down" => pos.y = Some(val),
                    "left" => pos.x = Some(-val),
                    "up" => pos.y = Some(-val),
                    _ => {}
                }
            }
            pos
        })
}
```

**Acceptance:** Helper function compiles. Add unit test if feasible.

---

### T009: Implement vertex_decl and line_to_decl parsers

**File:** `src/parser/grammar.rs`

**Description:**
Create parsers for vertex declarations and line_to segments:

```rust
/// Parse: vertex name [position]?
fn vertex_decl() -> impl Parser<Token, PathCommand, Error = Simple<Token>> + Clone {
    just(Token::Vertex)
        .ignore_then(identifier())
        .then(vertex_position().or_not())
        .map_with_span(|(name, position), span| {
            PathCommand::Vertex(VertexDecl {
                name: Spanned::new(name, span.clone()),
                position,
            })
        })
}

/// Parse: line_to target [position]?
fn line_to_decl() -> impl Parser<Token, PathCommand, Error = Simple<Token>> + Clone {
    just(Token::LineTo)
        .ignore_then(identifier())
        .then(vertex_position().or_not())
        .map_with_span(|(target, position), span| {
            PathCommand::LineTo(LineToDecl {
                target: Spanned::new(target, span.clone()),
                position,
            })
        })
}
```

**Acceptance:** Functions compile.

---

### T010: Implement arc_params and arc_to_decl parsers

**File:** `src/parser/grammar.rs`

**Description:**
Create parsers for arc parameters and arc_to segments:

```rust
/// Parse sweep direction
fn sweep_direction() -> impl Parser<Token, SweepDirection, Error = Simple<Token>> + Clone {
    choice((
        just(Token::Clockwise).to(SweepDirection::Clockwise),
        just(Token::Cw).to(SweepDirection::Clockwise),
        just(Token::Counterclockwise).to(SweepDirection::Counterclockwise),
        just(Token::Ccw).to(SweepDirection::Counterclockwise),
    ))
}

/// Parse arc parameters from modifier block
fn arc_params_from_modifiers(mods: &[(String, f64)]) -> ArcParams {
    // Check for radius first
    if let Some(&(_, radius)) = mods.iter().find(|(k, _)| k == "radius") {
        let sweep = mods.iter()
            .find(|(k, _)| k == "sweep")
            .map(|_| SweepDirection::Clockwise) // TODO: parse actual value
            .unwrap_or_default();
        ArcParams::Radius { radius, sweep }
    } else if let Some(&(_, bulge)) = mods.iter().find(|(k, _)| k == "bulge") {
        ArcParams::Bulge(bulge)
    } else {
        ArcParams::default()
    }
}

/// Parse: arc_to target [position, radius/bulge, sweep]?
fn arc_to_decl() -> impl Parser<Token, PathCommand, Error = Simple<Token>> + Clone {
    // Similar structure to line_to but with arc params extraction
    just(Token::ArcTo)
        .ignore_then(identifier())
        .then(arc_modifier_block().or_not())
        .map_with_span(|(target, mods), span| {
            let (position, params) = extract_arc_modifiers(mods);
            PathCommand::ArcTo(ArcToDecl {
                target: Spanned::new(target, span.clone()),
                position,
                params,
            })
        })
}
```

**Acceptance:** Functions compile.

---

### T011: Implement close_decl and path_block parsers

**File:** `src/parser/grammar.rs`

**Description:**
Create parsers for close directives and the full path block:

```rust
/// Parse: close or close_arc [params]
fn close_decl() -> impl Parser<Token, PathCommand, Error = Simple<Token>> + Clone {
    just(Token::Close).map(|_| PathCommand::Close)
    // TODO: Add close_arc variant if needed
}

/// Parse path command (vertex | line_to | arc_to | close)
fn path_command() -> impl Parser<Token, Spanned<PathCommand>, Error = Simple<Token>> + Clone {
    choice((
        vertex_decl(),
        line_to_decl(),
        arc_to_decl(),
        close_decl(),
    ))
    .map_with_span(|cmd, span| Spanned::new(cmd, span))
}

/// Parse path body: { commands* }
fn path_body() -> impl Parser<Token, PathBody, Error = Simple<Token>> + Clone {
    path_command()
        .repeated()
        .delimited_by(just(Token::BraceOpen), just(Token::BraceClose))
        .map(|commands| PathBody { commands })
}
```

**Acceptance:** Functions compile.

---

### T012: Implement path_decl parser and integrate with shape_decl

**File:** `src/parser/grammar.rs`

**Description:**
Create the main path declaration parser and integrate it into the existing shape parsing:

```rust
/// Parse: path "name"? identifier? [modifiers]? { body }
fn path_decl() -> impl Parser<Token, ShapeDecl, Error = Simple<Token>> + Clone {
    just(Token::Path)
        .ignore_then(string_literal().or_not())  // optional "name"
        .then(identifier().or_not())              // optional identifier
        .then(modifier_block().or_not())          // optional [modifiers]
        .then(path_body())                        // required { body }
        .map_with_span(|(((label, name), mods), body), span| {
            // Construct ShapeDecl with ShapeType::Path
            let path = PathDecl {
                name: name.or(label.map(|s| Identifier::new(s))),
                body,
                modifiers: mods.unwrap_or_default(),
            };
            ShapeDecl {
                shape_type: Spanned::new(ShapeType::Path(path), span.clone()),
                name: None, // Name is in PathDecl
                modifiers: vec![],
            }
        })
}
```

Then integrate into `shape_decl()` by adding `path_decl()` as an alternative:

```rust
fn shape_decl() -> ... {
    choice((
        path_decl(),  // Add this
        rect_decl(),
        circle_decl(),
        // ... other shapes
    ))
}
```

**Acceptance:** Path shapes parse correctly. Add integration test.

---

## Phase 4: Grammar Documentation

**Goal:** Update canonical grammar.ebnf to match implementation.

**Files:** `features/001-the-grammar-and-ast-for-our-dsl/contracts/grammar.ebnf`

**Checkpoint:** Grammar documentation is complete and consistent.

---

### T013: Add path shape productions to grammar.ebnf [P]

**File:** `features/001-the-grammar-and-ast-for-our-dsl/contracts/grammar.ebnf`

**Description:**
Add path-related productions to the grammar. Insert after the SHAPE DECLARATIONS section:

```ebnf
(* ============================================ *)
(* PATH SHAPE DECLARATIONS (Feature 007)        *)
(* ============================================ *)

path_decl = "path" [ string_literal ] [ identifier ] [ modifier_block ] path_block ;

path_block = "{" { path_command } "}" ;

path_command = vertex_decl
             | line_to_decl
             | arc_to_decl
             | close_decl ;

vertex_decl = "vertex" identifier [ position_block ] ;

position_block = "[" position_spec { "," position_spec } [ "," ] "]" ;

position_spec = ( "x" | "y" | "right" | "left" | "up" | "down" ) ":" number_value ;

line_to_decl = "line_to" identifier [ position_block ] ;

arc_to_decl = "arc_to" identifier [ arc_modifier_block ] ;

arc_modifier_block = "[" arc_modifier { "," arc_modifier } [ "," ] "]" ;

arc_modifier = position_spec
             | "radius" ":" number_value
             | "bulge" ":" number_value
             | "sweep" ":" sweep_direction ;

sweep_direction = "clockwise" | "counterclockwise" | "cw" | "ccw" ;

close_decl = "close" ;
```

Also update the `shape_type` production and version number.

**Acceptance:** Grammar is syntactically valid EBNF.

---

### T014: Add path examples to grammar.ebnf [P]

**File:** `features/001-the-grammar-and-ast-for-our-dsl/contracts/grammar.ebnf`

**Description:**
Add example path shapes to the EXAMPLE VALID INPUTS section:

```ebnf
(*
  // Path shapes (Feature 007)

  // Simple triangle
  path "triangle" {
    vertex a
    line_to b [x: 50, y: 0]
    line_to c [x: 25, y: 40]
    close
  }

  // Arrow with curved back
  path "arrow" [fill: accent-1] {
    vertex tip [x: 100, y: 25]
    line_to top [x: 60, y: 0]
    arc_to back [x: 0, y: 25, bulge: 0.3]
    line_to bottom [x: 60, y: 50]
    close
  }

  // Path in layout
  row {
    path "badge" [fill: green] {
      vertex a
      vertex b [x: 60, y: 0]
      vertex c [x: 60, y: 20]
      vertex d [x: 0, y: 20]
    }
    rect spacer [width: 10]
  }
*)
```

**Acceptance:** Examples are syntactically valid per grammar.

---

## Phase 5: Integration & Polish

**Goal:** Ensure paths work correctly with the rest of the system.

**Checkpoint:** All tests pass, paths work in layouts.

---

### T015: Add parser integration tests for paths

**File:** `src/parser/grammar.rs` (or `tests/parser_tests.rs` if separate)

**Description:**
Add comprehensive parser tests:

```rust
#[test]
fn test_parse_simple_path() {
    let input = r#"
        path "triangle" {
            vertex a
            line_to b [x: 50, y: 0]
            line_to c [x: 25, y: 40]
            close
        }
    "#;
    let result = parse(input);
    assert!(result.is_ok());
    // Verify AST structure
}

#[test]
fn test_parse_path_with_arc() {
    let input = r#"
        path "rounded" {
            vertex a
            arc_to b [x: 50, y: 0, radius: 10]
            line_to c [x: 50, y: 50]
            close
        }
    "#;
    let result = parse(input);
    assert!(result.is_ok());
}

#[test]
fn test_parse_path_in_layout() {
    let input = r#"
        row {
            path "shape1" { vertex a }
            rect spacer
        }
    "#;
    let result = parse(input);
    assert!(result.is_ok());
}

#[test]
fn test_parse_path_with_modifiers() {
    let input = r#"
        path "styled" [fill: blue, stroke: black] {
            vertex a
            vertex b [x: 100, y: 0]
        }
    "#;
    let result = parse(input);
    assert!(result.is_ok());
}

#[test]
fn test_parse_degenerate_path() {
    // Single vertex path (renders as point)
    let input = r#"path "dot" { vertex center }"#;
    let result = parse(input);
    assert!(result.is_ok());
}
```

**Acceptance:** All parser tests pass.

---

### T016: Verify no regressions in existing tests

**File:** N/A (run test suite)

**Description:**
Run the full test suite to ensure no regressions:

```bash
cargo test
cargo clippy
cargo fmt --check
```

Fix any issues that arise from the new code.

**Acceptance:** `cargo test` passes, `cargo clippy` has no warnings, code is formatted.

---

## Dependency Graph

```
T001 (SweepDirection, ArcParams)
  ↓
T002 (VertexDecl, VertexPosition)
  ↓
T003 (LineToDecl, ArcToDecl, PathCommand, PathBody)
  ↓
T004 (PathDecl, ShapeType::Path) ←─────────────────┐
  ↓                                                │
T005 (Lexer tokens)                                │
  ↓                                                │
T006 (Lexer tests) [P]                             │
  ↓                                                │
T007 (Parser imports)                              │
  ↓                                                │
T008 (vertex_position parser)                      │
  ↓                                                │
T009 (vertex_decl, line_to_decl parsers)          │
  ↓                                                │
T010 (arc_params, arc_to_decl parsers)            │
  ↓                                                │
T011 (close_decl, path_block parsers)             │
  ↓                                                │
T012 (path_decl, integration) ─────────────────────┘
  ↓
T013 (Grammar productions) [P]
T014 (Grammar examples) [P]
  ↓
T015 (Integration tests)
  ↓
T016 (Regression check)
```

## Parallel Execution Opportunities

**Phase 2:** T006 can run in parallel with T007 setup

**Phase 4:** T013 and T014 can run in parallel

**Within Phase 3:** Once T008 is complete, T009 and T010 can potentially be worked on by different agents if they don't share helper functions.

## Implementation Notes

1. **Start with types:** Phases 1-2 are straightforward. Get the types compiling first.

2. **Parser incrementally:** Build parser functions bottom-up (position → vertex → segment → block → decl).

3. **Test as you go:** After each parser function, add a quick test to verify it works.

4. **Grammar last:** Update grammar.ebnf after parser is working to ensure documentation matches reality.

5. **No rendering:** This feature only covers parsing. SVG rendering of paths is a separate feature.
