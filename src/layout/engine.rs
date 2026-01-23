//! Layout computation engine

use std::collections::HashMap;

use crate::parser::ast::*;

use super::config::LayoutConfig;
use super::error::LayoutError;
use super::types::*;

/// Compute the layout for a document
pub fn compute(doc: &Document, config: &LayoutConfig) -> Result<LayoutResult, LayoutError> {
    // First validate references
    super::validate_references(doc)?;

    let mut result = LayoutResult::new();
    let mut position = Point::new(0.0, 0.0);

    for stmt in &doc.statements {
        match &stmt.node {
            // Skip connections and constraints - handled later
            Statement::Connection(_) | Statement::Constraint(_) => continue,
            _ => {
                let element = layout_statement(&stmt.node, position, config);
                position.y += element.bounds.height + config.element_spacing;
                result.add_element(element);
            }
        }
    }

    result.compute_bounds();
    Ok(result)
}

/// Resolve position constraints after initial layout
pub fn resolve_constraints(result: &mut LayoutResult, doc: &Document) -> Result<(), LayoutError> {
    let graph = ConstraintGraph::from_document(doc);

    // Check for conflicts before applying
    detect_conflicts(doc)?;

    // Get topological order (or error on cycles)
    let order = graph.topological_order()?;

    // Apply constraints in order
    for subject_id in order {
        if let Some(constraints) = graph.constraints.get(&subject_id) {
            for (relation, anchor_id) in constraints {
                apply_constraint(result, &subject_id, relation, anchor_id, &graph.config)?;
            }
        }
    }

    // Recompute bounds after constraint resolution
    result.compute_bounds();
    Ok(())
}

fn layout_statement(stmt: &Statement, position: Point, config: &LayoutConfig) -> ElementLayout {
    match stmt {
        Statement::Shape(s) => layout_shape(s, position, config),
        Statement::Layout(l) => layout_container(l, position, config),
        Statement::Group(g) => layout_group(g, position, config),
        Statement::Connection(_) | Statement::Constraint(_) => {
            // These are handled separately
            unreachable!("Connections and constraints should be filtered out")
        }
    }
}

fn layout_shape(shape: &ShapeDecl, position: Point, config: &LayoutConfig) -> ElementLayout {
    let (width, height) = compute_shape_size(shape, config);
    let styles = ResolvedStyles::from_modifiers(&shape.modifiers);

    let label = extract_label(&shape.modifiers).map(|text| LabelLayout {
        text,
        position: Point::new(position.x + width / 2.0, position.y + height / 2.0),
        anchor: TextAnchor::Middle,
    });

    ElementLayout {
        id: shape.name.as_ref().map(|n| n.node.clone()),
        element_type: ElementType::Shape(shape.shape_type.node.clone()),
        bounds: BoundingBox::new(position.x, position.y, width, height),
        styles,
        children: vec![],
        label,
    }
}

fn compute_shape_size(shape: &ShapeDecl, config: &LayoutConfig) -> (f64, f64) {
    match &shape.shape_type.node {
        ShapeType::Rectangle => config.default_rect_size,
        ShapeType::Circle => {
            let d = config.default_circle_radius * 2.0;
            (d, d)
        }
        ShapeType::Ellipse => config.default_ellipse_size,
        ShapeType::Polygon => config.default_rect_size,
        ShapeType::Icon { .. } => config.default_rect_size,
        ShapeType::Line => (config.default_line_width, 4.0),
    }
}

fn extract_label(modifiers: &[Spanned<StyleModifier>]) -> Option<String> {
    modifiers.iter().find_map(|m| {
        if matches!(m.node.key.node, StyleKey::Label) {
            match &m.node.value.node {
                StyleValue::String(s) => Some(s.clone()),
                _ => None,
            }
        } else {
            None
        }
    })
}

