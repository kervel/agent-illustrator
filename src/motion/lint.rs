//! Motion lints: choreography that cannot do what it says, or says it badly.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::layout::keyframe::FrameState;
use crate::layout::lint::{LintCategory, LintWarning};
use crate::layout::types::{ElementLayout, LayoutResult};
use crate::parser::ast::{ConstraintExpr, Document, KeyframeOp, StyleKey};

use super::compile::Motion;

/// A beat longer than this drags.
pub const SLOW_BEAT: f64 = 2.5;
/// More focal movements than this at once and the eye cannot follow.
pub const BUSY_BEAT: usize = 2;

fn warn(category: LintCategory, frame: &str, message: String) -> LintWarning {
    LintWarning {
        category,
        message,
        frames: if frame.is_empty() { vec![] } else { vec![frame.to_string()] },
        pair: None,
    }
}

/// Element -> parent id, for "hidden because its container is".
fn parents(elems: &[ElementLayout], parent: Option<&str>, out: &mut HashMap<String, String>) {
    for e in elems {
        let me = e.id.as_ref().map(|i| i.0.as_str());
        if let (Some(m), Some(p)) = (me, parent) {
            out.insert(m.to_string(), p.to_string());
        }
        parents(&e.children, me.or(parent), out);
    }
}

fn hidden_in(state: &FrameState, id: &str, parent_of: &HashMap<String, String>) -> bool {
    let mut cur = Some(id.to_string());
    while let Some(c) = cur {
        if state.hidden_elements.contains(&c) {
            return true;
        }
        cur = parent_of.get(&c).cloned();
    }
    false
}

