# Data Model: AIL Format Ergonomics

## Overview

This document defines the new and modified AST data structures for the AIL format ergonomics feature. These types enable cross-hierarchy alignment, unified label handling, and shape-based connection labels.

---

## New Types

### ElementPath

A dot-separated path to an element, potentially nested within groups.

```rust
/// Path to an element through the group hierarchy
/// Examples: "my_element", "group1.item", "outer.inner.shape"
#[derive(Debug, Clone, PartialEq)]
pub struct ElementPath {
    /// Path segments (identifiers separated by dots)
    pub segments: Vec<Spanned<Identifier>>,
}

impl ElementPath {
    /// Create a simple path (single segment)
    pub fn simple(id: Identifier, span: Span) -> Self {
        Self {
            segments: vec![Spanned::new(id, span)],
        }
    }

    /// Get the final segment (leaf element name)
    pub fn leaf(&self) -> &Identifier {
        &self.segments.last().unwrap().node
    }

    /// Check if this is a simple (single-segment) path
    pub fn is_simple(&self) -> bool {
        self.segments.len() == 1
    }

    /// Format as dot-separated string
    pub fn to_string(&self) -> String {
        self.segments
            .iter()
            .map(|s| s.node.as_str())
            .collect::<Vec<_>>()
            .join(".")
    }
}
```

### Edge

Alignment edge on an element's bounding box.

```rust
/// Alignment edge types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    // Horizontal axis (affects x-coordinate)
    Left,
    HorizontalCenter,
    Right,

    // Vertical axis (affects y-coordinate)
    Top,
    VerticalCenter,
    Bottom,
}

impl Edge {
    /// Returns true if this edge is horizontal (affects x-position)
    pub fn is_horizontal(&self) -> bool {
        matches!(self, Edge::Left | Edge::HorizontalCenter | Edge::Right)
    }

    /// Returns true if this edge is vertical (affects y-position)
    pub fn is_vertical(&self) -> bool {
        matches!(self, Edge::Top | Edge::VerticalCenter | Edge::Bottom)
    }

    /// Get axis type for compatibility checking
    pub fn axis(&self) -> Axis {
        if self.is_horizontal() {
            Axis::Horizontal
        } else {
            Axis::Vertical
        }
    }
}

/// Axis type for alignment compatibility
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}
```

### AlignmentAnchor

An alignment reference point: element path plus edge.

```rust
/// A specific alignment anchor point
#[derive(Debug, Clone, PartialEq)]
pub struct AlignmentAnchor {
    /// Path to the element
    pub element: Spanned<ElementPath>,
    /// Edge of the element to align
    pub edge: Spanned<Edge>,
}
```

### AlignmentDecl

Alignment constraint declaration.

```rust
/// Alignment constraint: aligns edges of multiple elements
/// Example: align a.left = b.left = c.left
#[derive(Debug, Clone, PartialEq)]
pub struct AlignmentDecl {
    /// Anchors to align (at least 2)
    pub anchors: Vec<AlignmentAnchor>,
}

impl AlignmentDecl {
    /// Check that all anchors are on the same axis
    pub fn is_valid(&self) -> bool {
        if self.anchors.len() < 2 {
            return false;
        }
        let first_axis = self.anchors[0].edge.node.axis();
        self.anchors.iter().all(|a| a.edge.node.axis() == first_axis)
    }

    /// Get the axis of this alignment
    pub fn axis(&self) -> Option<Axis> {
        self.anchors.first().map(|a| a.edge.node.axis())
    }
}
```

### Role

Role modifier value for shapes.

```rust
/// Shape roles within containers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Default content role
    Content,
    /// Label role - positioned specially as container header/title
    Label,
}
```

---

## Modified Types

### StyleKey Extensions

```rust
/// Known style keys (extended)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StyleKey {
    // Existing keys...
    Fill,
    Stroke,
    StrokeWidth,
    Opacity,
    Label,
    LabelPosition,
    FontSize,
    Class,
    Gap,
    Size,
    Width,
    Height,
    Routing,
    Custom(String),

    // NEW: Role modifier for shape positioning
    Role,
}
```

### StyleValue Extensions

```rust
/// Style values (extended)
#[derive(Debug, Clone, PartialEq)]
pub enum StyleValue {
    // Existing variants...
    Color(String),
    Number { value: f64, unit: Option<String> },
    String(String),
    Keyword(String),

    // NEW: Identifier reference (for label: my_shape)
    Identifier(Identifier),
}
```

### Statement Extensions

