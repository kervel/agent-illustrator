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
    appears_before_line(m, base, &mut out);
    appears_and_show(doc, &mut out);
    chained_after(doc, &mut out);
    swap_apart(doc, base, &mut out);
    let show = |id: &str| m.display.get(id).cloned().unwrap_or_else(|| id.to_string());
    let mut parent_of = HashMap::new();
    parents(&base.root_elements, None, &mut parent_of);

    // An outline (no fill) that pops or grows in scales its border across
    // what it encloses: its edges sweep through the contents like stray
    // lines. Outlines draw in (or fade).
    for f in &m.frames {
        for a in f.atoms.iter().filter(|a| a.kind == "show" && (a.verb.contains("enter: pop") || a.verb.contains("enter: grow"))) {
            let Some(e) = base.get_element_by_name(&a.target_id) else { continue };
            let outline = e.styles.fill.as_deref().is_some_and(|f| f == "none")
                && e.styles.stroke.as_deref().is_some_and(|s| s != "none");
            if !outline {
                continue;
            }
            let b = e.bounds;
            let encloses = base.elements.iter().any(|(id, o)| {
                id != &a.target_id
                    && o.bounds.width > 0.0
                    && o.bounds.x > b.x
                    && o.bounds.y > b.y
                    && o.bounds.right() < b.right()
                    && o.bounds.bottom() < b.bottom()
            });
            if encloses {
                out.push(warn(
                    LintCategory::Motion,
                    &f.name,
                    format!(
                        "{} is an outline around other elements: entering with pop/grow scales its border through them \
                         (it reads as stray lines). Use `show {} [enter: draw]` (or fade)",
                        show(&a.target_id),
                        show(&a.target_id)
                    ),
                ));
            }
        }
    }

    // `show project.*` while `project` itself is hidden: nothing appears.
    for (fi, f) in m.frames.iter().enumerate() {
        let Some(st) = states.get(fi) else { continue };
        let mut seen = std::collections::HashSet::new();
        for a in f.atoms.iter().filter(|a| a.kind == "show") {
            if st.hidden_elements.contains(&a.target_id) {
                continue;
            }
            let mut cur = parent_of.get(&a.target_id).cloned();
            while let Some(p) = cur {
                if st.hidden_elements.contains(&p) {
                    if seen.insert(p.clone()) {
                        out.push(warn(
                            LintCategory::Motion,
                            &f.name,
                            format!(
                                "show {} has no effect: {} (which holds it) is still hidden at the end of \"{}\". \
                                 Show {} too, or put `appears: later` on the parts instead of on {}",
                                show(&a.target_id), show(&p), f.name, show(&p), show(&p)
                            ),
                        ));
                    }
                    break;
                }
                cur = parent_of.get(&p).cloned();
            }
        }
    }

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

    // `show x` on something on screen from before the keyframe (and never
    // hidden in it first): it does nothing, and x was there all along. The
    // usual cause is a missing `appears: later`.
    let mut kids: HashMap<&str, Vec<&str>> = HashMap::new();
    for (c, p) in &parent_of {
        kids.entry(p.as_str()).or_default().push(c.as_str());
    }
    // (A show in the first keyframe is an entrance: what it shows starts hidden.)
    for (fi, f) in m.frames.iter().enumerate().skip(1) {
        let prev = &states[fi - 1];
        let mut reported: HashSet<&str> = HashSet::new();
        for (ai, a) in f.atoms.iter().enumerate() {
            if a.kind != "show" || !m.display.contains_key(&a.target_id) || reported.contains(a.target_id.as_str()) {
                continue;
            }
            // Everything under it (and it) visible before the keyframe.
            let mut stack = vec![a.target_id.as_str()];
            let mut all_visible = true;
            while let Some(x) = stack.pop() {
                if hidden_in(prev, x, &parent_of) {
                    all_visible = false;
                    break;
                }
                stack.extend(kids.get(x).into_iter().flatten().copied());
            }
            let hidden_first = f.atoms[..ai].iter().any(|b| b.kind == "hide" && b.target_id == a.target_id);
            if all_visible && !hidden_first {
                reported.insert(a.target_id.as_str());
                out.push(warn(
                    LintCategory::Motion,
                    &f.name,
                    format!(
                        "show {}: it is already on screen before this keyframe, so this does nothing (missing `appears: later` on it?)",
                        show(&a.target_id)
                    ),
                ));
            }
        }
    }

    for (fi, f) in m.frames.iter().enumerate() {
        let end = &states[fi];
        let before = if fi > 0 { Some(&states[fi - 1]) } else { None };
        // On screen for a while in this frame: shown (and perhaps hidden
        // again), or flown as a traveller.
        let on_screen_meanwhile: HashSet<&str> = f
            .atoms
            .iter()
            .filter(|a| a.kind == "show" || a.kind == "fly")
            .map(|a| a.target_id.as_str())
            .collect();
        let shown_meanwhile = |id: &str| {
            let mut cur = Some(id.to_string());
            while let Some(c) = cur {
                if on_screen_meanwhile.contains(c.as_str()) {
                    return true;
                }
                cur = parent_of.get(&c).cloned();
            }
            false
        };
        let hidden_throughout = |id: &str| {
            hidden_in(end, id, &parent_of) && before.is_none_or(|b| hidden_in(b, id, &parent_of)) && !shown_meanwhile(id)
        };

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

        // A `draw` that retracts a line: it was drawn further already (often
        // a line declared without `drawn: 0`, so it starts complete).
        for (ci, curve) in &f.curves {
            if m.channels[*ci].prop != crate::motion::compile::Prop::DashOffset {
                continue;
            }
            let first = |v: &crate::motion::compile::Val| match v {
                crate::motion::compile::Val::V(x) => x.first().copied(),
                _ => None,
            };
            let mut at = first(&m.settled[fi][*ci]);
            for t in &curve.tweens {
                let from = t.from.as_ref().and_then(first).or(at);
                let to = first(&t.to);
                at = to.or(at);
                let (Some(from), Some(to)) = (from, to) else { continue };
                let a = &f.atoms[t.atom];
                if a.kind == "draw" && to > from + 1e-3 {
                    out.push(warn(
                        LintCategory::Motion,
                        &f.name,
                        format!(
                            "draw {} shrinks it from {:.0}% to {:.0}% drawn: it was already drawn further. \
                             Declare the line `drawn: 0` if it should start empty, or use `undraw` to retract it on purpose",
                            show(&a.target_id),
                            100.0 * (1.0 - from),
                            (100.0 * (1.0 - to)).max(0.0)
                        ),
                    ));
                }
            }
        }

        let mut seen: HashSet<(String, String)> = HashSet::new();
        for a in &f.atoms {
            // Moving something nobody can see.
            let acts_on_visible = matches!(
                a.kind.as_str(),
                "accent" | "pulse" | "shake" | "flash" | "ping" | "highlight" | "nudge" | "loop" | "move" | "transform"
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

/// Where along a polyline (0..1) a point sits, if it is within `tol` of it.
/// Where on a line (0..1) it is declared to pass `id`: a connection
/// reaches its end element at 1 (and starts at its source), a `through:`
/// path passes its stations; a caption that merely sits on a line is not on
/// it. Other paths: whatever their drawing passes through the centre of.
fn declared_at(
    base: &LayoutResult,
    line: &str,
    id: &str,
    pl: &crate::motion::geom::Polyline,
    c: crate::layout::types::Point,
) -> Option<f64> {
    use crate::layout::types::ElementType;
    use crate::parser::ast::ShapeType;
    let related = |n: &str| n == id || n.starts_with(&format!("{}_", id)) || id.starts_with(&format!("{}_", n));
    if let Some(conn) = base.connections.iter().find(|x| x.name.as_ref().is_some_and(|n| n.0 == line)) {
        // A connection between two parts of what is shown is inside it,
        // not a line it waits for.
        let inside = |n: &str| n == id || n.starts_with(&format!("{}_", id));
        if inside(&conn.from_id.0) && inside(&conn.to_id.0) {
            return None;
        }
        return if related(&conn.to_id.0) {
            Some(1.0)
        } else {
            None
        };
    }
    if let Some(e) = base.get_element_by_name(line) {
        if let ElementType::Shape(ShapeType::Path(decl)) = &e.element_type {
            if let Some(spec) = crate::layout::through::through_spec(&decl.modifiers) {
                if !spec.names.iter().any(|n| n == id || n.starts_with(&format!("{}_", id))) {
                    return None;
                }
            }
        }
    }
    fraction_on(pl, c, 3.0)
}

fn fraction_on(pl: &crate::motion::geom::Polyline, p: crate::layout::types::Point, tol: f64) -> Option<f64> {
    let total = *pl.cum.last()?;
    if total <= 0.0 {
        return None;
    }
    let mut best: Option<(f64, f64)> = None;
    for i in 0..pl.pts.len().saturating_sub(1) {
        let (a, b) = (pl.pts[i], pl.pts[i + 1]);
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let l2 = dx * dx + dy * dy;
        let u = if l2 == 0.0 { 0.0 } else { (((p.x - a.x) * dx + (p.y - a.y) * dy) / l2).clamp(0.0, 1.0) };
        let (qx, qy) = (a.x + dx * u, a.y + dy * u);
        let d = ((p.x - qx).powi(2) + (p.y - qy).powi(2)).sqrt();
        if d <= tol && best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, (pl.cum[i] + u * l2.sqrt()) / total));
        }
    }
    best.map(|(_, f)| f)
}

