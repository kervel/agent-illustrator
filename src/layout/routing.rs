//! Connection routing between elements

use crate::parser::ast::*;

use super::error::LayoutError;
use super::types::*;

/// Routing mode for connections
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RoutingMode {
    /// Direct straight line from source to target
    Direct,
    /// Orthogonal routing with horizontal/vertical segments (S-shaped paths)
    #[default]
    Orthogonal,
    /// Curved routing using quadratic Bezier (Feature 008)
    Curved,
}

/// Edge of a bounding box for connection attachment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

/// Get the attachment point on a bounding box edge
pub fn attachment_point(bounds: &BoundingBox, edge: Edge) -> Point {
    match edge {
        Edge::Top => Point::new(bounds.x + bounds.width / 2.0, bounds.y),
        Edge::Bottom => Point::new(bounds.x + bounds.width / 2.0, bounds.bottom()),
        Edge::Left => Point::new(bounds.x, bounds.y + bounds.height / 2.0),
        Edge::Right => Point::new(bounds.right(), bounds.y + bounds.height / 2.0),
    }
}

/// Calculate the point on a bounding box boundary in the direction of a target point.
/// This finds where a ray from the center toward the target intersects the boundary.
/// For circles (when width ≈ height and small), uses circular boundary.
/// For rectangles, uses rectangular boundary intersection.
pub fn boundary_point_toward(bounds: &BoundingBox, target: Point) -> Point {
    let center = bounds.center();
    let dx = target.x - center.x;
    let dy = target.y - center.y;

    // Handle degenerate case where target is at center
    if dx.abs() < 0.001 && dy.abs() < 0.001 {
        return center;
    }

    // Check if this is a circle (small, square bounding box)
    let is_circle = (bounds.width - bounds.height).abs() < 1.0 && bounds.width < 20.0;

    if is_circle {
        // For circles, use radius-based calculation
        let radius = bounds.width / 2.0;
        let dist = (dx * dx + dy * dy).sqrt();
        Point::new(center.x + dx / dist * radius, center.y + dy / dist * radius)
    } else {
        // For rectangles, find intersection with boundary
        // Calculate t values for each edge intersection
        let half_w = bounds.width / 2.0;
        let half_h = bounds.height / 2.0;

        // t value where ray intersects each edge (ray = center + t * direction)
        let t_left = if dx < -0.001 { -half_w / dx } else { f64::MAX };
        let t_right = if dx > 0.001 { half_w / dx } else { f64::MAX };
        let t_top = if dy < -0.001 { -half_h / dy } else { f64::MAX };
        let t_bottom = if dy > 0.001 { half_h / dy } else { f64::MAX };

        // Take the smallest positive t (first intersection)
        let t = t_left.min(t_right).min(t_top).min(t_bottom);

        if t == f64::MAX {
            center
        } else {
            Point::new(center.x + dx * t, center.y + dy * t)
        }
    }
}

/// Determine the best edges to connect two bounding boxes
pub fn best_edges(from: &BoundingBox, to: &BoundingBox) -> (Edge, Edge) {
    let dx = to.center().x - from.center().x;
    let dy = to.center().y - from.center().y;

    // Check if boxes overlap horizontally (one is below the other)
    let h_overlap = from.x < to.right() && from.right() > to.x;
    // Check if boxes overlap vertically (one is beside the other)
    let v_overlap = from.y < to.bottom() && from.bottom() > to.y;

    // If primarily vertical movement (dy much larger than dx), prefer vertical connection
    // This handles cases where elements are in a column but don't perfectly overlap
    let primarily_vertical = dy.abs() > dx.abs() * 1.5;

    if (h_overlap && !v_overlap) || primarily_vertical {
        // Stacked vertically - prefer vertical connection
        if dy > 0.0 {
            (Edge::Bottom, Edge::Top)
        } else {
            (Edge::Top, Edge::Bottom)
        }
    } else if v_overlap && !h_overlap {
        // Side by side - prefer horizontal connection
        if dx > 0.0 {
            (Edge::Right, Edge::Left)
        } else {
            (Edge::Left, Edge::Right)
        }
    } else if dx.abs() > dy.abs() {
        // Primarily horizontal
        if dx > 0.0 {
            (Edge::Right, Edge::Left)
        } else {
            (Edge::Left, Edge::Right)
        }
    } else {
        // Primarily vertical
        if dy > 0.0 {
            (Edge::Bottom, Edge::Top)
        } else {
            (Edge::Top, Edge::Bottom)
        }
    }
}

