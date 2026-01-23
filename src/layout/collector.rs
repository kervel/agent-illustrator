//! Collects constraints from AST for solving
//!
//! This module walks the AST and extracts all constraints that should be
//! solved by the constraint solver, including:
//! - Intrinsic constraints (explicit sizes)
//! - Layout container constraints (row, col, stack, grid)
//! - User constraints (constrain statements, align statements)

use crate::parser::ast::*;

use super::config::LayoutConfig;
use super::solver::{ConstraintOrigin, ConstraintSource, LayoutConstraint, LayoutVariable};

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

    // ========================================================================
    // T021: Intrinsic constraint collection
    // ========================================================================

    fn collect_intrinsics(&mut self, doc: &Document) {
        self.collect_shape_intrinsics(&doc.statements);
    }

    fn collect_shape_intrinsics(&mut self, stmts: &[Spanned<Statement>]) {
        for stmt in stmts {
            match &stmt.node {
                Statement::Shape(s) => {
                    if let Some(name) = &s.name {
                        let id = name.node.0.clone();

                        // Extract explicit width from modifiers
                        if let Some(w) = extract_number_modifier(&s.modifiers, "width")
                            .or_else(|| extract_number_modifier(&s.modifiers, "size"))
                        {
                            self.constraints.push(LayoutConstraint::Fixed {
                                variable: LayoutVariable::width(&id),
                                value: w,
                                source: ConstraintSource::intrinsic(format!("{} width", id)),
                            });
                        }

                        // Extract explicit height from modifiers
                        if let Some(h) = extract_number_modifier(&s.modifiers, "height")
                            .or_else(|| extract_number_modifier(&s.modifiers, "size"))
                        {
                            self.constraints.push(LayoutConstraint::Fixed {
                                variable: LayoutVariable::height(&id),
                                value: h,
                                source: ConstraintSource::intrinsic(format!("{} height", id)),
                            });
                        }
                    }
                }
                Statement::Layout(l) => self.collect_shape_intrinsics(&l.children),
                Statement::Group(g) => self.collect_shape_intrinsics(&g.children),
                Statement::Label(inner) => {
                    // Check if inner statement defines intrinsics
                    self.collect_shape_intrinsics(&[Spanned::new(inner.as_ref().clone(), 0..0)]);
                }
                _ => {}
            }
        }
    }

    // ========================================================================
    // T022: Layout container constraint generation
    // ========================================================================

    fn collect_layout_constraints(&mut self, stmts: &[Spanned<Statement>]) {
        for stmt in stmts {
            match &stmt.node {
                Statement::Layout(l) => {
                    let child_ids = self.get_child_ids(&l.children);
                    if child_ids.is_empty() {
                        continue;
                    }

                    match l.layout_type.node {
                        LayoutType::Row => {
                            self.collect_row_constraints(&child_ids, l, &stmt.span)
                        }
                        LayoutType::Column => {
                            self.collect_column_constraints(&child_ids, l, &stmt.span)
                        }
                        LayoutType::Stack => {
                            self.collect_stack_constraints(&child_ids, l, &stmt.span)
                        }
                        LayoutType::Grid => {
                            // Grid is more complex - skip for now
                        }
                    }

                    // Recurse into children
                    self.collect_layout_constraints(&l.children);
                }
                Statement::Group(g) => {
                    self.collect_layout_constraints(&g.children);
                }
                _ => {}
            }
        }
    }

    fn collect_row_constraints(
        &mut self,
        child_ids: &[String],
        layout: &LayoutDecl,
        span: &Span,
    ) {
        let gap = extract_number_modifier(&layout.modifiers, "gap")
            .unwrap_or(self.config.element_spacing);

        // Align all children vertically (same y)
        for i in 1..child_ids.len() {
            self.constraints.push(LayoutConstraint::Equal {
                left: LayoutVariable::y(&child_ids[i]),
                right: LayoutVariable::y(&child_ids[0]),
                offset: 0.0,
                source: ConstraintSource {
                    span: span.clone(),
                    description: format!("row vertical alignment: {} = {}", child_ids[i], child_ids[0]),
                    origin: ConstraintOrigin::LayoutContainer,
                },
            });
        }

        // Position children horizontally with gap
        // Note: This simplified version doesn't account for width, which would require
        // either knowing widths at constraint time or using derived variables.
        // For now, we just record the intent - actual positioning uses the procedural engine.
        for i in 1..child_ids.len() {
            let prev = &child_ids[i - 1];
            let curr = &child_ids[i];
            // Record that curr should be to the right of prev
            // This is a simplified constraint that the solver integration will expand
            self.constraints.push(LayoutConstraint::Equal {
                left: LayoutVariable::x(curr),
                right: LayoutVariable::x(prev),
                offset: gap, // Simplified: actual offset depends on prev.width
                source: ConstraintSource {
                    span: span.clone(),
                    description: format!("row horizontal: {} after {}", curr, prev),
                    origin: ConstraintOrigin::LayoutContainer,
                },
            });
        }
    }

    fn collect_column_constraints(
        &mut self,
        child_ids: &[String],
        layout: &LayoutDecl,
        span: &Span,
    ) {
        let gap = extract_number_modifier(&layout.modifiers, "gap")
            .unwrap_or(self.config.element_spacing);

        // Align all children horizontally (same x)
        for i in 1..child_ids.len() {
            self.constraints.push(LayoutConstraint::Equal {
                left: LayoutVariable::x(&child_ids[i]),
                right: LayoutVariable::x(&child_ids[0]),
                offset: 0.0,
                source: ConstraintSource {
                    span: span.clone(),
                    description: format!("col horizontal alignment: {} = {}", child_ids[i], child_ids[0]),
                    origin: ConstraintOrigin::LayoutContainer,
                },
            });
        }

        // Position children vertically with gap
        for i in 1..child_ids.len() {
            let prev = &child_ids[i - 1];
            let curr = &child_ids[i];
            self.constraints.push(LayoutConstraint::Equal {
                left: LayoutVariable::y(curr),
                right: LayoutVariable::y(prev),
                offset: gap, // Simplified: actual offset depends on prev.height
                source: ConstraintSource {
                    span: span.clone(),
                    description: format!("col vertical: {} after {}", curr, prev),
                    origin: ConstraintOrigin::LayoutContainer,
                },
            });
        }
    }

    fn collect_stack_constraints(
        &mut self,
        child_ids: &[String],
        _layout: &LayoutDecl,
        span: &Span,
    ) {
        // Stack: all children at same position (overlapping)
        for i in 1..child_ids.len() {
            // Same x
            self.constraints.push(LayoutConstraint::Equal {
                left: LayoutVariable::x(&child_ids[i]),
                right: LayoutVariable::x(&child_ids[0]),
                offset: 0.0,
                source: ConstraintSource {
                    span: span.clone(),
                    description: format!("stack x: {} = {}", child_ids[i], child_ids[0]),
                    origin: ConstraintOrigin::LayoutContainer,
                },
            });
            // Same y
            self.constraints.push(LayoutConstraint::Equal {
                left: LayoutVariable::y(&child_ids[i]),
                right: LayoutVariable::y(&child_ids[0]),
                offset: 0.0,
                source: ConstraintSource {
                    span: span.clone(),
                    description: format!("stack y: {} = {}", child_ids[i], child_ids[0]),
                    origin: ConstraintOrigin::LayoutContainer,
                },
            });
        }
    }

    /// Get IDs of named children in a list of statements
    fn get_child_ids(&self, children: &[Spanned<Statement>]) -> Vec<String> {
        let mut ids = Vec::new();
        for child in children {
            match &child.node {
                Statement::Shape(s) => {
                    if let Some(name) = &s.name {
                        ids.push(name.node.0.clone());
                    }
                }
                Statement::Layout(l) => {
                    if let Some(name) = &l.name {
                        ids.push(name.node.0.clone());
                    }
                }
                Statement::Group(g) => {
                    if let Some(name) = &g.name {
                        ids.push(name.node.0.clone());
                    }
                }
                Statement::Label(inner) => {
                    // Labels can contain named elements
                    if let Statement::Shape(s) = inner.as_ref() {
                        if let Some(name) = &s.name {
                            ids.push(name.node.0.clone());
                        }
                    }
                }
                _ => {}
            }
        }
        ids
    }

    // ========================================================================
    // T023: User constraint collection (constrain statements)
    // ========================================================================

    fn collect_user_constraints(&mut self, stmts: &[Spanned<Statement>]) {
        for stmt in stmts {
            match &stmt.node {
                Statement::Constrain(c) => self.collect_constrain(&c.expr, &stmt.span),
                Statement::Layout(l) => self.collect_user_constraints(&l.children),
                Statement::Group(g) => self.collect_user_constraints(&g.children),
                _ => {}
            }
        }
    }

    /// Public method to collect a single constrain expression
    /// Used by the engine to collect constraints selectively
    pub fn collect_constrain_expr(&mut self, expr: &ConstraintExpr, span: &Span) {
        self.collect_constrain(expr, span);
    }

    fn collect_constrain(&mut self, expr: &ConstraintExpr, span: &Span) {
        match expr {
            ConstraintExpr::Equal { left, right } => {
                let left_var = self.property_to_variable(left);
                let right_var = self.property_to_variable(right);

                self.constraints.push(LayoutConstraint::Equal {
                    left: left_var,
                    right: right_var,
                    offset: 0.0,
                    source: ConstraintSource::user(span.clone(), "constrain equality"),
                });
            }

            ConstraintExpr::EqualWithOffset {
                left,
                right,
                offset,
            } => {
                let left_var = self.property_to_variable(left);
                let right_var = self.property_to_variable(right);

                self.constraints.push(LayoutConstraint::Equal {
                    left: left_var,
                    right: right_var,
                    offset: *offset,
                    source: ConstraintSource::user(span.clone(), "constrain with offset"),
                });
            }

            ConstraintExpr::Constant { left, value } => {
                let var = self.property_to_variable(left);
                self.constraints.push(LayoutConstraint::Fixed {
                    variable: var,
                    value: *value,
                    source: ConstraintSource::user(span.clone(), "constrain constant"),
                });
            }

            ConstraintExpr::GreaterOrEqual { left, value } => {
                let var = self.property_to_variable(left);
                self.constraints.push(LayoutConstraint::GreaterOrEqual {
                    variable: var,
                    value: *value,
                    source: ConstraintSource::user(span.clone(), "constrain >="),
                });
            }

            ConstraintExpr::LessOrEqual { left, value } => {
                let var = self.property_to_variable(left);
                self.constraints.push(LayoutConstraint::LessOrEqual {
                    variable: var,
                    value: *value,
                    source: ConstraintSource::user(span.clone(), "constrain <="),
                });
            }

            ConstraintExpr::Midpoint { target, a, b } => {
                let target_var = self.property_to_variable(target);

                // For midpoint of elements, we use the same property type as target
                // e.g., if target is a.center_y, we use b.center_y and c.center_y
                // This ensures centers are compared with centers for proper alignment
                let a_var = LayoutVariable::new(
                    &a.node.0,
                    match target.property.node {
                        ConstraintProperty::CenterX | ConstraintProperty::Center => {
                            super::solver::LayoutProperty::CenterX
                        }
                        ConstraintProperty::CenterY => super::solver::LayoutProperty::CenterY,
                        ConstraintProperty::X | ConstraintProperty::Left => {
                            super::solver::LayoutProperty::X
                        }
                        ConstraintProperty::Y | ConstraintProperty::Top => {
                            super::solver::LayoutProperty::Y
                        }
                        _ => super::solver::LayoutProperty::X,
                    },
                );
                let b_var = LayoutVariable::new(
                    &b.node.0,
                    match target.property.node {
                        ConstraintProperty::CenterX | ConstraintProperty::Center => {
                            super::solver::LayoutProperty::CenterX
                        }
                        ConstraintProperty::CenterY => super::solver::LayoutProperty::CenterY,
                        ConstraintProperty::X | ConstraintProperty::Left => {
                            super::solver::LayoutProperty::X
                        }
                        ConstraintProperty::Y | ConstraintProperty::Top => {
                            super::solver::LayoutProperty::Y
                        }
                        _ => super::solver::LayoutProperty::X,
                    },
                );

                self.constraints.push(LayoutConstraint::Midpoint {
                    target: target_var,
                    a: a_var,
                    b: b_var,
                    source: ConstraintSource::user(span.clone(), "constrain midpoint"),
                });
            }

            ConstraintExpr::Contains {
                container,
                elements,
                padding,
            } => {
                let pad = padding.unwrap_or(0.0);

                // For containment, we generate inequality constraints:
                // container.left <= element.left - padding
                // container.right >= element.right + padding
                // (and same for top/bottom)
                for elem in elements {
                    // Left edge
                    self.constraints.push(LayoutConstraint::LessOrEqual {
                        variable: LayoutVariable::x(&container.node.0),
                        value: 0.0, // This is simplified - actual constraint is relative
                        source: ConstraintSource::user(
                            span.clone(),
                            format!("{} contains {} (left)", container.node.0, elem.node.0),
                        ),
                    });

                    // Note: Full containment constraints require more complex handling
                    // with derived properties. For now, we just record the intent.
                }

                // Store containment as a series of inequality hints
                // The actual implementation will handle this specially
                let _ = (container, elements, pad); // Mark as used
            }
        }
    }

    /// Convert a PropertyRef to a LayoutVariable
    fn property_to_variable(&self, prop_ref: &PropertyRef) -> LayoutVariable {
        let id = prop_ref.element.node.leaf().0.clone();

        // Map constraint properties to layout properties
        let property = match prop_ref.property.node {
            ConstraintProperty::X | ConstraintProperty::Left => super::solver::LayoutProperty::X,
            ConstraintProperty::Y | ConstraintProperty::Top => super::solver::LayoutProperty::Y,
            ConstraintProperty::Width => super::solver::LayoutProperty::Width,
            ConstraintProperty::Height => super::solver::LayoutProperty::Height,
            // Center properties are derived (center_x = x + width/2)
            ConstraintProperty::CenterX | ConstraintProperty::Center => {
                super::solver::LayoutProperty::CenterX
            }
            ConstraintProperty::CenterY => super::solver::LayoutProperty::CenterY,
            // Right/Bottom need special handling too (right = x + width)
            // For now, map to base properties - TODO: add Right/Bottom to LayoutProperty
            ConstraintProperty::Right => super::solver::LayoutProperty::X,
            ConstraintProperty::Bottom => super::solver::LayoutProperty::Y,
        };

        LayoutVariable::new(id, property)
    }
}