/// Something that sits on a line appears before the line, being drawn,
/// has reached it (a station popping ahead of its track).
fn appears_before_line(m: &Motion, base: &LayoutResult, out: &mut Vec<LintWarning>) {
    use crate::motion::compile::{drawable_polyline, Prop, Val};
    let tokens = crate::motion::tokens::MotionTokens::default();
    let show = |id: &str| m.display.get(id).cloned().unwrap_or_else(|| id.to_string());
    let lines: Vec<(String, crate::motion::geom::Polyline, usize)> = m
        .aux
        .drawables
        .keys()
        .filter_map(|id| {
            let pl = drawable_polyline(base, id)?;
            let mask = format!(".aimask-{}{}", m.scope, id);
            let ch = m.channels.iter().position(|k| k.sel == mask && k.prop == Prop::DashOffset)?;
            Some((id.clone(), pl, ch))
        })
        .collect();
    for (fi, f) in m.frames.iter().enumerate() {
        for a in f.atoms.iter().filter(|a| a.kind == "show") {
            let Some(e) = base.get_element_by_name(&a.target_id) else { continue };
            let c = e.bounds.center();
            for (line, pl, ch) in &lines {
                let Some(at) = declared_at(base, line, &a.target_id, pl, c) else { continue };
                let drawn = |t: f64| match &m.sample(fi, t, &tokens)[*ch] {
                    Val::V(v) => 1.0 - v.first().copied().unwrap_or(0.0),
                    _ => 1.0,
                };
                // `show x [from: y]` flies in: it is at the station when it
                // lands, not when it sets off.
                let lands = if a.verb.contains("from:") { a.start + a.dur } else { a.start };
                if drawn(lands) + 0.01 >= at {
                    continue;
                }
                let arrives = (0..=((f.duration - lands) / 0.01).ceil().max(0.0) as usize)
                    .map(|k| lands + k as f64 * 0.01)
                    .find(|t| drawn(*t) + 0.005 >= at);
                out.push(warn(
                    LintCategory::Motion,
                    &f.name,
                    match arrives {
                        Some(t) => {
                            // A connection drawn in (`show l [enter: draw]`) has
                            // reached its end when its entrance is done.
                            let conn = base.connections.iter().any(|x| x.name.as_ref().is_some_and(|n| &n.0 == line));
                            let fix = if conn {
                                format!("when {} shown", show(line))
                            } else {
                                format!("when {} reaches {}", show(line), show(&a.target_id))
                            };
                            format!(
                                "{} appears at {:.2}s but {} only reaches it at {:.2}s: time it on the line \
                                 (`{} {{ ... }}`)",
                                show(&a.target_id), lands, show(line), t, fix
                            )
                        }
                        None => format!(
                            "{} appears at {:.2}s on {}, which is not drawn to it in this step",
                            show(&a.target_id), a.start, show(line)
                        ),
                    },
                ));
            }
        }
    }
}

