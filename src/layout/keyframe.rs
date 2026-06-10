//! Keyframe processing for animated diagrams (Feature 011)
//!
//! Computes per-frame layout states from keyframe declarations.
//! Each keyframe produces a visibility set and optional transform overrides.
//! The diff engine compares each frame's layout against frame 0 to produce CSS.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::parser::ast::{Document, KeyframeDecl, KeyframeOp, Statement, StyleKey, StyleValue};
use super::config::LayoutConfig;
use super::types::{ConnectionLayout, ElementLayout, LayoutResult, ResolvedStyles};

/// Visibility and transform state for a single frame
#[derive(Debug, Clone)]
pub struct FrameState {
    /// Frame name (from keyframe declaration)
    pub name: String,
    /// Elements hidden in this frame (by ID)
    pub hidden_elements: HashSet<String>,
    /// Connections hidden in this frame (by name)
    pub hidden_connections: HashSet<String>,
    /// Per-element transform overrides (element_id -> style modifiers)
    pub transforms: HashMap<String, Vec<crate::parser::ast::Spanned<crate::parser::ast::StyleModifier>>>,
    /// If true, skip constraint re-solving for this frame
    pub no_resolve: bool,
}

/// Complete keyframe processing result
#[derive(Debug, Clone)]
pub struct KeyframeResult {
    /// Per-frame states, in order
    pub frames: Vec<FrameState>,
    /// Per-frame layout snapshots (frame 0 is the base)
    pub frame_layouts: Vec<FrameLayout>,
}

/// Layout snapshot for a single frame
#[derive(Debug, Clone)]
pub struct FrameLayout {
    pub name: String,
    /// Element positions/styles for this frame (element_id -> bounds + styles)
    pub element_diffs: BTreeMap<String, ElementDiff>,
    /// Connection visibility for this frame
    pub connection_diffs: BTreeMap<String, ConnectionDiff>,
}

/// Diff for a single element between frame N and frame 0
#[derive(Debug, Clone, Default)]
pub struct ElementDiff {
    /// Translate delta X (solved.x - base.x), emitted as transform on the wrapper group
    pub tx: Option<f64>,
    /// Translate delta Y (solved.y - base.y)
    pub ty: Option<f64>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub rotation: Option<f64>,
    pub opacity: Option<f64>,
    pub fill: Option<String>,
    pub stroke: Option<String>,
}

/// Diff for a connection between frame N and frame 0
#[derive(Debug, Clone)]
pub struct ConnectionDiff {
    pub opacity: Option<f64>,
}

impl ElementDiff {
    pub fn is_empty(&self) -> bool {
        self.tx.is_none()
            && self.ty.is_none()
            && self.width.is_none()
            && self.height.is_none()
            && self.rotation.is_none()
            && self.opacity.is_none()
            && self.fill.is_none()
            && self.stroke.is_none()
    }
}

/// Extract keyframe declarations from the document
pub fn extract_keyframes(doc: &Document) -> Vec<&KeyframeDecl> {
    doc.statements
        .iter()
        .filter_map(|stmt| match &stmt.node {
            Statement::Keyframe(kf) => Some(kf),
            _ => None,
        })
        .collect()
}

