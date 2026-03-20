//! Keyframe processing for animated diagrams (Feature 011)
//!
//! Computes per-frame layout states from keyframe declarations.
//! Each keyframe produces a visibility set and optional transform overrides.
//! The diff engine compares each frame's layout against frame 0 to produce CSS.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::parser::ast::{Document, KeyframeDecl, KeyframeOp, Statement};
use super::types::{ConnectionLayout, ElementLayout, LayoutResult};

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
/// Takes the base LayoutResult and frame states, produces per-frame diffs.
pub fn compute_frame_diffs(
    base_result: &LayoutResult,
    frame_states: &[FrameState],
) -> Vec<FrameLayout> {
    let mut frame_layouts = Vec::with_capacity(frame_states.len());

    // Build element lookup from base result
    let _base_elements: HashMap<&str, &ElementLayout> = base_result
        .root_elements
        .iter()
        .filter_map(|e| e.id.as_ref().map(|id| (id.0.as_str(), e)))
        .collect();

    // Also collect children recursively
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
    let mut all_elements: HashMap<&str, &ElementLayout> = HashMap::new();
    collect_all_elements(&base_result.root_elements, &mut all_elements);

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
        // No keyframes — nothing to diff
        return frame_layouts;
    };

    for state in frame_states {
        let mut element_diffs = BTreeMap::new();
        let mut connection_diffs = BTreeMap::new();

        // Compute element visibility diffs
        for (id, _elem) in &all_elements {
            let hidden_in_frame0 = frame0_hidden.contains(*id);
            let hidden_in_this_frame = state.hidden_elements.contains(*id);

            if hidden_in_frame0 != hidden_in_this_frame {
                let diff = ElementDiff {
                    opacity: Some(if hidden_in_this_frame { 0.0 } else { 1.0 }),
                    ..Default::default()
                };
                element_diffs.insert(id.to_string(), diff);
            } else if !hidden_in_this_frame {
                // Element is visible — check for transforms
                // TODO: Phase 3.3 - apply transform overrides and re-solve constraints
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
