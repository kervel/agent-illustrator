---
parent_branch: main
feature_number: "007"
status: In Progress
created_at: 2026-01-24T22:00:00+00:00
---

# Feature: Complex Shapes with Arcs and Curves

## Overview

Extend the Agent Illustrator DSL with a new shape type that allows defining complex polygons with both straight edges and curved segments (arcs, rounded corners). This enables AI agents to describe shapes beyond simple geometric primitives—shapes like rounded rectangles, pie slices, speech bubbles, arrows with curved heads, or any custom closed path composed of lines and arcs.

The design must remain **declarative and LLM-friendly**: the AI agent describes the shape semantically (vertices and edge types), and the renderer calculates exact coordinates and curve control points.

## Clarifications

### Session 2026-01-24
- Q: Should vertices be declared upfront or created implicitly? → A: Both allowed (pre-declared or implicit on first use)
- Q: How should the path define its starting point? → A: Origin-relative (first vertex at shape origin 0,0; subsequent relative to origin)
- Q: What happens with degenerate paths (<3 vertices)? → A: Render as point/line (1 vertex = point, 2 vertices = line segment)

## User Scenarios

### Scenario 1: AI Agent Defines a Rounded Rectangle

An AI agent wants to draw a button with rounded corners. Instead of using a plain rectangle, the agent defines a path with corner radii.

**Acceptance Criteria:**
- Agent can declare a shape with four corners and specify rounding radius for each
- Parser produces an AST representing the shape with curve information
- No coordinate calculation required from the agent

### Scenario 2: AI Agent Creates an Arrow Shape

An AI agent needs a custom arrow shape (not just a connection line). The agent defines a polygon with a pointed head and optionally curved back.

**Acceptance Criteria:**
- Agent can define a multi-segment closed shape
- Straight and curved segments can be mixed in a single shape
- Shape can be named and styled like other primitives

### Scenario 3: AI Agent Draws a Pie Slice

An AI agent creates a pie chart segment—two straight lines radiating from center with an arc connecting them.

**Acceptance Criteria:**
- Agent can specify arc segments by angle or by endpoint with curvature
- The arc connects two vertices along a curved path
- Shape can be filled and styled

### Scenario 4: AI Agent Uses Relative Vertex Positioning

An AI agent defines vertices using relative positions (offsets from previous vertex or from shape origin) rather than absolute coordinates.

**Acceptance Criteria:**
- Vertices can be specified as relative offsets
- The layout engine resolves relative positions to absolute coordinates
- The AI agent does not need to calculate absolute positions

## Functional Requirements

### FR-1: Path Shape Type

Introduce a new `path` shape type for defining custom closed shapes.

**Requirement:** Users can declare a path shape with a sequence of vertices and edge types.

**Syntax Example:**
```
path "arrow" {
    vertex start
    line_to tip
    line_to back_top
    arc_to back_bottom [radius: 10]
    close
}
```

**Testable Criterion:** Parser accepts the path declaration and produces an AST with vertex and segment nodes.

### FR-2: Vertex Declarations

Vertices define points in the shape path. Vertices can be either explicitly declared upfront or implicitly created when first referenced in a segment command.

**Requirement:** Vertices can be declared with optional positioning hints. All positions are relative to the shape origin (0,0), which corresponds to the first vertex.

**Syntax Options:**
```
vertex name                         // at origin if first, else must have position
vertex name [x: 10, y: 20]         // offset from shape origin (0,0)
vertex name [right: 30, down: 15]  // directional offset from origin
line_to newvert [x: 50, y: 0]      // implicit vertex creation with position
```

**Testable Criterion:** Parser accepts vertex declarations with and without position modifiers. Parser accepts implicit vertex creation in segment commands.

### FR-3: Straight Line Segments

Connect vertices with straight lines.

**Requirement:** `line_to` connects from the current position to a named vertex.

**Testable Criterion:** Parser accepts `line_to vertex_name` and captures it in AST.

### FR-4: Arc Segments

Connect vertices with curved arcs.

**Requirement:** `arc_to` connects from the current position to a named vertex along a curved path.

