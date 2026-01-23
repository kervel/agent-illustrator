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
            // Skip connections, constraints, alignments, and standalone labels at document root
            Statement::Connection(_)
            | Statement::Constraint(_)
            | Statement::Alignment(_)
            | Statement::Label(_) => continue,
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
        Statement::Label(inner) => {
            // Layout the inner element - Label positioning is handled by the parent container
            layout_statement(inner, position, config)
        }
        Statement::Connection(_) | Statement::Constraint(_) | Statement::Alignment(_) => {
            // These are handled separately
            unreachable!("Connections, constraints, and alignments should be filtered out")
        }
    }
}

fn layout_shape(shape: &ShapeDecl, position: Point, config: &LayoutConfig) -> ElementLayout {
    let (width, height) = compute_shape_size(shape, config);
    let styles = ResolvedStyles::from_modifiers(&shape.modifiers);

    // For Line shapes, position label above the line with an offset
    // For other shapes, center the label within the shape
    let label = extract_label(&shape.modifiers).map(|text| {
        let (label_x, label_y, anchor) = match &shape.shape_type.node {
            ShapeType::Line => {
                // Center horizontally on the line, position above with offset
                let label_offset = 12.0; // pixels above the line
                (
                    position.x + width / 2.0,
                    position.y + height / 2.0 - label_offset,
                    TextAnchor::Middle,
                )
            }
            _ => {
                // Default: center within the shape bounds
                (
                    position.x + width / 2.0,
                    position.y + height / 2.0,
                    TextAnchor::Middle,
                )
            }
        };
        LabelLayout {
            text,
            position: Point::new(label_x, label_y),
            anchor,
        }
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
    // Extract size modifiers from the shape
    let size = extract_size_modifier(&shape.modifiers);
    let width = extract_width_modifier(&shape.modifiers);
    let height = extract_height_modifier(&shape.modifiers);

    // If explicit width and height are provided, use them
    if let (Some(w), Some(h)) = (width, height) {
        return (w, h);
    }

    // If only size is provided, use it for both dimensions (square/circle)
    if let Some(s) = size {
        return (s, s);
    }

    // If only width is provided, use it for width and default for height
    // If only height is provided, use default for width and it for height
    let (default_width, default_height) = match &shape.shape_type.node {
        ShapeType::Rectangle => config.default_rect_size,
        ShapeType::Circle => {
            let d = config.default_circle_radius * 2.0;
            (d, d)
        }
        ShapeType::Ellipse => config.default_ellipse_size,
        ShapeType::Polygon => config.default_rect_size,
        ShapeType::Icon { .. } => config.default_rect_size,
        ShapeType::Line => (config.default_line_width, 4.0),
        ShapeType::Text { content } => {
            // Estimate text size based on content length
            // Use font_size from modifiers if available, otherwise default to 14px
            let font_size = extract_font_size(&shape.modifiers).unwrap_or(14.0);
            // Approximate width: ~0.6 * font_size per character
            let estimated_width = content.len() as f64 * font_size * 0.6;
            // Height is approximately the font size
            (estimated_width.max(20.0), font_size)
        }
    };

    let final_width = width.unwrap_or(default_width);
    let final_height = height.unwrap_or(default_height);

    (final_width, final_height)
}

/// Extract the size modifier value from modifiers
fn extract_size_modifier(modifiers: &[Spanned<StyleModifier>]) -> Option<f64> {
    modifiers.iter().find_map(|m| {
        if matches!(m.node.key.node, StyleKey::Size) {
            match &m.node.value.node {
                StyleValue::Number { value, .. } => Some(*value),
                _ => None,
            }
        } else {
            None
        }
    })
}

/// Extract the width modifier value from modifiers
fn extract_width_modifier(modifiers: &[Spanned<StyleModifier>]) -> Option<f64> {
    modifiers.iter().find_map(|m| {
        if matches!(m.node.key.node, StyleKey::Width) {
            match &m.node.value.node {
                StyleValue::Number { value, .. } => Some(*value),
                _ => None,
            }
        } else {
            None
        }
    })
}

/// Extract the height modifier value from modifiers
fn extract_height_modifier(modifiers: &[Spanned<StyleModifier>]) -> Option<f64> {
    modifiers.iter().find_map(|m| {
        if matches!(m.node.key.node, StyleKey::Height) {
            match &m.node.value.node {
                StyleValue::Number { value, .. } => Some(*value),
                _ => None,
            }
        } else {
            None
        }
    })
}

/// Extract the font_size modifier value from modifiers
fn extract_font_size(modifiers: &[Spanned<StyleModifier>]) -> Option<f64> {
    modifiers.iter().find_map(|m| {
        if matches!(m.node.key.node, StyleKey::FontSize) {
            match &m.node.value.node {
                StyleValue::Number { value, .. } => Some(*value),
                _ => None,
            }
        } else {
            None
        }
    })
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

/// Extract the first Label statement from a list of children.
/// Returns the inner statement of the Label if found.
/// DEPRECATED: Use `[role: label]` modifier instead.
fn extract_label_statement(children: &[Spanned<Statement>]) -> Option<&Statement> {
    children.iter().find_map(|child| {
        if let Statement::Label(inner) = &child.node {
            // Note: Deprecation warning would be emitted during parsing
            Some(inner.as_ref())
        } else {
            None
        }
    })
}

/// Check if a statement has a `role: label` modifier
fn has_role_label(stmt: &Statement) -> bool {
    let modifiers = match stmt {
        Statement::Shape(s) => &s.modifiers,
        Statement::Layout(l) => &l.modifiers,
        Statement::Group(g) => &g.modifiers,
        _ => return false,
    };

    modifiers.iter().any(|m| {
        matches!(m.node.key.node, StyleKey::Role)
            && matches!(
                &m.node.value.node,
                StyleValue::Keyword(k) if k == "label"
            )
    })
}

/// Extract the first child with `role: label` modifier from a list of children.
/// Returns the statement if found.
fn extract_role_label_statement(children: &[Spanned<Statement>]) -> Option<&Statement> {
    children.iter().find_map(|child| {
        if has_role_label(&child.node) {
            Some(&child.node)
        } else {
            None
        }
    })
}

/// Extract the gap value from modifiers (can be negative for overlap)
fn extract_gap(modifiers: &[Spanned<StyleModifier>]) -> Option<f64> {
    modifiers.iter().find_map(|m| {
        if matches!(m.node.key.node, StyleKey::Gap) {
            match &m.node.value.node {
                StyleValue::Number { value, .. } => Some(*value),
                _ => None,
            }
        } else {
            None
        }
    })
}

fn layout_container(layout: &LayoutDecl, position: Point, config: &LayoutConfig) -> ElementLayout {
    // Check for a child with [role: label] modifier (preferred)
    // Falls back to Label statement (deprecated) if not found
    let role_label_stmt = extract_role_label_statement(&layout.children);
    let label_stmt = role_label_stmt.or_else(|| extract_label_statement(&layout.children));

    // Extract gap modifier from layout modifiers (can be negative for overlap)
    let gap = extract_gap(&layout.modifiers);

    let (mut children, bounds) = match layout.layout_type.node {
        LayoutType::Row => layout_row(&layout.children, position, config, gap),
        LayoutType::Column => layout_column(&layout.children, position, config, gap),
        LayoutType::Grid => layout_grid(&layout.children, position, config),
        LayoutType::Stack => layout_stack(&layout.children, position, config),
    };

    let styles = ResolvedStyles::from_modifiers(&layout.modifiers);

    // Determine the label: role: label or Label statement takes precedence, otherwise use modifier
    let label = if let Some(inner_stmt) = label_stmt {
        // Layout the label element and position it above the container (centered)
        let label_element = layout_statement(inner_stmt, Point::new(0.0, 0.0), config);

        // Position the label centered above the container
        let label_x = bounds.x + (bounds.width - label_element.bounds.width) / 2.0;
        let label_y = bounds.y - label_element.bounds.height - 5.0;

        // Create a positioned copy of the label element and recursively offset all children
        let mut positioned_label = label_element.clone();
        let dx = label_x - positioned_label.bounds.x;
        let dy = label_y - positioned_label.bounds.y;
        offset_element(&mut positioned_label, dx, dy);

        children.push(positioned_label);

        // No simple text label since we're using an element label
        None
    } else {
        // Fall back to the old modifier-based label
        extract_label(&layout.modifiers).map(|text| LabelLayout {
            text,
            position: Point::new(bounds.x + bounds.width / 2.0, bounds.y - 5.0),
            anchor: TextAnchor::Middle,
        })
    };

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
    // Check for a child with [role: label] modifier (preferred)
    // Falls back to Label statement (deprecated) if not found
    let role_label_stmt = extract_role_label_statement(&group.children);
    let label_stmt = role_label_stmt.or_else(|| extract_label_statement(&group.children));

    // Groups default to column layout (no gap override)
    // Filter out Label statements from layout children
    let (mut children, bounds) = layout_column(&group.children, position, config, None);

    let styles = ResolvedStyles::from_modifiers(&group.modifiers);

    // Determine the label: Label statement takes precedence, otherwise use modifier
    let label = if let Some(inner_stmt) = label_stmt {
        // Layout the label element and position it on the left side of the group
        let label_element = layout_statement(inner_stmt, Point::new(0.0, 0.0), config);

        // Position the label on the left side of the group, vertically centered
        // The label is positioned to the left of the group bounds
        let label_x = bounds.x - label_element.bounds.width - 10.0;
        let label_y = bounds.y + (bounds.height - label_element.bounds.height) / 2.0;

        // Create a positioned copy of the label element and recursively offset all children
        let mut positioned_label = label_element.clone();
        let dx = label_x - positioned_label.bounds.x;
        let dy = label_y - positioned_label.bounds.y;
        offset_element(&mut positioned_label, dx, dy);

        children.push(positioned_label);

        // No simple text label since we're using an element label
        None
    } else {
        // Fall back to the old modifier-based label
        extract_label(&group.modifiers).map(|text| LabelLayout {
            text,
            position: Point::new(bounds.x - 10.0, bounds.y + bounds.height / 2.0),
            anchor: TextAnchor::End,
        })
    };

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
    gap_override: Option<f64>,
) -> (Vec<ElementLayout>, BoundingBox) {
    let mut layouts = vec![];
    let mut x = position.x + config.container_padding;
    let mut max_height = 0.0f64;

    // Use gap override if provided, otherwise use default element spacing
    let spacing = gap_override.unwrap_or(config.element_spacing);

    for child in children {
        // Skip connections, constraints, alignments, and labels (labels are handled separately by parent)
        // Labels include both Statement::Label and elements with [role: label] modifier
        if matches!(
            child.node,
            Statement::Connection(_) | Statement::Constraint(_) | Statement::Label(_) | Statement::Alignment(_)
        ) || has_role_label(&child.node)
        {
            continue;
        }

        let child_layout = layout_statement(
            &child.node,
            Point::new(x, position.y + config.container_padding),
            config,
        );
        x += child_layout.bounds.width + spacing;
        max_height = max_height.max(child_layout.bounds.height);
        layouts.push(child_layout);
    }

    let total_width = if layouts.is_empty() {
        config.container_padding * 2.0
    } else {
        x - position.x - spacing + config.container_padding
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
    gap_override: Option<f64>,
) -> (Vec<ElementLayout>, BoundingBox) {
    let mut layouts = vec![];
    let mut y = position.y + config.container_padding;
    let mut max_width = 0.0f64;

    // Use gap override if provided, otherwise use default element spacing
    let spacing = gap_override.unwrap_or(config.element_spacing);

    for child in children {
        // Skip connections, constraints, alignments, and labels (labels are handled separately by parent)
        // Labels include both Statement::Label and elements with [role: label] modifier
        if matches!(
            child.node,
            Statement::Connection(_) | Statement::Constraint(_) | Statement::Label(_) | Statement::Alignment(_)
        ) || has_role_label(&child.node)
        {
            continue;
        }

        let child_layout = layout_statement(
            &child.node,
            Point::new(position.x + config.container_padding, y),
            config,
        );
        y += child_layout.bounds.height + spacing;
        max_width = max_width.max(child_layout.bounds.width);
        layouts.push(child_layout);
    }

    let total_width = max_width + 2.0 * config.container_padding;
    let total_height = if layouts.is_empty() {
        config.container_padding * 2.0
    } else {
        y - position.y - spacing + config.container_padding
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
    // Filter out connections, constraints, alignments, and labels (labels are handled separately by parent)
    // Labels include both Statement::Label and elements with [role: label] modifier
    let filtered: Vec<_> = children
        .iter()
        .filter(|c| {
            !matches!(
                c.node,
                Statement::Connection(_) | Statement::Constraint(_) | Statement::Label(_) | Statement::Alignment(_)
            ) && !has_role_label(&c.node)
        })
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
        // Skip connections, constraints, alignments, and labels (labels are handled separately by parent)
        // Labels include both Statement::Label and elements with [role: label] modifier
        if matches!(
            child.node,
            Statement::Connection(_) | Statement::Constraint(_) | Statement::Label(_) | Statement::Alignment(_)
        ) || has_role_label(&child.node)
        {
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
                dependents
                    .entry(anchor.clone())
                    .or_default()
                    .push(subject.clone());
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
            subject.bounds.x = anchor_bounds.x + (anchor_bounds.width - subject.bounds.width) / 2.0;
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

// ============================================================================
// Alignment Resolution
// ============================================================================

/// Resolve alignment constraints after initial layout
pub fn resolve_alignments(result: &mut LayoutResult, doc: &Document) -> Result<(), LayoutError> {
    // Collect all alignment declarations from the document
    let alignments = collect_alignments(&doc.statements);

    for alignment in alignments {
        apply_alignment(result, &alignment)?;
    }

    // Recompute bounds after alignment resolution
    result.compute_bounds();
    Ok(())
}

/// Collect all alignment declarations from statements (including nested)
fn collect_alignments(stmts: &[Spanned<Statement>]) -> Vec<&AlignmentDecl> {
    let mut alignments = vec![];

    for stmt in stmts {
        match &stmt.node {
            Statement::Alignment(a) => alignments.push(a),
            Statement::Layout(l) => {
                alignments.extend(collect_alignments(&l.children));
            }
            Statement::Group(g) => {
                alignments.extend(collect_alignments(&g.children));
            }
            _ => {}
        }
    }

    alignments
}

/// Apply a single alignment constraint
fn apply_alignment(result: &mut LayoutResult, alignment: &AlignmentDecl) -> Result<(), LayoutError> {
    // Validate that all anchors are on the same axis
    if !alignment.is_valid() {
        let edges: Vec<String> = alignment
            .anchors
            .iter()
            .map(|a| format!("{:?}", a.edge.node))
            .collect();
        return Err(LayoutError::IncompatibleEdges {
            edges,
            span: alignment.anchors[0].edge.span.clone(),
        });
    }

    // Resolve the first anchor to get the reference coordinate
    let first_anchor = &alignment.anchors[0];
    let first_elem_name = resolve_element_path(&first_anchor.element.node, result)?;
    let first_elem = result
        .get_element_by_name(&first_elem_name)
        .ok_or_else(|| {
            LayoutError::undefined(&first_elem_name, first_anchor.element.span.clone(), vec![])
        })?;
    let reference_coord = get_edge_coordinate(&first_elem.bounds, &first_anchor.edge.node);

    // Apply alignment to all other anchors
    for anchor in &alignment.anchors[1..] {
        let elem_name = resolve_element_path(&anchor.element.node, result)?;
        let current_coord = {
            let elem = result.get_element_by_name(&elem_name).ok_or_else(|| {
                LayoutError::undefined(&elem_name, anchor.element.span.clone(), vec![])
            })?;
            get_edge_coordinate(&elem.bounds, &anchor.edge.node)
        };

        // Calculate the delta needed to align
        let delta = reference_coord - current_coord;

        // Shift the element (and its children recursively)
        let axis = anchor.edge.node.axis();
        shift_element_by_name(result, &elem_name, delta, axis)?;
    }

    Ok(())
}

/// Resolve an element path to an element name
/// For now, only handles simple (single-segment) paths
/// Full hierarchical path resolution will be implemented in Phase 7
fn resolve_element_path(path: &ElementPath, result: &LayoutResult) -> Result<String, LayoutError> {
    if path.is_simple() {
        // Simple case: just return the leaf name
        Ok(path.leaf().0.clone())
    } else {
        // For now, try the leaf name - full path resolution will be added later
        // This works because elements are indexed by their ID in the LayoutResult
        let leaf_name = path.leaf().0.clone();
        if result.get_element_by_name(&leaf_name).is_some() {
            Ok(leaf_name)
        } else {
            Err(LayoutError::PathNotFound {
                path: path.to_string(),
                span: path.segments.last().map(|s| s.span.clone()).unwrap_or(0..0),
                suggestions: vec![],
            })
        }
    }
}

/// Get the coordinate value for an edge of a bounding box
fn get_edge_coordinate(bounds: &BoundingBox, edge: &Edge) -> f64 {
    match edge {
        Edge::Left => bounds.x,
        Edge::Right => bounds.right(),
        Edge::HorizontalCenter => bounds.x + bounds.width / 2.0,
        Edge::Top => bounds.y,
        Edge::Bottom => bounds.bottom(),
        Edge::VerticalCenter => bounds.y + bounds.height / 2.0,
    }
}

/// Shift an element by name in the layout result
fn shift_element_by_name(
    result: &mut LayoutResult,
    name: &str,
    delta: f64,
    axis: Axis,
) -> Result<(), LayoutError> {
    // We need to find and shift the element in the root_elements tree
    for elem in &mut result.root_elements {
        if shift_element_recursive(elem, name, delta, axis) {
            // Also update the elements HashMap
            if let Some(indexed_elem) = result.elements.get_mut(name) {
                match axis {
                    Axis::Horizontal => {
                        indexed_elem.bounds.x += delta;
                        if let Some(label) = &mut indexed_elem.label {
                            label.position.x += delta;
                        }
                    }
                    Axis::Vertical => {
                        indexed_elem.bounds.y += delta;
                        if let Some(label) = &mut indexed_elem.label {
                            label.position.y += delta;
                        }
                    }
                }
            }
            return Ok(());
        }
    }

    Err(LayoutError::undefined(name, 0..0, vec![]))
}

/// Recursively search for and shift an element by name
/// Returns true if the element was found and shifted
fn shift_element_recursive(
    elem: &mut ElementLayout,
    name: &str,
    delta: f64,
    axis: Axis,
) -> bool {
    if elem.id.as_ref().map(|id| id.0.as_str()) == Some(name) {
        // Found the element - shift it and all its children
        shift_element_and_children(elem, delta, axis);
        return true;
    }

    // Search in children
    for child in &mut elem.children {
        if shift_element_recursive(child, name, delta, axis) {
            return true;
        }
    }

    false
}

/// Shift an element and all its children by delta on the specified axis
fn shift_element_and_children(elem: &mut ElementLayout, delta: f64, axis: Axis) {
    match axis {
        Axis::Horizontal => {
            elem.bounds.x += delta;
            if let Some(label) = &mut elem.label {
                label.position.x += delta;
            }
        }
        Axis::Vertical => {
            elem.bounds.y += delta;
            if let Some(label) = &mut elem.label {
                label.position.y += delta;
            }
        }
    }

    // Recursively shift children
    for child in &mut elem.children {
        shift_element_and_children(child, delta, axis);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn test_alignment_horizontal_left() {
        // Test horizontal left edge alignment
        let doc = parse(r#"
            rect a
            rect b
            align a.left = b.left
        "#)
        .unwrap();
        let config = LayoutConfig::default();
        let mut result = compute(&doc, &config).unwrap();

        // Before alignment, elements are stacked vertically with different x positions
        // After alignment, their left edges should be equal
        resolve_alignments(&mut result, &doc).unwrap();

        let a = result.get_element_by_name("a").unwrap();
        let b = result.get_element_by_name("b").unwrap();

        // Left edges should be aligned (same x coordinate)
        assert_eq!(a.bounds.x, b.bounds.x, "Left edges should be aligned");
    }

    #[test]
    fn test_alignment_horizontal_center() {
        // Test horizontal center alignment
        let doc = parse(r#"
            rect a [width: 100]
            rect b [width: 50]
            align a.horizontal_center = b.horizontal_center
        "#)
        .unwrap();
        let config = LayoutConfig::default();
        let mut result = compute(&doc, &config).unwrap();

        resolve_alignments(&mut result, &doc).unwrap();

        let a = result.get_element_by_name("a").unwrap();
        let b = result.get_element_by_name("b").unwrap();

        // Horizontal centers should be aligned
        let a_center = a.bounds.x + a.bounds.width / 2.0;
        let b_center = b.bounds.x + b.bounds.width / 2.0;
        assert!(
            (a_center - b_center).abs() < 0.001,
            "Horizontal centers should be aligned: {} vs {}",
            a_center,
            b_center
        );
    }

    #[test]
    fn test_alignment_vertical_top() {
        // Test vertical top edge alignment
        let doc = parse(r#"
            row {
                rect a
                rect b
            }
            align a.top = b.top
        "#)
        .unwrap();
        let config = LayoutConfig::default();
        let mut result = compute(&doc, &config).unwrap();

        resolve_alignments(&mut result, &doc).unwrap();

        let a = result.get_element_by_name("a").unwrap();
        let b = result.get_element_by_name("b").unwrap();

        // Top edges should be aligned (same y coordinate)
        assert_eq!(a.bounds.y, b.bounds.y, "Top edges should be aligned");
    }

    #[test]
    fn test_alignment_chain() {
        // Test aligning multiple elements in a chain
        let doc = parse(r#"
            rect a
            rect b
            rect c
            align a.left = b.left = c.left
        "#)
        .unwrap();
        let config = LayoutConfig::default();
        let mut result = compute(&doc, &config).unwrap();

        resolve_alignments(&mut result, &doc).unwrap();

        let a = result.get_element_by_name("a").unwrap();
        let b = result.get_element_by_name("b").unwrap();
        let c = result.get_element_by_name("c").unwrap();

        // All left edges should be aligned
        assert_eq!(a.bounds.x, b.bounds.x, "a and b left edges should be aligned");
        assert_eq!(b.bounds.x, c.bounds.x, "b and c left edges should be aligned");
    }

    #[test]
    fn test_alignment_incompatible_edges_error() {
        // Test that mixing horizontal and vertical edges produces an error
        let doc = parse(r#"
            rect a
            rect b
            align a.left = b.top
        "#)
        .unwrap();
        let config = LayoutConfig::default();
        let mut result = compute(&doc, &config).unwrap();

        let err = resolve_alignments(&mut result, &doc);
        assert!(err.is_err(), "Should error on incompatible edge types");
    }

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

    #[test]
    fn test_shape_size_modifier() {
        // Test size modifier creates square/circle with given dimension
        let doc = parse("circle p1 [size: 8]").unwrap();
        let config = LayoutConfig::default();
        let result = compute(&doc, &config).unwrap();

        assert_eq!(result.root_elements.len(), 1);
        assert_eq!(result.root_elements[0].bounds.width, 8.0);
        assert_eq!(result.root_elements[0].bounds.height, 8.0);
    }

    #[test]
    fn test_shape_width_height_modifiers() {
        // Test explicit width and height modifiers
        let doc = parse("rect r1 [width: 50, height: 30]").unwrap();
        let config = LayoutConfig::default();
        let result = compute(&doc, &config).unwrap();

        assert_eq!(result.root_elements.len(), 1);
        assert_eq!(result.root_elements[0].bounds.width, 50.0);
        assert_eq!(result.root_elements[0].bounds.height, 30.0);
    }

    #[test]
    fn test_shape_ellipse_with_dimensions() {
        // Test ellipse with explicit width and height
        let doc = parse("ellipse e1 [width: 100, height: 50]").unwrap();
        let config = LayoutConfig::default();
        let result = compute(&doc, &config).unwrap();

        assert_eq!(result.root_elements.len(), 1);
        assert_eq!(result.root_elements[0].bounds.width, 100.0);
        assert_eq!(result.root_elements[0].bounds.height, 50.0);
    }

    #[test]
    fn test_shape_only_width_uses_default_height() {
        // Test that specifying only width keeps default height
        let doc = parse("rect r1 [width: 100]").unwrap();
        let config = LayoutConfig::default();
        let result = compute(&doc, &config).unwrap();

        assert_eq!(result.root_elements.len(), 1);
        assert_eq!(result.root_elements[0].bounds.width, 100.0);
        // Height should be the default rect height (30.0)
        assert_eq!(result.root_elements[0].bounds.height, 30.0);
    }
}
