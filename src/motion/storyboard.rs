//! `--states`: the storyboard as the file declares it, per click step.
//!
//! A matrix of what is visible (every element whose visibility ever
//! changes), then per step what changes: what appears and disappears, the
//! properties a transform sets, moves, how far lines are drawn, layouts and
//! component states in use. Everything motion verbs, macros and `appears:`
//! do shows here, so the text can be checked against the STORYBOARD comment
//! without rendering.

use crate::layout::keyframe::FrameState;
use crate::layout::types::{ElementLayout, LayoutResult};
use crate::parser::ast::{Document, DrawTo, PinTo, StyleValue};
use std::collections::{BTreeMap, HashMap, HashSet};

fn parents(elems: &[ElementLayout], parent: Option<&str>, out: &mut HashMap<String, Option<String>>, order: &mut Vec<String>) {
    for e in elems {
        let id = e.id.as_ref().map(|i| i.0.clone());
        if let Some(id) = &id {
            out.insert(id.clone(), parent.map(str::to_string));
            order.push(id.clone());
        }
        parents(&e.children, id.as_deref().or(parent), out, order);
    }
}

fn value_text(v: &StyleValue) -> String {
    match v {
        StyleValue::String(s) => format!("{:?}", s),
        StyleValue::Number { value, unit } => format!("{}{}", value, unit.clone().unwrap_or_default()),
        other => crate::layout::types::ResolvedStyles::color_to_css(other)
            .map(|c| c.trim_start_matches("var(--").trim_end_matches(')').to_string())
            .unwrap_or_else(|| format!("{:?}", other)),
    }
}

fn draw_text(d: &DrawTo, show: &dyn Fn(&str) -> String) -> String {
    match d {
        DrawTo::Fraction(f) => format!("{:.0}%", f * 100.0),
        DrawTo::Element(e) => format!("to {}", show(e)),
        DrawTo::Vertex(v) => format!("to vertex {}", v),
    }
}

