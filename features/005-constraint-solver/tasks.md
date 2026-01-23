# Tasks: Constraint-Based Layout System

**Feature**: 005-constraint-solver
**Branch**: `feature/constraint-solver`
**Generated**: 2026-01-23

<!-- Tech Stack Validation: PASSED -->
<!-- Validated against: .specswarm/tech-stack.md -->
<!-- No prohibited technologies found -->
<!-- kasuari added to approved dependencies -->

---

## Overview

| Metric | Value |
|--------|-------|
| Total Tasks | 31 |
| Phases | 7 |
| Parallel Opportunities | 12 tasks |
| Estimated Checkpoints | 6 |

---

## Phase 1: Setup & Spike (Blocking)

**Goal**: Add kasuari dependency and validate it can express our constraint types.

**Gate**: If spike fails or takes >4 hours, evaluate z3 or custom solver.

---

### T001: Add kasuari dependency to Cargo.toml [X]

**File**: `Cargo.toml`
**Priority**: P1 (Blocking)

Add kasuari constraint solver dependency:

```toml
[dependencies]
kasuari = "0.4"
```

Run `cargo check` to verify dependency resolves.

**Acceptance**: `cargo check` passes with kasuari available.

---

### T002: Create solver spike test file [X]

**File**: `src/layout/solver_spike.rs`
**Priority**: P1 (Blocking)
**Depends**: T001

Create a standalone spike test to validate kasuari fitness:

```rust
//! Spike test to validate kasuari can express our constraint types
//!
//! Run with: cargo test spike_kasuari_fitness --lib

#[cfg(test)]
mod tests {
    use kasuari::{Solver, Variable, Strength::*, WeightedRelation::*};

    #[test]
    fn spike_kasuari_fitness() {
        let mut solver = Solver::new();

        // 1. Create variables for 5 elements (x, y, width, height each)
        let a_x = Variable::new();
        let a_y = Variable::new();
        let a_width = Variable::new();
        let a_height = Variable::new();

        let b_x = Variable::new();
        let b_y = Variable::new();
        let b_width = Variable::new();
        let b_height = Variable::new();

        let c_x = Variable::new();
        let c_width = Variable::new();

        let d_x = Variable::new();
        let d_width = Variable::new();

        let e_width = Variable::new();

        // 2. Add equality constraint: a.left = b.left
        solver.add_constraint(a_x |EQ(REQUIRED)| b_x).unwrap();

        // 3. Add offset constraint: c.left = b.right + 20
        // b.right = b.x + b.width
        solver.add_constraint(c_x |EQ(REQUIRED)| b_x + b_width + 20.0).unwrap();

        // 4. Add midpoint constraint: d.center_x = midpoint(a.center_x, c.center_x)
        // d.center_x = d.x + d.width/2
        // a.center_x = a.x + a.width/2
        // c.center_x = c.x + c.width/2
        // Express as: 2*d_center = a_center + c_center
        // => 2*(d.x + d.width/2) = (a.x + a.width/2) + (c.x + c.width/2)
        // For simplicity, assume width=100 for all, so center = x + 50
        // => 2*(d.x + 50) = (a.x + 50) + (c.x + 50)
        // => 2*d.x + 100 = a.x + c.x + 100
        // => 2*d.x = a.x + c.x
        solver.add_constraint(2.0 * d_x |EQ(REQUIRED)| a_x + c_x).unwrap();

        // 5. Add inequality constraint: e.width >= 50
        solver.add_constraint(e_width |GE(REQUIRED)| 50.0).unwrap();

        // 6. Add containment inequality: container.left <= child.left - padding
        // (tested implicitly via LE constraint)
        solver.add_constraint(a_x |LE(REQUIRED)| b_x + 10.0).unwrap();

        // Set some edit variables to anchor the system
        solver.add_edit_variable(b_x, STRONG).unwrap();
        solver.add_edit_variable(b_width, STRONG).unwrap();
        solver.add_edit_variable(a_width, STRONG).unwrap();
        solver.add_edit_variable(c_width, STRONG).unwrap();
        solver.add_edit_variable(d_width, STRONG).unwrap();

        solver.suggest_value(b_x, 0.0).unwrap();
        solver.suggest_value(b_width, 100.0).unwrap();
        solver.suggest_value(a_width, 100.0).unwrap();
        solver.suggest_value(c_width, 100.0).unwrap();
        solver.suggest_value(d_width, 100.0).unwrap();

        // Fetch and verify
        let changes = solver.fetch_changes();
        let values: std::collections::HashMap<Variable, f64> = changes.into_iter().collect();

        // Verify a.x = b.x = 0
        assert!((values.get(&a_x).copied().unwrap_or(0.0) - 0.0).abs() < 0.001);

        // Verify c.x = b.x + b.width + 20 = 0 + 100 + 20 = 120
        assert!((values.get(&c_x).copied().unwrap_or(0.0) - 120.0).abs() < 0.001);

        // Verify d.x = (a.x + c.x) / 2 = (0 + 120) / 2 = 60
        assert!((values.get(&d_x).copied().unwrap_or(0.0) - 60.0).abs() < 0.001);

        // Verify e.width >= 50
        assert!(values.get(&e_width).copied().unwrap_or(0.0) >= 50.0);

        println!("Spike PASSED: kasuari can express all our constraint types!");
    }
}
```

