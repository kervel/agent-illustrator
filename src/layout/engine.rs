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

    // Recompute bounds and anchors after constraint resolution
    result.compute_bounds();
    recompute_builtin_anchors(result);
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
        Statement::Connection(_) | Statement::Constraint(_) | Statement::Constrain(_) => {
            // These are handled separately
            unreachable!("Connections and constraints should be filtered out")
        }
        Statement::TemplateDecl(_) | Statement::Export(_) | Statement::AnchorDecl(_) => {
            // Template declarations, exports, and anchor declarations are metadata, not layout elements
            // They are handled during template resolution, not layout
            unreachable!("Template declarations, exports, and anchor declarations should be filtered out before layout")
        }
        Statement::TemplateInstance(_) => {
            // Template instances should be expanded before layout
            // After template resolution, instances are replaced with their expanded content
            unreachable!("Template instances should be expanded before layout")
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

    // Get element ID - check ShapeDecl.name first, then PathDecl.name for paths
    let id = shape.name.as_ref().map(|n| n.node.clone()).or_else(|| {
        if let ShapeType::Path(path_decl) = &shape.shape_type.node {
            path_decl.name.as_ref().map(|n| n.node.clone())
        } else {
            None
        }
    });

    let bounds = BoundingBox::new(position.x, position.y, width, height);
    // Feature 009: Compute anchors based on shape type
    let anchors = match &shape.shape_type.node {
        ShapeType::Path(_) => AnchorSet::path_shape(&bounds),
        _ => AnchorSet::simple_shape(&bounds),
    };

    ElementLayout {
        id,
        element_type: ElementType::Shape(shape.shape_type.node.clone()),
        bounds,
        styles,
        children: vec![],
        label,
        anchors,
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

    // Calculate minimum width needed to fit label (if present)
    let label_min_width = extract_label(&shape.modifiers).map(|text| {
        // Approximate: ~8px per character for 14px font, plus 20px padding
        let char_width = 8.0;
        let padding = 20.0;
        text.len() as f64 * char_width + padding
    });

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
        ShapeType::SvgEmbed {
            intrinsic_width,
            intrinsic_height,
            ..
        } => {
            // For embedded SVG, use intrinsic dimensions or fall back to default rect size
            let w = intrinsic_width.unwrap_or(config.default_rect_size.0);
            let h = intrinsic_height.unwrap_or(config.default_rect_size.1);
            (w, h)
        }
        ShapeType::Path(path_decl) => {
            // Compute bounds from path vertices
            compute_path_bounds(path_decl).unwrap_or(config.default_rect_size)
        }
    };

    // Start with specified or default width
    let base_width = width.unwrap_or(default_width);

    // If no explicit width was specified, ensure it's at least large enough for the label
    let final_width = if width.is_none() {
        label_min_width.map_or(base_width, |min| base_width.max(min))
    } else {
        base_width
    };

    let final_height = height.unwrap_or(default_height);

    (final_width, final_height)
}

/// Compute bounding box dimensions from path geometry
///
/// Computes the actual content dimensions based on the path's vertices AND
/// arc/curve extents. Arcs can bulge beyond their endpoints, so we compute
/// the arc's geometric bounds, not just the endpoint positions.
fn compute_path_bounds(path: &PathDecl) -> Option<(f64, f64)> {
    use crate::parser::ast::PathCommand;
    use std::collections::HashMap;

    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut has_points = false;

    // Track current position for arc/curve calculations
    let mut current_x = 0.0_f64;
    let mut current_y = 0.0_f64;

    // Build vertex map for resolving via references in curves
    let mut vertices: HashMap<String, (f64, f64)> = HashMap::new();

    let mut update_bounds = |x: f64, y: f64| {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
        has_points = true;
    };

    for cmd in &path.body.commands {
        match &cmd.node {
            PathCommand::Vertex(v) => {
                let (x, y) = if let Some(pos) = &v.position {
                    (pos.x.unwrap_or(0.0), pos.y.unwrap_or(0.0))
                } else {
                    (0.0, 0.0)
                };
                vertices.insert(v.name.node.as_str().to_string(), (x, y));
                update_bounds(x, y);
                current_x = x;
                current_y = y;
            }
            PathCommand::LineTo(lt) => {
                if let Some(pos) = &lt.position {
                    let x = pos.x.unwrap_or(0.0);
                    let y = pos.y.unwrap_or(0.0);
                    vertices.insert(lt.target.node.as_str().to_string(), (x, y));
                    update_bounds(x, y);
                    current_x = x;
                    current_y = y;
                }
            }
            PathCommand::ArcTo(at) => {
                if let Some(pos) = &at.position {
                    let end_x = pos.x.unwrap_or(0.0);
                    let end_y = pos.y.unwrap_or(0.0);
                    vertices.insert(at.target.node.as_str().to_string(), (end_x, end_y));

                    // Include endpoint
                    update_bounds(end_x, end_y);

                    // Compute arc bulge and include it in bounds
                    let (bulge_x, bulge_y) =
                        compute_arc_bulge_point(current_x, current_y, end_x, end_y, &at.params);
                    update_bounds(bulge_x, bulge_y);

                    current_x = end_x;
                    current_y = end_y;
                }
            }
            PathCommand::CurveTo(ct) => {
                if let Some(pos) = &ct.position {
                    let end_x = pos.x.unwrap_or(0.0);
                    let end_y = pos.y.unwrap_or(0.0);
                    vertices.insert(ct.target.node.as_str().to_string(), (end_x, end_y));
                    update_bounds(end_x, end_y);

                    // Compute curve apex and include in bounds
                    let (apex_x, apex_y) = compute_curve_apex(
                        current_x,
                        current_y,
                        end_x,
                        end_y,
                        ct.via.as_ref().and_then(|v| vertices.get(v.node.as_str())),
                    );
                    update_bounds(apex_x, apex_y);

                    current_x = end_x;
                    current_y = end_y;
                }
            }
            PathCommand::Close | PathCommand::CloseArc(_) => {}
        }
    }

    if has_points && min_x.is_finite() && max_x.is_finite() {
        let width = max_x - min_x;
        let height = max_y - min_y;
        // Ensure minimum size
        Some((width.max(1.0), height.max(1.0)))
    } else {
        None
    }
}

