//! Lint engine for detecting layout defects in diagrams.
//!
//! Runs after constraint solving and connection routing to check for
//! mechanical issues: overlapping elements, containment violations,
//! label collisions, and connections crossing elements.

use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::parser::ast::{ConstraintExpr, ConstraintProperty, Document, LayoutType, ShapeType, Statement};

use super::routing::{RoutingMode, MIN_FINAL_SEGMENT_LENGTH};
use super::types::{
    BoundingBox, ElementLayout, ElementType, LabelLayout, LayoutResult, Point,
    TextAnchor,
};

/// A lint warning about a layout defect
#[derive(Debug)]
pub struct LintWarning {
    pub category: LintCategory,
    pub message: String,
}

/// Category of lint defect
#[derive(Debug)]
pub enum LintCategory {
    Overlap,
    Containment,
    Label,
    Connection,
    Alignment,
    RedundantConstant,
    ReducibleBend,
    MissingAnchor,
    Contrast,
    SteepDirect,
    CrowdedLayout,
    OverConstrained,
    LabelOverflow,
}

impl fmt::Display for LintCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LintCategory::Overlap => write!(f, "overlap"),
            LintCategory::Containment => write!(f, "containment"),
            LintCategory::Label => write!(f, "label"),
            LintCategory::Connection => write!(f, "connection"),
            LintCategory::Alignment => write!(f, "alignment"),
            LintCategory::RedundantConstant => write!(f, "redundant-constant"),
            LintCategory::ReducibleBend => write!(f, "reducible-bend"),
            LintCategory::MissingAnchor => write!(f, "missing-anchor"),
            LintCategory::Contrast => write!(f, "contrast"),
            LintCategory::SteepDirect => write!(f, "steep-direct"),
            LintCategory::CrowdedLayout => write!(f, "crowded-layout"),
            LintCategory::OverConstrained => write!(f, "over-constrained"),
            LintCategory::LabelOverflow => write!(f, "label-overflow"),
        }
    }
}

/// Run all lint checks on a completed layout.
pub fn check(result: &LayoutResult, doc: &Document) -> Vec<LintWarning> {
    let mut warnings = Vec::new();
    let contains_ids = collect_contains_ids(doc);
    check_overlaps(result, &contains_ids, &mut warnings);
    check_contains(result, doc, &mut warnings);
    check_labels(result, &mut warnings);
    check_label_element_overlaps(result, &mut warnings);
    check_connections(result, &mut warnings);
    check_label_connection_overlaps(result, &mut warnings);
    check_alignment(result, &mut warnings);
    check_redundant_constants(doc, &mut warnings);
    check_reducible_bends(result, &mut warnings);
    check_missing_anchors(doc, result, &mut warnings);
    check_contrast(result, &mut warnings);
    check_steep_direct(result, &mut warnings);
    check_crowded_layouts(doc, &mut warnings);
    check_over_constrained(result, doc, &mut warnings);
    check_label_overflow(result, &mut warnings);
    warnings
}

/// Display name for an element: its ID if named, or positional path if anonymous.
fn element_display_name(
    elem: &ElementLayout,
    parent_name: Option<&str>,
    child_index: usize,
) -> String {
    if let Some(id) = &elem.id {
        format!("\"{}\"", id.0)
    } else {
        match parent_name {
            Some(parent) => format!("<child #{} of {}>", child_index + 1, parent),
            None => format!("<child #{} of root>", child_index + 1),
        }
    }
}

fn is_text_shape(elem: &ElementLayout) -> bool {
    matches!(elem.element_type, ElementType::Shape(ShapeType::Text { .. }))
}

fn is_opaque(elem: &ElementLayout) -> bool {
    elem.styles.opacity.is_none() || elem.styles.opacity == Some(1.0)
}


// ── Collect contains IDs ──────────────────────────────────────────

/// Scan the document for all element IDs involved in `contains` constraints
/// (both containers and contained elements).
fn collect_contains_ids(doc: &Document) -> HashSet<String> {
    let mut ids = HashSet::new();
    collect_contains_ids_from_stmts(&doc.statements, &mut ids);
    ids
}

fn collect_contains_ids_from_stmts(
    stmts: &[crate::parser::ast::Spanned<Statement>],
    ids: &mut HashSet<String>,
) {
    for stmt in stmts {
        match &stmt.node {
            Statement::Constrain(c) => {
                if let ConstraintExpr::Contains {
                    container,
                    elements,
                    ..
                } = &c.expr
                {
                    ids.insert(container.node.0.clone());
                    for elem in elements {
                        ids.insert(elem.node.0.clone());
                    }
                }
            }
            Statement::Layout(l) => {
                collect_contains_ids_from_stmts(&l.children, ids);
            }
            Statement::Group(g) => {
                collect_contains_ids_from_stmts(&g.children, ids);
            }
            _ => {}
        }
    }
}

// ── FR2: Overlap detection ────────────────────────────────────────

fn check_overlaps(
    result: &LayoutResult,
    contains_ids: &HashSet<String>,
    warnings: &mut Vec<LintWarning>,
) {
    // Check root-level siblings against each other
    check_overlap_siblings(&result.root_elements, None, contains_ids, warnings);

    // Then recurse into each element's children
    for elem in &result.root_elements {
        check_overlaps_recursive(elem, None, contains_ids, warnings);
    }
}

/// Check pairwise overlaps among sibling elements
fn check_overlap_siblings(
    siblings: &[ElementLayout],
    parent_name: Option<&str>,
    contains_ids: &HashSet<String>,
    warnings: &mut Vec<LintWarning>,
) {
    for i in 0..siblings.len() {
        for j in (i + 1)..siblings.len() {
            let a = &siblings[i];
            let b = &siblings[j];

            if !is_opaque(a) || !is_opaque(b) {
                continue;
            }

            if let Some(id) = a.id_str() {
                if contains_ids.contains(id) {
                    continue;
                }
            }
            if let Some(id) = b.id_str() {
                if contains_ids.contains(id) {
                    continue;
                }
            }

            if is_text_shape(a) != is_text_shape(b) {
                continue;
            }

            if a.bounds.intersects(&b.bounds) {
                let overlap_w = a.bounds.right().min(b.bounds.right()) - a.bounds.x.max(b.bounds.x);
                let overlap_h = a.bounds.bottom().min(b.bounds.bottom()) - a.bounds.y.max(b.bounds.y);
                let name_a = element_display_name(a, parent_name, i);
                let name_b = element_display_name(b, parent_name, j);
                warnings.push(LintWarning {
                    category: LintCategory::Overlap,
                    message: format!(
                        "elements {} and {} overlap by {:.0}x{:.0}px",
                        name_a, name_b, overlap_w, overlap_h
                    ),
                });
            }
        }
    }
}

/// Check if a group looks like a resolved template instance.
/// Template resolver prefixes all child IDs with `{parent_id}_`.
/// Also handles the common pattern: `alice` → `<anon>` → `alice_head`, `alice_torso`, ...
fn is_template_instance_group(parent: &ElementLayout) -> bool {
    let id = match &parent.id {
        Some(id) => &id.0,
        None => return false,
    };
    let prefix = format!("{}_", id);

    // Direct children match prefix
    let named_children: Vec<&str> = parent
        .children
        .iter()
        .filter_map(|c| c.id.as_ref().map(|id| id.0.as_str()))
        .collect();
    if !named_children.is_empty() && named_children.iter().all(|c| c.starts_with(&prefix)) {
        return true;
    }

    // Single anonymous wrapper child whose named descendants match prefix
    if parent.children.len() == 1 && parent.children[0].id.is_none() {
        let wrapper = &parent.children[0];
        let grandchildren: Vec<&str> = wrapper
            .children
            .iter()
            .filter_map(|c| c.id.as_ref().map(|id| id.0.as_str()))
            .collect();
        if !grandchildren.is_empty() && grandchildren.iter().all(|c| c.starts_with(&prefix)) {
            return true;
        }
    }

    false
}