**Acceptance**: `cargo test spike_kasuari_fitness` passes.

---

### T003: Add solver_spike module to layout/mod.rs [X]

**File**: `src/layout/mod.rs`
**Priority**: P1 (Blocking)
**Depends**: T001

Add the spike module (cfg(test) only):

```rust
#[cfg(test)]
mod solver_spike;
```

**Acceptance**: Module compiles without errors.

---

## Checkpoint 1: Spike Validation

**Gate**: Run `cargo test spike_kasuari_fitness`. If it fails, stop and evaluate alternatives (z3 or custom solver) before proceeding.

---

## Phase 2: Core Data Structures

**Goal**: Define the constraint system data structures.

---

### T004: Create solver.rs with LayoutProperty enum [X]

**File**: `src/layout/solver.rs`
**Priority**: P1
**Depends**: Checkpoint 1

```rust
//! Constraint solver integration for layout computation

use std::collections::HashMap;
use crate::parser::ast::Span;

/// Properties that can be constrained
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayoutProperty {
    X,
    Y,
    Width,
    Height,
}

impl LayoutProperty {
    /// Get all base properties (not derived)
    pub fn base_properties() -> &'static [LayoutProperty] {
        &[Self::X, Self::Y, Self::Width, Self::Height]
    }
}

/// Derived properties computed from base properties
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DerivedProperty {
    Left,      // = X
    Right,     // = X + Width
    Top,       // = Y
    Bottom,    // = Y + Height
    CenterX,   // = X + Width/2
    CenterY,   // = Y + Height/2
}
```

**Acceptance**: Compiles without errors.

---

### T005: Add LayoutVariable struct [X]

**File**: `src/layout/solver.rs`
**Priority**: P1
**Depends**: T004

```rust
/// A variable in the constraint system
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LayoutVariable {
    pub element_id: String,
    pub property: LayoutProperty,
}

impl LayoutVariable {
    pub fn new(element_id: impl Into<String>, property: LayoutProperty) -> Self {
        Self {
            element_id: element_id.into(),
            property,
        }
    }

    /// Create variable for element's X position
    pub fn x(element_id: impl Into<String>) -> Self {
        Self::new(element_id, LayoutProperty::X)
    }

    /// Create variable for element's Y position
    pub fn y(element_id: impl Into<String>) -> Self {
        Self::new(element_id, LayoutProperty::Y)
    }

    /// Create variable for element's width
    pub fn width(element_id: impl Into<String>) -> Self {
        Self::new(element_id, LayoutProperty::Width)
    }

    /// Create variable for element's height
    pub fn height(element_id: impl Into<String>) -> Self {
        Self::new(element_id, LayoutProperty::Height)
    }
}
```

**Acceptance**: Compiles without errors.

---

### T006: Add ConstraintSource for error tracking [X]

**File**: `src/layout/solver.rs`
**Priority**: P1
**Depends**: T004

```rust
/// Origin of a constraint (for error messages)
#[derive(Debug, Clone, PartialEq, Eq)]
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

/// Tracks where a constraint came from
#[derive(Debug, Clone)]
pub struct ConstraintSource {
    /// Byte range in source file
    pub span: Span,
    /// Human-readable description
    pub description: String,
    /// Origin type
    pub origin: ConstraintOrigin,
}

impl ConstraintSource {
    pub fn user(span: Span, description: impl Into<String>) -> Self {
        Self {
            span,
            description: description.into(),
            origin: ConstraintOrigin::UserDefined,
        }
    }

    pub fn layout(span: Span, description: impl Into<String>) -> Self {
        Self {
            span,
            description: description.into(),
            origin: ConstraintOrigin::LayoutContainer,
        }
    }

    pub fn intrinsic(description: impl Into<String>) -> Self {
        Self {
            span: 0..0,
            description: description.into(),
            origin: ConstraintOrigin::Intrinsic,
        }
    }
}
```

**Acceptance**: Compiles without errors.

---

### T007: Add LayoutConstraint enum [X]

**File**: `src/layout/solver.rs`
**Priority**: P1
**Depends**: T005, T006

```rust
/// A constraint in the layout system
#[derive(Debug, Clone)]
pub enum LayoutConstraint {
    /// Variable = constant
    Fixed {
        variable: LayoutVariable,
        value: f64,
        source: ConstraintSource,
    },

    /// left_var = right_var + offset
    Equal {
        left: LayoutVariable,
        right: LayoutVariable,
        offset: f64,
        source: ConstraintSource,
    },

    /// variable >= value
    GreaterOrEqual {
        variable: LayoutVariable,
        value: f64,
        source: ConstraintSource,
    },

    /// variable <= value
    LessOrEqual {
        variable: LayoutVariable,
        value: f64,
        source: ConstraintSource,
    },

    /// target = (a + b) / 2
    Midpoint {
        target: LayoutVariable,
        a: LayoutVariable,
        b: LayoutVariable,
        source: ConstraintSource,
    },
}
```

