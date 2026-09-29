//! Lines routed through named elements: `path track [through: [a, b, c]]`.
//!
//! A transit-map line must stay on its stations whatever the layout does, so
//! it is not drawn from coordinates: after constraints are solved (and
//! captions placed), each such path is rebuilt through the centres of the
//! elements it names. `routing: metro` keeps every segment horizontal,
//! vertical or at 45 degrees.

use crate::layout::types::{BoundingBox, ElementLayout, ElementType, LayoutResult, Point};
use crate::layout::solver::{ConstraintSource, LayoutConstraint, LayoutProperty, LayoutVariable};
use crate::layout::LayoutError;
use std::collections::{HashMap, HashSet};
use crate::parser::ast::{
    Identifier, LineToDecl, PathBody, PathCommand, ShapeType, Spanned, StyleKey, StyleModifier,
    StyleValue, VertexDecl, VertexPosition,
};

/// The `through:` spec of a path, if it has one.
pub struct ThroughSpec {
    pub names: Vec<String>,
    pub metro: bool,
    /// `spread: even`: stations on one flat run sit at equal distances.
    pub spread: bool,
    pub extend_start: f64,
    pub extend_end: f64,
}

pub fn through_spec(mods: &[Spanned<StyleModifier>]) -> Option<ThroughSpec> {
    let mut names = None;
    let mut metro = false;
    let mut spread = false;
    let mut ext = (0.0, 0.0);
    for m in mods {
        match (&m.node.key.node, &m.node.value.node) {
            (StyleKey::Custom(k), StyleValue::List(items)) if k == "through" => {
                names = Some(
                    items
                        .iter()
                        .filter_map(|i| match &i.node {
                            StyleValue::Identifier(id) => Some(id.0.replace('.', "_")),
                            StyleValue::Keyword(k) => Some(k.clone()),
                            StyleValue::String(s) => Some(s.replace('.', "_")),
                            _ => None,
                        })
                        .collect::<Vec<_>>(),
                );
            }
            (StyleKey::Routing, StyleValue::Identifier(id)) if id.0 == "metro" => metro = true,
            (StyleKey::Routing, StyleValue::Keyword(k)) if k == "metro" => metro = true,
            (StyleKey::Custom(k), StyleValue::Identifier(id)) if k == "spread" => spread = id.0 == "even",
            (StyleKey::Custom(k), StyleValue::Keyword(w)) if k == "spread" => spread = w == "even",
            (StyleKey::Custom(k), StyleValue::Number { value, .. }) if k == "extend" => ext = (*value, *value),
            (StyleKey::Custom(k), StyleValue::Number { value, .. }) if k == "extend_start" => ext.0 = *value,
            (StyleKey::Custom(k), StyleValue::Number { value, .. }) if k == "extend_end" => ext.1 = *value,
            _ => {}
        }
    }
    names.map(|names| ThroughSpec { names, metro, spread, extend_start: ext.0, extend_end: ext.1 })
}

/// Route points through `centres`. Metro: each hop is straight, or one
/// 45-degree run plus one straight run; the diagonal hugs the line's start
/// on the first hop (a branch leaves at once) and its end on every other
/// hop (it rejoins at the last moment).
pub fn route_points(centres: &[Point], metro: bool) -> Vec<Point> {
    if !metro || centres.len() < 2 {
        return centres.to_vec();
    }
    let mut out = vec![centres[0]];
    let n = centres.len();
    for i in 0..n - 1 {
        let (a, b) = (centres[i], centres[i + 1]);
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        if dx.abs() < 0.5 || dy.abs() < 0.5 || (dx.abs() - dy.abs()).abs() < 0.5 {
            out.push(b);
            continue;
        }
        let diag_first = i == 0;
        let bend = if dx.abs() > dy.abs() {
            // Diagonal covers |dy| horizontally; the rest is flat.
            let run = dy.abs() * dx.signum();
            if diag_first {
                Point::new(a.x + run, b.y)
            } else {
                Point::new(b.x - run, a.y)
            }
        } else {
            let run = dx.abs() * dy.signum();
            if diag_first {
                Point::new(b.x, a.y + run)
            } else {
                Point::new(a.x, b.y - run)
            }
        };
        out.push(bend);
        out.push(b);
    }
    out
}