```rust
/// Top-level statement in a document (extended)
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    // Existing variants...
    Shape(ShapeDecl),
    Connection(ConnectionDecl),
    Layout(LayoutDecl),
    Group(GroupDecl),
    Constraint(ConstraintDecl),
    Label(Box<Statement>),  // Deprecated, kept for backward compatibility

    // NEW: Alignment constraint
    Alignment(AlignmentDecl),
}
```

---

## Layout Types

### ResolvedAlignment

Intermediate representation after path resolution.

```rust
/// Alignment constraint with resolved element references
#[derive(Debug, Clone)]
pub struct ResolvedAlignment {
    /// Element identifiers to align (resolved from paths)
    pub elements: Vec<String>,
    /// Edge type for alignment
    pub edge: Edge,
    /// Source span for error reporting
    pub span: Span,
}

impl ResolvedAlignment {
    /// Get alignment axis
    pub fn axis(&self) -> Axis {
        self.edge.axis()
    }
}
```

### AlignmentResult

Result of alignment resolution.

```rust
/// Information about applied alignment for debugging/tracing
#[derive(Debug, Clone)]
pub struct AlignmentResult {
    /// Elements that were aligned
    pub elements: Vec<String>,
    /// Computed alignment coordinate
    pub coordinate: f64,
    /// Axis of alignment
    pub axis: Axis,
}
```

---

## Entity Relationships

```
Document
  └── Statement[] (1:N)
        ├── ... (existing variants)
        │
        └── Alignment(AlignmentDecl)                    ← NEW
              └── AlignmentAnchor[] (2:N)
                    ├── ElementPath (1:1)
                    │     └── Identifier[] segments (1:N)
                    └── Edge (1:1)

ElementPath
  └── Identifier[] segments
        └── Each segment is a group/element name in hierarchy

StyleKey
  └── ... existing variants
  └── Role                                              ← NEW

StyleValue
  └── ... existing variants
  └── Identifier(Identifier)                            ← NEW
```

---

## Validation Rules

### Syntactic Rules (Parser-Enforced)

1. **Element Path Format**: At least one segment
2. **Alignment Minimum**: At least 2 anchors in an alignment
3. **Edge Keywords**: Must be valid edge keyword

### Semantic Rules (Layout Engine)

1. **Alignment Axis Consistency**: All anchors in an alignment must be on same axis (horizontal or vertical)
2. **Path Resolution**: All path segments must resolve to existing named elements
3. **No Circular Alignments**: Alignment graph must be acyclic
4. **Role Value**: Must be valid role keyword (label, content)

---

## Grammar Additions

```ebnf
(* Element path *)
element_path = identifier { "." identifier } ;

(* Alignment edges *)
edge = "left" | "right" | "horizontal_center"
     | "top" | "bottom" | "vertical_center" ;

(* Alignment anchor: path + edge *)
alignment_anchor = element_path "." edge ;

(* Alignment statement *)
alignment_decl = "align" alignment_anchor { "=" alignment_anchor } ;

(* Role modifier value *)
role_value = "label" | "content" ;

(* Extended modifier value to include identifiers *)
modifier_value = color_value
               | number_value
               | string_literal
               | keyword
               | identifier ;        (* NEW: for label: my_shape *)
```

---

## Example AST Structures

### Simple Alignment

Input:
```
align header.left = sidebar.left
```

AST:
```rust
Statement::Alignment(AlignmentDecl {
    anchors: vec![
        AlignmentAnchor {
            element: Spanned::new(
                ElementPath { segments: vec![Identifier("header")] },
                0..6
            ),
            edge: Spanned::new(Edge::Left, 7..11),
        },
        AlignmentAnchor {
            element: Spanned::new(
                ElementPath { segments: vec![Identifier("sidebar")] },
                14..21
            ),
            edge: Spanned::new(Edge::Left, 22..26),
        },
    ],
})
```

### Nested Path Alignment

Input:
```
align panel1.header.horizontal_center = panel2.header.horizontal_center
```

AST:
```rust
Statement::Alignment(AlignmentDecl {
    anchors: vec![
        AlignmentAnchor {
            element: Spanned::new(
                ElementPath {
                    segments: vec![
                        Identifier("panel1"),
                        Identifier("header"),
                    ]
                },
                span
            ),
            edge: Spanned::new(Edge::HorizontalCenter, span),
        },
        // ... second anchor similar
    ],
})
```

### Role-Based Label

Input:
```
group mygroup {
    text "Title" [role: label]
    rect content
}
```

The text shape has StyleModifier:
```rust
StyleModifier {
    key: Spanned::new(StyleKey::Role, span),
    value: Spanned::new(StyleValue::Keyword("label".to_string()), span),
}
```

---

*Created: 2026-01-23*