/// `appears: X` on an element and a `show` of it in X: two places say when
/// it comes on. Keep one: `appears: later` with the show (when and how), or
/// `appears: X` alone (it enters at the start of X with the default entrance).
fn appears_and_show(doc: &Document, out: &mut Vec<LintWarning>) {
    use crate::parser::ast::{MotionNode, MotionVerb, Selector, Statement, StyleKey, StyleValue};
    fn decls(stmts: &[crate::parser::ast::Spanned<Statement>], out: &mut Vec<(String, String, std::ops::Range<usize>)>) {
        for st in stmts {
            let (name, mods, kids) = match &st.node {
                Statement::Shape(s) => (s.name.as_ref().map(|n| n.node.0.clone()), &s.modifiers[..], &[][..]),
                Statement::Group(g) => (g.name.as_ref().map(|n| n.node.0.clone()), &g.modifiers[..], &g.children[..]),
                Statement::Layout(l) => (l.name.as_ref().map(|n| n.node.0.clone()), &l.modifiers[..], &l.children[..]),
                _ => continue,
            };
            if let Some(name) = name {
                for m in mods {
                    if !matches!(&m.node.key.node, StyleKey::Custom(k) if k == "appears") {
                        continue;
                    }
                    let when = match &m.node.value.node {
                        StyleValue::String(s) | StyleValue::Keyword(s) => s.clone(),
                        StyleValue::Identifier(i) => i.0.clone(),
                        _ => continue,
                    };
                    if when != "later" {
                        out.push((name.clone(), when, m.node.value.span.clone()));
                    }
                }
            }
            decls(kids, out);
        }
    }
    fn explicit_show(nodes: &[crate::parser::ast::Spanned<MotionNode>], id: &str, inserted: &std::ops::Range<usize>) -> bool {
        nodes.iter().any(|n| match &n.node {
            MotionNode::Then(b) | MotionNode::After(_, b) | MotionNode::At(_, b) | MotionNode::When(_, _, b) | MotionNode::Beat(_, b) => {
                explicit_show(b, id, inserted)
            }
            MotionNode::Stmt(st) => {
                n.span != *inserted
                    && matches!(&st.verb, MotionVerb::Show(v) if st.targets.iter().any(|t| t == id)
                        || v.iter().any(|s| matches!(&s.node, Selector::Name(x) if x.replace('.', "_") == id)))
            }
        })
    }
    let mut found = Vec::new();
    decls(&doc.statements, &mut found);
    for (id, when, span) in found {
        for st in &doc.statements {
            let Statement::Keyframe(k) = &st.node else { continue };
            if k.name.node == when && explicit_show(&k.motion, &id, &span) {
                out.push(warn(
                    LintCategory::Motion,
                    &k.name.node,
                    format!(
                        "{} says when it comes on twice: `appears: {}` and a `show` in \"{}\". \
                         Write `appears: later` and keep the show (it says when and how), or drop the show",
                        id.replace('_', "."),
                        when,
                        when
                    ),
                ));
            }
        }
    }
}