fn layout_container(layout: &LayoutDecl, position: Point, config: &LayoutConfig) -> ElementLayout {
    let (children, bounds) = match layout.layout_type.node {
        LayoutType::Row => layout_row(&layout.children, position, config),
        LayoutType::Column => layout_column(&layout.children, position, config),
        LayoutType::Grid => layout_grid(&layout.children, position, config),
        LayoutType::Stack => layout_stack(&layout.children, position, config),
    };

    let styles = ResolvedStyles::from_modifiers(&layout.modifiers);
    let label = extract_label(&layout.modifiers).map(|text| LabelLayout {
        text,
        position: Point::new(bounds.x + bounds.width / 2.0, bounds.y - 5.0),
        anchor: TextAnchor::Middle,
    });

    ElementLayout {
        id: layout.name.as_ref().map(|n| n.node.clone()),
        element_type: ElementType::Layout(layout.layout_type.node),
        bounds,
        styles,
        children,
        label,
    }
}

fn layout_group(group: &GroupDecl, position: Point, config: &LayoutConfig) -> ElementLayout {
    // Groups default to column layout
    let (children, bounds) = layout_column(&group.children, position, config);

    let styles = ResolvedStyles::from_modifiers(&group.modifiers);
    // Place label on the left side of the group, vertically centered
    let label = extract_label(&group.modifiers).map(|text| LabelLayout {
        text,
        position: Point::new(bounds.x - 10.0, bounds.y + bounds.height / 2.0),
        anchor: TextAnchor::End,
    });

    ElementLayout {
        id: group.name.as_ref().map(|n| n.node.clone()),
        element_type: ElementType::Group,
        bounds,
        styles,
        children,
        label,
    }
}

fn layout_row(
    children: &[Spanned<Statement>],
    position: Point,
    config: &LayoutConfig,
) -> (Vec<ElementLayout>, BoundingBox) {
    let mut layouts = vec![];
    let mut x = position.x + config.container_padding;
    let mut max_height = 0.0f64;

    for child in children {
        // Skip connections and constraints
        if matches!(
            child.node,
            Statement::Connection(_) | Statement::Constraint(_)
        ) {
            continue;
        }

        let child_layout = layout_statement(
            &child.node,
            Point::new(x, position.y + config.container_padding),
            config,
        );
        x += child_layout.bounds.width + config.element_spacing;
        max_height = max_height.max(child_layout.bounds.height);
        layouts.push(child_layout);
    }

    let total_width = if layouts.is_empty() {
        config.container_padding * 2.0
    } else {
        x - position.x - config.element_spacing + config.container_padding
    };
    let total_height = max_height + 2.0 * config.container_padding;

    (
        layouts,
        BoundingBox::new(position.x, position.y, total_width, total_height),
    )
}

fn layout_column(
    children: &[Spanned<Statement>],
    position: Point,
    config: &LayoutConfig,
) -> (Vec<ElementLayout>, BoundingBox) {
    let mut layouts = vec![];
    let mut y = position.y + config.container_padding;
    let mut max_width = 0.0f64;

    for child in children {
        // Skip connections and constraints
        if matches!(
            child.node,
            Statement::Connection(_) | Statement::Constraint(_)
        ) {
            continue;
        }

        let child_layout = layout_statement(
            &child.node,
            Point::new(position.x + config.container_padding, y),
            config,
        );
        y += child_layout.bounds.height + config.element_spacing;
        max_width = max_width.max(child_layout.bounds.width);
        layouts.push(child_layout);
    }

    let total_width = max_width + 2.0 * config.container_padding;
    let total_height = if layouts.is_empty() {
        config.container_padding * 2.0
    } else {
        y - position.y - config.element_spacing + config.container_padding
    };

    (
        layouts,
        BoundingBox::new(position.x, position.y, total_width, total_height),
    )
}