fn check_overlaps_recursive(
    parent: &ElementLayout,
    template_prefix: Option<&str>,
    contains_ids: &HashSet<String>,
    warnings: &mut Vec<LintWarning>,
) {
    let parent_name = parent
        .id
        .as_ref()
        .map(|id| id.0.as_str());

    // Determine if this group is a template instance or part of one.
    // A template instance has children prefixed with `{parent_id}_`.
    // Nested groups inside a template inherit the template prefix.
    let current_prefix = if is_template_instance_group(parent) {
        Some(format!("{}_", parent.id.as_ref().unwrap().0))
    } else {
        template_prefix.map(|p| p.to_string())
    };

    // Skip overlap checks if:
    // - Children share a template prefix (constructive overlap in template internals)
    // - Parent is a stack layout (stacks are designed for overlapping)
    let skip_sibling_checks = matches!(parent.element_type, ElementType::Layout(LayoutType::Stack))
        || if let Some(ref pfx) = current_prefix {
            let named_children: Vec<_> = parent
                .children
                .iter()
                .filter_map(|c| c.id.as_ref().map(|id| id.0.as_str()))
                .collect();
            !named_children.is_empty()
                && named_children.iter().all(|id| id.starts_with(pfx.as_str()))
        } else {
            false
        };

    let children = &parent.children;
    if !skip_sibling_checks {
    for i in 0..children.len() {
        for j in (i + 1)..children.len() {
            let a = &children[i];
            let b = &children[j];

            // Skip if either has opacity < 1.0
            if !is_opaque(a) || !is_opaque(b) {
                continue;
            }

            // Skip if either is a contains target/container
            if let Some(id) = a.id_str() {
                if contains_ids.contains(id) {
                    continue;
                }
            }
            if let Some(id) = b.id_str() {
                if contains_ids.contains(id) {
                    continue;
                }
            }

            // Skip text-on-shape: one is text, the other is not
            if is_text_shape(a) != is_text_shape(b) {
                continue;
            }

            if a.bounds.intersects(&b.bounds) {
                let overlap_w = a.bounds.right().min(b.bounds.right())
                    - a.bounds.x.max(b.bounds.x);
                let overlap_h = a.bounds.bottom().min(b.bounds.bottom())
                    - a.bounds.y.max(b.bounds.y);
                let name_a = element_display_name(a, parent_name, i);
                let name_b = element_display_name(b, parent_name, j);
                warnings.push(LintWarning {
                    category: LintCategory::Overlap,
                    message: format!(
                        "elements {} and {} overlap by {:.0}x{:.0}px",
                        name_a, name_b, overlap_w, overlap_h
                    ),
                });
            }
        }
    }
    } // end skip_sibling_checks

    // Recurse into children that have children
    for child in children.iter() {
        if !child.children.is_empty() {
            check_overlaps_recursive(
                child,
                current_prefix.as_deref(),
                contains_ids,
                warnings,
            );
        }
    }
}

// ── FR3: Contains constraint verification ─────────────────────────

fn check_contains(
    result: &LayoutResult,
    doc: &Document,
    warnings: &mut Vec<LintWarning>,
) {
    check_contains_in_stmts(&doc.statements, result, warnings);
}

fn check_contains_in_stmts(
    stmts: &[crate::parser::ast::Spanned<Statement>],
    result: &LayoutResult,
    warnings: &mut Vec<LintWarning>,
) {
    for stmt in stmts {
        match &stmt.node {
            Statement::Constrain(c) => {
                if let ConstraintExpr::Contains {
                    container,
                    elements,
                    padding,
                } = &c.expr
                {
                    let pad = padding.unwrap_or(0.0);
                    if let Some(container_elem) =
                        result.get_element_by_name(&container.node.0)
                    {
                        let cb = container_elem.bounds;
                        for elem_id in elements {
                            if let Some(elem) =
                                result.get_element_by_name(&elem_id.node.0)
                            {
                                let eb = elem.bounds;
                                // Check left edge
                                if cb.x > eb.x - pad {
                                    let overflow = cb.x - (eb.x - pad);
                                    warnings.push(LintWarning {
                                        category: LintCategory::Containment,
                                        message: format!(
                                            "element \"{}\" extends {:.0}px past left edge of container \"{}\"",
                                            elem_id.node.0, overflow, container.node.0
                                        ),
                                    });
                                }
                                // Check right edge
                                if cb.right() < eb.right() + pad {
                                    let overflow = (eb.right() + pad) - cb.right();
                                    warnings.push(LintWarning {
                                        category: LintCategory::Containment,
                                        message: format!(
                                            "element \"{}\" extends {:.0}px past right edge of container \"{}\"",
                                            elem_id.node.0, overflow, container.node.0
                                        ),
                                    });
                                }
                                // Check top edge
                                if cb.y > eb.y - pad {
                                    let overflow = cb.y - (eb.y - pad);
                                    warnings.push(LintWarning {
                                        category: LintCategory::Containment,
                                        message: format!(
                                            "element \"{}\" extends {:.0}px past top edge of container \"{}\"",
                                            elem_id.node.0, overflow, container.node.0
                                        ),
                                    });
                                }
                                // Check bottom edge
                                if cb.bottom() < eb.bottom() + pad {
                                    let overflow = (eb.bottom() + pad) - cb.bottom();
                                    warnings.push(LintWarning {
                                        category: LintCategory::Containment,
                                        message: format!(
                                            "element \"{}\" extends {:.0}px past bottom edge of container \"{}\"",
                                            elem_id.node.0, overflow, container.node.0
                                        ),
                                    });
                                }
                            }
                        }
                    }
                }
            }
            Statement::Layout(l) => {
                check_contains_in_stmts(&l.children, result, warnings);
            }
            Statement::Group(g) => {
                check_contains_in_stmts(&g.children, result, warnings);
            }
            _ => {}
        }
    }
}

// ── FR4: Label overlap detection ──────────────────────────────────

struct LabelInfo {
    owner: String,
    bbox: BoundingBox,
    parent_opacity: Option<f64>,
}

fn estimate_label_bbox(label: &LabelLayout) -> BoundingBox {
    let font_size = label
        .styles
        .as_ref()
        .and_then(|s| s.font_size)
        .unwrap_or(14.0);
    let width = label.text.len() as f64 * (font_size * 0.6);
    let height = font_size;

    let x = match label.anchor {
        TextAnchor::Start => label.position.x,
        TextAnchor::Middle => label.position.x - width / 2.0,
        TextAnchor::End => label.position.x - width,
    };
    let y = label.position.y - height / 2.0;

    BoundingBox::new(x, y, width, height)
}

fn collect_labels_recursive(
    elem: &ElementLayout,
    labels: &mut Vec<LabelInfo>,
) {
    if let Some(label) = &elem.label {
        let owner = elem
            .id
            .as_ref()
            .map(|id| id.0.clone())
            .unwrap_or_else(|| "<anon>".to_string());
        labels.push(LabelInfo {
            owner,
            bbox: estimate_label_bbox(label),
            parent_opacity: elem.styles.opacity,
        });
    }
    // Standalone text elements act like labels for overlap checking
    if is_text_shape(elem) {
        let owner = elem
            .id
            .as_ref()
            .map(|id| id.0.clone())
            .unwrap_or_else(|| "<anon>".to_string());
        labels.push(LabelInfo {
            owner,
            bbox: elem.bounds,
            parent_opacity: elem.styles.opacity,
        });
    }
    for child in &elem.children {
        collect_labels_recursive(child, labels);
    }
}

fn check_labels(result: &LayoutResult, warnings: &mut Vec<LintWarning>) {
    let mut labels = Vec::new();

    // Collect element labels
    for elem in &result.root_elements {
        collect_labels_recursive(elem, &mut labels);
    }

    // Collect connection labels
    for conn in &result.connections {
        if let Some(label) = &conn.label {
            let owner = format!("{}→{}", conn.from_id.0, conn.to_id.0);
            labels.push(LabelInfo {
                owner,
                bbox: estimate_label_bbox(label),
                parent_opacity: None, // connections don't have opacity
            });
        }
    }

    // Check pairs
    for i in 0..labels.len() {
        for j in (i + 1)..labels.len() {
            let a = &labels[i];
            let b = &labels[j];

            // Skip if same owner
            if a.owner == b.owner {
                continue;
            }

            // Skip if either parent has opacity < 1.0
            if let Some(op) = a.parent_opacity {
                if op < 1.0 {
                    continue;
                }
            }
            if let Some(op) = b.parent_opacity {
                if op < 1.0 {
                    continue;
                }
            }

            if a.bbox.intersects(&b.bbox) {
                warnings.push(LintWarning {
                    category: LintCategory::Label,
                    message: format!(
                        "labels on \"{}\" and \"{}\" overlap",
                        a.owner, b.owner
                    ),
                });
            }
        }
    }
}

// ── Label-element edge overlap detection ──────────────────────────