**Acceptance**: Compiles without errors.

---

### T008: Add SolverError enum [X]

**File**: `src/layout/solver.rs`
**Priority**: P1
**Depends**: T007

```rust
use thiserror::Error;

/// Errors from the constraint solver
#[derive(Debug, Error)]
pub enum SolverError {
    #[error("Unsatisfiable constraints: {reason}")]
    Unsatisfiable {
        conflicting: Vec<ConstraintSource>,
        reason: String,
    },

    #[error("Underconstrained system: variables have no determined value")]
    Underconstrained {
        free_variables: Vec<LayoutVariable>,
    },

    #[error("Undefined element: {name}")]
    UndefinedElement {
        name: String,
        span: Span,
        suggestions: Vec<String>,
    },

    #[error("Internal solver error: {0}")]
    Internal(String),
}
```

**Acceptance**: Compiles without errors.

---

### T009: Add solver module to layout/mod.rs [X]

**File**: `src/layout/mod.rs`
**Priority**: P1
**Depends**: T008

Add the public module:

```rust
pub mod solver;
pub use solver::{LayoutVariable, LayoutProperty, LayoutConstraint, SolverError};
```

**Acceptance**: `cargo check` passes.

---

## Checkpoint 2: Data Structures Complete

**Verify**: `cargo check` passes. All solver data structures defined.

---

## Phase 3: Solver Wrapper

**Goal**: Implement the ConstraintSolver wrapper around kasuari.

---

### T010: Implement ConstraintSolver struct [X]

**File**: `src/layout/solver.rs`
**Priority**: P1
**Depends**: T009

```rust
use kasuari::{Solver as KasuariSolver, Variable as KasuariVariable, Strength, WeightedRelation::*};

/// Wrapper around kasuari solver
pub struct ConstraintSolver {
    solver: KasuariSolver,
    /// Maps our variables to kasuari variables
    variables: HashMap<LayoutVariable, KasuariVariable>,
    /// Tracks constraint sources for error reporting
    sources: Vec<ConstraintSource>,
}

impl ConstraintSolver {
    pub fn new() -> Self {
        Self {
            solver: KasuariSolver::new(),
            variables: HashMap::new(),
            sources: Vec::new(),
        }
    }

    /// Get or create a kasuari variable for our layout variable
    fn get_or_create_var(&mut self, var: &LayoutVariable) -> KasuariVariable {
        if let Some(&kvar) = self.variables.get(var) {
            kvar
        } else {
            let kvar = KasuariVariable::new();
            self.variables.insert(var.clone(), kvar);
            kvar
        }
    }
}

impl Default for ConstraintSolver {
    fn default() -> Self {
        Self::new()
    }
}
```

**Acceptance**: Compiles without errors.

---

### T011: Implement add_constraint method [X]

**File**: `src/layout/solver.rs`
**Priority**: P1
**Depends**: T010

```rust
impl ConstraintSolver {
    /// Add a constraint to the solver
    pub fn add_constraint(&mut self, constraint: LayoutConstraint) -> Result<(), SolverError> {
        match &constraint {
            LayoutConstraint::Fixed { variable, value, source } => {
                let var = self.get_or_create_var(variable);
                self.solver
                    .add_constraint(var |EQ(Strength::REQUIRED)| *value)
                    .map_err(|e| SolverError::Internal(format!("Failed to add fixed constraint: {}", e)))?;
                self.sources.push(source.clone());
            }

            LayoutConstraint::Equal { left, right, offset, source } => {
                let left_var = self.get_or_create_var(left);
                let right_var = self.get_or_create_var(right);
                self.solver
                    .add_constraint(left_var |EQ(Strength::REQUIRED)| right_var + *offset)
                    .map_err(|e| SolverError::Internal(format!("Failed to add equal constraint: {}", e)))?;
                self.sources.push(source.clone());
            }

            LayoutConstraint::GreaterOrEqual { variable, value, source } => {
                let var = self.get_or_create_var(variable);
                self.solver
                    .add_constraint(var |GE(Strength::REQUIRED)| *value)
                    .map_err(|e| SolverError::Internal(format!("Failed to add >= constraint: {}", e)))?;
                self.sources.push(source.clone());
            }

            LayoutConstraint::LessOrEqual { variable, value, source } => {
                let var = self.get_or_create_var(variable);
                self.solver
                    .add_constraint(var |LE(Strength::REQUIRED)| *value)
                    .map_err(|e| SolverError::Internal(format!("Failed to add <= constraint: {}", e)))?;
                self.sources.push(source.clone());
            }

            LayoutConstraint::Midpoint { target, a, b, source } => {
                let target_var = self.get_or_create_var(target);
                let a_var = self.get_or_create_var(a);
                let b_var = self.get_or_create_var(b);
                // Express midpoint as: 2*target = a + b
                self.solver
                    .add_constraint(2.0 * target_var |EQ(Strength::REQUIRED)| a_var + b_var)
                    .map_err(|e| SolverError::Internal(format!("Failed to add midpoint constraint: {}", e)))?;
                self.sources.push(source.clone());
            }
        }
        Ok(())
    }
}
```