/// Compute cumulative frame states from keyframe declarations.
/// Each frame builds on the previous frame's state.
pub fn compute_frame_states(keyframes: &[&KeyframeDecl]) -> Vec<FrameState> {
    let mut frames = Vec::with_capacity(keyframes.len());
    let mut hidden_elements: HashSet<String> = HashSet::new();
    let mut hidden_connections: HashSet<String> = HashSet::new();
    // Cumulative transforms: element id -> merged modifiers (later keys override earlier).
    let mut cumulative_transforms: HashMap<String, Vec<crate::parser::ast::Spanned<crate::parser::ast::StyleModifier>>> = HashMap::new();

    for kf in keyframes {
        // Apply operations cumulatively
        for op in &kf.operations {
            match &op.node {
                KeyframeOp::Show(targets) => {
                    for target in targets {
                        hidden_elements.remove(&target.node.0);
                        hidden_connections.remove(&target.node.0);
                    }
                }
                KeyframeOp::Hide(targets) => {
                    for target in targets {
                        hidden_elements.insert(target.node.0.clone());
                        hidden_connections.insert(target.node.0.clone());
                    }
                }
                KeyframeOp::Transform { target, modifiers } => {
                    let entry = cumulative_transforms.entry(target.node.0.clone()).or_default();
                    for m in modifiers {
                        // per-property override: drop any earlier modifier with the same key
                        entry.retain(|existing| existing.node.key.node != m.node.key.node);
                        entry.push(m.clone());
                    }
                }
            }
        }

        frames.push(FrameState {
            name: kf.name.node.clone(),
            hidden_elements: hidden_elements.clone(),
            hidden_connections: hidden_connections.clone(),
            transforms: cumulative_transforms.clone(),
            no_resolve: kf.no_resolve,
        });
    }

    frames
}

/// Compute layout diffs for all frames against frame 0 (the base layout).
/// For frames with transforms, re-solves constraints and re-routes connections.
pub fn compute_frame_diffs(
    base_result: &LayoutResult,
    frame_states: &[FrameState],
    doc: &Document,
    config: &LayoutConfig,
) -> Vec<FrameLayout> {
    let mut frame_layouts = Vec::with_capacity(frame_states.len());

    // Collect all element IDs recursively from base result
    fn collect_all_elements<'a>(
        elements: &'a [ElementLayout],
        map: &mut HashMap<&'a str, &'a ElementLayout>,
    ) {
        for elem in elements {
            if let Some(id) = &elem.id {
                map.insert(id.0.as_str(), elem);
            }
            collect_all_elements(&elem.children, map);
        }
    }
    let mut base_elements: HashMap<&str, &ElementLayout> = HashMap::new();
    collect_all_elements(&base_result.root_elements, &mut base_elements);

    // Build connection lookup by name
    let base_connections: HashMap<&str, &ConnectionLayout> = base_result
        .connections
        .iter()
        .filter_map(|c| c.name.as_ref().map(|n| (n.0.as_str(), c)))
        .collect();

    // Frame 0 state determines which elements start hidden
    let frame0_hidden = if !frame_states.is_empty() {
        &frame_states[0].hidden_elements
    } else {
        return frame_layouts;
    };

    for state in frame_states {
        let mut element_diffs = BTreeMap::new();
        let mut connection_diffs = BTreeMap::new();

        // Re-solve layout for this frame (transforms + constraint cascading)
        let solved_result = if !state.transforms.is_empty() {
            resolve_frame_layout(base_result, state, doc, config)
        } else {
            None
        };

        // Build element map for the solved frame (if re-solved)
        let solved_elements = if let Some(ref solved) = solved_result {
            let mut map = HashMap::new();
            collect_all_elements(&solved.root_elements, &mut map);
            Some(map)
        } else {
            None
        };

        // Compute element diffs
        for (id, base_elem) in &base_elements {
            let hidden_in_frame0 = frame0_hidden.contains(*id);
            let hidden_in_this_frame = state.hidden_elements.contains(*id);

            // Position/style diff from transforms. Computed whenever the
            // element is visible in this frame — including when it is being
            // shown this frame (so show + transform keeps the transform).
            let mut diff = if !hidden_in_this_frame {
                solved_elements
                    .as_ref()
                    .and_then(|m| m.get(id))
                    .map(|solved_elem| diff_element(base_elem, solved_elem))
                    .unwrap_or_default()
            } else {
                ElementDiff::default()
            };

            // Visibility diff. Hide always forces opacity 0; show forces
            // opacity 1 unless a transform in this frame set opacity explicitly.
            if hidden_in_frame0 != hidden_in_this_frame {
                if hidden_in_this_frame {
                    diff.opacity = Some(0.0);
                } else if diff.opacity.is_none() {
                    diff.opacity = Some(1.0);
                }
            }

            if !diff.is_empty() {
                element_diffs.insert(id.to_string(), diff);
            }
        }

        // Compute connection visibility diffs
        for (name, _conn) in &base_connections {
            let hidden_in_frame0 = frame_states[0].hidden_connections.contains(*name);
            let hidden_in_this_frame = state.hidden_connections.contains(*name);

            if hidden_in_frame0 != hidden_in_this_frame {
                connection_diffs.insert(
                    name.to_string(),
                    ConnectionDiff {
                        opacity: Some(if hidden_in_this_frame { 0.0 } else { 1.0 }),
                    },
                );
            }
        }

        frame_layouts.push(FrameLayout {
            name: state.name.clone(),
            element_diffs,
            connection_diffs,
        });
    }

    frame_layouts
}

