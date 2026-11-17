//! Keyframes inside an `each`: one per item, named from it (`keyframe
//! "step{i+1}"`, `keyframe "{s.id}"`), acting on what it names (`show
//! box[s.id]`, `hide tag` for this item's `tag`, `+ m.dx` in a constraint).

use std::collections::HashMap;

use crate::parser::ast::{
    ConstrainDecl, DrawTo, FlySubject, Identifier, KeyframeDecl, KeyframeOp, MotionArg, MotionEvent, MotionNode,
    MotionStmt, MotionValue, MotionVerb, PinTo, Selector, Spanned, Statement, StyleModifier,
};

use super::{
    fill_index, interpolate, name_at, rename_expr, rn_dotted, rn_id, rn_mods, subst_constrain, subst_mods, Problem, Vars,
};
use crate::template::names::head_of;

type Rename<'a> = dyn FnMut(&mut String) + 'a;

fn sel(s: &mut Selector, f: &mut Rename) {
    match s {
        Selector::Name(n) | Selector::Children(n) | Selector::Lines(n, _, _) => f(n),
        Selector::AllExcept(v) => v.iter_mut().for_each(|n| f(n)),
        Selector::Many(v) => v.iter_mut().for_each(|s| sel(s, f)),
        Selector::Class(_) => {}
    }
}

fn sels(v: &mut [Spanned<Selector>], f: &mut Rename) {
    v.iter_mut().for_each(|s| sel(&mut s.node, f));
}

fn id(i: &mut Spanned<Identifier>, f: &mut Rename) {
    f(&mut i.node.0);
}

fn draw_to(d: &mut DrawTo, f: &mut Rename) {
    if let DrawTo::Element(n) = d {
        f(n)
    }
}

fn opt_names(v: &mut MotionValue, f: &mut Rename) {
    match v {
        MotionValue::Name(n) => f(n),
        MotionValue::List(l) | MotionValue::Call(_, l) => l.iter_mut().for_each(|x| opt_names(&mut x.node, f)),
        _ => {}
    }
}

fn stmt_names(s: &mut MotionStmt, f: &mut Rename) {
    match &mut s.verb {
        MotionVerb::Show(v) | MotionVerb::Hide(v) | MotionVerb::Draw(v) | MotionVerb::Undraw(v) => sels(v, f),
        MotionVerb::Effect { targets, .. } | MotionVerb::Loop { targets, .. } => sels(targets, f),
        MotionVerb::Transform { target, .. } | MotionVerb::Count(target) | MotionVerb::SetState { target, .. } => {
            sel(&mut target.node, f)
        }
        MotionVerb::Disable(v) | MotionVerb::Enable(v) => v.iter_mut().for_each(|i| id(i, f)),
        MotionVerb::Fly { subject, from, to } => {
            match subject {
                FlySubject::Ghost(s) | FlySubject::Proxy(s) => sel(&mut s.node, f),
            }
            if let Some(x) = from {
                f(&mut x.node);
            }
            sels(to, f);
        }
        MotionVerb::Move { target, to, to_list, along } => {
            sel(&mut target.node, f);
            to.iter_mut().chain(to_list.iter_mut()).chain(along.iter_mut()).for_each(|x| f(&mut x.node));
        }
        MotionVerb::Swap { from, to } => {
            sel(&mut from.node, f);
            sel(&mut to.node, f);
        }
        MotionVerb::Camera(Some(x)) => f(&mut x.node),
        MotionVerb::Call { args, .. } => args.iter_mut().for_each(|a| {
            if let MotionArg::Name(n) = &mut a.node {
                f(n)
            }
        }),
        MotionVerb::Insert { code, .. } => f(&mut code.node),
        MotionVerb::Constrain(_) | MotionVerb::Camera(None) | MotionVerb::UseLayout(_) => {}
    }
    // `[to: x]`, `[from: r1, r2]`: options that name elements.
    for o in &mut s.opts {
        opt_names(&mut o.node.value.node, f);
    }
}

fn node_names(n: &mut MotionNode, f: &mut Rename) {
    match n {
        MotionNode::Stmt(s) => stmt_names(s, f),
        MotionNode::Then(b) | MotionNode::After(_, b) | MotionNode::At(_, b) | MotionNode::Beat(_, b) => {
            b.iter_mut().for_each(|c| node_names(&mut c.node, f))
        }
        MotionNode::When(ev, _, b) => {
            match ev {
                MotionEvent::Reaches { line, target } => {
                    f(&mut line.node);
                    f(&mut target.node);
                }
                MotionEvent::Arrives(t) | MotionEvent::Shown(t) | MotionEvent::Hidden(t) => f(&mut t.node),
                MotionEvent::Effected { target, .. } => f(&mut target.node),
                MotionEvent::BeatEnd(_) => {}
            }
            b.iter_mut().for_each(|c| node_names(&mut c.node, f))
        }
    }
}

