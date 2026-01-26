# Data Model: Anchor Support for Shape Connections

## Overview

This document defines the data structures for Feature 009: Anchor Support. Anchors provide named attachment points on shapes where connectors can attach, each with a position and an outward direction.

---

## Core Types

### AnchorDirection

```rust
/// Direction a connector should approach/leave an anchor.
/// Represents the outward normal at the anchor point.
/// Connectors should arrive/depart perpendicular to the shape.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnchorDirection {
    Up,           // 270° - connector comes from above
    Down,         // 90° - connector comes from below
    Left,         // 180° - connector comes from the left
    Right,        // 0° - connector comes from the right
    Angle(f64),   // Custom angle in degrees (0=right, 90=down)
}

impl AnchorDirection {
    /// Convert direction to a unit vector
    pub fn to_vector(&self) -> Point {
        let angle_rad = match self {
            AnchorDirection::Up => 270.0_f64.to_radians(),
            AnchorDirection::Down => 90.0_f64.to_radians(),
            AnchorDirection::Left => 180.0_f64.to_radians(),
            AnchorDirection::Right => 0.0_f64.to_radians(),
            AnchorDirection::Angle(deg) => deg.to_radians(),
        };
        Point::new(angle_rad.cos(), angle_rad.sin())
    }

    /// Infer direction from a constraint property
    pub fn from_property(prop: &ConstraintProperty) -> Self {
        match prop {
            ConstraintProperty::Left => AnchorDirection::Left,
            ConstraintProperty::Right => AnchorDirection::Right,
            ConstraintProperty::Top => AnchorDirection::Up,
            ConstraintProperty::Bottom => AnchorDirection::Down,
            _ => AnchorDirection::Down, // Default for center_x, center_y, etc.
        }
    }
}
```

### Anchor

```rust
/// A named attachment point on a shape with position and direction.
#[derive(Debug, Clone)]
pub struct Anchor {
    /// Identifier for this anchor (e.g., "top", "input")
    pub name: String,

    /// Computed position in document coordinates
    pub position: Point,

    /// Outward normal direction (connectors approach/leave perpendicular)
    pub direction: AnchorDirection,
}

impl Anchor {
    pub fn new(name: impl Into<String>, position: Point, direction: AnchorDirection) -> Self {
        Self {
            name: name.into(),
            position,
            direction,
        }
    }
}
```

### AnchorSet

```rust
/// Collection of anchors for an element.
#[derive(Debug, Clone, Default)]
pub struct AnchorSet {
    anchors: HashMap<String, Anchor>,
}

impl AnchorSet {
    /// Create anchors for simple shapes (rect, ellipse, circle)
    pub fn simple_shape(bounds: &BoundingBox) -> Self {
        Self::from_iter([
            Anchor::new("top", bounds.top_center(), AnchorDirection::Up),
            Anchor::new("bottom", bounds.bottom_center(), AnchorDirection::Down),
            Anchor::new("left", bounds.left_center(), AnchorDirection::Left),
            Anchor::new("right", bounds.right_center(), AnchorDirection::Right),
        ])
    }

    /// Create anchors for path shapes (includes corners)
    pub fn path_shape(bounds: &BoundingBox) -> Self {
        let mut set = Self::simple_shape(bounds);
        set.insert(Anchor::new("top_left", bounds.top_left(), AnchorDirection::Angle(225.0)));
        set.insert(Anchor::new("top_right", bounds.top_right(), AnchorDirection::Angle(315.0)));
        set.insert(Anchor::new("bottom_left", bounds.bottom_left(), AnchorDirection::Angle(135.0)));
        set.insert(Anchor::new("bottom_right", bounds.bottom_right(), AnchorDirection::Angle(45.0)));
        set
    }

    /// Create from custom anchor definitions (for templates)
    pub fn from_custom(anchors: impl IntoIterator<Item = Anchor>) -> Self {
        Self {
            anchors: anchors.into_iter().map(|a| (a.name.clone(), a)).collect(),
        }
    }

    pub fn get(&self, name: &str) -> Option<&Anchor> {
        self.anchors.get(name)
    }

    pub fn insert(&mut self, anchor: Anchor) {
        self.anchors.insert(anchor.name.clone(), anchor);
    }

    pub fn names(&self) -> Vec<&str> {
        self.anchors.keys().map(|s| s.as_str()).collect()
    }
}
```

---

## AST Types

### AnchorReference

```rust
/// Reference to an anchor on an element in connection syntax.
/// Parsed from: `element_name.anchor_name` or just `element_name`
#[derive(Debug, Clone)]
pub struct AnchorReference {
    /// The shape or template instance being referenced
    pub element: Spanned<Identifier>,

    /// The anchor name (None = auto-detect attachment point)
    pub anchor: Option<Spanned<String>>,
}

impl AnchorReference {
    /// Create from just an element name (backward compatibility)
    pub fn element_only(element: Spanned<Identifier>) -> Self {
        Self { element, anchor: None }
    }

    /// Create with explicit anchor
    pub fn with_anchor(element: Spanned<Identifier>, anchor: Spanned<String>) -> Self {
        Self { element, anchor: Some(anchor) }
    }
}
```