/// Public entry point for static frame rendering with transforms.
pub fn resolve_frame_for_static(
    base_result: &LayoutResult,
    state: &FrameState,
    doc: &Document,
    config: &LayoutConfig,
) -> Option<LayoutResult> {
    resolve_frame_layout(base_result, state, doc, config)
}

/// Re-solve layout for a single frame with transform overrides applied.
/// By default, re-solves constraints so dependents follow moved elements.
/// Constraints targeting transformed element properties are replaced with
/// the transform values, so the solver moves dependents correctly.
/// With `no_resolve`, just applies transforms directly (style-only use case).
fn resolve_frame_layout(
    base_result: &LayoutResult,
    state: &FrameState,
    doc: &Document,
    config: &LayoutConfig,
) -> Option<LayoutResult> {
    let mut result = base_result.clone();

    // Apply transform modifiers to target elements
    for (elem_id, modifiers) in &state.transforms {
        apply_transform_to_element(&mut result.root_elements, elem_id, modifiers);
    }

    // Rebuild the element index so the solver sees updated positions
    result.rebuild_index();

    if !state.no_resolve {
        // Build a modified document where constraints on transformed elements'
        // geometry properties are replaced with the transform values.
        let modified_doc = rewrite_constraints_for_transforms(doc, state);

        // Re-solve constraints using modified document.
        // Transformed positions are now baked into constraints,
        // so dependents cascade correctly.
        if let Err(_e) = super::engine::resolve_constrain_statements(&mut result, &modified_doc, config) {
            // If solving fails, fall back to direct transform (no cascading)
        }
        if let Err(_e) = super::engine::resolve_constraints(&mut result, &modified_doc, None) {
            // Same fallback
        }
    }

    // Recompute bounds after all position changes
    result.compute_bounds();

    // Re-route connections against updated element positions
    result.connections.clear();
    if let Err(_e) = super::routing::route_connections(&mut result, doc) {
        return None;
    }

    Some(result)
}

/// Remove constraints that directly position transformed elements,
/// so the solver uses the transformed positions (from element bounds)
/// as SUGGESTED values and cascades to dependents.
fn rewrite_constraints_for_transforms(doc: &Document, state: &FrameState) -> Document {
    use crate::parser::ast::*;

    // Collect element IDs that have geometry transforms (x, y, width, height)
    let mut geometry_transformed: HashSet<&str> = HashSet::new();
    for (elem_id, modifiers) in &state.transforms {
        let has_geometry = modifiers.iter().any(|m| matches!(
            m.node.key.node,
            StyleKey::X | StyleKey::Y | StyleKey::Width | StyleKey::Height
                | StyleKey::Dx | StyleKey::Dy | StyleKey::Scale
        ));
        if has_geometry {
            geometry_transformed.insert(elem_id.as_str());
        }
    }

    if geometry_transformed.is_empty() {
        return doc.clone();
    }

    // Clone document and remove constrain statements whose LHS
    // directly targets a geometry-transformed element
    let mut new_doc = doc.clone();
    new_doc.statements.retain(|stmt| {
        if let Statement::Constrain(constrain) = &stmt.node {
            // Check if the LHS references a transformed element
            if let Some(elem_name) = get_constraint_lhs_element(&constrain.expr) {
                if geometry_transformed.contains(elem_name.as_str()) {
                    return false; // Drop this constraint
                }
            }
        }
        true
    });

    new_doc
}

