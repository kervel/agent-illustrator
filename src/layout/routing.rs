//! Connection routing between elements

use crate::parser::ast::*;

use super::error::LayoutError;
use super::types::*;

fn cardinal_direction_for_anchor(direction: AnchorDirection) -> Point {
    match direction {
        AnchorDirection::Up => Point::new(0.0, -1.0),
        AnchorDirection::Down => Point::new(0.0, 1.0),
        AnchorDirection::Left => Point::new(-1.0, 0.0),
        AnchorDirection::Right => Point::new(1.0, 0.0),
        AnchorDirection::Angle(_) => {
            let vec = direction.to_vector();
            if vec.x.abs() >= vec.y.abs() {
                Point::new(vec.x.signum(), 0.0)
            } else {
                Point::new(0.0, vec.y.signum())
            }
        }
    }
}

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

// ============================================
// Anchor Resolution (Feature 009)
// ============================================

/// Resolve an anchor reference to a resolved anchor with position and direction.
///
/// When anchor is None, returns the center of the element with auto-computed direction
/// toward the target element.
/// When anchor is Some, looks up the anchor in the element's AnchorSet.
pub fn resolve_anchor(
    anchor_ref: &AnchorReference,
    elements: &std::collections::HashMap<String, ElementLayout>,
    target_bounds: Option<&BoundingBox>,
) -> Result<ResolvedAnchor, LayoutError> {
    let element_name = &anchor_ref.element.node.0;
    let element = elements.get(element_name).ok_or_else(|| {
        LayoutError::undefined(
            element_name.clone(),
            anchor_ref.element.span.clone(),
            vec![],
        )
    })?;

    match &anchor_ref.anchor {
        Some(anchor_name) => {
            // Explicit anchor - look up in element's anchor set
            let anchor = element.anchors.get(&anchor_name.node).ok_or_else(|| {
                let valid_anchors: Vec<String> =
                    element.anchors.names().map(String::from).collect();
                LayoutError::invalid_anchor(
                    element_name.clone(),
                    anchor_name.node.clone(),
                    valid_anchors,
                    anchor_name.span.clone(),
                )
            })?;
            Ok(ResolvedAnchor::from_anchor(anchor))
        }
        None => {
            // Auto-detect - return center with direction toward target
            let center = element.bounds.center();
            let direction = if let Some(target) = target_bounds {
                let dx = target.center().x - center.x;
                let dy = target.center().y - center.y;
                let angle = dy.atan2(dx).to_degrees();
                // Normalize to 0-360 range
                let angle = if angle < 0.0 { angle + 360.0 } else { angle };
                AnchorDirection::Angle(angle)
            } else {
                AnchorDirection::Right // Default direction
            };
            Ok(ResolvedAnchor::new(center, direction))
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

fn segment_direction(from: Point, to: Point) -> Point {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    if dx.abs() < 0.001 && dy.abs() < 0.001 {
        return Point::new(0.0, 0.0);
    }
    if dx.abs() >= dy.abs() {
        Point::new(dx.signum(), 0.0)
    } else {
        Point::new(0.0, dy.signum())
    }
}

fn segment_length(from: Point, to: Point) -> f64 {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    (dx * dx + dy * dy).sqrt()
}

fn simplify_path(points: Vec<Point>) -> Vec<Point> {
    if points.len() <= 2 {
        return points;
    }

    let mut cleaned: Vec<Point> = Vec::with_capacity(points.len());
    for point in points {
        if cleaned
            .last()
            .map(|last| (last.x - point.x).abs() < 0.001 && (last.y - point.y).abs() < 0.001)
            .unwrap_or(false)
        {
            continue;
        }
        cleaned.push(point);
    }

    let mut simplified: Vec<Point> = Vec::with_capacity(cleaned.len());
    for point in cleaned {
        while simplified.len() >= 2 {
            let len = simplified.len();
            let a = simplified[len - 2];
            let b = simplified[len - 1];
            let collinear = (a.x - b.x).abs() < 0.001 && (b.x - point.x).abs() < 0.001
                || (a.y - b.y).abs() < 0.001 && (b.y - point.y).abs() < 0.001;
            if collinear {
                simplified.pop();
            } else {
                break;
            }
        }
        simplified.push(point);
    }

    simplified
}

fn is_valid_orthogonal_path(path: &[Point], from_dir: Point, to_dir: Point) -> bool {
    if path.len() < 2 {
        return false;
    }

    let first_dir = segment_direction(path[0], path[1]);
    let last_dir = segment_direction(path[path.len() - 2], path[path.len() - 1]);
    if first_dir.x == 0.0 && first_dir.y == 0.0 {
        return false;
    }
    if last_dir.x == 0.0 && last_dir.y == 0.0 {
        return false;
    }
    if first_dir.x * from_dir.x + first_dir.y * from_dir.y <= 0.0 {
        return false;
    }
    if last_dir.x * to_dir.x + last_dir.y * to_dir.y <= 0.0 {
        return false;
    }

    let mut prev_dir = first_dir;
    for window in path.windows(2).skip(1) {
        let dir = segment_direction(window[0], window[1]);
        if dir.x == 0.0 && dir.y == 0.0 {
            return false;
        }
        if dir.x * prev_dir.x + dir.y * prev_dir.y < -0.5 {
            return false;
        }
        prev_dir = dir;
    }

    true
}

fn orthogonal_with_directions(
    start: Point,
    end: Point,
    from_dir: Point,
    to_dir: Point,
) -> Option<Vec<Point>> {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let from_h = from_dir.x.abs() > 0.5; // from is horizontal
    let to_h = to_dir.x.abs() > 0.5; // to is horizontal

    let mut candidates: Vec<Vec<Point>> = Vec::new();

    // Case 1: Different axes → try L-shape (2 segments)
    if from_h && !to_h {
        // from horizontal, to vertical
        if dx.signum() == from_dir.x && dy.signum() == to_dir.y {
            candidates.push(vec![start, Point::new(end.x, start.y), end]);
        }
    } else if !from_h && to_h {
        // from vertical, to horizontal
        if dy.signum() == from_dir.y && dx.signum() == to_dir.x {
            candidates.push(vec![start, Point::new(start.x, end.y), end]);
        }
    }

    // Case 2: Same axis, same direction → Z-shape (3 segments) through midpoint
    if from_h && to_h && from_dir.x == to_dir.x && dx.signum() == from_dir.x {
        let mid_x = (start.x + end.x) / 2.0;
        candidates.push(vec![
            start,
            Point::new(mid_x, start.y),
            Point::new(mid_x, end.y),
            end,
        ]);
    } else if !from_h && !to_h && from_dir.y == to_dir.y && dy.signum() == from_dir.y {
        let mid_y = (start.y + end.y) / 2.0;
        candidates.push(vec![
            start,
            Point::new(start.x, mid_y),
            Point::new(end.x, mid_y),
            end,
        ]);
    }

    // Case 3: Same axis, opposing direction → U-shape (3 segments)
    // e.g., from=RIGHT, to=LEFT: →  ↕  ←
    if from_h && to_h && from_dir.x == -to_dir.x {
        // Choose perpendicular offset to avoid crossing between start and end
        let perp_offset = if (end.y - start.y).abs() > MIN_FINAL_SEGMENT_LENGTH {
            // Use the midpoint y if there's enough vertical separation
            (start.y + end.y) / 2.0
        } else {
            // Go above or below, choosing the side with more room
            let offset = MIN_FINAL_SEGMENT_LENGTH.max((end.x - start.x).abs() / 4.0);
            if start.y < end.y {
                start.y - offset
            } else {
                start.y + offset
            }
        };
        let start_out_x = start.x + from_dir.x * MIN_FINAL_SEGMENT_LENGTH;
        let end_in_x = end.x - to_dir.x * MIN_FINAL_SEGMENT_LENGTH;
        candidates.push(vec![
            start,
            Point::new(start_out_x, start.y),
            Point::new(start_out_x, perp_offset),
            Point::new(end_in_x, perp_offset),
            Point::new(end_in_x, end.y),
            end,
        ]);
    } else if !from_h && !to_h && from_dir.y == -to_dir.y {
        let perp_offset = if (end.x - start.x).abs() > MIN_FINAL_SEGMENT_LENGTH {
            (start.x + end.x) / 2.0
        } else {
            let offset = MIN_FINAL_SEGMENT_LENGTH.max((end.y - start.y).abs() / 4.0);
            if start.x < end.x {
                start.x - offset
            } else {
                start.x + offset
            }
        };
        let start_out_y = start.y + from_dir.y * MIN_FINAL_SEGMENT_LENGTH;
        let end_in_y = end.y - to_dir.y * MIN_FINAL_SEGMENT_LENGTH;
        candidates.push(vec![
            start,
            Point::new(start.x, start_out_y),
            Point::new(perp_offset, start_out_y),
            Point::new(perp_offset, end_in_y),
            Point::new(end.x, end_in_y),
            end,
        ]);
    }

    // Case 4: Different axes, geometry opposes direction → stub + L + stub
    // e.g., from=DOWN but target is above → go down, across, up, then into target
    if !candidates.iter().any(|c| c.len() <= 3) {
        let stub = MIN_FINAL_SEGMENT_LENGTH;
        let start_out = Point::new(start.x + from_dir.x * stub, start.y + from_dir.y * stub);
        let end_in = Point::new(end.x - to_dir.x * stub, end.y - to_dir.y * stub);

        // Try routing: start_out → horizontal → end_in
        candidates.push(vec![
            start,
            start_out,
            Point::new(end_in.x, start_out.y),
            end_in,
            end,
        ]);
        // Try routing: start_out → vertical → end_in
        candidates.push(vec![
            start,
            start_out,
            Point::new(start_out.x, end_in.y),
            end_in,
            end,
        ]);
    }

    // Select best valid candidate by total path length (prefer shorter paths)
    let mut best: Option<(Vec<Point>, f64)> = None;
    for path in candidates {
        let simplified = simplify_path(path);
        if simplified.len() < 2 {
            continue;
        }
        // Skip paths with degenerate first/last segments
        if segment_length(simplified[0], simplified[1]) < 0.5
            || segment_length(
                simplified[simplified.len() - 2],
                simplified[simplified.len() - 1],
            ) < 0.5
        {
            continue;
        }
        if is_valid_orthogonal_path(&simplified, from_dir, to_dir) {
            let total_len: f64 = simplified
                .windows(2)
                .map(|w| segment_length(w[0], w[1]))
                .sum();
            let replace = match &best {
                None => true,
                Some((_, best_len)) => total_len < *best_len,
            };
            if replace {
                best = Some((simplified, total_len));
            }
        }
    }

    best.map(|(path, _)| path)
}

fn forced_stub_path(start: Point, end: Point, from_dir: Point, to_dir: Point) -> Vec<Point> {
    let distance = segment_length(start, end);
    let mut stub = MIN_FINAL_SEGMENT_LENGTH;
    if distance > 0.0 {
        stub = stub.min(distance / 2.0).max(1.0);
    }

    let start_out = Point::new(start.x + from_dir.x * stub, start.y + from_dir.y * stub);
    let end_in = Point::new(end.x - to_dir.x * stub, end.y - to_dir.y * stub);

    let hv = vec![
        start,
        start_out,
        Point::new(end_in.x, start_out.y),
        end_in,
        end,
    ];
    let vh = vec![
        start,
        start_out,
        Point::new(start_out.x, end_in.y),
        end_in,
        end,
    ];

    let hv_len: f64 = hv
        .windows(2)
        .map(|pair| segment_length(pair[0], pair[1]))
        .sum();
    let vh_len: f64 = vh
        .windows(2)
        .map(|pair| segment_length(pair[0], pair[1]))
        .sum();

    if hv_len <= vh_len {
        simplify_path(hv)
    } else {
        simplify_path(vh)
    }
}

/// Minimum length for the final segment to ensure proper marker orientation.
/// Short segments can cause browsers to calculate incorrect tangent directions.
const MIN_FINAL_SEGMENT_LENGTH: f64 = 15.0;
/// Route a connection between two bounding boxes with the specified routing mode
/// Optional via_points are control points for curved routing (Feature 008)
pub fn route_connection(
    from_bounds: &BoundingBox,
    to_bounds: &BoundingBox,
    mode: RoutingMode,
    via_points: &[Point],
) -> Vec<Point> {
    // Delegate to anchored version without anchors
    route_connection_with_anchors(from_bounds, to_bounds, mode, via_points, None, None)
}

/// Route a connection between two bounding boxes with optional explicit anchors (Feature 009)
///
/// When anchors are provided, uses their positions directly instead of auto-detecting
/// attachment points from the bounding boxes.
pub fn route_connection_with_anchors(
    from_bounds: &BoundingBox,
    to_bounds: &BoundingBox,
    mode: RoutingMode,
    via_points: &[Point],
    from_anchor: Option<&ResolvedAnchor>,
    to_anchor: Option<&ResolvedAnchor>,
) -> Vec<Point> {
    // Feature 009: Use anchor positions if provided, otherwise calculate from bounds
    let from_center = from_bounds.center();
    let to_center = to_bounds.center();

    let start = from_anchor
        .map(|a| a.position)
        .unwrap_or_else(|| boundary_point_toward(from_bounds, to_center));
    let end = to_anchor
        .map(|a| a.position)
        .unwrap_or_else(|| boundary_point_toward(to_bounds, from_center));

    // For curved routing, use cubic Bezier
    if mode == RoutingMode::Curved {
        let dx = end.x - start.x;
        let dy = end.y - start.y;
        let distance = (dx * dx + dy * dy).sqrt();

        // Determine control point distance (the "bulge" of the curve)
        let control_distance = if !via_points.is_empty() {
            // Use the first via point to determine the bulge amount
            let via = &via_points[0];
            let mid_x = (start.x + end.x) / 2.0;
            let mid_y = (start.y + end.y) / 2.0;
            let via_dx = via.x - mid_x;
            let via_dy = via.y - mid_y;
            (via_dx * via_dx + via_dy * via_dy).sqrt()
        } else {
            distance * 0.4
        };

        // Determine control point directions
        let (from_dir, to_dir) = if let (Some(from_anch), Some(to_anch)) = (from_anchor, to_anchor)
        {
            // Use anchor directions for perpendicular entry/exit
            (
                from_anch.direction.to_vector(),
                to_anch.direction.to_vector(),
            )
        } else {
            // Auto-compute directions perpendicular to the line
            let perp = if distance > 0.001 {
                Point::new(-dy / distance, dx / distance)
            } else {
                Point::new(0.0, 1.0)
            };
            // Both control points on the same side (creates an arc)
            (perp, perp)
        };

        // Control point 1: from start in the from direction
        let control1 = Point::new(
            start.x + from_dir.x * control_distance,
            start.y + from_dir.y * control_distance,
        );

        // Control point 2: from end in the to direction
        let control2 = Point::new(
            end.x + to_dir.x * control_distance,
            end.y + to_dir.y * control_distance,
        );

        // Handle via points with bisector tangents and explicit C commands.
        //
        // Each segment is an explicit cubic Bezier (C command). At each via
        // point, the tangent direction is the bisector of the incoming and
        // outgoing directions (Catmull-Rom style). This gives G1 continuity
        // (smooth transitions) while using independent distance scaling per
        // segment to prevent overshooting.
        //
        // Path: M start C ctrl1_a ctrl2_a via C ctrl1_b ctrl2_b end
        //
        // The bisector tangent solves both problems:
        // - Symmetric arcs: bisector is horizontal at apex → smooth arc
        // - Asymmetric turns: bisector follows the turn naturally → no loops
        //
        // Using explicit C (not S) allows different distances for ctrl2_a and
        // ctrl1_b, scaled to their respective segment lengths.
        if !via_points.is_empty() {
            // Helper: distance between two points
            let dist = |a: Point, b: Point| -> f64 {
                let dx = b.x - a.x;
                let dy = b.y - a.y;
                (dx * dx + dy * dy).sqrt()
            };

            // Helper: normalized direction from a to b
            let dir = |a: Point, b: Point| -> (f64, f64) {
                let dx = b.x - a.x;
                let dy = b.y - a.y;
                let len = (dx * dx + dy * dy).sqrt();
                if len < 0.001 {
                    (0.0, 0.0)
                } else {
                    (dx / len, dy / len)
                }
            };

            // Helper: weighted bisector direction at a via point.
            //
            // Instead of averaging unit incoming/outgoing vectors (which lets
            // the longer segment dominate), we weight each direction by the
            // OTHER segment's distance. This ensures that:
            //   - Short outgoing segments get more influence (the curve must
            //     turn quickly to reach the nearby endpoint)
            //   - Symmetric cases give equal weights → pure bisector
            //
            // Example: architecture ORM connector
            //   incoming = (-0.82, 0.57), seg1_dist = 331
            //   outgoing = ( 0.26, 0.97), seg2_dist =  95
            //   Weighted: (-0.82*95 + 0.26*331, 0.57*95 + 0.97*331)
            //           = (8.5, 375.4), normalized ≈ (0.02, 1.00)
            //   → nearly vertical tangent, no leftward excursion
            let bisector_dir =
                |prev: Point, via: Point, next: Point| -> (f64, f64) {
                    let (in_x, in_y) = dir(prev, via);
                    let (out_x, out_y) = dir(via, next);
                    let d_in = dist(prev, via);
                    let d_out = dist(via, next);
                    // Weight each direction by the OTHER segment's distance
                    let bx = in_x * d_out + out_x * d_in;
                    let by = in_y * d_out + out_y * d_in;
                    let blen = (bx * bx + by * by).sqrt();
                    if blen < 0.001 {
                        (in_x, in_y)
                    } else {
                        (bx / blen, by / blen)
                    }
                };

            // Build waypoint chain: [start, via0, via1, ..., end]
            let mut waypoints = Vec::with_capacity(via_points.len() + 2);
            waypoints.push(start);
            waypoints.extend_from_slice(via_points);
            waypoints.push(end);

            let n = waypoints.len(); // at least 3

            // Segment 1: start → waypoints[1]
            let seg1_d = dist(start, waypoints[1]);
            let ctrl1 = Point::new(
                start.x + from_dir.x * seg1_d / 3.0,
                start.y + from_dir.y * seg1_d / 3.0,
            );
            // ctrl2 of seg1: arrive at waypoints[1] along bisector (incoming side)
            let (bx, by) = bisector_dir(waypoints[0], waypoints[1], waypoints[2]);
            let ctrl2 = Point::new(
                waypoints[1].x - bx * seg1_d / 3.0,
                waypoints[1].y - by * seg1_d / 3.0,
            );
            let mut path = vec![start, ctrl1, ctrl2, waypoints[1]];

            // Intermediate + last segments: waypoints[i] → waypoints[i+1]
            for i in 1..(n - 1) {
                let prev = waypoints[i - 1];
                let curr = waypoints[i];
                let next = waypoints[i + 1];
                let seg_d = dist(curr, next);

                // ctrl1: depart curr along bisector (outgoing side)
                let (bx_c, by_c) = bisector_dir(prev, curr, next);
                let c1 = Point::new(
                    curr.x + bx_c * seg_d / 3.0,
                    curr.y + by_c * seg_d / 3.0,
                );

                if i + 1 < n - 1 {
                    // Next point is another via: use bisector for ctrl2
                    let next_next = waypoints[i + 2];
                    let (bx_n, by_n) = bisector_dir(curr, next, next_next);
                    let c2 = Point::new(
                        next.x - bx_n * seg_d / 3.0,
                        next.y - by_n * seg_d / 3.0,
                    );
                    path.push(c1);
                    path.push(c2);
                    path.push(next);
                } else {
                    // Next point is end: use to_dir for perpendicular entry
                    let c2 = Point::new(
                        end.x + to_dir.x * seg_d / 3.0,
                        end.y + to_dir.y * seg_d / 3.0,
                    );
                    path.push(c1);
                    path.push(c2);
                    path.push(end);
                }
            }

            return path;
        }

        // No via points: simple cubic Bezier
        return vec![start, control1, control2, end];
    }

    // For direct routing, use the pre-calculated start/end positions
    if mode == RoutingMode::Direct {
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

    // Orthogonal routing: use anchor positions if provided, otherwise edge-based attachment
    // Feature 009: When anchors are specified, use their positions
    let (default_from_edge, default_to_edge) = best_edges(from_bounds, to_bounds);
    let (start, from_edge) = if let Some(anchor) = from_anchor {
        (anchor.position, default_from_edge)
    } else {
        (
            attachment_point(from_bounds, default_from_edge),
            default_from_edge,
        )
    };
    let (end, to_edge) = if let Some(anchor) = to_anchor {
        (anchor.position, default_to_edge)
    } else {
        (
            attachment_point(to_bounds, default_to_edge),
            default_to_edge,
        )
    };

    // Orthogonal routing: create paths with horizontal/vertical segments only
    if let (Some(from_anchor), Some(to_anchor)) = (from_anchor, to_anchor) {
        let from_dir = cardinal_direction_for_anchor(from_anchor.direction);
        let anchor_facing = cardinal_direction_for_anchor(to_anchor.direction);
        // The wire must arrive INTO the anchor from the anchor's facing direction,
        // so the last segment goes opposite to the anchor's outward-facing direction.
        let to_dir = Point::new(-anchor_facing.x, -anchor_facing.y);
        if let Some(path) = orthogonal_with_directions(start, end, from_dir, to_dir) {
            return path;
        }
        return forced_stub_path(start, end, from_dir, to_dir);
    }

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
                    "curved" => return RoutingMode::Curved, // Feature 008
                    _ => {}                                 // Unknown value, use default
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
            let center = element.bounds.center();
            // Note: trace output is controlled at a higher level
            // We could add trace parameter here if needed in the future
            points.push(center);
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
                Statement::Connection(conns) => {
                    for conn in conns {
                        // Feature 009: Access element via AnchorReference.element
                        let from_element = result
                            .get_element_by_name(&conn.from.element.node.0)
                            .ok_or_else(|| {
                                LayoutError::undefined(
                                    conn.from.element.node.0.clone(),
                                    conn.from.element.span.clone(),
                                    vec![],
                                )
                            })?;
                        let to_element = result
                            .get_element_by_name(&conn.to.element.node.0)
                            .ok_or_else(|| {
                                LayoutError::undefined(
                                    conn.to.element.node.0.clone(),
                                    conn.to.element.span.clone(),
                                    vec![],
                                )
                            })?;

                        let routing_mode = extract_routing_mode(&conn.modifiers);
                        let from_bounds = from_element.bounds;
                        let to_bounds = to_element.bounds;

                        // Feature 009: Resolve anchors for connection endpoints
                        let from_anchor =
                            resolve_anchor(&conn.from, &result.elements, Some(&to_bounds))?;
                        let to_anchor =
                            resolve_anchor(&conn.to, &result.elements, Some(&from_bounds))?;

                        // Use anchors only if explicitly specified, otherwise auto-detect
                        let from_anchor_opt = conn.from.anchor.as_ref().map(|_| &from_anchor);
                        let to_anchor_opt = conn.to.anchor.as_ref().map(|_| &to_anchor);

                        let via_refs = extract_via_references(&conn.modifiers);
                        let via_points = resolve_via_points(&via_refs, result)?;
                        let path = route_connection_with_anchors(
                            &from_bounds,
                            &to_bounds,
                            routing_mode,
                            &via_points,
                            from_anchor_opt,
                            to_anchor_opt,
                        );
                        let styles = ResolvedStyles::from_modifiers(&conn.modifiers);
                        let (label, label_ref_id) =
                            extract_connection_label_with_ref(&conn.modifiers, &path, result);

                        if let Some(id) = label_ref_id {
                            label_element_ids.insert(id);
                        }

                        result.connections.push(ConnectionLayout {
                            from_id: conn.from.element.node.clone(),
                            to_id: conn.to.element.node.clone(),
                            direction: conn.direction,
                            path,
                            styles,
                            label,
                            routing_mode,
                        });
                    }
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

    // Resolve overlapping connection labels
    resolve_label_overlaps(&mut result.connections);

    // Remove elements that are used as connection labels from the layout
    for id in &label_element_ids {
        result.remove_element_by_name(id);
    }

    result.compute_bounds();
    Ok(())
}

/// Resolve overlapping connection labels by nudging them apart
fn resolve_label_overlaps(connections: &mut [ConnectionLayout]) {
    // Approximate character width and line height for label bounds estimation
    const CHAR_WIDTH: f64 = 7.0;
    const LINE_HEIGHT: f64 = 14.0;
    const PADDING: f64 = 4.0;
    const MIN_SEPARATION: f64 = 2.0;

    // Collect labels with their estimated bounds
    struct LabelBounds {
        conn_idx: usize,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    }

    let mut labels: Vec<LabelBounds> = connections
        .iter()
        .enumerate()
        .filter_map(|(idx, conn)| {
            conn.label.as_ref().map(|label| {
                let width = label.text.len() as f64 * CHAR_WIDTH + PADDING * 2.0;
                let height = LINE_HEIGHT + PADDING;
                // Adjust x based on anchor
                let x = match label.anchor {
                    super::types::TextAnchor::Start => label.position.x,
                    super::types::TextAnchor::Middle => label.position.x - width / 2.0,
                    super::types::TextAnchor::End => label.position.x - width,
                };
                let y = label.position.y - height / 2.0;
                LabelBounds {
                    conn_idx: idx,
                    x,
                    y,
                    width,
                    height,
                }
            })
        })
        .collect();

    // Check for overlaps and nudge labels apart
    // Simple approach: for each pair, if they overlap, move them apart vertically
    let n = labels.len();
    for i in 0..n {
        for j in (i + 1)..n {
            let a = &labels[i];
            let b = &labels[j];

            // Check if bounding boxes overlap
            let overlap_x = a.x < b.x + b.width && a.x + a.width > b.x;
            let overlap_y = a.y < b.y + b.height && a.y + a.height > b.y;

            if overlap_x && overlap_y {
                // Calculate how much to separate them vertically
                let overlap_amount = (a.y + a.height - b.y).min(b.y + b.height - a.y);
                let nudge = (overlap_amount / 2.0) + MIN_SEPARATION;

                // Determine which one is above (smaller y = higher up)
                let (upper_idx, lower_idx) = if a.y <= b.y { (i, j) } else { (j, i) };

                // Apply nudges to the actual connection labels
                if let Some(ref mut label) = connections[labels[upper_idx].conn_idx].label {
                    label.position.y -= nudge;
                }
                if let Some(ref mut label) = connections[labels[lower_idx].conn_idx].label {
                    label.position.y += nudge;
                }

                // Update our tracking bounds too
                labels[upper_idx].y -= nudge;
                labels[lower_idx].y += nudge;
            }
        }
    }
}

/// Label position for connection labels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelPosition {
    Left,
    Right,
    Center,
}

/// Compute perpendicular offset position and appropriate text anchor.
/// `perp_x`/`perp_y` is the unit perpendicular vector (right side of travel direction).
/// Positive `offset` goes to the right of travel, negative to the left.
fn perpendicular_label_position(
    base_x: f64, base_y: f64,
    perp_x: f64, perp_y: f64,
    offset: f64,
) -> (f64, f64, TextAnchor) {
    let x = base_x + perp_x * offset;
    let y = base_y + perp_y * offset;
    let anchor = if perp_x.abs() > perp_y.abs() {
        if perp_x * offset > 0.0 { TextAnchor::Start } else { TextAnchor::End }
    } else {
        TextAnchor::Middle
    };
    (x, y, anchor)
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
                    result.get_element_by_name(&id.0).map(|element| {
                        // Mark this element ID for removal from rendering
                        label_ref_id = Some(id.0.clone());
                        // Capture styles from the referenced element
                        label_styles = Some(element.styles.clone());
                        if let ElementType::Shape(ShapeType::Text { content }) =
                            &element.element_type
                        {
                            content.clone()
                        } else {
                            // If it's not a text shape, use the identifier as the label text
                            id.0.clone()
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

    // Extract label_offset modifier (perpendicular distance from path, default 10.0)
    let label_offset = modifiers.iter().find_map(|m| {
        if matches!(m.node.key.node, StyleKey::LabelOffset) {
            match &m.node.value.node {
                StyleValue::Number { value, .. } => Some(value.max(0.0)),
                _ => None,
            }
        } else {
            None
        }
    }).unwrap_or(10.0);

    // Extract label_at modifier (fraction along path, default 0.5)
    let label_at = modifiers.iter().find_map(|m| {
        if matches!(m.node.key.node, StyleKey::LabelAt) {
            match &m.node.value.node {
                StyleValue::Number { value, .. } => Some(value.clamp(0.0, 1.0)),
                _ => None,
            }
        } else {
            None
        }
    }).unwrap_or(0.5);

    // Calculate label position - for curves, use the actual curve point at label_at
    let (mid_x, mid_y, anchor) = if path.len() == 4 {
        // Cubic Bezier: calculate point at t=label_at
        let t = label_at;
        let mt = 1.0 - t;
        let p0 = path[0];
        let p1 = path[1];
        let p2 = path[2];
        let p3 = path[3];
        let base_mid_x =
            mt*mt*mt * p0.x + 3.0*mt*mt*t * p1.x + 3.0*mt*t*t * p2.x + t*t*t * p3.x;
        let base_mid_y =
            mt*mt*mt * p0.y + 3.0*mt*mt*t * p1.y + 3.0*mt*t*t * p2.y + t*t*t * p3.y;

        // Compute tangent via B'(t) derivative
        let dt_x = 3.0*mt*mt*(p1.x - p0.x) + 6.0*mt*t*(p2.x - p1.x) + 3.0*t*t*(p3.x - p2.x);
        let dt_y = 3.0*mt*mt*(p1.y - p0.y) + 6.0*mt*t*(p2.y - p1.y) + 3.0*t*t*(p3.y - p2.y);
        let tangent_len = (dt_x * dt_x + dt_y * dt_y).sqrt();

        // Right-side perpendicular (clockwise 90°): (ty, -tx) normalized
        let (perp_x, perp_y) = if tangent_len > 0.001 {
            (dt_y / tangent_len, -dt_x / tangent_len)
        } else {
            (1.0, 0.0) // degenerate fallback: offset to the right in screen space
        };

        match label_position {
            Some(LabelPosition::Right) => perpendicular_label_position(base_mid_x, base_mid_y, perp_x, perp_y, label_offset),
            Some(LabelPosition::Left) => perpendicular_label_position(base_mid_x, base_mid_y, perp_x, perp_y, -label_offset),
            Some(LabelPosition::Center) => (base_mid_x, base_mid_y, TextAnchor::Middle),
            None => perpendicular_label_position(base_mid_x, base_mid_y, perp_x, perp_y, label_offset),
        }
    } else if path.len() >= 2 {
        // Other paths: interpolate along polyline at label_at fraction

        // Compute total polyline length and find point at label_at fraction
        let mut segment_lengths: Vec<f64> = Vec::with_capacity(path.len() - 1);
        let mut total_length = 0.0;
        for i in 0..path.len() - 1 {
            let dx = path[i + 1].x - path[i].x;
            let dy = path[i + 1].y - path[i].y;
            let len = (dx * dx + dy * dy).sqrt();
            segment_lengths.push(len);
            total_length += len;
        }

        let target_dist = label_at * total_length;
        let mut accumulated = 0.0;
        let mut base_mid_x = path[0].x;
        let mut base_mid_y = path[0].y;
        let mut seg_dx = 0.0;
        let mut seg_dy = 0.0;
        for (i, &seg_len) in segment_lengths.iter().enumerate() {
            if accumulated + seg_len >= target_dist {
                let frac = if seg_len > 0.0 { (target_dist - accumulated) / seg_len } else { 0.0 };
                base_mid_x = path[i].x + frac * (path[i + 1].x - path[i].x);
                base_mid_y = path[i].y + frac * (path[i + 1].y - path[i].y);
                seg_dx = path[i + 1].x - path[i].x;
                seg_dy = path[i + 1].y - path[i].y;
                break;
            }
            accumulated += seg_len;
        }

        // Compute right-side perpendicular from segment tangent
        let tangent_len = (seg_dx * seg_dx + seg_dy * seg_dy).sqrt();
        let (perp_x, perp_y) = if tangent_len > 0.001 {
            (seg_dy / tangent_len, -seg_dx / tangent_len)
        } else {
            (1.0, 0.0) // degenerate fallback
        };

        match label_position {
            Some(LabelPosition::Right) => perpendicular_label_position(base_mid_x, base_mid_y, perp_x, perp_y, label_offset),
            Some(LabelPosition::Left) => perpendicular_label_position(base_mid_x, base_mid_y, perp_x, perp_y, -label_offset),
            Some(LabelPosition::Center) => (base_mid_x, base_mid_y, TextAnchor::Middle),
            None => perpendicular_label_position(base_mid_x, base_mid_y, perp_x, perp_y, label_offset),
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

        // Curved routing returns 4 points: start, control1, control2, end (cubic Bezier)
        assert_eq!(path.len(), 4, "Curved routing should return 4 points");

        // Start point should be on right edge of from_bounds
        assert!(
            (path[0].x - 50.0).abs() < 1.0,
            "Start x should be at right edge of from_bounds"
        );

        // End point should be on left edge of to_bounds
        assert!(
            (path[3].x - 150.0).abs() < 1.0,
            "End x should be at left edge of to_bounds"
        );

        // Control points should create a curve
        // For horizontal connection without anchors, control points are offset perpendicular
        assert!(
            (path[1].y - 25.0).abs() > 1.0 || (path[2].y - 25.0).abs() > 1.0,
            "At least one control point y should be offset from the center line"
        );
    }

    #[test]
    fn test_routing_mode_curved_exists() {
        // Verify the Curved variant exists and is distinct
        assert_ne!(RoutingMode::Curved, RoutingMode::Direct);
        assert_ne!(RoutingMode::Curved, RoutingMode::Orthogonal);
    }

    fn make_label_modifiers_with_offset(label: &str, offset: f64) -> Vec<Spanned<StyleModifier>> {
        let mut modifiers = make_label_modifiers(label, None);
        modifiers.push(Spanned::new(
            StyleModifier {
                key: Spanned::new(StyleKey::LabelOffset, 0..12),
                value: Spanned::new(StyleValue::Number { value: offset, unit: None }, 14..16),
            },
            0..16,
        ));
        modifiers
    }

    #[test]
    fn test_label_offset_custom_value() {
        // Vertical path with custom label_offset
        let path = vec![Point::new(50.0, 0.0), Point::new(50.0, 100.0)];
        let modifiers = make_label_modifiers_with_offset("Test", 25.0);

        let label = extract_connection_label(&modifiers, &path, &empty_result()).unwrap();

        // Auto-detect for vertical: right side, offset should be 25
        assert_eq!(label.position.x, 75.0, "Custom offset 25 should add 25 to x");
    }

    #[test]
    fn test_label_offset_zero() {
        // Vertical path with zero offset — label on the path
        let path = vec![Point::new(50.0, 0.0), Point::new(50.0, 100.0)];
        let modifiers = make_label_modifiers_with_offset("Test", 0.0);

        let label = extract_connection_label(&modifiers, &path, &empty_result()).unwrap();

        assert_eq!(label.position.x, 50.0, "Zero offset should keep label on path");
    }

    #[test]
    fn test_label_offset_default() {
        // Vertical path without label_offset — should default to 10
        let path = vec![Point::new(50.0, 0.0), Point::new(50.0, 100.0)];
        let modifiers = make_label_modifiers("Test", None);

        let label = extract_connection_label(&modifiers, &path, &empty_result()).unwrap();

        assert_eq!(label.position.x, 60.0, "Default offset should be 10");
    }

    #[test]
    fn test_label_offset_diagonal_path() {
        // Diagonal path: tangent-relative offset should be perpendicular to the diagonal
        let path = vec![Point::new(0.0, 0.0), Point::new(100.0, 100.0)];
        let modifiers = make_label_modifiers("Test", None);
        let label = extract_connection_label(&modifiers, &path, &empty_result()).unwrap();
        // Tangent is (1,1)/sqrt(2), right perp is (1,-1)/sqrt(2)
        // Offset 10 along perp: (50 + 10/sqrt(2), 50 - 10/sqrt(2))
        let expected_offset = 10.0 / 2.0_f64.sqrt();
        assert!((label.position.x - (50.0 + expected_offset)).abs() < 0.01,
            "Diagonal x: expected {}, got {}", 50.0 + expected_offset, label.position.x);
        assert!((label.position.y - (50.0 - expected_offset)).abs() < 0.01,
            "Diagonal y: expected {}, got {}", 50.0 - expected_offset, label.position.y);
    }
}
