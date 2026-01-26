//! Error types for the layout engine

use thiserror::Error;

use crate::parser::ast::Span;

use super::solver::SolverError;

/// Errors that can occur during layout computation
#[derive(Debug, Error)]
pub enum LayoutError {
    /// Reference to an undefined element identifier
    #[error("undefined identifier '{name}'")]
    UndefinedIdentifier {
        name: String,
        span: Span,
        suggestions: Vec<String>,
    },

    /// Position constraints that cannot all be satisfied
    #[error("conflicting constraints: {reason}")]
    ConflictingConstraints {
        constraints: Vec<String>,
        reason: String,
    },

    /// Circular dependency in position constraints
    #[error("circular constraint dependency: {}", cycle.join(" -> "))]
    CircularConstraint { cycle: Vec<String> },

    /// Invalid layout configuration
    #[error("invalid layout for element '{element}': {reason}")]
    InvalidLayout { element: String, reason: String },

    /// Element path not found during constraint resolution
    #[error("element path '{path}' not found")]
    PathNotFound {
        path: String,
        span: Span,
        suggestions: Vec<String>,
    },

    /// Constraint solver error
    #[error("constraint solver error: {0}")]
    SolverError(#[from] SolverError),

    /// Invalid anchor reference (Feature 009)
    #[error("invalid anchor '{anchor}' on element '{element}' (valid anchors: {valid_anchors})")]
    InvalidAnchor {
        element: String,
        anchor: String,
        valid_anchors: String,
        span: Span,
    },
}

impl LayoutError {
    /// Create an undefined identifier error with suggestions
    pub fn undefined(name: impl Into<String>, span: Span, suggestions: Vec<String>) -> Self {
        Self::UndefinedIdentifier {
            name: name.into(),
            span,
            suggestions,
        }
    }

    /// Create a conflicting constraints error
    pub fn conflicting(constraints: Vec<String>, reason: impl Into<String>) -> Self {
        Self::ConflictingConstraints {
            constraints,
            reason: reason.into(),
        }
    }

    /// Create a circular constraint error
    pub fn circular(cycle: Vec<String>) -> Self {
        Self::CircularConstraint { cycle }
    }

    /// Create an invalid layout error
    pub fn invalid_layout(element: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::InvalidLayout {
            element: element.into(),
            reason: reason.into(),
        }
    }

    /// Get the source span if available
    pub fn span(&self) -> Option<&Span> {
        match self {
            Self::UndefinedIdentifier { span, .. } => Some(span),
            Self::PathNotFound { span, .. } => Some(span),
            Self::InvalidAnchor { span, .. } => Some(span),
            _ => None,
        }
    }

    /// Get suggestions if available
    pub fn suggestions(&self) -> Option<&[String]> {
        match self {
            Self::UndefinedIdentifier { suggestions, .. } => Some(suggestions),
            Self::PathNotFound { suggestions, .. } => Some(suggestions),
            _ => None,
        }
    }

    /// Create a path not found error
    pub fn path_not_found(path: impl Into<String>, span: Span, suggestions: Vec<String>) -> Self {
        Self::PathNotFound {
            path: path.into(),
            span,
            suggestions,
        }
    }

    /// Create a solver error from a SolverError
    pub fn solver_error(e: SolverError) -> Self {
        Self::SolverError(e)
    }

    /// Create an invalid anchor error (Feature 009)
    pub fn invalid_anchor(
        element: impl Into<String>,
        anchor: impl Into<String>,
        valid_anchors: Vec<String>,
        span: Span,
    ) -> Self {
        Self::InvalidAnchor {
            element: element.into(),
            anchor: anchor.into(),
            valid_anchors: valid_anchors.join(", "),
            span,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_undefined_identifier_display() {
        let err = LayoutError::undefined("foo", 0..3, vec!["bar".to_string()]);
        assert!(err.to_string().contains("foo"));
    }

    #[test]
    fn test_conflicting_constraints_display() {
        let err = LayoutError::conflicting(
            vec!["a below b".to_string(), "a above b".to_string()],
            "cannot satisfy both",
        );
        assert!(err.to_string().contains("conflicting"));
    }

    #[test]
    fn test_circular_constraint_display() {
        let err = LayoutError::circular(vec!["a".to_string(), "b".to_string(), "a".to_string()]);
        assert!(err.to_string().contains("a -> b -> a"));
    }
}
