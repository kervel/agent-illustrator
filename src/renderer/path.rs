//! Path shape rendering utilities (Feature 007)
//!
//! Converts PathDecl AST nodes into SVG path `d` attribute strings.

use crate::layout::Point;
use crate::parser::ast::{ArcParams, PathCommand, PathDecl, SweepDirection, VertexPosition};
use std::collections::HashMap;

/// A segment in a resolved path
#[derive(Debug, Clone)]
pub enum PathSegment {
    /// Move to starting point
    MoveTo(Point),
    /// Straight line to point
    LineTo(Point),
    /// Arc to point with radius and flags
    ArcTo {
        end: Point,
        radius: f64,
        large_arc: bool,
        sweep: bool, // true = clockwise in SVG coordinates (y-down)
    },
    /// Quadratic Bezier curve (Feature 008)
    QuadraticTo {
        control: Point,
        end: Point,
    },
    /// Smooth quadratic continuation (Feature 008)
    /// Uses SVG T command - control point is auto-reflected
    SmoothQuadraticTo(Point),
    /// Close path back to start
    Close,
}

/// A resolved path ready for SVG rendering
#[derive(Debug)]
pub struct ResolvedPath {
    pub segments: Vec<PathSegment>,
}

impl ResolvedPath {
    /// Convert to SVG path `d` attribute string
    pub fn to_svg_d(&self) -> String {
        if self.segments.is_empty() {
            return String::new();
        }

        let mut d = String::new();

        for seg in &self.segments {
            match seg {
                PathSegment::MoveTo(p) => {
                    d.push_str(&format!("M{:.2} {:.2}", p.x, p.y));
                }
                PathSegment::LineTo(p) => {
                    d.push_str(&format!(" L{:.2} {:.2}", p.x, p.y));
                }
                PathSegment::ArcTo {
                    end,
                    radius,
                    large_arc,
                    sweep,
                } => {
                    let large = if *large_arc { 1 } else { 0 };
                    let sw = if *sweep { 1 } else { 0 };
                    // SVG arc: A rx ry x-axis-rotation large-arc-flag sweep-flag x y
                    // Using equal rx and ry for circular arcs
                    d.push_str(&format!(
                        " A{:.2} {:.2} 0 {} {} {:.2} {:.2}",
                        radius, radius, large, sw, end.x, end.y
                    ));
                }
                PathSegment::QuadraticTo { control, end } => {
                    // SVG Q command: Q cx cy ex ey (quadratic Bezier)
                    d.push_str(&format!(
                        " Q{:.2} {:.2} {:.2} {:.2}",
                        control.x, control.y, end.x, end.y
                    ));
                }
                PathSegment::SmoothQuadraticTo(end) => {
                    // SVG T command: T ex ey (smooth quadratic - auto-reflected control point)
                    d.push_str(&format!(" T{:.2} {:.2}", end.x, end.y));
                }
                PathSegment::Close => {
                    d.push_str(" Z");
                }
            }
        }

        d
    }
}