fn extend(points: &mut [Point], start: f64, end: f64) {
    let n = points.len();
    if n < 2 {
        return;
    }
    let push = |from: Point, to: Point, by: f64| -> Point {
        let (dx, dy) = (to.x - from.x, to.y - from.y);
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-9 {
            return to;
        }
        Point::new(to.x + dx / len * by, to.y + dy / len * by)
    };
    if start != 0.0 {
        points[0] = push(points[1], points[0], start);
    }
    if end != 0.0 {
        points[n - 1] = push(points[n - 2], points[n - 1], end);
    }
}

fn path_body(points: &[Point], origin: Point) -> PathBody {
    let pos = |p: &Point| VertexPosition { x: Some(p.x - origin.x), y: Some(p.y - origin.y) };
    let id = |i: usize| Spanned::new(Identifier::new(format!("p{}", i)), 0..0);
    let mut commands = Vec::new();
    for (i, p) in points.iter().enumerate() {
        let cmd = if i == 0 {
            PathCommand::Vertex(VertexDecl { name: id(i), position: Some(pos(p)) })
        } else {
            PathCommand::LineTo(LineToDecl { target: id(i), position: Some(pos(p)) })
        };
        commands.push(Spanned::new(cmd, 0..0));
    }
    PathBody { commands }
}

fn visit(elems: &mut [ElementLayout], f: &mut dyn FnMut(&mut ElementLayout)) {
    for e in elems {
        f(e);
        visit(&mut e.children, f);
    }
}

/// Rebuild every through-path from the current positions of its stations.
pub fn place_through_paths(result: &mut LayoutResult) -> Result<(), LayoutError> {
    let mut plans: Vec<(String, Vec<Point>)> = Vec::new();
    for (id, e) in &result.elements {
        let ElementType::Shape(ShapeType::Path(decl)) = &e.element_type else { continue };
        let Some(spec) = through_spec(&decl.modifiers) else { continue };
        let mut centres = Vec::new();
        for n in &spec.names {
            let Some(el) = result.elements.get(n) else {
                let mut known: Vec<String> = result.elements.keys().cloned().collect();
                known.sort();
                return Err(LayoutError::undefined(n, 0..0, known));
            };
            centres.push(el.bounds.center());
        }
        let mut pts = route_points(&centres, spec.metro);
        extend(&mut pts, spec.extend_start, spec.extend_end);
        plans.push((id.clone(), pts));
    }
    if plans.is_empty() {
        return Ok(());
    }
    for (id, pts) in plans {
        let min_x = pts.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
        let min_y = pts.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
        let max_x = pts.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
        let max_y = pts.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
        let origin = Point::new(min_x, min_y);
        let bounds = BoundingBox { x: min_x, y: min_y, width: max_x - min_x, height: max_y - min_y };
        let body = path_body(&pts, origin);
        visit(&mut result.root_elements, &mut |e| {
            if e.id.as_ref().is_some_and(|i| i.0 == id) {
                if let ElementType::Shape(ShapeType::Path(decl)) = &mut e.element_type {
                    decl.body = body.clone();
                }
                e.bounds = bounds;
            }
        });
    }
    result.rebuild_index();
    result.compute_bounds();
    Ok(())
}

/// Room between the names of two neighbouring stations.
pub const STATION_GAP: f64 = 24.0;

/// One pair of neighbouring stations on a line that share a band: how far
/// apart their centres are, and how far apart their names need them.
pub struct StationPair {
    pub line: String,
    pub a: String,
    pub b: String,
    /// +1 when the line runs left to right, -1 when right to left.
    pub dir: f64,
    /// Distance between the centres, along the line's direction.
    pub dist: f64,
    pub need: f64,
    /// Both stations sit in the same row/column, which spaces them itself.
    pub same_flow: bool,
}

