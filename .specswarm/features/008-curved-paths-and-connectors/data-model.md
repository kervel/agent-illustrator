# Data Model: Curved Paths and Connectors

## AST Extensions

### CurveToDecl (New)

A path command representing a quadratic Bezier curve segment.

```rust
pub struct CurveToDecl {
    /// Target vertex name (endpoint of the curve)
    pub target: Identifier,

    /// Optional steering vertex reference (control point)
    /// When None, system auto-generates control point
    pub via: Option<Identifier>,

    /// Optional position modifiers for the target
    pub position: Option<VertexPosition>,
}
```

**Relationships**:
- Referenced by: `PathCommand::CurveTo(CurveToDecl)`
- References: `Identifier` (target vertex), `Identifier` (via element)

### PathCommand (Extended)

```rust
pub enum PathCommand {
    Vertex(VertexDecl),
    LineTo(LineToDecl),
    ArcTo(ArcToDecl),
    CurveTo(CurveToDecl),  // NEW
    Close,
    CloseArc(ArcParams),
}
```

---

## Layout Extensions

### RoutingMode (Extended)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RoutingMode {
    /// Straight line from source to target
    Direct,

    /// S-shaped paths with horizontal/vertical segments
    #[default]
    Orthogonal,

    /// Smooth curved path using quadratic Bezier
    Curved,  // NEW
}
```

### ConnectionLayout (Extended)

```rust
pub struct ConnectionLayout {
    /// Waypoints along the connection path
    pub path: Vec<Point>,

    /// Control points for curved segments (NEW)
    /// - Empty for Direct/Orthogonal routing
    /// - One point per curved segment for Curved routing
    pub control_points: Vec<Point>,

    /// Arrow markers
    pub arrow_start: bool,
    pub arrow_end: bool,

    /// Visual styling
    pub styles: StyleMap,

    /// Optional label
    pub label: Option<LabelLayout>,
}
```

---

## Renderer Extensions

### PathSegment (Extended)

```rust
pub enum PathSegment {
    /// Move to absolute position
    MoveTo(Point),

    /// Line to absolute position
    LineTo(Point),

    /// Arc to position with radius parameters
    ArcTo {
        end: Point,
        radius: f64,
        large_arc: bool,
        sweep: bool,
    },

    /// Quadratic Bezier curve (NEW)
    /// SVG: Q cx cy, ex ey
    QuadraticTo {
        control: Point,
        end: Point,
    },

    /// Smooth quadratic continuation (NEW)
    /// SVG: T ex ey
    /// Control point is reflection of previous segment's control
    SmoothQuadraticTo(Point),

    /// Close path
    Close,
}
```

---

## Modifier Syntax

### Via Modifier

Used in both path commands and connection declarations.

```
[via: element_name]              // Single control point
[via: elem1, elem2]              // Multiple control points (chained)
[via: elem1, elem2, elem3]       // Three-point spline
```

**Parsed as**:
- Key: `StyleKey::Custom("via")`
- Value: `StyleValue::Identifier(id)` or list of identifiers

### Routing Modifier (Extended)

```
[routing: direct]       // Existing
[routing: orthogonal]   // Existing
[routing: curved]       // NEW
```

---

## Entity Relationships

```
┌──────────────────┐     ┌──────────────────┐
│   PathDecl       │     │   ConnectionDecl │
│                  │     │                  │
│  body: Vec<      │     │  modifiers: Vec< │
│   PathCommand>   │     │   StyleModifier> │
└────────┬─────────┘     └────────┬─────────┘
         │                        │
         │ contains               │ contains
         ▼                        ▼
┌──────────────────┐     ┌──────────────────┐
│  CurveToDecl     │     │  [via: ref]      │
│                  │     │  modifier        │
│  via: Option<    │     │                  │
│   Identifier>    │     │  value:          │
└────────┬─────────┘     │   Identifier     │
         │               └────────┬─────────┘
         │ references             │ references
         ▼                        ▼
┌──────────────────────────────────────────┐
│              ElementLayout               │
│                                          │
│  bounds: BoundingBox                     │
│  -> center() used as control point       │
└──────────────────────────────────────────┘
```

---

## State Transitions

### CurveToDecl Processing

```
Parse Phase:
  Input: "curve_to target [via: ctrl]"
  Output: CurveToDecl { target, via: Some(ctrl), position: None }

Layout Phase:
  1. Resolve target vertex position
  2. If via present:
     a. Resolve via element by name
     b. Get element's bounding box center
     c. Use as control point
  3. If via absent:
     a. Compute default control point
     b. perpendicular offset from chord midpoint
  Output: PathSegment::QuadraticTo { control, end }

Render Phase:
  Input: PathSegment::QuadraticTo { control: (50, 20), end: (100, 50) }
  Output: "Q 50 20, 100 50"
```

### Curved Connection Processing

```
Parse Phase:
  Input: "a -> b [routing: curved, via: c]"
  Output: ConnectionDecl with modifiers [routing: curved, via: c]

Layout Phase:
  1. Extract routing mode: Curved
  2. Extract via references: [c]
  3. Resolve element 'c' -> Point (cx, cy)
  4. Compute connection path with control points
  Output: ConnectionLayout {
    path: [start, end],
    control_points: [(cx, cy)],
    ...
  }

Render Phase:
  Input: path=[p1, p2], control_points=[c1]
  Output: <path d="M p1.x p1.y Q c1.x c1.y, p2.x p2.y" .../>
```

---

## Validation Rules

| Rule | Enforcement Point | Error |
|------|-------------------|-------|
| Via reference must exist | Layout phase | "Steering vertex 'X' not found" |
| Via element must have position | Layout phase | "Steering vertex 'X' has no position" |
| curve_to requires target | Parser | "Expected identifier after 'curve_to'" |
| Via list cannot be empty | Parser | "Expected identifier after 'via:'" |
| No self-referential via | Layout phase | "Element cannot reference itself as steering vertex" |