/// Detect labels that straddle the edge of a shape element: the label
/// bbox intersects the element but is NOT fully contained.  A label
/// completely inside a box is fine (looks intentional); one that crosses
/// an edge looks like a placement accident.
fn check_label_element_overlaps(result: &LayoutResult, warnings: &mut Vec<LintWarning>) {
    // Collect all labels with owner info
    let mut labels: Vec<LabelInfo> = Vec::new();
    for elem in &result.root_elements {
        collect_labels_recursive(elem, &mut labels);
    }
    for conn in &result.connections {
        if let Some(label) = &conn.label {
            let owner = format!("{}→{}", conn.from_id.0, conn.to_id.0);
            labels.push(LabelInfo {
                owner,
                bbox: estimate_label_bbox(label),
                parent_opacity: None,
            });
        }
    }

    // Collect all opaque, non-text shape elements
    let mut shapes: Vec<OpaqueElement> = Vec::new();
    for (i, elem) in result.root_elements.iter().enumerate() {
        collect_opaque_elements(elem, None, i, &mut shapes);
    }

    for label in &labels {
        for shape in &shapes {
            // Skip if label belongs to this element (own label inside own box)
            if label.owner == shape.id {
                continue;
            }

            // Skip transparent labels
            if let Some(op) = label.parent_opacity {
                if op < 1.0 {
                    continue;
                }
            }

            // The key check: intersects the edge but NOT fully inside
            if label.bbox.intersects(&shape.bounds)
                && !shape.bounds.contains_bbox(&label.bbox)
            {
                let overlap_w = label.bbox.right().min(shape.bounds.right())
                    - label.bbox.x.max(shape.bounds.x);
                let overlap_h = label.bbox.bottom().min(shape.bounds.bottom())
                    - label.bbox.y.max(shape.bounds.y);
                warnings.push(LintWarning {
                    category: LintCategory::Label,
                    message: format!(
                        "label on \"{}\" straddles the edge of element \"{}\"; \
                         overlaps by {:.0}x{:.0}px",
                        label.owner, shape.id,
                        overlap_w, overlap_h
                    ),
                });
            }
        }
    }
}

// ── FR5: Connection-element intersection ──────────────────────────

/// Check if a line segment intersects an axis-aligned bounding box.
fn line_segment_intersects_bbox(p1: &Point, p2: &Point, bbox: &BoundingBox) -> bool {
    // If either endpoint is inside, it intersects
    if bbox.contains(*p1) || bbox.contains(*p2) {
        return true;
    }

    // Check segment against each of the 4 bbox edges
    let edges = [
        // top edge
        (
            Point::new(bbox.x, bbox.y),
            Point::new(bbox.right(), bbox.y),
        ),
        // bottom edge
        (
            Point::new(bbox.x, bbox.bottom()),
            Point::new(bbox.right(), bbox.bottom()),
        ),
        // left edge
        (
            Point::new(bbox.x, bbox.y),
            Point::new(bbox.x, bbox.bottom()),
        ),
        // right edge
        (
            Point::new(bbox.right(), bbox.y),
            Point::new(bbox.right(), bbox.bottom()),
        ),
    ];

    for (e1, e2) in &edges {
        if segments_intersect(p1, p2, e1, e2) {
            return true;
        }
    }

    false
}

/// Check if two line segments intersect using parametric intersection.
fn segments_intersect(a1: &Point, a2: &Point, b1: &Point, b2: &Point) -> bool {
    let d1x = a2.x - a1.x;
    let d1y = a2.y - a1.y;
    let d2x = b2.x - b1.x;
    let d2y = b2.y - b1.y;

    let denom = d1x * d2y - d1y * d2x;

    if denom.abs() < 1e-10 {
        // Parallel or coincident — skip (conservative: don't report)
        return false;
    }

    let dx = b1.x - a1.x;
    let dy = b1.y - a1.y;

    let t = (dx * d2y - dy * d2x) / denom;
    let u = (dx * d1y - dy * d1x) / denom;

    (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)
}

struct OpaqueElement {
    id: String,
    bounds: BoundingBox,
}

fn is_visual_shape(elem: &ElementLayout) -> bool {
    matches!(elem.element_type, ElementType::Shape(_))
}

fn collect_opaque_elements(
    elem: &ElementLayout,
    parent_name: Option<&str>,
    child_index: usize,
    elements: &mut Vec<OpaqueElement>,
) {
    // Only collect visual shapes (not groups/layouts) that are opaque and non-text
    if is_visual_shape(elem) && !is_text_shape(elem) && is_opaque(elem) {
        let id = if let Some(name) = &elem.id {
            name.0.clone()
        } else {
            element_display_name(elem, parent_name, child_index)
        };
        elements.push(OpaqueElement {
            id,
            bounds: elem.bounds,
        });
    }

    let name = elem.id.as_ref().map(|id| id.0.as_str());
    for (i, child) in elem.children.iter().enumerate() {
        collect_opaque_elements(child, name, i, elements);
    }
}

fn check_connections(result: &LayoutResult, warnings: &mut Vec<LintWarning>) {
    // Collect all opaque, non-text elements
    let mut opaque_elements = Vec::new();
    for (i, elem) in result.root_elements.iter().enumerate() {
        collect_opaque_elements(elem, None, i, &mut opaque_elements);
    }

    for conn in &result.connections {
        let from_id = &conn.from_id.0;
        let to_id = &conn.to_id.0;

        let path_start = match conn.path.first() {
            Some(p) => p,
            None => continue,
        };
        let path_end = match conn.path.last() {
            Some(p) => p,
            None => continue,
        };

        // Skip curved connections: their path stores Bézier control points,
        // not the actual curve.  Line-segment intersection on control points
        // produces false positives.
        if conn.routing_mode == RoutingMode::Curved {
            continue;
        }

        // Track which elements this connection crosses (deduplicate)
        let mut crossed: HashSet<String> = HashSet::new();

        // Check each path segment
        for seg in conn.path.windows(2) {
            let p1 = &seg[0];
            let p2 = &seg[1];

            for oe in &opaque_elements {
                // Skip if already reported for this connection
                if crossed.contains(&oe.id) {
                    continue;
                }

                if line_segment_intersects_bbox(p1, p2, &oe.bounds) {
                    // Skip if the connection starts or ends inside this element —
                    // the connection originates/terminates there, so crossing is expected
                    if oe.bounds.contains(*path_start) || oe.bounds.contains(*path_end) {
                        continue;
                    }

                    crossed.insert(oe.id.clone());
                    warnings.push(LintWarning {
                        category: LintCategory::Connection,
                        message: format!(
                            "connection {}→{} crosses element \"{}\"",
                            from_id, to_id, oe.id
                        ),
                    });
                }
            }
        }
    }
}

// ── Label-connection overlap detection ─────────────────────────────

/// Check if any label (element label, connection label, or standalone text)
/// overlaps with a connection path segment.  This catches labels placed at
/// bend points or too close to connector lines.
fn check_label_connection_overlaps(result: &LayoutResult, warnings: &mut Vec<LintWarning>) {
    // Collect user-level labels only (skip template internals).
    // Template children have IDs like `q_main_g_label`; we skip labels whose
    // owner shares a prefix with a root-level template group.
    let template_prefixes: Vec<String> = result
        .root_elements
        .iter()
        .filter(|e| is_template_instance_group(e))
        .filter_map(|e| e.id.as_ref().map(|id| format!("{}_", id.0)))
        .collect();

    let is_template_internal = |owner: &str| -> bool {
        template_prefixes.iter().any(|pfx| owner.starts_with(pfx))
    };

    // Collect labels from elements + standalone text (skipping template internals)
    let mut labels: Vec<LabelInfo> = Vec::new();
    for elem in &result.root_elements {
        collect_labels_recursive(elem, &mut labels);
    }
    labels.retain(|l| !is_template_internal(&l.owner));

    // Collect connection labels
    for conn in &result.connections {
        if let Some(label) = &conn.label {
            let owner = format!("{}→{}", conn.from_id.0, conn.to_id.0);
            labels.push(LabelInfo {
                owner,
                bbox: estimate_label_bbox(label),
                parent_opacity: None,
            });
        }
    }

    for label in &labels {
        // Skip transparent labels
        if let Some(op) = label.parent_opacity {
            if op < 1.0 {
                continue;
            }
        }

        for conn in &result.connections {
            // Skip curved connections (control points ≠ actual curve)
            if conn.routing_mode == RoutingMode::Curved {
                continue;
            }

            let conn_name = format!("{}→{}", conn.from_id.0, conn.to_id.0);

            // Skip: a connection label overlapping its own connection is expected
            // (the label is placed at the midpoint of the path by design)
            if label.owner == conn_name {
                continue;
            }

            // Skip: label on an element that is an endpoint of this connection
            // (e.g., junction labels at railway switches, pin labels at transistor leads)
            if label.owner == conn.from_id.0 || label.owner == conn.to_id.0 {
                continue;
            }

            for seg in conn.path.windows(2) {
                let p1 = &seg[0];
                let p2 = &seg[1];

                if line_segment_intersects_bbox(p1, p2, &label.bbox) {
                    warnings.push(LintWarning {
                        category: LintCategory::Connection,
                        message: format!(
                            "label on \"{}\" overlaps connection {}",
                            label.owner, conn_name
                        ),
                    });
                    // Only report once per label-connection pair
                    break;
                }
            }
        }
    }
}