/// How far a station's parts reach either side of its centre, and how tall
/// they stack: the dot and every part of its component (its name, placed
/// later, is centred on it; its measured size is known already).
fn station_extent(result: &LayoutResult, name: &str, parts_of: &HashMap<&str, Vec<&str>>, owner_of: &HashMap<String, String>) -> Option<(f64, f64)> {
    let el = result.elements.get(name)?;
    let mut half = el.bounds.width / 2.0;
    let mut height = el.bounds.height;
    // A station's own `label:` (a circle with its name below it) is its
    // name just as much as a separate caption part is.
    if let Some(l) = &el.label {
        let m = crate::layout::text::measure_runs(&l.rich.lines, l.font_size);
        half = half.max(m.width / 2.0);
        let outside = l
            .placement
            .as_ref()
            .is_some_and(|p| p.position != crate::layout::ShapeLabelPosition::Inside);
        if outside {
            height += m.height;
        }
    }
    if let Some(parts) = owner_of.get(name).and_then(|o| parts_of.get(o.as_str())) {
        for p in parts {
            if *p == name {
                continue;
            }
            if let Some(pe) = result.elements.get(*p) {
                if pe.children.is_empty() {
                    half = half.max(pe.bounds.width / 2.0);
                    height += pe.bounds.height;
                }
            }
        }
    }
    Some((half, height))
}

/// Neighbouring stations on every `through:` line that sit side by side
/// (their components share a horizontal band), in the line's direction.
///
/// `pinned`: while the layout is still being solved only pinned stations
/// are where they will end up, so a line runs left to right unless both its
/// ends are pinned (None: the layout is final, read the direction off it).
pub fn station_pairs(
    result: &LayoutResult,
    owner_of: &HashMap<String, String>,
    flow_parent: &HashMap<String, String>,
    pinned: Option<&HashSet<String>>,
) -> Vec<StationPair> {
    let mut out = Vec::new();
    let mut parts_of: HashMap<&str, Vec<&str>> = HashMap::new();
    for (part, owner) in owner_of {
        parts_of.entry(owner.as_str()).or_default().push(part.as_str());
    }
    // Sorted: the sizes are summed, and HashMap order would change the last
    // bits of every position from run to run.
    for v in parts_of.values_mut() {
        v.sort_unstable();
    }
    let mut ids: Vec<&String> = result.elements.keys().collect();
    ids.sort();
    for id in ids {
        let e = &result.elements[id];
        let ElementType::Shape(ShapeType::Path(decl)) = &e.element_type else { continue };
        let Some(spec) = through_spec(&decl.modifiers) else { continue };
        let centres: Vec<Option<Point>> =
            spec.names.iter().map(|n| result.elements.get(n).map(|e| e.bounds.center())).collect();
        let (Some(Some(first)), Some(Some(last))) = (centres.first(), centres.last()) else { continue };
        // Lines run left to right unless their ends say otherwise; a line
        // that runs mostly up or down is not spaced here.
        if (last.y - first.y).abs() > (last.x - first.x).abs() && (last.x - first.x).abs() > 1.0 {
            continue;
        }
        let ends_known = match pinned {
            None => true,
            Some(p) => {
                p.contains(&spec.names[0]) && p.contains(&spec.names[spec.names.len() - 1])
            }
        };
        let dir = if ends_known && last.x < first.x - 1.0 { -1.0 } else { 1.0 };
        for w in spec.names.windows(2) {
            let (a, b) = (&w[0], &w[1]);
            let (Some(ea), Some(eb)) = (result.elements.get(a), result.elements.get(b)) else { continue };
            let (Some((ha, hta)), Some((hb, htb))) =
                (station_extent(result, a, &parts_of, owner_of), station_extent(result, b, &parts_of, owner_of))
            else {
                continue;
            };
            let (pa, pb) = (ea.bounds.center(), eb.bounds.center());
            // Side by side: their names can meet. A hop that climbs more
            // than the stations are tall (a branch leaving) cannot crowd.
            if (pb.y - pa.y).abs() >= (hta + htb) / 2.0 {
                continue;
            }
            let (ca, cb) = (pa.x, pb.x);
            let need = ha + hb + STATION_GAP;
            let oa = owner_of.get(a).unwrap_or(a);
            let ob = owner_of.get(b).unwrap_or(b);
            let same_flow = flow_parent.get(oa).is_some() && flow_parent.get(oa) == flow_parent.get(ob);
            out.push(StationPair {
                line: id.clone(),
                a: a.clone(),
                b: b.clone(),
                dir,
                dist: (cb - ca) * dir,
                need,
                same_flow,
            });
        }
    }
    out
}

