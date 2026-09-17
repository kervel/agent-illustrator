//! Lint engine for detecting layout defects in diagrams.
//!
//! Runs after constraint solving and connection routing to check for
//! mechanical issues: overlapping elements, containment violations,
//! label collisions, and connections crossing elements.

use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::parser::ast::{
    ConstraintExpr, ConstraintProperty, Document, LayoutType, ShapeType, Statement,
};

use super::routing::{RoutingMode, MIN_FINAL_SEGMENT_LENGTH};
use super::types::{
    BoundingBox, ElementLayout, ElementType, LabelLayout, LayoutResult, Point, TextAnchor,
};

/// A lint warning about a layout defect
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintWarning {
    pub category: LintCategory,
    pub message: String,
    /// Keyframes in which this defect occurs.  Empty means the defect is
    /// frame-independent (no keyframes, or present in every frame).
    pub frames: Vec<String>,
    /// The two element names this warning is about, sorted, when it is about
    /// a pair. Lets `dedup_warnings` recognise one piece of geometry reported
    /// under two categories.
    pub pair: Option<(String, String)>,
}

impl LintWarning {
    /// Human-readable frame annotation, e.g. ` [frames: step-2, step-3]`.
    /// Empty when the warning is frame-independent.
    pub fn frame_suffix(&self) -> String {
        match self.frames.len() {
            0 => String::new(),
            1 => format!(" [frame: {}]", self.frames[0]),
            n if n <= 3 => format!(" [frames: {}]", self.frames.join(", ")),
            n => format!(
                " [frames: {}, +{} more]",
                self.frames[..3].join(", "),
                n - 3
            ),
        }
    }
}

/// Category of lint defect
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    UnknownModifier,
    OverriddenConstraint,
}

impl LintCategory {
    /// Every category, in the order they are documented.
    pub const ALL: [LintCategory; 15] = [
        LintCategory::Overlap,
        LintCategory::Containment,
        LintCategory::Label,
        LintCategory::Connection,
        LintCategory::Alignment,
        LintCategory::RedundantConstant,
        LintCategory::ReducibleBend,
        LintCategory::MissingAnchor,
        LintCategory::Contrast,
        LintCategory::SteepDirect,
        LintCategory::CrowdedLayout,
        LintCategory::OverConstrained,
        LintCategory::LabelOverflow,
        LintCategory::UnknownModifier,
        LintCategory::OverriddenConstraint,
    ];

    /// Parse a category from its kebab-case name (as printed by `Display`).
    pub fn parse(name: &str) -> Option<LintCategory> {
        LintCategory::ALL
            .into_iter()
            .find(|c| c.to_string() == name)
    }

    /// Comma-separated list of all category names, for error messages.
    pub fn all_names() -> String {
        LintCategory::ALL
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }
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
            LintCategory::UnknownModifier => write!(f, "unknown-modifier"),
            LintCategory::OverriddenConstraint => write!(f, "overridden-constraint"),
        }
    }
}

/// Run all lint checks on a completed layout.
/// If the document contains keyframes, overlap checks run per-frame
/// with hidden elements excluded.
pub fn check(
    result: &LayoutResult,
    doc: &Document,
    config: &crate::layout::LayoutConfig,
) -> Vec<LintWarning> {
    let mut warnings = Vec::new();
    let contains_ids = collect_contains_ids(doc);

    // Keyframe-aware collision detection (Feature 011).
    //
    // Every check that asks "do these two things collide?" is only
    // meaningful for things that are on screen *at the same time*.  With
    // keyframes we therefore run those checks once per frame against that
    // frame's visible set, and merge the results afterwards: a defect that
    // shows up in every frame is reported once without annotation, one that
    // only shows up in some frames is reported once naming those frames.
    let keyframes = super::keyframe::extract_keyframes(doc);
    let frame_states = super::keyframe::compute_frame_states(&keyframes);

    if frame_states.is_empty() {
        check_collisions(result, &contains_ids, &FrameScope::all_visible(), &mut warnings);
    } else {
        let mut per_frame: Vec<(String, Vec<LintWarning>)> = Vec::with_capacity(frame_states.len());
        for state in &frame_states {
            let scope = FrameScope {
                hidden_elements: &state.hidden_elements,
                hidden_connections: &state.hidden_connections,
            };
            // Ask the frame its own question. A keyframe that moves an element
            // changes what collides with what, so checking a later frame's
            // visible set against frame-0 coordinates reports pairs that have
            // moved apart and — worse — misses pairs the keyframe moved
            // together. Falls back to the base layout when a frame cannot be
            // re-solved, which is the old behaviour rather than no check.
            let solved = super::keyframe::resolve_frame_for_static(result, state, doc, config);
            let frame_result = solved.as_ref().unwrap_or(result);
            let mut frame_warnings = Vec::new();
            check_collisions(frame_result, &contains_ids, &scope, &mut frame_warnings);
            // The hand-placed-label rule is a collision question too: text
            // parked in a box that is hidden whenever the text is shown is not
            // sitting on anything.
            check_hand_placed_labels(frame_result, doc, &mut frame_warnings, &scope);
            per_frame.push((state.name.clone(), frame_warnings));
        }
        merge_frame_warnings(per_frame, &mut warnings);
    }

    check_contains(result, doc, &mut warnings);
    check_alignment(result, &mut warnings);
    check_redundant_constants(doc, &mut warnings);
    check_reducible_bends(result, &mut warnings);
    check_missing_anchors(doc, result, &mut warnings);
    check_contrast(result, &mut warnings);
    check_steep_direct(result, &mut warnings);
    check_crowded_layouts(doc, &mut warnings);
    check_over_constrained(result, doc, &mut warnings);
    check_label_overflow(result, &mut warnings);
    check_text_fits_its_box(result, doc, &mut warnings);
    // Without keyframes this is the only pass; with them it runs per frame
    // inside the loop above, where the visible set is known.
    if frame_states.is_empty() {
        check_hand_placed_labels(result, doc, &mut warnings, &FrameScope::all_visible());
    }
    check_label_markup(doc, &mut warnings);
    check_unknown_modifiers(doc, &mut warnings);
    check_unanimatable_transform_keys(doc, &mut warnings);
    check_unknown_colors(doc, &mut warnings);
    check_overridden_constraints(result, &mut warnings);
    dedup_warnings(&mut warnings);
    warnings
}

/// Which elements and connections are on screen for the check currently running.
///
/// Element visibility is resolved by pruning during traversal: hiding a group
/// hides everything inside it, so a subtree is skipped as soon as its root is
/// hidden and nested children need no explicit entry in `hidden_elements`.
pub(crate) struct FrameScope<'a> {
    hidden_elements: &'a HashSet<String>,
    hidden_connections: &'a HashSet<String>,
}

/// An empty scope: nothing is hidden (documents without keyframes).
static NOTHING_HIDDEN: std::sync::OnceLock<HashSet<String>> = std::sync::OnceLock::new();

impl FrameScope<'static> {
    fn all_visible() -> Self {
        let empty = NOTHING_HIDDEN.get_or_init(HashSet::new);
        FrameScope {
            hidden_elements: empty,
            hidden_connections: empty,
        }
    }
}

impl FrameScope<'_> {
    fn hides_element(&self, elem: &ElementLayout) -> bool {
        elem.id
            .as_ref()
            .is_some_and(|id| self.hidden_elements.contains(&id.0))
    }

    fn hides_connection(&self, name: Option<&str>) -> bool {
        name.is_some_and(|n| self.hidden_connections.contains(n))
    }
}

/// All checks whose verdict depends on what is visible at the same moment.
fn check_collisions(
    result: &LayoutResult,
    contains_ids: &ContainsRelations,
    scope: &FrameScope<'_>,
    warnings: &mut Vec<LintWarning>,
) {
    check_overlaps(result, contains_ids, scope, warnings);
    check_labels(result, scope, warnings);
    check_label_element_overlaps(result, scope, warnings);
    check_connections(result, scope, warnings);
    check_label_connection_overlaps(result, scope, warnings);
    check_near_misses(result, scope, warnings);
}

/// Collapse per-frame warnings into one warning per distinct defect.
///
/// A defect seen in every frame is frame-independent and reported bare; one
/// seen in a subset carries the frame names so the reader knows where to look.
fn merge_frame_warnings(
    per_frame: Vec<(String, Vec<LintWarning>)>,
    out: &mut Vec<LintWarning>,
) {
    let frame_count = per_frame.len();
    // Preserve first-seen order; group by (category, message).
    let mut order: Vec<(LintCategory, String)> = Vec::new();
    let mut frames_by_defect: HashMap<(LintCategory, String), Vec<String>> = HashMap::new();

    for (frame_name, frame_warnings) in per_frame {
        let mut seen_this_frame: HashSet<(LintCategory, String)> = HashSet::new();
        for w in frame_warnings {
            let key = (w.category, w.message.clone());
            if !seen_this_frame.insert(key.clone()) {
                continue;
            }
            let entry = frames_by_defect.entry(key.clone()).or_insert_with(|| {
                order.push(key.clone());
                Vec::new()
            });
            entry.push(frame_name.clone());
        }
    }

    for key in order {
        let frames = frames_by_defect.remove(&key).unwrap_or_default();
        let (category, message) = key;
        out.push(LintWarning {
            category,
            message,
            frames: if frames.len() == frame_count {
                Vec::new()
            } else {
                frames
            },
            pair: None,
        });
    }
}

/// Drop exact duplicates, keeping the first occurrence.
fn dedup_warnings(warnings: &mut Vec<LintWarning>) {
    let mut seen: HashSet<(LintCategory, String)> = HashSet::new();
    warnings.retain(|w| seen.insert((w.category, w.message.clone())));

    // The same geometry reported twice — once as an overlap, once as a label
    // straddle — reads as two problems. Keep the label one: it names the text.
    let labelled: HashSet<(String, String)> = warnings
        .iter()
        .filter(|w| matches!(w.category, LintCategory::Label))
        .filter_map(|w| w.pair.clone())
        .collect();
    warnings.retain(|w| {
        !(matches!(w.category, LintCategory::Overlap)
            && w.pair.as_ref().is_some_and(|p| labelled.contains(p)))
    });
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
    matches!(
        elem.element_type,
        ElementType::Shape(ShapeType::Text { .. })
    )
}

/// Callouts are annotation pins: they are meant to overlap the content they
/// point at, so they are exempt from overlap/label-straddle checks.
fn is_callout(elem: &ElementLayout) -> bool {
    matches!(
        elem.element_type,
        ElementType::Shape(ShapeType::Callout { .. })
    )
}

fn is_opaque(elem: &ElementLayout) -> bool {
    elem.styles.opacity.is_none() || elem.styles.opacity == Some(1.0)
}