fn layout_grid(
    children: &[Spanned<Statement>],
    position: Point,
    config: &LayoutConfig,
) -> (Vec<ElementLayout>, BoundingBox) {
    // Filter out connections and constraints
    let filtered: Vec<_> = children
        .iter()
        .filter(|c| !matches!(c.node, Statement::Connection(_) | Statement::Constraint(_)))
        .collect();

    if filtered.is_empty() {
        return (
            vec![],
            BoundingBox::new(
                position.x,
                position.y,
                config.container_padding * 2.0,
                config.container_padding * 2.0,
            ),
        );
    }

    let n = filtered.len();
    let cols = (n as f64).sqrt().ceil() as usize;
    let rows = (n + cols - 1) / cols;

    // First pass: compute max cell size
    let mut max_cell_width = 0.0f64;
    let mut max_cell_height = 0.0f64;

    for child in &filtered {
        let temp = layout_statement(&child.node, Point::new(0.0, 0.0), config);
        max_cell_width = max_cell_width.max(temp.bounds.width);
        max_cell_height = max_cell_height.max(temp.bounds.height);
    }

    // Second pass: place in grid
    let mut layouts = vec![];
    for (i, child) in filtered.iter().enumerate() {
        let row = i / cols;
        let col = i % cols;
        let x = position.x
            + config.container_padding
            + col as f64 * (max_cell_width + config.element_spacing);
        let y = position.y
            + config.container_padding
            + row as f64 * (max_cell_height + config.element_spacing);
        layouts.push(layout_statement(&child.node, Point::new(x, y), config));
    }

    let total_width = cols as f64 * (max_cell_width + config.element_spacing)
        - config.element_spacing
        + 2.0 * config.container_padding;
    let total_height = rows as f64 * (max_cell_height + config.element_spacing)
        - config.element_spacing
        + 2.0 * config.container_padding;

    (
        layouts,
        BoundingBox::new(position.x, position.y, total_width, total_height),
    )
}

fn layout_stack(
    children: &[Spanned<Statement>],
    position: Point,
    config: &LayoutConfig,
) -> (Vec<ElementLayout>, BoundingBox) {
    // First pass: compute all layouts and find max size
    let mut temp_layouts = vec![];
    let mut max_width = 0.0f64;
    let mut max_height = 0.0f64;

    for child in children {
        // Skip connections and constraints
        if matches!(
            child.node,
            Statement::Connection(_) | Statement::Constraint(_)
        ) {
            continue;
        }

        let child_layout = layout_statement(
            &child.node,
            Point::new(
                position.x + config.container_padding,
                position.y + config.container_padding,
            ),
            config,
        );
        max_width = max_width.max(child_layout.bounds.width);
        max_height = max_height.max(child_layout.bounds.height);
        temp_layouts.push(child_layout);
    }

    // Second pass: center each child within the max bounds
    let mut layouts = vec![];
    for mut layout in temp_layouts {
        let dx = (max_width - layout.bounds.width) / 2.0;
        let dy = (max_height - layout.bounds.height) / 2.0;
        offset_element(&mut layout, dx, dy);
        layouts.push(layout);
    }

    (
        layouts,
        BoundingBox::new(
            position.x,
            position.y,
            max_width + 2.0 * config.container_padding,
            max_height + 2.0 * config.container_padding,
        ),
    )
}

/// Recursively offset an element and all its children
fn offset_element(element: &mut ElementLayout, dx: f64, dy: f64) {
    element.bounds.x += dx;
    element.bounds.y += dy;
    if let Some(label) = &mut element.label {
        label.position.x += dx;
        label.position.y += dy;
    }
    for child in &mut element.children {
        offset_element(child, dx, dy);
    }
}

// ============================================================================
// Constraint Resolution
// ============================================================================

struct ConstraintGraph {
    constraints: HashMap<String, Vec<(PositionRelation, String)>>,
    config: LayoutConfig,
}