/// Constraints that give neighbouring stations room for their names (the
/// line stretches to fit) and, with `spread: even`, put the stations of each
/// flat run at equal distances. Stations a row or column already spaces,
/// and stations whose position is pinned by a constraint, are left alone.
pub fn station_constraints(
    result: &LayoutResult,
    owner_of: &HashMap<String, String>,
    flow_parent: &HashMap<String, String>,
    pinned_x: &HashSet<String>,
) -> Vec<LayoutConstraint> {
    let mut out = Vec::new();
    let cx = |n: &str| LayoutVariable::new(n, LayoutProperty::CenterX);
    for p in station_pairs(result, owner_of, flow_parent, Some(pinned_x)) {
        if p.same_flow || pinned_x.contains(&p.b) || p.dist >= p.need - 0.5 {
            continue;
        }
        let source = ConstraintSource::intrinsic(format!("{}: room for the names of {} and {}", p.line, p.a, p.b));
        out.push(if p.dir > 0.0 {
            LayoutConstraint::GreaterOrEqualRelational { left: cx(&p.b), right: cx(&p.a), offset: p.need, source }
        } else {
            LayoutConstraint::LessOrEqualRelational { left: cx(&p.b), right: cx(&p.a), offset: -p.need, source }
        });
    }
    // Even spread: every interior station of a flat run sits midway
    // between its neighbours.
    let mut ids: Vec<&String> = result.elements.keys().collect();
    ids.sort();
    for id in ids {
        let e = &result.elements[id];
        let ElementType::Shape(ShapeType::Path(decl)) = &e.element_type else { continue };
        let Some(spec) = through_spec(&decl.modifiers) else { continue };
        if !spec.spread {
            continue;
        }
        let y = |n: &str| result.elements.get(n).map(|e| e.bounds.center().y);
        for w in spec.names.windows(3) {
            let (Some(ya), Some(yb), Some(yc)) = (y(&w[0]), y(&w[1]), y(&w[2])) else { continue };
            if (ya - yb).abs() > 1.0 || (yb - yc).abs() > 1.0 || pinned_x.contains(&w[1]) {
                continue;
            }
            let ob = owner_of.get(&w[1]).unwrap_or(&w[1]);
            if flow_parent.contains_key(ob) {
                continue;
            }
            out.push(LayoutConstraint::Midpoint {
                target: cx(&w[1]),
                a: cx(&w[0]),
                b: cx(&w[2]),
                offset: 0.0,
                source: ConstraintSource::intrinsic(format!("{}: spread: even", id)),
            });
        }
    }
    out
}

/// Child -> the row/column/grid it sits in.
pub fn flow_parents(result: &LayoutResult) -> HashMap<String, String> {
    use crate::parser::ast::LayoutType;
    fn walk(elems: &[ElementLayout], parent: Option<&str>, out: &mut HashMap<String, String>) {
        for e in elems {
            let me = e.id.as_ref().map(|i| i.0.as_str());
            if let (Some(m), Some(p)) = (me, parent) {
                out.insert(m.to_string(), p.to_string());
            }
            let is_flow = matches!(
                e.element_type,
                ElementType::Layout(LayoutType::Row | LayoutType::Column | LayoutType::Grid)
            );
            walk(&e.children, if is_flow { me } else { None }, out);
        }
    }
    let mut out = HashMap::new();
    walk(&result.root_elements, None, &mut out);
    out
}
