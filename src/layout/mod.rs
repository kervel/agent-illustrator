//! Layout engine for computing element positions and sizes
//!
//! This module takes a parsed AST and computes the spatial layout,
//! producing a LayoutResult with positioned elements and routed connections.

pub mod collector;
pub mod config;
pub mod engine;
pub mod error;
pub mod routing;
pub mod solver;
pub mod types;

pub use collector::ConstraintCollector;
pub use solver::{
    ConstraintSolver, LayoutConstraint, LayoutProperty, LayoutVariable, Solution, SolverError,
};

#[cfg(test)]
mod solver_spike;

pub use config::LayoutConfig;
pub use engine::{compute, resolve_constrain_statements, resolve_constraints};
pub use error::LayoutError;
pub use routing::{route_connections, RoutingMode};
pub use types::*;

use std::collections::HashSet;

use crate::parser::ast::*;

/// Validate that all identifier references in the document resolve to defined elements.
pub fn validate_references(doc: &Document) -> Result<(), LayoutError> {
    let defined = collect_defined_identifiers(doc);

    for stmt in &doc.statements {
        validate_refs_in_statement(&stmt.node, &defined, &stmt.span)?;
    }
    Ok(())
}

fn collect_defined_identifiers(doc: &Document) -> HashSet<String> {
    let mut ids = HashSet::new();
    for stmt in &doc.statements {
        collect_ids_from_statement(&stmt.node, &mut ids);
    }
    ids
}

fn collect_ids_from_statement(stmt: &Statement, ids: &mut HashSet<String>) {
    match stmt {
        Statement::Shape(s) => {
            // Check ShapeDecl.name first
            if let Some(name) = &s.name {
                ids.insert(name.node.0.clone());
            }
            // For path shapes, the name is inside PathDecl
            if let ShapeType::Path(path_decl) = &s.shape_type.node {
                if let Some(name) = &path_decl.name {
                    ids.insert(name.node.0.clone());
                }
            }
        }
        Statement::Layout(l) => {
            if let Some(name) = &l.name {
                ids.insert(name.node.0.clone());
            }
            for child in &l.children {
                collect_ids_from_statement(&child.node, ids);
            }
        }
        Statement::Group(g) => {
            if let Some(name) = &g.name {
                ids.insert(name.node.0.clone());
            }
            for child in &g.children {
                collect_ids_from_statement(&child.node, ids);
            }
        }
        Statement::Label(inner) => {
            // Labels can contain elements that define identifiers
            collect_ids_from_statement(inner, ids);
        }
        Statement::Connection(_) | Statement::Constraint(_) | Statement::Constrain(_) => {
            // Connections and constraints don't define new identifiers
        }
        Statement::TemplateDecl(t) => {
            // Template declarations define new template names (not element identifiers)
            // The template name becomes available for instantiation
            ids.insert(t.name.node.0.clone());
        }
        Statement::TemplateInstance(inst) => {
            // Template instances define new element identifiers
            ids.insert(inst.instance_name.node.0.clone());
        }
        Statement::Export(_) | Statement::AnchorDecl(_) => {
            // Exports and anchor declarations don't define new element identifiers
        }
    }
}

fn validate_refs_in_statement(
    stmt: &Statement,
    defined: &HashSet<String>,
    _span: &Span,
) -> Result<(), LayoutError> {
    match stmt {
        Statement::Connection(conns) => {
            for c in conns {
                // Feature 009: AnchorReference.element contains the identifier
                if !defined.contains(&c.from.element.node.0) {
                    return Err(LayoutError::UndefinedIdentifier {
                        name: c.from.element.node.0.clone(),
                        span: c.from.element.span.clone(),
                        suggestions: find_similar(defined, &c.from.element.node.0, 2),
                    });
                }
                if !defined.contains(&c.to.element.node.0) {
                    return Err(LayoutError::UndefinedIdentifier {
                        name: c.to.element.node.0.clone(),
                        span: c.to.element.span.clone(),
                        suggestions: find_similar(defined, &c.to.element.node.0, 2),
                    });
                }
            }
        }
        Statement::Constraint(c) => {
            if !defined.contains(&c.subject.node.0) {
                return Err(LayoutError::UndefinedIdentifier {
                    name: c.subject.node.0.clone(),
                    span: c.subject.span.clone(),
                    suggestions: find_similar(defined, &c.subject.node.0, 2),
                });
            }
            if let Some(anchor) = &c.anchor {
                if !defined.contains(&anchor.node.0) {
                    return Err(LayoutError::UndefinedIdentifier {
                        name: anchor.node.0.clone(),
                        span: anchor.span.clone(),
                        suggestions: find_similar(defined, &anchor.node.0, 2),
                    });
                }
            }
        }
        Statement::Layout(l) => {
            for child in &l.children {
                validate_refs_in_statement(&child.node, defined, &child.span)?;
            }
        }
        Statement::Group(g) => {
            for child in &g.children {
                validate_refs_in_statement(&child.node, defined, &child.span)?;
            }
        }
        Statement::Label(inner) => {
            // Validate references inside the label's inner element
            validate_refs_in_statement(inner, defined, _span)?;
        }
        Statement::Constrain(c) => {
            // Validate element references in constrain expressions
            validate_constraint_expr_refs(&c.expr, defined, _span)?;
        }
        Statement::Shape(_) => {}
        Statement::TemplateDecl(_) => {
            // Template declarations are validated separately during template resolution
        }
        Statement::TemplateInstance(inst) => {
            // Validate that the template name is defined
            if !defined.contains(&inst.template_name.node.0) {
                return Err(LayoutError::UndefinedIdentifier {
                    name: inst.template_name.node.0.clone(),
                    span: inst.template_name.span.clone(),
                    suggestions: find_similar(defined, &inst.template_name.node.0, 2),
                });
            }
        }
        Statement::Export(_) | Statement::AnchorDecl(_) => {
            // Exports and anchor declarations are validated during template resolution
        }
    }
    Ok(())
}