impl ConstraintGraph {
    fn from_document(doc: &Document) -> Self {
        let mut constraints: HashMap<String, Vec<(PositionRelation, String)>> = HashMap::new();

        fn collect_constraints(
            stmts: &[Spanned<Statement>],
            constraints: &mut HashMap<String, Vec<(PositionRelation, String)>>,
        ) {
            for stmt in stmts {
                match &stmt.node {
                    Statement::Constraint(c) => {
                        constraints
                            .entry(c.subject.node.0.clone())
                            .or_default()
                            .push((c.relation.node, c.anchor.node.0.clone()));
                    }
                    Statement::Layout(l) => {
                        collect_constraints(&l.children, constraints);
                    }
                    Statement::Group(g) => {
                        collect_constraints(&g.children, constraints);
                    }
                    _ => {}
                }
            }
        }

        collect_constraints(&doc.statements, &mut constraints);

        Self {
            constraints,
            config: LayoutConfig::default(),
        }
    }

    fn topological_order(&self) -> Result<Vec<String>, LayoutError> {
        // Build dependency graph: subject depends on anchor
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut dependents: HashMap<String, Vec<String>> = HashMap::new();

        // Initialize all nodes
        for subject in self.constraints.keys() {
            in_degree.entry(subject.clone()).or_insert(0);
            for (_, anchor) in self.constraints.get(subject).unwrap() {
                in_degree.entry(anchor.clone()).or_insert(0);
            }
        }

        // Build edges
        for (subject, relations) in &self.constraints {
            for (_, anchor) in relations {
                *in_degree.entry(subject.clone()).or_insert(0) += 1;
                dependents.entry(anchor.clone()).or_default().push(subject.clone());
            }
        }

        // Kahn's algorithm
        let mut queue: Vec<String> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(k, _)| k.clone())
            .collect();

        let mut result = vec![];

        while let Some(node) = queue.pop() {
            result.push(node.clone());
            if let Some(deps) = dependents.get(&node) {
                for dep in deps {
                    if let Some(deg) = in_degree.get_mut(dep) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(dep.clone());
                        }
                    }
                }
            }
        }

        // Check for cycle
        if result.len() < in_degree.len() {
            // Find cycle for error message
            let remaining: Vec<String> = in_degree
                .iter()
                .filter(|(_, &deg)| deg > 0)
                .map(|(k, _)| k.clone())
                .collect();
            return Err(LayoutError::circular(remaining));
        }

        Ok(result)
    }
}

fn apply_constraint(
    result: &mut LayoutResult,
    subject_id: &str,
    relation: &PositionRelation,
    anchor_id: &str,
    _config: &LayoutConfig,
) -> Result<(), LayoutError> {
    let anchor_bounds = result
        .get_element_by_name(anchor_id)
        .ok_or_else(|| LayoutError::undefined(anchor_id, 0..0, vec![]))?
        .bounds;

    let subject = result
        .get_element_mut_by_name(subject_id)
        .ok_or_else(|| LayoutError::undefined(subject_id, 0..0, vec![]))?;

    let spacing = 20.0; // Could come from config

    match relation {
        PositionRelation::RightOf => {
            subject.bounds.x = anchor_bounds.right() + spacing;
            subject.bounds.y = anchor_bounds.y;
        }
        PositionRelation::LeftOf => {
            subject.bounds.x = anchor_bounds.x - subject.bounds.width - spacing;
            subject.bounds.y = anchor_bounds.y;
        }
        PositionRelation::Below => {
            subject.bounds.x = anchor_bounds.x;
            subject.bounds.y = anchor_bounds.bottom() + spacing;
        }
        PositionRelation::Above => {
            subject.bounds.x = anchor_bounds.x;
            subject.bounds.y = anchor_bounds.y - subject.bounds.height - spacing;
        }
        PositionRelation::Inside => {
            subject.bounds.x =
                anchor_bounds.x + (anchor_bounds.width - subject.bounds.width) / 2.0;
            subject.bounds.y =
                anchor_bounds.y + (anchor_bounds.height - subject.bounds.height) / 2.0;
        }
    }

    // Update label position if present
    if let Some(label) = &mut subject.label {
        label.position = subject.bounds.center();
    }

    Ok(())
}