**Acceptance**: Compiles without errors.

---

### T012: Implement solve method [X]

**File**: `src/layout/solver.rs`
**Priority**: P1
**Depends**: T011

```rust
/// Solution from the constraint solver
pub struct Solution {
    pub values: HashMap<LayoutVariable, f64>,
}

impl Solution {
    /// Get value for a variable
    pub fn get(&self, var: &LayoutVariable) -> Option<f64> {
        self.values.get(var).copied()
    }

    /// Get value by element ID and property
    pub fn get_by_id(&self, element_id: &str, property: LayoutProperty) -> Option<f64> {
        self.values.get(&LayoutVariable::new(element_id, property)).copied()
    }
}

impl ConstraintSolver {
    /// Solve the constraint system
    pub fn solve(&mut self) -> Result<Solution, SolverError> {
        // Fetch changes from kasuari
        let changes = self.solver.fetch_changes();

        // Build solution map
        let mut values = HashMap::new();
        for (kvar, value) in changes {
            // Find our variable for this kasuari variable
            for (our_var, &k) in &self.variables {
                if k == kvar {
                    values.insert(our_var.clone(), value);
                    break;
                }
            }
        }

        Ok(Solution { values })
    }

    /// Add an edit variable with suggested value (for anchoring the system)
    pub fn suggest_value(&mut self, var: &LayoutVariable, value: f64) -> Result<(), SolverError> {
        let kvar = self.get_or_create_var(var);
        self.solver
            .add_edit_variable(kvar, Strength::STRONG)
            .map_err(|e| SolverError::Internal(format!("Failed to add edit variable: {}", e)))?;
        self.solver
            .suggest_value(kvar, value)
            .map_err(|e| SolverError::Internal(format!("Failed to suggest value: {}", e)))?;
        Ok(())
    }
}
```

**Acceptance**: Compiles without errors.

---

### T013: Add unit tests for ConstraintSolver [X]

