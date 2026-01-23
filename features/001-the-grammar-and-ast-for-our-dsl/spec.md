---
parent_branch: master
feature_number: "001"
status: In Progress
created_at: 2026-01-23T00:00:00+00:00
---

# Feature: Grammar and AST for the Agent Illustrator DSL

## Overview

Define the formal grammar and Abstract Syntax Tree (AST) data structures for the Agent Illustrator declarative illustration language. This provides the foundation that enables AI agents to describe illustrations semantically—specifying shapes, relationships, and layouts—without dealing with coordinates or rendering details.

The grammar must be unambiguous and predictable so that AI agents can produce correct syntax on the first attempt. The AST must cleanly separate syntax concerns from rendering, enabling future layout engines to interpret the same language.

**Key Design Goal: Compactness** — The grammar should minimize token count since AI agents pay per-token. Verbose syntax wastes resources and context window.

## Clarifications

### Session 2026-01-23
- Q: Should connections support labels/annotations? → A: Yes, labeled connections with style modifiers like shapes
- Q: What core shape primitives in initial version? → A: Geometric basics (rectangle, circle, ellipse, line, polygon) plus named icon references (`icon "server"`)
- Q: How to handle explicit positioning hints? → A: Constraint syntax with separate statements (`place client1 right-of server1`), but design is experimental and may evolve

## User Scenarios

### Scenario 1: AI Agent Generates Simple Illustration
An AI agent receives a prompt: "Draw a server connected to two clients." The agent writes valid DSL code describing shapes and connections. The parser successfully converts this to an AST without errors.

**Acceptance Criteria:**
- Agent can express named shapes (server, client1, client2)
- Agent can express connections between shapes
- Parser produces a valid AST from the input
- No coordinate specification required from the agent

### Scenario 2: AI Agent Uses Semantic Layout
An AI agent needs to show "three items arranged horizontally with the middle one highlighted." The agent uses layout directives and styling without calculating positions.

**Acceptance Criteria:**
- Agent can specify layout arrangements (horizontal, vertical, grid)
- Agent can apply styles/modifiers to specific elements (highlighting)
- Parser correctly captures layout intent in the AST
- No explicit positioning required

### Scenario 3: Parser Provides Clear Error on Invalid Input
An AI agent generates malformed syntax. The parser returns a clear, actionable error message indicating exactly what went wrong and where.

**Acceptance Criteria:**
- Error messages include line and column numbers
- Error messages describe what was expected vs. what was found
- Error recovery allows identifying multiple errors when possible

### Scenario 4: Complex Nested Structures
An AI agent describes a diagram with groups containing other groups and shapes—e.g., "a datacenter containing two server racks, each containing multiple servers."

**Acceptance Criteria:**
- Grammar supports nested grouping/containment
- AST preserves hierarchical structure
- Arbitrary nesting depth is supported

## Functional Requirements

### FR-1: Core Shape Primitives
The grammar must support declaring shapes with identifiers and types.

**Requirement:** Users can declare shapes using a shape type and an optional name.
**Supported Shape Types:**
- Geometric: `rectangle`, `circle`, `ellipse`, `line`, `polygon`
- Semantic: `icon "name"` for named icon references (e.g., `icon "server"`, `icon "database"`)

**Testable Criterion:** Parser accepts `rectangle "server1"` and produces an AST node with type=rectangle, name="server1". Parser accepts `icon "server" "myserver"` and produces an AST node with type=icon, icon_name="server", name="myserver".

### FR-2: Connection/Relationship Declarations
The grammar must support expressing relationships between shapes.

**Requirement:** Users can declare connections between named shapes, with optional labels and style modifiers.
**Testable Criteria:**
- Parser accepts `connect server1 -> client1` and produces an AST node representing a directed connection
- Parser accepts `connect server1 -> client1 [label: "HTTP", style: dashed]` and captures label and style in AST

### FR-3: Layout Directives
The grammar must support layout hints that the renderer interprets.