/// Create an orthogonal path between two points
pub fn route_orthogonal(from: Point, to: Point) -> Vec<Point> {
    let dx = (to.x - from.x).abs();
    let dy = (to.y - from.y).abs();

    // If nearly aligned on both axes, draw direct line
    if dx < 15.0 && dy < 15.0 {
        vec![from, to]
    } else if dx < 15.0 {
        // Vertically aligned - direct vertical line
        vec![from, to]
    } else if dy < 15.0 {
        // Horizontally aligned - direct horizontal line
        vec![from, to]
    } else {
        // Need to route around - create S-shaped path
        // Go down first, then across, then down to target
        let mid_y = (from.y + to.y) / 2.0;
        vec![from, Point::new(from.x, mid_y), Point::new(to.x, mid_y), to]
    }
}

/// Minimum length for the final segment to ensure proper marker orientation.
/// Short segments can cause browsers to calculate incorrect tangent directions.
const MIN_FINAL_SEGMENT_LENGTH: f64 = 15.0;

/// Compute control point for quadratic Bezier curve between two points (Feature 008)
///
/// Creates a control point perpendicular to the chord at 25% of the chord length.
/// This produces gentle, visually pleasing curves by default.
fn compute_curve_control_point(start: Point, end: Point) -> Point {
    let chord_x = end.x - start.x;
    let chord_y = end.y - start.y;
    let chord_length = (chord_x * chord_x + chord_y * chord_y).sqrt();

    // Degenerate case: start and end are the same point
    if chord_length < 0.001 {
        return start;
    }

    // Perpendicular vector (counterclockwise rotation)
    let perp_x = -chord_y / chord_length;
    let perp_y = chord_x / chord_length;

    // Midpoint of chord
    let mid_x = (start.x + end.x) / 2.0;
    let mid_y = (start.y + end.y) / 2.0;

    // Offset perpendicular at 25% of chord length
    let offset = chord_length * 0.25;

    Point::new(mid_x + perp_x * offset, mid_y + perp_y * offset)
}

