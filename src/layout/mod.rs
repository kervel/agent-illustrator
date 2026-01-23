//! Layout engine for computing element positions and sizes
//!
//! This module takes a parsed AST and computes the spatial layout,
//! producing a LayoutResult with positioned elements and routed connections.

pub mod config;
pub mod engine;
pub mod error;
pub mod routing;
pub mod types;

pub use config::LayoutConfig;
pub use engine::{compute, resolve_constraints};
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
            if let Some(name) = &s.name {
                ids.insert(name.node.0.clone());
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
        Statement::Connection(_) | Statement::Constraint(_) | Statement::Alignment(_) => {
            // Connections, constraints, and alignments don't define new identifiers
        }
    }
}

fn validate_refs_in_statement(
    stmt: &Statement,
    defined: &HashSet<String>,
    _span: &Span,
) -> Result<(), LayoutError> {
    match stmt {
        Statement::Connection(c) => {
            if !defined.contains(&c.from.node.0) {
                return Err(LayoutError::UndefinedIdentifier {
                    name: c.from.node.0.clone(),
                    span: c.from.span.clone(),
                    suggestions: find_similar(defined, &c.from.node.0, 2),
                });
            }
            if !defined.contains(&c.to.node.0) {
                return Err(LayoutError::UndefinedIdentifier {
                    name: c.to.node.0.clone(),
                    span: c.to.span.clone(),
                    suggestions: find_similar(defined, &c.to.node.0, 2),
                });
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
            if !defined.contains(&c.anchor.node.0) {
                return Err(LayoutError::UndefinedIdentifier {
                    name: c.anchor.node.0.clone(),
                    span: c.anchor.span.clone(),
                    suggestions: find_similar(defined, &c.anchor.node.0, 2),
                });
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
        Statement::Alignment(a) => {
            // Validate that all element paths in the alignment reference defined elements
            for anchor in &a.anchors {
                // For now, only check simple (single-segment) paths against defined identifiers
                // Full path resolution will be implemented in Phase 5
                let leaf_name = &anchor.element.node.leaf().0;
                if !defined.contains(leaf_name) {
                    return Err(LayoutError::UndefinedIdentifier {
                        name: leaf_name.clone(),
                        span: anchor.element.span.clone(),
                        suggestions: find_similar(defined, leaf_name, 2),
                    });
                }
            }
        }
        Statement::Shape(_) => {}
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