/// Extract the element name from the LHS of a constraint expression.
fn get_constraint_lhs_element(expr: &crate::parser::ast::ConstraintExpr) -> Option<String> {
    use crate::parser::ast::ConstraintExpr;
    match expr {
        ConstraintExpr::Equal { left, .. }
        | ConstraintExpr::EqualWithOffset { left, .. }
        | ConstraintExpr::Constant { left, .. }
        | ConstraintExpr::GreaterOrEqual { left, .. }
        | ConstraintExpr::LessOrEqual { left, .. } => {
            Some(left.element.node.leaf().0.clone())
        }
        ConstraintExpr::Midpoint { target, .. } => {
            Some(target.element.node.leaf().0.clone())
        }
        ConstraintExpr::Contains { container, .. } => {
            Some(container.node.0.clone())
        }
    }
}

/// Apply transform modifiers to a specific element in the tree
fn apply_transform_to_element(
    elements: &mut [ElementLayout],
    target_id: &str,
    modifiers: &[crate::parser::ast::Spanned<crate::parser::ast::StyleModifier>],
) {
    for elem in elements.iter_mut() {
        if elem.id.as_ref().map_or(false, |id| id.0 == target_id) {
            apply_modifiers_ordered(elem, modifiers);
            return;
        }
        // Recurse into children
        apply_transform_to_element(&mut elem.children, target_id, modifiers);
    }
}

/// Apply transform modifiers in a fixed order against the element's base bounds:
/// 1) absolutes + visual, 2) dx/dy deltas, 3) scale about center.
fn apply_modifiers_ordered(
    elem: &mut ElementLayout,
    modifiers: &[crate::parser::ast::Spanned<crate::parser::ast::StyleModifier>],
) {
    let num = |v: &StyleValue| -> Option<f64> {
        if let StyleValue::Number { value, .. } = v { Some(*value) } else { None }
    };

    // Pass 1: absolutes + visual
    for m in modifiers {
        match &m.node.key.node {
            StyleKey::Rotation => { if let Some(v) = num(&m.node.value.node) { elem.styles.rotation = Some(v); } }
            StyleKey::Fill => { elem.styles.fill = ResolvedStyles::color_to_css(&m.node.value.node); }
            StyleKey::Stroke => { elem.styles.stroke = ResolvedStyles::color_to_css(&m.node.value.node); }
            StyleKey::Opacity => { if let Some(v) = num(&m.node.value.node) { elem.styles.opacity = Some(v); } }
            StyleKey::Width => { if let Some(v) = num(&m.node.value.node) { elem.bounds.width = v; } }
            StyleKey::Height => { if let Some(v) = num(&m.node.value.node) { elem.bounds.height = v; } }
            StyleKey::X => { if let Some(v) = num(&m.node.value.node) { elem.bounds.x = v; } }
            StyleKey::Y => { if let Some(v) = num(&m.node.value.node) { elem.bounds.y = v; } }
            _ => {}
        }
    }
    // Pass 2: deltas
    for m in modifiers {
        match &m.node.key.node {
            StyleKey::Dx => { if let Some(v) = num(&m.node.value.node) { elem.bounds.x += v; } }
            StyleKey::Dy => { if let Some(v) = num(&m.node.value.node) { elem.bounds.y += v; } }
            _ => {}
        }
    }
    // Pass 3: scale about center
    for m in modifiers {
        if let StyleKey::Scale = &m.node.key.node {
            if let Some(s) = num(&m.node.value.node) {
                let nw = elem.bounds.width * s;
                let nh = elem.bounds.height * s;
                elem.bounds.x -= (nw - elem.bounds.width) / 2.0;
                elem.bounds.y -= (nh - elem.bounds.height) / 2.0;
                elem.bounds.width = nw;
                elem.bounds.height = nh;
            }
        }
    }
}

