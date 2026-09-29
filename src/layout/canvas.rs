//! The stage: `rect stage [width: 1600, height: 900, canvas: true]`.
//!
//! A slide has a fixed size. Marking the stage as the canvas makes it the
//! viewBox (nothing grows the picture), and anything that leaves it in any
//! frame is reported instead of silently widening the slide.

use std::collections::{BTreeMap, HashSet};

use crate::layout::keyframe::FrameState;
use crate::layout::lint::{LintCategory, LintWarning};
use crate::layout::types::{BoundingBox, ElementLayout, LayoutResult};
use crate::parser::ast::{Document, Spanned, Statement, StyleKey, StyleModifier, StyleValue};

fn is_canvas(mods: &[Spanned<StyleModifier>]) -> bool {
    mods.iter().any(|m| {
        matches!(&m.node.key.node, StyleKey::Custom(k) if k == "canvas")
            && match &m.node.value.node {
                StyleValue::Keyword(v) | StyleValue::String(v) => v != "false",
                StyleValue::Identifier(v) => v.0 != "false",
                StyleValue::Number { value, .. } => *value != 0.0,
                _ => true,
            }
    })
}

/// The id of the element declared as the canvas, if any.
pub fn canvas_id(doc: &Document) -> Option<String> {
    fn find(stmts: &[Spanned<Statement>]) -> Option<String> {
        for st in stmts {
            match &st.node {
                Statement::Shape(s) if is_canvas(&s.modifiers) => {
                    return s.name.as_ref().map(|n| n.node.0.clone());
                }
                Statement::Group(g) => {
                    if let Some(f) = find(&g.children) {
                        return Some(f);
                    }
                }
                Statement::Layout(l) => {
                    if let Some(f) = find(&l.children) {
                        return Some(f);
                    }
                }
                _ => {}
            }
        }
        None
    }
    find(&doc.statements)
}

/// Pin the picture's bounds to the canvas.
pub fn apply(result: &mut LayoutResult, canvas: &str) {
    if let Some(c) = result.get_element_by_name(canvas) {
        result.bounds = c.bounds;
    }
}

fn leaves<'a>(
    elems: &'a [ElementLayout],
    hidden: &HashSet<String>,
    out: &mut Vec<&'a ElementLayout>,
) {
    for e in elems {
        if e.id.as_ref().is_some_and(|i| hidden.contains(&i.0)) {
            continue;
        }
        if e.children.is_empty() {
            out.push(e);
        } else {
            leaves(&e.children, hidden, out);
        }
    }
}

fn overflow(b: &BoundingBox, c: &BoundingBox) -> Option<String> {
    let mut parts = Vec::new();
    let tol = 1.0;
    if b.x < c.x - tol {
        parts.push(format!("{:.0}px past the left edge", c.x - b.x));
    }
    if b.right() > c.right() + tol {
        parts.push(format!("{:.0}px past the right edge", b.right() - c.right()));
    }
    if b.y < c.y - tol {
        parts.push(format!("{:.0}px past the top edge", c.y - b.y));
    }
    if b.bottom() > c.bottom() + tol {
        parts.push(format!("{:.0}px past the bottom edge", b.bottom() - c.bottom()));
    }
    (!parts.is_empty()).then(|| parts.join(", "))
}

/// Everything that leaves the canvas, per frame.
pub fn check(
    result: &LayoutResult,
    doc: &Document,
    config: &crate::layout::LayoutConfig,
    display: &dyn Fn(&str) -> String,
) -> Vec<LintWarning> {
    let Some(canvas) = canvas_id(doc) else { return vec![] };
    let keyframes = crate::layout::keyframe::extract_keyframes(doc);
    let states = crate::layout::keyframe::compute_frame_states(&keyframes);
    let frames: Vec<(String, LayoutResult, HashSet<String>)> = if states.is_empty() {
        vec![(String::new(), result.clone(), HashSet::new())]
    } else {
        states
            .iter()
            .map(|st: &FrameState| {
                let l = if st.needs_resolve() {
                    crate::layout::keyframe::resolve_frame_for_static(result, st, doc, config)
                        .unwrap_or_else(|| result.clone())
                } else {
                    result.clone()
                };
                (st.name.clone(), l, st.hidden_elements.clone())
            })
            .collect()
    };
    // element -> (message, frames)
    let mut found: BTreeMap<String, (String, Vec<String>)> = BTreeMap::new();
    for (name, layout, hidden) in &frames {
        let Some(c) = layout.get_element_by_name(&canvas).map(|e| e.bounds) else { continue };
        let mut ls = Vec::new();
        leaves(&layout.root_elements, hidden, &mut ls);
        for e in ls {
            let Some(id) = e.id.as_ref().map(|i| i.0.clone()) else { continue };
            if id == canvas || e.is_through_path() && e.bounds.width == 0.0 {
                continue;
            }
            if let Some(msg) = overflow(&e.bounds, &c) {
                let entry = found.entry(id).or_insert((msg, Vec::new()));
                if !name.is_empty() {
                    entry.1.push(name.clone());
                }
            }
        }
    }
    let all_frames = frames.len();
    found
        .into_iter()
        .map(|(id, (msg, fr))| LintWarning {
            category: LintCategory::CanvasOverflow,
            message: format!(
                "'{}' leaves the canvas '{}': {}. Shorten or wrap it (`max_width:` on a label), or give it more room",
                display(&id),
                display(&canvas),
                msg
            ),
            frames: if fr.len() == all_frames { vec![] } else { fr },
            pair: None,
        })
        .collect()
}