**Syntax:**
```
arc_to target [radius: 20]                    // circular arc with radius
arc_to target [radius: 20, sweep: clockwise]  // direction control
arc_to target [bulge: 0.3]                    // bulge factor (0=straight, 1=semicircle)
```

**Testable Criterion:** Parser accepts arc segments with radius or bulge parameters.

### FR-5: Close Path

Close the shape by connecting back to the first vertex.

**Requirement:** `close` or `close_arc [radius: r]` completes the path. For paths with 3+ vertices, this creates a closed shape. Degenerate paths (1-2 vertices) do not require close and render as point or line segment respectively.

**Testable Criterion:** Parser accepts close directives and marks path as closed in AST. Parser accepts paths with 1-2 vertices without close directive.

### FR-6: Corner Rounding Shorthand

For convenience, allow rounding corners on simple shapes.

**Requirement:** A `rounded` modifier can be applied to specify corner radii.

**Syntax:**
```
path "box" [rounded: 10] {
    vertex tl
    vertex tr
    vertex br
    vertex bl
}
```

**Testable Criterion:** Parser accepts rounded modifier and applies arc semantics at corners.

### FR-7: Style Modifiers on Path Shapes

Path shapes support the same style modifiers as other shapes.

**Requirement:** fill, stroke, stroke_width, opacity apply to path shapes.

**Testable Criterion:** Parser accepts `path "name" [fill: blue, stroke: black] { ... }` and captures styles.

### FR-8: Nested Path Shapes in Layouts

Path shapes can participate in layout containers.

**Requirement:** Path shapes work within row, col, grid, stack containers.

**Testable Criterion:** Parser accepts path shapes inside layout blocks.

### FR-9: Path Shape Sizing

Path shapes need size hints for the layout engine.

**Requirement:** Path shapes can have explicit width/height or be auto-sized based on vertex positions.

**Testable Criterion:** Parser accepts `path "name" [width: 100, height: 50] { ... }`.

## Success Criteria

1. **LLM Usability**: An AI agent can define a rounded rectangle, arrow, or pie slice correctly on first attempt without coordinate calculations
2. **Expressiveness**: 90% of common custom shapes (buttons, callouts, arrows, badges) can be expressed with the path syntax
3. **Parser Correctness**: All valid path definitions parse without error; invalid definitions produce clear error messages
4. **Rendering Accuracy**: Rendered shapes visually match the semantic intent (arcs are smooth, corners are properly rounded)
5. **Token Efficiency**: Path definitions use fewer tokens than equivalent SVG path data

## Key Entities

### PathShape

A closed shape defined by vertices and connecting segments.

### Vertex

A named point in the path, with optional position hints.

### PathSegment

A connection between vertices, either straight (line) or curved (arc).

### ArcParameters

Curve specification: radius, sweep direction, or bulge factor.

### ClosePath

Directive to close the shape back to the starting vertex.

## Assumptions

1. **Closed Paths Preferred**: Paths with 3+ vertices should be closed; degenerate paths (1-2 vertices) render as point/line
2. **Single Contour**: No holes or multiple contours in v1; shapes are simple closed paths
3. **Origin-Relative Coordinates**: All vertex positions are offsets from the shape origin (0,0); the layout engine positions the shape itself
4. **Auto-sizing Default**: If no explicit size is given, the layout engine determines shape bounds from vertex positions
5. **Clockwise Winding**: Default path winding is clockwise; counterclockwise can be specified for arcs
6. **Smooth Joins**: Where arcs meet lines, the renderer ensures visual continuity
7. **2D Only**: All shapes are 2D; no 3D or perspective transformations

## Technical Boundaries

This feature covers:
- Grammar extensions for path shape syntax
- AST types for path shapes, vertices, and segments
- Parser implementation for path declarations
- Integration with existing modifier and styling system

This feature does NOT cover:
- Rendering path shapes to SVG (layout/render pipeline)
- Layout algorithm for positioning vertices (constraint solver)
- Bezier curves or splines (arcs only in v1)
- Animation or morphing between paths
