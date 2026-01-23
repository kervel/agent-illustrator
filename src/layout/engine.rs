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
            // Skip connections, constraints, constrain, and standalone labels at document root
            Statement::Connection(_)
            | Statement::Constraint(_)
            | Statement::Constrain(_)
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
/// This includes:
/// 1. Relational constraints (right-of, left-of, etc.)
/// 2. Position offsets from x/y modifiers in place statements
pub fn resolve_constraints(result: &mut LayoutResult, doc: &Document) -> Result<(), LayoutError> {
    let graph = ConstraintGraph::from_document(doc);

    // Check for conflicts before applying
    detect_conflicts(doc)?;

    // Get topological order (or error on cycles)
    let order = graph.topological_order()?;

    // Apply relational constraints in order
    for subject_id in order {
        if let Some(constraints) = graph.constraints.get(&subject_id) {
            for (relation, anchor_id) in constraints {
                apply_constraint(result, &subject_id, relation, anchor_id, &graph.config)?;
            }
        }
    }

    // Apply position offsets from place statements
    apply_position_offsets(result, doc)?;

    // Recompute bounds after constraint resolution
    result.compute_bounds();
    Ok(())
}

/// Apply x/y position offsets from place statements
fn apply_position_offsets(result: &mut LayoutResult, doc: &Document) -> Result<(), LayoutError> {
    // Collect all place statements with position modifiers
    let offsets = collect_position_offsets(&doc.statements);

    for (subject_id, x_offset, y_offset) in offsets {
        if x_offset.abs() > 0.0001 {
            shift_element_by_name(result, &subject_id, x_offset, Axis::Horizontal)?;
        }
        if y_offset.abs() > 0.0001 {
            shift_element_by_name(result, &subject_id, y_offset, Axis::Vertical)?;
        }
    }

    Ok(())
}