/// Validate element references within a constraint expression
fn validate_constraint_expr_refs(
    expr: &crate::parser::ast::ConstraintExpr,
    defined: &HashSet<String>,
    _span: &Span,
) -> Result<(), LayoutError> {
    use crate::parser::ast::ConstraintExpr;

    // Helper to validate a property ref
    let validate_prop_ref =
        |prop_ref: &crate::parser::ast::PropertyRef| -> Result<(), LayoutError> {
            let leaf_name = &prop_ref.element.node.leaf().0;
            if !defined.contains(leaf_name) {
                return Err(LayoutError::UndefinedIdentifier {
                    name: leaf_name.clone(),
                    span: prop_ref.element.span.clone(),
                    suggestions: find_similar(defined, leaf_name, 2),
                });
            }
            Ok(())
        };

    // Helper to validate an identifier
    let validate_ident =
        |id: &crate::parser::ast::Spanned<crate::parser::ast::Identifier>| -> Result<(), LayoutError> {
            if !defined.contains(&id.node.0) {
                return Err(LayoutError::UndefinedIdentifier {
                    name: id.node.0.clone(),
                    span: id.span.clone(),
                    suggestions: find_similar(defined, &id.node.0, 2),
                });
            }
            Ok(())
        };

    match expr {
        ConstraintExpr::Equal { left, right } => {
            validate_prop_ref(left)?;
            validate_prop_ref(right)?;
        }
        ConstraintExpr::EqualWithOffset { left, right, .. } => {
            validate_prop_ref(left)?;
            validate_prop_ref(right)?;
        }
        ConstraintExpr::Constant { left, .. } => {
            validate_prop_ref(left)?;
        }
        ConstraintExpr::GreaterOrEqual { left, .. } => {
            validate_prop_ref(left)?;
        }
        ConstraintExpr::LessOrEqual { left, .. } => {
            validate_prop_ref(left)?;
        }
        ConstraintExpr::Midpoint { target, a, b, .. } => {
            validate_prop_ref(target)?;
            validate_ident(a)?;
            validate_ident(b)?;
        }
        ConstraintExpr::Contains {
            container,
            elements,
            ..
        } => {
            validate_ident(container)?;
            for elem in elements {
                validate_ident(elem)?;
            }
        }
    }
    Ok(())
}

/// Compute Levenshtein edit distance between two strings
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    let mut dp = vec![vec![0usize; n + 1]; m + 1];

    for i in 0..=m {
        dp[i][0] = i;
    }
    for j in 0..=n {
        dp[0][j] = j;
    }

    for i in 1..=m {
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }

    dp[m][n]
}

/// Find similar identifiers within a maximum edit distance
fn find_similar(defined: &HashSet<String>, target: &str, max_distance: usize) -> Vec<String> {
    let mut candidates: Vec<(String, usize)> = defined
        .iter()
        .filter_map(|name| {
            let dist = levenshtein_distance(name, target);
            if dist <= max_distance && dist > 0 {
                Some((name.clone(), dist))
            } else {
                None
            }
        })
        .collect();

    candidates.sort_by_key(|(_, d)| *d);
    candidates
        .into_iter()
        .map(|(name, _)| name)
        .take(3)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_same() {
        assert_eq!(levenshtein_distance("hello", "hello"), 0);
    }

    #[test]
    fn test_levenshtein_one_off() {
        assert_eq!(levenshtein_distance("server", "servr"), 1);
        assert_eq!(levenshtein_distance("server", "servar"), 1);
    }

    #[test]
    fn test_levenshtein_different() {
        assert_eq!(levenshtein_distance("cat", "dog"), 3);
    }

    #[test]
    fn test_find_similar() {
        let mut defined = HashSet::new();
        defined.insert("server".to_string());
        defined.insert("client".to_string());
        defined.insert("database".to_string());

        let suggestions = find_similar(&defined, "servr", 2);
        assert!(suggestions.contains(&"server".to_string()));
    }
}
