# Tasks: Path Rendering Implementation

## Overview

Implement SVG rendering for `PathDecl` shapes, converting path commands to SVG `<path d="...">` elements.

## Task Summary

| Task | Description | Est. Complexity |
|------|-------------|-----------------|
| R01 | Add `add_path` method to SvgBuilder | Low |
| R02 | Add path rendering helper module | Medium |
| R03 | Implement vertex resolution | Medium |
| R04 | Implement line segment generation | Low |
| R05 | Implement arc calculation (bulge) | High |
| R06 | Implement arc calculation (radius) | Medium |
| R07 | Integrate into render_element | Low |
| R08 | Add rendering tests | Medium |

---

## R01: Add `add_path` method to SvgBuilder

**File:** `src/renderer/svg.rs`

**Description:**
Add a method to SvgBuilder for rendering arbitrary SVG paths:

```rust
/// Add a path element with custom d attribute
pub fn add_path(
    &mut self,
    id: Option<&str>,
    d: &str,
    classes: &[String],
    styles: &str,
) {
    let prefix = self.prefix();
    let id_attr = id.map(|i| format!(r#" id="{}""#, i)).unwrap_or_default();
    let class_list = std::iter::once(format!("{}path", prefix))
        .chain(classes.iter().cloned())
        .collect::<Vec<_>>()
        .join(" ");

    self.elements.push(format!(
        r#"{}<path{} class="{}" d="{}"{}/>"#,
        self.indent_str(),
        id_attr,
        class_list,
        d,
        if styles.is_empty() { String::new() } else { format!(" {}", styles) }
    ));
}
```

**Acceptance:** Method compiles and is available for use.

---

## R02: Add path rendering helper module

**File:** `src/renderer/path.rs` (new file)

**Description:**
Create a new module for path rendering logic:

```rust
//! Path shape rendering utilities

use crate::layout::Point;
use crate::parser::ast::{PathDecl, PathCommand, ArcParams, SweepDirection};
use std::collections::HashMap;

/// Resolved path ready for SVG rendering
pub struct ResolvedPath {
    pub segments: Vec<PathSegment>,
}

pub enum PathSegment {
    MoveTo(Point),
    LineTo(Point),
    ArcTo {
        end: Point,
        radius: f64,
        large_arc: bool,
        sweep: bool,  // true = clockwise in SVG coords
    },
    Close,
}

impl ResolvedPath {
    /// Convert to SVG path d attribute string
    pub fn to_svg_d(&self) -> String {
        // Implementation in R04-R06
    }
}

/// Resolve a PathDecl into concrete coordinates
pub fn resolve_path(decl: &PathDecl, origin: Point) -> ResolvedPath {
    // Implementation in R03
}
```

Add to `src/renderer/mod.rs`:
```rust
mod path;
pub use path::{resolve_path, ResolvedPath};
```

**Acceptance:** Module structure compiles.

---

## R03: Implement vertex resolution

**File:** `src/renderer/path.rs`

**Description:**
Implement the `resolve_path` function to convert PathDecl commands into resolved points:

```rust
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
                if current_pos.is_none() {
                    // First vertex is the starting point
                    segments.push(PathSegment::MoveTo(pos));
                    start_pos = Some(pos);
                }
                current_pos = Some(pos);
            }
            PathCommand::LineTo(lt) => {
                let pos = get_or_create_vertex(&lt.target, &lt.position, origin, &mut vertices);
                segments.push(PathSegment::LineTo(pos));
                current_pos = Some(pos);
            }
            PathCommand::ArcTo(arc) => {
                let pos = get_or_create_vertex(&arc.target, &arc.position, origin, &mut vertices);
                let from = current_pos.unwrap_or(origin);
                let arc_seg = compute_arc_segment(from, pos, &arc.params);
                segments.push(arc_seg);
                current_pos = Some(pos);
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

fn resolve_vertex_position(pos: &Option<VertexPosition>, origin: Point) -> Point {
    match pos {
        Some(p) => Point::new(
            origin.x + p.x.unwrap_or(0.0),
            origin.y + p.y.unwrap_or(0.0),
        ),
        None => origin,
    }
}
```

**Acceptance:** Vertex positions resolve correctly. Unit tests pass.

---

## R04: Implement line segment generation

**File:** `src/renderer/path.rs`

**Description:**
Implement `to_svg_d()` for line segments:

```rust
impl ResolvedPath {
    pub fn to_svg_d(&self) -> String {
        let mut d = String::new();

        for seg in &self.segments {
            match seg {
                PathSegment::MoveTo(p) => {
                    d.push_str(&format!("M{} {}", p.x, p.y));
                }
                PathSegment::LineTo(p) => {
                    d.push_str(&format!(" L{} {}", p.x, p.y));
                }
                PathSegment::ArcTo { end, radius, large_arc, sweep } => {
                    // R05/R06 will implement this
                    let large = if *large_arc { 1 } else { 0 };
                    let sw = if *sweep { 1 } else { 0 };
                    d.push_str(&format!(" A{} {} 0 {} {} {} {}",
                        radius, radius, large, sw, end.x, end.y));
                }
                PathSegment::Close => {
                    d.push_str(" Z");
                }
            }
        }

        d
    }
}
```

