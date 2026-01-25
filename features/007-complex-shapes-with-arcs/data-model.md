# Data Model: Complex Shapes with Arcs and Curves

## Overview

This document defines the AST extensions for the `path` shape type. These types extend the existing AST in `src/parser/ast.rs`.

## New Types

### PathDecl

```rust
/// Path shape declaration
/// A custom shape defined by vertices and connecting segments
#[derive(Debug, Clone, PartialEq)]
pub struct PathDecl {
    /// Shape name (optional, for referencing in connections/constraints)
    pub name: Option<Spanned<Identifier>>,
    /// Path body: vertices and segments
    pub body: PathBody,
    /// Style modifiers (fill, stroke, size, etc.)
    pub modifiers: Vec<Spanned<StyleModifier>>,
}
```

### PathBody

```rust
/// The body of a path shape
#[derive(Debug, Clone, PartialEq)]
pub struct PathBody {
    /// Sequence of path commands (vertices, segments, close)
    pub commands: Vec<Spanned<PathCommand>>,
}
```

### PathCommand

```rust
/// Commands that can appear inside a path block
#[derive(Debug, Clone, PartialEq)]
pub enum PathCommand {
    /// Explicit vertex declaration: `vertex name [position]`
    Vertex(VertexDecl),
    /// Straight line segment: `line_to target [implicit_vertex_pos]`
    LineTo(LineToDecl),
    /// Arc segment: `arc_to target [arc_params]`
    ArcTo(ArcToDecl),
    /// Close path with straight line: `close`
    Close,
    /// Close path with arc: `close_arc [arc_params]`
    CloseArc(ArcParams),
}
```

### VertexDecl

```rust
/// Vertex declaration
#[derive(Debug, Clone, PartialEq)]
pub struct VertexDecl {
    /// Vertex name (required for referencing)
    pub name: Spanned<Identifier>,
    /// Optional position (relative to shape origin)
    pub position: Option<VertexPosition>,
}

/// Vertex position specification
#[derive(Debug, Clone, PartialEq)]
pub struct VertexPosition {
    /// X offset from origin (or None for implicit)
    pub x: Option<f64>,
    /// Y offset from origin (or None for implicit)
    pub y: Option<f64>,
}
```

### LineToDecl

```rust
/// Line segment declaration
#[derive(Debug, Clone, PartialEq)]
pub struct LineToDecl {
    /// Target vertex (existing or implicit)
    pub target: Spanned<Identifier>,
    /// Optional position for implicit vertex creation
    pub position: Option<VertexPosition>,
}
```

### ArcToDecl

```rust
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
```

### ArcParams

```rust
/// Arc curve parameters
#[derive(Debug, Clone, PartialEq)]
pub enum ArcParams {
    /// Radius-based arc: `[radius: 20, sweep: clockwise]`
    Radius {
        radius: f64,
        sweep: SweepDirection,
    },
    /// Bulge-based arc: `[bulge: 0.3]`
    /// Bulge factor: 0 = straight line, 1 = semicircle, negative = opposite side
    Bulge(f64),
}

impl Default for ArcParams {
    fn default() -> Self {
        // Default: quarter-circle bulge
        ArcParams::Bulge(0.414) // tan(π/8)
    }
}
```

### SweepDirection

```rust
/// Arc sweep direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SweepDirection {
    #[default]
    Clockwise,
    Counterclockwise,
}
```

## Integration with Existing Types

### ShapeType Extension

```rust
/// Built-in shape types (extended)
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

### New StyleKey Variants

```rust
/// Style keys (extended for path modifiers)
pub enum StyleKey {
    // ... existing keys ...

    /// Arc radius (Feature 007)
    Radius,
    /// Arc bulge factor (Feature 007)
    Bulge,
    /// Arc sweep direction (Feature 007)
    Sweep,
    /// Corner rounding for paths (Feature 007)
    Rounded,
    /// Directional position: right offset (Feature 007)
    Right,
    /// Directional position: down offset (Feature 007)
    Down,
    /// Directional position: left offset (Feature 007)
    Left,
    /// Directional position: up offset (Feature 007)
    Up,
}
```

## Entity Relationships

```
PathDecl
  ├── Identifier (name, 0:1)
  ├── PathBody (1:1)
  │     └── PathCommand[] (1:N)
  │           ├── VertexDecl
  │           │     ├── Identifier (name, 1:1)
  │           │     └── VertexPosition (0:1)
  │           ├── LineToDecl
  │           │     ├── Identifier (target, 1:1)
  │           │     └── VertexPosition (0:1)
  │           ├── ArcToDecl
  │           │     ├── Identifier (target, 1:1)
  │           │     ├── VertexPosition (0:1)
  │           │     └── ArcParams (1:1)
  │           ├── Close (marker)
  │           └── CloseArc
  │                 └── ArcParams (1:1)
  └── StyleModifier[] (0:N)

ArcParams (enum)
  ├── Radius
  │     ├── radius: f64 (1:1)
  │     └── sweep: SweepDirection (1:1)
  └── Bulge
        └── bulge: f64 (1:1)
```

## Validation Rules

### Syntactic (Parser)

1. **Vertex name format**: Must match identifier rules (`[a-zA-Z_][a-zA-Z0-9_-]*`)
2. **Path block required**: `path "name" { ... }` must have braces
3. **Position format**: `[x: num, y: num]` or `[right: num, down: num]` etc.
4. **Arc params mutually exclusive**: Cannot specify both `radius` and `bulge`

### Semantic (Post-parse validation)

1. **First vertex at origin**: If first vertex has no position, it's at (0,0)
2. **Subsequent vertices need position**: Non-first vertices without position modifiers on their declaration OR their referencing segment are invalid
3. **Arc radius validity**: Radius must be >= half the chord length (otherwise no arc can connect the points)
4. **Close requires 3+ vertices**: `close` on a path with <3 vertices is a warning (renders as point/line)
5. **Duplicate vertex names**: Warning (later declaration shadows earlier)

## Defaults

| Property | Default Value |
|----------|---------------|
| First vertex position | (0, 0) |
| Arc sweep direction | clockwise |
| Arc bulge (when radius not specified) | 0.414 (≈ tan(22.5°), gentle curve) |
| Path fill | none (transparent) |
| Path stroke | inherit from parent or theme |