/// Compute the diff between two element states
fn diff_element(base: &ElementLayout, solved: &ElementLayout) -> ElementDiff {
    let mut diff = ElementDiff::default();
    let eps = 0.1; // Sub-pixel threshold

    if (base.bounds.x - solved.bounds.x).abs() > eps {
        diff.tx = Some(solved.bounds.x - base.bounds.x);
    }
    if (base.bounds.y - solved.bounds.y).abs() > eps {
        diff.ty = Some(solved.bounds.y - base.bounds.y);
    }
    if (base.bounds.width - solved.bounds.width).abs() > eps {
        diff.width = Some(solved.bounds.width);
    }
    if (base.bounds.height - solved.bounds.height).abs() > eps {
        diff.height = Some(solved.bounds.height);
    }

    let base_rot = base.styles.rotation.unwrap_or(0.0);
    let solved_rot = solved.styles.rotation.unwrap_or(0.0);
    if (base_rot - solved_rot).abs() > eps {
        diff.rotation = Some(solved_rot);
    }

    let base_opacity = base.styles.opacity.unwrap_or(1.0);
    let solved_opacity = solved.styles.opacity.unwrap_or(1.0);
    if (base_opacity - solved_opacity).abs() > f64::EPSILON {
        diff.opacity = Some(solved_opacity);
    }

    if base.styles.fill != solved.styles.fill {
        diff.fill = solved.styles.fill.clone();
    }
    if base.styles.stroke != solved.styles.stroke {
        diff.stroke = solved.styles.stroke.clone();
    }

    diff
}

