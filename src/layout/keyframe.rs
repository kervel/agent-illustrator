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
    pub x: Option<f64>,
    pub y: Option<f64>,
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
        self.x.is_none()
            && self.y.is_none()
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
                KeyframeOp::Transform { .. } => {
                    // Transforms don't affect visibility
                }
            }
        }

        // Collect transforms for this frame
        let mut transforms = HashMap::new();
        for op in &kf.operations {
            if let KeyframeOp::Transform { target, modifiers } = &op.node {
                transforms.insert(target.node.0.clone(), modifiers.clone());
            }
        }

        frames.push(FrameState {
            name: kf.name.node.clone(),
            hidden_elements: hidden_elements.clone(),
            hidden_connections: hidden_connections.clone(),
            transforms,
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

        // If this frame has transforms, re-solve the layout
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

            // Visibility diff
            if hidden_in_frame0 != hidden_in_this_frame {
                element_diffs.insert(id.to_string(), ElementDiff {
                    opacity: Some(if hidden_in_this_frame { 0.0 } else { 1.0 }),
                    ..Default::default()
                });
            } else if !hidden_in_this_frame {
                // Element is visible — check for position/style diffs from transforms
                if let Some(ref solved_map) = solved_elements {
                    if let Some(solved_elem) = solved_map.get(id) {
                        let diff = diff_element(base_elem, solved_elem);
                        if !diff.is_empty() {
                            element_diffs.insert(id.to_string(), diff);
                        }
                    }
                }
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
/// Style-only changes (fill, stroke, opacity) are applied directly.
/// Geometry changes (x, y, width, height, rotation) are applied after
/// constraint solving so arrows re-route correctly.
fn resolve_frame_layout(
    base_result: &LayoutResult,
    state: &FrameState,
    doc: &Document,
    _config: &LayoutConfig,
) -> Option<LayoutResult> {
    let mut result = base_result.clone();

    // First re-solve constraints to get canonical positions
    // (this is a no-op if positions haven't changed, but needed for routing)
    // We intentionally do NOT re-solve here — positions come from the base.
    // Instead, apply geometry transforms AFTER the base solve,
    // then re-route connections against the new positions.

    // Apply ALL transform modifiers (style + geometry) to target elements
    for (elem_id, modifiers) in &state.transforms {
        apply_transform_to_element(&mut result.root_elements, elem_id, modifiers);
    }

    // Recompute bounds after geometry changes
    result.compute_bounds();

    // Re-route connections against updated element positions
    result.connections.clear();
    if let Err(_e) = super::routing::route_connections(&mut result, doc) {
        return None;
    }

    Some(result)
}

/// Apply transform modifiers to a specific element in the tree
fn apply_transform_to_element(
    elements: &mut [ElementLayout],
    target_id: &str,
    modifiers: &[crate::parser::ast::Spanned<crate::parser::ast::StyleModifier>],
) {
    for elem in elements.iter_mut() {
        if elem.id.as_ref().map_or(false, |id| id.0 == target_id) {
            // Apply style modifiers
            for modifier in modifiers {
                match &modifier.node.key.node {
                    StyleKey::Rotation => {
                        if let StyleValue::Number { value, .. } = &modifier.node.value.node {
                            elem.styles.rotation = Some(*value);
                        }
                    }
                    StyleKey::Fill => {
                        elem.styles.fill = ResolvedStyles::color_to_css(&modifier.node.value.node);
                    }
                    StyleKey::Stroke => {
                        elem.styles.stroke = ResolvedStyles::color_to_css(&modifier.node.value.node);
                    }
                    StyleKey::Opacity => {
                        if let StyleValue::Number { value, .. } = &modifier.node.value.node {
                            elem.styles.opacity = Some(*value);
                        }
                    }
                    StyleKey::Width => {
                        if let StyleValue::Number { value, .. } = &modifier.node.value.node {
                            elem.bounds.width = *value;
                        }
                    }
                    StyleKey::Height => {
                        if let StyleValue::Number { value, .. } = &modifier.node.value.node {
                            elem.bounds.height = *value;
                        }
                    }
                    StyleKey::X => {
                        if let StyleValue::Number { value, .. } = &modifier.node.value.node {
                            elem.bounds.x = *value;
                        }
                    }
                    StyleKey::Y => {
                        if let StyleValue::Number { value, .. } = &modifier.node.value.node {
                            elem.bounds.y = *value;
                        }
                    }
                    _ => {} // Other modifiers ignored for now
                }
            }
            return;
        }
        // Recurse into children
        apply_transform_to_element(&mut elem.children, target_id, modifiers);
    }
}

/// Compute the diff between two element states
fn diff_element(base: &ElementLayout, solved: &ElementLayout) -> ElementDiff {
    let mut diff = ElementDiff::default();
    let eps = 0.1; // Sub-pixel threshold

    if (base.bounds.x - solved.bounds.x).abs() > eps {
        diff.x = Some(solved.bounds.x);
    }
    if (base.bounds.y - solved.bounds.y).abs() > eps {
        diff.y = Some(solved.bounds.y);
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

    fn make_keyframe(name: &str, ops: Vec<KeyframeOp>) -> KeyframeDecl {
        KeyframeDecl {
            name: Spanned::new(name.to_string(), 0..0),
            operations: ops
                .into_iter()
                .map(|op| Spanned::new(op, 0..0))
                .collect(),
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