/// Compute the apex point of an arc (where it bulges furthest from the chord)
fn compute_arc_bulge_point(
    start_x: f64,
    start_y: f64,
    end_x: f64,
    end_y: f64,
    params: &crate::parser::ast::ArcParams,
) -> (f64, f64) {
    use crate::parser::ast::{ArcParams, SweepDirection};

    // Chord vector and length
    let dx = end_x - start_x;
    let dy = end_y - start_y;
    let chord_len = (dx * dx + dy * dy).sqrt();

    if chord_len < 0.001 {
        // Degenerate: start and end are the same
        return (start_x, start_y);
    }

    // Midpoint of chord
    let mid_x = (start_x + end_x) / 2.0;
    let mid_y = (start_y + end_y) / 2.0;

    // Perpendicular unit vector (counterclockwise rotation of chord direction)
    let perp_x = -dy / chord_len;
    let perp_y = dx / chord_len;

    // Compute sagitta (bulge height) based on arc parameters
    let (sagitta, clockwise) = match params {
        ArcParams::Radius { radius, sweep } => {
            let r = *radius;
            if chord_len > 2.0 * r {
                // Radius too small - use semicircle
                (chord_len / 2.0, matches!(sweep, SweepDirection::Clockwise))
            } else {
                // sagitta = r - sqrt(r² - (chord/2)²)
                let half_chord = chord_len / 2.0;
                let h = r - (r * r - half_chord * half_chord).sqrt();
                (h, matches!(sweep, SweepDirection::Clockwise))
            }
        }
        ArcParams::Bulge(bulge) => {
            // Bulge = tan(θ/4), sagitta = |bulge| * chord / 2
            let h = bulge.abs() * chord_len / 2.0;
            // Positive bulge = clockwise in our coordinate system
            (h, *bulge > 0.0)
        }
    };

    // Direction of bulge: clockwise means to the "right" of chord direction
    // In standard coordinates, "right" of (dx, dy) is (dy, -dx)
    // Our perpendicular is (-dy, dx) which is "left", so negate for clockwise
    let sign = if clockwise { -1.0 } else { 1.0 };

    (mid_x + sign * perp_x * sagitta, mid_y + sign * perp_y * sagitta)
}