/// Get the set of visible element IDs for a given frame.
/// Used by the linter for per-frame overlap detection.
pub fn visible_elements_in_frame(
    all_element_ids: &HashSet<String>,
    frame_state: &FrameState,
) -> HashSet<String> {
    all_element_ids
        .difference(&frame_state.hidden_elements)
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::*;
    use crate::parser::Spanned;

    fn make_id(name: &str) -> Spanned<Identifier> {
        Spanned::new(Identifier(name.to_string()), 0..0)
    }

    /// Build a minimal rectangle ElementLayout for geometry tests.
    fn mk_elem(id: &str, x: f64, y: f64, w: f64, h: f64) -> ElementLayout {
        use crate::layout::types::{AnchorSet, BoundingBox, ElementType};
        use crate::parser::ast::ShapeType;
        ElementLayout {
            id: Some(Identifier(id.to_string())),
            element_type: ElementType::Shape(ShapeType::Rectangle),
            bounds: BoundingBox { x, y, width: w, height: h },
            styles: ResolvedStyles::default(),
            children: Vec::new(),
            label: None,
            anchors: AnchorSet::default(),
            path_normalize: false,
            z_order: 0,
        }
    }

    fn modi(key: StyleKey, v: f64) -> Spanned<StyleModifier> {
        Spanned::new(StyleModifier {
            key: Spanned::new(key, 0..0),
            value: Spanned::new(StyleValue::Number { value: v, unit: None }, 0..0),
        }, 0..0)
    }

    #[test]
    fn scale_grows_about_center() {
        let mut elem = mk_elem("e", 100.0, 100.0, 200.0, 100.0); // center (200,150)
        let mods = vec![modi(StyleKey::Scale, 2.0)];
        apply_transform_to_element(std::slice::from_mut(&mut elem), "e", &mods);
        assert!((elem.bounds.width - 400.0).abs() < 0.001, "width {}", elem.bounds.width);
        assert!((elem.bounds.height - 200.0).abs() < 0.001, "height {}", elem.bounds.height);
        assert!((elem.bounds.x - 0.0).abs() < 0.001, "x {}", elem.bounds.x);   // 100 - (400-200)/2
        assert!((elem.bounds.y - 50.0).abs() < 0.001, "y {}", elem.bounds.y);  // 100 - (200-100)/2
    }

    #[test]
    fn dx_dy_offset_from_base_after_absolute() {
        let mut elem = mk_elem("e", 10.0, 10.0, 5.0, 5.0);
        // absolute x=100 then dx=5 => 105, regardless of modifier declaration order
        let mods = vec![modi(StyleKey::Dx, 5.0), modi(StyleKey::X, 100.0)];
        apply_transform_to_element(std::slice::from_mut(&mut elem), "e", &mods);
        assert!((elem.bounds.x - 105.0).abs() < 0.001, "x {}", elem.bounds.x);
    }

    #[test]
    fn transforms_persist_and_merge_forward() {
        let kf1 = make_keyframe("a", vec![
            KeyframeOp::Transform {
                target: make_id("box"),
                modifiers: vec![modi(StyleKey::Width, 360.0)],
            },
        ]);
        let kf2 = make_keyframe("b", vec![
            KeyframeOp::Transform {
                target: make_id("box"),
                modifiers: vec![modi(StyleKey::X, 120.0)],
            },
        ]);
        let states = compute_frame_states(&[&kf1, &kf2]);
        // Frame b must still carry the width from frame a, plus its own x.
        let box_mods = states[1].transforms.get("box").expect("box transformed in frame b");
        let keys: Vec<&StyleKey> = box_mods.iter().map(|m| &m.node.key.node).collect();
        assert!(keys.contains(&&StyleKey::Width), "width persists into frame b, got {:?}", keys);
        assert!(keys.contains(&&StyleKey::X), "x added in frame b");
    }

    fn make_keyframe(name: &str, ops: Vec<KeyframeOp>) -> KeyframeDecl {
        KeyframeDecl {
            name: Spanned::new(name.to_string(), 0..0),
            operations: ops
                .into_iter()
                .map(|op| Spanned::new(op, 0..0))
                .collect(),
            no_resolve: false,
        }
    }

    #[test]
    fn test_cumulative_show_hide() {
        let kf1 = make_keyframe("startup", vec![
            KeyframeOp::Hide(vec![make_id("a"), make_id("b"), make_id("c")]),
        ]);
        let kf2 = make_keyframe("step1", vec![
            KeyframeOp::Show(vec![make_id("a")]),
        ]);
        let kf3 = make_keyframe("step2", vec![
            KeyframeOp::Show(vec![make_id("b")]),
            KeyframeOp::Hide(vec![make_id("a")]),
        ]);

        let keyframes: Vec<&KeyframeDecl> = vec![&kf1, &kf2, &kf3];
        let states = compute_frame_states(&keyframes);

        assert_eq!(states.len(), 3);

        // Frame 0 (startup): a, b, c all hidden
        assert!(states[0].hidden_elements.contains("a"));
        assert!(states[0].hidden_elements.contains("b"));
        assert!(states[0].hidden_elements.contains("c"));

        // Frame 1 (step1): a shown, b and c still hidden
        assert!(!states[1].hidden_elements.contains("a"));
        assert!(states[1].hidden_elements.contains("b"));
        assert!(states[1].hidden_elements.contains("c"));

        // Frame 2 (step2): b shown, a re-hidden, c still hidden
        assert!(states[2].hidden_elements.contains("a"));
        assert!(!states[2].hidden_elements.contains("b"));
        assert!(states[2].hidden_elements.contains("c"));
    }

    #[test]
    fn test_empty_keyframes() {
        let states = compute_frame_states(&[]);
        assert!(states.is_empty());
    }

    #[test]
    fn test_show_without_prior_hide() {
        // Showing something that was never hidden should be a no-op
        let kf = make_keyframe("test", vec![
            KeyframeOp::Show(vec![make_id("a")]),
        ]);
        let states = compute_frame_states(&[&kf]);
        assert!(!states[0].hidden_elements.contains("a"));
    }

    #[test]
    fn test_connection_visibility() {
        let kf1 = make_keyframe("startup", vec![
            KeyframeOp::Hide(vec![make_id("conn1")]),
        ]);
        let kf2 = make_keyframe("reveal", vec![
            KeyframeOp::Show(vec![make_id("conn1")]),
        ]);

        let states = compute_frame_states(&[&kf1, &kf2]);
        assert!(states[0].hidden_connections.contains("conn1"));
        assert!(!states[1].hidden_connections.contains("conn1"));
    }
}