// ============================================================================
// Helper functions
// ============================================================================

/// Extract a numeric modifier value by key name
fn extract_number_modifier(modifiers: &[Spanned<StyleModifier>], key: &str) -> Option<f64> {
    for m in modifiers {
        let key_matches = match &m.node.key.node {
            StyleKey::Width if key == "width" => true,
            StyleKey::Height if key == "height" => true,
            StyleKey::Size if key == "size" => true,
            StyleKey::Gap if key == "gap" => true,
            StyleKey::Custom(k) if k == key => true,
            _ => false,
        };

        if key_matches {
            if let StyleValue::Number { value, .. } = &m.node.value.node {
                return Some(*value);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn test_collect_intrinsic_width() {
        let doc = parse("rect a [width: 100]").unwrap();
        let mut collector = ConstraintCollector::new(LayoutConfig::default());
        collector.collect(&doc);

        // Should have one fixed constraint for width
        assert_eq!(collector.constraints.len(), 1);
        match &collector.constraints[0] {
            LayoutConstraint::Fixed { variable, value, .. } => {
                assert_eq!(variable.element_id, "a");
                assert_eq!(variable.property, super::super::solver::LayoutProperty::Width);
                assert!((value - 100.0).abs() < 0.001);
            }
            _ => panic!("Expected Fixed constraint"),
        }
    }

    #[test]
    fn test_collect_intrinsic_size() {
        let doc = parse("rect a [size: 50]").unwrap();
        let mut collector = ConstraintCollector::new(LayoutConfig::default());
        collector.collect(&doc);

        // Should have two fixed constraints (width and height)
        assert_eq!(collector.constraints.len(), 2);
    }

    #[test]
    fn test_collect_constrain_equality() {
        let doc = parse("rect a\nrect b\nconstrain a.left = b.left").unwrap();
        let mut collector = ConstraintCollector::new(LayoutConfig::default());
        collector.collect(&doc);

        // Find the equality constraint
        let eq_constraint = collector
            .constraints
            .iter()
            .find(|c| matches!(c, LayoutConstraint::Equal { .. }));

        assert!(eq_constraint.is_some());
        if let Some(LayoutConstraint::Equal { left, right, offset, .. }) = eq_constraint {
            assert_eq!(left.element_id, "a");
            assert_eq!(right.element_id, "b");
            assert!((offset - 0.0).abs() < 0.001);
        }
    }

    #[test]
    fn test_collect_constrain_with_offset() {
        let doc = parse("rect a\nrect b\nconstrain a.x = b.x + 20").unwrap();
        let mut collector = ConstraintCollector::new(LayoutConfig::default());
        collector.collect(&doc);

        let eq_constraint = collector
            .constraints
            .iter()
            .find(|c| matches!(c, LayoutConstraint::Equal { .. }));

        assert!(eq_constraint.is_some());
        if let Some(LayoutConstraint::Equal { offset, .. }) = eq_constraint {
            assert!((*offset - 20.0).abs() < 0.001);
        }
    }

    #[test]
    fn test_collect_constrain_constant() {
        let doc = parse("rect a\nconstrain a.width = 150").unwrap();
        let mut collector = ConstraintCollector::new(LayoutConfig::default());
        collector.collect(&doc);

        let fixed_constraints: Vec<_> = collector
            .constraints
            .iter()
            .filter(|c| matches!(c, LayoutConstraint::Fixed { .. }))
            .collect();

        // Should have one fixed constraint for width=150
        assert!(fixed_constraints.len() >= 1);
        let user_constraint = fixed_constraints.iter().find(|c| {
            if let LayoutConstraint::Fixed { value, source, .. } = c {
                (*value - 150.0).abs() < 0.001 && source.origin == ConstraintOrigin::UserDefined
            } else {
                false
            }
        });
        assert!(user_constraint.is_some());
    }

    #[test]
    fn test_collect_constrain_inequality() {
        let doc = parse("rect a\nconstrain a.width >= 50").unwrap();
        let mut collector = ConstraintCollector::new(LayoutConfig::default());
        collector.collect(&doc);

        let ge_constraint = collector
            .constraints
            .iter()
            .find(|c| matches!(c, LayoutConstraint::GreaterOrEqual { .. }));

        assert!(ge_constraint.is_some());
        if let Some(LayoutConstraint::GreaterOrEqual { value, .. }) = ge_constraint {
            assert!((*value - 50.0).abs() < 0.001);
        }
    }

    #[test]
    fn test_collect_constrain_midpoint() {
        let doc = parse("rect a\nrect b\nrect c\nconstrain a.x = midpoint(b, c)").unwrap();
        let mut collector = ConstraintCollector::new(LayoutConfig::default());
        collector.collect(&doc);

        let mid_constraint = collector
            .constraints
            .iter()
            .find(|c| matches!(c, LayoutConstraint::Midpoint { .. }));

        assert!(mid_constraint.is_some());
        if let Some(LayoutConstraint::Midpoint { target, a, b, .. }) = mid_constraint {
            assert_eq!(target.element_id, "a");
            assert_eq!(a.element_id, "b");
            assert_eq!(b.element_id, "c");
        }
    }

    #[test]
    fn test_collect_row_constraints() {
        let doc = parse("row { rect a rect b rect c }").unwrap();
        let mut collector = ConstraintCollector::new(LayoutConfig::default());
        collector.collect(&doc);

        // Row should generate:
        // - 2 vertical alignment constraints (b.y = a.y, c.y = a.y)
        // - 2 horizontal positioning constraints (b.x after a, c.x after b)
        let layout_constraints: Vec<_> = collector
            .constraints
            .iter()
            .filter(|c| {
                if let LayoutConstraint::Equal { source, .. } = c {
                    source.origin == ConstraintOrigin::LayoutContainer
                } else {
                    false
                }
            })
            .collect();

        assert_eq!(layout_constraints.len(), 4);
    }
}