/// `swap a -> b` turns a into b where a is: b somewhere else pops there.
/// (A morph travels; that is its point.)
fn swap_apart(doc: &Document, base: &LayoutResult, out: &mut Vec<LintWarning>) {
    use crate::parser::ast::{MotionNode, MotionOpt, MotionValue, MotionVerb, Spanned, Statement};
    fn walk(nodes: &[Spanned<MotionNode>], f: &mut dyn FnMut(&crate::parser::ast::MotionStmt)) {
        for n in nodes {
            match &n.node {
                MotionNode::Stmt(s) => f(s),
                MotionNode::Then(b) | MotionNode::After(_, b) | MotionNode::At(_, b) | MotionNode::When(_, _, b) | MotionNode::Beat(_, b) => walk(b, f),
            }
        }
    }
    let via = |opts: &[Spanned<MotionOpt>]| {
        opts.iter().find(|o| o.node.key.node == "via").and_then(|o| match &o.node.value.node {
            MotionValue::Name(n) => Some(n.clone()),
            _ => None,
        })
    };
    for st in &doc.statements {
        let Statement::Keyframe(k) = &st.node else { continue };
        walk(&k.motion, &mut |s| {
            if !matches!(s.verb, MotionVerb::Swap { .. }) || via(&s.opts).as_deref() == Some("morph") {
                return;
            }
            // `flip: pg`: that part turns, the rest crossfades; the part must
            // be in place.
            let member = s.opts.iter().find(|o| o.node.key.node == "flip").and_then(|o| match &o.node.value.node {
                MotionValue::Name(n) => Some(n.replace('.', "_")),
                _ => None,
            });
            for (a, b) in s.targets.iter().zip(&s.partners) {
                let (a, b) = match &member {
                    Some(m) => (format!("{}_{}", a, m), format!("{}_{}", b, m)),
                    None => (a.clone(), b.clone()),
                };
                let (a, b) = (&a, &b);
                let (Some(ea), Some(eb)) = (base.get_element_by_name(a), base.get_element_by_name(b)) else { continue };
                let (ca, cb) = (ea.bounds.center(), eb.bounds.center());
                if (ca.x - cb.x).abs() > 4.0 || (ca.y - cb.y).abs() > 4.0 {
                    out.push(warn(
                        LintCategory::Motion,
                        &k.name.node,
                        format!(
                            "swap {} -> {}: {} is {:.0}px away from {}, so it turns into something elsewhere. \
                             Place it there (`constrain {}.center = {}.center`), or change one element with \
                             `transform {} [label: ..., swap: fade]`",
                            a.replace('_', "."),
                            b.replace('_', "."),
                            b.replace('_', "."),
                            ((ca.x - cb.x).powi(2) + (ca.y - cb.y).powi(2)).sqrt(),
                            a.replace('_', "."),
                            b.replace('_', "."),
                            a.replace('_', "."),
                            a.replace('_', "."),
                        ),
                    ));
                }
            }
        });
    }
}