/// Every name a keyframe refers to (show/hide targets, move destinations,
/// `when` events, constraint names to disable), handed to `f` to rewrite.
/// The flat `operations` and the `motion` tree both carry them.
fn names(kf: &mut KeyframeDecl, f: &mut Rename) {
    for op in &mut kf.operations {
        match &mut op.node {
            KeyframeOp::Show(v) | KeyframeOp::Hide(v) | KeyframeOp::Disable(v) | KeyframeOp::Enable(v) => {
                v.iter_mut().for_each(|i| id(i, f))
            }
            KeyframeOp::Transform { target, .. } | KeyframeOp::Mark { target, .. } | KeyframeOp::Unmark(target) => {
                id(target, f)
            }
            KeyframeOp::Draw { target, to } => {
                id(target, f);
                draw_to(to, f);
            }
            KeyframeOp::Pin { target, to } => {
                id(target, f);
                match to {
                    PinTo::Element(n) => f(n),
                    PinTo::Along { path, at } => {
                        f(path);
                        draw_to(at, f);
                    }
                    PinTo::Home => {}
                }
            }
            KeyframeOp::Camera { focus: Some(x), .. } => id(x, f),
            KeyframeOp::Camera { focus: None, .. } | KeyframeOp::Constrain(_) => {}
        }
    }
    kf.motion.iter_mut().for_each(|n| node_names(&mut n.node, f));
}

/// A part of a keyframe that holds more than names.
enum Part<'a> {
    Constrain(&'a mut ConstrainDecl),
    Modifiers(&'a mut [Spanned<StyleModifier>]),
}

fn node_parts<'a>(n: &'a mut MotionNode, out: &mut Vec<Part<'a>>) {
    match n {
        MotionNode::Stmt(s) => match &mut s.verb {
            MotionVerb::Constrain(d) => out.push(Part::Constrain(d)),
            MotionVerb::Transform { modifiers, .. } => out.push(Part::Modifiers(modifiers)),
            _ => {}
        },
        MotionNode::Then(b) | MotionNode::After(_, b) | MotionNode::At(_, b) | MotionNode::Beat(_, b) | MotionNode::When(_, _, b) => {
            b.iter_mut().for_each(|x| node_parts(&mut x.node, out))
        }
    }
}

/// Every `constrain` and `transform [...]` a keyframe holds.
fn parts(kf: &mut KeyframeDecl) -> Vec<Part<'_>> {
    let mut out = Vec::new();
    for op in &mut kf.operations {
        match &mut op.node {
            KeyframeOp::Constrain(d) => out.push(Part::Constrain(d)),
            KeyframeOp::Transform { modifiers, .. } => out.push(Part::Modifiers(modifiers)),
            _ => {}
        }
    }
    kf.motion.iter_mut().for_each(|n| node_parts(&mut n.node, &mut out));
    out
}

/// A keyframe in an `each` body acting on what this item made (`hide tag`
/// -> `hide tag_2`).
pub(super) fn rename(kf: &mut KeyframeDecl, m: &HashMap<String, String>) {
    names(kf, &mut |n| *n = rn_dotted(n, m));
    for p in parts(kf) {
        match p {
            Part::Constrain(d) => {
                rename_expr(&mut d.expr, m);
                if let Some(n) = &mut d.name {
                    rn_id(n, m);
                }
            }
            Part::Modifiers(mods) => rn_mods(mods, m),
        }
    }
}

/// A keyframe in an `each` body, for one item: its name, title and note
/// filled in (`"step{i+1}"`, `"{s.id}"`), the names it acts on (`show
/// box[s.id]`, `move dot to m.at`), and numbers in its constraints (`+ m.dx`).
pub(super) fn subst(kf: &mut KeyframeDecl, vars: &Vars, problems: &mut Vec<Problem>) {
    kf.name.node = interpolate(&kf.name.node, vars, problems);
    for t in kf.title.iter_mut().chain(kf.note.iter_mut()) {
        *t = interpolate(t, vars, problems);
    }
    names(kf, &mut |n| {
        if n.contains("_{") {
            *n = fill_index(n, vars, problems);
        }
        if vars.contains_key(head_of(n).0) {
            if let Some((full, _)) = name_at(&n.clone(), vars, problems) {
                *n = full;
            }
        }
    });
    for p in parts(kf) {
        match p {
            Part::Constrain(d) => subst_constrain(d, vars, problems),
            Part::Modifiers(mods) => subst_mods(mods, vars, problems),
        }
    }
}

/// The keyframes `stmts` declare, in order, nested blocks included.
pub(super) fn declared(stmts: &[Spanned<Statement>], out: &mut Vec<String>) {
    for st in stmts {
        match &st.node {
            Statement::Keyframe(k) => out.push(k.name.node.clone()),
            Statement::Each(e) => declared(&e.body, out),
            Statement::If(b) => {
                declared(&b.then_body, out);
                declared(&b.else_body, out);
            }
            _ => {}
        }
    }
}
