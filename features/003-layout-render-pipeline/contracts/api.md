# API Contracts: Layout and Render Pipeline

## Overview

This document defines the public API surface for the layout and render pipeline.

---

## Primary Entry Point

### `render(source: &str) -> Result<String, RenderError>`

Transforms DSL source code into SVG output.

**Parameters:**
- `source`: DSL source code as a string

**Returns:**
- `Ok(String)`: Valid SVG document
- `Err(RenderError)`: Error with context

**Example:**
```rust
let svg = render(r#"
    rect server [fill: #3B82F6, label: "Server"]
    rect db [fill: #10B981, label: "Database"]
    server -> db
"#)?;
```

---

### `render_with_config(source: &str, config: RenderConfig) -> Result<String, RenderError>`

Transforms DSL source with custom configuration.

**Parameters:**
- `source`: DSL source code
- `config`: Rendering configuration

**Returns:**
- `Ok(String)`: Valid SVG document
- `Err(RenderError)`: Error with context

---

## Configuration Types

### `RenderConfig`

```rust
pub struct RenderConfig {
    /// Layout engine configuration
    pub layout: LayoutConfig,

    /// SVG output options
    pub svg: SvgConfig,
}
```

### `LayoutConfig`

```rust
pub struct LayoutConfig {
    /// Default rectangle size (width, height)
    pub default_rect_size: (f64, f64),

    /// Default circle radius
    pub default_circle_radius: f64,

    /// Spacing between sibling elements
    pub element_spacing: f64,

    /// Padding inside layout containers
    pub container_padding: f64,

    /// Minimum spacing for connection routes
    pub connection_spacing: f64,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            default_rect_size: (100.0, 50.0),
            default_circle_radius: 30.0,
            element_spacing: 20.0,
            container_padding: 10.0,
            connection_spacing: 10.0,
        }
    }
}
```

### `SvgConfig`

```rust
pub struct SvgConfig {
    /// Padding around the illustration in viewBox
    pub viewbox_padding: f64,

    /// Whether to include XML declaration
    pub standalone: bool,

    /// Whether to format output for readability
    pub pretty_print: bool,

    /// CSS class prefix for generated classes
    pub class_prefix: Option<String>,
}

impl Default for SvgConfig {
    fn default() -> Self {
        Self {
            viewbox_padding: 20.0,
            standalone: true,
            pretty_print: true,
            class_prefix: Some("ai-".to_string()),
        }
    }
}
```

---

## Error Types

### `RenderError`

```rust
pub enum RenderError {
    /// Parse error with source location
    Parse(ParseError),

    /// Layout error (invalid references, conflicts)
    Layout(LayoutError),

    /// Rendering error (shouldn't happen with valid layout)
    Render(String),
}
```

### `LayoutError`

```rust
pub enum LayoutError {
    /// Reference to undefined element
    UndefinedIdentifier {
        name: String,
        span: Span,
        suggestions: Vec<String>,
    },

    /// Constraints cannot all be satisfied
    ConflictingConstraints {
        constraints: Vec<ConstraintInfo>,
        reason: String,
    },

    /// Constraint dependencies form a cycle
    CircularConstraint {
        cycle: Vec<String>,
    },

    /// Invalid layout structure
    InvalidLayout {
        element: String,
        reason: String,
    },
}
```

---

## Internal APIs

### Layout Module

```rust
pub mod layout {
    /// Compute layout from validated AST
    pub fn compute(doc: &Document, config: &LayoutConfig) -> Result<LayoutResult, LayoutError>;

    /// Validate all identifier references
    pub fn validate_references(doc: &Document) -> Result<(), LayoutError>;

    /// Resolve position constraints
    pub fn resolve_constraints(
        layout: &mut LayoutResult,
        constraints: &[ConstraintDecl]
    ) -> Result<(), LayoutError>;

    /// Route connections between elements
    pub fn route_connections(
        layout: &mut LayoutResult,
        connections: &[ConnectionDecl]
    ) -> Result<(), LayoutError>;
}
```

### Renderer Module

```rust
pub mod renderer {
    /// Render layout result to SVG string
    pub fn render_svg(layout: &LayoutResult, config: &SvgConfig) -> String;

    /// Render a single element to SVG
    fn render_element(element: &ElementLayout, builder: &mut SvgBuilder);

    /// Render a connection path to SVG
    fn render_connection(conn: &ConnectionLayout, builder: &mut SvgBuilder);
}
```

---

## CSS Classes

The renderer generates semantic CSS classes for styling:

### Shape Classes
- `.ai-shape` - All shapes
- `.ai-rect` - Rectangles
- `.ai-circle` - Circles
- `.ai-ellipse` - Ellipses
- `.ai-polygon` - Polygons
- `.ai-icon` - Icons

### Connection Classes
- `.ai-connection` - All connections
- `.ai-connection-forward` - Directed arrows (->)
- `.ai-connection-backward` - Reverse arrows (<-)
- `.ai-connection-bidirectional` - Bidirectional (<->)
- `.ai-connection-undirected` - Undirected (--)

### Label Classes
- `.ai-label` - All labels
- `.ai-label-shape` - Shape labels
- `.ai-label-connection` - Connection labels

### Container Classes
- `.ai-container` - Layout containers
- `.ai-row` - Row containers
- `.ai-column` - Column containers
- `.ai-grid` - Grid containers
- `.ai-stack` - Stack containers
- `.ai-group` - Semantic groups

### State Classes
- `.ai-highlighted` - Highlighted elements (from class modifier)

---

## SVG Output Structure

```xml
<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg"
     viewBox="0 0 {width} {height}"
     class="ai-illustration">
  <defs>
    <!-- Arrow markers -->
    <marker id="ai-arrow" ...>
      <path d="M0,0 L10,5 L0,10 Z"/>
    </marker>
  </defs>

  <!-- Elements -->
  <g class="ai-elements">
    <rect id="server" class="ai-shape ai-rect" ... />
    <text class="ai-label ai-label-shape">Server</text>
  </g>

  <!-- Connections -->
  <g class="ai-connections">
    <path class="ai-connection ai-connection-forward" marker-end="url(#ai-arrow)" ... />
  </g>
</svg>
```

---

*Created: 2026-01-23*