### AnchorDecl (for templates)

```rust
/// Anchor declaration in a template definition.
/// Parsed from: `anchor name [position: element.property, direction: dir]`
#[derive(Debug, Clone)]
pub struct AnchorDecl {
    /// Name of this anchor
    pub name: Spanned<Identifier>,

    /// Position specification
    pub position: AnchorPosition,

    /// Optional explicit direction (inferred from position if omitted)
    pub direction: Option<AnchorDirectionSpec>,
}

/// How an anchor position is specified in template syntax.
#[derive(Debug, Clone)]
pub enum AnchorPosition {
    /// Simple property reference: `body.left`
    PropertyRef {
        element: Identifier,
        property: ConstraintProperty,
    },

    /// Property with offset: `body.top + 10`
    Expression {
        element: Identifier,
        property: ConstraintProperty,
        offset: f64,
    },
}

/// Direction specification in template anchor syntax.
#[derive(Debug, Clone)]
pub enum AnchorDirectionSpec {
    Cardinal(CardinalDirection),
    Angle(f64),
}

#[derive(Debug, Clone, Copy)]
pub enum CardinalDirection {
    Up,
    Down,
    Left,
    Right,
}

impl From<CardinalDirection> for AnchorDirection {
    fn from(c: CardinalDirection) -> Self {
        match c {
            CardinalDirection::Up => AnchorDirection::Up,
            CardinalDirection::Down => AnchorDirection::Down,
            CardinalDirection::Left => AnchorDirection::Left,
            CardinalDirection::Right => AnchorDirection::Right,
        }
    }
}
```

### Updated ConnectionDecl

```rust
/// Connection between two elements, optionally with explicit anchors.
#[derive(Debug, Clone)]
pub struct ConnectionDecl {
    /// Source element (optionally with anchor)
    pub from: AnchorReference,   // CHANGED from Spanned<Identifier>

    /// Target element (optionally with anchor)
    pub to: AnchorReference,     // CHANGED from Spanned<Identifier>

    /// Arrow direction
    pub direction: ConnectionDirection,

    /// Style modifiers [stroke: red, routing: curved, etc.]
    pub modifiers: Vec<Spanned<StyleModifier>>,
}
```

---

## Layout Types

### ResolvedAnchor

```rust
/// Fully resolved anchor for use in connection routing.
#[derive(Debug, Clone)]
pub struct ResolvedAnchor {
    pub position: Point,
    pub direction: AnchorDirection,
}
```

### Extended ElementLayout

```rust
/// Element layout with associated anchors.
pub struct ElementLayout {
    // ... existing fields ...
    pub bounds: BoundingBox,
    pub styles: ResolvedStyles,

    /// Computed anchors for this element
    pub anchors: AnchorSet,  // NEW
}
```

---

## Validation Rules

1. **Built-in anchor names are reserved**: Cannot define custom anchors named `top`, `bottom`, `left`, `right` in templates (they would conflict with built-in anchors)

2. **Anchor names must be unique**: Within a template, all anchor names must be distinct

3. **Position references must be valid**: The element referenced in an anchor position must exist within the template scope

4. **Connection anchor references must exist**: When using `element.anchor`, the anchor name must be valid for that element type

---

## Error Types

```rust
pub enum AnchorError {
    /// Anchor name not found on element
    InvalidAnchor {
        element: Identifier,
        anchor: String,
        valid_anchors: Vec<String>,
    },

    /// Reserved anchor name used in template
    ReservedAnchorName {
        name: String,
    },

    /// Duplicate anchor name in template
    DuplicateAnchor {
        name: String,
        first_defined: Span,
    },

    /// Invalid element reference in anchor position
    InvalidElementReference {
        anchor: String,
        element: String,
    },
}
```

---

## Built-in Anchor Reference

### Simple Shapes (rect, ellipse, circle)

| Anchor | Position | Direction |
|--------|----------|-----------|
| `top` | Top edge center | Up (270°) |
| `bottom` | Bottom edge center | Down (90°) |
| `left` | Left edge center | Left (180°) |
| `right` | Right edge center | Right (0°) |

### Path Shapes

All simple shape anchors plus:

| Anchor | Position | Direction |
|--------|----------|-----------|
| `top_left` | Top-left corner | Diagonal (225°) |
| `top_right` | Top-right corner | Diagonal (315°) |
| `bottom_left` | Bottom-left corner | Diagonal (135°) |
| `bottom_right` | Bottom-right corner | Diagonal (45°) |

### Layout Containers (row, col, stack)

Same as simple shapes (computed from container bounding box).

---

## State Transitions

```
┌─────────────┐     ┌──────────────┐     ┌───────────────┐
│   Parse     │ ──► │    Layout    │ ──► │    Routing    │
│ AnchorDecl  │     │ Compute Pos  │     │ Use Anchors   │
│ AnchorRef   │     │ AnchorSet    │     │ ResolvedAnchor│
└─────────────┘     └──────────────┘     └───────────────┘
```

1. **Parse**: AnchorDecl and AnchorReference AST nodes created
2. **Layout**: AnchorSet computed for each element after bounding boxes resolved
3. **Routing**: AnchorReference resolved to ResolvedAnchor (position + direction)