/// Grid cells exist only so `g.cell(r, c)` has something to address; nothing
/// is ever drawn for them.  A cell cannot collide with anything — least of all
/// with the child that was placed in it.
fn is_reference_only(elem: &ElementLayout) -> bool {
    matches!(elem.element_type, ElementType::GridCell)
}

/// A layout container with no fill and no border paints nothing: it renders as
/// a bare `<g>` and only describes a region. Something drawn *inside* that
/// region has not collided with anything — a rule laid across a grid, a
/// highlight over a column. Two regions genuinely crossing each other, or a
/// shape sticking out of one, are still reported.
fn is_bare_container(elem: &ElementLayout) -> bool {
    matches!(
        elem.element_type,
        ElementType::Layout(_) | ElementType::Group
    ) && elem.styles.fill.is_none()
        && elem.styles.fill_pattern.is_none()
        && elem
            .styles
            .stroke
            .as_ref()
            .is_none_or(|s| s.eq_ignore_ascii_case("none"))
}

/// Aspect ratio at which a shape stops being a box and becomes a rule.
///
/// Well clear of ordinary shapes on purpose. A 3:1 card must not qualify, or
/// this becomes the blanket exemption it exists to avoid.
const RULE_ASPECT: f64 = 20.0;

/// True for a long thin shape: an axis, a baseline, a vertical now-marker.
///
/// These are drawn ACROSS a diagram, so intersecting what they cross is their
/// function rather than a defect. A 2px rule over a 560px bar, or a 16px pin
/// centred on a 2px axis, is the picture working.
///
/// Measured on a real 8-diagram deck that was correct and shipped: 25 overlap
/// findings, all of this shape, none actionable. That is not a signal-to-noise
/// problem but a category nobody can leave switched on — and the deck did
/// switch it off, which then hid a rule drawn through a bar's name and a
/// caption grazing a card, because the near-miss rule reports under the same
/// category.
fn is_rule_like(elem: &ElementLayout) -> bool {
    if is_text_shape(elem) || !is_visual_shape(elem) {
        return false;
    }
    let (w, h) = (elem.bounds.width, elem.bounds.height);
    if w <= 0.0 || h <= 0.0 {
        return false;
    }
    w / h >= RULE_ASPECT || h / w >= RULE_ASPECT
}

/// True when a pair's intersection is a rule doing its job.
///
/// Requires the rule to CROSS: its long axis has to span past both edges of
/// the other element. A rule that runs the full height of a bar is crossing
/// it; one that stops halfway, or clips a corner, is a layout accident and
/// still worth reporting.
///
/// Text is never exempt: a rule drawn through a label is exactly the defect
/// this category exists to catch, and it is the one that shipped while the
/// category was excluded.
fn is_structural_crossing(a: &ElementLayout, b: &ElementLayout) -> bool {
    if is_text_shape(a) || is_text_shape(b) {
        return false;
    }
    crosses(a, b) || crosses(b, a)
}

/// Does `rule` span clear past `other` along its own long axis?
fn crosses(rule: &ElementLayout, other: &ElementLayout) -> bool {
    if !is_rule_like(rule) {
        return false;
    }
    let (r, o) = (&rule.bounds, &other.bounds);
    if r.width >= r.height {
        // Horizontal rule: must reach past both the left and right edges.
        r.x <= o.x && r.right() >= o.right()
    } else {
        r.y <= o.y && r.bottom() >= o.bottom()
    }
}

/// True when an element paints nothing at all.
///
/// `fill: none` (or no fill on a shape that defaults to none, or a zero fill
/// opacity) plus no visible stroke means the element renders no ink. It marks
/// out a region — a canvas extent, a tick anchor, a spacer — and a region
/// cannot collide with what sits in it, nor can text be said to sit "inside"
/// it in any way an author could fix by moving the text into its label.
///
/// This is deliberately narrow. A *translucent* zone is still visible and
/// still collides; only genuinely invisible elements are exempt. An element
/// exempted here is exempt from LINT only — it keeps participating in layout
/// exactly as before, which matters because invisible tick anchors are
/// routinely the load-bearing geometry other elements are sized from.
pub(crate) fn paints_nothing(elem: &ElementLayout) -> bool {
    !has_visible_fill(elem) && !has_visible_border(elem)
}

/// Counterpart to [`has_visible_border`]: does this element paint any fill?
fn has_visible_fill(elem: &ElementLayout) -> bool {
    if elem.styles.fill_pattern.is_some() {
        return true;
    }
    if let Some(fo) = elem.styles.fill_opacity {
        if fo <= 0.0 {
            return false;
        }
    }
    match elem.styles.fill.as_deref() {
        None => false,
        Some(f) if f.eq_ignore_ascii_case("none") => false,
        Some(_) => true,
    }
}

/// True when one of the pair is a bare region wholly containing the other.
fn is_drawn_inside_a_bare_region(a: &ElementLayout, b: &ElementLayout) -> bool {
    (is_bare_container(a) && a.bounds.contains_bbox(&b.bounds))
        || (is_bare_container(b) && b.bounds.contains_bbox(&a.bounds))
}

/// Name an anonymous element by the grid cell it sits in, if it sits in one.
/// `<child #12 of deling>` says nothing; `cell [3,1] of deling` says where to look.
fn grid_cell_name(elem: &ElementLayout, siblings: &[&ElementLayout]) -> Option<String> {
    if elem.id.is_some() {
        return None; // it has a name of its own
    }
    let center = elem.bounds.center();
    siblings
        .iter()
        .find(|s| is_reference_only(s) && s.bounds.contains(center))
        .and_then(|cell| cell.id.as_ref())
        .and_then(|id| parse_grid_cell_id(&id.0))
        .map(|(grid, row, col)| format!("cell [{},{}] of {}", row, col, grid))
}

/// Split a generated cell id back into (grid, row, col).
fn parse_grid_cell_id(id: &str) -> Option<(&str, &str, &str)> {
    let (grid, coords) = id.split_once("__cell_")?;
    let (row, col) = coords.split_once('_')?;
    Some((grid, row, col))
}

/// More lenient visibility check for connection-crossing detection:
/// elements with opacity >= 0.5 are visible enough to cause visual overlap
/// with connections passing through them.
fn is_substantially_visible(elem: &ElementLayout) -> bool {
    match elem.styles.opacity {
        None => true,
        Some(o) => o >= 0.5,
    }
}

/// A non-opaque shape is "borderless" if it has no visible stroke
/// (stroke is "none" or stroke_width is 0).
fn has_visible_border(elem: &ElementLayout) -> bool {
    if let Some(ref stroke) = elem.styles.stroke {
        if stroke.eq_ignore_ascii_case("none") {
            return false;
        }
    }
    if let Some(sw) = elem.styles.stroke_width {
        if sw <= 0.0 {
            return false;
        }
    }
    // Default stroke is visible
    true
}

/// Check whether a text-on-shape overlap should be flagged.
/// Fully inside or fully outside → OK.
/// Straddling the edge → flag it, UNLESS the shape is semi-transparent
/// and has no visible border (pure background zone).
fn is_text_shape_straddle(text: &ElementLayout, shape: &ElementLayout) -> bool {
    let text_inside = shape.bounds.contains_bbox(&text.bounds);
    let text_outside = !text.bounds.intersects(&shape.bounds);
    if text_inside || text_outside {
        return false; // fully in or fully out → fine
    }
    // Straddling. Allow only if the shape is a borderless transparent zone.
    if !is_opaque(shape) && !has_visible_border(shape) {
        return false;
    }
    true
}

// ── Collect contains IDs ──────────────────────────────────────────

/// Scan the document for all element IDs involved in `contains` constraints
/// (both containers and contained elements).
/// Which elements a `contains` constraint wraps, per container.
///
/// A container overlapping its own contents is the whole point of `contains`,
/// so that pair is exempt. Everything else is not: being wrapped by a box does
/// not stop an element from colliding with the rest of the diagram, and the
/// container itself can still land on something unrelated.
#[derive(Default)]
pub(crate) struct ContainsRelations {
    by_container: HashMap<String, HashSet<String>>,
}

impl ContainsRelations {
    /// True when one of these two wraps the other.
    fn wraps(&self, a: Option<&str>, b: Option<&str>) -> bool {
        let (Some(a), Some(b)) = (a, b) else {
            return false;
        };
        self.by_container.get(a).is_some_and(|c| c.contains(b))
            || self.by_container.get(b).is_some_and(|c| c.contains(a))
    }

}

fn collect_contains_ids(doc: &Document) -> ContainsRelations {
    let mut relations = ContainsRelations::default();
    collect_contains_ids_from_stmts(&doc.statements, &mut relations);
    relations
}

fn collect_contains_ids_from_stmts(
    stmts: &[crate::parser::ast::Spanned<Statement>],
    relations: &mut ContainsRelations,
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
                    let entry = relations
                        .by_container
                        .entry(container.node.0.clone())
                        .or_default();
                    for elem in elements {
                        entry.insert(elem.node.0.clone());
                    }
                }
            }
            Statement::Layout(l) => {
                collect_contains_ids_from_stmts(&l.children, relations);
            }
            Statement::Group(g) => {
                collect_contains_ids_from_stmts(&g.children, relations);
            }
            _ => {}
        }
    }
}

// ── FR2: Overlap detection ────────────────────────────────────────

/// Distance at which text is close enough to a visible edge to read as
/// struck through.
///
/// Deliberately tiny. The defect is glyphs touching a line, not glyphs near
/// one; a generous threshold would fire on every caption in a dense figure and
/// make the category unusable, which is exactly how `overlap` became something
/// authors excluded wholesale.
const NEAR_MISS_EPSILON: f64 = 2.0;