/// Resolve a PathDecl into concrete coordinates
///
/// The origin point is the top-left corner of the element's bounding box.
/// All vertex positions are relative to this origin.
pub fn resolve_path(decl: &PathDecl, origin: Point) -> ResolvedPath {
    let mut vertices: HashMap<String, Point> = HashMap::new();
    let mut segments: Vec<PathSegment> = Vec::new();
    let mut current_pos: Option<Point> = None;
    let mut start_pos: Option<Point> = None;

    for cmd in &decl.body.commands {
        match &cmd.node {
            PathCommand::Vertex(v) => {
                let pos = resolve_vertex_position(&v.position, origin);
                vertices.insert(v.name.node.as_str().to_string(), pos);

                if start_pos.is_none() {
                    // First vertex command becomes the starting point
                    segments.push(PathSegment::MoveTo(pos));
                    start_pos = Some(pos);
                } else {
                    // Subsequent vertex declarations are implicitly connected
                    segments.push(PathSegment::LineTo(pos));
                }
                current_pos = Some(pos);
            }
            PathCommand::LineTo(lt) => {
                let pos = get_or_create_vertex(
                    lt.target.node.as_str(),
                    &lt.position,
                    origin,
                    &mut vertices,
                );

                // If we haven't started the path yet, move to current position first
                if current_pos.is_none() {
                    // Start from origin if no vertex was defined
                    segments.push(PathSegment::MoveTo(origin));
                    start_pos = Some(origin);
                }

                segments.push(PathSegment::LineTo(pos));
                current_pos = Some(pos);
            }
            PathCommand::ArcTo(arc) => {
                let pos = get_or_create_vertex(
                    arc.target.node.as_str(),
                    &arc.position,
                    origin,
                    &mut vertices,
                );

                // If we haven't started the path yet, move to current position first
                if current_pos.is_none() {
                    segments.push(PathSegment::MoveTo(origin));
                    start_pos = Some(origin);
                    current_pos = Some(origin);
                }

                let from = current_pos.unwrap_or(origin);
                let arc_seg = compute_arc_segment(from, pos, &arc.params);
                segments.push(arc_seg);
                current_pos = Some(pos);
            }
            PathCommand::CurveTo(ct) => {
                let end_pos = get_or_create_vertex(
                    ct.target.node.as_str(),
                    &ct.position,
                    origin,
                    &mut vertices,
                );

                // If we haven't started the path yet, move to current position first
                if current_pos.is_none() {
                    segments.push(PathSegment::MoveTo(origin));
                    start_pos = Some(origin);
                    current_pos = Some(origin);
                }

                let from = current_pos.unwrap_or(origin);

                // Note: Via reference resolution happens in the layout phase (T015-T018)
                // For now, if via is provided but not resolved, we fall back to auto-generated control point
                // The layout phase will inject resolved via positions before rendering
                let control = if let Some(via_ref) = &ct.via {
                    // Try to look up via as a vertex (may have been defined earlier in path)
                    vertices
                        .get(via_ref.node.as_str())
                        .copied()
                        .unwrap_or_else(|| compute_default_control_point(from, end_pos))
                } else {
                    // Auto-generate control point
                    compute_default_control_point(from, end_pos)
                };

                segments.push(PathSegment::QuadraticTo {
                    control,
                    end: end_pos,
                });
                current_pos = Some(end_pos);
            }
            PathCommand::Close => {
                segments.push(PathSegment::Close);
                current_pos = start_pos;
            }
            PathCommand::CloseArc(params) => {
                if let (Some(from), Some(to)) = (current_pos, start_pos) {
                    let arc_seg = compute_arc_segment(from, to, params);
                    segments.push(arc_seg);
                }
                segments.push(PathSegment::Close);
                current_pos = start_pos;
            }
        }
    }

    ResolvedPath { segments }
}

/// Resolve a vertex position relative to the origin
fn resolve_vertex_position(pos: &Option<VertexPosition>, origin: Point) -> Point {
    match pos {
        Some(p) => Point::new(origin.x + p.x.unwrap_or(0.0), origin.y + p.y.unwrap_or(0.0)),
        None => origin,
    }
}

/// Get an existing vertex or create a new one with the given position
fn get_or_create_vertex(
    name: &str,
    position: &Option<VertexPosition>,
    origin: Point,
    vertices: &mut HashMap<String, Point>,
) -> Point {
    // If position is provided, use it (and update/create the vertex)
    if let Some(pos) = position {
        let point = Point::new(
            origin.x + pos.x.unwrap_or(0.0),
            origin.y + pos.y.unwrap_or(0.0),
        );
        vertices.insert(name.to_string(), point);
        return point;
    }

    // Otherwise, look up existing vertex
    if let Some(&point) = vertices.get(name) {
        return point;
    }

    // Vertex doesn't exist and no position given - use origin as fallback
    // (This shouldn't happen in well-formed paths)
    origin
}

/// Compute an arc segment from start to end with the given parameters
fn compute_arc_segment(from: Point, to: Point, params: &ArcParams) -> PathSegment {
    match params {
        ArcParams::Bulge(bulge) => compute_bulge_arc(from, to, *bulge),
        ArcParams::Radius { radius, sweep } => compute_radius_arc(from, to, *radius, *sweep),
    }
}