// ── FR6: Near-alignment detection ─────────────────────────────────

/// Maximum offset (in px) between connected element centers to consider
/// the connection "almost aligned" on that axis.
const ALIGNMENT_THRESHOLD: f64 = 15.0;

fn check_alignment(result: &LayoutResult, warnings: &mut Vec<LintWarning>) {
    for conn in &result.connections {
        let from = match result.get_element_by_name(&conn.from_id.0) {
            Some(e) => e,
            None => continue,
        };
        let to = match result.get_element_by_name(&conn.to_id.0) {
            Some(e) => e,
            None => continue,
        };

        let from_center = from.bounds.center();
        let to_center = to.bounds.center();
        let dx = (from_center.x - to_center.x).abs();
        let dy = (from_center.y - to_center.y).abs();

        // Skip if already perfectly aligned on either axis
        if dx < 0.5 || dy < 0.5 {
            continue;
        }

        if dy < ALIGNMENT_THRESHOLD && dx > dy * 4.0 {
            // Nearly horizontal — small Y offset
            warnings.push(LintWarning {
                category: LintCategory::Alignment,
                message: format!(
                    "connection {}→{} is nearly horizontal (off by {:.0}px); aligning Y positions would straighten it",
                    conn.from_id.0, conn.to_id.0, dy
                ),
            });
        } else if dx < ALIGNMENT_THRESHOLD && dy > dx * 4.0 {
            // Nearly vertical — small X offset
            warnings.push(LintWarning {
                category: LintCategory::Alignment,
                message: format!(
                    "connection {}→{} is nearly vertical (off by {:.0}px); aligning X positions would straighten it",
                    conn.from_id.0, conn.to_id.0, dx
                ),
            });
        }
    }
}

// ── Redundant constant detection ──────────────────────────────────

/// Display name for a ConstraintProperty (for warning messages).
fn property_display_name(prop: &ConstraintProperty) -> &str {
    match prop {
        ConstraintProperty::X => "x",
        ConstraintProperty::Y => "y",
        ConstraintProperty::Width => "width",
        ConstraintProperty::Height => "height",
        ConstraintProperty::Left => "left",
        ConstraintProperty::Right => "right",
        ConstraintProperty::Top => "top",
        ConstraintProperty::Bottom => "bottom",
        ConstraintProperty::CenterX => "center_x",
        ConstraintProperty::CenterY => "center_y",
        ConstraintProperty::Center => "center",
        ConstraintProperty::AnchorX(name) => name,
        ConstraintProperty::AnchorY(name) => name,
    }
}

/// Collect all `Constant { left, value }` constraints from statements, recursing into groups/layouts.
fn collect_constant_constraints(
    stmts: &[crate::parser::ast::Spanned<Statement>],
    out: &mut Vec<(String, String, f64)>, // (element_display, property_display, value)
) {
    for stmt in stmts {
        match &stmt.node {
            Statement::Constrain(c) => {
                if let ConstraintExpr::Constant { left, value } = &c.expr {
                    // Skip Center (composite property)
                    if left.property.node == ConstraintProperty::Center {
                        continue;
                    }
                    let elem_name = left.element.node.to_string();
                    let prop_name = property_display_name(&left.property.node).to_string();
                    out.push((elem_name, prop_name, *value));
                }
            }
            Statement::Layout(l) => {
                collect_constant_constraints(&l.children, out);
            }
            Statement::Group(g) => {
                collect_constant_constraints(&g.children, out);
            }
            _ => {}
        }
    }
}

fn check_redundant_constants(doc: &Document, warnings: &mut Vec<LintWarning>) {
    let mut constants: Vec<(String, String, f64)> = Vec::new();
    collect_constant_constraints(&doc.statements, &mut constants);

    // Group by (property_name, value_bits) → list of element names
    let mut groups: HashMap<(String, u64), Vec<String>> = HashMap::new();
    for (elem, prop, value) in &constants {
        let key = (prop.clone(), value.to_bits());
        groups.entry(key).or_default().push(elem.clone());
    }

    // Emit warnings for groups with 2+ distinct elements
    for ((prop, value_bits), elements) in &groups {
        // Deduplicate elements (same element could appear multiple times)
        let mut unique: Vec<&String> = Vec::new();
        for e in elements {
            if !unique.contains(&e) {
                unique.push(e);
            }
        }
        if unique.len() < 2 {
            continue;
        }

        let value = f64::from_bits(*value_bits);
        let anchor = &unique[0];
        let rest = &unique[1..];

        let message = if unique.len() == 2 {
            format!(
                "consider \"constrain {}.{} = {}.{}\" instead of repeating the constant {}",
                rest[0], prop, anchor, prop, value
            )
        } else {
            let rest_names: Vec<&str> = rest.iter().map(|s| s.as_str()).collect();
            format!(
                "{} elements ({}) set .{} to the same constant {}; consider relating them to {}.{}",
                unique.len(),
                rest_names.join(", "),
                prop,
                value,
                anchor,
                prop
            )
        };

        warnings.push(LintWarning {
            category: LintCategory::RedundantConstant,
            message,
        });
    }
}

// ── Reducible bend detection ───────────────────────────────────

/// Maximum length of an interior segment (between two bends) to flag
/// as a reducible detour.  Derived from the router's stub length so the
/// two stay in sync.
const REDUCIBLE_BEND_THRESHOLD: f64 = 2.0 * MIN_FINAL_SEGMENT_LENGTH;

fn check_reducible_bends(result: &LayoutResult, warnings: &mut Vec<LintWarning>) {
    for conn in &result.connections {
        if conn.routing_mode != RoutingMode::Orthogonal {
            continue;
        }

        let path = &conn.path;
        // Need at least 4 points (3 segments) to have an interior segment
        if path.len() < 4 {
            continue;
        }

        // Interior segments are indices 1..N-2 (skipping first and last segment)
        let num_segments = path.len() - 1;
        let mut shortest_len = f64::MAX;
        let mut shortest_orientation = "";

        for i in 1..(num_segments - 1) {
            let p1 = &path[i];
            let p2 = &path[i + 1];
            let dx = (p2.x - p1.x).abs();
            let dy = (p2.y - p1.y).abs();
            let len = dx + dy; // Manhattan length for orthogonal segments

            if len < REDUCIBLE_BEND_THRESHOLD && len < shortest_len {
                shortest_len = len;
                shortest_orientation = if dx > dy { "horizontally" } else { "vertically" };
            }
        }

        if shortest_len < f64::MAX {
            warnings.push(LintWarning {
                category: LintCategory::ReducibleBend,
                message: format!(
                    "connection {}→{}: path jogs {:.0}px {} between bends; \
                     moving elements at least {:.0}px further apart {} would eliminate 2 corners",
                    conn.from_id.0, conn.to_id.0,
                    shortest_len, shortest_orientation,
                    shortest_len, shortest_orientation
                ),
            });
        }
    }
}

// ── Missing anchor detection ───────────────────────────────────

/// Maximum dimension (width or height) below which an element is considered
/// too small for explicit anchors to matter — auto-detection works fine.
const SMALL_ELEMENT_THRESHOLD: f64 = 30.0;

fn is_small_element(result: &LayoutResult, name: &str) -> bool {
    if let Some(elem) = result.get_element_by_name(name) {
        elem.bounds.width <= SMALL_ELEMENT_THRESHOLD
            && elem.bounds.height <= SMALL_ELEMENT_THRESHOLD
    } else {
        false
    }
}

/// Look up the solved connection layout for a given from/to pair.
fn find_connection_layout<'a>(result: &'a LayoutResult, from: &str, to: &str) -> Option<&'a super::types::ConnectionLayout> {
    result.connections.iter().find(|c| {
        c.from_id.0 == from && c.to_id.0 == to
    })
}

fn check_missing_anchors(doc: &Document, result: &LayoutResult, warnings: &mut Vec<LintWarning>) {
    check_missing_anchors_in_stmts(&doc.statements, result, warnings);
}