**Requirement:** Users can wrap shapes in layout containers (row, column, grid).
**Testable Criterion:** Parser accepts `row { shape1; shape2; shape3 }` and produces an AST with a layout container containing three children.

### FR-4: Style Modifiers
The grammar must support applying visual styles to elements.

**Requirement:** Users can apply styles like color, stroke, fill, emphasis to shapes.
**Testable Criterion:** Parser accepts `rectangle "warning" [fill: red, stroke: bold]` and captures styles in AST.

### FR-5: Nested Groups
The grammar must support arbitrary nesting of groups and shapes.

**Requirement:** Groups can contain shapes and other groups.
**Testable Criterion:** Parser accepts `group "datacenter" { group "rack1" { rectangle "server1" } }` and produces nested AST structure.

### FR-6: Comments
The grammar must support comments for documentation.

**Requirement:** Line and block comments are ignored by the parser.
**Testable Criterion:** Parser ignores `// line comment` and `/* block comment */` tokens.

### FR-7: Error Reporting
The parser must provide clear, actionable error messages.

**Requirement:** Syntax errors include position information and helpful context.
**Testable Criterion:** Invalid input produces an error with line number, column, and description of the problem.

### FR-8: Positioning Constraints (Experimental)
The grammar must support explicit positioning hints via constraint statements.

**Requirement:** Users can express relative positioning between elements.
**Testable Criterion:** Parser accepts `place client1 right-of server1` and produces an AST constraint node.
**Note:** This syntax is experimental and may evolve based on real-world usage patterns.

### FR-9: AST Data Structures
The AST must be well-typed and represent all valid language constructs.

**Requirement:** AST types are defined in Rust with clear ownership semantics.
**Testable Criterion:** All grammar constructs have corresponding AST node types that can be pattern-matched.

## Success Criteria

1. **First-Attempt Correctness**: An AI agent following the grammar documentation can produce valid syntax 95% of the time without trial-and-error
2. **Parse Speed**: Documents up to 1000 elements parse in under 100 milliseconds on standard hardware
3. **Error Clarity**: User testing shows 90% of error messages are understood and actionable without consulting documentation
4. **Completeness**: All language constructs can be round-tripped (parse -> AST -> future pretty-print matches semantically)
5. **Compactness**: Common illustration patterns can be expressed in minimal tokens; syntax avoids unnecessary verbosity to reduce AI agent costs and context usage

## Key Entities

### Shape
A visual element with a type, optional name, and optional style modifiers.

### Connection
A relationship between two named shapes, with optional direction, label, and style modifiers.

### LayoutContainer
A grouping construct that arranges children according to a layout strategy (row, column, grid, stack).

### Group
A named container for organizational/semantic grouping (no layout implication).

### StyleModifier
Key-value pairs representing visual properties (fill, stroke, opacity, emphasis).

### Document
The root AST node containing all top-level declarations.

### PositionConstraint
An explicit positioning hint relating one element's position to another (experimental).

### Icon
A semantic shape reference by name (e.g., "server", "database") rather than geometric primitive.

## Assumptions

1. **Text-based Format**: The DSL is plain text, not binary or visual
2. **UTF-8 Encoding**: All input is assumed to be valid UTF-8
3. **Whitespace Insensitive**: Whitespace is not semantically significant (except as token separator)
4. **Case Sensitive**: Keywords and identifiers are case-sensitive
5. **No Macros/Imports Initially**: First version is self-contained; imports can be added later
6. **Shape Types are Extensible**: Core shapes (rectangle, circle, ellipse, polygon) with ability to add more
7. **Identifiers follow common conventions**: Alphanumeric plus underscores, starting with letter or underscore

## Technical Boundaries

This feature covers:
- Formal grammar definition (EBNF or similar notation)
- Lexer token definitions
- AST Rust type definitions
- Parser implementation that produces AST from source text
- Error types and error message formatting

This feature does NOT cover:
- Rendering the AST to SVG (separate feature)
- Layout algorithm implementation (separate feature)
- Semantic validation beyond syntax (separate feature)
- CLI interface (separate feature)
