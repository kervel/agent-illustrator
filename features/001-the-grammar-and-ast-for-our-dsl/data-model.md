# Data Model: Grammar and AST for the Agent Illustrator DSL

## Overview

This document defines the Abstract Syntax Tree (AST) data structures for the Agent Illustrator DSL. All types use Rust idioms with clear ownership semantics.

## Core Types

### Span (Source Location)

```rust
/// Byte range in source text
pub type Span = std::ops::Range<usize>;

/// AST node with source location
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}
```

### Document (Root Node)

```rust
/// Root AST node - a complete illustration document
pub struct Document {
    pub statements: Vec<Spanned<Statement>>,
}
```

### Statement

```rust
/// Top-level statement in a document
pub enum Statement {
    /// Shape declaration: `rect "name" [styles]`
    Shape(ShapeDecl),

    /// Connection: `a -> b [styles]`
    Connection(ConnectionDecl),

    /// Layout container: `row { ... }`
    Layout(LayoutDecl),

    /// Semantic group: `group "name" { ... }`
    Group(GroupDecl),

    /// Position constraint: `place a right-of b`
    Constraint(ConstraintDecl),

    /// Comment (preserved for round-tripping)
    Comment(String),
}
```

## Shape Types

### ShapeDecl

```rust
/// Shape declaration
pub struct ShapeDecl {
    pub shape_type: Spanned<ShapeType>,
    pub name: Option<Spanned<Identifier>>,
    pub modifiers: Vec<Spanned<StyleModifier>>,
}

/// Built-in shape types
pub enum ShapeType {
    // Geometric primitives
    Rectangle,
    Circle,
    Ellipse,
    Line,
    Polygon,

    // Semantic shapes
    Icon { icon_name: String },
}
```

### Identifier

```rust
/// Valid identifier (alphanumeric + underscore, starts with letter/_)
pub struct Identifier(pub String);

impl Identifier {
    /// Validates identifier format
    pub fn new(s: &str) -> Result<Self, ParseError>;
}
```

## Connection Types

### ConnectionDecl

```rust
/// Connection between shapes
pub struct ConnectionDecl {
    pub from: Spanned<Identifier>,
    pub to: Spanned<Identifier>,
    pub direction: ConnectionDirection,
    pub modifiers: Vec<Spanned<StyleModifier>>,
}

/// Connection directionality
pub enum ConnectionDirection {
    /// `->` directed from source to target
    Forward,
    /// `<-` directed from target to source
    Backward,
    /// `<->` bidirectional
    Bidirectional,
    /// `--` undirected
    Undirected,
}
```

## Layout Types

### LayoutDecl

```rust
/// Layout container
pub struct LayoutDecl {
    pub layout_type: Spanned<LayoutType>,
    pub name: Option<Spanned<Identifier>>,
    pub children: Vec<Spanned<Statement>>,
    pub modifiers: Vec<Spanned<StyleModifier>>,
}

/// Layout arrangement strategies
pub enum LayoutType {
    /// Horizontal arrangement
    Row,
    /// Vertical arrangement
    Column,
    /// Grid arrangement (may have cols/rows modifiers)
    Grid,
    /// Stacked/layered arrangement
    Stack,
}
```

### GroupDecl

```rust
/// Semantic group (no layout implication)
pub struct GroupDecl {
    pub name: Option<Spanned<Identifier>>,
    pub children: Vec<Spanned<Statement>>,
    pub modifiers: Vec<Spanned<StyleModifier>>,
}
```

## Constraint Types

### ConstraintDecl

```rust
/// Position constraint (experimental)
pub struct ConstraintDecl {
    pub subject: Spanned<Identifier>,
    pub relation: Spanned<PositionRelation>,
    pub anchor: Spanned<Identifier>,
}

/// Relative position relations
pub enum PositionRelation {
    RightOf,
    LeftOf,
    Above,
    Below,
    Inside,
    // Future: could add distance modifiers
}
```

## Style Types

### StyleModifier

```rust
/// Key-value style modifier
pub struct StyleModifier {
    pub key: Spanned<StyleKey>,
    pub value: Spanned<StyleValue>,
}

/// Known style keys (extensible)
pub enum StyleKey {
    Fill,
    Stroke,
    StrokeWidth,
    Opacity,
    Label,
    FontSize,
    // CSS class for custom styling
    Class,
    // Catch-all for unknown keys (forward compatibility)
    Custom(String),
}

/// Style values
pub enum StyleValue {
    /// Named color or CSS color
    Color(String),
    /// Numeric value (with optional unit)
    Number { value: f64, unit: Option<String> },
    /// String value (for labels, classes)
    String(String),
    /// Keyword (bold, dashed, etc.)
    Keyword(String),
}
```

## Entity Relationships

```
Document
  └── Statement[] (1:N)
        ├── ShapeDecl
        │     ├── ShapeType (1:1)
        │     ├── Identifier (0:1)
        │     └── StyleModifier[] (0:N)
        │
        ├── ConnectionDecl
        │     ├── Identifier (from) (1:1)
        │     ├── Identifier (to) (1:1)
        │     ├── ConnectionDirection (1:1)
        │     └── StyleModifier[] (0:N)
        │
        ├── LayoutDecl
        │     ├── LayoutType (1:1)
        │     ├── Identifier (0:1)
        │     ├── Statement[] (children) (0:N) ← recursive
        │     └── StyleModifier[] (0:N)
        │
        ├── GroupDecl
        │     ├── Identifier (0:1)
        │     ├── Statement[] (children) (0:N) ← recursive
        │     └── StyleModifier[] (0:N)
        │
        ├── ConstraintDecl
        │     ├── Identifier (subject) (1:1)
        │     ├── PositionRelation (1:1)
        │     └── Identifier (anchor) (1:1)
        │
        └── Comment (preserved text)
```

## Validation Rules

These are **syntactic** rules enforced during parsing:

1. **Identifier Format**: Must match `[a-zA-Z_][a-zA-Z0-9_]*`
2. **Non-empty Containers**: Layout and Group must have `{}` even if empty
3. **Connection Endpoints**: Both `from` and `to` must be valid identifiers
4. **Style Modifier Format**: Must be `key: value` pairs within `[]`

**Semantic** validation (not in this feature) would check:
- Reference resolution (do named shapes exist?)
- Circular group containment
- Conflicting constraints
