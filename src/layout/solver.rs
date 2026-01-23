//! Constraint solver integration for layout computation
//!
//! This module provides a wrapper around the kasuari Cassowary constraint solver,
//! translating our layout constraints into the solver's format and extracting solutions.

use std::collections::HashMap;

use kasuari::{
    Solver as KasuariSolver, Strength, Variable as KasuariVariable, WeightedRelation::*,
};
use thiserror::Error;

use crate::parser::ast::Span;

// ============================================================================
// T004: LayoutProperty enum
// ============================================================================

/// Properties that can be constrained
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayoutProperty {
    X,
    Y,
    Width,
    Height,
    /// Center X = X + Width/2 (derived property)
    CenterX,
    /// Center Y = Y + Height/2 (derived property)
    CenterY,
}

impl LayoutProperty {
    /// Get all base properties (not derived)
    pub fn base_properties() -> &'static [LayoutProperty] {
        &[Self::X, Self::Y, Self::Width, Self::Height]
    }
}

/// Derived properties computed from base properties
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum DerivedProperty {
    Left,    // = X
    Right,   // = X + Width
    Top,     // = Y
    Bottom,  // = Y + Height
    CenterX, // = X + Width/2
    CenterY, // = Y + Height/2
}

// ============================================================================
// T005: LayoutVariable struct
// ============================================================================

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

// ============================================================================
// T006: ConstraintSource for error tracking
// ============================================================================

/// Origin of a constraint (for error messages)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstraintOrigin {
    /// User wrote explicit constraint
    UserDefined,
    /// Generated from layout container (row, col, etc.)
    LayoutContainer,
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

// ============================================================================
// T007: LayoutConstraint enum
// ============================================================================

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

    /// target = (a + b) / 2 + offset
    Midpoint {
        target: LayoutVariable,
        a: LayoutVariable,
        b: LayoutVariable,
        offset: f64,
        source: ConstraintSource,
    },
}

impl LayoutConstraint {
    /// Get the source of this constraint
    #[allow(dead_code)]
    pub fn source(&self) -> &ConstraintSource {
        match self {
            LayoutConstraint::Fixed { source, .. } => source,
            LayoutConstraint::Equal { source, .. } => source,
            LayoutConstraint::GreaterOrEqual { source, .. } => source,
            LayoutConstraint::LessOrEqual { source, .. } => source,
            LayoutConstraint::Midpoint { source, .. } => source,
        }
    }
}

// ============================================================================
// T008: SolverError enum
// ============================================================================

/// Errors from the constraint solver
#[derive(Debug, Error)]
pub enum SolverError {
    #[error("Unsatisfiable constraints: {reason}")]
    Unsatisfiable {
        conflicting: Vec<ConstraintSource>,
        reason: String,
    },

    #[error("Underconstrained system: variables have no determined value")]
    Underconstrained { free_variables: Vec<LayoutVariable> },

    #[error("Undefined element: {name}")]
    UndefinedElement {
        name: String,
        span: Span,
        suggestions: Vec<String>,
    },

    #[error("Internal solver error: {0}")]
    Internal(String),
}

// ============================================================================
// T010: ConstraintSolver struct
// ============================================================================

/// Wrapper around kasuari solver
pub struct ConstraintSolver {
    solver: KasuariSolver,
    /// Maps our variables to kasuari variables
    variables: HashMap<LayoutVariable, KasuariVariable>,
    /// Tracks constraint sources for error reporting
    #[allow(dead_code)]
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

    /// Get or create a kasuari variable for a base property (X, Y, Width, Height)
    fn get_or_create_base_var(&mut self, element_id: &str, property: LayoutProperty) -> KasuariVariable {
        let var = LayoutVariable::new(element_id, property);
        if let Some(&kvar) = self.variables.get(&var) {
            kvar
        } else {
            let kvar = KasuariVariable::new();
            self.variables.insert(var, kvar);
            kvar
        }
    }

    /// Get or create a kasuari variable for our layout variable (for base properties only)
    fn get_or_create_var(&mut self, var: &LayoutVariable) -> KasuariVariable {
        if let Some(&kvar) = self.variables.get(var) {
            kvar
        } else {
            let kvar = KasuariVariable::new();
            self.variables.insert(var.clone(), kvar);
            kvar
        }
    }