/// Collect position offsets from place statements
fn collect_position_offsets(stmts: &[Spanned<Statement>]) -> Vec<(String, f64, f64)> {
    use crate::parser::ast::StyleKey;

    let mut offsets = vec![];

    for stmt in stmts {
        match &stmt.node {
            Statement::Constraint(c) => {
                let mut x_offset = 0.0;
                let mut y_offset = 0.0;

                for modifier in &c.modifiers {
                    match &modifier.node.key.node {
                        StyleKey::X => {
                            if let crate::parser::ast::StyleValue::Number { value, .. } =
                                &modifier.node.value.node
                            {
                                x_offset = *value;
                            }
                        }
                        StyleKey::Y => {
                            if let crate::parser::ast::StyleValue::Number { value, .. } =
                                &modifier.node.value.node
                            {
                                y_offset = *value;
                            }
                        }
                        _ => {}
                    }
                }

                if x_offset.abs() > 0.0001 || y_offset.abs() > 0.0001 {
                    offsets.push((c.subject.node.0.clone(), x_offset, y_offset));
                }
            }
            Statement::Layout(l) => {
                offsets.extend(collect_position_offsets(&l.children));
            }
            Statement::Group(g) => {
                offsets.extend(collect_position_offsets(&g.children));
            }
            _ => {}
        }
    }

    offsets
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
        Statement::Connection(_)
        | Statement::Constraint(_)
        | Statement::Constrain(_) => {
            // These are handled separately
            unreachable!("Connections and constraints should be filtered out")
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
            styles: None,
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
            styles: None,
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
            styles: None,
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
        // Skip connections, constraints, and labels (labels are handled separately by parent)
        // Labels include both Statement::Label and elements with [role: label] modifier
        if matches!(
            child.node,
            Statement::Connection(_)
                | Statement::Constraint(_)
                | Statement::Constrain(_)
                | Statement::Label(_)
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
        // Skip connections, constraints, and labels (labels are handled separately by parent)
        // Labels include both Statement::Label and elements with [role: label] modifier
        if matches!(
            child.node,
            Statement::Connection(_)
                | Statement::Constraint(_)
                | Statement::Constrain(_)
                | Statement::Label(_)
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
    // Filter out connections, constraints, and labels (labels are handled separately by parent)
    // Labels include both Statement::Label and elements with [role: label] modifier
    let filtered: Vec<_> = children
        .iter()
        .filter(|c| {
            !matches!(
                c.node,
                Statement::Connection(_)
                    | Statement::Constraint(_)
                    | Statement::Constrain(_)
                    | Statement::Label(_)
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
        // Skip connections, constraints, and labels (labels are handled separately by parent)
        // Labels include both Statement::Label and elements with [role: label] modifier
        if matches!(
            child.node,
            Statement::Connection(_)
                | Statement::Constraint(_)
                | Statement::Constrain(_)
                | Statement::Label(_)
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
                        // Only add relational constraints (not pure offset constraints)
                        if let (Some(relation), Some(anchor)) = (&c.relation, &c.anchor) {
                            constraints
                                .entry(c.subject.node.0.clone())
                                .or_default()
                                .push((relation.node, anchor.node.0.clone()));
                        }
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
                    // Only collect relational constraints for conflict detection
                    if let (Some(relation), Some(anchor)) = (&c.relation, &c.anchor) {
                        map.entry(c.subject.node.0.clone())
                            .or_default()
                            .push((relation.node, anchor.node.0.clone()));
                    }
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
// Element Shifting Helpers
// ============================================================================

/// Shift an element by name in the layout result
fn shift_element_by_name(
    result: &mut LayoutResult,
    name: &str,
    delta: f64,
    axis: Axis,
) -> Result<(), LayoutError> {
    // First, collect all IDs that will be shifted (the element and all its children)
    let mut ids_to_update = vec![];
    for elem in &result.root_elements {
        if collect_element_ids_recursive(elem, name, &mut ids_to_update) {
            break;
        }
    }

    if ids_to_update.is_empty() {
        return Err(LayoutError::undefined(name, 0..0, vec![]));
    }

    // Shift the element in the tree
    for elem in &mut result.root_elements {
        if shift_element_recursive(elem, name, delta, axis) {
            break;
        }
    }

    // Update all affected elements in the HashMap
    for id in ids_to_update {
        if let Some(indexed_elem) = result.elements.get_mut(&id) {
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
    }

    Ok(())
}

/// Recursively collect all element IDs starting from an element with the given name
/// Returns true if the element was found
fn collect_element_ids_recursive(elem: &ElementLayout, name: &str, ids: &mut Vec<String>) -> bool {
    if elem.id.as_ref().map(|id| id.0.as_str()) == Some(name) {
        // Found the element - collect its ID and all children's IDs
        collect_all_ids(elem, ids);
        return true;
    }

    // Search in children
    for child in &elem.children {
        if collect_element_ids_recursive(child, name, ids) {
            return true;
        }
    }

    false
}

/// Collect all IDs from an element and its children
fn collect_all_ids(elem: &ElementLayout, ids: &mut Vec<String>) {
    if let Some(id) = &elem.id {
        ids.push(id.0.clone());
    }
    for child in &elem.children {
        collect_all_ids(child, ids);
    }
}

/// Recursively search for and shift an element by name
/// Returns true if the element was found and shifted
fn shift_element_recursive(elem: &mut ElementLayout, name: &str, delta: f64, axis: Axis) -> bool {
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

// ============================================================================
// Constrain Statement Resolution (Feature 005)
// ============================================================================

/// Resolve `constrain` statements after initial layout
///
/// This function collects constraints from the document, uses the constraint solver
/// to resolve them, and applies the resulting adjustments to the layout.
pub fn resolve_constrain_statements(
    result: &mut LayoutResult,
    doc: &Document,
    config: &LayoutConfig,
) -> Result<(), LayoutError> {
    use super::collector::ConstraintCollector;
    use super::solver::{ConstraintSolver, LayoutProperty};

    // Collect constraints from the document
    let mut collector = ConstraintCollector::new(config.clone());

    // Only collect user constraints (constrain statements), not intrinsics or layout
    // since those are handled by the procedural layout engine
    collect_constrain_statements(&doc.statements, &mut collector);

    if collector.constraints.is_empty() {
        return Ok(());
    }

    // Create solver and add constraints
    let mut solver = ConstraintSolver::new();

    // First, add current positions as suggested values to anchor the system
    for elem in &result.root_elements {
        add_element_positions_as_suggestions(&mut solver, elem, result)?;
    }

    // Then add the user constraints
    for constraint in collector.constraints {
        solver
            .add_constraint(constraint)
            .map_err(LayoutError::solver_error)?;
    }

    // Solve the constraint system
    let solution = solver.solve().map_err(LayoutError::solver_error)?;

    // Apply the solution to the layout
    for (var, value) in &solution.values {
        // Find the element and update its position
        let current = get_element_property(result, &var.element_id, var.property);
        if let Some(current_value) = current {
            let delta = value - current_value;
            if delta.abs() > 0.001 {
                let axis = match var.property {
                    LayoutProperty::X | LayoutProperty::Width | LayoutProperty::CenterX => {
                        Axis::Horizontal
                    }
                    LayoutProperty::Y | LayoutProperty::Height | LayoutProperty::CenterY => {
                        Axis::Vertical
                    }
                };
                // Only shift for X/Y position changes (not width/height or derived properties)
                // Note: CenterX/CenterY constraints are expressed as X+Width/2 or Y+Height/2,
                // so the solver returns X/Y values, not CenterX/CenterY directly
                if matches!(var.property, LayoutProperty::X | LayoutProperty::Y) {
                    shift_element_by_name(result, &var.element_id, delta, axis)?;
                }
            }
        }
    }

    // Recompute bounds after applying constraints
    result.compute_bounds();
    Ok(())
}

/// Collect only constrain statements (not intrinsics or layout constraints)
fn collect_constrain_statements(
    stmts: &[Spanned<Statement>],
    collector: &mut super::collector::ConstraintCollector,
) {
    for stmt in stmts {
        match &stmt.node {
            Statement::Constrain(c) => {
                collector.collect_constrain_expr(&c.expr, &stmt.span);
            }
            Statement::Layout(l) => {
                collect_constrain_statements(&l.children, collector);
            }
            Statement::Group(g) => {
                collect_constrain_statements(&g.children, collector);
            }
            _ => {}
        }
    }
}

/// Add current element positions as solver suggestions
fn add_element_positions_as_suggestions(
    solver: &mut super::solver::ConstraintSolver,
    elem: &ElementLayout,
    result: &LayoutResult,
) -> Result<(), LayoutError> {
    use super::solver::LayoutVariable;

    if let Some(id) = &elem.id {
        let name = id.0.as_str();

        // Suggest current position values
        solver
            .suggest_value(&LayoutVariable::x(name), elem.bounds.x)
            .map_err(LayoutError::solver_error)?;
        solver
            .suggest_value(&LayoutVariable::y(name), elem.bounds.y)
            .map_err(LayoutError::solver_error)?;
        solver
            .suggest_value(&LayoutVariable::width(name), elem.bounds.width)
            .map_err(LayoutError::solver_error)?;
        solver
            .suggest_value(&LayoutVariable::height(name), elem.bounds.height)
            .map_err(LayoutError::solver_error)?;
    }

    // Recurse into children
    for child in &elem.children {
        add_element_positions_as_suggestions(solver, child, result)?;
    }

    Ok(())
}

/// Get current property value for an element
fn get_element_property(
    result: &LayoutResult,
    element_id: &str,
    property: super::solver::LayoutProperty,
) -> Option<f64> {
    use super::solver::LayoutProperty;

    let elem = result.get_element_by_name(element_id)?;
    Some(match property {
        LayoutProperty::X => elem.bounds.x,
        LayoutProperty::Y => elem.bounds.y,
        LayoutProperty::Width => elem.bounds.width,
        LayoutProperty::Height => elem.bounds.height,
        // Derived properties
        LayoutProperty::CenterX => elem.bounds.x + elem.bounds.width / 2.0,
        LayoutProperty::CenterY => elem.bounds.y + elem.bounds.height / 2.0,
    })
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