/// Report text whose box stops just short of a visible edge.
///
/// `overlap` and `label` both ask whether two boxes intersect. Text abutting a
/// 2px rule at zero overlap answers no, so nothing fires — and on screen the
/// rule runs through the words. This is the one collision class that survived
/// every other rule in the friction report: a caption resting on an axis, and
/// a vertical rule drawn through a bar's name that reached a screenshot
/// instead of the linter.
///
/// Scoped narrowly on purpose: only TEXT, only against something that paints,
/// only when the two do not already intersect (that is the overlap rule's
/// finding, and naming one defect twice is what taught authors to stop
/// reading lint output).
fn check_near_misses(result: &LayoutResult, scope: &FrameScope<'_>, warnings: &mut Vec<LintWarning>) {
    let mut elements: Vec<OpaqueElement> = Vec::new();
    for (i, elem) in result.root_elements.iter().enumerate() {
        collect_visible_elements(elem, None, i, scope, &mut elements);
    }

    let mut texts: Vec<(String, BoundingBox)> = Vec::new();
    fn collect_texts(
        elem: &ElementLayout,
        scope: &FrameScope<'_>,
        out: &mut Vec<(String, BoundingBox)>,
    ) {
        if scope.hides_element(elem) {
            return;
        }
        if is_text_shape(elem) && !paints_nothing(elem) {
            if let Some(id) = &elem.id {
                out.push((id.0.clone(), elem.bounds));
            }
        }
        for child in &elem.children {
            collect_texts(child, scope, out);
        }
    }
    for elem in &result.root_elements {
        collect_texts(elem, scope, &mut texts);
    }

    for (text_id, tb) in &texts {
        for other in &elements {
            if &other.id == text_id {
                continue;
            }
            // An actual intersection is the overlap rule's to report.
            if tb.intersects(&other.bounds) {
                continue;
            }
            let gap_x = (other.bounds.x - tb.right()).max(tb.x - other.bounds.right());
            let gap_y = (other.bounds.y - tb.bottom()).max(tb.y - other.bounds.bottom());
            // Close on one axis while genuinely spanning the other: a rule
            // that runs past the text, not a neighbour beside it.
            let grazes_horizontally =
                gap_y <= NEAR_MISS_EPSILON && gap_y >= -NEAR_MISS_EPSILON && gap_x < 0.0;
            let grazes_vertically =
                gap_x <= NEAR_MISS_EPSILON && gap_x >= -NEAR_MISS_EPSILON && gap_y < 0.0;
            if !(grazes_horizontally || grazes_vertically) {
                continue;
            }
            warnings.push(LintWarning {
                category: LintCategory::Overlap,
                message: format!(
                    "text \"{}\" grazes the edge of \"{}\"; at this distance the glyphs read as \
                     struck through — move it clear or give it a gap",
                    text_id, other.id
                ),
                frames: Vec::new(),
                pair: Some(sorted_pair(text_id, &other.id)),
            });
        }
    }
}

fn check_overlaps(
    result: &LayoutResult,
    contains_ids: &ContainsRelations,
    scope: &FrameScope<'_>,
    warnings: &mut Vec<LintWarning>,
) {
    // Filter out hidden elements for keyframe-aware overlap detection
    let visible_roots: Vec<&ElementLayout> = result
        .root_elements
        .iter()
        .filter(|e| !scope.hides_element(e))
        .collect();

    // Collect references for sibling check
    let visible_refs: Vec<ElementLayout> = visible_roots.iter().map(|e| (*e).clone()).collect();
    check_overlap_siblings(&visible_refs, None, contains_ids, scope, warnings);

    // Then recurse into each visible element's children
    for elem in &visible_roots {
        check_overlaps_recursive(elem, None, contains_ids, scope, warnings);
    }
}

/// Do these two elements overlap in a way worth reporting?
///
/// Returns the overlapping width and height when they do. The exemptions live
/// here so every caller — siblings, nested children, and a shape checked
/// against the contents of a region — judges a pair the same way.
fn reportable_overlap(
    a: &ElementLayout,
    b: &ElementLayout,
    contains_ids: &ContainsRelations,
) -> Option<(f64, f64)> {
    // Reference-only grid cells are never drawn; ignore them.
    if is_reference_only(a) || is_reference_only(b) {
        return None;
    }

    // Callouts are annotation pins; overlapping their target is intended.
    if is_callout(a) || is_callout(b) {
        return None;
    }

    // Two transparent zones.
    if !is_opaque(a) && !is_opaque(b) {
        return None;
    }

    // For two non-text shapes, skip if either is non-opaque (zone background)
    if !is_text_shape(a) && !is_text_shape(b) && (!is_opaque(a) || !is_opaque(b)) {
        return None;
    }

    // A `contains` container sitting over its own contents is the point.
    if contains_ids.wraps(a.id_str(), b.id_str()) {
        return None;
    }

    // Nothing else about a `contains` container is special: whether it is a
    // backdrop to rest on or a box that hides what it covers is decided by
    // its opacity, a few lines up — a background zone is drawn see-through
    // (the idiom the docs give is `opacity: 0.3`) and is already exempt,
    // while a solid one really does cover what lands under it.

    // Text-on-shape: only flag if the text straddles the edge
    if is_text_shape(a) != is_text_shape(b) {
        let (text, shape) = if is_text_shape(a) { (a, b) } else { (b, a) };
        if !is_text_shape_straddle(text, shape) {
            return None;
        }
    }

    if !a.bounds.intersects(&b.bounds) {
        return None;
    }
    Some((
        a.bounds.right().min(b.bounds.right()) - a.bounds.x.max(b.bounds.x),
        a.bounds.bottom().min(b.bounds.bottom()) - a.bounds.y.max(b.bounds.y),
    ))
}

fn overlap_warning(name_a: &str, name_b: &str, w: f64, h: f64) -> LintWarning {
    LintWarning {
        category: LintCategory::Overlap,
        message: format!(
            "elements {} and {} overlap by {:.0}x{:.0}px",
            name_a, name_b, w, h
        ),
        frames: Vec::new(),
        pair: Some(sorted_pair(name_a, name_b)),
    }
}

