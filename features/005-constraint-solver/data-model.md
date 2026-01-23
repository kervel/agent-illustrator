# Data Model: Constraint-Based Layout System

**Feature**: 005-constraint-solver
**Date**: 2026-01-23

---

## Core Entities

### LayoutVariable

Represents a solvable variable in the constraint system.

```rust
pub struct LayoutVariable {
    pub element_id: String,
    pub property: LayoutProperty,
}

pub enum LayoutProperty {
    // Position (top-left corner)
    X,
    Y,
    // Size
    Width,
    Height,
}
```

**Derived Properties** (computed from X, Y, Width, Height):
- `Left = X`
- `Right = X + Width`
- `Top = Y`
- `Bottom = Y + Height`
- `CenterX = X + Width / 2`
- `CenterY = Y + Height / 2`

---

### LayoutConstraint

Represents a constraint to be solved.

```rust
pub enum LayoutConstraint {
    /// Variable = constant
    /// Example: a.width = 100
    Fixed {
        variable: LayoutVariable,
        value: f64,
        source: ConstraintSource,
    },

    /// Variable = other_variable + offset
    /// Example: a.left = b.left + 20
    Equal {
        left: LayoutVariable,
        right: LayoutVariable,
        offset: f64,
        source: ConstraintSource,
    },

    /// Variable >= value
    /// Example: a.width >= 50
    GreaterOrEqual {
        variable: LayoutVariable,
        value: f64,
        source: ConstraintSource,
    },

    /// Variable <= value
    /// Example: a.height <= 200
    LessOrEqual {
        variable: LayoutVariable,
        value: f64,
        source: ConstraintSource,
    },

    /// Target = (A + B) / 2
    /// Example: a.center_x = midpoint(b.center_x, c.center_x)
    Midpoint {
        target: LayoutVariable,
        a: LayoutVariable,
        b: LayoutVariable,
        source: ConstraintSource,
    },
}
```

---

### ConstraintSource

Tracks where a constraint came from for error reporting.

```rust
pub struct ConstraintSource {
    /// Byte range in source file
    pub span: Span,
    /// Human-readable description
    pub description: String,
    /// Whether from user syntax or generated internally
    pub origin: ConstraintOrigin,
}

pub enum ConstraintOrigin {
    /// User wrote explicit constraint
    UserDefined,
    /// Generated from layout container (row, col, etc.)
    LayoutContainer,
    /// Generated from align statement
    Alignment,
    /// Generated from intrinsic properties (text size, etc.)
    Intrinsic,
}
```

---

### ConstraintSystem

Collection of all constraints for a document.

```rust
pub struct ConstraintSystem {
    /// All variables in the system
    pub variables: HashMap<(String, LayoutProperty), VariableId>,
    /// All constraints to solve
    pub constraints: Vec<LayoutConstraint>,
}

impl ConstraintSystem {
    /// Create from a parsed document
    pub fn from_document(doc: &Document, config: &LayoutConfig) -> Self;

    /// Add intrinsic constraints (shape sizes, text measurements)
    pub fn add_intrinsics(&mut self, doc: &Document);

    /// Add layout container constraints (row/col/grid/stack)
    pub fn add_layout_constraints(&mut self, doc: &Document);

    /// Add user constraints (align, constrain)
    pub fn add_user_constraints(&mut self, doc: &Document);
}
```

---

### Solution

Result of solving the constraint system.

```rust
pub struct Solution {
    /// Computed values for all variables
    pub values: HashMap<(String, LayoutProperty), f64>,
}

impl Solution {
    /// Get value for a variable
    pub fn get(&self, element_id: &str, prop: LayoutProperty) -> Option<f64>;

    /// Get bounding box for an element
    pub fn get_bounds(&self, element_id: &str) -> Option<BoundingBox>;
}
```

---

### SolverError

Errors from the constraint solver.

```rust
pub enum SolverError {
    /// System has no solution
    Unsatisfiable {
        /// Constraints involved in the conflict
        conflicting: Vec<ConstraintSource>,
        /// Explanation of why they conflict
        reason: String,
    },

    /// System is underconstrained (infinite solutions)
    Underconstrained {
        /// Variables without determined values
        free_variables: Vec<LayoutVariable>,
    },

    /// Reference to undefined element
    UndefinedElement {
        name: String,
        span: Span,
        suggestions: Vec<String>,
    },
}
```

---

## AST Extensions

### New ConstraintExpr Type

```rust
/// Expression in a constrain statement
pub enum ConstraintExpr {
    /// a.prop = b.prop
    Equal {
        left: PropertyRef,
        right: PropertyRef,
    },

    /// a.prop = b.prop + offset
    EqualWithOffset {
        left: PropertyRef,
        right: PropertyRef,
        offset: f64,
    },

    /// a.prop = constant
    Constant {
        left: PropertyRef,
        value: f64,
    },

    /// a.center = midpoint(b, c)
    Midpoint {
        target: PropertyRef,
        a: Spanned<Identifier>,
        b: Spanned<Identifier>,
    },

    /// a.prop >= value
    GreaterOrEqual {
        left: PropertyRef,
        value: f64,
    },

    /// a.prop <= value
    LessOrEqual {
        left: PropertyRef,
        value: f64,
    },

    /// container contains a, b, c [padding: 20]
    Contains {
        container: Spanned<Identifier>,
        elements: Vec<Spanned<Identifier>>,
        padding: Option<f64>,
    },
}
```

### PropertyRef

Reference to an element property.

```rust
pub struct PropertyRef {
    pub element: Spanned<ElementPath>,
    pub property: Spanned<ConstraintProperty>,
}

pub enum ConstraintProperty {
    // Position
    X, Y,
    // Size
    Width, Height,
    // Edges
    Left, Right, Top, Bottom,
    // Centers
    CenterX, CenterY, Center,
}
```

---

## Constraint Generation Rules

### From Layout Containers

**Row**:
```
child[0].left = container.left + padding
child[i+1].left = child[i].right + gap
child[n].right <= container.right - padding (if container has fixed width)
```

**Column**:
```
child[0].top = container.top + padding
child[i+1].top = child[i].bottom + gap
child[n].bottom <= container.bottom - padding (if container has fixed height)
```

**Stack**:
```
for each child:
    child.center_x = container.center_x
    child.center_y = container.center_y
```

**Grid**:
```
// Alignment within rows and columns
row_elements[].top = row_top
col_elements[].left = col_left
```

### From Align Statements

```
align a.left = b.left = c.left
=>
a.left = b.left
b.left = c.left
```

### From Constrain Statements

```
constrain a.left = b.right + 20
=>
Equal { left: a.X, right: b.X + b.Width, offset: 20 }
```

---

## State Transitions

```
                 ┌──────────────┐
                 │   Document   │
                 │    (AST)     │
                 └──────┬───────┘
                        │ collect_constraints()
                        ▼
              ┌──────────────────┐
              │ ConstraintSystem │
              │   (unsolved)     │
              └────────┬─────────┘
                       │ solve()
           ┌───────────┴───────────┐
           │                       │
           ▼                       ▼
    ┌──────────┐           ┌─────────────┐
    │ Solution │           │ SolverError │
    └────┬─────┘           └─────────────┘
         │ apply()
         ▼
  ┌──────────────┐
  │ LayoutResult │
  └──────────────┘
```

---

*Created: 2026-01-23*