fn check_missing_anchors_in_stmts(
    stmts: &[crate::parser::ast::Spanned<Statement>],
    result: &LayoutResult,
    warnings: &mut Vec<LintWarning>,
) {
    for stmt in stmts {
        match &stmt.node {
            Statement::Connection(connections) => {
                for conn in connections {
                    let from_name = &conn.from.element.node.0;
                    let to_name = &conn.to.element.node.0;

                    // Skip if either endpoint is a small element — anchors
                    // don't improve routing when all anchor positions converge
                    if is_small_element(result, from_name) || is_small_element(result, to_name) {
                        continue;
                    }

                    // Check the solved connection to decide if anchors would help
                    if let Some(solved) = find_connection_layout(result, from_name, to_name) {
                        // Skip direct routing — straight lines don't need anchor guidance
                        if solved.routing_mode == RoutingMode::Direct {
                            continue;
                        }
                        // Skip if the solved path is a straight line (2 points) — this means
                        // auto-detection already found the optimal edges. Anchors only help
                        // when the path has bends (3+ points) that might be avoidable.
                        if solved.path.len() <= 2 {
                            continue;
                        }
                    }

                    if conn.from.anchor.is_none() {
                        warnings.push(LintWarning {
                            category: LintCategory::MissingAnchor,
                            message: format!(
                                "connection {}\u{2192}{}: no explicit anchor on source; \
                                 use e.g. {}.bottom -> {}.top for better routing",
                                from_name, to_name, from_name, to_name
                            ),
                        });
                    }
                    if conn.to.anchor.is_none() {
                        warnings.push(LintWarning {
                            category: LintCategory::MissingAnchor,
                            message: format!(
                                "connection {}\u{2192}{}: no explicit anchor on target; \
                                 use e.g. {}.bottom -> {}.top for better routing",
                                from_name, to_name, from_name, to_name
                            ),
                        });
                    }
                }
            }
            Statement::Layout(l) => {
                check_missing_anchors_in_stmts(&l.children, result, warnings);
            }
            Statement::Group(g) => {
                check_missing_anchors_in_stmts(&g.children, result, warnings);
            }
            _ => {}
        }
    }
}

// ── Contrast detection ─────────────────────────────────────────

/// Check if a CSS variable name refers to a dark fill.
fn is_dark_css_variable(name: &str) -> bool {
    // Dark fills: names containing "-dark", or specific foreground/text tokens
    let dark_patterns = ["-dark", "foreground-1", "foreground-2", "text-dark", "text-1", "text-2"];
    dark_patterns.iter().any(|p| name.contains(p))
}

/// Parse a hex color (#rgb or #rrggbb) and return relative luminance.
fn hex_luminance(hex: &str) -> Option<f64> {
    let hex = hex.trim_start_matches('#');
    let (r, g, b) = match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            (r, g, b)
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            (r, g, b)
        }
        _ => return None,
    };
    // sRGB linearization
    fn linearize(c: u8) -> f64 {
        let s = c as f64 / 255.0;
        if s <= 0.04045 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
    }
    Some(0.2126 * linearize(r) + 0.7152 * linearize(g) + 0.0722 * linearize(b))
}

/// Check if a fill string represents a dark color.
/// Returns Some(description) if dark, None otherwise.
fn is_dark_fill(fill: &str) -> Option<String> {
    // Check for CSS variable: var(--something) or just the token name
    if fill.starts_with("var(--") {
        let var_name = fill.trim_start_matches("var(--").trim_end_matches(')');
        if is_dark_css_variable(var_name) {
            return Some(fill.to_string());
        }
        return None;
    }
    // Check for bare CSS variable token (without var() wrapper)
    if is_dark_css_variable(fill) {
        return Some(fill.to_string());
    }
    // Check for hex color
    if fill.starts_with('#') {
        if let Some(lum) = hex_luminance(fill) {
            if lum < 0.3 {
                return Some(fill.to_string());
            }
        }
    }
    None
}

fn check_contrast(result: &LayoutResult, warnings: &mut Vec<LintWarning>) {
    for elem in &result.root_elements {
        check_contrast_recursive(elem, warnings);
    }
}

fn check_contrast_recursive(elem: &ElementLayout, warnings: &mut Vec<LintWarning>) {
    // Check if this element has a label AND a dark fill AND no explicit label color
    if let Some(label) = &elem.label {
        let has_label_color = label.styles.as_ref().and_then(|s| s.fill.as_ref()).is_some();
        if !has_label_color {
            if let Some(fill) = &elem.styles.fill {
                if let Some(dark_desc) = is_dark_fill(fill) {
                    let name = elem
                        .id
                        .as_ref()
                        .map(|id| format!("\"{}\"", id.0))
                        .unwrap_or_else(|| "<anon>".to_string());
                    warnings.push(LintWarning {
                        category: LintCategory::Contrast,
                        message: format!(
                            "element {} has dark fill ({}) with a label; \
                             label text may be unreadable without CSS overrides for light text",
                            name, dark_desc
                        ),
                    });
                }
            }
        }
    }
    for child in &elem.children {
        check_contrast_recursive(child, warnings);
    }
}

// ── Steep direct connection detection ──────────────────────────

fn check_steep_direct(result: &LayoutResult, warnings: &mut Vec<LintWarning>) {
    for conn in &result.connections {
        if conn.routing_mode != RoutingMode::Direct {
            continue;
        }
        // Only check 2-point (straight line) paths
        if conn.path.len() != 2 {
            continue;
        }
        // Skip small elements — they're in the schematic/graph domain
        // where diagonals are normal, not flowchart territory
        if is_small_element(result, &conn.from_id.0) || is_small_element(result, &conn.to_id.0) {
            continue;
        }
        let p1 = &conn.path[0];
        let p2 = &conn.path[1];
        let dy = p2.y - p1.y;
        let dx = p2.x - p1.x;
        let angle = dy.atan2(dx).abs();

        // Steep diagonal: 30°-60° or 120°-150° (in radians: π/6 to π/3 or 2π/3 to 5π/6)
        let pi_6 = std::f64::consts::FRAC_PI_6;
        let pi_3 = std::f64::consts::FRAC_PI_3;
        let two_pi_3 = 2.0 * std::f64::consts::FRAC_PI_3;
        let five_pi_6 = 5.0 * std::f64::consts::FRAC_PI_6;

        let is_steep = (angle >= pi_6 && angle <= pi_3)
            || (angle >= two_pi_3 && angle <= five_pi_6);

        if is_steep {
            let angle_deg = angle.to_degrees().round() as i32;
            warnings.push(LintWarning {
                category: LintCategory::SteepDirect,
                message: format!(
                    "connection {}\u{2192}{} uses direct routing at {}\u{00b0} angle; \
                     steep diagonals look poor mixed with orthogonal routes \u{2014} \
                     consider routing: orthogonal or routing: curved (ignore if intended)",
                    conn.from_id.0, conn.to_id.0, angle_deg
                ),
            });
        }
    }
}

// ── Crowded layout detection ───────────────────────────────────

fn check_crowded_layouts(doc: &Document, warnings: &mut Vec<LintWarning>) {
    check_crowded_layouts_in_stmts(&doc.statements, warnings);
}

fn check_crowded_layouts_in_stmts(
    stmts: &[crate::parser::ast::Spanned<Statement>],
    warnings: &mut Vec<LintWarning>,
) {
    for stmt in stmts {
        match &stmt.node {
            Statement::Layout(l) => {
                let layout_type = &l.layout_type.node;
                if matches!(layout_type, LayoutType::Row | LayoutType::Column) {
                    // Count direct children that are shapes or groups (not text labels)
                    let child_count = l.children.iter().filter(|c| {
                        matches!(
                            c.node,
                            Statement::Shape(_) | Statement::Group(_) | Statement::Layout(_)
                        )
                    }).count();

                    if child_count > 8 {
                        let layout_name = l.name.as_ref()
                            .map(|n| format!("\"{}\"", n.node.0))
                            .unwrap_or_else(|| "<anon>".to_string());
                        let layout_kind = match layout_type {
                            LayoutType::Row => "row",
                            LayoutType::Column => "col",
                            _ => unreachable!(),
                        };
                        warnings.push(LintWarning {
                            category: LintCategory::CrowdedLayout,
                            message: format!(
                                "{} {} has {} children; for >8 elements, consider using group with constraints instead",
                                layout_kind, layout_name, child_count
                            ),
                        });
                    }
                }
                // Recurse into children
                check_crowded_layouts_in_stmts(&l.children, warnings);
            }
            Statement::Group(g) => {
                check_crowded_layouts_in_stmts(&g.children, warnings);
            }
            _ => {}
        }
    }
}