/// Route a connection between two bounding boxes with the specified routing mode
/// Optional via_points are control points for curved routing (Feature 008)
pub fn route_connection(
    from_bounds: &BoundingBox,
    to_bounds: &BoundingBox,
    mode: RoutingMode,
    via_points: &[Point],
) -> Vec<Point> {
    // For curved routing, use quadratic Bezier (Feature 008)
    if mode == RoutingMode::Curved {
        let from_center = from_bounds.center();
        let to_center = to_bounds.center();

        // Calculate boundary points
        let start = boundary_point_toward(from_bounds, to_center);
        let end = boundary_point_toward(to_bounds, from_center);

        if via_points.is_empty() {
            // No via points - auto-generate control point
            let control = compute_curve_control_point(start, end);
            return vec![start, control, end];
        } else if via_points.len() == 1 {
            // Single via point - use as control point
            return vec![start, via_points[0], end];
        } else {
            // Multiple via points - create chained curve segments
            // For n via points, we create path: start, via1, junction1, via2, junction2, ..., end
            // The renderer will interpret this as: Q via1 junction1, T junction2, T junction3, ..., T end
            let mut path = vec![start];

            // Calculate junction points between via points
            // For smooth chaining, junctions are at midpoints between consecutive via points
            for i in 0..via_points.len() {
                path.push(via_points[i]);
                if i < via_points.len() - 1 {
                    // Add junction point (midpoint to next via)
                    let junction = Point::new(
                        (via_points[i].x + via_points[i + 1].x) / 2.0,
                        (via_points[i].y + via_points[i + 1].y) / 2.0,
                    );
                    path.push(junction);
                }
            }
            path.push(end);
            return path;
        }
    }

    // For direct routing, use boundary points that point toward each other's centers
    if mode == RoutingMode::Direct {
        let from_center = from_bounds.center();
        let to_center = to_bounds.center();

        // Calculate boundary points: direction is toward the other shape's center,
        // but the point is on the boundary of this shape
        let start = boundary_point_toward(from_bounds, to_center);
        let end = boundary_point_toward(to_bounds, from_center);

        let dx = to_center.x - from_center.x;
        let dy = to_center.y - from_center.y;

        // For large shapes, consider snapping to axis-aligned lines
        let min_snap_size = 15.0;
        let target_too_small = to_bounds.width < min_snap_size || to_bounds.height < min_snap_size;
        let source_too_small =
            from_bounds.width < min_snap_size || from_bounds.height < min_snap_size;

        // Only snap for large shapes on both ends
        if !target_too_small && !source_too_small {
            if dy.abs() > dx.abs() * 2.0 {
                // Primarily vertical - snap if alignment is close
                if (from_center.x - to_center.x).abs() < to_bounds.width / 2.0 {
                    let mid_x = (from_center.x + to_center.x) / 2.0;
                    return vec![Point::new(mid_x, start.y), Point::new(mid_x, end.y)];
                }
            } else if dx.abs() > dy.abs() * 2.0 {
                // Primarily horizontal - snap if alignment is close
                if (from_center.y - to_center.y).abs() < to_bounds.height / 2.0 {
                    let mid_y = (from_center.y + to_center.y) / 2.0;
                    return vec![Point::new(start.x, mid_y), Point::new(end.x, mid_y)];
                }
            }
        }

        return vec![start, end];
    }

    // Orthogonal routing: use edge-based attachment points
    let (from_edge, to_edge) = best_edges(from_bounds, to_bounds);
    let start = attachment_point(from_bounds, from_edge);
    let end = attachment_point(to_bounds, to_edge);

    // Orthogonal routing: create paths with horizontal/vertical segments only

    // For vertical connections (Bottom to Top), create a proper downward path
    // even if the x coordinates are different
    if from_edge == Edge::Bottom && to_edge == Edge::Top {
        let dx = (end.x - start.x).abs();
        if dx > 15.0 {
            // Elements not aligned - create S-shaped path going down first
            // The horizontal segment should be BETWEEN source and destination:
            // - Below the source (greater than start.y)
            // - Above the destination (less than end.y)
            // Also ensure the final segment is long enough for proper marker orientation
            let vertical_distance = end.y - start.y;
            let mid_y = if vertical_distance > MIN_FINAL_SEGMENT_LENGTH * 2.0 {
                // Enough room: place horizontal segment at midpoint, but ensure
                // at least MIN_FINAL_SEGMENT_LENGTH for the final segment
                let midpoint = (start.y + end.y) / 2.0;
                midpoint
                    .max(start.y + MIN_FINAL_SEGMENT_LENGTH)
                    .min(end.y - MIN_FINAL_SEGMENT_LENGTH)
            } else if vertical_distance > MIN_FINAL_SEGMENT_LENGTH {
                // Limited room: prioritize final segment length for arrow orientation
                end.y - MIN_FINAL_SEGMENT_LENGTH
            } else {
                // Very close: just use midpoint
                (start.y + end.y) / 2.0
            };
            return vec![
                start,
                Point::new(start.x, mid_y),
                Point::new(end.x, mid_y),
                end,
            ];
        }
    }

    // For horizontal connections (Right to Left), similar treatment
    if from_edge == Edge::Right && to_edge == Edge::Left {
        let dy = (end.y - start.y).abs();
        if dy > 15.0 {
            // Ensure the final segment is long enough for proper marker orientation
            // Position the vertical segment so the final horizontal segment is at least MIN_FINAL_SEGMENT_LENGTH
            let mid_x = end.x - MIN_FINAL_SEGMENT_LENGTH;
            return vec![
                start,
                Point::new(mid_x, start.y),
                Point::new(mid_x, end.y),
                end,
            ];
        }
    }

    route_orthogonal(start, end)
}

/// Extract the routing mode from connection modifiers
fn extract_routing_mode(modifiers: &[Spanned<StyleModifier>]) -> RoutingMode {
    for modifier in modifiers {
        if matches!(modifier.node.key.node, StyleKey::Routing) {
            if let StyleValue::Keyword(k) = &modifier.node.value.node {
                match k.as_str() {
                    "direct" => return RoutingMode::Direct,
                    "orthogonal" => return RoutingMode::Orthogonal,
                    "curved" => return RoutingMode::Curved,  // Feature 008
                    _ => {} // Unknown value, use default
                }
            }
        }
    }
    RoutingMode::default() // Orthogonal
}

/// Extract via references from connection modifiers (Feature 008)
/// Returns a list of identifier names for steering vertices
fn extract_via_references(modifiers: &[Spanned<StyleModifier>]) -> Vec<String> {
    let mut via_refs = Vec::new();
    for modifier in modifiers {
        if matches!(modifier.node.key.node, StyleKey::Custom(ref k) if k == "via") {
            match &modifier.node.value.node {
                StyleValue::Identifier(id) => {
                    via_refs.push(id.0.clone());
                }
                StyleValue::Keyword(k) => {
                    // Sometimes identifiers are parsed as keywords
                    via_refs.push(k.clone());
                }
                StyleValue::IdentifierList(ids) => {
                    // Multiple via points: [via: c1, c2, c3]
                    for id in ids {
                        via_refs.push(id.0.clone());
                    }
                }
                _ => {}
            }
        }
    }
    via_refs
}