**Acceptance:** Simple paths (lines only) render correctly.

---

## R05: Implement arc calculation (bulge)

**File:** `src/renderer/path.rs`

**Description:**
Implement arc calculation from bulge factor:

```rust
fn compute_arc_segment(from: Point, to: Point, params: &ArcParams) -> PathSegment {
    match params {
        ArcParams::Bulge(bulge) => {
            if bulge.abs() < 0.001 {
                // Essentially a straight line
                return PathSegment::LineTo(to);
            }

            // Calculate chord length
            let dx = to.x - from.x;
            let dy = to.y - from.y;
            let chord = (dx * dx + dy * dy).sqrt();

            if chord < 0.001 {
                return PathSegment::LineTo(to);
            }

            // Calculate sagitta and radius
            let sagitta = bulge.abs() * chord / 2.0;
            let radius = (chord * chord + 4.0 * sagitta * sagitta) / (8.0 * sagitta);

            PathSegment::ArcTo {
                end: to,
                radius,
                large_arc: false,  // Bulge always uses small arc
                sweep: *bulge > 0.0,  // Positive bulge = clockwise
            }
        }
        ArcParams::Radius { radius, sweep } => {
            compute_radius_arc(from, to, *radius, *sweep)
        }
    }
}
```

**Acceptance:** Arcs with bulge parameter render as expected curves.

---

## R06: Implement arc calculation (radius)

**File:** `src/renderer/path.rs`

**Description:**
Implement arc calculation from radius parameter:

```rust
fn compute_radius_arc(from: Point, to: Point, radius: f64, sweep: SweepDirection) -> PathSegment {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let chord = (dx * dx + dy * dy).sqrt();

    // Clamp radius if chord is too long
    let r = if chord > 2.0 * radius {
        chord / 2.0  // Make it a semicircle
    } else {
        radius
    };

    PathSegment::ArcTo {
        end: to,
        radius: r,
        large_arc: false,  // Always use smaller arc
        sweep: matches!(sweep, SweepDirection::Clockwise),
    }
}
```

**Acceptance:** Arcs with radius parameter render correctly.

---

## R07: Integrate into render_element

**File:** `src/renderer/svg.rs`

**Description:**
Replace the placeholder path rendering with actual implementation:

```rust
ElementType::Shape(ShapeType::Path(path_decl)) => {
    let origin = Point::new(element.bounds.x, element.bounds.y);
    let resolved = crate::renderer::path::resolve_path(path_decl, origin);
    let d = resolved.to_svg_d();

    if d.is_empty() {
        // Empty path - render nothing or placeholder
        return;
    }

    render_shape_with_rotation(element, builder, |b| {
        b.add_path(id, &d, &classes, &styles);
    });
}
```

**Acceptance:** Path shapes render as actual SVG paths instead of placeholders.

---

## R08: Add rendering tests

**File:** `tests/integration_tests.rs`

**Description:**
Add integration tests for path rendering:

```rust
#[test]
fn test_path_triangle_renders() {
    let input = r#"
        path "triangle" [fill: blue, stroke: black] {
            vertex a
            line_to b [x: 50, y: 0]
            line_to c [x: 25, y: 40]
            close
        }
    "#;
    let svg = render(input).expect("should render");
    assert!(svg.contains("<path"), "Should contain path element");
    assert!(svg.contains("d=\"M"), "Should have d attribute with M command");
    assert!(svg.contains(" L"), "Should have L commands for lines");
    assert!(svg.contains(" Z"), "Should have Z command for close");
}

#[test]
fn test_path_with_arc_renders() {
    let input = r#"
        path "curved" {
            vertex a
            arc_to b [x: 50, y: 0, bulge: 0.5]
            close
        }
    "#;
    let svg = render(input).expect("should render");
    assert!(svg.contains(" A"), "Should have A command for arc");
}

#[test]
fn test_path_in_layout_renders() {
    let input = r#"
        row {
            path "shape" { vertex a line_to b [x: 30] }
            rect spacer
        }
    "#;
    let svg = render(input).expect("should render");
    assert!(svg.contains("<path"), "Path in layout should render");
}
```

**Acceptance:** All rendering tests pass.

---

## Dependency Graph

```
R01 (add_path method)
  ↓
R02 (path module structure)
  ↓
R03 (vertex resolution)
  ↓
R04 (line generation) ──────┐
  ↓                         │
R05 (arc bulge) ────────────┤
  ↓                         │
R06 (arc radius) ───────────┤
                            ↓
                    R07 (integration)
                            ↓
                    R08 (tests)
```

## Execution Order

1. R01 + R02 (parallel) - Setup
2. R03 - Vertex resolution
3. R04 - Line generation (test with lines only)
4. R05 + R06 (parallel) - Arc calculations
5. R07 - Full integration
6. R08 - Tests