pub fn check(
    m: &Motion,
    doc: &Document,
    states: &[FrameState],
    base: &LayoutResult,
) -> Vec<LintWarning> {
    let mut out = Vec::new();
    let show = |id: &str| m.display.get(id).cloned().unwrap_or_else(|| id.to_string());
    let mut parent_of = HashMap::new();
    parents(&base.root_elements, None, &mut parent_of);

    // Numbers where names belong.
    for kf in crate::layout::keyframe::extract_keyframes(doc) {
        // In a keyframe written with motion statements a number is a
        // mistake; a legacy keyframe that pins by number still works, so
        // there it is only a warning (it must not start failing CI).
        let coord_cat = if uses_motion(&kf.motion) { LintCategory::MotionCoordinate } else { LintCategory::Motion };
        for op in &kf.operations {
            match &op.node {
                KeyframeOp::Transform { target, modifiers } => {
                    for md in modifiers {
                        if matches!(md.node.key.node, StyleKey::X | StyleKey::Y | StyleKey::Dx | StyleKey::Dy) {
                            out.push(warn(
                                coord_cat,
                                &kf.name.node,
                                format!(
                                    "transform {} [{:?}: ...] places it by a number; say where by name \
                                     (`move {} to <element>`, a constraint to another element)",
                                    show(&target.node.0),
                                    md.node.key.node,
                                    show(&target.node.0)
                                )
                                .replace("X:", "x:")
                                .replace("Y:", "y:")
                                .replace("Dx:", "dx:")
                                .replace("Dy:", "dy:"),
                            ));
                        }
                    }
                }
                KeyframeOp::Constrain(d) => {
                    if let ConstraintExpr::Constant { left, value } = &d.expr {
                        out.push(warn(
                            coord_cat,
                            &kf.name.node,
                            format!(
                                "constrain {}.{:?} = {} in a keyframe pins it to a coordinate; \
                                 pin it to an element instead",
                                show(&left.element.node.leaf().0),
                                left.property.node,
                                value
                            ),
                        ));
                    }
                }
                _ => {}
            }
        }
    }

    for (fi, f) in m.frames.iter().enumerate() {
        let end = &states[fi];
        let before = if fi > 0 { Some(&states[fi - 1]) } else { None };
        let hidden_throughout =
            |id: &str| hidden_in(end, id, &parent_of) && before.is_none_or(|b| hidden_in(b, id, &parent_of));

        // A slow beat.
        if f.duration > SLOW_BEAT {
            out.push(warn(
                LintCategory::Motion,
                &f.name,
                format!(
                    "keyframe \"{}\" runs {:.1}s; keep a beat under {:.1}s (split it, or use `[auto]` for a follow-on keyframe)",
                    f.name, f.duration, SLOW_BEAT
                ),
            ));
        }

        let mut seen: HashSet<(String, String)> = HashSet::new();
        for a in &f.atoms {
            // Moving something nobody can see.
            let acts_on_visible = matches!(
                a.kind.as_str(),
                "pulse" | "shake" | "flash" | "ping" | "highlight" | "nudge" | "loop" | "move" | "transform"
            );
            if acts_on_visible && m.display.contains_key(&a.target_id) && hidden_throughout(&a.target_id) {
                if seen.insert((a.kind.clone(), a.target_id.clone())) {
                    out.push(warn(
                        LintCategory::Motion,
                        &f.name,
                        format!("{} on {}, which is hidden the whole keyframe: nobody sees it", a.kind, show(&a.target_id)),
                    ));
                }
            }
            // Flying to a place that is not there.
            if a.kind == "fly" {
                if let Some(d) = &a.dest_id {
                    if hidden_in(end, d, &parent_of) && before.is_none_or(|b| hidden_in(b, d, &parent_of)) {
                        let shown_here = f.atoms.iter().any(|b| b.kind == "show" && &b.target_id == d);
                        if !shown_here {
                            out.push(warn(
                                LintCategory::Motion,
                                &f.name,
                                format!("fly lands on {}, which stays hidden: show it as the flight arrives", show(d)),
                            ));
                        }
                    }
                }
            }
        }

        // An arrow to nothing.
        for c in &base.connections {
            let visible = c.name.as_ref().is_none_or(|n| !end.hidden_connections.contains(&n.0));
            if !visible {
                continue;
            }
            for endpoint in [&c.from_id.0, &c.to_id.0] {
                if hidden_in(end, endpoint, &parent_of) {
                    let what = c.name.as_ref().map(|n| show(&n.0)).unwrap_or_else(|| {
                        format!("{} -> {}", show(&c.from_id.0), show(&c.to_id.0))
                    });
                    out.push(warn(
                        LintCategory::Motion,
                        &f.name,
                        format!("connection {} is on screen but its end {} is hidden", what, show(endpoint)),
                    ));
                }
            }
        }

        // Too much at once: focal movements (flights, moves, draws, swaps,
        // camera) whose windows overlap.
        let mut windows: BTreeMap<(usize, usize), (f64, f64, String)> = BTreeMap::new();
        for a in &f.atoms {
            if !matches!(a.kind.as_str(), "fly" | "move" | "draw" | "undraw" | "swap" | "camera") {
                continue;
            }
            let key = (a.span.start, a.span.end);
            let e = windows.entry(key).or_insert((a.start, a.start + a.dur, a.verb.clone()));
            e.0 = e.0.min(a.start);
            e.1 = e.1.max(a.start + a.dur);
        }
        let list: Vec<&(f64, f64, String)> = windows.values().collect();
        let mut worst = (0usize, 0.0f64);
        for w in &list {
            let t = w.0 + 1e-6;
            let n = list.iter().filter(|o| o.0 <= t && t < o.1).count();
            if n > worst.0 {
                worst = (n, w.0);
            }
        }
        if worst.0 > BUSY_BEAT {
            out.push(warn(
                LintCategory::Motion,
                &f.name,
                format!(
                    "{} focal movements at once around {:.2}s; one focal movement per beat (and one supporting) reads best — put the rest in `then {{ }}`",
                    worst.0, worst.1
                ),
            ));
        }
    }
    out
}

/// Whether a keyframe is written with motion (beats, motion-only verbs,
/// timing) rather than as a plain show/hide/transform state change.
fn uses_motion(nodes: &[crate::parser::ast::Spanned<crate::parser::ast::MotionNode>]) -> bool {
    use crate::parser::ast::{MotionNode, MotionVerb, StyleKey};
    nodes.iter().any(|n| match &n.node {
        MotionNode::Then(_) | MotionNode::After(..) | MotionNode::At(..) => true,
        MotionNode::Stmt(st) => {
            !st.opts.is_empty()
                || match &st.verb {
                    MotionVerb::Show(_)
                    | MotionVerb::Hide(_)
                    | MotionVerb::Constrain(_)
                    | MotionVerb::Disable(_)
                    | MotionVerb::Enable(_) => false,
                    MotionVerb::Transform { modifiers, .. } => modifiers.iter().any(|m| {
                        matches!(&m.node.key.node, StyleKey::Custom(k) if crate::motion::TIMING_KEYS.contains(&k.as_str()))
                    }),
                    _ => true,
                }
        }
    })
}