/// Resolve via references to element center points (Feature 008)
fn resolve_via_points(
    via_refs: &[String],
    result: &LayoutResult,
) -> Result<Vec<Point>, LayoutError> {
    let mut points = Vec::new();
    for name in via_refs {
        if let Some(element) = result.get_element_by_name(name) {
            points.push(element.bounds.center());
        } else {
            return Err(LayoutError::UndefinedIdentifier {
                name: name.clone(),
                span: 0..0, // We don't have span info here
                suggestions: vec![],
            });
        }
    }
    Ok(points)
}

/// Route all connections in a document
pub fn route_connections(result: &mut LayoutResult, doc: &Document) -> Result<(), LayoutError> {
    // Track element IDs that are used as connection labels (to remove them from rendering)
    let mut label_element_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    fn process_statements(
        stmts: &[Spanned<Statement>],
        result: &mut LayoutResult,
        label_element_ids: &mut std::collections::HashSet<String>,
    ) -> Result<(), LayoutError> {
        for stmt in stmts {
            match &stmt.node {
                Statement::Connection(conn) => {
                    let from_element =
                        result
                            .get_element_by_name(&conn.from.node.0)
                            .ok_or_else(|| {
                                LayoutError::undefined(
                                    conn.from.node.0.clone(),
                                    conn.from.span.clone(),
                                    vec![],
                                )
                            })?;
                    let to_element =
                        result.get_element_by_name(&conn.to.node.0).ok_or_else(|| {
                            LayoutError::undefined(
                                conn.to.node.0.clone(),
                                conn.to.span.clone(),
                                vec![],
                            )
                        })?;

                    let routing_mode = extract_routing_mode(&conn.modifiers);

                    // Feature 008: Copy bounds before resolving via points to avoid borrow conflict
                    let from_bounds = from_element.bounds;
                    let to_bounds = to_element.bounds;

                    // Feature 008: Resolve via references to control points
                    let via_refs = extract_via_references(&conn.modifiers);
                    let via_points = resolve_via_points(&via_refs, result)?;

                    let path =
                        route_connection(&from_bounds, &to_bounds, routing_mode, &via_points);
                    let styles = ResolvedStyles::from_modifiers(&conn.modifiers);

                    // Extract label and track if it references an element
                    let (label, label_ref_id) =
                        extract_connection_label_with_ref(&conn.modifiers, &path, result);

                    // Mark the referenced element for removal from rendering
                    if let Some(id) = label_ref_id {
                        label_element_ids.insert(id);
                    }

                    result.connections.push(ConnectionLayout {
                        from_id: conn.from.node.clone(),
                        to_id: conn.to.node.clone(),
                        direction: conn.direction,
                        path,
                        styles,
                        label,
                        routing_mode,
                    });
                }
                Statement::Layout(l) => {
                    process_statements(&l.children, result, label_element_ids)?;
                }
                Statement::Group(g) => {
                    process_statements(&g.children, result, label_element_ids)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    process_statements(&doc.statements, result, &mut label_element_ids)?;

    // Remove elements that are used as connection labels from the layout
    for id in &label_element_ids {
        result.remove_element_by_name(id);
    }

    result.compute_bounds();
    Ok(())
}

/// Label position for connection labels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelPosition {
    Left,
    Right,
    Center,
}

/// Extract connection label (wrapper for tests - returns just the label without tracking references)
#[cfg(test)]
fn extract_connection_label(
    modifiers: &[Spanned<StyleModifier>],
    path: &[Point],
    result: &LayoutResult,
) -> Option<LabelLayout> {
    extract_connection_label_with_ref(modifiers, path, result).0
}

/// Extract connection label and return both the label and the referenced element ID (if any)
fn extract_connection_label_with_ref(
    modifiers: &[Spanned<StyleModifier>],
    path: &[Point],
    result: &LayoutResult,
) -> (Option<LabelLayout>, Option<String>) {
    let mut label_ref_id: Option<String> = None;
    let mut label_styles: Option<ResolvedStyles> = None;

    let text = modifiers.iter().find_map(|m| {
        if matches!(m.node.key.node, StyleKey::Label) {
            match &m.node.value.node {
                StyleValue::String(s) => Some(s.clone()),
                // Support identifier references: [label: my_shape]
                StyleValue::Identifier(id) => {
                    // Look up the element by name and extract text content
                    result.get_element_by_name(&id.0).and_then(|element| {
                        // Mark this element ID for removal from rendering
                        label_ref_id = Some(id.0.clone());
                        // Capture styles from the referenced element
                        label_styles = Some(element.styles.clone());
                        if let ElementType::Shape(ShapeType::Text { content }) =
                            &element.element_type
                        {
                            Some(content.clone())
                        } else {
                            // If it's not a text shape, use the identifier as the label text
                            Some(id.0.clone())
                        }
                    })
                }
                _ => None,
            }
        } else {
            None
        }
    });

    let text = match text {
        Some(t) => t,
        None => return (None, label_ref_id),
    };

    // Extract label_position modifier if present
    let label_position = modifiers.iter().find_map(|m| {
        if matches!(m.node.key.node, StyleKey::LabelPosition) {
            match &m.node.value.node {
                StyleValue::Keyword(k) => match k.as_str() {
                    "left" => Some(LabelPosition::Left),
                    "right" => Some(LabelPosition::Right),
                    "center" => Some(LabelPosition::Center),
                    _ => None,
                },
                _ => None,
            }
        } else {
            None
        }
    });

    // Calculate midpoint of path
    let mid_idx = path.len() / 2;
    let (mid_x, mid_y, anchor) = if path.len() >= 2 {
        let start = path[0];
        let end = path[path.len() - 1];
        let p1 = path[mid_idx.saturating_sub(1)];
        let p2 = path[mid_idx.min(path.len() - 1)];
        let base_mid_x = (p1.x + p2.x) / 2.0;
        let base_mid_y = (p1.y + p2.y) / 2.0;

        // Position based on explicit label_position or auto-detect
        match label_position {
            Some(LabelPosition::Right) => (base_mid_x + 10.0, base_mid_y, TextAnchor::Start),
            Some(LabelPosition::Left) => (base_mid_x - 10.0, base_mid_y, TextAnchor::End),
            Some(LabelPosition::Center) => (base_mid_x, base_mid_y, TextAnchor::Middle),
            None => {
                // Auto-detect based on path direction
                let dx = (end.x - start.x).abs();
                let dy = (end.y - start.y).abs();

                if dy > dx {
                    // Vertical path: position label to the right
                    (base_mid_x + 10.0, base_mid_y, TextAnchor::Start)
                } else {
                    // Horizontal path: position label above
                    (base_mid_x, base_mid_y - 10.0, TextAnchor::Middle)
                }
            }
        }
    } else if !path.is_empty() {
        (path[0].x, path[0].y, TextAnchor::Middle)
    } else {
        (0.0, 0.0, TextAnchor::Middle)
    };

    (
        Some(LabelLayout {
            text,
            position: Point::new(mid_x, mid_y),
            anchor,
            styles: label_styles,
        }),
        label_ref_id,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attachment_point_top() {
        let bounds = BoundingBox::new(0.0, 0.0, 100.0, 50.0);
        let point = attachment_point(&bounds, Edge::Top);
        assert_eq!(point.x, 50.0);
        assert_eq!(point.y, 0.0);
    }

    #[test]
    fn test_attachment_point_bottom() {
        let bounds = BoundingBox::new(0.0, 0.0, 100.0, 50.0);
        let point = attachment_point(&bounds, Edge::Bottom);
        assert_eq!(point.x, 50.0);
        assert_eq!(point.y, 50.0);
    }

    #[test]
    fn test_attachment_point_left() {
        let bounds = BoundingBox::new(0.0, 0.0, 100.0, 50.0);
        let point = attachment_point(&bounds, Edge::Left);
        assert_eq!(point.x, 0.0);
        assert_eq!(point.y, 25.0);
    }

    #[test]
    fn test_attachment_point_right() {
        let bounds = BoundingBox::new(0.0, 0.0, 100.0, 50.0);
        let point = attachment_point(&bounds, Edge::Right);
        assert_eq!(point.x, 100.0);
        assert_eq!(point.y, 25.0);
    }

    #[test]
    fn test_best_edges_horizontal() {
        let a = BoundingBox::new(0.0, 0.0, 50.0, 50.0);
        let b = BoundingBox::new(200.0, 0.0, 50.0, 50.0);
        let (from, to) = best_edges(&a, &b);
        assert_eq!(from, Edge::Right);
        assert_eq!(to, Edge::Left);
    }

    #[test]
    fn test_best_edges_vertical() {
        let a = BoundingBox::new(0.0, 0.0, 50.0, 50.0);
        let b = BoundingBox::new(0.0, 200.0, 50.0, 50.0);
        let (from, to) = best_edges(&a, &b);
        assert_eq!(from, Edge::Bottom);
        assert_eq!(to, Edge::Top);
    }

    #[test]
    fn test_route_orthogonal_direct_horizontal() {
        let from = Point::new(0.0, 50.0);
        let to = Point::new(100.0, 50.0);
        let path = route_orthogonal(from, to);
        assert_eq!(path.len(), 2);
    }

    #[test]
    fn test_route_orthogonal_s_shaped() {
        let from = Point::new(0.0, 0.0);
        let to = Point::new(100.0, 100.0);
        let path = route_orthogonal(from, to);
        // S-shape with midpoints: start -> mid1 -> mid2 -> end
        assert_eq!(path.len(), 4);
        // Now goes down first, then across: mid_y = 50
        assert_eq!(path[1].x, 0.0);
        assert_eq!(path[1].y, 50.0);
        assert_eq!(path[2].x, 100.0);
        assert_eq!(path[2].y, 50.0);
    }

    #[test]
    fn test_route_connection_direct_mode() {
        // Two non-aligned bounding boxes
        let from_bounds = BoundingBox::new(0.0, 0.0, 50.0, 50.0);
        let to_bounds = BoundingBox::new(200.0, 100.0, 50.0, 50.0);

        // Direct routing should give exactly 2 points (straight line)
        let path = route_connection(&from_bounds, &to_bounds, RoutingMode::Direct, &[]);
        assert_eq!(
            path.len(),
            2,
            "Direct routing should produce exactly 2 points"
        );

        // Verify start is on from_bounds edge and end is on to_bounds edge
        assert!((path[0].x >= 0.0 && path[0].x <= 50.0) || path[0].x == 50.0);
        assert!((path[1].x >= 200.0 && path[1].x <= 250.0) || path[1].x == 200.0);
    }

    #[test]
    fn test_route_connection_orthogonal_mode() {
        // Two non-aligned bounding boxes that should create S-shape
        let from_bounds = BoundingBox::new(0.0, 0.0, 50.0, 50.0);
        let to_bounds = BoundingBox::new(200.0, 100.0, 50.0, 50.0);

        // Orthogonal routing may produce more than 2 points
        let path = route_connection(&from_bounds, &to_bounds, RoutingMode::Orthogonal, &[]);
        assert!(
            path.len() >= 2,
            "Orthogonal routing should produce at least 2 points"
        );
    }

    #[test]
    fn test_routing_mode_default_is_orthogonal() {
        assert_eq!(RoutingMode::default(), RoutingMode::Orthogonal);
    }

    #[test]
    fn test_direct_routing_snaps_to_vertical_when_within_bounds() {
        // Two boxes where the from box's center x falls within target bounds
        // From box center at x=25, target spans x=5 to x=55
        // Since 25 is within [5, 55], snapping is safe
        let from_bounds = BoundingBox::new(0.0, 0.0, 50.0, 50.0);
        let to_bounds = BoundingBox::new(5.0, 200.0, 50.0, 50.0);

        let path = route_connection(&from_bounds, &to_bounds, RoutingMode::Direct, &[]);

        assert_eq!(path.len(), 2);
        // The end point should be snapped to have the same x as the start
        assert_eq!(
            path[0].x, path[1].x,
            "Vertical line should snap when start.x is within target bounds"
        );
    }

    #[test]
    fn test_direct_routing_snaps_to_horizontal_when_within_bounds() {
        // Two boxes where the from box's center y falls within target bounds
        // From box center at y=25, target spans y=5 to y=55
        // Since 25 is within [5, 55], snapping is safe
        let from_bounds = BoundingBox::new(0.0, 0.0, 50.0, 50.0);
        let to_bounds = BoundingBox::new(200.0, 5.0, 50.0, 50.0);

        let path = route_connection(&from_bounds, &to_bounds, RoutingMode::Direct, &[]);

        assert_eq!(path.len(), 2);
        // The end point should be snapped to have the same y as the start
        assert_eq!(
            path[0].y, path[1].y,
            "Horizontal line should snap when start.y is within target bounds"
        );
    }

    #[test]
    fn test_direct_routing_preserves_diagonal_when_outside_bounds() {
        // Two boxes where start point falls outside target bounds
        // From box center at (25, 25), target at (200, 100) with size 50x50
        // Target spans x=200 to x=250, y=100 to y=150
        // start.x=25 is NOT within [200, 250], start.y=25 is NOT within [100, 150]
        // Snapping would miss the target, so diagonal should be preserved
        let from_bounds = BoundingBox::new(0.0, 0.0, 50.0, 50.0);
        let to_bounds = BoundingBox::new(200.0, 100.0, 50.0, 50.0);

        let path = route_connection(&from_bounds, &to_bounds, RoutingMode::Direct, &[]);

        assert_eq!(path.len(), 2);
        // Both x and y should be different (diagonal line preserved)
        assert_ne!(
            path[0].x, path[1].x,
            "Diagonal line should not snap when outside target bounds"
        );
        assert_ne!(
            path[0].y, path[1].y,
            "Diagonal line should not snap when outside target bounds"
        );
    }

    #[test]
    fn test_direct_routing_no_snap_for_small_shapes() {
        // Small target shape (like a track junction point, size: 6)
        // Should never snap, even if start is within bounds
        let from_bounds = BoundingBox::new(0.0, 0.0, 50.0, 50.0);
        let to_bounds = BoundingBox::new(20.0, 200.0, 6.0, 6.0); // Small 6x6 shape

        let path = route_connection(&from_bounds, &to_bounds, RoutingMode::Direct, &[]);

        assert_eq!(path.len(), 2);
        // Start point at (25, 50), end should be at target center (23, 203)
        // Diagonal should be preserved for small shapes
        assert_ne!(
            path[0].x, path[1].x,
            "Small shapes should never snap - preserve diagonal"
        );
    }

    #[test]
    fn test_direct_routing_snaps_for_large_shapes_only() {
        // Large target shape where snapping makes sense
        // From center at x=25, target spans x=0 to x=100
        // Since 25 is within [0, 100], vertical snap is safe
        let from_bounds = BoundingBox::new(0.0, 0.0, 50.0, 50.0);
        let to_bounds = BoundingBox::new(0.0, 200.0, 100.0, 50.0); // Large 100x50 shape

        let path = route_connection(&from_bounds, &to_bounds, RoutingMode::Direct, &[]);

        assert_eq!(path.len(), 2);
        // Should snap to vertical since start.x (25) is within target x bounds [0, 100]
        assert_eq!(
            path[0].x, path[1].x,
            "Large shapes should snap when start is within bounds"
        );
    }

    #[test]
    fn test_direct_routing_perfect_vertical_unchanged() {
        // Two boxes perfectly vertically aligned
        let from_bounds = BoundingBox::new(0.0, 0.0, 50.0, 50.0);
        let to_bounds = BoundingBox::new(0.0, 200.0, 50.0, 50.0);

        let path = route_connection(&from_bounds, &to_bounds, RoutingMode::Direct, &[]);

        assert_eq!(path.len(), 2);
        assert_eq!(
            path[0].x, path[1].x,
            "Perfectly vertical should stay vertical"
        );
    }

    #[test]
    fn test_direct_routing_perfect_horizontal_unchanged() {
        // Two boxes perfectly horizontally aligned
        let from_bounds = BoundingBox::new(0.0, 0.0, 50.0, 50.0);
        let to_bounds = BoundingBox::new(200.0, 0.0, 50.0, 50.0);

        let path = route_connection(&from_bounds, &to_bounds, RoutingMode::Direct, &[]);

        assert_eq!(path.len(), 2);
        assert_eq!(
            path[0].y, path[1].y,
            "Perfectly horizontal should stay horizontal"
        );
    }

    // Helper to create an empty LayoutResult for testing
    fn empty_result() -> LayoutResult {
        LayoutResult::new()
    }

    // Helper function to create modifiers for testing
    fn make_label_modifiers(label: &str, position: Option<&str>) -> Vec<Spanned<StyleModifier>> {
        let mut modifiers = vec![Spanned::new(
            StyleModifier {
                key: Spanned::new(StyleKey::Label, 0..5),
                value: Spanned::new(StyleValue::String(label.to_string()), 7..7 + label.len()),
            },
            0..7 + label.len(),
        )];

        if let Some(pos) = position {
            modifiers.push(Spanned::new(
                StyleModifier {
                    key: Spanned::new(StyleKey::LabelPosition, 0..14),
                    value: Spanned::new(StyleValue::Keyword(pos.to_string()), 16..16 + pos.len()),
                },
                0..16 + pos.len(),
            ));
        }

        modifiers
    }

    #[test]
    fn test_label_position_right() {
        let path = vec![Point::new(0.0, 0.0), Point::new(0.0, 100.0)];
        let modifiers = make_label_modifiers("Test", Some("right"));

        let label = extract_connection_label(&modifiers, &path, &empty_result()).unwrap();

        // Should be positioned to the right (x + 10)
        assert_eq!(label.position.x, 10.0, "Right position should add 10 to x");
        assert_eq!(label.position.y, 50.0, "Y should be at midpoint");
        assert_eq!(
            label.anchor,
            TextAnchor::Start,
            "Right position should use Start anchor"
        );
    }

    #[test]
    fn test_label_position_left() {
        let path = vec![Point::new(0.0, 0.0), Point::new(0.0, 100.0)];
        let modifiers = make_label_modifiers("Test", Some("left"));

        let label = extract_connection_label(&modifiers, &path, &empty_result()).unwrap();

        // Should be positioned to the left (x - 10)
        assert_eq!(
            label.position.x, -10.0,
            "Left position should subtract 10 from x"
        );
        assert_eq!(label.position.y, 50.0, "Y should be at midpoint");
        assert_eq!(
            label.anchor,
            TextAnchor::End,
            "Left position should use End anchor"
        );
    }

    #[test]
    fn test_label_position_center() {
        let path = vec![Point::new(0.0, 0.0), Point::new(0.0, 100.0)];
        let modifiers = make_label_modifiers("Test", Some("center"));

        let label = extract_connection_label(&modifiers, &path, &empty_result()).unwrap();

        // Should be positioned at center (no offset)
        assert_eq!(
            label.position.x, 0.0,
            "Center position should have no x offset"
        );
        assert_eq!(label.position.y, 50.0, "Y should be at midpoint");
        assert_eq!(
            label.anchor,
            TextAnchor::Middle,
            "Center position should use Middle anchor"
        );
    }

    #[test]
    fn test_label_position_auto_vertical_path() {
        // Vertical path (dy > dx): auto should position to the right
        let path = vec![Point::new(50.0, 0.0), Point::new(50.0, 100.0)];
        let modifiers = make_label_modifiers("Test", None);

        let label = extract_connection_label(&modifiers, &path, &empty_result()).unwrap();

        // Auto-detect for vertical: right side
        assert_eq!(label.position.x, 60.0, "Vertical auto should add 10 to x");
        assert_eq!(
            label.anchor,
            TextAnchor::Start,
            "Vertical auto should use Start anchor"
        );
    }

    #[test]
    fn test_label_position_auto_horizontal_path() {
        // Horizontal path (dx > dy): auto should position above
        let path = vec![Point::new(0.0, 50.0), Point::new(100.0, 50.0)];
        let modifiers = make_label_modifiers("Test", None);

        let label = extract_connection_label(&modifiers, &path, &empty_result()).unwrap();

        // Auto-detect for horizontal: above
        assert_eq!(
            label.position.x, 50.0,
            "Horizontal auto should have x at midpoint"
        );
        assert_eq!(
            label.position.y, 40.0,
            "Horizontal auto should subtract 10 from y"
        );
        assert_eq!(
            label.anchor,
            TextAnchor::Middle,
            "Horizontal auto should use Middle anchor"
        );
    }

    // Feature 008: Curved routing tests
    #[test]
    fn test_route_connection_curved_mode() {
        // Two horizontally separated boxes
        let from_bounds = BoundingBox::new(0.0, 0.0, 50.0, 50.0);
        let to_bounds = BoundingBox::new(150.0, 0.0, 50.0, 50.0);

        let path = route_connection(&from_bounds, &to_bounds, RoutingMode::Curved, &[]);

        // Curved routing returns exactly 3 points: start, control, end
        assert_eq!(path.len(), 3, "Curved routing should return 3 points");

        // Start point should be on right edge of from_bounds
        assert!(
            (path[0].x - 50.0).abs() < 1.0,
            "Start x should be at right edge of from_bounds"
        );

        // End point should be on left edge of to_bounds
        assert!(
            (path[2].x - 150.0).abs() < 1.0,
            "End x should be at left edge of to_bounds"
        );

        // Control point should be between start and end, offset perpendicular
        assert!(
            path[1].x > path[0].x && path[1].x < path[2].x,
            "Control x should be between start and end"
        );
        // For horizontal connection, control y should be offset (perpendicular)
        assert!(
            (path[1].y - 25.0).abs() > 1.0,
            "Control y should be offset from the line"
        );
    }

    #[test]
    fn test_compute_curve_control_point() {
        // Horizontal line
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 0.0);
        let control = compute_curve_control_point(start, end);

        // Control should be at midpoint x
        assert!(
            (control.x - 50.0).abs() < 0.01,
            "Control x should be at midpoint"
        );
        // Control should be offset perpendicular (25% of chord = 25 pixels)
        assert!(
            (control.y - 25.0).abs() < 0.01,
            "Control y should be offset by 25% of chord length"
        );
    }

    #[test]
    fn test_routing_mode_curved_exists() {
        // Verify the Curved variant exists and is distinct
        assert_ne!(RoutingMode::Curved, RoutingMode::Direct);
        assert_ne!(RoutingMode::Curved, RoutingMode::Orthogonal);
    }
}