fn detect_conflicts(doc: &Document) -> Result<(), LayoutError> {
    let mut subject_relations: HashMap<String, Vec<(PositionRelation, String)>> = HashMap::new();

    fn collect_all(
        stmts: &[Spanned<Statement>],
        map: &mut HashMap<String, Vec<(PositionRelation, String)>>,
    ) {
        for stmt in stmts {
            match &stmt.node {
                Statement::Constraint(c) => {
                    map.entry(c.subject.node.0.clone())
                        .or_default()
                        .push((c.relation.node, c.anchor.node.0.clone()));
                }
                Statement::Layout(l) => collect_all(&l.children, map),
                Statement::Group(g) => collect_all(&g.children, map),
                _ => {}
            }
        }
    }

    collect_all(&doc.statements, &mut subject_relations);

    // Check for conflicting relations on the same subject
    for (subject, relations) in &subject_relations {
        for (i, (rel1, anchor1)) in relations.iter().enumerate() {
            for (rel2, anchor2) in relations.iter().skip(i + 1) {
                if are_conflicting(rel1, rel2) {
                    return Err(LayoutError::conflicting(
                        vec![
                            format!("{} {:?} {}", subject, rel1, anchor1),
                            format!("{} {:?} {}", subject, rel2, anchor2),
                        ],
                        "Cannot satisfy opposing position constraints on the same element",
                    ));
                }
            }
        }
    }

    Ok(())
}

fn are_conflicting(a: &PositionRelation, b: &PositionRelation) -> bool {
    matches!(
        (a, b),
        (PositionRelation::RightOf, PositionRelation::LeftOf)
            | (PositionRelation::LeftOf, PositionRelation::RightOf)
            | (PositionRelation::Above, PositionRelation::Below)
            | (PositionRelation::Below, PositionRelation::Above)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn test_layout_single_shape() {
        let doc = parse("rect server").unwrap();
        let config = LayoutConfig::default();
        let result = compute(&doc, &config).unwrap();

        assert_eq!(result.root_elements.len(), 1);
        assert_eq!(result.root_elements[0].bounds.width, 80.0);
        assert_eq!(result.root_elements[0].bounds.height, 30.0);
    }

    #[test]
    fn test_layout_row() {
        let doc = parse("row { rect a rect b }").unwrap();
        let config = LayoutConfig::default();
        let result = compute(&doc, &config).unwrap();

        assert_eq!(result.root_elements.len(), 1);
        let container = &result.root_elements[0];
        assert_eq!(container.children.len(), 2);

        // Second element should be to the right of first
        let a_bounds = &container.children[0].bounds;
        let b_bounds = &container.children[1].bounds;
        assert!(b_bounds.x > a_bounds.right());
    }

    #[test]
    fn test_layout_column() {
        let doc = parse("col { rect a rect b }").unwrap();
        let config = LayoutConfig::default();
        let result = compute(&doc, &config).unwrap();

        assert_eq!(result.root_elements.len(), 1);
        let container = &result.root_elements[0];
        assert_eq!(container.children.len(), 2);

        // Second element should be below first
        let a_bounds = &container.children[0].bounds;
        let b_bounds = &container.children[1].bounds;
        assert!(b_bounds.y > a_bounds.bottom());
    }

    #[test]
    fn test_layout_nested() {
        let doc = parse(
            r#"
            col {
                row { rect a rect b }
                row { rect c rect d }
            }
        "#,
        )
        .unwrap();
        let config = LayoutConfig::default();
        let result = compute(&doc, &config).unwrap();

        assert_eq!(result.root_elements.len(), 1);
        let outer = &result.root_elements[0];
        assert_eq!(outer.children.len(), 2);

        // Each row should have 2 children
        assert_eq!(outer.children[0].children.len(), 2);
        assert_eq!(outer.children[1].children.len(), 2);
    }
}