    /// Create a kasuari expression for a layout variable
    /// For base properties (X, Y, Width, Height), returns the variable as an expression
    /// For derived properties (CenterX, CenterY), returns the appropriate expression
    fn get_expression(&mut self, var: &LayoutVariable) -> kasuari::Expression {
        match var.property {
            LayoutProperty::X | LayoutProperty::Y | LayoutProperty::Width | LayoutProperty::Height => {
                self.get_or_create_var(var).into()
            }
            LayoutProperty::CenterX => {
                // center_x = x + width / 2
                let x = self.get_or_create_base_var(&var.element_id, LayoutProperty::X);
                let width = self.get_or_create_base_var(&var.element_id, LayoutProperty::Width);
                x + width * 0.5
            }
            LayoutProperty::CenterY => {
                // center_y = y + height / 2
                let y = self.get_or_create_base_var(&var.element_id, LayoutProperty::Y);
                let height = self.get_or_create_base_var(&var.element_id, LayoutProperty::Height);
                y + height * 0.5
            }
        }
    }
}

impl Default for ConstraintSolver {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// T011: add_constraint method
// ============================================================================

impl ConstraintSolver {
    /// Convert a kasuari error to a SolverError with context
    fn convert_kasuari_error(
        &self,
        e: kasuari::AddConstraintError,
        source: &ConstraintSource,
        constraint_desc: &str,
    ) -> SolverError {
        // kasuari returns AddConstraintError::UnsatisfiableConstraint for conflicts
        match e {
            kasuari::AddConstraintError::UnsatisfiableConstraint => {
                // Include both the new constraint and any existing constraints that might conflict
                let mut conflicting = vec![source.clone()];
                // Add existing sources as potentially conflicting
                conflicting.extend(self.sources.iter().cloned());
                SolverError::Unsatisfiable {
                    conflicting,
                    reason: format!(
                        "Cannot satisfy {}: conflicts with existing constraints",
                        constraint_desc
                    ),
                }
            }
            kasuari::AddConstraintError::DuplicateConstraint => {
                // Duplicate constraints are warnings, not errors - but we treat them as errors for now
                SolverError::Internal(format!("Duplicate constraint: {}", constraint_desc))
            }
            kasuari::AddConstraintError::InternalSolverError(msg) => {
                SolverError::Internal(format!("Internal solver error for {}: {}", constraint_desc, msg))
            }
        }
    }