// ── Over-constrained detection ─────────────────────────────────

/// Resolve a constraint property to its solved value from an element's bounds.
fn resolve_property_value(bounds: &BoundingBox, prop: &ConstraintProperty) -> Option<f64> {
    match prop {
        ConstraintProperty::Left | ConstraintProperty::X => Some(bounds.x),
        ConstraintProperty::Right => Some(bounds.x + bounds.width),
        ConstraintProperty::Top | ConstraintProperty::Y => Some(bounds.y),
        ConstraintProperty::Bottom => Some(bounds.y + bounds.height),
        ConstraintProperty::CenterX => Some(bounds.x + bounds.width / 2.0),
        ConstraintProperty::CenterY => Some(bounds.y + bounds.height / 2.0),
        ConstraintProperty::Width => Some(bounds.width),
        ConstraintProperty::Height => Some(bounds.height),
        ConstraintProperty::Center => None, // composite, skip
        ConstraintProperty::AnchorX(_) | ConstraintProperty::AnchorY(_) => None, // skip custom anchors
    }
}

/// Format a constraint expression for display in warning messages.
fn format_constraint_expr(expr: &ConstraintExpr) -> String {
    match expr {
        ConstraintExpr::Equal { left, right } => {
            format!(
                "{}.{} = {}.{}",
                left.element.node,
                property_display_name(&left.property.node),
                right.element.node,
                property_display_name(&right.property.node)
            )
        }
        ConstraintExpr::EqualWithOffset { left, right, offset } => {
            if *offset >= 0.0 {
                format!(
                    "{}.{} = {}.{} + {}",
                    left.element.node,
                    property_display_name(&left.property.node),
                    right.element.node,
                    property_display_name(&right.property.node),
                    offset
                )
            } else {
                format!(
                    "{}.{} = {}.{} - {}",
                    left.element.node,
                    property_display_name(&left.property.node),
                    right.element.node,
                    property_display_name(&right.property.node),
                    -offset
                )
            }
        }
        ConstraintExpr::Constant { left, value } => {
            format!(
                "{}.{} = {}",
                left.element.node,
                property_display_name(&left.property.node),
                value
            )
        }
        ConstraintExpr::GreaterOrEqual { left, value } => {
            format!(
                "{}.{} >= {}",
                left.element.node,
                property_display_name(&left.property.node),
                value
            )
        }
        ConstraintExpr::LessOrEqual { left, value } => {
            format!(
                "{}.{} <= {}",
                left.element.node,
                property_display_name(&left.property.node),
                value
            )
        }
        _ => String::new(), // Contains, Midpoint — skip
    }
}

fn check_over_constrained(
    result: &LayoutResult,
    doc: &Document,
    warnings: &mut Vec<LintWarning>,
) {
    check_over_constrained_in_stmts(&doc.statements, result, warnings);
}

fn check_over_constrained_in_stmts(
    stmts: &[crate::parser::ast::Spanned<Statement>],
    result: &LayoutResult,
    warnings: &mut Vec<LintWarning>,
) {
    const EPSILON: f64 = 1.0;

    for stmt in stmts {
        match &stmt.node {
            Statement::Constrain(c) => {
                match &c.expr {
                    ConstraintExpr::Equal { left, right } => {
                        let lhs_elem = result.get_element_by_name(&left.element.node.leaf().0);
                        let rhs_elem = result.get_element_by_name(&right.element.node.leaf().0);
                        if let (Some(le), Some(re)) = (lhs_elem, rhs_elem) {
                            if let (Some(lv), Some(rv)) = (
                                resolve_property_value(&le.bounds, &left.property.node),
                                resolve_property_value(&re.bounds, &right.property.node),
                            ) {
                                let residual = (lv - rv).abs();
                                if residual > EPSILON {
                                    let desc = format_constraint_expr(&c.expr);
                                    warnings.push(LintWarning {
                                        category: LintCategory::OverConstrained,
                                        message: format!(
                                            "constraint \"{}\" is violated by {:.0}px; the system may be over-constrained",
                                            desc, residual
                                        ),
                                    });
                                }
                            }
                        }
                    }
                    ConstraintExpr::EqualWithOffset { left, right, offset } => {
                        let lhs_elem = result.get_element_by_name(&left.element.node.leaf().0);
                        let rhs_elem = result.get_element_by_name(&right.element.node.leaf().0);
                        if let (Some(le), Some(re)) = (lhs_elem, rhs_elem) {
                            if let (Some(lv), Some(rv)) = (
                                resolve_property_value(&le.bounds, &left.property.node),
                                resolve_property_value(&re.bounds, &right.property.node),
                            ) {
                                let residual = (lv - (rv + offset)).abs();
                                if residual > EPSILON {
                                    let desc = format_constraint_expr(&c.expr);
                                    warnings.push(LintWarning {
                                        category: LintCategory::OverConstrained,
                                        message: format!(
                                            "constraint \"{}\" is violated by {:.0}px; the system may be over-constrained",
                                            desc, residual
                                        ),
                                    });
                                }
                            }
                        }
                    }
                    ConstraintExpr::Constant { left, value } => {
                        let elem = result.get_element_by_name(&left.element.node.leaf().0);
                        if let Some(e) = elem {
                            if let Some(solved) = resolve_property_value(&e.bounds, &left.property.node) {
                                let residual = (solved - value).abs();
                                if residual > EPSILON {
                                    let desc = format_constraint_expr(&c.expr);
                                    warnings.push(LintWarning {
                                        category: LintCategory::OverConstrained,
                                        message: format!(
                                            "constraint \"{}\" is violated by {:.0}px; the system may be over-constrained",
                                            desc, residual
                                        ),
                                    });
                                }
                            }
                        }
                    }
                    ConstraintExpr::GreaterOrEqual { left, value } => {
                        let elem = result.get_element_by_name(&left.element.node.leaf().0);
                        if let Some(e) = elem {
                            if let Some(solved) = resolve_property_value(&e.bounds, &left.property.node) {
                                if solved < value - EPSILON {
                                    let desc = format_constraint_expr(&c.expr);
                                    let violation = value - solved;
                                    warnings.push(LintWarning {
                                        category: LintCategory::OverConstrained,
                                        message: format!(
                                            "constraint \"{}\" is violated by {:.0}px; the system may be over-constrained",
                                            desc, violation
                                        ),
                                    });
                                }
                            }
                        }
                    }
                    ConstraintExpr::LessOrEqual { left, value } => {
                        let elem = result.get_element_by_name(&left.element.node.leaf().0);
                        if let Some(e) = elem {
                            if let Some(solved) = resolve_property_value(&e.bounds, &left.property.node) {
                                if solved > value + EPSILON {
                                    let desc = format_constraint_expr(&c.expr);
                                    let violation = solved - value;
                                    warnings.push(LintWarning {
                                        category: LintCategory::OverConstrained,
                                        message: format!(
                                            "constraint \"{}\" is violated by {:.0}px; the system may be over-constrained",
                                            desc, violation
                                        ),
                                    });
                                }
                            }
                        }
                    }
                    _ => {} // Contains, Midpoint — skip
                }
            }
            Statement::Layout(l) => {
                check_over_constrained_in_stmts(&l.children, result, warnings);
            }
            Statement::Group(g) => {
                check_over_constrained_in_stmts(&g.children, result, warnings);
            }
            _ => {}
        }
    }
}

// ── Label overflow detection ──────────────────────────────────────

/// Detect labels that are larger than their containing shape.
/// This catches cases like a "+3.3V" label on a 4px-high power rail,
/// where the text visibly overflows the element.
fn check_label_overflow(result: &LayoutResult, warnings: &mut Vec<LintWarning>) {
    for elem in &result.root_elements {
        check_label_overflow_recursive(elem, warnings);
    }
}