/// Compute arc from bulge factor
///
/// Bulge = tan(θ/4) where θ is the included angle of the arc
/// - bulge = 0 → straight line
/// - bulge = 1 → semicircle
/// - bulge ≈ 0.414 → quarter circle (45°)
/// - negative bulge → curve on opposite side
fn compute_bulge_arc(from: Point, to: Point, bulge: f64) -> PathSegment {
    // If bulge is essentially zero, render as line
    if bulge.abs() < 0.001 {
        return PathSegment::LineTo(to);
    }

    // Calculate chord length
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let chord = (dx * dx + dy * dy).sqrt();

    // Degenerate case: start and end are the same point
    if chord < 0.001 {
        return PathSegment::LineTo(to);
    }

    // Calculate sagitta (perpendicular distance from chord midpoint to arc)
    // sagitta = |bulge| * chord / 2
    let sagitta = bulge.abs() * chord / 2.0;

    // Calculate radius from chord and sagitta
    // Using: radius = (chord² / 8*sagitta) + (sagitta / 2)
    // Simplified: radius = (chord² + 4*sagitta²) / (8*sagitta)
    let radius = (chord * chord + 4.0 * sagitta * sagitta) / (8.0 * sagitta);

    PathSegment::ArcTo {
        end: to,
        radius,
        large_arc: false,   // Bulge-based arcs always use the small arc
        sweep: bulge > 0.0, // Positive bulge = clockwise in SVG coords
    }
}

/// Compute arc from explicit radius and sweep direction
fn compute_radius_arc(from: Point, to: Point, radius: f64, sweep: SweepDirection) -> PathSegment {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let chord = (dx * dx + dy * dy).sqrt();

    // If radius is too small for the chord, clamp to semicircle
    let r = if chord > 2.0 * radius {
        chord / 2.0
    } else if radius < 0.001 {
        // Zero radius means straight line
        return PathSegment::LineTo(to);
    } else {
        radius
    };

    PathSegment::ArcTo {
        end: to,
        radius: r,
        large_arc: false, // Always use the smaller arc
        sweep: matches!(sweep, SweepDirection::Clockwise),
    }
}

