---
parent_branch: 002-railway-topology-smoke-test
feature_number: 003
status: In Progress
created_at: 2026-01-23T11:47:23+01:00
---

# Feature: Layout and Render Pipeline

## Overview

The Agent Illustrator project has a working parser that produces an AST from the declarative illustration DSL. The next step is to transform this AST into visual output. This feature creates the complete pipeline from AST to rendered SVG: a layout engine that computes positions and sizes for all elements, and a renderer that produces SVG output.

The core philosophy is **semantic over geometric**: the DSL describes meaning and relationships, while the layout engine decides exact coordinates. This allows AI agents to describe illustrations declaratively without reasoning about pixel-level positioning.

## Clarifications

### Session 2026-01-23

- Q: When position constraints conflict, how should the system behave? → A: Fail with error (report conflicting constraints and refuse to render)
- Q: When a connection references an undefined element identifier, should this be caught at layout time or parse time? → A: Layout error (parser accepts it; layout engine validates references)

## User Scenarios

### Scenario 1: Render a Simple Illustration

An AI agent generates DSL code describing shapes and connections:

```
rect "server" [fill: #3B82F6, label: "API Server"]
rect "db" [fill: #10B981, label: "Database"]
server -> db [label: "queries"]
```

The system parses this, computes layout, and produces an SVG where:
- Both rectangles are positioned with readable spacing
- The connection arrow routes cleanly between them
- Labels are placed within or near their associated elements

### Scenario 2: Layout Containers Organize Elements

An AI agent uses layout containers to express spatial relationships:

```
row {
  rect "client1" [label: "Client A"]
  rect "client2" [label: "Client B"]
  rect "client3" [label: "Client C"]
}
rect "server" [label: "Server"]
place server below client2
```

The system:
- Arranges the three clients horizontally with equal spacing
- Places the server below the middle client
- All connections route appropriately

### Scenario 3: Nested Layouts for Complex Diagrams

```
column {
  row "frontend" {
    rect "web" [label: "Web App"]
    rect "mobile" [label: "Mobile App"]
  }
  row "backend" {
    rect "api" [label: "API"]
    rect "worker" [label: "Worker"]
  }
}
web -> api
mobile -> api
api -> worker
```

The system handles nested layouts, computing positions hierarchically while respecting parent container bounds.

## Functional Requirements

### FR1: Layout Engine

**FR1.1**: The layout engine shall accept a parsed Document AST and produce a LayoutResult containing computed positions (x, y coordinates) and sizes (width, height) for every element.

**FR1.2**: Layout containers (row, column, grid, stack) shall arrange their children according to their type:
- `row`: Children placed horizontally, left-to-right, with configurable spacing
- `column`: Children placed vertically, top-to-bottom, with configurable spacing
- `grid`: Children placed in a grid pattern with auto-determined dimensions
- `stack`: Children placed at the same position (for layering/overlapping)

**FR1.3**: Position constraints (`place X right-of Y`, `place X below Y`, etc.) shall be resolved after initial layout, adjusting element positions to satisfy the constraints. If constraints conflict (cannot all be satisfied simultaneously), the layout engine shall fail with an error identifying the conflicting constraints.

**FR1.6**: The layout engine shall validate that all element identifiers referenced in connections exist in the document. Missing references shall be reported as layout errors with the undefined identifier and source location.

**FR1.4**: The layout engine shall provide reasonable default sizes for elements that do not specify dimensions.

**FR1.5**: Connection routing shall compute paths between connected elements that avoid overlapping with other elements when possible.

### FR2: SVG Renderer

**FR2.1**: The renderer shall accept a LayoutResult and produce valid SVG output as a string.

**FR2.2**: Shapes shall render according to their type:
- `rect`: SVG `<rect>` element
- `circle`: SVG `<circle>` element
- `ellipse`: SVG `<ellipse>` element
- `line`: SVG `<line>` element
- `polygon`: SVG `<polygon>` element
- `icon`: Placeholder shape (rectangle or circle) with icon identifier for future icon support

**FR2.3**: Style modifiers from the AST (fill, stroke, stroke-width, opacity, etc.) shall be applied as SVG attributes or inline styles.

**FR2.4**: Labels shall render as SVG `<text>` elements positioned appropriately (centered within shapes, or adjacent to connections).

**FR2.5**: Connections shall render as SVG `<path>` or `<line>` elements with appropriate arrowheads for directed connections.

**FR2.6**: The renderer shall set appropriate SVG viewBox dimensions to contain all rendered elements with padding.

### FR3: Pipeline Integration

**FR3.1**: A single entry point function shall accept DSL source code and return SVG output, orchestrating parse -> layout -> render.

**FR3.2**: Errors at any pipeline stage (parse, layout, render) shall be reported with appropriate context (source location for parse errors, element identifiers for layout/render errors).

**FR3.3**: The pipeline shall support configuration options for:
- Default element sizing
- Spacing between elements
- Connection routing style
- Output format options (standalone SVG vs. embeddable fragment)

## Success Criteria

- Users can transform valid DSL input into rendered SVG output in a single function call
- Layout containers correctly arrange their children in the specified direction
- Position constraints adjust element positions as specified
- Generated SVG displays correctly in standard browsers and SVG viewers
- Connection arrows route between elements without passing through other shapes for simple diagrams
- The system handles diagrams with up to 50 elements while maintaining readable output
- Error messages identify the specific element or constraint causing layout failures

## Key Entities

### LayoutResult

The intermediate representation between AST and rendering:
- Maps element identifiers to computed bounding boxes (x, y, width, height)
- Stores connection routing paths
- Contains resolved style information ready for rendering

### BoundingBox

Represents the computed spatial extent of an element:
- Position (x, y) in the coordinate system
- Dimensions (width, height)
- Optional rotation/transform information for future use

### ConnectionPath

Represents the routing of a connection between two elements:
- Start point (edge of source element)
- End point (edge of target element)
- Intermediate waypoints if routing around obstacles
- Arrow style for each end

## Assumptions

- The SVG coordinate system uses a standard top-left origin with y increasing downward
- Default element dimensions are chosen to accommodate typical label lengths (approximately 100x50 units for rectangles)
- Default spacing between elements is sufficient for readability (approximately 20-40 units)
- Simple orthogonal routing (horizontal/vertical segments only) is acceptable for the initial implementation
- Icons will be rendered as placeholder shapes initially; actual icon rendering is a future enhancement
- The grid layout auto-determines row/column count based on child count (approximately square distribution)
- Font sizing and text measurement will use approximate character-width calculations initially
