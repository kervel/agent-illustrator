# Data Model: Layout and Render Pipeline

## Overview

This document defines the data structures that bridge the AST (input) and SVG (output) stages of the rendering pipeline.

---

## Entity: LayoutResult

The complete output of the layout engine, ready for rendering.

### Fields

| Field | Type | Description |
|-------|------|-------------|
| elements | `HashMap<Identifier, ElementLayout>` | Positioned elements by ID |
| connections | `Vec<ConnectionLayout>` | Routed connections |
| bounds | `BoundingBox` | Total illustration bounds |
| config | `LayoutConfig` | Configuration used |

### Relationships
- Contains multiple `ElementLayout` instances
- Contains multiple `ConnectionLayout` instances
- Has one `BoundingBox` for overall bounds

---

## Entity: ElementLayout

Layout information for a single element (shape, layout container, or group).

### Fields

| Field | Type | Description |
|-------|------|-------------|
| id | `Option<Identifier>` | Element identifier (if named) |
| element_type | `ElementType` | Type of element |
| bounds | `BoundingBox` | Computed position and size |
| styles | `ResolvedStyles` | Resolved style properties |
| children | `Vec<ElementLayout>` | Nested elements (for containers) |

### Element Types
```
enum ElementType {
    Shape(ShapeType),    // rect, circle, ellipse, polygon, icon
    Layout(LayoutType),  // row, column, grid, stack
    Group,               // semantic grouping
}
```

---

## Entity: BoundingBox

Spatial extent of an element in the coordinate system.

### Fields

| Field | Type | Description |
|-------|------|-------------|
| x | `f64` | Left edge x-coordinate |
| y | `f64` | Top edge y-coordinate |
| width | `f64` | Horizontal extent |
| height | `f64` | Vertical extent |

### Derived Properties
- `right()` → `x + width`
- `bottom()` → `y + height`
- `center()` → `(x + width/2, y + height/2)`

### Validation Rules
- Width must be > 0
- Height must be > 0
- Coordinates may be negative (for relative positioning)

---

## Entity: ConnectionLayout

Routing information for a connection between elements.

### Fields

| Field | Type | Description |
|-------|------|-------------|
| from_id | `Identifier` | Source element |
| to_id | `Identifier` | Target element |
| direction | `ConnectionDirection` | Arrow direction |
| path | `Vec<Point>` | Waypoints from start to end |
| styles | `ResolvedStyles` | Line and arrow styles |
| label | `Option<LabelLayout>` | Connection label if present |

### Path Structure
- First point: attachment on source element edge
- Last point: attachment on target element edge
- Intermediate points: routing waypoints

---

## Entity: Point

A 2D coordinate.

### Fields

| Field | Type | Description |
|-------|------|-------------|
| x | `f64` | X-coordinate |
| y | `f64` | Y-coordinate |

---

## Entity: ResolvedStyles

Fully resolved style properties ready for SVG output.

### Fields

| Field | Type | Description |
|-------|------|-------------|
| fill | `Option<Color>` | Fill color |
| stroke | `Option<Color>` | Stroke color |
| stroke_width | `Option<f64>` | Stroke width |
| opacity | `Option<f64>` | Opacity (0.0-1.0) |
| font_size | `Option<f64>` | Text font size |
| css_classes | `Vec<String>` | CSS class names |

### Default Resolution
- Fill: `#f0f0f0` (light gray)
- Stroke: `#333333` (dark gray)
- Stroke width: `2.0`
- Opacity: `1.0`
- Font size: `14.0`

---

## Entity: LabelLayout

Positioned label text.

### Fields

| Field | Type | Description |
|-------|------|-------------|
| text | `String` | Label content |
| position | `Point` | Text anchor position |
| anchor | `TextAnchor` | Alignment (start, middle, end) |
| styles | `ResolvedStyles` | Text styles |

---

## Entity: LayoutConfig

Configuration options for the layout engine.

### Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| default_rect_size | `(f64, f64)` | `(100.0, 50.0)` | Default rectangle dimensions |
| default_circle_radius | `f64` | `30.0` | Default circle radius |
| element_spacing | `f64` | `20.0` | Gap between sibling elements |
| container_padding | `f64` | `10.0` | Padding inside containers |
| connection_spacing | `f64` | `10.0` | Min distance from shapes to routes |
| viewbox_padding | `f64` | `20.0` | Padding around illustration |

---

## Entity: LayoutError

Error information from layout processing.

### Variants

| Variant | Fields | Description |
|---------|--------|-------------|
| UndefinedIdentifier | `name`, `span`, `suggestions` | Reference to non-existent element |
| ConflictingConstraints | `constraints`, `reason` | Unsatisfiable constraints |
| CircularConstraint | `cycle` | Constraint dependency cycle |
| InvalidLayout | `element`, `reason` | Layout container error |

---

## State Transitions

### Layout Pipeline States

```
Document (AST)
    ↓ validate_references()
ValidatedDocument
    ↓ compute_initial_layout()
InitialLayout
    ↓ resolve_constraints()
ConstrainedLayout
    ↓ route_connections()
LayoutResult
    ↓ render_svg()
String (SVG)
```

### Element State During Layout

```
Unpositioned → Sized → Positioned → Styled
```

---

## Relationships Diagram

```
LayoutResult
├── elements: HashMap<Identifier, ElementLayout>
│   └── ElementLayout
│       ├── bounds: BoundingBox
│       ├── styles: ResolvedStyles
│       └── children: Vec<ElementLayout>
├── connections: Vec<ConnectionLayout>
│   └── ConnectionLayout
│       ├── path: Vec<Point>
│       ├── styles: ResolvedStyles
│       └── label: Option<LabelLayout>
└── bounds: BoundingBox
```

---

*Created: 2026-01-23*