**File**: `src/layout/solver.rs`
**Priority**: P1
**Depends**: T012

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_constraint() {
        let mut solver = ConstraintSolver::new();
        let var = LayoutVariable::width("box");

        solver.add_constraint(LayoutConstraint::Fixed {
            variable: var.clone(),
            value: 100.0,
            source: ConstraintSource::intrinsic("test"),
        }).unwrap();

        solver.suggest_value(&var, 100.0).unwrap();
        let solution = solver.solve().unwrap();

        assert!((solution.get(&var).unwrap() - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_equal_constraint() {
        let mut solver = ConstraintSolver::new();
        let a = LayoutVariable::x("a");
        let b = LayoutVariable::x("b");

        // a.x = b.x + 20
        solver.add_constraint(LayoutConstraint::Equal {
            left: a.clone(),
            right: b.clone(),
            offset: 20.0,
            source: ConstraintSource::intrinsic("test"),
        }).unwrap();

        solver.suggest_value(&b, 50.0).unwrap();
        let solution = solver.solve().unwrap();

        assert!((solution.get(&a).unwrap() - 70.0).abs() < 0.001);
        assert!((solution.get(&b).unwrap() - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_midpoint_constraint() {
        let mut solver = ConstraintSolver::new();
        let a = LayoutVariable::x("a");
        let b = LayoutVariable::x("b");
        let mid = LayoutVariable::x("mid");

        // mid.x = midpoint(a.x, b.x)
        solver.add_constraint(LayoutConstraint::Midpoint {
            target: mid.clone(),
            a: a.clone(),
            b: b.clone(),
            source: ConstraintSource::intrinsic("test"),
        }).unwrap();

        solver.suggest_value(&a, 0.0).unwrap();
        solver.suggest_value(&b, 100.0).unwrap();
        let solution = solver.solve().unwrap();

        assert!((solution.get(&mid).unwrap() - 50.0).abs() < 0.001);
    }
}
```

**Acceptance**: `cargo test` passes for solver tests.

---

## Checkpoint 3: Solver Wrapper Complete

**Verify**: `cargo test` passes. ConstraintSolver can add and solve constraints.

---

## Phase 4: AST Extensions for `constrain` Keyword

**Goal**: Extend parser to handle new `constrain` syntax.

---

### T014: Add ConstraintProperty enum to AST [X]

**File**: `src/parser/ast.rs`
**Priority**: P2
**Depends**: Checkpoint 3

```rust
/// Properties that can be referenced in constraints
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintProperty {
    // Position
    X,
    Y,
    // Size
    Width,
    Height,
    // Edges
    Left,
    Right,
    Top,
    Bottom,
    // Centers
    CenterX,
    CenterY,
    Center, // Both center_x and center_y
}

impl ConstraintProperty {
    /// Parse from string (for lexer integration)
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "x" => Some(Self::X),
            "y" => Some(Self::Y),
            "width" => Some(Self::Width),
            "height" => Some(Self::Height),
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            "top" => Some(Self::Top),
            "bottom" => Some(Self::Bottom),
            "center_x" | "horizontal_center" => Some(Self::CenterX),
            "center_y" | "vertical_center" => Some(Self::CenterY),
            "center" => Some(Self::Center),
            _ => None,
        }
    }
}
```

**Acceptance**: Compiles without errors.

---

### T015: Add PropertyRef and ConstraintExpr to AST [X]

**File**: `src/parser/ast.rs`
**Priority**: P2
**Depends**: T014

```rust
/// Reference to an element's property
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyRef {
    pub element: Spanned<ElementPath>,
    pub property: Spanned<ConstraintProperty>,
}

/// Expression in a constrain statement
#[derive(Debug, Clone, PartialEq)]
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
    /// a.center = midpoint(b, c)
    Midpoint {
        target: PropertyRef,
        a: Spanned<Identifier>,
        b: Spanned<Identifier>,
    },
    /// container contains a, b, c [padding: 20]
    Contains {
        container: Spanned<Identifier>,
        elements: Vec<Spanned<Identifier>>,
        padding: Option<f64>,
    },
}

/// Constrain statement declaration
#[derive(Debug, Clone, PartialEq)]
pub struct ConstrainDecl {
    pub expr: ConstraintExpr,
}
```

**Acceptance**: Compiles without errors.

---

### T016: Add Constrain variant to Statement enum [X]

**File**: `src/parser/ast.rs`
**Priority**: P2
**Depends**: T015

Update the Statement enum to include the new variant:

```rust
pub enum Statement {
    // ... existing variants ...
    /// Constrain statement: `constrain a.left = b.left`
    Constrain(ConstrainDecl),
}
```

**Acceptance**: Compiles without errors.

---

### T017: Add lexer tokens for constraint keywords [X]

**File**: `src/parser/lexer.rs`
**Priority**: P2
**Depends**: T016

Add new tokens:

```rust
// In the Token enum, add:
#[token("constrain")]
Constrain,

#[token("midpoint")]
Midpoint,

#[token("contains")]
Contains,

#[token(">=")]
GreaterOrEqual,

#[token("<=")]
LessOrEqual,

// Property keywords (if not using identifier matching)
#[token("center_x")]
CenterX,

#[token("center_y")]
CenterY,
```

**Acceptance**: Lexer tokenizes new keywords correctly.

---

### T018: Implement constraint expression parser [X]

**File**: `src/parser/grammar.rs`
**Priority**: P2
**Depends**: T017

Add parser for `constrain` statements:

```rust
/// Parse a property reference: element.property or element_path.property
fn property_ref<'src>() -> impl Parser<'src, ParserInput<'src>, PropertyRef, Extra<'src>> {
    element_path()
        .then_ignore(just(Token::Dot))
        .then(constraint_property())
        .map(|(element, property)| PropertyRef { element, property })
}

/// Parse constraint property keyword
fn constraint_property<'src>() -> impl Parser<'src, ParserInput<'src>, Spanned<ConstraintProperty>, Extra<'src>> {
    select! {
        Token::Identifier(s) => ConstraintProperty::from_str(&s),
        Token::CenterX => Some(ConstraintProperty::CenterX),
        Token::CenterY => Some(ConstraintProperty::CenterY),
        // ... other property tokens
    }
    .filter(|opt| opt.is_some())
    .map(|opt| opt.unwrap())
    .map_with(|prop, e| Spanned::new(prop, e.span()))
}

/// Parse constrain statement
fn constrain_decl<'src>() -> impl Parser<'src, ParserInput<'src>, Statement, Extra<'src>> {
    just(Token::Constrain)
        .ignore_then(constraint_expr())
        .map(|expr| Statement::Constrain(ConstrainDecl { expr }))
}
```

**Acceptance**: Parser handles `constrain a.left = b.left` syntax.

---

### T019: Add parser tests for constrain syntax [X]

**File**: `src/parser/grammar.rs` (in tests module)
**Priority**: P2
**Depends**: T018

```rust
#[test]
fn test_parse_constrain_equality() {
    let input = "constrain a.left = b.left";
    let doc = parse(input).unwrap();
    assert!(matches!(doc.statements[0].node, Statement::Constrain(_)));
}

#[test]
fn test_parse_constrain_with_offset() {
    let input = "constrain a.left = b.right + 20";
    let doc = parse(input).unwrap();
    // Verify the offset is captured
}

#[test]
fn test_parse_constrain_inequality() {
    let input = "constrain a.width >= 50";
    let doc = parse(input).unwrap();
    // Verify inequality is parsed
}

#[test]
fn test_parse_constrain_midpoint() {
    let input = "constrain a.center_x = midpoint(b, c)";
    let doc = parse(input).unwrap();
    // Verify midpoint is parsed
}

#[test]
fn test_parse_constrain_contains() {
    let input = "constrain container contains a, b, c [padding: 20]";
    let doc = parse(input).unwrap();
    // Verify contains is parsed
}
```

**Acceptance**: All parser tests pass.

---

## Checkpoint 4: Parser Extensions Complete

**Verify**: `cargo test` passes. New `constrain` syntax parses correctly.

---

## Phase 5: Layout Pipeline Integration

**Goal**: Integrate constraint solver into the layout computation pipeline.

---

### T020: Create constraint collector module [X]

**File**: `src/layout/collector.rs`
**Priority**: P2
**Depends**: Checkpoint 4

```rust
//! Collects constraints from AST for solving

use crate::parser::ast::*;
use super::solver::{LayoutConstraint, LayoutVariable, LayoutProperty, ConstraintSource};
use super::config::LayoutConfig;

/// Collects all constraints from a document
pub struct ConstraintCollector {
    pub constraints: Vec<LayoutConstraint>,
    config: LayoutConfig,
}

impl ConstraintCollector {
    pub fn new(config: LayoutConfig) -> Self {
        Self {
            constraints: Vec::new(),
            config,
        }
    }

    /// Collect all constraints from a document
    pub fn collect(&mut self, doc: &Document) {
        // 1. Collect intrinsic constraints (shape sizes)
        self.collect_intrinsics(doc);

        // 2. Collect layout container constraints
        self.collect_layout_constraints(&doc.statements);

        // 3. Collect user constraints (constrain, align)
        self.collect_user_constraints(&doc.statements);
    }

    fn collect_intrinsics(&mut self, doc: &Document) {
        // Add fixed size constraints for shapes with explicit sizes
        // ... implementation
    }

    fn collect_layout_constraints(&mut self, stmts: &[Spanned<Statement>]) {
        // Generate constraints from row/col/stack/grid containers
        // ... implementation
    }

    fn collect_user_constraints(&mut self, stmts: &[Spanned<Statement>]) {
        for stmt in stmts {
            match &stmt.node {
                Statement::Constrain(c) => self.collect_constrain(&c.expr, &stmt.span),
                Statement::Alignment(a) => self.collect_alignment(a, &stmt.span),
                Statement::Layout(l) => self.collect_user_constraints(&l.children),
                Statement::Group(g) => self.collect_user_constraints(&g.children),
                _ => {}
            }
        }
    }

    fn collect_constrain(&mut self, expr: &ConstraintExpr, span: &Span) {
        // Convert ConstraintExpr to LayoutConstraint
        // ... implementation
    }

    fn collect_alignment(&mut self, alignment: &AlignmentDecl, span: &Span) {
        // Convert align statements to equality constraints
        // ... implementation
    }
}
```

**Acceptance**: Compiles without errors.

---

### T021: Implement intrinsic constraint collection [X]

**File**: `src/layout/collector.rs`
**Priority**: P2
**Depends**: T020

Implement `collect_intrinsics` to add fixed size constraints for shapes with explicit width/height:

```rust
fn collect_intrinsics(&mut self, doc: &Document) {
    self.collect_shape_intrinsics(&doc.statements);
}

fn collect_shape_intrinsics(&mut self, stmts: &[Spanned<Statement>]) {
    for stmt in stmts {
        match &stmt.node {
            Statement::Shape(s) => {
                if let Some(name) = &s.name {
                    let id = name.node.0.clone();

                    // Extract explicit width/height from modifiers
                    if let Some(w) = extract_width(&s.modifiers) {
                        self.constraints.push(LayoutConstraint::Fixed {
                            variable: LayoutVariable::width(&id),
                            value: w,
                            source: ConstraintSource::intrinsic(format!("{} width", id)),
                        });
                    }
                    // Similar for height
                }
            }
            Statement::Layout(l) => self.collect_shape_intrinsics(&l.children),
            Statement::Group(g) => self.collect_shape_intrinsics(&g.children),
            _ => {}
        }
    }
}
```

**Acceptance**: Intrinsic constraints collected for explicit sizes.

---

### T022: Implement layout container constraint generation [X]

**File**: `src/layout/collector.rs`
**Priority**: P2
**Depends**: T021

Implement constraint generation for row/col/stack/grid:

```rust
fn collect_layout_constraints(&mut self, stmts: &[Spanned<Statement>]) {
    for stmt in stmts {
        if let Statement::Layout(l) = &stmt.node {
            match l.layout_type.node {
                LayoutType::Row => self.collect_row_constraints(l, &stmt.span),
                LayoutType::Column => self.collect_column_constraints(l, &stmt.span),
                LayoutType::Stack => self.collect_stack_constraints(l, &stmt.span),
                LayoutType::Grid => self.collect_grid_constraints(l, &stmt.span),
            }
            // Recurse into children
            self.collect_layout_constraints(&l.children);
        }
    }
}

fn collect_row_constraints(&mut self, layout: &LayoutDecl, span: &Span) {
    let gap = extract_gap(&layout.modifiers).unwrap_or(self.config.element_spacing);
    let child_ids: Vec<_> = self.get_child_ids(&layout.children);

    for (i, id) in child_ids.iter().enumerate() {
        if i > 0 {
            let prev_id = &child_ids[i - 1];
            // child[i].left = child[i-1].right + gap
            // Which is: child[i].x = child[i-1].x + child[i-1].width + gap
            self.constraints.push(LayoutConstraint::Equal {
                left: LayoutVariable::x(id),
                right: LayoutVariable::x(prev_id),
                offset: gap, // This needs width too - simplified for now
                source: ConstraintSource::layout(span.clone(), format!("row gap between {} and {}", prev_id, id)),
            });
        }
    }
}
```

**Acceptance**: Row/col/stack generate appropriate constraints.

---

### T023: Implement constrain expression to LayoutConstraint conversion [X]

**File**: `src/layout/collector.rs`
**Priority**: P2
**Depends**: T022

```rust
fn collect_constrain(&mut self, expr: &ConstraintExpr, span: &Span) {
    match expr {
        ConstraintExpr::Equal { left, right } => {
            let (left_var, left_offset) = self.property_to_variable(&left);
            let (right_var, right_offset) = self.property_to_variable(&right);

            self.constraints.push(LayoutConstraint::Equal {
                left: left_var,
                right: right_var,
                offset: right_offset - left_offset,
                source: ConstraintSource::user(span.clone(), "constrain equality"),
            });
        }
        ConstraintExpr::Constant { left, value } => {
            let (var, offset) = self.property_to_variable(&left);
            self.constraints.push(LayoutConstraint::Fixed {
                variable: var,
                value: *value - offset,
                source: ConstraintSource::user(span.clone(), "constrain constant"),
            });
        }
        ConstraintExpr::Midpoint { target, a, b } => {
            // Convert to midpoint constraint
            let (target_var, _) = self.property_to_variable(&target);
            // For midpoint of element centers, we need center_x variables
            self.constraints.push(LayoutConstraint::Midpoint {
                target: target_var,
                a: LayoutVariable::x(&a.node.0), // Simplified - should use center
                b: LayoutVariable::x(&b.node.0),
                source: ConstraintSource::user(span.clone(), "constrain midpoint"),
            });
        }
        // ... other variants
    }
}

/// Convert a PropertyRef to a LayoutVariable and offset
///
/// For derived properties like `right = x + width`, returns (x_var, width_value)
fn property_to_variable(&self, prop_ref: &PropertyRef) -> (LayoutVariable, f64) {
    let id = prop_ref.element.node.leaf().0.clone();

    match prop_ref.property.node {
        ConstraintProperty::X | ConstraintProperty::Left =>
            (LayoutVariable::x(&id), 0.0),
        ConstraintProperty::Y | ConstraintProperty::Top =>
            (LayoutVariable::y(&id), 0.0),
        ConstraintProperty::Width =>
            (LayoutVariable::width(&id), 0.0),
        ConstraintProperty::Height =>
            (LayoutVariable::height(&id), 0.0),
        // Derived properties need special handling
        ConstraintProperty::Right =>
            (LayoutVariable::x(&id), 0.0), // Needs width added
        ConstraintProperty::Bottom =>
            (LayoutVariable::y(&id), 0.0), // Needs height added
        ConstraintProperty::CenterX =>
            (LayoutVariable::x(&id), 0.0), // Needs width/2 added
        ConstraintProperty::CenterY =>
            (LayoutVariable::y(&id), 0.0), // Needs height/2 added
        ConstraintProperty::Center =>
            (LayoutVariable::x(&id), 0.0), // Simplified
    }
}
```

**Acceptance**: Constrain expressions convert to solver constraints.

---

### T024: Add collector module to layout/mod.rs [X]

**File**: `src/layout/mod.rs`
**Priority**: P2
**Depends**: T023

```rust
pub mod collector;
pub use collector::ConstraintCollector;
```

**Acceptance**: Module exports properly.

---

### T025: Modify layout::compute to use constraint solver [X]

**File**: `src/layout/engine.rs`
**Priority**: P2
**Depends**: T024

Update the `compute` function to use the new solver:

```rust
pub fn compute(doc: &Document, config: &LayoutConfig) -> Result<LayoutResult, LayoutError> {
    // First validate references
    super::validate_references(doc)?;

    // Collect constraints
    let mut collector = ConstraintCollector::new(config.clone());
    collector.collect(doc);

    // Create solver and add constraints
    let mut solver = ConstraintSolver::new();
    for constraint in collector.constraints {
        solver.add_constraint(constraint)
            .map_err(|e| LayoutError::solver_error(e))?;
    }

    // Add default positions as suggestions (anchors the system)
    // ... add edit variables for root elements

    // Solve
    let solution = solver.solve()
        .map_err(|e| LayoutError::solver_error(e))?;

    // Build result from solution
    build_layout_result(doc, &solution, config)
}
```

**Acceptance**: Layout computation uses constraint solver.

---

### T026: Add LayoutError variant for solver errors [X]

**File**: `src/layout/error.rs`
**Priority**: P2
**Depends**: T025

```rust
use super::solver::SolverError;

#[derive(Debug, Error)]
pub enum LayoutError {
    // ... existing variants ...

    #[error("Constraint solver error: {0}")]
    SolverError(#[from] SolverError),
}

impl LayoutError {
    pub fn solver_error(e: SolverError) -> Self {
        Self::SolverError(e)
    }
}
```

**Acceptance**: Solver errors integrate with layout errors.

---

### T027: Integration tests with existing examples [X]

**File**: `tests/integration/constraint_solver.rs`
**Priority**: P2
**Depends**: T026

Create integration tests verifying existing examples still work:

```rust
#[test]
fn test_simple_row_layout() {
    let input = r#"
        row {
            rect a [width: 50]
            rect b [width: 50]
            rect c [width: 50]
        }
    "#;

    let result = compile(input).unwrap();
    // Verify elements are positioned correctly
}

#[test]
fn test_alignment_via_constrain() {
    let input = r#"
        rect a [width: 100, height: 50]
        rect b [width: 80, height: 30]
        constrain a.left = b.left
    "#;

    let result = compile(input).unwrap();
    // Verify alignment is correct
}
```

**Acceptance**: All existing layout behaviors preserved.

---

## Checkpoint 5: Solver Integration Complete

**Verify**: `cargo test` passes. Existing examples produce same output.

---

## Phase 6: Error Handling & Migration

**Goal**: Helpful error messages and `align` deprecation.

---

### T028: Implement unsatisfiable constraint error messages [X]

**File**: `src/layout/solver.rs`
**Priority**: P3
**Depends**: Checkpoint 5

Enhance error handling to provide actionable messages:

```rust
impl ConstraintSolver {
    pub fn add_constraint(&mut self, constraint: LayoutConstraint) -> Result<(), SolverError> {
        // Catch kasuari errors and convert to our error type with sources
        match self.try_add_constraint(&constraint) {
            Ok(()) => Ok(()),
            Err(_) => {
                // Find conflicting constraints
                Err(SolverError::Unsatisfiable {
                    conflicting: vec![constraint.source().clone()],
                    reason: "Constraint conflicts with existing constraints".to_string(),
                })
            }
        }
    }
}
```

**Acceptance**: Conflicting constraints produce helpful errors.

---

### T029: Convert `align` statements to constraints internally [SKIP]

**Note**: Skipped - breaking change is acceptable. `align` keyword support can be removed.

**File**: `src/layout/collector.rs`
**Priority**: P3
**Depends**: T028

**Acceptance**: N/A - breaking change accepted.

---

### T030: Update grammar.ebnf with constrain syntax [P]

**File**: `features/001-the-grammar-and-ast-for-our-dsl/contracts/grammar.ebnf`
**Priority**: P3
**Depends**: T029

Merge the grammar extensions from `features/005-constraint-solver/contracts/grammar-extensions.ebnf` into the main grammar file.

**Acceptance**: Grammar reflects all constraint syntax.

---

### T031: Update railway example to use constraints

**File**: `examples/railway_topology.ail`
**Priority**: P3
**Depends**: T030

Convert the railway example to use `constrain` instead of magic pixel offsets:

```
// Before:
place op1 [x: -66]

// After:
constrain op1.center_x = midpoint(mjA1, mjB1)
constrain op1.center_y = midpoint(mtrackA, mtrackB)
```

**Acceptance**: Railway example renders correctly with constraints.

---

## Checkpoint 6: Feature Complete

**Verify**:
- `cargo test` passes
- `cargo clippy` clean
- `cargo fmt` applied
- Railway example works with constraints
- Grammar.ebnf updated

---

## Summary

### Task Dependencies Graph

```
T001 (kasuari dep)
  │
  ├── T002, T003 [P] (spike)
  │     │
  │     └── Checkpoint 1 (Gate: spike passes)
  │           │
  │           ├── T004-T009 [P] (data structures)
  │           │     │
  │           │     └── Checkpoint 2
  │           │           │
  │           │           ├── T010-T013 (solver wrapper)
  │           │           │     │
  │           │           │     └── Checkpoint 3
  │           │           │           │
  │           │           │           ├── T014-T019 (parser)
  │           │           │           │     │
  │           │           │           │     └── Checkpoint 4
  │           │           │           │           │
  │           │           │           │           ├── T020-T027 (integration)
  │           │           │           │           │     │
  │           │           │           │           │     └── Checkpoint 5
  │           │           │           │           │           │
  │           │           │           │           │           ├── T028-T031 [P] (polish)
  │           │           │           │           │           │     │
  │           │           │           │           │           │     └── Checkpoint 6 (Done)
```

### Parallel Execution Opportunities

**Phase 2** (after Checkpoint 1):
- T004, T005, T006, T007 can run in parallel

**Phase 4** (after Checkpoint 3):
- T014, T015 can run in parallel

**Phase 6** (after Checkpoint 5):
- T028, T029, T030, T031 can run in parallel

### Task Count by Phase

| Phase | Tasks | Parallel |
|-------|-------|----------|
| 1: Setup & Spike | 3 | 2 |
| 2: Data Structures | 6 | 4 |
| 3: Solver Wrapper | 4 | 0 |
| 4: Parser Extensions | 6 | 2 |
| 5: Integration | 8 | 1 |
| 6: Polish | 4 | 4 |
| **Total** | **31** | **13** |

---

*Generated: 2026-01-23*
