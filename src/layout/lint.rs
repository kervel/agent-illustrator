//! Lint engine for detecting layout defects in diagrams.
//!
//! Runs after constraint solving and connection routing to check for
//! mechanical issues: overlapping elements, containment violations,
//! label collisions, and connections crossing elements.

use std::collections::HashSet;
use std::fmt;

use crate::parser::ast::{ConstraintExpr, Document, LayoutType, ShapeType, Statement};

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
}

impl fmt::Display for LintCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LintCategory::Overlap => write!(f, "overlap"),
            LintCategory::Containment => write!(f, "containment"),
            LintCategory::Label => write!(f, "label"),
            LintCategory::Connection => write!(f, "connection"),
            LintCategory::Alignment => write!(f, "alignment"),
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
    check_connections(result, &mut warnings);
    check_alignment(result, &mut warnings);
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
    for elem in &result.root_elements {
        check_overlaps_recursive(elem, None, contains_ids, warnings);
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
    let width = label.text.len() as f64 * (font_size * 0.5);
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
}