pub fn states_text(doc: &Document, states: &[FrameState], result: &LayoutResult, display: &HashMap<String, String>) -> String {
    let show = |id: &str| display.get(id).cloned().unwrap_or_else(|| id.to_string());
    let kfs = crate::layout::keyframe::extract_keyframes(doc);
    // Click steps: a frame and the [auto] frames after it.
    let mut steps: Vec<Vec<usize>> = Vec::new();
    for (i, k) in kfs.iter().enumerate() {
        match steps.last_mut() {
            Some(s) if i > 0 && k.auto.is_some() => s.push(i),
            _ => steps.push(vec![i]),
        }
    }
    let mut parent = HashMap::new();
    let mut order = Vec::new();
    parents(&result.root_elements, None, &mut parent, &mut order);
    let hidden_in = |set: &HashSet<String>, id: &str| -> bool {
        let mut cur = Some(id.to_string());
        while let Some(e) = cur {
            if set.contains(&e) {
                return true;
            }
            cur = parent.get(&e).cloned().flatten();
        }
        false
    };
    let visible = |st: &FrameState, id: &str| !hidden_in(&st.hidden_elements, id) && !st.hidden_connections.contains(id);
    // Before step 0: frame 0's hides, applied before it plays.
    let pre: HashSet<String> = kfs
        .first()
        .map(|k| {
            k.operations
                .iter()
                // Its hides apply up front, and what it shows enters: both
                // are hidden before it plays.
                .filter_map(|op| match &op.node {
                    crate::parser::ast::KeyframeOp::Hide(ids) | crate::parser::ast::KeyframeOp::Show(ids) => {
                        Some(ids.iter().map(|i| i.node.0.clone()).collect::<Vec<_>>())
                    }
                    _ => None,
                })
                .flatten()
                .collect()
        })
        .unwrap_or_default();
    // Rows: whatever some frame hides (explicitly), in document order.
    let hidden_ever: HashSet<&String> =
        states.iter().flat_map(|s| s.hidden_elements.iter().chain(s.hidden_connections.iter())).chain(pre.iter()).collect();
    // Named connections (a gate, a flow) sit after the elements.
    for st in states {
        for c in &st.hidden_connections {
            if !order.contains(c) {
                order.push(c.clone());
            }
        }
    }
    let rows: Vec<&String> = order.iter().filter(|id| hidden_ever.contains(id)).collect();
    let ends: Vec<&FrameState> = steps.iter().map(|s| &states[*s.last().unwrap()]).collect();

    let mut out = String::new();
    out.push_str("steps:\n");
    for (k, s) in steps.iter().enumerate() {
        let names: Vec<&str> = s.iter().map(|i| kfs[*i].name.node.as_str()).collect();
        let title = s.iter().filter_map(|i| kfs[*i].title.as_deref()).last();
        out.push_str(&format!(
            "  {:>2}  {}{}\n",
            k,
            names.join(" + "),
            title.map(|t| format!("   title: {:?}", t)).unwrap_or_default()
        ));
    }
    out.push_str("\nvisible at the end of each step (● shown, · hidden):\n");
    let w = rows.iter().map(|r| show(r).chars().count()).max().unwrap_or(4).max(4);
    out.push_str(&format!("  {:w$} ", "", w = w));
    for k in 0..steps.len() {
        out.push_str(&format!("{:>3}", k));
    }
    out.push('\n');
    for r in &rows {
        out.push_str(&format!("  {:w$} ", show(r), w = w));
        for st in &ends {
            out.push_str(if visible(st, r) { "  ●" } else { "  ·" });
        }
        out.push('\n');
    }

    out.push_str("\nchanges per step:\n");
    let empty = FrameState::initial();
    for (k, st) in ends.iter().enumerate() {
        let prev: &FrameState = if k == 0 { &empty } else { ends[k - 1] };
        out.push_str(&format!("  step {} ({}):\n", k, steps[k].iter().map(|i| kfs[*i].name.node.as_str()).collect::<Vec<_>>().join(" + ")));
        let (mut on, mut off) = (Vec::new(), Vec::new());
        for r in &rows {
            let before = if k == 0 { !hidden_in(&pre, r) } else { visible(prev, r) };
            let after = visible(st, r);
            if after && !before {
                on.push(show(r));
            }
            if !after && before {
                off.push(show(r));
            }
        }
        if !on.is_empty() {
            out.push_str(&format!("    + {}\n", on.join(", ")));
        }
        if !off.is_empty() {
            out.push_str(&format!("    - {}\n", off.join(", ")));
        }
        // Transforms that changed.
        let mut ids: Vec<&String> = st.transforms.keys().chain(prev.transforms.keys()).collect();
        ids.sort();
        ids.dedup();
        for id in ids {
            let now: BTreeMap<String, String> = st
                .transforms
                .get(id)
                .into_iter()
                .flatten()
                .map(|m| (m.node.key.node.source_name(), value_text(&m.node.value.node)))
                .collect();
            let was: BTreeMap<String, String> = prev
                .transforms
                .get(id)
                .map(|v| v.iter().map(|m| (m.node.key.node.source_name(), value_text(&m.node.value.node))).collect())
                .unwrap_or_default();
            let skip = |k2: &str| k2 == "rotation" || k2 == "dx" || k2 == "dy";
            let mut changed: Vec<String> = now
                .iter()
                .filter(|(k2, v)| was.get(*k2) != Some(v) && !skip(k2))
                .map(|(k2, v)| format!("{}: {}", k2, v))
                .collect();
            // Back to what the file declares.
            changed.extend(was.keys().filter(|k2| !now.contains_key(*k2) && !skip(k2)).map(|k2| format!("{}: initial", k2)));
            if !changed.is_empty() {
                out.push_str(&format!("    ~ {} [{}]\n", show(id), changed.join(", ")));
            }
        }
        for (id, to) in &st.pins {
            if prev.pins.get(id) != Some(to) {
                let where_ = match to {
                    PinTo::Home => "home".to_string(),
                    PinTo::Element(e) => show(e),
                    PinTo::Along { path, .. } => format!("along {}", show(path)),
                };
                out.push_str(&format!("    > {} moved to {}\n", show(id), where_));
            }
        }
        for (id, d) in &st.drawn {
            if prev.drawn.get(id) != Some(d) {
                out.push_str(&format!("    / {} drawn {}\n", show(id), draw_text(d, &show)));
            }
        }
        // Named layouts (`use layout X`) show as the layout, not as the
        // constraints they switch.
        let is_layout = |n: &str| n.starts_with("layout_");
        let layout_of = |n: &str| n.trim_start_matches("layout_").rsplit_once('_').map(|(l, _)| l.to_string());
        let mut dis: Vec<&String> = st.disabled_constraints.difference(&prev.disabled_constraints).filter(|n| !is_layout(n)).collect();
        dis.sort();
        let mut en: Vec<&String> = prev.disabled_constraints.difference(&st.disabled_constraints).filter(|n| !is_layout(n)).collect();
        en.sort();
        let active = |f: &FrameState| -> Vec<String> {
            let mut v: Vec<String> = f
                .added_constraints
                .iter()
                .filter_map(|c| c.name.as_ref().map(|n| n.node.0.clone()))
                .filter(|n| is_layout(n) && !f.disabled_constraints.contains(n))
                .filter_map(|n| layout_of(&n))
                .collect();
            v.sort();
            v.dedup();
            v
        };
        let (now_l, was_l) = (active(st), active(prev));
        if now_l != was_l {
            out.push_str(&format!("    layout: {}\n", if now_l.is_empty() { "default".to_string() } else { now_l.join(", ") }));
        }
        if !dis.is_empty() {
            out.push_str(&format!("    constraints off: {}\n", dis.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")));
        }
        if !en.is_empty() {
            out.push_str(&format!("    constraints on: {}\n", en.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")));
        }
    }
    out
}
