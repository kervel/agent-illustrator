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

/// Route a connection between two bounding boxes with the specified routing mode
pub fn route_connection(
    from_bounds: &BoundingBox,
    to_bounds: &BoundingBox,
    mode: RoutingMode,
) -> Vec<Point> {
    let (from_edge, to_edge) = best_edges(from_bounds, to_bounds);
    let start = attachment_point(from_bounds, from_edge);
    let end = attachment_point(to_bounds, to_edge);

    // For direct routing, return a straight line with smart snap-to-axis logic
    if mode == RoutingMode::Direct {
        let dx = end.x - start.x;
        let dy = end.y - start.y;

        // Don't snap for small target shapes (e.g., track junction points)
        // Small shapes need precise diagonal connections preserved
        let min_snap_size = 15.0;
        let target_too_small =
            to_bounds.width < min_snap_size || to_bounds.height < min_snap_size;

        let snapped_end = if target_too_small {
            // No snapping for small shapes - preserve diagonal lines
            end
        } else if dy.abs() > dx.abs() {
            // Primarily vertical - consider snapping to vertical (keeping start.x)
            // Only snap if start.x falls within target bounds horizontally
            if start.x >= to_bounds.x && start.x <= to_bounds.right() {
                Point::new(start.x, end.y)
            } else {
                end
            }
        } else if dx.abs() > dy.abs() {
            // Primarily horizontal - consider snapping to horizontal (keeping start.y)
            // Only snap if start.y falls within target bounds vertically
            if start.y >= to_bounds.y && start.y <= to_bounds.bottom() {
                Point::new(end.x, start.y)
            } else {
                end
            }
        } else {
            end
        };

        return vec![start, snapped_end];
    }

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
                    _ => {} // Unknown value, use default
                }
            }
        }
    }
    RoutingMode::default() // Orthogonal
}

/// Route all connections in a document
pub fn route_connections(result: &mut LayoutResult, doc: &Document) -> Result<(), LayoutError> {
    fn process_statements(
        stmts: &[Spanned<Statement>],
        result: &mut LayoutResult,
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
                    let path =
                        route_connection(&from_element.bounds, &to_element.bounds, routing_mode);
                    let styles = ResolvedStyles::from_modifiers(&conn.modifiers);
                    let label = extract_connection_label(&conn.modifiers, &path);

                    result.connections.push(ConnectionLayout {
                        from_id: conn.from.node.clone(),
                        to_id: conn.to.node.clone(),
                        direction: conn.direction,
                        path,
                        styles,
                        label,
                    });
                }
                Statement::Layout(l) => {
                    process_statements(&l.children, result)?;
                }
                Statement::Group(g) => {
                    process_statements(&g.children, result)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    process_statements(&doc.statements, result)?;
    result.compute_bounds();
    Ok(())
}

fn extract_connection_label(
    modifiers: &[Spanned<StyleModifier>],
    path: &[Point],
) -> Option<LabelLayout> {
    let text = modifiers.iter().find_map(|m| {
        if matches!(m.node.key.node, StyleKey::Label) {
            match &m.node.value.node {
                StyleValue::String(s) => Some(s.clone()),
                _ => None,
            }
        } else {
            None
        }
    })?;

    // Calculate midpoint of path
    let mid_idx = path.len() / 2;
    let (mid_x, mid_y, anchor) = if path.len() >= 2 {
        let start = path[0];
        let end = path[path.len() - 1];
        let p1 = path[mid_idx.saturating_sub(1)];
        let p2 = path[mid_idx.min(path.len() - 1)];
        let mid_x = (p1.x + p2.x) / 2.0;
        let mid_y = (p1.y + p2.y) / 2.0;

        // Determine if path is primarily vertical or horizontal
        let dx = (end.x - start.x).abs();
        let dy = (end.y - start.y).abs();

        if dy > dx {
            // Vertical path: position label to the right
            (mid_x + 10.0, mid_y, TextAnchor::Start)
        } else {
            // Horizontal path: position label above
            (mid_x, mid_y - 10.0, TextAnchor::Middle)
        }
    } else if !path.is_empty() {
        (path[0].x, path[0].y, TextAnchor::Middle)
    } else {
        (0.0, 0.0, TextAnchor::Middle)
    };

    Some(LabelLayout {
        text,
        position: Point::new(mid_x, mid_y),
        anchor,
    })
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
        let path = route_connection(&from_bounds, &to_bounds, RoutingMode::Direct);
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
        let path = route_connection(&from_bounds, &to_bounds, RoutingMode::Orthogonal);
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

        let path = route_connection(&from_bounds, &to_bounds, RoutingMode::Direct);

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

        let path = route_connection(&from_bounds, &to_bounds, RoutingMode::Direct);

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

        let path = route_connection(&from_bounds, &to_bounds, RoutingMode::Direct);

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

        let path = route_connection(&from_bounds, &to_bounds, RoutingMode::Direct);

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

        let path = route_connection(&from_bounds, &to_bounds, RoutingMode::Direct);

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

        let path = route_connection(&from_bounds, &to_bounds, RoutingMode::Direct);

        assert_eq!(path.len(), 2);
        assert_eq!(path[0].x, path[1].x, "Perfectly vertical should stay vertical");
    }

    #[test]
    fn test_direct_routing_perfect_horizontal_unchanged() {
        // Two boxes perfectly horizontally aligned
        let from_bounds = BoundingBox::new(0.0, 0.0, 50.0, 50.0);
        let to_bounds = BoundingBox::new(200.0, 0.0, 50.0, 50.0);

        let path = route_connection(&from_bounds, &to_bounds, RoutingMode::Direct);

        assert_eq!(path.len(), 2);
        assert_eq!(path[0].y, path[1].y, "Perfectly horizontal should stay horizontal");
    }
}