    /// Add a constraint to the solver
    pub fn add_constraint(&mut self, constraint: LayoutConstraint) -> Result<(), SolverError> {
        match &constraint {
            LayoutConstraint::Fixed {
                variable,
                value,
                source,
            } => {
                // Use expression to handle derived properties like CenterX/CenterY
                let expr = self.get_expression(variable);
                let desc = format!("{}.{:?} = {}", variable.element_id, variable.property, value);
                self.solver
                    .add_constraint(expr | EQ(Strength::REQUIRED) | *value)
                    .map_err(|e| self.convert_kasuari_error(e, source, &desc))?;
                self.sources.push(source.clone());
            }

            LayoutConstraint::Equal {
                left,
                right,
                offset,
                source,
            } => {
                // Use expressions to handle derived properties like CenterX/CenterY
                let left_expr = self.get_expression(left);
                let right_expr = self.get_expression(right);
                let desc = if *offset == 0.0 {
                    format!(
                        "{}.{:?} = {}.{:?}",
                        left.element_id, left.property, right.element_id, right.property
                    )
                } else {
                    format!(
                        "{}.{:?} = {}.{:?} + {}",
                        left.element_id, left.property, right.element_id, right.property, offset
                    )
                };
                self.solver
                    .add_constraint(left_expr | EQ(Strength::REQUIRED) | right_expr + *offset)
                    .map_err(|e| self.convert_kasuari_error(e, source, &desc))?;
                self.sources.push(source.clone());
            }

            LayoutConstraint::GreaterOrEqual {
                variable,
                value,
                source,
            } => {
                // Use expression to handle derived properties like CenterX/CenterY
                let expr = self.get_expression(variable);
                let desc = format!("{}.{:?} >= {}", variable.element_id, variable.property, value);
                self.solver
                    .add_constraint(expr | GE(Strength::REQUIRED) | *value)
                    .map_err(|e| self.convert_kasuari_error(e, source, &desc))?;
                self.sources.push(source.clone());
            }

            LayoutConstraint::LessOrEqual {
                variable,
                value,
                source,
            } => {
                // Use expression to handle derived properties like CenterX/CenterY
                let expr = self.get_expression(variable);
                let desc = format!("{}.{:?} <= {}", variable.element_id, variable.property, value);
                self.solver
                    .add_constraint(expr | LE(Strength::REQUIRED) | *value)
                    .map_err(|e| self.convert_kasuari_error(e, source, &desc))?;
                self.sources.push(source.clone());
            }

            LayoutConstraint::Midpoint {
                target,
                a,
                b,
                offset,
                source,
            } => {
                // Use expressions to handle derived properties like CenterX/CenterY
                let target_expr = self.get_expression(target);
                let a_expr = self.get_expression(a);
                let b_expr = self.get_expression(b);
                let desc = if *offset != 0.0 {
                    format!(
                        "{}.{:?} = midpoint({}.{:?}, {}.{:?}) + {}",
                        target.element_id, target.property,
                        a.element_id, a.property,
                        b.element_id, b.property,
                        offset
                    )
                } else {
                    format!(
                        "{}.{:?} = midpoint({}.{:?}, {}.{:?})",
                        target.element_id, target.property,
                        a.element_id, a.property,
                        b.element_id, b.property
                    )
                };
                // Express midpoint + offset as: 2*target = a + b + 2*offset
                // Which is equivalent to: target = (a + b) / 2 + offset
                self.solver
                    .add_constraint(2.0 * target_expr | EQ(Strength::REQUIRED) | a_expr + b_expr + 2.0 * offset)
                    .map_err(|e| self.convert_kasuari_error(e, source, &desc))?;
                self.sources.push(source.clone());
            }
        }
        Ok(())
    }
}

// ============================================================================
// T012: Solution and solve method
// ============================================================================

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
    #[allow(dead_code)]
    pub fn get_by_id(&self, element_id: &str, property: LayoutProperty) -> Option<f64> {
        self.values
            .get(&LayoutVariable::new(element_id, property))
            .copied()
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
                if k == *kvar {
                    values.insert(our_var.clone(), *value);
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

// ============================================================================
// T013: Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_constraint() {
        let mut solver = ConstraintSolver::new();
        let var = LayoutVariable::width("box");

        solver
            .add_constraint(LayoutConstraint::Fixed {
                variable: var.clone(),
                value: 100.0,
                source: ConstraintSource::intrinsic("test"),
            })
            .unwrap();

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
        solver
            .add_constraint(LayoutConstraint::Equal {
                left: a.clone(),
                right: b.clone(),
                offset: 20.0,
                source: ConstraintSource::intrinsic("test"),
            })
            .unwrap();

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
        solver
            .add_constraint(LayoutConstraint::Midpoint {
                target: mid.clone(),
                a: a.clone(),
                b: b.clone(),
                offset: 0.0,
                source: ConstraintSource::intrinsic("test"),
            })
            .unwrap();

        solver.suggest_value(&a, 0.0).unwrap();
        solver.suggest_value(&b, 100.0).unwrap();
        let solution = solver.solve().unwrap();

        assert!((solution.get(&mid).unwrap() - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_inequality_constraint() {
        let mut solver = ConstraintSolver::new();
        let width = LayoutVariable::width("box");

        // width >= 50
        solver
            .add_constraint(LayoutConstraint::GreaterOrEqual {
                variable: width.clone(),
                value: 50.0,
                source: ConstraintSource::intrinsic("test"),
            })
            .unwrap();

        solver.suggest_value(&width, 30.0).unwrap(); // Try to set it lower
        let solution = solver.solve().unwrap();

        // Should be at least 50 despite suggestion of 30
        assert!(solution.get(&width).unwrap() >= 50.0 - 0.001);
    }

    #[test]
    fn test_complex_constraint_system() {
        let mut solver = ConstraintSolver::new();

        // Create a row of 3 elements with fixed widths and gaps
        let a_x = LayoutVariable::x("a");
        let a_width = LayoutVariable::width("a");
        let b_x = LayoutVariable::x("b");
        let b_width = LayoutVariable::width("b");
        let c_x = LayoutVariable::x("c");
        let c_width = LayoutVariable::width("c");

        // Fixed widths
        solver
            .add_constraint(LayoutConstraint::Fixed {
                variable: a_width.clone(),
                value: 100.0,
                source: ConstraintSource::intrinsic("a width"),
            })
            .unwrap();
        solver
            .add_constraint(LayoutConstraint::Fixed {
                variable: b_width.clone(),
                value: 80.0,
                source: ConstraintSource::intrinsic("b width"),
            })
            .unwrap();
        solver
            .add_constraint(LayoutConstraint::Fixed {
                variable: c_width.clone(),
                value: 60.0,
                source: ConstraintSource::intrinsic("c width"),
            })
            .unwrap();

        // Anchor a_x at a non-zero position to ensure it appears in changes
        solver
            .add_constraint(LayoutConstraint::Fixed {
                variable: a_x.clone(),
                value: 10.0,
                source: ConstraintSource::intrinsic("a x position"),
            })
            .unwrap();

        // Position b relative to a (b_x = a_x + 120)
        solver
            .add_constraint(LayoutConstraint::Equal {
                left: b_x.clone(),
                right: a_x.clone(),
                offset: 120.0, // a_width + gap
                source: ConstraintSource::layout(0..10, "row gap a-b"),
            })
            .unwrap();

        // Position c relative to b (c_x = b_x + 100)
        solver
            .add_constraint(LayoutConstraint::Equal {
                left: c_x.clone(),
                right: b_x.clone(),
                offset: 100.0, // b_width + gap
                source: ConstraintSource::layout(0..10, "row gap b-c"),
            })
            .unwrap();

        // Suggest values to trigger changes
        solver.suggest_value(&a_width, 100.0).unwrap();
        solver.suggest_value(&b_width, 80.0).unwrap();
        solver.suggest_value(&c_width, 60.0).unwrap();

        let solution = solver.solve().unwrap();

        // Verify positions - a_x starts at 10, b_x = 10 + 120 = 130, c_x = 130 + 100 = 230
        assert!(
            (solution.get(&a_x).unwrap_or(10.0) - 10.0).abs() < 0.001,
            "a_x should be 10"
        );
        assert!(
            (solution.get(&b_x).unwrap() - 130.0).abs() < 0.001,
            "b_x should be 130"
        );
        assert!(
            (solution.get(&c_x).unwrap() - 230.0).abs() < 0.001,
            "c_x should be 230"
        );
    }

    #[test]
    fn test_conflicting_constraints_error() {
        let mut solver = ConstraintSolver::new();
        let x = LayoutVariable::x("box");

        // First constraint: x = 100
        solver
            .add_constraint(LayoutConstraint::Fixed {
                variable: x.clone(),
                value: 100.0,
                source: ConstraintSource::user(0..10, "first constraint"),
            })
            .unwrap();

        // Second conflicting constraint: x = 200
        let result = solver.add_constraint(LayoutConstraint::Fixed {
            variable: x.clone(),
            value: 200.0,
            source: ConstraintSource::user(15..25, "second constraint"),
        });

        // Should fail with Unsatisfiable error
        assert!(result.is_err());
        match result.unwrap_err() {
            SolverError::Unsatisfiable { reason, conflicting } => {
                assert!(reason.contains("conflicts"));
                assert!(!conflicting.is_empty());
            }
            other => panic!("Expected Unsatisfiable error, got: {:?}", other),
        }
    }

    #[test]
    fn test_conflicting_inequality_constraints_error() {
        let mut solver = ConstraintSolver::new();
        let x = LayoutVariable::x("box");

        // x >= 200
        solver
            .add_constraint(LayoutConstraint::GreaterOrEqual {
                variable: x.clone(),
                value: 200.0,
                source: ConstraintSource::user(0..10, "ge constraint"),
            })
            .unwrap();

        // x <= 100 (conflicts with x >= 200)
        let result = solver.add_constraint(LayoutConstraint::LessOrEqual {
            variable: x.clone(),
            value: 100.0,
            source: ConstraintSource::user(15..25, "le constraint"),
        });

        // Should fail with Unsatisfiable error
        assert!(result.is_err());
        match result.unwrap_err() {
            SolverError::Unsatisfiable { reason, .. } => {
                assert!(reason.contains("conflicts"));
            }
            other => panic!("Expected Unsatisfiable error, got: {:?}", other),
        }
    }
}