fn check_label_overflow_recursive(elem: &ElementLayout, warnings: &mut Vec<LintWarning>) {
    if let Some(label) = &elem.label {
        // Skip text elements — they don't have a "container" to overflow
        if !is_text_shape(elem) {
            let label_bbox = estimate_label_bbox(label);
            let shape_bounds = &elem.bounds;

            // Check if label is wider or taller than the shape
            let width_overflow = label_bbox.width > shape_bounds.width + 2.0;
            let height_overflow = label_bbox.height > shape_bounds.height + 2.0;

            if width_overflow || height_overflow {
                let name = elem
                    .id
                    .as_ref()
                    .map(|id| format!("\"{}\"", id.0))
                    .unwrap_or_else(|| "<anon>".to_string());
                let label_text = &label.text;

                let detail = if width_overflow && height_overflow {
                    format!(
                        "label \"{}\" on {} overflows both width ({:.0}px label vs {:.0}px shape) and height ({:.0}px vs {:.0}px); consider using a separate text element positioned nearby",
                        label_text, name,
                        label_bbox.width, shape_bounds.width,
                        label_bbox.height, shape_bounds.height,
                    )
                } else if width_overflow {
                    format!(
                        "label \"{}\" on {} overflows width ({:.0}px label vs {:.0}px shape); consider making the shape wider or using a separate text element",
                        label_text, name,
                        label_bbox.width, shape_bounds.width,
                    )
                } else {
                    format!(
                        "label \"{}\" on {} overflows height ({:.0}px label vs {:.0}px shape); consider making the shape taller or using a separate text element",
                        label_text, name,
                        label_bbox.height, shape_bounds.height,
                    )
                };

                warnings.push(LintWarning {
                    category: LintCategory::LabelOverflow,
                    message: detail,
                });
            }
        }
    }
    for child in &elem.children {
        check_label_overflow_recursive(child, warnings);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::Identifier;

    fn make_rect(
        id: Option<&str>,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    ) -> ElementLayout {
        ElementLayout {
            id: id.map(|s| Identifier(s.to_string())),
            element_type: ElementType::Shape(ShapeType::Rectangle),
            bounds: BoundingBox::new(x, y, w, h),
            styles: super::super::types::ResolvedStyles::default(),
            children: vec![],
            label: None,
            anchors: super::super::types::AnchorSet::default(),
            path_normalize: false,
        }
    }

    fn make_rect_with_opacity(
        id: Option<&str>,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        opacity: f64,
    ) -> ElementLayout {
        let mut elem = make_rect(id, x, y, w, h);
        elem.styles.opacity = Some(opacity);
        elem
    }

    fn make_text(
        id: Option<&str>,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    ) -> ElementLayout {
        ElementLayout {
            id: id.map(|s| Identifier(s.to_string())),
            element_type: ElementType::Shape(ShapeType::Text {
                content: "text".to_string(),
            }),
            bounds: BoundingBox::new(x, y, w, h),
            styles: super::super::types::ResolvedStyles::default(),
            children: vec![],
            label: None,
            anchors: super::super::types::AnchorSet::default(),
            path_normalize: false,
        }
    }

    fn make_group(
        id: Option<&str>,
        children: Vec<ElementLayout>,
    ) -> ElementLayout {
        // Compute bounds from children
        let mut bounds = BoundingBox::zero();
        for child in &children {
            bounds = bounds.union(&child.bounds);
        }
        ElementLayout {
            id: id.map(|s| Identifier(s.to_string())),
            element_type: ElementType::Group,
            bounds,
            styles: super::super::types::ResolvedStyles::default(),
            children,
            label: None,
            anchors: super::super::types::AnchorSet::default(),
            path_normalize: false,
        }
    }

    // ── Overlap tests ──

    #[test]
    fn test_overlap_detected() {
        let group = make_group(
            Some("g"),
            vec![
                make_rect(Some("a"), 0.0, 0.0, 100.0, 50.0),
                make_rect(Some("b"), 80.0, 0.0, 100.0, 50.0),
            ],
        );
        let mut warnings = Vec::new();
        let contains_ids = HashSet::new();
        check_overlaps_recursive(&group, None, &contains_ids, &mut warnings);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("\"a\""));
        assert!(warnings[0].message.contains("\"b\""));
    }

    #[test]
    fn test_overlap_skipped_for_opacity() {
        let group = make_group(
            Some("g"),
            vec![
                make_rect(Some("a"), 0.0, 0.0, 100.0, 50.0),
                make_rect_with_opacity(Some("bg"), 0.0, 0.0, 200.0, 200.0, 0.2),
            ],
        );
        let mut warnings = Vec::new();
        check_overlaps_recursive(&group, None, &HashSet::new(), &mut warnings);
        assert_eq!(warnings.len(), 0);
    }

    #[test]
    fn test_overlap_skipped_for_contains_target() {
        let group = make_group(
            Some("g"),
            vec![
                make_rect(Some("container"), 0.0, 0.0, 200.0, 200.0),
                make_rect(Some("child"), 10.0, 10.0, 50.0, 50.0),
            ],
        );
        let mut contains_ids = HashSet::new();
        contains_ids.insert("container".to_string());
        contains_ids.insert("child".to_string());
        let mut warnings = Vec::new();
        check_overlaps_recursive(&group, None, &contains_ids, &mut warnings);
        assert_eq!(warnings.len(), 0);
    }

    #[test]
    fn test_overlap_skipped_for_text_on_shape() {
        let group = make_group(
            Some("g"),
            vec![
                make_rect(Some("box"), 0.0, 0.0, 100.0, 50.0),
                make_text(Some("label"), 10.0, 10.0, 80.0, 14.0),
            ],
        );
        let mut warnings = Vec::new();
        check_overlaps_recursive(&group, None, &HashSet::new(), &mut warnings);
        assert_eq!(warnings.len(), 0);
    }

    #[test]
    fn test_no_overlap() {
        let group = make_group(
            Some("g"),
            vec![
                make_rect(Some("a"), 0.0, 0.0, 50.0, 50.0),
                make_rect(Some("b"), 100.0, 0.0, 50.0, 50.0),
            ],
        );
        let mut warnings = Vec::new();
        check_overlaps_recursive(&group, None, &HashSet::new(), &mut warnings);
        assert_eq!(warnings.len(), 0);
    }

    // ── Line segment intersection tests ──

    #[test]
    fn test_segment_crosses_bbox() {
        let bbox = BoundingBox::new(10.0, 10.0, 20.0, 20.0);
        let p1 = Point::new(0.0, 20.0);
        let p2 = Point::new(40.0, 20.0);
        assert!(line_segment_intersects_bbox(&p1, &p2, &bbox));
    }

    #[test]
    fn test_segment_misses_bbox() {
        let bbox = BoundingBox::new(10.0, 10.0, 20.0, 20.0);
        let p1 = Point::new(0.0, 0.0);
        let p2 = Point::new(5.0, 5.0);
        assert!(!line_segment_intersects_bbox(&p1, &p2, &bbox));
    }

    #[test]
    fn test_segment_endpoint_inside() {
        let bbox = BoundingBox::new(10.0, 10.0, 20.0, 20.0);
        let p1 = Point::new(15.0, 15.0);
        let p2 = Point::new(50.0, 50.0);
        assert!(line_segment_intersects_bbox(&p1, &p2, &bbox));
    }

    #[test]
    fn test_segment_both_inside() {
        let bbox = BoundingBox::new(0.0, 0.0, 100.0, 100.0);
        let p1 = Point::new(10.0, 10.0);
        let p2 = Point::new(50.0, 50.0);
        assert!(line_segment_intersects_bbox(&p1, &p2, &bbox));
    }

    // ── Anonymous element display name ──

    #[test]
    fn test_named_element_display() {
        let elem = make_rect(Some("foo"), 0.0, 0.0, 10.0, 10.0);
        assert_eq!(
            element_display_name(&elem, Some("parent"), 0),
            "\"foo\""
        );
    }

    #[test]
    fn test_anonymous_element_display() {
        let elem = make_rect(None, 0.0, 0.0, 10.0, 10.0);
        assert_eq!(
            element_display_name(&elem, Some("group_a"), 1),
            "<child #2 of group_a>"
        );
    }

    // ── Redundant constant tests ──

    use crate::parser::ast::{
        ConstrainDecl, ElementPath, GroupDecl, Span,
    };

    fn make_constant_constraint(elem: &str, prop: ConstraintProperty, value: f64) -> crate::parser::ast::Spanned<Statement> {
        let span: Span = 0..0;
        crate::parser::ast::Spanned::new(
            Statement::Constrain(ConstrainDecl {
                expr: ConstraintExpr::Constant {
                    left: crate::parser::ast::PropertyRef {
                        element: crate::parser::ast::Spanned::new(
                            ElementPath::simple(Identifier(elem.to_string()), span.clone()),
                            span.clone(),
                        ),
                        property: crate::parser::ast::Spanned::new(prop, span.clone()),
                    },
                    value,
                },
            }),
            span,
        )
    }

    fn make_doc(stmts: Vec<crate::parser::ast::Spanned<Statement>>) -> Document {
        Document { statements: stmts }
    }

    #[test]
    fn test_redundant_constants_detected() {
        let doc = make_doc(vec![
            make_constant_constraint("a", ConstraintProperty::CenterY, 200.0),
            make_constant_constraint("b", ConstraintProperty::CenterY, 200.0),
        ]);
        let mut warnings = Vec::new();
        check_redundant_constants(&doc, &mut warnings);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].category.to_string(), "redundant-constant");
        assert!(warnings[0].message.contains("center_y"));
    }

    #[test]
    fn test_different_values_no_warning() {
        let doc = make_doc(vec![
            make_constant_constraint("a", ConstraintProperty::CenterY, 200.0),
            make_constant_constraint("b", ConstraintProperty::CenterY, 300.0),
        ]);
        let mut warnings = Vec::new();
        check_redundant_constants(&doc, &mut warnings);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_different_properties_no_warning() {
        let doc = make_doc(vec![
            make_constant_constraint("a", ConstraintProperty::CenterX, 200.0),
            make_constant_constraint("b", ConstraintProperty::CenterY, 200.0),
        ]);
        let mut warnings = Vec::new();
        check_redundant_constants(&doc, &mut warnings);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_three_elements_one_warning() {
        let doc = make_doc(vec![
            make_constant_constraint("a", ConstraintProperty::CenterX, 100.0),
            make_constant_constraint("b", ConstraintProperty::CenterX, 100.0),
            make_constant_constraint("c", ConstraintProperty::CenterX, 100.0),
        ]);
        let mut warnings = Vec::new();
        check_redundant_constants(&doc, &mut warnings);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("3 elements"));
    }

    #[test]
    fn test_single_element_no_warning() {
        let doc = make_doc(vec![
            make_constant_constraint("a", ConstraintProperty::CenterX, 100.0),
        ]);
        let mut warnings = Vec::new();
        check_redundant_constants(&doc, &mut warnings);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_inside_group() {
        let span: Span = 0..0;
        let doc = make_doc(vec![
            crate::parser::ast::Spanned::new(
                Statement::Group(GroupDecl {
                    name: Some(crate::parser::ast::Spanned::new(Identifier("g".to_string()), span.clone())),
                    children: vec![
                        make_constant_constraint("a", ConstraintProperty::CenterY, 50.0),
                        make_constant_constraint("b", ConstraintProperty::CenterY, 50.0),
                    ],
                    modifiers: vec![],
                    anchors: vec![],
                    is_template_instance: false,
                }),
                span,
            ),
        ]);
        let mut warnings = Vec::new();
        check_redundant_constants(&doc, &mut warnings);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("center_y"));
    }

    // ── Reducible bend tests ──

    use crate::parser::ast::ConnectionDirection;
    use super::super::types::ConnectionLayout;

    fn make_connection(
        from: &str,
        to: &str,
        path: Vec<Point>,
        routing_mode: RoutingMode,
    ) -> ConnectionLayout {
        ConnectionLayout {
            from_id: Identifier(from.to_string()),
            to_id: Identifier(to.to_string()),
            direction: ConnectionDirection::Forward,
            path,
            styles: super::super::types::ResolvedStyles::default(),
            label: None,
            routing_mode,
        }
    }

    fn make_layout_with_connections(connections: Vec<ConnectionLayout>) -> LayoutResult {
        LayoutResult {
            elements: HashMap::new(),
            root_elements: vec![],
            connections,
            bounds: BoundingBox::zero(),
        }
    }

    #[test]
    fn test_reducible_bend_detected() {
        // Path with a short (20px) interior horizontal segment:
        // down 50px, right 20px, down 50px
        let path = vec![
            Point::new(100.0, 100.0),
            Point::new(100.0, 150.0),
            Point::new(120.0, 150.0), // 20px horizontal interior segment
            Point::new(120.0, 200.0),
        ];
        let conn = make_connection("a", "b", path, RoutingMode::Orthogonal);
        let result = make_layout_with_connections(vec![conn]);
        let mut warnings = Vec::new();
        check_reducible_bends(&result, &mut warnings);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].category.to_string(), "reducible-bend");
        assert!(warnings[0].message.contains("20px"), "message: {}", warnings[0].message);
        assert!(warnings[0].message.contains("horizontally"), "message: {}", warnings[0].message);
        assert!(warnings[0].message.contains("further apart"), "message: {}", warnings[0].message);
        assert!(warnings[0].message.contains("at least"), "message: {}", warnings[0].message);
        assert!(warnings[0].message.contains("eliminate 2 corners"), "message: {}", warnings[0].message);
    }

    #[test]
    fn test_long_interior_no_warning() {
        // Path with a long (100px) interior segment — not reducible
        let path = vec![
            Point::new(100.0, 100.0),
            Point::new(100.0, 150.0),
            Point::new(200.0, 150.0), // 100px horizontal interior
            Point::new(200.0, 200.0),
        ];
        let conn = make_connection("a", "b", path, RoutingMode::Orthogonal);
        let result = make_layout_with_connections(vec![conn]);
        let mut warnings = Vec::new();
        check_reducible_bends(&result, &mut warnings);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_straight_path_no_warning() {
        // 2-point straight line — no interior segments
        let path = vec![
            Point::new(100.0, 100.0),
            Point::new(200.0, 100.0),
        ];
        let conn = make_connection("a", "b", path, RoutingMode::Orthogonal);
        let result = make_layout_with_connections(vec![conn]);
        let mut warnings = Vec::new();
        check_reducible_bends(&result, &mut warnings);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_l_shape_no_warning() {
        // 3-point L-shape — no interior segment (only first and last)
        let path = vec![
            Point::new(100.0, 100.0),
            Point::new(100.0, 200.0),
            Point::new(200.0, 200.0),
        ];
        let conn = make_connection("a", "b", path, RoutingMode::Orthogonal);
        let result = make_layout_with_connections(vec![conn]);
        let mut warnings = Vec::new();
        check_reducible_bends(&result, &mut warnings);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_non_orthogonal_skipped() {
        // Same path shape but with Curved routing — should be skipped
        let path = vec![
            Point::new(100.0, 100.0),
            Point::new(100.0, 150.0),
            Point::new(120.0, 150.0),
            Point::new(120.0, 200.0),
        ];
        let conn = make_connection("a", "b", path, RoutingMode::Curved);
        let result = make_layout_with_connections(vec![conn]);
        let mut warnings = Vec::new();
        check_reducible_bends(&result, &mut warnings);
        assert!(warnings.is_empty());
    }

    // ── Label overflow tests ─────────────────────────────────────

    fn make_rect_with_label(
        id: &str,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        label_text: &str,
    ) -> ElementLayout {
        let mut elem = make_rect(Some(id), x, y, w, h);
        elem.label = Some(LabelLayout {
            text: label_text.to_string(),
            position: Point::new(x + w / 2.0, y + h / 2.0),
            anchor: TextAnchor::Middle,
            styles: None,
        });
        elem
    }

    #[test]
    fn test_label_overflow_height() {
        // 4px-high rail with a label — should trigger overflow
        let elem = make_rect_with_label("rail", 0.0, 0.0, 60.0, 4.0, "+3.3V");
        let result = LayoutResult {
            root_elements: vec![elem],
            connections: vec![],
            elements: HashMap::new(),
            bounds: BoundingBox::new(0.0, 0.0, 100.0, 100.0),
        };
        let mut warnings = Vec::new();
        check_label_overflow(&result, &mut warnings);
        assert_eq!(warnings.len(), 1);
        assert!(matches!(warnings[0].category, LintCategory::LabelOverflow));
        assert!(warnings[0].message.contains("overflows height"));
    }

    #[test]
    fn test_label_overflow_width() {
        // Tiny 10px-wide rect with a long label
        let elem = make_rect_with_label("small", 0.0, 0.0, 10.0, 30.0, "Very Long Label Text");
        let result = LayoutResult {
            root_elements: vec![elem],
            connections: vec![],
            elements: HashMap::new(),
            bounds: BoundingBox::new(0.0, 0.0, 100.0, 100.0),
        };
        let mut warnings = Vec::new();
        check_label_overflow(&result, &mut warnings);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("overflows width"));
    }

    #[test]
    fn test_label_fits_no_warning() {
        // Normal-sized rect with a short label — no overflow
        let elem = make_rect_with_label("box", 0.0, 0.0, 140.0, 50.0, "Service");
        let result = LayoutResult {
            root_elements: vec![elem],
            connections: vec![],
            elements: HashMap::new(),
            bounds: BoundingBox::new(0.0, 0.0, 100.0, 100.0),
        };
        let mut warnings = Vec::new();
        check_label_overflow(&result, &mut warnings);
        assert!(warnings.is_empty());
    }
}