/// `after 0.2 { a }  after 0.3 { b }  after 0.4 { c }`: each offset counts
/// from the one before, so c's time is a sum nobody wrote down, and moving a
/// comes with moving everything after it. Say what it waits for instead.
fn chained_after(doc: &Document, out: &mut Vec<LintWarning>) {
    use crate::parser::ast::{MotionNode, Statement};
    fn walk(nodes: &[crate::parser::ast::Spanned<MotionNode>], frame: &str, out: &mut Vec<LintWarning>) {
        let offsets: Vec<f64> = nodes
            .iter()
            .filter_map(|n| match &n.node {
                MotionNode::After(t, _) if *t > 0.0 => Some(*t),
                _ => None,
            })
            .collect();
        if offsets.len() >= 2 {
            let total: f64 = offsets.iter().sum();
            out.push(warn(
                LintCategory::Motion,
                frame,
                format!(
                    "{} `after` offsets in a row ({} = {:.2}s): each counts from the one before. \
                     Time by what they wait for (`when <line> reaches <el>`, `when <el> shown`, \
                     `beat name {{ ... }}` + `after name`) or from the frame start (`at <t>`)",
                    offsets.len(),
                    offsets.iter().map(|t| format!("{}", t)).collect::<Vec<_>>().join(" + "),
                    total
                ),
            ));
        }
        for n in nodes {
            match &n.node {
                // A macro's body is inlined as `after 0`: its own business.
                MotionNode::After(t, _) if *t == 0.0 => {}
                MotionNode::Then(b) | MotionNode::After(_, b) | MotionNode::At(_, b) | MotionNode::When(_, _, b) | MotionNode::Beat(_, b) => {
                    walk(b, frame, out)
                }
                MotionNode::Stmt(_) => {}
            }
        }
    }
    for st in &doc.statements {
        if let Statement::Keyframe(k) = &st.node {
            walk(&k.motion, &k.name.node, out);
        }
    }
}

/// Whether a keyframe is written with motion (beats, motion-only verbs,
/// timing) rather than as a plain show/hide/transform state change.
fn uses_motion(nodes: &[crate::parser::ast::Spanned<crate::parser::ast::MotionNode>]) -> bool {
    use crate::parser::ast::{MotionNode, MotionVerb, StyleKey};
    nodes.iter().any(|n| match &n.node {
        MotionNode::Then(_) | MotionNode::After(..) | MotionNode::At(..) | MotionNode::When(..) | MotionNode::Beat(..) => true,
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