/// Compute default control point for quadratic Bezier curve (Feature 008)
///
/// Creates a control point perpendicular to the chord at 25% of the chord length.
/// This produces gentle, visually pleasing curves by default.
///
/// The offset is in the positive perpendicular direction (counterclockwise from
/// the chord vector), creating consistent "outward" curves.
fn compute_default_control_point(start: Point, end: Point) -> Point {
    let chord = Point::new(end.x - start.x, end.y - start.y);
    let chord_length = (chord.x * chord.x + chord.y * chord.y).sqrt();

    // Degenerate case: start and end are the same point
    if chord_length < 0.001 {
        return start;
    }

    // Perpendicular vector (counterclockwise rotation)
    let perpendicular = Point::new(-chord.y / chord_length, chord.x / chord_length);

    // Midpoint of chord
    let midpoint = Point::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);

    // Offset perpendicular at 25% of chord length
    let offset = chord_length * 0.25;

    Point::new(
        midpoint.x + perpendicular.x * offset,
        midpoint.y + perpendicular.y * offset,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::{ArcToDecl, CurveToDecl, Identifier, LineToDecl, PathBody, Spanned, VertexDecl};

    fn make_vertex(name: &str, x: Option<f64>, y: Option<f64>) -> Spanned<PathCommand> {
        Spanned::new(
            PathCommand::Vertex(VertexDecl {
                name: Spanned::new(Identifier::new(name), 0..1),
                position: if x.is_some() || y.is_some() {
                    Some(VertexPosition { x, y })
                } else {
                    None
                },
            }),
            0..1,
        )
    }

    fn make_line_to(target: &str, x: Option<f64>, y: Option<f64>) -> Spanned<PathCommand> {
        Spanned::new(
            PathCommand::LineTo(LineToDecl {
                target: Spanned::new(Identifier::new(target), 0..1),
                position: if x.is_some() || y.is_some() {
                    Some(VertexPosition { x, y })
                } else {
                    None
                },
            }),
            0..1,
        )
    }

    fn make_arc_to(
        target: &str,
        x: Option<f64>,
        y: Option<f64>,
        params: ArcParams,
    ) -> Spanned<PathCommand> {
        Spanned::new(
            PathCommand::ArcTo(ArcToDecl {
                target: Spanned::new(Identifier::new(target), 0..1),
                position: if x.is_some() || y.is_some() {
                    Some(VertexPosition { x, y })
                } else {
                    None
                },
                params,
            }),
            0..1,
        )
    }

    fn make_close() -> Spanned<PathCommand> {
        Spanned::new(PathCommand::Close, 0..1)
    }

    fn make_curve_to(
        target: &str,
        x: Option<f64>,
        y: Option<f64>,
        via: Option<&str>,
    ) -> Spanned<PathCommand> {
        Spanned::new(
            PathCommand::CurveTo(CurveToDecl {
                target: Spanned::new(Identifier::new(target), 0..1),
                via: via.map(|v| Spanned::new(Identifier::new(v), 0..1)),
                position: if x.is_some() || y.is_some() {
                    Some(VertexPosition { x, y })
                } else {
                    None
                },
            }),
            0..1,
        )
    }

    #[test]
    fn test_simple_triangle() {
        let decl = PathDecl {
            name: None,
            body: PathBody {
                commands: vec![
                    make_vertex("a", None, None),
                    make_line_to("b", Some(50.0), Some(0.0)),
                    make_line_to("c", Some(25.0), Some(40.0)),
                    make_close(),
                ],
            },
            modifiers: vec![],
        };

        let origin = Point::new(10.0, 20.0);
        let resolved = resolve_path(&decl, origin);
        let d = resolved.to_svg_d();

        assert!(d.starts_with("M10.00 20.00"), "Should start at origin");
        assert!(d.contains("L60.00 20.00"), "Should line to (10+50, 20+0)");
        assert!(d.contains("L35.00 60.00"), "Should line to (10+25, 20+40)");
        assert!(d.ends_with(" Z"), "Should close the path");
    }

    #[test]
    fn test_arc_with_bulge() {
        let decl = PathDecl {
            name: None,
            body: PathBody {
                commands: vec![
                    make_vertex("a", None, None),
                    make_arc_to("b", Some(50.0), Some(0.0), ArcParams::Bulge(0.5)),
                ],
            },
            modifiers: vec![],
        };

        let origin = Point::new(0.0, 0.0);
        let resolved = resolve_path(&decl, origin);
        let d = resolved.to_svg_d();

        assert!(d.starts_with("M0.00 0.00"), "Should start at origin");
        assert!(d.contains(" A"), "Should contain arc command");
        assert!(d.contains("50.00 0.00"), "Should end at (50, 0)");
    }

    #[test]
    fn test_arc_with_radius() {
        let decl = PathDecl {
            name: None,
            body: PathBody {
                commands: vec![
                    make_vertex("a", None, None),
                    make_arc_to(
                        "b",
                        Some(50.0),
                        Some(0.0),
                        ArcParams::Radius {
                            radius: 30.0,
                            sweep: SweepDirection::Clockwise,
                        },
                    ),
                ],
            },
            modifiers: vec![],
        };

        let origin = Point::new(0.0, 0.0);
        let resolved = resolve_path(&decl, origin);
        let d = resolved.to_svg_d();

        assert!(
            d.contains(" A30.00 30.00 0 0 1"),
            "Should have correct arc params"
        );
    }

    #[test]
    fn test_empty_path() {
        let decl = PathDecl {
            name: None,
            body: PathBody { commands: vec![] },
            modifiers: vec![],
        };

        let resolved = resolve_path(&decl, Point::new(0.0, 0.0));
        assert!(resolved.to_svg_d().is_empty());
    }

    #[test]
    fn test_single_vertex() {
        let decl = PathDecl {
            name: None,
            body: PathBody {
                commands: vec![make_vertex("a", Some(25.0), Some(25.0))],
            },
            modifiers: vec![],
        };

        let resolved = resolve_path(&decl, Point::new(0.0, 0.0));
        let d = resolved.to_svg_d();

        assert!(
            d.starts_with("M25.00 25.00"),
            "Should move to vertex position"
        );
    }

    #[test]
    fn test_path_to_d() {
        let path = ResolvedPath {
            segments: vec![
                PathSegment::MoveTo(Point::new(0.0, 0.0)),
                PathSegment::LineTo(Point::new(100.0, 0.0)),
                PathSegment::LineTo(Point::new(100.0, 100.0)),
                PathSegment::Close,
            ],
        };

        let d = path.to_svg_d();
        assert_eq!(d, "M0.00 0.00 L100.00 0.00 L100.00 100.00 Z");
    }

    // Feature 008: Curve tests
    #[test]
    fn test_curve_to_auto_control() {
        // Test curve_to without via - should auto-generate control point
        let decl = PathDecl {
            name: None,
            body: PathBody {
                commands: vec![
                    make_vertex("a", None, None),
                    make_curve_to("b", Some(100.0), Some(0.0), None),
                ],
            },
            modifiers: vec![],
        };

        let origin = Point::new(0.0, 0.0);
        let resolved = resolve_path(&decl, origin);
        let d = resolved.to_svg_d();

        // Should start at origin
        assert!(d.starts_with("M0.00 0.00"), "Should start at origin");
        // Should contain Q command (quadratic Bezier)
        assert!(d.contains(" Q"), "Should contain quadratic curve command");
        // Should end at (100, 0)
        assert!(d.contains("100.00 0.00"), "Should end at (100, 0)");
    }

    #[test]
    fn test_curve_to_with_via_vertex() {
        // Test curve_to with via referencing an earlier vertex
        let decl = PathDecl {
            name: None,
            body: PathBody {
                commands: vec![
                    make_vertex("a", None, None),
                    make_vertex("ctrl", Some(50.0), Some(-30.0)), // control point above the chord
                    make_curve_to("b", Some(100.0), Some(0.0), Some("ctrl")),
                ],
            },
            modifiers: vec![],
        };

        let origin = Point::new(0.0, 0.0);
        let resolved = resolve_path(&decl, origin);
        let d = resolved.to_svg_d();

        // Should contain Q command with control point at (50, -30) and end at (100, 0)
        assert!(d.contains(" Q50.00 -30.00 100.00 0.00"), "Should use ctrl vertex as control point");
    }

    #[test]
    fn test_quadratic_svg_output() {
        // Test that QuadraticTo generates correct SVG Q command
        let path = ResolvedPath {
            segments: vec![
                PathSegment::MoveTo(Point::new(0.0, 0.0)),
                PathSegment::QuadraticTo {
                    control: Point::new(50.0, -30.0),
                    end: Point::new(100.0, 0.0),
                },
            ],
        };

        let d = path.to_svg_d();
        assert_eq!(d, "M0.00 0.00 Q50.00 -30.00 100.00 0.00");
    }

    #[test]
    fn test_smooth_quadratic_svg_output() {
        // Test that SmoothQuadraticTo generates correct SVG T command
        let path = ResolvedPath {
            segments: vec![
                PathSegment::MoveTo(Point::new(0.0, 0.0)),
                PathSegment::QuadraticTo {
                    control: Point::new(25.0, -20.0),
                    end: Point::new(50.0, 0.0),
                },
                PathSegment::SmoothQuadraticTo(Point::new(100.0, 0.0)),
            ],
        };

        let d = path.to_svg_d();
        assert_eq!(d, "M0.00 0.00 Q25.00 -20.00 50.00 0.00 T100.00 0.00");
    }

    #[test]
    fn test_default_control_point_calculation() {
        // Test the default control point calculation
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 0.0);
        let control = compute_default_control_point(start, end);

        // Control point should be at midpoint (50, 0) plus perpendicular offset
        // Chord = (100, 0), perpendicular = (-0, 100) normalized = (0, 1)
        // Offset is 25% of 100 = 25 in perpendicular direction
        // So control should be at (50, 25) - below the chord (positive Y in SVG)
        assert!((control.x - 50.0).abs() < 0.01, "Control x should be 50");
        assert!((control.y - 25.0).abs() < 0.01, "Control y should be 25 (perpendicular offset)");
    }

    #[test]
    fn test_default_control_point_degenerate() {
        // Test degenerate case where start == end
        let start = Point::new(50.0, 50.0);
        let end = Point::new(50.0, 50.0);
        let control = compute_default_control_point(start, end);

        // Should return start point for degenerate case
        assert!((control.x - 50.0).abs() < 0.01);
        assert!((control.y - 50.0).abs() < 0.01);
    }
}