/// Compute the apex point of a quadratic Bezier curve (where it bulges furthest from the chord)
///
/// For a quadratic Bezier with start P0, control P1, end P2:
/// - The apex is at t=0.5: B(0.5) = 0.25*P0 + 0.5*P1 + 0.25*P2
/// - This is the chord midpoint moved halfway toward the control point
///
/// If no control point (via) is specified, uses a default 25% perpendicular offset.
fn compute_curve_apex(
    start_x: f64,
    start_y: f64,
    end_x: f64,
    end_y: f64,
    via: Option<&(f64, f64)>,
) -> (f64, f64) {
    // Chord midpoint
    let mid_x = (start_x + end_x) / 2.0;
    let mid_y = (start_y + end_y) / 2.0;

    if let Some(&(ctrl_x, ctrl_y)) = via {
        // Actual curve apex: midpoint moved halfway toward control point
        // B(0.5) = midpoint + 0.5 * (control - midpoint)
        let apex_x = mid_x + 0.5 * (ctrl_x - mid_x);
        let apex_y = mid_y + 0.5 * (ctrl_y - mid_y);
        (apex_x, apex_y)
    } else {
        // Default: 25% perpendicular offset (matches auto-generated control points)
        let dx = end_x - start_x;
        let dy = end_y - start_y;
        let chord_len = (dx * dx + dy * dy).sqrt();

        if chord_len < 0.001 {
            return (mid_x, mid_y);
        }

        let offset = chord_len * 0.25;
        // Perpendicular direction (counterclockwise rotation)
        let perp_x = -dy / chord_len;
        let perp_y = dx / chord_len;

        // Default apex is at 25% perpendicular offset, and curve reaches halfway there
        // So actual curve apex is at 12.5% perpendicular offset
        (mid_x + perp_x * offset * 0.5, mid_y + perp_y * offset * 0.5)
    }
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

    // Feature 009: Containers get simple shape anchors
    let anchors = AnchorSet::simple_shape(&bounds);

    ElementLayout {
        id: layout.name.as_ref().map(|n| n.node.clone()),
        element_type: ElementType::Layout(layout.layout_type.node),
        bounds,
        styles,
        children,
        label,
        anchors,
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

    // Feature 009: Groups get simple shape anchors, plus custom anchors from template
    let mut anchors = AnchorSet::simple_shape(&bounds);

    // Feature 009: Resolve custom anchor declarations from template expansion
    if !group.anchors.is_empty() {
        resolve_custom_anchors(&group.anchors, &children, &mut anchors);
    }

    ElementLayout {
        id: group.name.as_ref().map(|n| n.node.clone()),
        element_type: ElementType::Group,
        bounds,
        styles,
        children,
        label,
        anchors,
    }
}

/// Resolve custom anchor declarations by looking up element properties in children (Feature 009)
fn resolve_custom_anchors(
    anchor_decls: &[AnchorDecl],
    children: &[ElementLayout],
    anchor_set: &mut AnchorSet,
) {
    // Build a map of child IDs to their bounds for quick lookup
    let child_map: HashMap<&str, &BoundingBox> = children
        .iter()
        .filter_map(|c| c.id.as_ref().map(|id| (id.as_str(), &c.bounds)))
        .collect();

    for decl in anchor_decls {
        // Get the element reference from the position
        let (prop_ref, offset) = match &decl.position {
            AnchorPosition::PropertyRef(pr) => (pr, 0.0),
            AnchorPosition::PropertyRefWithOffset { prop_ref, offset } => (prop_ref, *offset),
        };

        // Get the element name (first segment of the path)
        let element_name = prop_ref
            .element
            .node
            .segments
            .first()
            .map(|s| s.node.as_str())
            .unwrap_or("");

        // Look up the child's bounds
        if let Some(child_bounds) = child_map.get(element_name) {
            // Get the position from the property
            let base_position = match prop_ref.property.node {
                ConstraintProperty::Left => child_bounds.left_center(),
                ConstraintProperty::Right => child_bounds.right_center(),
                ConstraintProperty::Top => child_bounds.top_center(),
                ConstraintProperty::Bottom => child_bounds.bottom_center(),
                ConstraintProperty::CenterX => child_bounds.center(),
                ConstraintProperty::CenterY => child_bounds.center(),
                ConstraintProperty::Center => child_bounds.center(),
                _ => child_bounds.center(),
            };

            // Apply offset (along the axis of the property)
            let position = match prop_ref.property.node {
                ConstraintProperty::Left | ConstraintProperty::Right | ConstraintProperty::CenterX => {
                    Point::new(base_position.x + offset, base_position.y)
                }
                ConstraintProperty::Top | ConstraintProperty::Bottom | ConstraintProperty::CenterY => {
                    Point::new(base_position.x, base_position.y + offset)
                }
                _ => base_position,
            };

            // Determine direction: use explicit if provided, otherwise infer from property
            let direction = if let Some(dir_spec) = &decl.direction {
                match dir_spec {
                    AnchorDirectionSpec::Cardinal(c) => match c {
                        CardinalDirection::Up => AnchorDirection::Up,
                        CardinalDirection::Down => AnchorDirection::Down,
                        CardinalDirection::Left => AnchorDirection::Left,
                        CardinalDirection::Right => AnchorDirection::Right,
                    },
                    AnchorDirectionSpec::Angle(a) => AnchorDirection::Angle(*a),
                }
            } else {
                // Infer from property
                match prop_ref.property.node {
                    ConstraintProperty::Left => AnchorDirection::Left,
                    ConstraintProperty::Right => AnchorDirection::Right,
                    ConstraintProperty::Top => AnchorDirection::Up,
                    ConstraintProperty::Bottom => AnchorDirection::Down,
                    _ => AnchorDirection::Right, // Default
                }
            };

            // Create and add the anchor
            let anchor = Anchor::new(decl.name.node.as_str(), position, direction);
            anchor_set.insert(anchor);
        }
        // If child not found, silently skip (could add warning in future)
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

    // Collect row/col alignment constraints (siblings stay aligned)
    collect_layout_alignment_constraints(&doc.statements, &mut collector);

    // Collect user constraints (constrain statements)
    collect_constrain_statements(&doc.statements, &mut collector);

    // Also collect x/y modifiers from shapes as position constraints
    collect_position_constraints_from_shapes(&doc.statements, &mut collector);

    if collector.constraints.is_empty() {
        return Ok(());
    }

    // Separate constraints into internal (within a group) and external (between groups)
    // Internal constraints: both elements share a common prefix (e.g., p1_head and p1_body)
    // External constraints: elements have different prefixes or are root elements
    let (internal_constraints, external_constraints): (Vec<_>, Vec<_>) =
        collector.constraints.into_iter().partition(|c| is_internal_constraint(c));


    // PASS 1: Solve internal constraints first
    // These position children relative to each other within their groups
    if !internal_constraints.is_empty() {
        let mut internal_solver = ConstraintSolver::new();

        // Add child sizes and positions as variables
        for elem in &result.root_elements {
            add_all_element_suggestions(&mut internal_solver, elem)?;
        }

        for constraint in internal_constraints {
            internal_solver
                .add_constraint(constraint)
                .map_err(LayoutError::solver_error)?;
        }

        let internal_solution = internal_solver.solve().map_err(LayoutError::solver_error)?;

        // Apply internal solution - shift children within their groups
        for (var, value) in &internal_solution.values {
            let current = get_element_property(result, &var.element_id, var.property);
            if let Some(current_value) = current {
                let delta = value - current_value;
                if delta.abs() > 0.001 && matches!(var.property, LayoutProperty::X | LayoutProperty::Y) {
                    let axis = if var.property == LayoutProperty::X {
                        Axis::Horizontal
                    } else {
                        Axis::Vertical
                    };
                    // For internal constraints, shift just the element (not children)
                    // because we're positioning siblings relative to each other
                    shift_single_element_by_name(result, &var.element_id, delta, axis)?;
                }
            }
        }

        // Recompute group bounds after internal constraints
        recompute_group_bounds(result);
    }

    // PASS 2: Solve external constraints
    // These position groups relative to each other
    if !external_constraints.is_empty() {
        // Collect the target (element_id, property) pairs from external constraints
        // We only want to move the specific property that is targeted
        let target_vars: std::collections::HashSet<(String, LayoutProperty)> = external_constraints
            .iter()
            .filter_map(|c| get_constraint_target_var(c))
            .collect();

        // Collect all elements referenced in external constraints
        // We need position variables for ALL of them, not just root elements
        let referenced_elements: std::collections::HashSet<String> = external_constraints
            .iter()
            .flat_map(|c| get_constraint_referenced_elements(c))
            .collect();

        let mut external_solver = ConstraintSolver::new();

        // Add positions for all elements referenced in external constraints
        // For each property: if it's targeted → SUGGESTED (can move), else → FIXED (reference)
        for element_name in &referenced_elements {
            add_element_by_name_with_per_property_strength(&mut external_solver, result, element_name, &target_vars, config.trace)?;
        }

        for constraint in external_constraints {
            external_solver
                .add_constraint(constraint)
                .map_err(LayoutError::solver_error)?;
        }

        let external_solution = external_solver.solve().map_err(LayoutError::solver_error)?;

        // Trace: print all solution values
        if config.trace {
            for (var, value) in &external_solution.values {
                eprintln!("TRACE: solution {} {:?} = {}", var.element_id, var.property, value);
            }
        }

        // Apply external solution - but only to explicitly targeted (element, property) pairs
        // This prevents constraints from affecting properties they don't target
        for (var, value) in &external_solution.values {
            // Only apply to (element, property) pairs that are explicit targets
            // For derived properties (CenterX -> X, CenterY -> Y, Right -> X, Bottom -> Y),
            // check if either the base property OR the derived property is targeted
            let is_targeted = target_vars.contains(&(var.element_id.clone(), var.property))
                || match var.property {
                    LayoutProperty::X => {
                        target_vars.contains(&(var.element_id.clone(), LayoutProperty::CenterX))
                            || target_vars.contains(&(var.element_id.clone(), LayoutProperty::Right))
                    }
                    LayoutProperty::Y => {
                        target_vars.contains(&(var.element_id.clone(), LayoutProperty::CenterY))
                            || target_vars.contains(&(var.element_id.clone(), LayoutProperty::Bottom))
                    }
                    _ => false,
                };

            if config.trace {
                eprintln!("TRACE: {} {:?} is_targeted={}", var.element_id, var.property, is_targeted);
            }

            if !is_targeted {
                continue;
            }

            let current = get_element_property(result, &var.element_id, var.property);
            if let Some(current_value) = current {
                let delta = value - current_value;
                if delta.abs() > 0.001 && matches!(var.property, LayoutProperty::X | LayoutProperty::Y) {
                    let axis = if var.property == LayoutProperty::X {
                        Axis::Horizontal
                    } else {
                        Axis::Vertical
                    };
                    if config.trace {
                        eprintln!("TRACE: shifting {} by {} on {:?}", var.element_id, delta, axis);
                    }
                    shift_element_by_name(result, &var.element_id, delta, axis)?;
                }
            }
        }
    }

    // Recompute bounds and anchors after applying constraints
    result.compute_bounds();
    recompute_builtin_anchors(result);
    recompute_custom_anchors(result, doc);
    Ok(())
}

/// Recompute built-in anchors (top, bottom, left, right, and corners for paths)
/// for all elements after constraint resolution has moved them.
/// This ensures anchors stay in sync with element bounds.
fn recompute_builtin_anchors(result: &mut LayoutResult) {
    // Update anchors in the tree structure
    for elem in &mut result.root_elements {
        recompute_builtin_anchors_recursive(elem);
    }

    // Update anchors in the HashMap (used for connection routing)
    for elem in result.elements.values_mut() {
        elem.anchors.update_builtin_from_bounds(&elem.element_type, &elem.bounds);
    }
}

/// Recursively update built-in anchors for an element and all its children
fn recompute_builtin_anchors_recursive(elem: &mut ElementLayout) {
    elem.anchors.update_builtin_from_bounds(&elem.element_type, &elem.bounds);

    for child in &mut elem.children {
        recompute_builtin_anchors_recursive(child);
    }
}

/// Recompute custom anchors for all groups after constraint resolution
/// This is needed because anchor positions are computed during initial layout,
/// but internal constraints may move children afterward.
fn recompute_custom_anchors(result: &mut LayoutResult, doc: &Document) {
    recompute_anchors_in_statements(&doc.statements, result);
}

fn recompute_anchors_in_statements(stmts: &[Spanned<Statement>], result: &mut LayoutResult) {
    for stmt in stmts {
        match &stmt.node {
            Statement::Group(g) => {
                if !g.anchors.is_empty() {
                    if let Some(group_name) = g.name.as_ref().map(|n| n.node.as_str()) {
                        recompute_group_anchors(result, group_name, &g.anchors);
                    }
                }
                recompute_anchors_in_statements(&g.children, result);
            }
            Statement::Layout(l) => {
                recompute_anchors_in_statements(&l.children, result);
            }
            _ => {}
        }
    }
}

fn recompute_group_anchors(
    result: &mut LayoutResult,
    group_name: &str,
    anchor_decls: &[AnchorDecl],
) {
    // First, collect the children bounds
    let children_bounds: HashMap<String, BoundingBox> = result
        .elements
        .iter()
        .filter(|(id, _)| id.starts_with(&format!("{}_", group_name)))
        .map(|(id, elem)| (id.clone(), elem.bounds))
        .collect();

    // Resolve each anchor declaration
    let mut new_anchors = AnchorSet::default();

    // Keep the built-in anchors
    if let Some(group_elem) = result.elements.get(group_name) {
        new_anchors = AnchorSet::simple_shape(&group_elem.bounds);
    }

    for decl in anchor_decls {
        let (prop_ref, offset) = match &decl.position {
            AnchorPosition::PropertyRef(pr) => (pr, 0.0),
            AnchorPosition::PropertyRefWithOffset { prop_ref, offset } => (prop_ref, *offset),
        };

        // Get the element name (first segment of the path)
        let element_name = prop_ref
            .element
            .node
            .segments
            .first()
            .map(|s| s.node.0.clone())
            .unwrap_or_default();

        if let Some(child_bounds) = children_bounds.get(&element_name) {
            let base_position = match prop_ref.property.node {
                ConstraintProperty::Left => child_bounds.left_center(),
                ConstraintProperty::Right => child_bounds.right_center(),
                ConstraintProperty::Top => child_bounds.top_center(),
                ConstraintProperty::Bottom => child_bounds.bottom_center(),
                ConstraintProperty::CenterX | ConstraintProperty::CenterY | ConstraintProperty::Center => {
                    child_bounds.center()
                }
                _ => child_bounds.center(),
            };

            let position = match prop_ref.property.node {
                ConstraintProperty::Left | ConstraintProperty::Right | ConstraintProperty::CenterX => {
                    Point::new(base_position.x + offset, base_position.y)
                }
                ConstraintProperty::Top | ConstraintProperty::Bottom | ConstraintProperty::CenterY => {
                    Point::new(base_position.x, base_position.y + offset)
                }
                _ => base_position,
            };

            let direction = if let Some(dir_spec) = &decl.direction {
                match dir_spec {
                    AnchorDirectionSpec::Cardinal(c) => match c {
                        CardinalDirection::Up => AnchorDirection::Up,
                        CardinalDirection::Down => AnchorDirection::Down,
                        CardinalDirection::Left => AnchorDirection::Left,
                        CardinalDirection::Right => AnchorDirection::Right,
                    },
                    AnchorDirectionSpec::Angle(a) => AnchorDirection::Angle(*a),
                }
            } else {
                match prop_ref.property.node {
                    ConstraintProperty::Left => AnchorDirection::Left,
                    ConstraintProperty::Right => AnchorDirection::Right,
                    ConstraintProperty::Top => AnchorDirection::Up,
                    ConstraintProperty::Bottom => AnchorDirection::Down,
                    _ => AnchorDirection::Right,
                }
            };

            new_anchors.insert(Anchor::new(decl.name.node.as_str(), position, direction));
        }
    }

    // Update the group's anchors in both the tree and the HashMap
    for elem in &mut result.root_elements {
        update_element_anchors_recursive(elem, group_name, &new_anchors);
    }
    if let Some(elem) = result.elements.get_mut(group_name) {
        elem.anchors = new_anchors;
    }
}

fn update_element_anchors_recursive(elem: &mut ElementLayout, name: &str, anchors: &AnchorSet) {
    if elem.id.as_ref().map(|id| id.0.as_str()) == Some(name) {
        elem.anchors = anchors.clone();
        return;
    }
    for child in &mut elem.children {
        update_element_anchors_recursive(child, name, anchors);
    }
}

/// Check if a constraint is internal (both elements are children of the same group)
/// Internal constraints have element IDs that share a common prefix (e.g., p1_head and p1_body)
fn is_internal_constraint(constraint: &super::solver::LayoutConstraint) -> bool {
    use super::solver::{ConstraintOrigin, LayoutConstraint};

    match constraint {
        LayoutConstraint::Equal { left, right, source, .. } => {
            // Layout container constraints are internal (alignment within row/col)
            if matches!(source.origin, ConstraintOrigin::LayoutContainer) {
                return true;
            }
            // User-defined constraints are internal if both elements share a common prefix
            // (i.e., they are siblings within the same template instance)
            // e.g., "gnd_line1" and "gnd_line2" share prefix "gnd"
            if matches!(source.origin, ConstraintOrigin::UserDefined) {
                return elements_share_parent_prefix(&left.element_id, &right.element_id);
            }
            false
        }
        _ => false,
    }
}

/// Check if two element IDs share a common parent prefix
/// Template children are named like "parent_child", so "gnd_line1" and "gnd_line2" share "gnd"
fn elements_share_parent_prefix(a: &str, b: &str) -> bool {
    // Extract prefix (everything before the FIRST underscore)
    // Template instance prefix is always the first segment before underscore
    // e.g., "q1_gate_ext" has prefix "q1", not "q1_gate"
    fn get_prefix(s: &str) -> Option<&str> {
        s.find('_').map(|idx| &s[..idx])
    }

    match (get_prefix(a), get_prefix(b)) {
        (Some(prefix_a), Some(prefix_b)) => prefix_a == prefix_b,
        _ => false,
    }
}

/// Extract the target (element_id, property) from a constraint
/// For Equal constraints, we extract the left-hand side variable
/// For Midpoint constraints, we extract the target variable
fn get_constraint_target_var(
    constraint: &super::solver::LayoutConstraint,
) -> Option<(String, super::solver::LayoutProperty)> {
    use super::solver::LayoutConstraint;

    match constraint {
        LayoutConstraint::Equal { left, .. } => {
            Some((left.element_id.clone(), left.property))
        }
        LayoutConstraint::Midpoint { target, .. } => {
            Some((target.element_id.clone(), target.property))
        }
        LayoutConstraint::GreaterOrEqual { variable, .. } => {
            Some((variable.element_id.clone(), variable.property))
        }
        LayoutConstraint::LessOrEqual { variable, .. } => {
            Some((variable.element_id.clone(), variable.property))
        }
        LayoutConstraint::Fixed { variable, .. } => {
            Some((variable.element_id.clone(), variable.property))
        }
        LayoutConstraint::Suggested { variable, .. } => {
            Some((variable.element_id.clone(), variable.property))
        }
    }
}

/// Collect all element IDs referenced in a constraint
/// This includes both left/right sides of Equal constraints, both endpoints of Midpoint, etc.
fn get_constraint_referenced_elements(
    constraint: &super::solver::LayoutConstraint,
) -> Vec<String> {
    use super::solver::LayoutConstraint;

    match constraint {
        LayoutConstraint::Equal { left, right, .. } => {
            vec![left.element_id.clone(), right.element_id.clone()]
        }
        LayoutConstraint::Midpoint { target, a, b, .. } => {
            vec![target.element_id.clone(), a.element_id.clone(), b.element_id.clone()]
        }
        LayoutConstraint::GreaterOrEqual { variable, .. } => {
            vec![variable.element_id.clone()]
        }
        LayoutConstraint::LessOrEqual { variable, .. } => {
            vec![variable.element_id.clone()]
        }
        LayoutConstraint::Fixed { variable, .. } => {
            vec![variable.element_id.clone()]
        }
        LayoutConstraint::Suggested { variable, .. } => {
            vec![variable.element_id.clone()]
        }
    }
}

/// Add position and size suggestions for ALL elements (including children)
/// Used for internal constraint solving
fn add_all_element_suggestions(
    solver: &mut super::solver::ConstraintSolver,
    elem: &ElementLayout,
) -> Result<(), LayoutError> {
    use super::solver::{ConstraintSource, LayoutConstraint, LayoutVariable};

    if let Some(id) = &elem.id {
        let name = id.0.as_str();

        solver
            .add_constraint(LayoutConstraint::Suggested {
                variable: LayoutVariable::x(name),
                value: elem.bounds.x,
                source: ConstraintSource::layout(0..0, "element x position"),
            })
            .map_err(LayoutError::solver_error)?;
        solver
            .add_constraint(LayoutConstraint::Suggested {
                variable: LayoutVariable::y(name),
                value: elem.bounds.y,
                source: ConstraintSource::layout(0..0, "element y position"),
            })
            .map_err(LayoutError::solver_error)?;
        solver
            .add_constraint(LayoutConstraint::Fixed {
                variable: LayoutVariable::width(name),
                value: elem.bounds.width,
                source: ConstraintSource::intrinsic("fixed width"),
            })
            .map_err(LayoutError::solver_error)?;
        solver
            .add_constraint(LayoutConstraint::Fixed {
                variable: LayoutVariable::height(name),
                value: elem.bounds.height,
                source: ConstraintSource::intrinsic("fixed height"),
            })
            .map_err(LayoutError::solver_error)?;
    }

    for child in &elem.children {
        add_all_element_suggestions(solver, child)?;
    }

    Ok(())
}

/// Shift only a single element (not its children)
/// Used for internal constraints where siblings are positioned relative to each other
fn shift_single_element_by_name(
    result: &mut LayoutResult,
    name: &str,
    delta: f64,
    axis: Axis,
) -> Result<(), LayoutError> {
    // Shift in the tree structure
    for elem in &mut result.root_elements {
        if shift_single_element_recursive(elem, name, delta, axis) {
            break;
        }
    }

    // Update in the HashMap
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

    Ok(())
}

/// Recursively find and shift a single element (without shifting its children)
fn shift_single_element_recursive(
    elem: &mut ElementLayout,
    name: &str,
    delta: f64,
    axis: Axis,
) -> bool {
    if elem.id.as_ref().map(|id| id.0.as_str()) == Some(name) {
        // Shift only this element's bounds, not children
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
        return true;
    }

    for child in &mut elem.children {
        if shift_single_element_recursive(child, name, delta, axis) {
            return true;
        }
    }

    false
}

/// Recompute bounding boxes for all groups based on their children
fn recompute_group_bounds(result: &mut LayoutResult) {
    // First pass: recompute bounds in the tree
    for elem in &mut result.root_elements {
        recompute_element_bounds_recursive(elem);
    }

    // Second pass: collect all updated bounds
    let mut updates: Vec<(String, BoundingBox)> = Vec::new();
    for elem in &result.root_elements {
        collect_bounds_updates(elem, &mut updates);
    }

    // Third pass: apply updates to HashMap
    for (id, bounds) in updates {
        if let Some(indexed) = result.elements.get_mut(&id) {
            indexed.bounds = bounds;
        }
    }
}

fn recompute_element_bounds_recursive(elem: &mut ElementLayout) {
    // First, recurse into children
    for child in &mut elem.children {
        recompute_element_bounds_recursive(child);
    }

    // If this element has children, recompute its bounds from children
    if !elem.children.is_empty() {
        let mut bounds = elem.children[0].bounds;
        for child in &elem.children[1..] {
            bounds = bounds.union(&child.bounds);
        }
        elem.bounds = bounds;
    }
}

fn collect_bounds_updates(elem: &ElementLayout, updates: &mut Vec<(String, BoundingBox)>) {
    if let Some(id) = &elem.id {
        updates.push((id.0.clone(), elem.bounds));
    }
    for child in &elem.children {
        collect_bounds_updates(child, updates);
    }
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

/// Collect only alignment constraints from row/col layouts
///
/// This ensures siblings in a row stay at the same y position,
/// and siblings in a col stay at the same x position.
/// We don't collect positioning constraints here because the procedural
/// layout already computes those correctly.
fn collect_layout_alignment_constraints(
    stmts: &[Spanned<Statement>],
    collector: &mut super::collector::ConstraintCollector,
) {
    use super::solver::{ConstraintOrigin, ConstraintSource, LayoutConstraint, LayoutVariable};
    use crate::parser::ast::LayoutType;

    for stmt in stmts {
        match &stmt.node {
            Statement::Layout(l) => {
                // Collect child IDs
                let child_ids: Vec<String> = l
                    .children
                    .iter()
                    .filter_map(|child| {
                        match &child.node {
                            Statement::Shape(s) => s.name.as_ref().map(|n| n.node.0.clone()),
                            Statement::Layout(inner_l) => {
                                inner_l.name.as_ref().map(|n| n.node.0.clone())
                            }
                            Statement::Group(g) => g.name.as_ref().map(|n| n.node.0.clone()),
                            _ => None,
                        }
                    })
                    .collect();

                // Extract gap from modifiers
                let gap = crate::layout::collector::extract_number_modifier(&l.modifiers, "gap")
                    .unwrap_or(20.0); // Default gap

                if child_ids.len() > 1 {
                    match l.layout_type.node {
                        LayoutType::Row => {
                            // Row: align all children vertically (same y)
                            // AND maintain horizontal spacing (child[i].x = child[i-1].right + gap)
                            for i in 1..child_ids.len() {
                                // Vertical alignment
                                collector.constraints.push(LayoutConstraint::Equal {
                                    left: LayoutVariable::y(&child_ids[i]),
                                    right: LayoutVariable::y(&child_ids[0]),
                                    offset: 0.0,
                                    source: ConstraintSource {
                                        span: stmt.span.clone(),
                                        description: format!(
                                            "row alignment: {}.y = {}.y",
                                            child_ids[i], child_ids[0]
                                        ),
                                        origin: ConstraintOrigin::LayoutContainer,
                                    },
                                });

                                // Horizontal positioning: child[i].x = child[i-1].right + gap
                                // This is expressed as: child[i].x = child[i-1].x + child[i-1].width + gap
                                // But since we're using Equal which takes two variables with an offset,
                                // and we can't express width directly, we use Right property:
                                // child[i].x - child[i-1].right = gap
                                // Which is: child[i].x = child[i-1].right + gap
                                collector.constraints.push(LayoutConstraint::Equal {
                                    left: LayoutVariable::x(&child_ids[i]),
                                    right: LayoutVariable::new(
                                        &child_ids[i - 1],
                                        super::solver::LayoutProperty::Right,
                                    ),
                                    offset: gap,
                                    source: ConstraintSource {
                                        span: stmt.span.clone(),
                                        description: format!(
                                            "row spacing: {}.x = {}.right + {}",
                                            child_ids[i], child_ids[i - 1], gap
                                        ),
                                        origin: ConstraintOrigin::LayoutContainer,
                                    },
                                });
                            }
                        }
                        LayoutType::Column => {
                            // Column: align all children horizontally (same x)
                            // AND maintain vertical spacing (child[i].y = child[i-1].bottom + gap)
                            for i in 1..child_ids.len() {
                                // Horizontal alignment
                                collector.constraints.push(LayoutConstraint::Equal {
                                    left: LayoutVariable::x(&child_ids[i]),
                                    right: LayoutVariable::x(&child_ids[0]),
                                    offset: 0.0,
                                    source: ConstraintSource {
                                        span: stmt.span.clone(),
                                        description: format!(
                                            "col alignment: {}.x = {}.x",
                                            child_ids[i], child_ids[0]
                                        ),
                                        origin: ConstraintOrigin::LayoutContainer,
                                    },
                                });

                                // Vertical positioning: child[i].y = child[i-1].bottom + gap
                                collector.constraints.push(LayoutConstraint::Equal {
                                    left: LayoutVariable::y(&child_ids[i]),
                                    right: LayoutVariable::new(
                                        &child_ids[i - 1],
                                        super::solver::LayoutProperty::Bottom,
                                    ),
                                    offset: gap,
                                    source: ConstraintSource {
                                        span: stmt.span.clone(),
                                        description: format!(
                                            "col spacing: {}.y = {}.bottom + {}",
                                            child_ids[i], child_ids[i - 1], gap
                                        ),
                                        origin: ConstraintOrigin::LayoutContainer,
                                    },
                                });
                            }
                        }
                        LayoutType::Stack => {
                            // Stack: align all children (same x and y)
                            for i in 1..child_ids.len() {
                                collector.constraints.push(LayoutConstraint::Equal {
                                    left: LayoutVariable::x(&child_ids[i]),
                                    right: LayoutVariable::x(&child_ids[0]),
                                    offset: 0.0,
                                    source: ConstraintSource {
                                        span: stmt.span.clone(),
                                        description: format!(
                                            "stack alignment: {}.x = {}.x",
                                            child_ids[i], child_ids[0]
                                        ),
                                        origin: ConstraintOrigin::LayoutContainer,
                                    },
                                });
                                collector.constraints.push(LayoutConstraint::Equal {
                                    left: LayoutVariable::y(&child_ids[i]),
                                    right: LayoutVariable::y(&child_ids[0]),
                                    offset: 0.0,
                                    source: ConstraintSource {
                                        span: stmt.span.clone(),
                                        description: format!(
                                            "stack alignment: {}.y = {}.y",
                                            child_ids[i], child_ids[0]
                                        ),
                                        origin: ConstraintOrigin::LayoutContainer,
                                    },
                                });
                            }
                        }
                        LayoutType::Grid => {
                            // Grid alignment is more complex - skip for now
                            // Grid cells are aligned within their grid structure
                        }
                    }
                }

                // Recurse into children
                collect_layout_alignment_constraints(&l.children, collector);
            }
            Statement::Group(g) => {
                collect_layout_alignment_constraints(&g.children, collector);
            }
            _ => {}
        }
    }
}

/// Collect x/y modifiers from shapes and convert them to position constraints
///
/// This allows `rect box [x: 100, y: 50]` to override row/col layout positions.
/// The constraints are REQUIRED strength, so they take precedence over
/// the STRONG suggestions from the procedural layout engine.
fn collect_position_constraints_from_shapes(
    stmts: &[Spanned<Statement>],
    collector: &mut super::collector::ConstraintCollector,
) {
    use super::solver::{ConstraintOrigin, ConstraintSource, LayoutConstraint, LayoutVariable};
    use crate::parser::ast::{StyleKey, StyleValue};

    for stmt in stmts {
        match &stmt.node {
            Statement::Shape(s) => {
                if let Some(name) = &s.name {
                    let id = &name.node.0;

                    // Check for x modifier
                    for modifier in &s.modifiers {
                        if matches!(modifier.node.key.node, StyleKey::X) {
                            if let StyleValue::Number { value, .. } = &modifier.node.value.node {
                                collector.constraints.push(LayoutConstraint::Fixed {
                                    variable: LayoutVariable::x(id),
                                    value: *value,
                                    source: ConstraintSource {
                                        span: modifier.span.clone(),
                                        description: format!("{}.x = {}", id, value),
                                        origin: ConstraintOrigin::UserDefined,
                                    },
                                });
                            }
                        }
                        if matches!(modifier.node.key.node, StyleKey::Y) {
                            if let StyleValue::Number { value, .. } = &modifier.node.value.node {
                                collector.constraints.push(LayoutConstraint::Fixed {
                                    variable: LayoutVariable::y(id),
                                    value: *value,
                                    source: ConstraintSource {
                                        span: modifier.span.clone(),
                                        description: format!("{}.y = {}", id, value),
                                        origin: ConstraintOrigin::UserDefined,
                                    },
                                });
                            }
                        }
                    }
                }
            }
            Statement::Layout(l) => {
                collect_position_constraints_from_shapes(&l.children, collector);
            }
            Statement::Group(g) => {
                collect_position_constraints_from_shapes(&g.children, collector);
            }
            _ => {}
        }
    }
}

/// Add position and size for a specific element by name, with per-property targeting
/// For each axis (X/Y), if any property on that axis is targeted → SUGGESTED (can move)
/// Otherwise → FIXED (used as reference value)
fn add_element_by_name_with_per_property_strength(
    solver: &mut super::solver::ConstraintSolver,
    result: &LayoutResult,
    element_name: &str,
    target_vars: &std::collections::HashSet<(String, super::solver::LayoutProperty)>,
    trace: bool,
) -> Result<(), LayoutError> {
    use super::solver::{ConstraintSource, LayoutConstraint, LayoutProperty, LayoutVariable};

    if let Some(elem) = result.get_element_by_name(element_name) {
        // Check if X axis is targeted (X, CenterX, Right, or Left all map to X)
        let x_is_targeted = target_vars.contains(&(element_name.to_string(), LayoutProperty::X))
            || target_vars.contains(&(element_name.to_string(), LayoutProperty::CenterX))
            || target_vars.contains(&(element_name.to_string(), LayoutProperty::Right));

        // Check if Y axis is targeted (Y, CenterY, Bottom, or Top all map to Y)
        let y_is_targeted = target_vars.contains(&(element_name.to_string(), LayoutProperty::Y))
            || target_vars.contains(&(element_name.to_string(), LayoutProperty::CenterY))
            || target_vars.contains(&(element_name.to_string(), LayoutProperty::Bottom));

        if trace {
            eprintln!(
                "TRACE: adding {} x_targeted={} y_targeted={} at ({}, {})",
                element_name, x_is_targeted, y_is_targeted, elem.bounds.x, elem.bounds.y
            );
        }

        // Add X position - SUGGESTED if targeted, FIXED if reference only
        if x_is_targeted {
            solver
                .add_constraint(LayoutConstraint::Suggested {
                    variable: LayoutVariable::x(element_name),
                    value: elem.bounds.x,
                    source: ConstraintSource::layout(0..0, "target element x"),
                })
                .map_err(LayoutError::solver_error)?;
        } else {
            solver
                .add_constraint(LayoutConstraint::Fixed {
                    variable: LayoutVariable::x(element_name),
                    value: elem.bounds.x,
                    source: ConstraintSource::intrinsic("reference element x"),
                })
                .map_err(LayoutError::solver_error)?;
        }

        // Add Y position - SUGGESTED if targeted, FIXED if reference only
        if y_is_targeted {
            solver
                .add_constraint(LayoutConstraint::Suggested {
                    variable: LayoutVariable::y(element_name),
                    value: elem.bounds.y,
                    source: ConstraintSource::layout(0..0, "target element y"),
                })
                .map_err(LayoutError::solver_error)?;
        } else {
            solver
                .add_constraint(LayoutConstraint::Fixed {
                    variable: LayoutVariable::y(element_name),
                    value: elem.bounds.y,
                    source: ConstraintSource::intrinsic("reference element y"),
                })
                .map_err(LayoutError::solver_error)?;
        }

        // Size is always FIXED
        solver
            .add_constraint(LayoutConstraint::Fixed {
                variable: LayoutVariable::width(element_name),
                value: elem.bounds.width,
                source: ConstraintSource::intrinsic("fixed width"),
            })
            .map_err(LayoutError::solver_error)?;
        solver
            .add_constraint(LayoutConstraint::Fixed {
                variable: LayoutVariable::height(element_name),
                value: elem.bounds.height,
                source: ConstraintSource::intrinsic("fixed height"),
            })
            .map_err(LayoutError::solver_error)?;
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
        LayoutProperty::Right => elem.bounds.x + elem.bounds.width,
        LayoutProperty::Bottom => elem.bounds.y + elem.bounds.height,
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

    #[test]
    fn test_template_internal_constraints_centering() {
        // Regression test: template-internal constraints should keep children aligned
        // when the template instance is moved by external constraints.
        // Before the fix, children would get double-shifted.
        use crate::template::{resolve_templates, TemplateRegistry};

        let doc = parse(r#"
            template "stack3" {
                rect line1 [width: 40, height: 3]
                rect line2 [width: 26, height: 3]
                rect line3 [width: 12, height: 3]
                constrain line2.top = line1.bottom + 4
                constrain line3.top = line2.bottom + 4
                constrain line2.center_x = line1.center_x
                constrain line3.center_x = line1.center_x
            }
            stack3 gnd
            constrain gnd.x = 200
        "#).unwrap();

        let mut registry = TemplateRegistry::new();
        let doc = resolve_templates(doc, &mut registry).expect("template resolution failed");
        let config = LayoutConfig::default();
        let mut result = compute(&doc, &config).unwrap();
        // Apply constraint solver (needed for `constrain` statements to take effect)
        resolve_constrain_statements(&mut result, &doc, &config).unwrap();

        // Find the template instance
        let gnd = result.elements.get("gnd").expect("gnd should exist");
        let line1 = result.elements.get("gnd_line1").expect("gnd_line1 should exist");
        let line2 = result.elements.get("gnd_line2").expect("gnd_line2 should exist");
        let line3 = result.elements.get("gnd_line3").expect("gnd_line3 should exist");

        // All three lines should be centered on the same x coordinate
        let center1 = line1.bounds.x + line1.bounds.width / 2.0;
        let center2 = line2.bounds.x + line2.bounds.width / 2.0;
        let center3 = line3.bounds.x + line3.bounds.width / 2.0;

        // Allow small floating point tolerance
        assert!((center1 - center2).abs() < 1.0,
            "line1 and line2 should be centered: {} vs {}", center1, center2);
        assert!((center1 - center3).abs() < 1.0,
            "line1 and line3 should be centered: {} vs {}", center1, center3);

        // The template should be at x=200 (or near it given constraint solving)
        assert!(gnd.bounds.x >= 195.0 && gnd.bounds.x <= 205.0,
            "gnd should be near x=200, got {}", gnd.bounds.x);
    }
}