/// The two names a pair-warning is about, quotes stripped and order-independent
/// so the same geometry reported by two checks compares equal.
fn sorted_pair(a: &str, b: &str) -> (String, String) {
    let clean = |s: &str| s.trim_matches('"').to_string();
    let (a, b) = (clean(a), clean(b));
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

/// Check an element against what a bare region actually draws.
///
/// A container that paints nothing is not itself something to collide with,
/// but its contents are — and they sit one level down, where the sibling
/// checks never look. Without this, a shape landing on a grid's digits is
/// reported against neither the grid nor the digit.
fn check_against_region_contents(
    outside: &ElementLayout,
    outside_name: &str,
    region: &ElementLayout,
    contains_ids: &ContainsRelations,
    scope: &FrameScope<'_>,
    warnings: &mut Vec<LintWarning>,
) {
    if scope.hides_element(region) {
        return;
    }
    let siblings: Vec<&ElementLayout> = region.children.iter().collect();
    for (i, child) in region.children.iter().enumerate() {
        if scope.hides_element(child) {
            continue;
        }
        // Descend through nested bare regions to the things that are drawn.
        if is_bare_container(child) {
            check_against_region_contents(
                outside,
                outside_name,
                child,
                contains_ids,
                scope,
                warnings,
            );
            continue;
        }
        if let Some((w, h)) = reportable_overlap(outside, child, contains_ids) {
            let child_name = grid_cell_name(child, &siblings).unwrap_or_else(|| {
                element_display_name(child, region.id.as_ref().map(|id| id.0.as_str()), i)
            });
            warnings.push(overlap_warning(outside_name, &child_name, w, h));
        }
    }
}

/// Check pairwise overlaps among sibling elements
fn check_overlap_siblings(
    siblings: &[ElementLayout],
    parent_name: Option<&str>,
    contains_ids: &ContainsRelations,
    scope: &FrameScope<'_>,
    warnings: &mut Vec<LintWarning>,
) {
    let refs: Vec<&ElementLayout> = siblings.iter().collect();
    for i in 0..siblings.len() {
        for j in (i + 1)..siblings.len() {
            let a = &siblings[i];
            let b = &siblings[j];

            let name = |elem: &ElementLayout, index: usize| {
                grid_cell_name(elem, &refs)
                    .unwrap_or_else(|| element_display_name(elem, parent_name, index))
            };

            // An element that paints nothing cannot collide with anything — not
            // with the other party, and not with whatever that party draws one
            // level down either. This must come before the bare-region descent
            // below, which picks one side as "the region" and walks into the
            // other's children: for an invisible *shape* (not a Layout/Group)
            // that picked the wrong side and reported every child of a row
            // against the canvas it sits on.
            if paints_nothing(a) || paints_nothing(b) {
                continue;
            }

            // A rule crossing what it rules over is the picture working.
            if is_structural_crossing(a, b) {
                continue;
            }

            // A container that paints nothing is not something to collide
            // with — but what it draws one level down is.
            if is_drawn_inside_a_bare_region(a, b) {
                let (region, outside, outside_index) =
                    if is_bare_container(a) { (a, b, j) } else { (b, a, i) };
                check_against_region_contents(
                    outside,
                    &name(outside, outside_index),
                    region,
                    contains_ids,
                    scope,
                    warnings,
                );
                continue;
            }

            if let Some((w, h)) = reportable_overlap(a, b, contains_ids) {
                warnings.push(overlap_warning(&name(a, i), &name(b, j), w, h));
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

    // Direct children match prefix. Generated grid cells are skipped: their
    // ids are `{grid}__cell_r_c`, which would otherwise make every named grid
    // look like a template instance and silence its overlap checks.
    let named_children: Vec<&str> = parent
        .children
        .iter()
        .filter(|c| !is_reference_only(c))
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
    contains_ids: &ContainsRelations,
    scope: &FrameScope<'_>,
    warnings: &mut Vec<LintWarning>,
) {
    let parent_name = parent.id.as_ref().map(|id| id.0.as_str());

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
                .filter(|c| !is_reference_only(c))
                .filter_map(|c| c.id.as_ref().map(|id| id.0.as_str()))
                .collect();
            !named_children.is_empty()
                && named_children.iter().all(|id| id.starts_with(pfx.as_str()))
        } else {
            false
        };

    // Hidden children take their whole subtree out of this frame's checks.
    // Original child indices are kept so anonymous elements get the same
    // display name in every frame (and dedup can match them up).
    let children: Vec<(usize, &ElementLayout)> = parent
        .children
        .iter()
        .enumerate()
        .filter(|(_, c)| !scope.hides_element(c))
        .collect();
    // Reference-only cells are kept for naming but never take part in a pair.
    let all_children: Vec<&ElementLayout> = parent.children.iter().collect();
    if !skip_sibling_checks {
        for i in 0..children.len() {
            for j in (i + 1)..children.len() {
                let (index_a, a) = children[i];
                let (index_b, b) = children[j];

                let name = |elem: &ElementLayout, index: usize| {
                    grid_cell_name(elem, &all_children)
                        .unwrap_or_else(|| element_display_name(elem, parent_name, index))
                };

                // An element that paints nothing cannot collide with anything — not
                // with the other party, and not with whatever that party draws one
                // level down either. This must come before the bare-region descent
                // below, which picks one side as "the region" and walks into the
                // other's children: for an invisible *shape* (not a Layout/Group)
                // that picked the wrong side and reported every child of a row
                // against the canvas it sits on.
                if paints_nothing(a) || paints_nothing(b) {
                    continue;
                }

                // A rule crossing what it rules over is the picture working.
                if is_structural_crossing(a, b) {
                    continue;
                }

                // A container that paints nothing is not something to collide
                // with — but what it draws one level down is.
                if is_drawn_inside_a_bare_region(a, b) {
                    let (region, outside, outside_index) =
                        if is_bare_container(a) { (a, b, index_b) } else { (b, a, index_a) };
                    check_against_region_contents(
                        outside,
                        &name(outside, outside_index),
                        region,
                        contains_ids,
                        scope,
                        warnings,
                    );
                    continue;
                }

                if let Some((w, h)) = reportable_overlap(a, b, contains_ids) {
                    let mut name_a = name(a, index_a);
                    let mut name_b = name(b, index_b);
                    // Two anonymous children of the same cell would otherwise
                    // read as one element overlapping itself.
                    if name_a == name_b {
                        name_a = format!("{} (child #{})", name_a, index_a + 1);
                        name_b = format!("{} (child #{})", name_b, index_b + 1);
                    }
                    warnings.push(overlap_warning(&name_a, &name_b, w, h));
                }
            }
        }
    } // end skip_sibling_checks

    // Recurse into children that have children
    for (_, child) in children.iter() {
        if !child.children.is_empty() {
            check_overlaps_recursive(
                child,
                current_prefix.as_deref(),
                contains_ids,
                scope,
                warnings,
            );
        }
    }
}

// ── FR3: Contains constraint verification ─────────────────────────

/// Containment violations below this are solver float residue, not a mistake.
/// Reporting a 0px overshoot on correct code is how the linter taught agents
/// that its warnings are noise.
const CONTAINMENT_EPSILON: f64 = 0.5;

fn check_contains(result: &LayoutResult, doc: &Document, warnings: &mut Vec<LintWarning>) {
    check_contains_in_stmts(&doc.statements, result, warnings);
    check_contains_overrides_size(result, doc, warnings);
}

/// `contains` frees both dimensions, which quietly discards a size the author
/// wrote down. A `height: 3` rule told to contain a row of cells comes back as
/// tall as the cells, and nothing says so. Warn, and point at the alternative:
/// constraining the two edges that matter leaves the other dimension alone.
fn check_contains_overrides_size(
    result: &LayoutResult,
    doc: &Document,
    warnings: &mut Vec<LintWarning>,
) {
    use crate::parser::ast::StyleKey;

    // Containers of a `contains` constraint, and the sizes they declared.
    let mut containers: Vec<String> = Vec::new();
    fn collect_containers(stmts: &[crate::parser::ast::Spanned<Statement>], out: &mut Vec<String>) {
        for stmt in stmts {
            match &stmt.node {
                Statement::Constrain(c) => {
                    if let ConstraintExpr::Contains { container, .. } = &c.expr {
                        out.push(container.node.0.clone());
                    }
                }
                Statement::Layout(l) => collect_containers(&l.children, out),
                Statement::Group(g) => collect_containers(&g.children, out),
                _ => {}
            }
        }
    }
    collect_containers(&doc.statements, &mut containers);
    if containers.is_empty() {
        return;
    }

    fn declared_sizes(
        stmts: &[crate::parser::ast::Spanned<Statement>],
        containers: &[String],
        result: &LayoutResult,
        warnings: &mut Vec<LintWarning>,
    ) {
        for stmt in stmts {
            match &stmt.node {
                Statement::Shape(shape) => {
                    let Some(name) = shape.name.as_ref().map(|n| n.node.0.clone()) else {
                        continue;
                    };
                    if !containers.contains(&name) {
                        continue;
                    }
                    for m in &shape.modifiers {
                        let (axis, declared) = match (&m.node.key.node, &m.node.value.node) {
                            (StyleKey::Width, crate::parser::ast::StyleValue::Number { value, .. }) => {
                                ("width", *value)
                            }
                            (StyleKey::Height, crate::parser::ast::StyleValue::Number { value, .. }) => {
                                ("height", *value)
                            }
                            _ => continue,
                        };
                        let actual = result.get_element_by_name(&name).map(|e| {
                            if axis == "width" { e.bounds.width } else { e.bounds.height }
                        });
                        let Some(actual) = actual else { continue };
                        if (actual - declared).abs() < 1.0 {
                            continue; // the size survived; nothing to report
                        }
                        warnings.push(LintWarning {
                            category: LintCategory::OverConstrained,
                            message: format!(
                                "\"{}\" declares {}: {:.0} but `contains` sizes both axes, so it came out {:.0}; \
                                 to keep the other axis, constrain the edges instead (e.g. \"{}\".left / .right)",
                                name, axis, declared, actual, name
                            ),
                            frames: Vec::new(),
                            pair: None,
                        });
                    }
                }
                Statement::Layout(l) => declared_sizes(&l.children, containers, result, warnings),
                Statement::Group(g) => declared_sizes(&g.children, containers, result, warnings),
                _ => {}
            }
        }
    }
    declared_sizes(&doc.statements, &containers, result, warnings);
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
                    if let Some(container_elem) = result.get_element_by_name(&container.node.0) {
                        let cb = container_elem.bounds;
                        for elem_id in elements {
                            if let Some(elem) = result.get_element_by_name(&elem_id.node.0) {
                                let eb = elem.bounds;
                                // Check left edge
                                if cb.x > eb.x - pad + CONTAINMENT_EPSILON {
                                    let overflow = cb.x - (eb.x - pad);
                                    warnings.push(LintWarning {
                                        category: LintCategory::Containment,
                                        message: format!(
                                            "element \"{}\" extends {:.0}px past left edge of container \"{}\"",
                                            elem_id.node.0, overflow, container.node.0
                                        ),
                                        frames: Vec::new(),
                                        pair: None,
                                    });
                                }
                                // Check right edge
                                if cb.right() + CONTAINMENT_EPSILON < eb.right() + pad {
                                    let overflow = (eb.right() + pad) - cb.right();
                                    warnings.push(LintWarning {
                                        category: LintCategory::Containment,
                                        message: format!(
                                            "element \"{}\" extends {:.0}px past right edge of container \"{}\"",
                                            elem_id.node.0, overflow, container.node.0
                                        ),
                                        frames: Vec::new(),
                                        pair: None,
                                    });
                                }
                                // Check top edge
                                if cb.y > eb.y - pad + CONTAINMENT_EPSILON {
                                    let overflow = cb.y - (eb.y - pad);
                                    warnings.push(LintWarning {
                                        category: LintCategory::Containment,
                                        message: format!(
                                            "element \"{}\" extends {:.0}px past top edge of container \"{}\"",
                                            elem_id.node.0, overflow, container.node.0
                                        ),
                                        frames: Vec::new(),
                                        pair: None,
                                    });
                                }
                                // Check bottom edge
                                if cb.bottom() + CONTAINMENT_EPSILON < eb.bottom() + pad {
                                    let overflow = (eb.bottom() + pad) - cb.bottom();
                                    warnings.push(LintWarning {
                                        category: LintCategory::Containment,
                                        message: format!(
                                            "element \"{}\" extends {:.0}px past bottom edge of container \"{}\"",
                                            elem_id.node.0, overflow, container.node.0
                                        ),
                                        frames: Vec::new(),
                                        pair: None,
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
        .unwrap_or(label.font_size);
    // Measure the laid-out lines, not the flattened wording: a three-line
    // card joined into one string looks three times too wide.
    let metrics = crate::layout::text::measure_runs(&label.rich.lines, font_size);
    let width = metrics.width;
    let height = metrics.height;

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
    scope: &FrameScope<'_>,
    labels: &mut Vec<LabelInfo>,
) {
    collect_labels_in(elem, &[], scope, labels)
}

/// `siblings` is the element's own row in the tree, so an unnamed element can
/// still be named by the grid cell it sits in.
fn collect_labels_in(
    elem: &ElementLayout,
    siblings: &[&ElementLayout],
    scope: &FrameScope<'_>,
    labels: &mut Vec<LabelInfo>,
) {
    // A hidden element takes its label — and its whole subtree — off screen.
    if scope.hides_element(elem) {
        return;
    }
    let owner = || {
        elem.id
            .as_ref()
            .map(|id| id.0.clone())
            .or_else(|| grid_cell_name(elem, siblings))
            .unwrap_or_else(|| "<anon>".to_string())
    };
    if let Some(label) = &elem.label {
        labels.push(LabelInfo {
            owner: owner(),
            bbox: estimate_label_bbox(label),
            parent_opacity: elem.styles.opacity,
        });
    }
    // Standalone text elements act like labels for overlap checking
    if is_text_shape(elem) {
        labels.push(LabelInfo {
            owner: owner(),
            bbox: elem.bounds,
            parent_opacity: elem.styles.opacity,
        });
    }
    let children: Vec<&ElementLayout> = elem.children.iter().collect();
    for child in &elem.children {
        collect_labels_in(child, &children, scope, labels);
    }
}

fn check_labels(result: &LayoutResult, scope: &FrameScope<'_>, warnings: &mut Vec<LintWarning>) {
    let mut labels = Vec::new();

    // Collect element labels
    for elem in &result.root_elements {
        collect_labels_recursive(elem, scope, &mut labels);
    }

    // Collect connection labels
    for conn in &result.connections {
        if scope.hides_connection(conn.name.as_ref().map(|n| n.0.as_str())) {
            continue;
        }
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
                    message: format!("labels on \"{}\" and \"{}\" overlap", a.owner, b.owner),
                    frames: Vec::new(),
                    pair: None,
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
fn check_label_element_overlaps(
    result: &LayoutResult,
    scope: &FrameScope<'_>,
    warnings: &mut Vec<LintWarning>,
) {
    // Collect all labels with owner info
    let mut labels: Vec<LabelInfo> = Vec::new();
    for elem in &result.root_elements {
        collect_labels_recursive(elem, scope, &mut labels);
    }
    for conn in &result.connections {
        if scope.hides_connection(conn.name.as_ref().map(|n| n.0.as_str())) {
            continue;
        }
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
        collect_opaque_elements(elem, None, i, scope, &mut shapes);
    }

    for label in &labels {
        for shape in &shapes {
            // Skip if label belongs to this element (own label inside own box)
            if label.owner == shape.id {
                continue;
            }

            // Skip labels owned by a callout (annotation pins overlap on purpose).
            if result
                .get_element_by_name(&label.owner)
                .is_some_and(is_callout)
            {
                continue;
            }

            // Skip transparent labels
            if let Some(op) = label.parent_opacity {
                if op < 1.0 {
                    continue;
                }
            }

            // The key check: intersects the edge but NOT fully inside
            if label.bbox.intersects(&shape.bounds) && !shape.bounds.contains_bbox(&label.bbox) {
                let overlap_w =
                    label.bbox.right().min(shape.bounds.right()) - label.bbox.x.max(shape.bounds.x);
                let overlap_h = label.bbox.bottom().min(shape.bounds.bottom())
                    - label.bbox.y.max(shape.bounds.y);
                warnings.push(LintWarning {
                    category: LintCategory::Label,
                    message: format!(
                        "label on \"{}\" straddles the edge of element \"{}\"; \
                         overlaps by {:.0}x{:.0}px",
                        label.owner, shape.id, overlap_w, overlap_h
                    ),
                    frames: Vec::new(),
                    pair: Some(sorted_pair(&label.owner, &shape.id)),
                });
            }
        }
    }
}

/// Sample N points along a cubic Bézier curve defined by (p0, p1, p2, p3).
fn sample_cubic_bezier(p0: &Point, p1: &Point, p2: &Point, p3: &Point, n: usize) -> Vec<Point> {
    (0..=n)
        .map(|i| {
            let t = i as f64 / n as f64;
            let u = 1.0 - t;
            Point {
                x: u * u * u * p0.x + 3.0 * u * u * t * p1.x + 3.0 * u * t * t * p2.x + t * t * t * p3.x,
                y: u * u * u * p0.y + 3.0 * u * u * t * p1.y + 3.0 * u * t * t * p2.y + t * t * t * p3.y,
            }
        })
        .collect()
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
        (Point::new(bbox.x, bbox.y), Point::new(bbox.right(), bbox.y)),
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
    scope: &FrameScope<'_>,
    elements: &mut Vec<OpaqueElement>,
) {
    collect_opaque_elements_in(elem, parent_name, child_index, &[], scope, elements)
}

/// `siblings` is the element's own row in the tree, so an unnamed element can
/// still be named by the grid cell it sits in.
fn collect_opaque_elements_in(
    elem: &ElementLayout,
    parent_name: Option<&str>,
    child_index: usize,
    siblings: &[&ElementLayout],
    scope: &FrameScope<'_>,
    elements: &mut Vec<OpaqueElement>,
) {
    if scope.hides_element(elem) {
        return;
    }
    // Only collect visual shapes (not groups/layouts) that are opaque and
    // non-text. An element that paints nothing is not something a connection
    // can cross: a `via:` waypoint is an invisible 1px circle that the route
    // is deliberately threaded through, and reporting the route for hitting
    // it describes the author's own instruction back at them.
    if is_visual_shape(elem) && !is_text_shape(elem) && is_opaque(elem) && !paints_nothing(elem) {
        let id = if let Some(name) = &elem.id {
            name.0.clone()
        } else {
            grid_cell_name(elem, siblings)
                .unwrap_or_else(|| element_display_name(elem, parent_name, child_index))
        };
        elements.push(OpaqueElement {
            id,
            bounds: elem.bounds,
        });
    }

    let name = elem.id.as_ref().map(|id| id.0.as_str());
    let children: Vec<&ElementLayout> = elem.children.iter().collect();
    for (i, child) in elem.children.iter().enumerate() {
        collect_opaque_elements_in(child, name, i, &children, scope, elements);
    }
}

fn collect_visible_elements(
    elem: &ElementLayout,
    parent_name: Option<&str>,
    child_index: usize,
    scope: &FrameScope<'_>,
    elements: &mut Vec<OpaqueElement>,
) {
    collect_visible_elements_in(elem, parent_name, child_index, &[], scope, elements)
}

/// `siblings` is the element's own row in the tree, so an unnamed element can
/// still be named by the grid cell it sits in.
fn collect_visible_elements_in(
    elem: &ElementLayout,
    parent_name: Option<&str>,
    child_index: usize,
    siblings: &[&ElementLayout],
    scope: &FrameScope<'_>,
    elements: &mut Vec<OpaqueElement>,
) {
    if scope.hides_element(elem) {
        return;
    }
    // Collect visual shapes that are substantially visible (opacity >= 0.5)
    if is_visual_shape(elem)
        && !is_text_shape(elem)
        && is_substantially_visible(elem)
        && !paints_nothing(elem)
    {
        let id = if let Some(name) = &elem.id {
            name.0.clone()
        } else {
            grid_cell_name(elem, siblings)
                .unwrap_or_else(|| element_display_name(elem, parent_name, child_index))
        };
        elements.push(OpaqueElement {
            id,
            bounds: elem.bounds,
        });
    }

    let name = elem.id.as_ref().map(|id| id.0.as_str());
    let children: Vec<&ElementLayout> = elem.children.iter().collect();
    for (i, child) in elem.children.iter().enumerate() {
        collect_visible_elements_in(child, name, i, &children, scope, elements);
    }
}

fn check_connections(
    result: &LayoutResult,
    scope: &FrameScope<'_>,
    warnings: &mut Vec<LintWarning>,
) {
    // Collect all substantially visible, non-text elements in this frame
    let mut opaque_elements = Vec::new();
    for (i, elem) in result.root_elements.iter().enumerate() {
        collect_visible_elements(elem, None, i, scope, &mut opaque_elements);
    }

    for conn in &result.connections {
        // Skip connections that are hidden in this frame
        if scope.hides_connection(conn.name.as_ref().map(|n| n.0.as_str())) {
            continue;
        }

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

        if conn.routing_mode == RoutingMode::Curved {
            // For curved connections, sample points along the Bézier curve and
            // check if any sample falls inside an element's bounds.
            // The path has 4 points for a cubic Bézier: start, cp1, cp2, end.
            let samples = if conn.path.len() == 4 {
                sample_cubic_bezier(&conn.path[0], &conn.path[1], &conn.path[2], &conn.path[3], 20)
            } else {
                // Fallback: treat as polyline
                conn.path.clone()
            };

            for oe in &opaque_elements {
                if crossed.contains(&oe.id) {
                    continue;
                }
                if oe.bounds.contains(*path_start) || oe.bounds.contains(*path_end) {
                    continue;
                }
                if samples.iter().any(|p| oe.bounds.contains(*p)) {
                    crossed.insert(oe.id.clone());
                    warnings.push(LintWarning {
                        category: LintCategory::Connection,
                        message: format!(
                            "connection {}→{} overlaps element \"{}\"",
                            from_id, to_id, oe.id
                        ),
                        frames: Vec::new(),
                        pair: None,
                    });
                }
            }
        } else {
            // For straight/orthogonal connections, check each path segment
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
                            frames: Vec::new(),
                            pair: None,
                        });
                    }
                }
            }
        }
    }
}

// ── Label-connection overlap detection ─────────────────────────────

/// Check if any label (element label, connection label, or standalone text)
/// overlaps with a connection path segment.  This catches labels placed at
/// bend points or too close to connector lines.
/// True when a label is drawn within one of the connection's endpoints.
///
/// Anonymous elements have no id to match on, so a subtitle inside the column
/// an arrow terminates at cannot be recognised by name — but it is still
/// inside the thing the connection points to.
fn label_sits_in_endpoint(result: &LayoutResult, id: &str, bbox: &BoundingBox) -> bool {
    result
        .get_element_by_name(id)
        .is_some_and(|elem| elem.bounds.intersects(bbox))
}

/// An element's own id plus every descendant's.
///
/// A connection terminating at a container reaches everything drawn in it, so
/// rules about "what this connection touches" have to treat the family as one.
fn endpoint_family(result: &LayoutResult, id: &str) -> HashSet<String> {
    fn collect(elem: &ElementLayout, out: &mut HashSet<String>) {
        if let Some(id) = &elem.id {
            out.insert(id.0.clone());
        }
        for child in &elem.children {
            collect(child, out);
        }
    }
    let mut out = HashSet::new();
    out.insert(id.to_string());
    if let Some(elem) = result.get_element_by_name(id) {
        collect(elem, &mut out);
    }
    out
}

fn check_label_connection_overlaps(
    result: &LayoutResult,
    scope: &FrameScope<'_>,
    warnings: &mut Vec<LintWarning>,
) {
    // Collect user-level labels only (skip template internals).
    // Template children have IDs like `q_main_g_label`; we skip labels whose
    // owner shares a prefix with a root-level template group.
    let template_prefixes: Vec<String> = result
        .root_elements
        .iter()
        .filter(|e| is_template_instance_group(e))
        .filter_map(|e| e.id.as_ref().map(|id| format!("{}_", id.0)))
        .collect();

    let is_template_internal =
        |owner: &str| -> bool { template_prefixes.iter().any(|pfx| owner.starts_with(pfx)) };

    // Collect labels from elements + standalone text (skipping template internals)
    let mut labels: Vec<LabelInfo> = Vec::new();
    for elem in &result.root_elements {
        collect_labels_recursive(elem, scope, &mut labels);
    }
    labels.retain(|l| !is_template_internal(&l.owner));

    // Collect connection labels
    for conn in &result.connections {
        if scope.hides_connection(conn.name.as_ref().map(|n| n.0.as_str())) {
            continue;
        }
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
            if scope.hides_connection(conn.name.as_ref().map(|n| n.0.as_str())) {
                continue;
            }

            let conn_name = format!("{}→{}", conn.from_id.0, conn.to_id.0);

            // Skip: a connection label overlapping its own connection is expected
            // (the label is placed at the midpoint of the path by design)
            if label.owner == conn_name {
                continue;
            }

            // Skip: label on an element that is an endpoint of this
            // connection, or on anything inside one (e.g. junction labels at
            // railway switches, pin labels at transistor leads, a level name
            // inside the column the arrow terminates at). A connection that
            // ends AT a container necessarily reaches its contents, so
            // reporting the contact describes the connection the author asked
            // for.
            if endpoint_family(result, &conn.from_id.0).contains(&label.owner)
                || endpoint_family(result, &conn.to_id.0).contains(&label.owner)
                || label_sits_in_endpoint(result, &conn.from_id.0, &label.bbox)
                || label_sits_in_endpoint(result, &conn.to_id.0, &label.bbox)
            {
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
                        frames: Vec::new(),
                        pair: None,
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
                frames: Vec::new(),
                pair: None,
            });
        } else if dx < ALIGNMENT_THRESHOLD && dy > dx * 4.0 {
            // Nearly vertical — small X offset
            warnings.push(LintWarning {
                category: LintCategory::Alignment,
                message: format!(
                    "connection {}→{} is nearly vertical (off by {:.0}px); aligning X positions would straighten it",
                    conn.from_id.0, conn.to_id.0, dx
                ),
                frames: Vec::new(),
                pair: None,
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
        ConstraintProperty::Anchor(name) => name,
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

/// Report a `constrain` that a later statement superseded.
///
/// Overriding is allowed — an author restating their intent is legitimate, and
/// it is how a file composing a shared part re-pins something in it. But a
/// silent override hides a mistake as effectively as the hard error it
/// replaced, so it is always reported.
fn check_overridden_constraints(result: &LayoutResult, warnings: &mut Vec<LintWarning>) {
    // The superseded constraint never reached the solver, so `over-constrained`
    // reporting it as "violated" is describing a statement that was
    // deliberately dropped. The override warning below says the useful version
    // of the same thing, and naming one line twice is what taught authors to
    // stop reading lint output.
    for (element, property) in &result.overridden_constraints {
        let qualified = format!("{element}.{property}");
        warnings.retain(|w| {
            w.category != LintCategory::OverConstrained || !w.message.contains(&qualified)
        });
    }

    for (element, property) in &result.overridden_constraints {
        warnings.push(LintWarning {
            category: LintCategory::OverriddenConstraint,
            message: format!(
                "a later constrain on \"{element}\".{property} overrides an earlier one; \
                 the last statement wins — delete the earlier one if that was intended"
            ),
            frames: vec![],
            pair: None,
        });
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
            frames: Vec::new(),
            pair: None,
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
                shortest_orientation = if dx > dy {
                    "horizontally"
                } else {
                    "vertically"
                };
            }
        }

        if shortest_len < f64::MAX {
            warnings.push(LintWarning {
                category: LintCategory::ReducibleBend,
                message: format!(
                    "connection {}→{}: path jogs {:.0}px {} between bends; \
                     moving elements at least {:.0}px further apart {} would eliminate 2 corners",
                    conn.from_id.0,
                    conn.to_id.0,
                    shortest_len,
                    shortest_orientation,
                    shortest_len,
                    shortest_orientation
                ),
                frames: Vec::new(),
                pair: None,
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
fn find_connection_layout<'a>(
    result: &'a LayoutResult,
    from: &str,
    to: &str,
) -> Option<&'a super::types::ConnectionLayout> {
    result
        .connections
        .iter()
        .find(|c| c.from_id.0 == from && c.to_id.0 == to)
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
                            frames: Vec::new(),
                            pair: None,
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
                            frames: Vec::new(),
                            pair: None,
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
    let dark_patterns = [
        "-dark",
        "foreground-1",
        "foreground-2",
        "text-dark",
        "text-1",
        "text-2",
    ];
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
        if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
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

/// True when a label is drawn inside its element's box.
///
/// Both the contrast and overflow rules are about text sitting ON a shape: a
/// label that hangs above or beside its element is drawn on the background, so
/// it neither inherits the shape's fill for contrast nor has to fit inside it.
fn label_is_inside(label: &crate::layout::types::LabelLayout) -> bool {
    use crate::layout::types::ShapeLabelPosition;
    label
        .placement
        .as_ref()
        .is_none_or(|p| p.position == ShapeLabelPosition::Inside)
}

fn check_contrast(result: &LayoutResult, warnings: &mut Vec<LintWarning>) {
    for elem in &result.root_elements {
        check_contrast_recursive(elem, warnings);
    }
}

fn check_contrast_recursive(elem: &ElementLayout, warnings: &mut Vec<LintWarning>) {
    // Check if this element has a label AND a dark fill AND no explicit label color
    if let Some(label) = &elem.label.as_ref().filter(|l| label_is_inside(l)) {
        let has_label_color = label
            .styles
            .as_ref()
            .and_then(|s| s.fill.as_ref())
            .is_some();
        // A fill painted at low opacity composites to something pale
        // whatever its colour, so it is not a dark background for the label.
        // Calling it dark is what drives a stylesheet to pick light text,
        // which then lands near-white on near-white — the opposite of the
        // problem this rule exists to prevent.
        let washed_out = elem.styles.fill_opacity.is_some_and(|o| o < 0.5);
        if !has_label_color && !washed_out {
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
                        frames: Vec::new(),
                        pair: None,
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

        let is_steep =
            (angle >= pi_6 && angle <= pi_3) || (angle >= two_pi_3 && angle <= five_pi_6);

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
                frames: Vec::new(),
                pair: None,
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
                    let child_count = l
                        .children
                        .iter()
                        .filter(|c| {
                            matches!(
                                c.node,
                                Statement::Shape(_) | Statement::Group(_) | Statement::Layout(_)
                            )
                        })
                        .count();

                    if child_count > 8 {
                        let layout_name = l
                            .name
                            .as_ref()
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
                            frames: Vec::new(),
                            pair: None,
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
        ConstraintProperty::Anchor(_) => None, // composite point, skip
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
        ConstraintExpr::EqualWithOffset {
            left,
            right,
            offset,
        } => {
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

fn check_over_constrained(result: &LayoutResult, doc: &Document, warnings: &mut Vec<LintWarning>) {
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
                                        frames: Vec::new(),
                                        pair: None,
                                    });
                                }
                            }
                        }
                    }
                    ConstraintExpr::EqualWithOffset {
                        left,
                        right,
                        offset,
                    } => {
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
                                        frames: Vec::new(),
                                        pair: None,
                                    });
                                }
                            }
                        }
                    }
                    ConstraintExpr::Constant { left, value } => {
                        let elem = result.get_element_by_name(&left.element.node.leaf().0);
                        if let Some(e) = elem {
                            if let Some(solved) =
                                resolve_property_value(&e.bounds, &left.property.node)
                            {
                                let residual = (solved - value).abs();
                                if residual > EPSILON {
                                    let desc = format_constraint_expr(&c.expr);
                                    warnings.push(LintWarning {
                                        category: LintCategory::OverConstrained,
                                        message: format!(
                                            "constraint \"{}\" is violated by {:.0}px; the system may be over-constrained",
                                            desc, residual
                                        ),
                                        frames: Vec::new(),
                                        pair: None,
                                    });
                                }
                            }
                        }
                    }
                    ConstraintExpr::GreaterOrEqual { left, value } => {
                        let elem = result.get_element_by_name(&left.element.node.leaf().0);
                        if let Some(e) = elem {
                            if let Some(solved) =
                                resolve_property_value(&e.bounds, &left.property.node)
                            {
                                if solved < value - EPSILON {
                                    let desc = format_constraint_expr(&c.expr);
                                    let violation = value - solved;
                                    warnings.push(LintWarning {
                                        category: LintCategory::OverConstrained,
                                        message: format!(
                                            "constraint \"{}\" is violated by {:.0}px; the system may be over-constrained",
                                            desc, violation
                                        ),
                                        frames: Vec::new(),
                                        pair: None,
                                    });
                                }
                            }
                        }
                    }
                    ConstraintExpr::LessOrEqual { left, value } => {
                        let elem = result.get_element_by_name(&left.element.node.leaf().0);
                        if let Some(e) = elem {
                            if let Some(solved) =
                                resolve_property_value(&e.bounds, &left.property.node)
                            {
                                if solved > value + EPSILON {
                                    let desc = format_constraint_expr(&c.expr);
                                    let violation = solved - value;
                                    warnings.push(LintWarning {
                                        category: LintCategory::OverConstrained,
                                        message: format!(
                                            "constraint \"{}\" is violated by {:.0}px; the system may be over-constrained",
                                            desc, violation
                                        ),
                                        frames: Vec::new(),
                                        pair: None,
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
/// Every CSS named colour, plus the keywords a colour property also accepts.
/// A word that is neither one of these nor a palette token is passed straight
/// through to the SVG, where the browser draws black or nothing.
const CSS_COLOR_NAMES: &[&str] = &[
    "aliceblue", "antiquewhite", "aqua", "aquamarine", "azure", "beige", "bisque", "black",
    "blanchedalmond", "blue", "blueviolet", "brown", "burlywood", "cadetblue", "chartreuse",
    "chocolate", "coral", "cornflowerblue", "cornsilk", "crimson", "cyan", "darkblue",
    "darkcyan", "darkgoldenrod", "darkgray", "darkgreen", "darkgrey", "darkkhaki",
    "darkmagenta", "darkolivegreen", "darkorange", "darkorchid", "darkred", "darksalmon",
    "darkseagreen", "darkslateblue", "darkslategray", "darkslategrey", "darkturquoise",
    "darkviolet", "deeppink", "deepskyblue", "dimgray", "dimgrey", "dodgerblue", "firebrick",
    "floralwhite", "forestgreen", "fuchsia", "gainsboro", "ghostwhite", "gold", "goldenrod",
    "gray", "green", "greenyellow", "grey", "honeydew", "hotpink", "indianred", "indigo",
    "ivory", "khaki", "lavender", "lavenderblush", "lawngreen", "lemonchiffon", "lightblue",
    "lightcoral", "lightcyan", "lightgoldenrodyellow", "lightgray", "lightgreen", "lightgrey",
    "lightpink", "lightsalmon", "lightseagreen", "lightskyblue", "lightslategray",
    "lightslategrey", "lightsteelblue", "lightyellow", "lime", "limegreen", "linen", "magenta",
    "maroon", "mediumaquamarine", "mediumblue", "mediumorchid", "mediumpurple",
    "mediumseagreen", "mediumslateblue", "mediumspringgreen", "mediumturquoise",
    "mediumvioletred", "midnightblue", "mintcream", "mistyrose", "moccasin", "navajowhite",
    "navy", "oldlace", "olive", "olivedrab", "orange", "orangered", "orchid", "palegoldenrod",
    "palegreen", "paleturquoise", "palevioletred", "papayawhip", "peachpuff", "peru", "pink",
    "plum", "powderblue", "purple", "rebeccapurple", "red", "rosybrown", "royalblue",
    "saddlebrown", "salmon", "sandybrown", "seagreen", "seashell", "sienna", "silver",
    "skyblue", "slateblue", "slategray", "slategrey", "snow", "springgreen", "steelblue",
    "tan", "teal", "thistle", "tomato", "turquoise", "violet", "wheat", "white", "whitesmoke",
    "yellow", "yellowgreen", "none", "transparent", "currentcolor", "inherit"
];

/// Colour values that name nothing.
///
/// `fill: geenkleur` parses, renders, and shows up as black — the same class
/// of silent mistake as a misspelled modifier, and easier to make when a
/// custom stylesheet defines the token names.
fn check_unknown_colors(doc: &Document, warnings: &mut Vec<LintWarning>) {
    use crate::parser::ast::{StyleKey, StyleValue};

    fn check(
        modifiers: &[crate::parser::ast::Spanned<crate::parser::ast::StyleModifier>],
        owner: &str,
        warnings: &mut Vec<LintWarning>,
    ) {
        for m in modifiers {
            let key = match &m.node.key.node {
                StyleKey::Fill => "fill",
                StyleKey::Stroke => "stroke",
                StyleKey::LabelFill => "label_fill",
                _ => continue,
            };
            // Hex, palette tokens and fill functions are resolved elsewhere;
            // a bare word is the only thing that can silently mean nothing.
            let word = match &m.node.value.node {
                StyleValue::Identifier(id) => id.0.as_str(),
                StyleValue::Keyword(k) => k.as_str(),
                _ => continue,
            };
            let lower = word.to_ascii_lowercase();
            if CSS_COLOR_NAMES.contains(&lower.as_str()) {
                continue;
            }
            // `fill: hatch` and friends are pattern shorthands, not colours.
            if matches!(lower.as_str(), "hatch" | "dots" | "grid" | "gradient" | "radial") {
                continue;
            }
            warnings.push(LintWarning {
                category: LintCategory::UnknownModifier,
                message: format!(
                    "{}: \"{}\" on {} is not a palette token or a CSS colour name; \
                     it reaches the SVG as-is and will not render as intended",
                    key, word, owner
                ),
                frames: Vec::new(),
                pair: None,
            });
        }
    }

    fn walk(stmts: &[crate::parser::ast::Spanned<Statement>], warnings: &mut Vec<LintWarning>) {
        for stmt in stmts {
            match &stmt.node {
                Statement::Shape(shape) => {
                    let owner = shape
                        .name
                        .as_ref()
                        .map(|n| format!("\"{}\"", n.node.0))
                        .unwrap_or_else(|| "an unnamed element".to_string());
                    check(&shape.modifiers, &owner, warnings);
                }
                Statement::Layout(l) => walk(&l.children, warnings),
                Statement::Group(g) => walk(&g.children, warnings),
                Statement::Keyframe(kf) => {
                    for op in &kf.operations {
                        if let crate::parser::ast::KeyframeOp::Transform { target, modifiers } =
                            &op.node
                        {
                            let owner =
                                format!("\"{}\" in keyframe \"{}\"", target.node.0, kf.name.node);
                            check(modifiers, &owner, warnings);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    walk(&doc.statements, warnings);
}

/// Modifier keys that no part of the pipeline reads.
///
/// An unrecognised key parses fine and is then dropped on the floor, so an
/// invented name (`fill_label` for `label_fill`, say) renders without a word
/// of complaint and the author only notices from the picture. These are the
/// keys that are not `StyleKey` variants but are still consumed somewhere.
const KNOWN_CUSTOM_KEYS: &[&str] = &[
    "at",           // grid placement
    "cell_width",   // grid
    "cell_height",  // grid
    "cols",         // grid
    "rows",         // grid
    "col_labels",   // grid
    "row_labels",   // grid
    "trim",         // svg templates
    "via",          // connection routing
    "padding",      // contains
];

/// Report `transform` keys that are recognised but cannot be animated.
///
/// `unknown-modifier` catches a misspelling. This catches the opposite and
/// nastier case: a correct key that the renderer silently drops, so the author
/// doing it *right* is the one who gets no signal.
fn check_unanimatable_transform_keys(doc: &Document, warnings: &mut Vec<LintWarning>) {
    use crate::layout::keyframe::is_animatable;
    use crate::parser::ast::{KeyframeOp, StyleKey};

    for stmt in &doc.statements {
        let Statement::Keyframe(kf) = &stmt.node else {
            continue;
        };
        for op in &kf.operations {
            let KeyframeOp::Transform { target, modifiers } = &op.node else {
                continue;
            };
            for m in modifiers {
                // A Custom key is a typo; `unknown-modifier` owns that case.
                if matches!(m.node.key.node, StyleKey::Custom(_)) {
                    continue;
                }
                if is_animatable(&m.node.key.node) {
                    continue;
                }
                let key = style_key_name(&m.node.key.node);
                warnings.push(LintWarning {
                    category: LintCategory::UnknownModifier,
                    message: format!(
                        "\"{key}\" on \"{}\" in keyframe \"{}\" cannot be animated and is \
                         ignored; set it on the element itself instead",
                        target.node.0, kf.name.node
                    ),
                    frames: vec![kf.name.node.clone()],
                    pair: None,
                });
            }
        }
    }
}

/// Spell a StyleKey the way an author writes it.
fn style_key_name(key: &crate::parser::ast::StyleKey) -> String {
    use crate::parser::ast::StyleKey as K;
    match key {
        K::Custom(c) => return c.clone(),
        other => {
            // Debug spelling is CamelCase; the DSL is snake_case.
            let dbg = format!("{other:?}");
            let mut out = String::new();
            for (i, ch) in dbg.chars().enumerate() {
                if ch.is_uppercase() && i > 0 {
                    out.push('_');
                }
                out.extend(ch.to_lowercase());
            }
            out
        }
    }
}

fn check_unknown_modifiers(doc: &Document, warnings: &mut Vec<LintWarning>) {
    fn owner_name(name: Option<&crate::parser::ast::Spanned<crate::parser::ast::Identifier>>) -> String {
        name.map(|n| format!("\"{}\"", n.node.0))
            .unwrap_or_else(|| "an unnamed element".to_string())
    }

    fn check(
        modifiers: &[crate::parser::ast::Spanned<crate::parser::ast::StyleModifier>],
        owner: &str,
        warnings: &mut Vec<LintWarning>,
    ) {
        for m in modifiers {
            if let crate::parser::ast::StyleKey::Custom(key) = &m.node.key.node {
                if KNOWN_CUSTOM_KEYS.contains(&key.as_str()) {
                    continue;
                }
                warnings.push(LintWarning {
                    category: LintCategory::UnknownModifier,
                    message: format!(
                        "unknown modifier \"{}\" on {} is ignored; check the spelling against --grammar",
                        key, owner
                    ),
                    frames: Vec::new(),
                    pair: None,
                });
            }
        }
    }

    fn walk(
        stmts: &[crate::parser::ast::Spanned<Statement>],
        warnings: &mut Vec<LintWarning>,
    ) {
        for stmt in stmts {
            match &stmt.node {
                Statement::Shape(shape) => check(&shape.modifiers, &owner_name(shape.name.as_ref()), warnings),
                Statement::Layout(l) => {
                    check(&l.modifiers, &owner_name(l.name.as_ref()), warnings);
                    walk(&l.children, warnings);
                }
                Statement::Group(g) => {
                    check(&g.modifiers, &owner_name(g.name.as_ref()), warnings);
                    walk(&g.children, warnings);
                }
                Statement::Connection(conns) => {
                    for c in conns {
                        let owner = format!("connection {}->{}", c.from.element.node.0, c.to.element.node.0);
                        check(&c.modifiers, &owner, warnings);
                    }
                }
                Statement::Keyframe(kf) => {
                    for op in &kf.operations {
                        if let crate::parser::ast::KeyframeOp::Transform { target, modifiers } = &op.node {
                            let owner = format!("\"{}\" in keyframe \"{}\"", target.node.0, kf.name.node);
                            check(modifiers, &owner, warnings);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    walk(&doc.statements, warnings);
}

/// Text wider than the box it was given.
///
/// A box is only as wide as the author said, and how wide a wording renders is
/// not something the author can work out by eye — so when the text does not
/// fit, say so and name the width it needs. Keyframe wordings are checked too:
/// text rewritten by a later frame is laid out from its frame-0 wording.
fn check_text_fits_its_box(
    result: &LayoutResult,
    doc: &Document,
    warnings: &mut Vec<LintWarning>,
) {
    // Every wording each element ever shows: its own, plus keyframe rewrites.
    let mut wordings: HashMap<String, Vec<(String, Option<String>)>> = HashMap::new();
    for kf in super::keyframe::extract_keyframes(doc) {
        for op in &kf.operations {
            if let crate::parser::ast::KeyframeOp::Transform { target, modifiers } = &op.node {
                for m in modifiers {
                    if !matches!(m.node.key.node, crate::parser::ast::StyleKey::Label) {
                        continue;
                    }
                    let text = match &m.node.value.node {
                        crate::parser::ast::StyleValue::String(s) => s.clone(),
                        crate::parser::ast::StyleValue::Keyword(k) => k.clone(),
                        _ => continue,
                    };
                    wordings
                        .entry(target.node.0.clone())
                        .or_default()
                        .push((text, Some(kf.name.node.clone())));
                }
            }
        }
    }

    fn walk(
        elem: &ElementLayout,
        wordings: &HashMap<String, Vec<(String, Option<String>)>>,
        warnings: &mut Vec<LintWarning>,
    ) {
        if let (Some(id), ElementType::Shape(ShapeType::Text { content })) =
            (elem.id.as_ref(), &elem.element_type)
        {
            let font_size = elem.styles.font_size.unwrap_or(14.0);
            let mut candidates = vec![(content.clone(), None)];
            candidates.extend(wordings.get(&id.0).cloned().unwrap_or_default());

            for (text, frame) in candidates {
                let needed = super::keyframe::estimated_text_width(&text, font_size);
                if needed <= elem.bounds.width + 2.0 {
                    continue;
                }
                warnings.push(LintWarning {
                    category: LintCategory::LabelOverflow,
                    message: format!(
                        "text \"{}\" on \"{}\" needs about {:.0}px but its box is {:.0}px; \
                         widen it (or drop the explicit width and let it size itself)",
                        text, id.0, needed, elem.bounds.width
                    ),
                    frames: frame.into_iter().collect(),
                    pair: None,
                });
            }
        }
        for child in &elem.children {
            walk(child, wordings, warnings);
        }
    }

    for elem in &result.root_elements {
        walk(elem, &wordings, warnings);
    }
}

fn check_label_overflow(result: &LayoutResult, warnings: &mut Vec<LintWarning>) {
    for elem in &result.root_elements {
        check_label_overflow_recursive(elem, warnings);
    }
}

fn check_label_overflow_recursive(elem: &ElementLayout, warnings: &mut Vec<LintWarning>) {
    // An outside label is not trying to fit in the box, so it cannot overflow it.
    if let Some(label) = &elem.label.as_ref().filter(|l| label_is_inside(l)) {
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
                        "label \"{}\" on {} overflows both width ({:.0}px label vs {:.0}px shape) and height ({:.0}px vs {:.0}px); break it with <br>, or drop the explicit size and let the box fit itself",
                        label_text, name,
                        label_bbox.width, shape_bounds.width,
                        label_bbox.height, shape_bounds.height,
                    )
                } else if width_overflow {
                    format!(
                        "label \"{}\" on {} overflows width ({:.0}px label vs {:.0}px shape); a word too long to wrap — shorten it, break it with <br>, or widen the shape",
                        label_text, name,
                        label_bbox.width, shape_bounds.width,
                    )
                } else {
                    format!(
                        "label \"{}\" on {} overflows height ({:.0}px label vs {:.0}px shape); drop the explicit height and the box grows to its lines",
                        label_text, name,
                        label_bbox.height, shape_bounds.height,
                    )
                };

                warnings.push(LintWarning {
                    category: LintCategory::LabelOverflow,
                    message: detail,
                    frames: Vec::new(),
                    pair: None,
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

    fn make_rect(id: Option<&str>, x: f64, y: f64, w: f64, h: f64) -> ElementLayout {
        ElementLayout {
            id: id.map(|s| Identifier(s.to_string())),
            element_type: ElementType::Shape(ShapeType::Rectangle),
            bounds: BoundingBox::new(x, y, w, h),
            styles: super::super::types::ResolvedStyles::default(),
            children: vec![],
            label: None,
            anchors: super::super::types::AnchorSet::default(),
            path_normalize: false,
            z_order: 0,
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

    fn make_text(id: Option<&str>, x: f64, y: f64, w: f64, h: f64) -> ElementLayout {
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
            z_order: 0,
        }
    }

    fn make_group(id: Option<&str>, children: Vec<ElementLayout>) -> ElementLayout {
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
            z_order: 0,
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
        let contains_ids = ContainsRelations::default();
        check_overlaps_recursive(&group, None, &contains_ids, &FrameScope::all_visible(), &mut warnings);
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
        check_overlaps_recursive(&group, None, &ContainsRelations::default(), &FrameScope::all_visible(), &mut warnings);
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
        let mut contains_ids = ContainsRelations::default();
        contains_ids
            .by_container
            .entry("container".to_string())
            .or_default()
            .insert("child".to_string());
        let mut warnings = Vec::new();
        check_overlaps_recursive(&group, None, &contains_ids, &FrameScope::all_visible(), &mut warnings);
        assert_eq!(warnings.len(), 0);
    }

    #[test]
    fn test_overlap_reported_for_contains_target_and_an_outsider() {
        // Being wrapped exempts the pair, not the element.
        let group = make_group(
            Some("g"),
            vec![
                make_rect(Some("container"), 0.0, 0.0, 200.0, 200.0),
                make_rect(Some("child"), 10.0, 10.0, 50.0, 50.0),
                make_rect(Some("outsider"), 30.0, 30.0, 50.0, 50.0),
            ],
        );
        let mut contains_ids = ContainsRelations::default();
        contains_ids
            .by_container
            .entry("container".to_string())
            .or_default()
            .insert("child".to_string());
        let mut warnings = Vec::new();
        check_overlaps_recursive(&group, None, &contains_ids, &FrameScope::all_visible(), &mut warnings);
        assert!(
            warnings
                .iter()
                .any(|w| w.message.contains("\"child\"") && w.message.contains("\"outsider\"")),
            "got: {:?}",
            warnings.iter().map(|w| &w.message).collect::<Vec<_>>()
        );
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
        check_overlaps_recursive(&group, None, &ContainsRelations::default(), &FrameScope::all_visible(), &mut warnings);
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
        check_overlaps_recursive(&group, None, &ContainsRelations::default(), &FrameScope::all_visible(), &mut warnings);
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
        assert_eq!(element_display_name(&elem, Some("parent"), 0), "\"foo\"");
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

    use crate::parser::ast::{ConstrainDecl, ElementPath, GroupDecl, Span};

    fn make_constant_constraint(
        elem: &str,
        prop: ConstraintProperty,
        value: f64,
    ) -> crate::parser::ast::Spanned<Statement> {
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
                name: None,
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
        let doc = make_doc(vec![make_constant_constraint(
            "a",
            ConstraintProperty::CenterX,
            100.0,
        )]);
        let mut warnings = Vec::new();
        check_redundant_constants(&doc, &mut warnings);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_inside_group() {
        let span: Span = 0..0;
        let doc = make_doc(vec![crate::parser::ast::Spanned::new(
            Statement::Group(GroupDecl {
                name: Some(crate::parser::ast::Spanned::new(
                    Identifier("g".to_string()),
                    span.clone(),
                )),
                children: vec![
                    make_constant_constraint("a", ConstraintProperty::CenterY, 50.0),
                    make_constant_constraint("b", ConstraintProperty::CenterY, 50.0),
                ],
                modifiers: vec![],
                anchors: vec![],
                is_template_instance: false,
            }),
            span,
        )]);
        let mut warnings = Vec::new();
        check_redundant_constants(&doc, &mut warnings);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("center_y"));
    }

    // ── Reducible bend tests ──

    use super::super::types::ConnectionLayout;
    use crate::parser::ast::ConnectionDirection;

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
            name: None,
        }
    }

    fn make_layout_with_connections(connections: Vec<ConnectionLayout>) -> LayoutResult {
        LayoutResult {
            elements: HashMap::new(),
            root_elements: vec![],
            connections,
            bounds: BoundingBox::zero(),
            overridden_constraints: vec![],
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
        assert!(
            warnings[0].message.contains("20px"),
            "message: {}",
            warnings[0].message
        );
        assert!(
            warnings[0].message.contains("horizontally"),
            "message: {}",
            warnings[0].message
        );
        assert!(
            warnings[0].message.contains("further apart"),
            "message: {}",
            warnings[0].message
        );
        assert!(
            warnings[0].message.contains("at least"),
            "message: {}",
            warnings[0].message
        );
        assert!(
            warnings[0].message.contains("eliminate 2 corners"),
            "message: {}",
            warnings[0].message
        );
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
        let path = vec![Point::new(100.0, 100.0), Point::new(200.0, 100.0)];
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
        elem.label = Some(LabelLayout::from_source(
            label_text,
            Point::new(x + w / 2.0, y + h / 2.0),
            TextAnchor::Middle,
            14.0,
        ));
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
            overridden_constraints: vec![],
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
            overridden_constraints: vec![],
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
            overridden_constraints: vec![],
        };
        let mut warnings = Vec::new();
        check_label_overflow(&result, &mut warnings);
        assert!(warnings.is_empty());
    }
}

/// A `text` element positioned inside a filled shape is almost always a label
/// that was hand-placed with constraints.
///
/// Say so, and say what to write instead: agents act on a warning that
/// contains the replacement syntax far more often than on one that only
/// describes the symptom.
fn check_hand_placed_labels(
    result: &LayoutResult,
    doc: &Document,
    warnings: &mut Vec<LintWarning>,
    scope: &FrameScope<'_>,
) {
    let contains = collect_contains_ids(doc);

    struct Seen {
        id: String,
        bounds: BoundingBox,
        is_text: bool,
        /// This element's own id plus every ancestor's, so a `contains` on an
        /// enclosing group still counts as deliberate.
        lineage: Vec<String>,
    }

    fn walk(
        elem: &ElementLayout,
        lineage: &[String],
        out: &mut Vec<Seen>,
        scope: &FrameScope<'_>,
    ) {
        // Text parked in a box that is hidden whenever the text is shown is
        // not sitting on anything. A hidden element takes its whole subtree
        // out of this frame's question.
        if scope.hides_element(elem) {
            return;
        }
        let mut next = lineage.to_vec();
        if let Some(id) = &elem.id {
            next.push(id.0.clone());
            // A translucent zone is a background wash, not a box with a
            // label: text over it is an annotation and stays legitimate.
            // `is_opaque` exempts a translucent zone background. An element
            // that paints nothing at all sets no opacity, so it passed that
            // guard and the rule told authors to move text into the label of
            // an invisible sizing rect. Both exemptions are wanted.
            let labellable = is_visual_shape(elem)
                && !is_callout(elem)
                && is_opaque(elem)
                && !paints_nothing(elem);
            if is_text_shape(elem) || labellable {
                out.push(Seen {
                    id: id.0.clone(),
                    bounds: elem.bounds,
                    is_text: is_text_shape(elem),
                    lineage: next.clone(),
                });
            }
        }
        for child in &elem.children {
            walk(child, &next, out, scope);
        }
    }

    let mut seen = Vec::new();
    for elem in &result.root_elements {
        walk(elem, &[], &mut seen, scope);
    }

    for shape in seen.iter().filter(|s| !s.is_text) {
        // Group per box: three lines hand-placed in one card are one mistake
        // with one fix, not three warnings.
        let inside: Vec<&Seen> = seen
            .iter()
            .filter(|text| text.is_text)
            .filter(|text| shape.bounds.contains_bbox(&text.bounds))
            // A `contains` on the text, or on anything it sits inside, means
            // the author asked for this arrangement.
            .filter(|text| {
                !text
                    .lineage
                    .iter()
                    .any(|a| contains.wraps(Some(&shape.id), Some(a)))
            })
            .collect();

        if inside.is_empty() {
            continue;
        }

        let names = inside
            .iter()
            .map(|t| format!("\"{}\"", t.id))
            .collect::<Vec<_>>()
            .join(", ");
        let subject = if inside.len() == 1 { "text" } else { "texts" };
        warnings.push(LintWarning {
            category: LintCategory::Label,
            message: format!(
                "{subject} {names} sit inside \"{}\" — write {} [label: \"…\"] instead of \
                 positioning them (<br> for more lines, label_position: below for a caption)",
                shape.id, shape.id
            ),
            frames: Vec::new(),
            // Only a single text identifies a pair the overlap check could
            // also have reported.
            pair: (inside.len() == 1).then(|| sorted_pair(&shape.id, &inside[0].id)),
        });
    }
}

/// `<bold>` parses as literal text and renders as the characters `<bold>`.
/// That is a plausible-looking wrong picture, which is exactly what the label
/// markup subset exists to prevent.
fn check_label_markup(doc: &Document, warnings: &mut Vec<LintWarning>) {
    let mut labels = Vec::new();
    collect_raw_labels(&doc.statements, &mut labels);
    for (owner, raw) in labels {
        for tag in unrecognised_tags(&raw) {
            warnings.push(LintWarning {
                category: LintCategory::Label,
                message: format!(
                    "label on \"{owner}\" contains unsupported markup <{tag}>; \
                     supported: <br> <b> <i> <small> <span fill=…>"
                ),
                frames: Vec::new(),
                pair: None,
            });
        }
    }
}

/// Every `label:` in the document, with the name of the element carrying it.
fn collect_raw_labels(
    stmts: &[crate::parser::ast::Spanned<Statement>],
    out: &mut Vec<(String, String)>,
) {
    for stmt in stmts {
        let (modifiers, children, name) = match &stmt.node {
            Statement::Shape(s) => (&s.modifiers, None, s.name.as_ref().map(|n| n.node.to_string())),
            Statement::Layout(l) => (
                &l.modifiers,
                Some(&l.children),
                l.name.as_ref().map(|n| n.node.to_string()),
            ),
            Statement::Group(g) => (
                &g.modifiers,
                Some(&g.children),
                g.name.as_ref().map(|n| n.node.to_string()),
            ),
            _ => continue,
        };
        if let Some(raw) = modifiers.iter().find_map(|m| {
            if matches!(m.node.key.node, crate::parser::ast::StyleKey::Label) {
                match &m.node.value.node {
                    crate::parser::ast::StyleValue::String(s) => Some(s.clone()),
                    _ => None,
                }
            } else {
                None
            }
        }) {
            out.push((name.unwrap_or_else(|| "<anon>".to_string()), raw));
        }
        if let Some(children) = children {
            collect_raw_labels(children, out);
        }
    }
}

/// Tag-shaped runs in a label that the markup parser does not recognise.
///
/// A bare `<` with no `>` after it, or `<` followed by a non-letter, is
/// ordinary text like "a < b" and is not reported.
fn unrecognised_tags(raw: &str) -> Vec<String> {
    let chars: Vec<char> = raw.chars().collect();
    let mut found = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '<' {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        if chars.get(j) == Some(&'/') {
            j += 1;
        }
        let start = j;
        while j < chars.len() && chars[j].is_ascii_alphabetic() {
            j += 1;
        }
        let name: String = chars[start..j].iter().collect();
        if !name.is_empty()
            && chars.get(j) == Some(&'>')
            && !crate::layout::text::SUPPORTED_TAGS.contains(&name.to_ascii_lowercase().as_str())
            && !found.contains(&name)
        {
            found.push(name);
        }
        i += 1;
    }
    found
}
