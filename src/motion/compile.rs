//! The motion compiler: keyframe statements -> explicit per-channel tracks.
//!
//! A *channel* is one CSS property on one hook in the rendered SVG (the
//! opacity of `.kf-box`, the `d` of `.conn-feed-base`, ...). For every frame
//! the compiler knows each channel's settled value, and the segments that
//! carry it from the previous frame's settled value to this one.
//!
//! Changes are attributed by replay: a keyframe's statements are applied to
//! the state one at a time (in source order), the layout is re-solved when a
//! statement changes geometry, and every channel that changed takes the
//! timing of the statement that changed it. A dependent that follows a moved
//! element therefore moves *with* it, in the same beat.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::layout::keyframe::FrameState;
use crate::layout::types::{ElementLayout, ElementType, LayoutResult, Point};
use crate::layout::LayoutConfig;
use crate::parser::ast::{MotionEvent, 
    Document, DrawTo, FlySubject, KeyframeDecl, KeyframeOp, MotionNode, MotionOpt, MotionStmt,
    MotionValue, MotionVerb, PinTo, ShapeType, Span, Spanned, Statement,
};

use super::ease::Ease;
use super::geom::{flatten_d, Polyline};
use super::tokens::MotionTokens;
use super::{opt, opt_name, opt_number};

// ---------------------------------------------------------------- values

/// A channel value: a numeric vector (opacity, translate, scale, ...) or a
/// string (colours, path data, text).
#[derive(Debug, Clone, PartialEq)]
pub enum Val {
    V(Vec<f64>),
    S(String),
}

impl Val {
    pub fn n(x: f64) -> Val {
        Val::V(vec![x])
    }
    pub fn xy(x: f64, y: f64) -> Val {
        Val::V(vec![x, y])
    }
    fn close(&self, other: &Val) -> bool {
        match (self, other) {
            (Val::V(a), Val::V(b)) => {
                a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-3)
            }
            (Val::S(a), Val::S(b)) => a == b,
            _ => false,
        }
    }
}

/// CSS property a channel drives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Prop {
    Opacity,
    Translate,
    Scale,
    Rotate,
    Fill,
    Stroke,
    StrokeWidth,
    StrokeDasharray,
    Width,
    Height,
    D,
    DashOffset,
    ClipPath,
    Text,
}

impl Prop {
    pub fn css(&self) -> &'static str {
        match self {
            Prop::Opacity => "opacity",
            Prop::Translate => "translate",
            Prop::Scale => "scale",
            Prop::Rotate => "rotate",
            Prop::Fill => "fill",
            Prop::Stroke => "stroke",
            Prop::StrokeWidth => "stroke-width",
            Prop::StrokeDasharray => "stroke-dasharray",
            Prop::Width => "width",
            Prop::Height => "height",
            Prop::D => "d",
            Prop::DashOffset => "stroke-dashoffset",
            Prop::ClipPath => "clip-path",
            Prop::Text => "text",
        }
    }

    /// Render a value as a CSS value for this property.
    pub fn format(&self, v: &Val) -> String {
        let r = |x: f64| {
            let y = (x * 1000.0).round() / 1000.0;
            if y == 0.0 { "0".to_string() } else { format!("{}", y) }
        };
        match (self, v) {
            (Prop::Translate, Val::V(p)) => format!("{}px {}px", r(p[0]), r(p[1])),
            (Prop::Scale, Val::V(p)) => format!("{} {}", r(p[0]), r(p[1])),
            (Prop::Rotate, Val::V(p)) => format!("{}deg", r(p[0])),
            (Prop::StrokeWidth | Prop::Width | Prop::Height, Val::V(p)) => format!("{}px", r(p[0])),
            (Prop::ClipPath, Val::V(p)) => format!(
                "inset({}% {}% {}% {}%)",
                r(p[0]),
                r(p[1]),
                r(p[2]),
                r(p[3])
            ),
            (Prop::D, Val::S(s)) => format!("path(\"{}\")", s),
            (_, Val::V(p)) => r(p[0]),
            (_, Val::S(s)) => s.clone(),
        }
    }
}

/// A channel: one property on the nodes a CSS selector picks out.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChKey {
    pub sel: String,
    pub prop: Prop,
}

// -------------------------------------------------------------- segments

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Abs,
    Add,
    Mul,
    /// Absolute, but jumps from key to key (text: a counter's digits).
    Step,
}

/// A persistent change: from the current value (or `from`) to `to`.
#[derive(Debug, Clone)]
pub struct Tween {
    pub start: f64,
    pub dur: f64,
    pub ease: Ease,
    pub from: Option<Val>,
    pub to: Val,
    pub atom: usize,
}

/// A transient shape laid over the base curve, gone when it ends.
#[derive(Debug, Clone)]
pub struct Overlay {
    pub start: f64,
    pub dur: f64,
    pub ease: Ease,
    pub keys: Vec<Val>,
    pub mode: Mode,
    /// Repeats until the frame is left (`loop`).
    pub looping: bool,
    pub atom: usize,
}

/// One statement applied to one target: the unit of timing.
#[derive(Debug, Clone)]
pub struct Atom {
    pub start: f64,
    pub dur: f64,
    pub ease: Ease,
    pub ease_name: String,
    pub verb: String,
    pub target: String,
    pub span: Span,
    /// The bare verb (`show`, `fly`, `pulse`, ...), for lints.
    pub kind: String,
    /// The element acted on, and (fly, swap) the one it goes to.
    pub target_id: String,
    pub dest_id: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Curve {
    pub tweens: Vec<Tween>,
    pub overlays: Vec<Overlay>,
}

#[derive(Debug, Clone)]
pub struct FrameMotion {
    pub name: String,
    pub duration: f64,
    pub auto: Option<f64>,
    pub title: Option<String>,
    pub note: Option<String>,
    pub atoms: Vec<Atom>,
    pub curves: BTreeMap<usize, Curve>,
    /// Each `when ...` / beat end as resolved: (time, what, index of the
    /// first atom of its block), for --timeline.
    pub events: Vec<(f64, String, usize)>,
}

/// Auxiliary nodes the renderer must add for motion to have something to move.
#[derive(Debug, Clone, Default)]
pub struct Aux {
    /// Element -> its parent element (for "is it visible?" at export).
    pub parents: HashMap<String, Option<String>>,
    /// Element ids that get a wrapper hook (all named elements).
    pub origins: HashMap<String, (f64, f64)>,
    /// Drawables: id -> is a connection (true) or path shape (false).
    pub drawables: BTreeMap<String, bool>,
    /// Ghost copies: (hook class suffix, source element id).
    /// (suffix, source, parts of the source hidden when it flies).
    pub ghosts: Vec<(String, String, Vec<String>)>,
    /// Per element, the transient overlay nodes it needs: flash, ping, highlight(colour).
    pub overlays: BTreeMap<String, BTreeSet<String>>,
    /// Text elements that get a counter ticker node.
    pub tickers: BTreeSet<String>,
    /// Whether a camera group is needed.
    pub camera: bool,
    /// Drawn connections whose arrowhead is its own node.
    pub heads: BTreeSet<String>,
    /// Text variants per element (as the renderer numbers them).
    pub text_variants: HashMap<String, Vec<String>>,
    /// Connections with crossfade variants (their base path gets a wrapper).
    pub conn_variants: std::collections::HashSet<(String, String)>,
}

/// The compiled motion of a document.
#[derive(Debug, Clone)]
pub struct Motion {
    pub scope: String,
    pub channels: Vec<ChKey>,
    /// settled[0] is the state before frame 0; settled[i+1] after frame i.
    pub settled: Vec<Vec<Val>>,
    pub frames: Vec<FrameMotion>,
    pub aux: Aux,
    /// Viewbox centre, for the camera.
    pub view_center: (f64, f64),
    /// How to write each id (dotted component paths).
    pub display: HashMap<String, String>,
}

// ------------------------------------------------------------ evaluation

fn lerp_val(a: &Val, b: &Val, p: f64, tokens: &MotionTokens) -> Val {
    // Overshooting eases leave [0, 1] for numbers only; strings snap.
    if !matches!((a, b), (Val::V(_), Val::V(_))) {
        if p <= 0.0 {
            return a.clone();
        }
        if p >= 1.0 {
            return b.clone();
        }
    }
    match (a, b) {
        (Val::V(x), Val::V(y)) if x.len() == y.len() => {
            Val::V(x.iter().zip(y).map(|(u, v)| u + (v - u) * p).collect())
        }
        (Val::S(x), Val::S(y)) => {
            if x == y {
                return a.clone();
            }
            if let (Some(c1), Some(c2)) = (tokens.rgb(x), tokens.rgb(y)) {
                let l = |u: f64, v: f64| (u + (v - u) * p).round().clamp(0.0, 255.0);
                return Val::S(format!(
                    "rgb({}, {}, {})",
                    l(c1.0, c2.0),
                    l(c1.1, c2.1),
                    l(c1.2, c2.2)
                ));
            }
            if let Some(s) = lerp_numbers_in(x, y, p) {
                return Val::S(s);
            }
            if p < 0.5 { a.clone() } else { b.clone() }
        }
        _ => {
            if p < 0.5 {
                a.clone()
            } else {
                b.clone()
            }
        }
    }
}

/// Interpolate two strings that differ only in their numbers (path data).
fn lerp_numbers_in(a: &str, b: &str, p: f64) -> Option<String> {
    fn split(s: &str) -> (Vec<String>, Vec<f64>) {
        let mut text = vec![String::new()];
        let mut nums = Vec::new();
        let mut cur = String::new();
        let flush = |cur: &mut String, text: &mut Vec<String>, nums: &mut Vec<f64>| {
            if !cur.is_empty() {
                if let Ok(v) = cur.parse() {
                    nums.push(v);
                    text.push(String::new());
                } else {
                    text.last_mut().unwrap().push_str(cur);
                }
                cur.clear();
            }
        };
        for c in s.chars() {
            if c.is_ascii_digit() || c == '.' || (c == '-' && cur.is_empty()) {
                cur.push(c);
            } else {
                flush(&mut cur, &mut text, &mut nums);
                text.last_mut().unwrap().push(c);
            }
        }
        flush(&mut cur, &mut text, &mut nums);
        (text, nums)
    }
    let (ta, na) = split(a);
    let (tb, nb) = split(b);
    if ta != tb || na.len() != nb.len() {
        return None;
    }
    let mut out = String::new();
    for (i, t) in ta.iter().enumerate() {
        out.push_str(t);
        if i < na.len() {
            let v = na[i] + (nb[i] - na[i]) * p;
            out.push_str(&format!("{}", (v * 100.0).round() / 100.0));
        }
    }
    Some(out)
}

fn combine(base: &Val, key: &Val, mode: Mode) -> Val {
    match (mode, base, key) {
        (Mode::Abs | Mode::Step, _, k) => k.clone(),
        (Mode::Add, Val::V(b), Val::V(k)) if b.len() == k.len() => {
            Val::V(b.iter().zip(k).map(|(x, y)| x + y).collect())
        }
        (Mode::Mul, Val::V(b), Val::V(k)) if b.len() == k.len() => {
            Val::V(b.iter().zip(k).map(|(x, y)| x * y).collect())
        }
        (_, b, _) => b.clone(),
    }
}

impl Curve {
    /// The persistent (tweened) value at `t`, starting from `init`.
    fn base(&self, init: &Val, t: f64, upto: usize, tokens: &MotionTokens) -> Val {
        // The last tween (in start order, before `upto`) that has started wins.
        let mut idx: Option<usize> = None;
        for (i, tw) in self.tweens.iter().enumerate().take(upto) {
            if tw.start <= t + 1e-9 {
                idx = Some(i);
            }
        }
        let Some(i) = idx else { return init.clone() };
        let tw = &self.tweens[i];
        let from = match &tw.from {
            Some(f) => f.clone(),
            None => self.base(init, tw.start, i, tokens),
        };
        let p = if tw.dur <= 0.0 { 1.0 } else { ((t - tw.start) / tw.dur).clamp(0.0, 1.0) };
        lerp_val(&from, &tw.to, tw.ease.eval(p), tokens)
    }

    /// Full value at `t` (base + overlays active at `t`).
    pub fn value(&self, init: &Val, t: f64, tokens: &MotionTokens) -> Val {
        let mut v = self.base(init, t, self.tweens.len(), tokens);
        for ov in &self.overlays {
            if let Some(k) = ov.key_at(t, tokens) {
                v = combine(&v, &k, ov.mode);
            }
        }
        v
    }
}

impl Overlay {
    fn key_at(&self, t: f64, tokens: &MotionTokens) -> Option<Val> {
        if t < self.start - 1e-9 {
            return None;
        }
        let mut local = t - self.start;
        if self.looping {
            if self.dur > 0.0 {
                local %= self.dur;
            }
        } else if local > self.dur + 1e-9 {
            return None;
        }
        let p = if self.dur <= 0.0 { 1.0 } else { (local / self.dur).clamp(0.0, 1.0) };
        if self.mode == Mode::Step {
            // Key j takes over at j/(n-1): the offsets the player uses.
            let n = self.keys.len();
            let i = ((self.ease.eval(p) * (n.max(2) - 1) as f64 + 1e-9).floor() as usize).min(n - 1);
            return Some(self.keys[i].clone());
        }
        Some(keys_at(&self.keys, self.ease.eval(p), tokens))
    }
}

/// Piecewise-linear interpolation through evenly spaced keys.
fn keys_at(keys: &[Val], p: f64, tokens: &MotionTokens) -> Val {
    if keys.len() == 1 {
        return keys[0].clone();
    }
    let n = keys.len() - 1;
    let x = p.clamp(0.0, 1.0) * n as f64;
    let i = (x.floor() as usize).min(n - 1);
    lerp_val(&keys[i], &keys[i + 1], x - i as f64, tokens)
}

impl Motion {
    /// Value of every channel at time `t` into frame `f`.
    pub fn sample(&self, f: usize, t: f64, tokens: &MotionTokens) -> Vec<Val> {
        let frame = &self.frames[f];
        let init = &self.settled[f];
        let t = t.max(0.0);
        (0..self.channels.len())
            .map(|c| match frame.curves.get(&c) {
                Some(curve) if t < frame.duration + 1e-9 || curve.overlays.iter().any(|o| o.looping) => {
                    if t >= frame.duration {
                        // Settled, plus any ambient loop.
                        let mut v = self.settled[f + 1][c].clone();
                        for ov in curve.overlays.iter().filter(|o| o.looping) {
                            if let Some(k) = ov.key_at(t, tokens) {
                                v = combine(&v, &k, ov.mode);
                            }
                        }
                        v
                    } else {
                        curve.value(&init[c], t, tokens)
                    }
                }
                _ if t >= frame.duration => self.settled[f + 1][c].clone(),
                _ => init[c].clone(),
            })
            .collect()
    }
}

// ---------------------------------------------------------------- inputs

pub struct CompileInput<'a> {
    pub doc: &'a Document,
    pub base: &'a LayoutResult,
    pub layout_config: &'a LayoutConfig,
    pub tokens: &'a MotionTokens,
    pub scope: &'a str,
    pub text_variants: HashMap<String, Vec<String>>,
    /// (connection id, frame name) pairs whose route reshapes in that frame
    /// and is drawn as a crossfaded variant path instead of a `d` morph.
    pub conn_variants: std::collections::HashSet<(String, String)>,
}

/// Diagram-wide defaults from `motion [enter: ..., exit: ...]`.
#[derive(Debug, Clone)]
struct Defaults {
    enter: String,
    exit: String,
}

#[derive(Debug, Clone)]
struct ElemInfo {
    parent: Option<String>,
    base: crate::layout::types::BoundingBox,
    is_rect: bool,
    base_text: Option<String>,
    /// Text, or a box that paints nothing of its own (a label, a table row,
    /// a code line): accented with an underline rather than an outline.
    text_like: bool,
}

fn element_text(elem: &ElementLayout) -> Option<String> {
    match &elem.element_type {
        ElementType::Shape(ShapeType::Text { content }) => Some(content.clone()),
        _ => elem.label.as_ref().map(|l| l.source.clone()),
    }
}

fn walk_elements<'a>(
    elems: &'a [ElementLayout],
    parent: Option<&str>,
    f: &mut dyn FnMut(&'a ElementLayout, Option<&str>),
) {
    for e in elems {
        f(e, parent);
        let me = e.id.as_ref().map(|i| i.0.as_str()).or(parent);
        walk_elements(&e.children, me, f);
    }
}

/// The measured polyline of a drawable in `layout`, as rendered.
pub fn drawable_polyline(layout: &LayoutResult, id: &str) -> Option<Polyline> {
    for c in &layout.connections {
        if c.name.as_ref().is_some_and(|n| n.0 == id) {
            let marker = matches!(
                c.direction,
                crate::parser::ast::ConnectionDirection::Forward
                    | crate::parser::ast::ConnectionDirection::Bidirectional
            );
            let sw = c.styles.stroke_width.unwrap_or(2.0);
            let d = crate::renderer::svg::connection_path_d(&c.path, c.routing_mode, marker, sw);
            return Some(flatten_d(&d));
        }
    }
    let e = layout.get_element_by_name(id)?;
    if let ElementType::Shape(ShapeType::Path(decl)) = &e.element_type {
        let origin = Point::new(e.bounds.x, e.bounds.y);
        let d = crate::renderer::path::resolve_path_with_options(decl, origin, e.path_normalize).to_svg_d();
        return Some(flatten_d(&d));
    }
    None
}

/// The raw route points of a connection (vertices), for `vertex N`.
fn connection_vertices(layout: &LayoutResult, id: &str) -> Option<Vec<Point>> {
    layout
        .connections
        .iter()
        .find(|c| c.name.as_ref().is_some_and(|n| n.0 == id))
        .map(|c| c.path.clone())
}

/// Resolve a draw target to a fraction of the drawable's length.
pub fn resolve_draw_to(layout: &LayoutResult, id: &str, to: &DrawTo) -> Option<f64> {
    let pl = drawable_polyline(layout, id)?;
    Some(match to {
        DrawTo::Fraction(f) => *f,
        DrawTo::Vertex(n) => {
            if let Some(v) = connection_vertices(layout, id) {
                let p = *v.get(*n)?;
                pl.project(p).0
            } else {
                pl.vertex_fraction(*n)?
            }
        }
        DrawTo::Element(e) => {
            let el = layout.get_element_by_name(e)?;
            pl.project(el.bounds.center()).0
        }
    })
}

/// Where a pin sends an element's centre, given the base layout.
pub fn resolve_pin_point(layout: &LayoutResult, to: &PinTo) -> Option<Point> {
    match to {
        PinTo::Home => None,
        PinTo::Element(e) => layout.get_element_by_name(e).map(|el| el.bounds.center()),
        PinTo::Along { path, at } => {
            let pl = drawable_polyline(layout, path)?;
            let f = resolve_draw_to(layout, path, at)?;
            Some(pl.point_at(f))
        }
    }
}

// ------------------------------------------------------------- compiler

struct Snap {
    vals: BTreeMap<ChKey, Val>,
}

struct Compiler<'a> {
    input: &'a CompileInput<'a>,
    elems: BTreeMap<String, ElemInfo>,
    /// Drawables and whether their first draw starts them empty.
    drawables: BTreeMap<String, bool>,
    drawn_initial: BTreeMap<String, f64>,
    heads: BTreeSet<String>,
    defaults: Defaults,
    channels: Vec<ChKey>,
    ch_index: HashMap<ChKey, usize>,
    aux: Aux,
    scope: String,
    /// Options of the statement being compiled.
    current_opts: Vec<Spanned<MotionOpt>>,
    display: HashMap<String, String>,
    /// Frame 0's statements already applied in the pre-state (those at its
    /// start), by span: they set the scene and do not animate.
    pre_applied: HashSet<(usize, usize)>,
}

fn has_marker(layout: &LayoutResult, id: &str) -> bool {
    layout.connections.iter().any(|c| {
        c.name.as_ref().is_some_and(|n| n.0 == id)
            && matches!(
                c.direction,
                crate::parser::ast::ConnectionDirection::Forward
                    | crate::parser::ast::ConnectionDirection::Bidirectional
            )
    })
}

/// Arrowhead triangle for a drawn connection, matching the SVG marker.
pub fn head_d(layout: &LayoutResult, id: &str) -> Option<String> {
    let c = layout.connections.iter().find(|c| c.name.as_ref().is_some_and(|n| n.0 == id))?;
    if c.path.len() < 2 {
        return None;
    }
    let sw = c.styles.stroke_width.unwrap_or(2.0);
    let tip = c.path[c.path.len() - 1];
    let prev = c.path[c.path.len() - 2];
    let (dx, dy) = (tip.x - prev.x, tip.y - prev.y);
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-6 {
        return None;
    }
    let (ux, uy) = (dx / len, dy / len);
    let (nx, ny) = (-uy, ux);
    // Marker: 10x10 viewBox drawn at 4*sw, refX = 1 -> base 3.6*sw behind the tip.
    let back = 4.0 * sw;
    let half = 2.0 * sw;
    let bx = tip.x - ux * back;
    let by = tip.y - uy * back;
    let r = |x: f64| (x * 100.0).round() / 100.0;
    Some(format!(
        "M{} {} L{} {} L{} {} Z",
        r(bx + nx * half),
        r(by + ny * half),
        r(tip.x - ux * 0.4 * sw),
        r(tip.y - uy * 0.4 * sw),
        r(bx - nx * half),
        r(by - ny * half)
    ))
}

impl<'a> Compiler<'a> {
    fn ch(&mut self, sel: String, prop: Prop) -> usize {
        let key = ChKey { sel, prop };
        if let Some(i) = self.ch_index.get(&key) {
            return *i;
        }
        let i = self.channels.len();
        self.channels.push(key.clone());
        self.ch_index.insert(key, i);
        i
    }

    fn wrap(&self, id: &str) -> String {
        format!(".kf-{}{}", self.scope, id)
    }

    fn solve(&self, state: &FrameState) -> LayoutResult {
        if state.needs_resolve() {
            crate::layout::keyframe::resolve_frame_for_static(
                self.input.base,
                state,
                self.input.doc,
                self.input.layout_config,
            )
            .unwrap_or_else(|| self.input.base.clone())
        } else {
            self.input.base.clone()
        }
    }

    fn snapshot(&self, state: &FrameState, layout: &LayoutResult, frame: Option<&str>) -> Snap {
        let mut vals = BTreeMap::new();
        let s = &self.scope;
        let mut solved: HashMap<String, &ElementLayout> = HashMap::new();
        walk_elements(&layout.root_elements, None, &mut |e, _| {
            if let Some(id) = &e.id {
                solved.insert(id.0.clone(), e);
            }
        });
        let delta = |id: &str| -> (f64, f64) {
            match (solved.get(id), self.elems.get(id)) {
                (Some(e), Some(info)) => (e.bounds.x - info.base.x, e.bounds.y - info.base.y),
                _ => (0.0, 0.0),
            }
        };
        for (id, info) in &self.elems {
            let Some(e) = solved.get(id) else { continue };
            let base_el = self.input.base.get_element_by_name(id);
            let w = self.wrap(id);
            let hidden = state.hidden_elements.contains(id);
            // The wrapper carries the element's whole opacity (the renderer
            // moves a declared `opacity:` onto it), so one channel owns it.
            let _ = base_el;
            let opacity = if hidden { 0.0 } else { e.styles.opacity.unwrap_or(1.0) };
            vals.insert(ChKey { sel: w.clone(), prop: Prop::Opacity }, Val::n(opacity));
            let (dx, dy) = delta(id);
            let (px, py) = info.parent.as_deref().map(delta).unwrap_or((0.0, 0.0));
            vals.insert(ChKey { sel: w.clone(), prop: Prop::Translate }, Val::xy(dx - px, dy - py));
            let rot = e.styles.rotation.unwrap_or(0.0)
                - base_el.and_then(|b| b.styles.rotation).unwrap_or(0.0);
            vals.insert(ChKey { sel: w.clone(), prop: Prop::Rotate }, Val::n(rot));
            let shape = format!(".kfp-{}{}", s, id);
            if let Some(f) = &e.styles.fill {
                vals.insert(ChKey { sel: shape.clone(), prop: Prop::Fill }, Val::S(f.clone()));
            }
            if let Some(f) = &e.styles.stroke {
                vals.insert(ChKey { sel: shape.clone(), prop: Prop::Stroke }, Val::S(f.clone()));
            }
            if let Some(sw) = e.styles.stroke_width {
                vals.insert(ChKey { sel: shape.clone(), prop: Prop::StrokeWidth }, Val::n(sw));
            }
            if let Some(d) = &e.styles.stroke_dasharray {
                vals.insert(ChKey { sel: shape.clone(), prop: Prop::StrokeDasharray }, Val::S(d.clone()));
            }
            if info.is_rect {
                vals.insert(ChKey { sel: shape.clone(), prop: Prop::Width }, Val::n(e.bounds.width));
                vals.insert(ChKey { sel: shape.clone(), prop: Prop::Height }, Val::n(e.bounds.height));
            }
            if let Some(lf) = &e.styles.label_fill {
                vals.insert(ChKey { sel: format!("{} > text", w), prop: Prop::Fill }, Val::S(lf.clone()));
            }
            if let Some(variants) = self.input.text_variants.get(id) {
                let cur = element_text(e);
                let is_base = cur == info.base_text;
                vals.insert(
                    ChKey { sel: format!(".aitxt-{}{}-base", s, id), prop: Prop::Opacity },
                    Val::n(if is_base { 1.0 } else { 0.0 }),
                );
                for (n, t) in variants.iter().enumerate() {
                    let on = !is_base && cur.as_deref() == Some(t.as_str());
                    vals.insert(
                        ChKey { sel: format!(".aitxt-{}{}-v{}", s, id, n), prop: Prop::Opacity },
                        Val::n(if on { 1.0 } else { 0.0 }),
                    );
                }
            }
        }
        for (i, c) in layout.connections.iter().enumerate() {
            let cid = c.name.as_ref().map(|n| n.0.clone()).unwrap_or_else(|| format!("idx{}", i));
            let hidden = c.name.as_ref().is_some_and(|n| state.hidden_connections.contains(&n.0));
            vals.insert(
                ChKey { sel: format!(".conn-{}{}", s, cid), prop: Prop::Opacity },
                Val::n(if hidden { 0.0 } else { 1.0 }),
            );
            let marker = matches!(
                c.direction,
                crate::parser::ast::ConnectionDirection::Forward
                    | crate::parser::ast::ConnectionDirection::Bidirectional
            );
            let sw = c.styles.stroke_width.unwrap_or(2.0);
            let base_c = self.input.base.connections.get(i);
            let morphable = base_c.is_none_or(|b| {
                b.path.len() == c.path.len() && b.routing_mode == c.routing_mode
            });
            let has_variants = self.input.conn_variants.iter().any(|(id, _)| id == &cid);
            let variant_here = frame
                .filter(|f| self.input.conn_variants.contains(&(cid.clone(), f.to_string())))
                .map(str::to_string);
            let shown_path = if morphable || variant_here.is_none() { &c.path } else { &base_c.unwrap().path };
            let d = crate::renderer::svg::connection_path_d(shown_path, c.routing_mode, marker, sw);
            vals.insert(ChKey { sel: format!(".aid-{}{}", s, cid), prop: Prop::D }, Val::S(d));
            if has_variants {
                let use_variant = !morphable && variant_here.is_some();
                vals.insert(
                    ChKey { sel: format!(".aibase-{}{}", s, cid), prop: Prop::Opacity },
                    Val::n(if use_variant { 0.0 } else { 1.0 }),
                );
                for (vid, vf) in &self.input.conn_variants {
                    if vid != &cid {
                        continue;
                    }
                    let on = use_variant && variant_here.as_deref() == Some(vf.as_str());
                    vals.insert(
                        ChKey { sel: format!(".aivar-{}{}-f{}", s, cid, vf), prop: Prop::Opacity },
                        Val::n(if on { 1.0 } else { 0.0 }),
                    );
                }
            }
            if self.heads.contains(&cid) {
                if let Some(h) = head_d(layout, &cid) {
                    vals.insert(ChKey { sel: format!(".aihead-{}{}", s, cid), prop: Prop::D }, Val::S(h));
                }
            }
        }
        for (id, _) in &self.drawables {
            let frac = match state.drawn.get(id) {
                Some(to) => resolve_draw_to(layout, id, to).unwrap_or(1.0),
                None => *self.drawn_initial.get(id).unwrap_or(&1.0),
            };
            vals.insert(
                ChKey { sel: format!(".aimask-{}{}", s, id), prop: Prop::DashOffset },
                Val::n(mask_offset(frac)),
            );
            if self.heads.contains(id) {
                vals.insert(
                    ChKey { sel: format!(".aihead-{}{}", s, id), prop: Prop::Opacity },
                    Val::n(if frac >= 0.999 { 1.0 } else { 0.0 }),
                );
            }
        }
        if self.aux.camera {
            let (vx, vy) = self.view_center();
            let (t, z) = match &state.camera {
                Some((focus, zoom)) => {
                    let c = layout
                        .get_element_by_name(focus)
                        .map(|e| e.bounds.center())
                        .unwrap_or(Point::new(vx, vy));
                    ((vx - zoom * c.x, vy - zoom * c.y), *zoom)
                }
                None => ((0.0, 0.0), 1.0),
            };
            vals.insert(ChKey { sel: format!(".aicam-{}", s), prop: Prop::Translate }, Val::xy(t.0, t.1));
            vals.insert(ChKey { sel: format!(".aicam-{}", s), prop: Prop::Scale }, Val::xy(z, z));
        }
        Snap { vals }
    }

    fn view_center(&self) -> (f64, f64) {
        let b = &self.input.base.bounds;
        (b.x + b.width / 2.0, b.y + b.height / 2.0)
    }
}

/// Mask dash offset for a drawn fraction (dasharray "1 2", pathLength 1).
/// Slightly past 1 at zero, so a square cap never leaves a dot.
pub fn mask_offset(frac: f64) -> f64 {
    if frac <= 0.0005 {
        1.001
    } else {
        1.0 - frac.min(1.0)
    }
}

// ---------------------------------------------------------------- timing

struct Timing {
    delay: f64,
    dur: f64,
    ease: Ease,
    ease_name: String,
    stagger: f64,
    order: String,
}

fn verb_name(v: &MotionVerb) -> &'static str {
    match v {
        MotionVerb::Show(_) => "show",
        MotionVerb::Hide(_) => "hide",
        MotionVerb::Transform { .. } => "transform",
        MotionVerb::Constrain(_) => "constrain",
        MotionVerb::Disable(_) => "disable",
        MotionVerb::Enable(_) => "enable",
        MotionVerb::Draw(_) => "draw",
        MotionVerb::Undraw(_) => "undraw",
        MotionVerb::Fly { .. } => "fly",
        MotionVerb::Move { .. } => "move",
        MotionVerb::Effect { .. } => "effect",
        MotionVerb::Loop { .. } => "loop",
        MotionVerb::Count(_) => "count",
        MotionVerb::Swap { .. } => "swap",
        MotionVerb::Camera(_) => "camera",
        MotionVerb::Call { .. } => "call",
        MotionVerb::Insert { .. } => "insert",
        MotionVerb::UseLayout(_) => "use layout",
        MotionVerb::SetState { .. } => "set",
    }
}

pub struct TimingError(pub String, pub Span);

fn timing(
    s: &MotionStmt,
    tokens: &MotionTokens,
    defaults: &Defaults,
) -> Result<Timing, TimingError> {
    let err = |msg: String, v: &Spanned<MotionValue>| TimingError(msg, v.span.clone());
    let (def_dur, def_ease) = match &s.verb {
        MotionVerb::Hide(_) => ("fast", "settle"),
        MotionVerb::Fly { .. } | MotionVerb::Camera(_) => ("slow", "glide"),
        MotionVerb::Move { .. } => ("normal", "glide"),
        MotionVerb::Draw(_) | MotionVerb::Undraw(_) => ("normal", "glide"),
        MotionVerb::Count(_) => ("slow", "settle"),
        MotionVerb::Effect { .. } | MotionVerb::Loop { .. } => ("normal", "glide"),
        MotionVerb::Show(_) => {
            let enter = opt_name(&s.opts, "enter").unwrap_or(&defaults.enter);
            if enter == "pop" { ("normal", "pop") } else { ("normal", "settle") }
        }
        _ => ("normal", "settle"),
    };
    // An accent must stay long enough to be seen.
    let is_accent = matches!(&s.verb, MotionVerb::Effect { name, .. } if name.node == "accent");
    let dur = match opt(&s.opts, "duration") {
        None if is_accent => 1.2,
        None => tokens.duration(def_dur).unwrap_or(0.5),
        Some(v) => match &v.node {
            MotionValue::Number(n) => *n,
            MotionValue::Name(n) => tokens.duration(n).ok_or_else(|| {
                let mut known: Vec<_> = tokens.durations.keys().cloned().collect();
                known.sort();
                err(format!("unknown duration '{}': use seconds or one of {}", n, known.join(", ")), v)
            })?,
            _ => return Err(err("duration: expected seconds or fast|normal|slow".into(), v)),
        },
    };
    let ease_name = opt_name(&s.opts, "ease").unwrap_or(def_ease).to_string();
    let ease = match opt(&s.opts, "ease") {
        None => tokens.ease(def_ease).unwrap_or(Ease::Linear),
        Some(v) => match &v.node {
            MotionValue::Name(n) => tokens.ease(n).ok_or_else(|| {
                let mut known: Vec<_> = tokens.eases.keys().cloned().collect();
                known.sort();
                err(format!("unknown ease '{}': use one of {}", n, known.join(", ")), v)
            })?,
            _ => return Err(err("ease: expected a named ease (pop, settle, glide, snap, linear)".into(), v)),
        },
    };
    let delay = opt_number(&s.opts, "delay").unwrap_or(0.0);
    let stagger = opt_number(&s.opts, "stagger").unwrap_or(0.0);
    let order = opt_name(&s.opts, "order")
        .or_else(|| match opt_name(&s.opts, "from") {
            Some(o @ ("start" | "end" | "center" | "random")) => Some(o),
            _ => None,
        })
        .unwrap_or("start")
        .to_string();
    Ok(Timing { delay, dur, ease, ease_name, stagger, order })
}

/// Stagger rank of target `i` of `n`.
fn stagger_rank(order: &str, i: usize, n: usize, seed: u64) -> f64 {
    match order {
        "end" => (n - 1 - i) as f64,
        "center" => (i as f64 - (n as f64 - 1.0) / 2.0).abs(),
        "random" => {
            // Deterministic shuffle: rank = position of i in a seeded permutation.
            let mut idx: Vec<usize> = (0..n).collect();
            let mut x = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            for k in (1..n).rev() {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                let j = (x % (k as u64 + 1)) as usize;
                idx.swap(k, j);
            }
            idx.iter().position(|&v| v == i).unwrap_or(i) as f64
        }
        _ => i as f64,
    }
}

/// A timed statement (with its per-target starts) in source order.
struct TimedStmt<'a> {
    stmt: &'a MotionStmt,
    span: Span,
    starts: Vec<f64>,
    timing: Timing,
}

/// What events are timed against: the base layout, how far each line is
/// drawn when the frame starts, and the beats seen so far.
struct EventCtx<'b> {
    base: &'b LayoutResult,
    drawn: HashMap<String, f64>,
    beats: HashMap<String, f64>,
    show: &'b dyn Fn(&str) -> String,
    /// Resolved events, for the timeline.
    events: Vec<(f64, String, usize)>,
}

/// Progress p reached by an ease, as a fraction of its duration.
fn ease_inverse(e: Ease, p: f64) -> f64 {
    if p <= 0.0 {
        return 0.0;
    }
    // The first time the curve reaches p (an overshooting ease may reach it
    // twice): scan, then bisect.
    let n = 200;
    let mut lo = 0.0;
    for k in 1..=n {
        let x = k as f64 / n as f64;
        if e.eval(x) >= p {
            let mut hi = x;
            for _ in 0..40 {
                let mid = (lo + hi) / 2.0;
                if e.eval(mid) >= p {
                    hi = mid;
                } else {
                    lo = mid;
                }
            }
            return hi;
        }
        lo = x;
    }
    1.0
}

fn event_time(ev: &MotionEvent, span: &Span, out: &[TimedStmt], ctx: &EventCtx) -> Result<f64, TimingError> {
    let err = |m: String| TimingError(m, span.clone());
    let last_of = |id: &str, pick: &dyn Fn(&MotionVerb) -> bool| -> Option<(f64, &TimedStmt)> {
        out.iter().rev().find_map(|ts| {
            if !pick(&ts.stmt.verb) {
                return None;
            }
            let i = ts.stmt.targets.iter().position(|t| t == id)?;
            Some((ts.starts.get(i).copied().unwrap_or(0.0), ts))
        })
    };
    let show = ctx.show;
    match ev {
        MotionEvent::BeatEnd(b) => ctx
            .beats
            .get(&b.node)
            .copied()
            .ok_or_else(|| err(format!("after {}: no `beat {} {{ ... }}` earlier in this keyframe", b.node, b.node))),
        MotionEvent::Arrives(e) => last_of(&e.node, &|v| matches!(v, MotionVerb::Move { .. } | MotionVerb::Fly { .. }))
            .map(|(s, ts)| s + ts.timing.dur)
            // A transform that moves, turns or scales it: when that ends.
            .or_else(|| {
                use crate::parser::ast::StyleKey as K;
                out.iter().rev().find_map(|ts| {
                    let MotionVerb::Transform { modifiers, .. } = &ts.stmt.verb else { return None };
                    let geometric = modifiers.iter().any(|m| {
                        matches!(m.node.key.node, K::X | K::Y | K::Dx | K::Dy | K::Rotation | K::Scale | K::Width | K::Height)
                    });
                    if !geometric {
                        return None;
                    }
                    let i = ts.stmt.targets.iter().position(|t| *t == e.node)?;
                    Some(ts.starts.get(i).copied().unwrap_or(0.0) + ts.timing.dur)
                })
            })
            // `show x [from: y]` flies it in: it arrives when it lands.
            .or_else(|| {
                out.iter().rev().find_map(|ts| {
                    if !matches!(ts.stmt.verb, MotionVerb::Show(_)) || opt(&ts.stmt.opts, "from").is_none() {
                        return None;
                    }
                    let i = ts.stmt.targets.iter().position(|t| *t == e.node)?;
                    Some(ts.starts.get(i).copied().unwrap_or(0.0) + ts.timing.dur)
                })
            })
            // Moved by a layout change (`use layout beside`, a constraint):
            // when that change has played.
            .or_else(|| {
                out.iter()
                    .rev()
                    .find(|ts| matches!(ts.stmt.verb, MotionVerb::Constrain(_) | MotionVerb::Enable(_) | MotionVerb::Disable(_)))
                    .map(|ts| ts.starts.first().copied().unwrap_or(0.0) + ts.timing.dur)
            })
            .ok_or_else(|| err(format!(
                "when {} arrives: nothing moves, flies, turns or resizes {} earlier in this keyframe (and no layout changes); for an entrance use `when {} shown`",
                show(&e.node), show(&e.node), show(&e.node)
            ))),
        MotionEvent::Shown(e) => last_of(&e.node, &|v| matches!(v, MotionVerb::Show(_)))
            .map(|(s, ts)| s + ts.timing.dur)
            .ok_or_else(|| err(format!("when {} shown: no `show {}` earlier in this keyframe", show(&e.node), show(&e.node)))),
        MotionEvent::Hidden(e) => last_of(&e.node, &|v| matches!(v, MotionVerb::Hide(_)))
            .map(|(s, ts)| s + ts.timing.dur)
            .ok_or_else(|| err(format!("when {} hidden: no `hide {}` earlier in this keyframe", show(&e.node), show(&e.node)))),
        MotionEvent::Reaches { line, target } => {
            let is_draw = |v: &MotionVerb| matches!(v, MotionVerb::Draw(_) | MotionVerb::Undraw(_));
            let draws: Vec<(f64, &TimedStmt)> = out
                .iter()
                .filter(|ts| is_draw(&ts.stmt.verb))
                .filter_map(|ts| {
                    let i = ts.stmt.targets.iter().position(|t| *t == line.node)?;
                    Some((ts.starts.get(i).copied().unwrap_or(0.0), ts))
                })
                .collect();
            let Some((start, ts)) = draws.last().copied() else {
                return Err(err(format!(
                    "when {} reaches {}: nothing draws {} earlier in this keyframe",
                    show(&line.node), show(&target.node), show(&line.node)
                )));
            };
            let to_of = |ts: &TimedStmt| -> Option<f64> {
                let undraw = matches!(ts.stmt.verb, MotionVerb::Undraw(_));
                let to = opt(&ts.stmt.opts, "to")
                    .and_then(|v| crate::motion::draw_to(&v.node))
                    .unwrap_or(DrawTo::Fraction(if undraw { 0.0 } else { 1.0 }));
                resolve_draw_to(ctx.base, &line.node, &to)
            };
            let from = if draws.len() >= 2 {
                to_of(draws[draws.len() - 2].1).unwrap_or(0.0)
            } else {
                ctx.drawn.get(&line.node).copied().unwrap_or(1.0)
            };
            let to = to_of(ts).unwrap_or(1.0);
            let at = resolve_draw_to(ctx.base, &line.node, &DrawTo::Element(target.node.clone()))
                .ok_or_else(|| err(format!("when {} reaches {}: {} is not on {}", show(&line.node), show(&target.node), show(&target.node), show(&line.node))))?;
            let p = if (to - from).abs() < 1e-9 { -1.0 } else { (at - from) / (to - from) };
            if !(-1e-6..=1.0 + 1e-6).contains(&p) {
                return Err(err(format!(
                    "when {} reaches {}: its last draw goes from {:.0}% to {:.0}% of the line, and {} is at {:.0}%",
                    show(&line.node), show(&target.node), from * 100.0, to * 100.0, show(&target.node), at * 100.0
                )));
            }
            Ok(start + ts.timing.dur * ease_inverse(ts.timing.ease, p.clamp(0.0, 1.0)))
        }
    }
}

fn walk<'a>(
    nodes: &'a [Spanned<MotionNode>],
    start: f64,
    tokens: &MotionTokens,
    defaults: &Defaults,
    out: &mut Vec<TimedStmt<'a>>,
    ctx: &mut EventCtx,
) -> Result<f64, TimingError> {
    let mut cursor = start;
    let mut end = start;
    for n in nodes {
        match &n.node {
            MotionNode::Stmt(s) => {
                let t = timing(s, tokens, defaults)?;
                let count = s.targets.len().max(1);
                let starts: Vec<f64> = (0..count)
                    .map(|i| cursor + t.delay + t.stagger * stagger_rank(&t.order, i, count, n.span.start as u64))
                    .collect();
                let stmt_dur = match &s.verb {
                    MotionVerb::Loop { .. } => 0.0,
                    MotionVerb::Constrain(_) | MotionVerb::Disable(_) | MotionVerb::Enable(_) => t.dur,
                    _ => t.dur,
                };
                for st in &starts {
                    end = end.max(st + stmt_dur);
                }
                out.push(TimedStmt { stmt: s, span: n.span.clone(), starts, timing: t });
            }
            MotionNode::Then(b) => {
                let s = end;
                let e = walk(b, s, tokens, defaults, out, ctx)?;
                cursor = s;
                end = end.max(e);
            }
            MotionNode::At(t, b) => {
                let s = *t;
                let e = walk(b, s, tokens, defaults, out, ctx)?;
                cursor = s;
                end = end.max(e);
            }
            MotionNode::After(d, b) => {
                let s = cursor + d;
                let e = walk(b, s, tokens, defaults, out, ctx)?;
                cursor = s;
                end = end.max(e);
            }
            MotionNode::When(ev, off, b) => {
                let at = event_time(ev, &n.span, out, ctx)?;
                let s = (at + off).max(0.0);
                let show = ctx.show;
                let what = match ev {
                    MotionEvent::Reaches { line, target } => format!("{} reaches {}", show(&line.node), show(&target.node)),
                    MotionEvent::Arrives(e) => format!("{} arrives", show(&e.node)),
                    MotionEvent::Shown(e) => format!("{} shown", show(&e.node)),
                    MotionEvent::Hidden(e) => format!("{} hidden", show(&e.node)),
                    MotionEvent::BeatEnd(b) => format!("after {}", b.node),
                };
                let nudge = if off.abs() > 1e-9 { format!(" {} {}", if *off < 0.0 { "-" } else { "+" }, off.abs()) } else { String::new() };
                ctx.events.push((s, format!("when {}{} (= {:.2}s{})", what, nudge, at, if nudge.is_empty() { String::new() } else { format!(", starts {:.2}s", s) }), out.len()));
                let e = walk(b, s, tokens, defaults, out, ctx)?;
                end = end.max(e);
            }
            MotionNode::Beat(name, b) => {
                let e = walk(b, cursor, tokens, defaults, out, ctx)?;
                ctx.events.push((e, format!("beat {} ends (= {:.2}s)", name, e), usize::MAX));
                ctx.beats.insert(name.clone(), e);
                end = end.max(e);
            }
        }
    }
    Ok(end)
}

// ------------------------------------------------------------- compile

impl<'a> Compiler<'a> {
    fn new(input: &'a CompileInput<'a>) -> Self {
        let mut elems = BTreeMap::new();
        let mut origins = HashMap::new();
        walk_elements(&input.base.root_elements, None, &mut |e, parent| {
            if let Some(id) = &e.id {
                if matches!(e.element_type, ElementType::GridCell) {
                    return;
                }
                let c = e.pivot_point();
                origins.insert(id.0.clone(), (c.x, c.y));
                elems.insert(
                    id.0.clone(),
                    ElemInfo {
                        parent: parent.map(str::to_string),
                        base: e.bounds,
                        is_rect: matches!(e.element_type, ElementType::Shape(ShapeType::Rectangle)),
                        base_text: element_text(e),
                        text_like: matches!(e.element_type, ElementType::Shape(ShapeType::Text { .. }))
                            || (matches!(e.element_type, ElementType::Shape(ShapeType::Rectangle))
                                && e.styles.fill.as_deref().is_none_or(|f| f == "none")
                                && e.styles.stroke.as_deref().is_none_or(|s| s == "none")),
                    },
                );
            }
        });
        let mut defaults = Defaults { enter: "rise".into(), exit: "fade".into() };
        for st in &input.doc.statements {
            if let Statement::MotionDefaults(opts) = &st.node {
                if let Some(e) = opt_name(opts, "enter") {
                    defaults.enter = e.to_string();
                }
                if let Some(e) = opt_name(opts, "exit") {
                    defaults.exit = e.to_string();
                }
            }
        }
        let (drawables, drawn_initial, heads) = drawable_info(input.doc, input.base);
        let aux = Aux {
            parents: elems.iter().map(|(k, v): (&String, &ElemInfo)| (k.clone(), v.parent.clone())).collect(),
            origins,
            drawables: drawables.clone(),
            heads: heads.clone(),
            text_variants: input.text_variants.clone(),
            conn_variants: input.conn_variants.clone(),
            ..Default::default()
        };
        Compiler {
            input,
            elems,
            drawables,
            drawn_initial,
            heads,
            defaults,
            channels: Vec::new(),
            ch_index: HashMap::new(),
            aux,
            scope: input.scope.to_string(),
            current_opts: Vec::new(),
            display: super::expand::ElementIndex::build(input.doc).display,
            pre_applied: HashSet::new(),
        }
    }
}

/// A declared `drawn:` value as a draw target.
fn drawn_decl(mods: &[Spanned<crate::parser::ast::StyleModifier>]) -> Option<(DrawTo, Span)> {
    use crate::parser::ast::{StyleKey, StyleValue};
    mods.iter().find_map(|m| {
        if !matches!(&m.node.key.node, StyleKey::Custom(k) if k == "drawn") {
            return None;
        }
        let to = match &m.node.value.node {
            StyleValue::Number { value, unit: Some(u) } if u == "%" => DrawTo::Fraction((value / 100.0).clamp(0.0, 1.0)),
            StyleValue::Number { value, .. } => DrawTo::Fraction(value.clamp(0.0, 1.0)),
            StyleValue::Keyword(k) if k == "none" => DrawTo::Fraction(0.0),
            StyleValue::Identifier(id) if id.0 == "none" => DrawTo::Fraction(0.0),
            StyleValue::Identifier(id) => DrawTo::Element(id.0.replace('.', "_")),
            _ => return None,
        };
        Some((to, m.node.value.span.clone()))
    })
}

/// Every `drawn:` declaration: drawable id -> (how far, where it was written).
pub fn declared_drawn(doc: &Document) -> BTreeMap<String, (DrawTo, Span)> {
    fn walk(stmts: &[Spanned<Statement>], out: &mut BTreeMap<String, (DrawTo, Span)>) {
        for st in stmts {
            match &st.node {
                Statement::Shape(s) => {
                    if let ShapeType::Path(p) = &s.shape_type.node {
                        let id = s.name.as_ref().or(p.name.as_ref()).map(|n| n.node.0.clone());
                        if let (Some(id), Some(d)) = (id, drawn_decl(&s.modifiers)) {
                            out.insert(id, d);
                        }
                    }
                }
                Statement::Connection(conns) => {
                    for c in conns {
                        if let (Some(n), Some(d)) = (&c.name, drawn_decl(&c.modifiers)) {
                            out.insert(n.node.0.clone(), d);
                        }
                    }
                }
                Statement::Group(g) => walk(&g.children, out),
                Statement::Layout(l) => walk(&l.children, out),
                _ => {}
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(&doc.statements, &mut out);
    out
}

/// Drawables (id -> is a connection), how far each is drawn before any
/// keyframe, and the drawn connections whose arrowhead becomes its own node.
///
/// A line is drawn as declared: fully, unless its declaration says
/// `drawn: 0 | 60% | <element on it>`. A later `draw` never reaches back and
/// changes how it starts.
pub fn drawable_info(
    doc: &Document,
    base: &LayoutResult,
) -> (BTreeMap<String, bool>, BTreeMap<String, f64>, BTreeSet<String>) {
    let mut drawables = BTreeMap::new();
    let mut drawn_initial = BTreeMap::new();
    let mut heads = BTreeSet::new();
    let is_conn = |t: &str| base.connections.iter().any(|c| c.name.as_ref().is_some_and(|n| n.0 == t));
    for (id, (to, _)) in declared_drawn(doc) {
        let f = resolve_draw_to(base, &id, &to).unwrap_or(1.0);
        drawables.insert(id.clone(), is_conn(&id));
        drawn_initial.insert(id, f);
    }
    for kf in crate::layout::keyframe::extract_keyframes(doc) {
        super::for_each_stmt(&kf.motion, &mut |n| {
            let MotionNode::Stmt(s) = &n.node else { return };
            let draws = matches!(s.verb, MotionVerb::Draw(_) | MotionVerb::Undraw(_))
                || matches!(s.verb, MotionVerb::Show(_)) && opt_name(&s.opts, "enter") == Some("draw");
            if !draws {
                return;
            }
            for t in &s.targets {
                drawables.insert(t.clone(), is_conn(t));
                drawn_initial.entry(t.clone()).or_insert(1.0);
            }
        });
    }
    for (t, conn) in &drawables {
        if *conn && has_marker(base, t) {
            heads.insert(t.clone());
        }
    }
    (drawables, drawn_initial, heads)
}

/// A draw target that is not on the line is a mistake, not a clamp.
pub fn check_draw_target(layout: &LayoutResult, id: &str, to: &DrawTo, what: &str) -> Result<(), String> {
    let Some(pl) = drawable_polyline(layout, id) else {
        return Err(format!("{}: '{}' is not a connection or path", what, id));
    };
    match to {
        DrawTo::Fraction(_) => Ok(()),
        DrawTo::Vertex(n) => {
            let count = connection_vertices(layout, id).map(|v| v.len()).unwrap_or(pl.vertices.len());
            if *n < count {
                Ok(())
            } else {
                Err(format!("{}: '{}' has {} vertices (0..{}), there is no vertex {}", what, id, count, count.saturating_sub(1), n))
            }
        }
        DrawTo::Element(e) => {
            let Some(el) = layout.get_element_by_name(e) else {
                return Err(format!("{}: '{}' is not an element", what, e));
            };
            let (_, dist) = pl.project(el.bounds.center());
            let tolerance = (el.bounds.width.min(el.bounds.height) / 2.0).max(8.0);
            if dist <= tolerance {
                Ok(())
            } else {
                Err(format!(
                    "{}: '{}' is not on '{}' (the line passes {:.0}px from it); \
                     route the line through it (`through: [...]`) or draw to something on it",
                    what, e, id, dist
                ))
            }
        }
    }
}

/// Everything the compiler reports as a problem.
#[derive(Debug, Clone)]
pub struct CompileError {
    pub message: String,
    pub span: Span,
}

pub fn compile(input: &CompileInput) -> Result<Motion, CompileError> {
    let mut c = Compiler::new(input);
    // A camera needs its group before any snapshot is taken.
    let keyframes = crate::layout::keyframe::extract_keyframes(input.doc);
    c.aux.camera = keyframes.iter().any(|k| {
        let mut found = false;
        super::for_each_stmt(&k.motion, &mut |n| {
            if let MotionNode::Stmt(MotionStmt { verb: MotionVerb::Camera(_), .. }) = &n.node {
                found = true;
            }
        });
        found
    });

    // Pre-state: frame 0 sets the scene with what it does at its start
    // (applied instantly), except what it brings on stage (shows, draws,
    // swaps-in, counts), which enter. What it does later (behind `when`,
    // `then`, `at`, `after`, a delay) is its animation, and plays.
    let mut state = FrameState::initial();
    if let Some(kf0) = keyframes.first() {
        for op in &kf0.operations {
            if let KeyframeOp::Show(targets) = &op.node {
                for t in targets {
                    state.hidden_elements.insert(t.node.0.clone());
                    state.hidden_connections.insert(t.node.0.clone());
                }
            }
        }
        fn at_start(nodes: &[Spanned<MotionNode>], out: &mut Vec<KeyframeOp>, spans: &mut HashSet<(usize, usize)>) {
            for n in nodes {
                match &n.node {
                    MotionNode::Stmt(s) => {
                        let delayed = opt_number(&s.opts, "delay").is_some_and(|d| d > 1e-9);
                        if !delayed {
                            out.extend(crate::motion::expand::stmt_state_ops(s, &n.span));
                            spans.insert((n.span.start, n.span.end));
                        }
                    }
                    MotionNode::After(t, b) | MotionNode::At(t, b) if *t <= 1e-9 => at_start(b, out, spans),
                    MotionNode::Beat(_, b) => at_start(b, out, spans),
                    _ => {}
                }
            }
        }
        let mut ops = Vec::new();
        at_start(&kf0.motion, &mut ops, &mut c.pre_applied);

        for op in &ops {
            match op {
                KeyframeOp::Show(_) | KeyframeOp::Draw { .. } => {}
                KeyframeOp::Transform { modifiers, .. }
                    if modifiers.iter().any(|m| matches!(m.node.key.node, crate::parser::ast::StyleKey::Label)) => {}
                other => state.apply(other),
            }
        }
    }
    // A declared `drawn: X` must name something on the line.
    for (id, (to, span)) in declared_drawn(input.doc) {
        check_draw_target(input.base, &id, &to, &format!("drawn: on {}", c.show(&id)))
            .map_err(|m| CompileError { message: c.dotted(&m), span })?;
    }
    let mut layout = c.solve(&state);
    let mut snap = c.snapshot(&state, &layout, keyframes.first().map(|k| k.name.node.as_str()));

    let mut settled_maps: Vec<BTreeMap<ChKey, Val>> = vec![snap.vals.clone()];
    let mut frames: Vec<FrameMotion> = Vec::new();
    // Curves are keyed by channel index; collected per frame.
    for (fi, kf) in keyframes.iter().enumerate() {
        let mut timed = Vec::new();
        // How far each line is drawn as this frame starts.
        let drawn: HashMap<String, f64> = c
            .drawables
            .keys()
            .map(|id| {
                let f = state
                    .drawn
                    .get(id)
                    .and_then(|to| resolve_draw_to(input.base, id, to))
                    .or_else(|| c.drawn_initial.get(id).copied())
                    .unwrap_or(1.0);
                (id.clone(), f)
            })
            .collect();
        let show_fn = |id: &str| c.show(id);
        let mut ctx = EventCtx { base: input.base, drawn, beats: HashMap::new(), show: &show_fn, events: Vec::new() };
        let duration = walk(&kf.motion, 0.0, input.tokens, &c.defaults, &mut timed, &mut ctx)
            .map_err(|TimingError(m, s)| CompileError { message: m, span: s })?;
        let mut fm = FrameMotion {
            name: kf.name.node.clone(),
            duration,
            auto: kf.auto,
            title: kf.title.clone(),
            note: kf.note.clone(),
            atoms: Vec::new(),
            curves: BTreeMap::new(),
            events: std::mem::take(&mut ctx.events),
        };
        let mut first_atom = Vec::with_capacity(timed.len());
        for ts in &timed {
            first_atom.push(fm.atoms.len());
            c.run_stmt(fi, &kf.name.node, ts, &mut state, &mut layout, &mut snap, &mut fm)?;
        }
        for ev in &mut fm.events {
            ev.2 = first_atom.get(ev.2).copied().unwrap_or(usize::MAX);
        }
        // Loops and transients may outlast statements only by design; the
        // frame lasts as long as its longest non-looping segment.
        let mut dur: f64 = 0.0;
        for curve in fm.curves.values() {
            for tw in &curve.tweens {
                dur = dur.max(tw.start + tw.dur);
            }
            for ov in curve.overlays.iter().filter(|o| !o.looping) {
                dur = dur.max(ov.start + ov.dur);
            }
        }
        fm.duration = dur;
        settled_maps.push(snap.vals.clone());
        frames.push(fm);
    }

    // Keep only channels that vary or are animated.
    let mut all_keys: BTreeSet<ChKey> = BTreeSet::new();
    for m in &settled_maps {
        all_keys.extend(m.keys().cloned());
    }
    let animated: BTreeSet<usize> = frames.iter().flat_map(|f| f.curves.keys().copied()).collect();
    for key in &all_keys {
        let vals: Vec<Option<&Val>> = settled_maps.iter().map(|m| m.get(key)).collect();
        let first = vals.iter().flatten().next();
        let varies = vals.iter().any(|v| match (v, first) {
            (Some(v), Some(f)) => !v.close(f),
            (None, _) => true,
            _ => false,
        });
        if varies {
            c.ch(key.sel.clone(), key.prop);
        }
    }
    let _ = animated;

    // Fill the settled table; defaults for channels that only animate.
    let neutral = |k: &ChKey| -> Val {
        let aux_hook = ["aighost-", "aiflash-", "aihl-", "aiping-", "aitick-", "aiacu-", "aiacr-", "aiaco-", "aiacb-"]
            .iter()
            .any(|p| k.sel.starts_with(&format!(".{}", p)));
        match k.prop {
            Prop::Opacity if aux_hook => Val::n(0.0),
            Prop::Opacity => Val::n(1.0),
            Prop::Translate => Val::xy(0.0, 0.0),
            Prop::Scale => Val::xy(1.0, 1.0),
            Prop::Rotate => Val::n(0.0),
            Prop::ClipPath => Val::V(vec![0.0, 0.0, 0.0, 0.0]),
            Prop::DashOffset => Val::n(0.0),
            _ => Val::S(String::new()),
        }
    };
    let settled: Vec<Vec<Val>> = settled_maps
        .iter()
        .map(|m| c.channels.iter().map(|k| m.get(k).cloned().unwrap_or_else(|| neutral(k))).collect())
        .collect();
    // Drop curves on channels that turned out not to matter and never
    // animate away from their settled value (e.g. a translate that is 0).
    for f in &mut frames {
        f.curves.retain(|_, curve| !curve.tweens.is_empty() || !curve.overlays.is_empty());
    }
    let view_center = c.view_center();
    let display = c.display.clone();
    Ok(Motion {
        display,
        scope: c.scope.clone(),
        channels: c.channels,
        settled,
        frames,
        aux: c.aux,
        view_center,
    })
}

impl<'a> Compiler<'a> {
    fn show(&self, id: &str) -> String {
        self.display.get(id).cloned().unwrap_or_else(|| id.to_string())
    }

    /// Rewrite quoted internal ids in a message as the author writes them.
    fn dotted(&self, msg: &str) -> String {
        let mut out = msg.to_string();
        for (id, shown) in &self.display {
            if id != shown {
                out = out.replace(&format!("'{}'", id), &format!("'{}'", shown));
            }
        }
        out
    }

    fn curve<'f>(&mut self, fm: &'f mut FrameMotion, sel: String, prop: Prop) -> &'f mut Curve {
        let i = self.ch(sel, prop);
        fm.curves.entry(i).or_default()
    }

    fn tween(&mut self, fm: &mut FrameMotion, key: &ChKey, tw: Tween) {
        self.curve(fm, key.sel.clone(), key.prop).tweens.push(tw);
    }

    fn overlay(&mut self, fm: &mut FrameMotion, sel: String, prop: Prop, ov: Overlay) {
        self.curve(fm, sel, prop).overlays.push(ov);
    }

    fn center_now(&self, layout: &LayoutResult, id: &str) -> Option<Point> {
        layout.get_element_by_name(id).map(|e| e.bounds.center())
    }

    fn base_center(&self, id: &str) -> Option<Point> {
        self.elems.get(id).map(|e| e.base.center())
    }

    /// Current wrapper translate of `id` (relative to its parent hook).
    fn translate_now(&self, snap: &Snap, id: &str) -> (f64, f64) {
        match snap.vals.get(&ChKey { sel: self.wrap(id), prop: Prop::Translate }) {
            Some(Val::V(v)) => (v[0], v[1]),
            _ => (0.0, 0.0),
        }
    }

    /// Sum of ancestors' translates: what the wrapper's own translate sits on.
    fn ancestors_translate(&self, snap: &Snap, id: &str) -> (f64, f64) {
        let mut acc = (0.0, 0.0);
        let mut cur = self.elems.get(id).and_then(|e| e.parent.clone());
        while let Some(p) = cur {
            let t = self.translate_now(snap, &p);
            acc = (acc.0 + t.0, acc.1 + t.1);
            cur = self.elems.get(&p).and_then(|e| e.parent.clone());
        }
        acc
    }

    #[allow(clippy::too_many_arguments)]
    fn run_stmt(
        &mut self,
        fi: usize,
        frame_name: &str,
        ts: &TimedStmt,
        state: &mut FrameState,
        layout: &mut LayoutResult,
        snap: &mut Snap,
        fm: &mut FrameMotion,
    ) -> Result<(), CompileError> {
        let s = ts.stmt;
        let t = &ts.timing;
        self.current_opts = s.opts.clone();
        let n = s.targets.len().max(1);
        for i in 0..n {
            let start = ts.starts[i.min(ts.starts.len() - 1)];
            let target = s.targets.get(i).cloned().unwrap_or_default();
            let atom = fm.atoms.len();
            fm.atoms.push(Atom {
                start,
                dur: t.dur,
                ease: t.ease,
                ease_name: t.ease_name.clone(),
                verb: describe_verb(s),
                target: {
                    let show = |x: &str| self.display.get(x).cloned().unwrap_or_else(|| x.to_string());
                    match s.partners.get(i) {
                        Some(p) => format!("{} -> {}", show(&target), show(p)),
                        None => show(&target),
                    }
                },
                span: ts.span.clone(),
                kind: match &s.verb {
                    MotionVerb::Effect { name, .. } => name.node.clone(),
                    other => verb_name(other).to_string(),
                },
                target_id: target.clone(),
                dest_id: s.partners.get(i).cloned(),
            });

            if let MotionVerb::Draw(_) | MotionVerb::Undraw(_) = &s.verb {
                if let Some(to) = opt(&s.opts, "to").and_then(|v| super::draw_to(&v.node)) {
                    let what = format!("{} {}", verb_name(&s.verb), self.show(&target));
                    super::compile::check_draw_target(layout, &target, &to, &what).map_err(|m| CompileError {
                        message: self.dotted(&m),
                        span: ts.span.clone(),
                    })?;
                }
            }

            // State change, for this target only.
            let ops = super::expand::atom_state_ops(s, i, &ts.span);
            // Frame 0 already carries the non-entering changes of what it
            // does at its start (pre-state); what it does later plays.
            let pre = fi == 0 && self.pre_applied.contains(&(ts.span.start, ts.span.end));
            let ops: Vec<KeyframeOp> = if pre {
                ops.into_iter()
                    .filter(|op| {
                        matches!(op, KeyframeOp::Show(_) | KeyframeOp::Draw { .. })
                            || matches!(op, KeyframeOp::Transform { modifiers, .. }
                                if modifiers.iter().any(|m| matches!(m.node.key.node, crate::parser::ast::StyleKey::Label)))
                    })
                    .collect()
            } else {
                ops
            };
            let mut geometry = false;
            for op in &ops {
                geometry |= FrameState::op_changes_geometry(op);
                state.apply(op);
            }
            if geometry {
                *layout = self.solve(state);
            }
            let before = std::mem::replace(snap, self.snapshot(state, layout, Some(frame_name)));
            let mut changed: Vec<ChKey> = Vec::new();
            for (k, v) in &snap.vals {
                match before.vals.get(k) {
                    Some(old) if old.close(v) => {}
                    _ => changed.push(k.clone()),
                }
            }
            let prev = |k: &ChKey| before.vals.get(k).cloned();
            self.attribute(s, i, &target, start, t, atom, &changed, &before, snap, layout, fm, &prev)?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn attribute(
        &mut self,
        s: &MotionStmt,
        i: usize,
        target: &str,
        start: f64,
        t: &Timing,
        atom: usize,
        changed: &[ChKey],
        before: &Snap,
        snap: &Snap,
        layout: &LayoutResult,
        fm: &mut FrameMotion,
        _prev: &dyn Fn(&ChKey) -> Option<Val>,
    ) -> Result<(), CompileError> {
        let tokens = self.input.tokens;
        let dist = tokens.distance;
        let fast = tokens.duration("fast").unwrap_or(0.25);
        let wrap = self.wrap(target);
        let is_elem = self.elems.contains_key(target);
        let mut handled: BTreeSet<ChKey> = BTreeSet::new();
        let op_key = ChKey { sel: wrap.clone(), prop: Prop::Opacity };
        let tr_key = ChKey { sel: wrap.clone(), prop: Prop::Translate };
        let became_visible = changed.contains(&op_key)
            && before.vals.get(&op_key).is_some_and(|v| v.close(&Val::n(0.0)));
        let became_hidden = changed.contains(&op_key)
            && snap.vals.get(&op_key).is_some_and(|v| v.close(&Val::n(0.0)));
        let simple = |start: f64, dur: f64, ease: Ease, to: Val| Tween { start, dur, ease, from: None, to, atom };

        match &s.verb {
            MotionVerb::Show(_) if became_visible && is_elem => {
                let enter = opt(&s.opts, "enter").map(|v| v.node.clone());
                let enter_name = match &enter {
                    Some(MotionValue::Name(n)) | Some(MotionValue::Call(n, _)) => n.clone(),
                    _ => self.defaults.enter.clone(),
                };
                let settled_op = snap.vals.get(&op_key).cloned().unwrap_or(Val::n(1.0));
                let op_dur = if enter_name == "pop" { t.dur.min(fast) } else { t.dur };
                self.tween(fm, &op_key, simple(start, op_dur, if enter_name == "pop" { Ease::Linear } else { t.ease }, settled_op));
                handled.insert(op_key.clone());
                let settled_tr = match snap.vals.get(&tr_key) {
                    Some(Val::V(v)) => (v[0], v[1]),
                    _ => (0.0, 0.0),
                };
                if let Some(from) = opt_name(&s.opts, "from").filter(|f| !matches!(*f, "start" | "end" | "center" | "random")) {
                    let from = from.replace('.', "_");
                    let (Some(src), Some(me)) = (self.center_now(layout, &from), self.center_now(layout, target)) else {
                        return Err(CompileError { message: format!("show {} [from: {}]: '{}' is not an element", target, from, from), span: fm.atoms[atom].span.clone() });
                    };
                    let (ox, oy) = (src.x - me.x, src.y - me.y);
                    self.tween(fm, &tr_key, Tween {
                        start, dur: t.dur, ease: t.ease,
                        from: Some(Val::xy(settled_tr.0 + ox, settled_tr.1 + oy)),
                        to: Val::xy(settled_tr.0, settled_tr.1), atom,
                    });
                    handled.insert(tr_key.clone());
                    let scale0 = opt_number(&s.opts, "scale").unwrap_or_else(|| {
                        match (layout.get_element_by_name(&from), layout.get_element_by_name(target)) {
                            (Some(a), Some(b)) if b.bounds.width > 0.0 && b.bounds.height > 0.0 => {
                                (a.bounds.width / b.bounds.width).min(a.bounds.height / b.bounds.height)
                            }
                            _ => 1.0,
                        }
                    });
                    if (scale0 - 1.0).abs() > 1e-6 {
                        let sc = ChKey { sel: wrap.clone(), prop: Prop::Scale };
                        self.tween(fm, &sc, Tween { start, dur: t.dur, ease: t.ease, from: Some(Val::xy(scale0, scale0)), to: Val::xy(1.0, 1.0), atom });
                    }
                } else {
                    match enter_name.as_str() {
                        "fade" | "none" => {}
                        // expand: the height tween comes from the state
                        // change; the content is revealed top-down with it, so
                        // nothing spills out of a box that is still opening.
                        "expand" => {
                            let clip = ChKey { sel: wrap.clone(), prop: Prop::ClipPath };
                            self.tween(fm, &clip, Tween { start, dur: t.dur, ease: t.ease, from: Some(wipe_hidden("down")), to: Val::V(vec![0.0; 4]), atom });
                        }
                        "rise" | "drop" => {
                            let dy = if enter_name == "rise" { dist } else { -dist };
                            self.tween(fm, &tr_key, Tween {
                                start, dur: t.dur, ease: t.ease,
                                from: Some(Val::xy(settled_tr.0, settled_tr.1 + dy)),
                                to: Val::xy(settled_tr.0, settled_tr.1), atom,
                            });
                            handled.insert(tr_key.clone());
                        }
                        "pop" | "grow" => {
                            let sc = ChKey { sel: wrap.clone(), prop: Prop::Scale };
                            self.tween(fm, &sc, Tween { start, dur: t.dur, ease: t.ease, from: Some(Val::xy(0.0, 0.0)), to: Val::xy(1.0, 1.0), atom });
                        }
                        "wipe" => {
                            let dir = wipe_dir(enter.as_ref());
                            let clip = ChKey { sel: wrap.clone(), prop: Prop::ClipPath };
                            self.tween(fm, &clip, Tween { start, dur: t.dur, ease: t.ease, from: Some(wipe_hidden(&dir)), to: Val::V(vec![0.0; 4]), atom });
                        }
                        "draw" => {}
                        other => {
                            return Err(CompileError {
                                message: format!("unknown enter preset '{}': use fade, pop, rise, drop, grow, expand, wipe(left|right|up|down) or draw", other),
                                span: fm.atoms[atom].span.clone(),
                            })
                        }
                    }
                }
            }
            MotionVerb::Show(_) if became_visible_conn(changed, before, &self.scope, target) => {
                let enter = opt_name(&s.opts, "enter").unwrap_or("fade").to_string();
                if enter == "draw" {
                    let ok = ChKey { sel: format!(".conn-{}{}", self.scope, target), prop: Prop::Opacity };
                    self.tween(fm, &ok, simple(start, 0.0, Ease::Linear, Val::n(1.0)));
                    handled.insert(ok);
                    self.overlay(fm, format!(".aimask-{}{}", self.scope, target), Prop::DashOffset, Overlay {
                        start, dur: t.dur, ease: t.ease, keys: vec![Val::n(1.001), Val::n(0.0)], mode: Mode::Abs, looping: false, atom,
                    });
                    if self.heads.contains(target) {
                        self.overlay(fm, format!(".aihead-{}{}", self.scope, target), Prop::Opacity, Overlay {
                            start, dur: t.dur, ease: Ease::Linear, keys: vec![Val::n(0.0), Val::n(0.0), Val::n(0.0), Val::n(0.0), Val::n(0.0), Val::n(0.0), Val::n(0.0), Val::n(0.0), Val::n(1.0)], mode: Mode::Abs, looping: false, atom,
                        });
                    }
                }
            }
            MotionVerb::Hide(_) if became_hidden && is_elem => {
                let exit = opt(&s.opts, "exit").map(|v| v.node.clone());
                let exit_name = match &exit {
                    Some(MotionValue::Name(n)) | Some(MotionValue::Call(n, _)) => n.clone(),
                    _ => self.defaults.exit.clone(),
                };
                self.tween(fm, &op_key, simple(start, t.dur, t.ease, Val::n(0.0)));
                handled.insert(op_key.clone());
                let e_in = tokens.ease("in").unwrap_or(Ease::Linear);
                match exit_name.as_str() {
                    "fade" | "none" => {}
                    // collapse: the content is wiped up as the box closes.
                    "collapse" => self.overlay(fm, wrap.clone(), Prop::ClipPath, Overlay {
                        start, dur: t.dur, ease: t.ease, keys: vec![Val::V(vec![0.0; 4]), wipe_hidden("down")], mode: Mode::Abs, looping: false, atom,
                    }),
                    "shrink" => self.overlay(fm, wrap.clone(), Prop::Scale, Overlay {
                        start, dur: t.dur, ease: e_in, keys: vec![Val::xy(1.0, 1.0), Val::xy(0.0, 0.0)], mode: Mode::Mul, looping: false, atom,
                    }),
                    "fall" | "lift" => {
                        let dy = if exit_name == "fall" { dist } else { -dist };
                        self.overlay(fm, wrap.clone(), Prop::Translate, Overlay {
                            start, dur: t.dur, ease: e_in, keys: vec![Val::xy(0.0, 0.0), Val::xy(0.0, dy)], mode: Mode::Add, looping: false, atom,
                        })
                    }
                    "wipe" => {
                        let dir = wipe_dir(exit.as_ref());
                        self.overlay(fm, wrap.clone(), Prop::ClipPath, Overlay {
                            start, dur: t.dur, ease: t.ease, keys: vec![Val::V(vec![0.0; 4]), wipe_hidden(&opposite(&dir))], mode: Mode::Abs, looping: false, atom,
                        })
                    }
                    other => {
                        return Err(CompileError {
                            message: format!("unknown exit preset '{}': use fade, shrink, fall, lift, collapse or wipe(left|right|up|down)", other),
                            span: fm.atoms[atom].span.clone(),
                        })
                    }
                }
            }
            MotionVerb::Transform { .. } => {
                if let Some(swap) = opt_name(&s.opts, "swap") {
                    // Text change: roll the old wording out and the new one in.
                    let prefix = format!(".aitxt-{}{}", self.scope, target);
                    let text_keys: Vec<ChKey> = changed.iter().filter(|k| k.sel.starts_with(&prefix) && k.prop == Prop::Opacity).cloned().collect();
                    for k in &text_keys {
                        let on = snap.vals.get(k).is_some_and(|v| v.close(&Val::n(1.0)));
                        match swap {
                            "cut" => self.tween(fm, k, simple(start + t.dur / 2.0, 0.0, Ease::Linear, Val::n(if on { 1.0 } else { 0.0 }))),
                            "roll" => {
                                let half = t.dur / 2.0;
                                if on {
                                    self.tween(fm, k, simple(start + half * 0.6, half, t.ease, Val::n(1.0)));
                                    self.overlay(fm, k.sel.clone(), Prop::Translate, Overlay {
                                        start: start + half * 0.6, dur: half, ease: t.ease, keys: vec![Val::xy(0.0, dist), Val::xy(0.0, 0.0)], mode: Mode::Abs, looping: false, atom,
                                    });
                                } else {
                                    self.tween(fm, k, simple(start, half, tokens.ease("in").unwrap_or(Ease::Linear), Val::n(0.0)));
                                    self.overlay(fm, k.sel.clone(), Prop::Translate, Overlay {
                                        start, dur: half, ease: tokens.ease("in").unwrap_or(Ease::Linear), keys: vec![Val::xy(0.0, 0.0), Val::xy(0.0, -dist)], mode: Mode::Abs, looping: false, atom,
                                    });
                                }
                            }
                            "fade" => {
                                let half = t.dur / 2.0;
                                if on {
                                    self.tween(fm, k, simple(start + half * 0.5, half * 1.5, t.ease, Val::n(1.0)));
                                } else {
                                    self.tween(fm, k, simple(start, half, t.ease, Val::n(0.0)));
                                }
                            }
                            other => {
                                return Err(CompileError {
                                    message: format!("unknown swap '{}' for a label change: use roll, fade or cut", other),
                                    span: fm.atoms[atom].span.clone(),
                                })
                            }
                        }
                        handled.insert(k.clone());
                    }
                }
            }
            MotionVerb::Draw(_) | MotionVerb::Undraw(_) => {
                let hk = ChKey { sel: format!(".aihead-{}{}", self.scope, target), prop: Prop::Opacity };
                if changed.contains(&hk) {
                    let on = snap.vals.get(&hk).is_some_and(|v| v.close(&Val::n(1.0)));
                    let tip = (t.dur * 0.15).min(0.12);
                    let at = if on { start + t.dur - tip } else { start };
                    self.tween(fm, &hk, simple(at, tip, Ease::Linear, Val::n(if on { 1.0 } else { 0.0 })));
                    handled.insert(hk);
                }
            }
            MotionVerb::Move { along: Some(path), .. } if changed.contains(&tr_key) => {
                // Ride the path from where the element is to where it is pinned.
                if let Some(pl) = drawable_polyline(layout, &path.node.replace('.', "_")) {
                    let cur_center = self.center_before(before, target);
                    let end_center = self.center_now(layout, target);
                    if let (Some(a), Some(b)) = (cur_center, end_center) {
                        let fa = pl.project(a).0;
                        let fb = pl.project(b).0;
                        let base = self.base_center(target).unwrap_or(a);
                        let anc = self.ancestors_translate(snap, target);
                        let mut pts = pl.sub_points(fa, fb, 24);
                        // Start exactly where the element is, even if that is
                        // just off the line (a token sitting on a station).
                        pts[0] = a;
                        let keys: Vec<Val> = pts
                            .into_iter()
                            .map(|p| Val::xy(p.x - base.x - anc.0, p.y - base.y - anc.1))
                            .collect();
                        let to = snap.vals.get(&tr_key).cloned().unwrap_or(Val::xy(0.0, 0.0));
                        self.overlay(fm, wrap.clone(), Prop::Translate, Overlay { start, dur: t.dur, ease: t.ease, keys, mode: Mode::Abs, looping: false, atom });
                        self.tween(fm, &tr_key, simple(start + t.dur, 0.0, Ease::Linear, to));
                        handled.insert(tr_key.clone());
                    }
                }
            }
            MotionVerb::Swap { .. } => {
                let partner = s.partners.get(i).cloned().unwrap_or_default();
                let via = opt_name(&s.opts, "via").unwrap_or("fade");
                let pw = self.wrap(&partner);
                let pk = ChKey { sel: pw.clone(), prop: Prop::Opacity };
                let half = t.dur / 2.0;
                // A flip is one continuous turn: accelerate into edge-on and
                // decelerate out of it, so edge-on is an instant. Easing the
                // first half out (fast, then crawling edge-on) left every
                // page invisible for most of the turn: a blink.
                let (e_in, e_out) = (t.ease.in_half(), t.ease.out_half());
                match via {
                    "flip" => {
                        // `flip: pg` turns only that member over (the icon)
                        // and crossfades everything else (the caption), so
                        // text is never squashed to a sliver.
                        let member = opt_name(&s.opts, "flip").map(|m| m.replace('.', "_"));
                        let parts = member.as_ref().map(|m| (format!("{}_{}", target, m), format!("{}_{}", partner, m)));
                        match parts.filter(|(a, b)| self.elems.contains_key(a) && self.elems.contains_key(b)) {
                            Some((a_icon, b_icon)) => {
                                self.tween(fm, &op_key, simple(start + t.dur, 0.0, Ease::Linear, Val::n(0.0)));
                                self.tween(fm, &pk, simple(start, 0.0, Ease::Linear, Val::n(1.0)));
                                let (aw, bw) = (self.wrap(&a_icon), self.wrap(&b_icon));
                                self.overlay(fm, aw.clone(), Prop::Scale, Overlay { start, dur: half, ease: e_in, keys: vec![Val::xy(1.0, 1.0), Val::xy(0.0, 1.0)], mode: Mode::Mul, looping: false, atom });
                                // ...and stays turned away until the swap ends.
                                self.overlay(fm, aw, Prop::Scale, Overlay { start: start + half, dur: half, ease: Ease::Linear, keys: vec![Val::xy(0.0, 1.0), Val::xy(0.0, 1.0)], mode: Mode::Mul, looping: false, atom });
                                self.overlay(fm, bw.clone(), Prop::Scale, Overlay { start, dur: half, ease: Ease::Linear, keys: vec![Val::xy(0.0, 1.0), Val::xy(0.0, 1.0)], mode: Mode::Mul, looping: false, atom });
                                self.overlay(fm, bw, Prop::Scale, Overlay { start: start + half, dur: half, ease: e_out, keys: vec![Val::xy(0.0, 1.0), Val::xy(1.0, 1.0)], mode: Mode::Mul, looping: false, atom });
                                // The rest of each side crossfades over the whole turn.
                                let kids = |of: &str, skip: &str| -> Vec<String> {
                                    self.elems
                                        .iter()
                                        .filter(|(id, e)| e.parent.as_deref() == Some(of) && id.as_str() != skip)
                                        .map(|(id, _)| id.clone())
                                        .collect()
                                };
                                let (a_kids, b_kids) = (kids(target, &a_icon), kids(&partner, &b_icon));
                                for k in a_kids {
                                    let w = self.wrap(&k);
                                    self.overlay(fm, w, Prop::Opacity, Overlay { start, dur: t.dur, ease: t.ease, keys: vec![Val::n(1.0), Val::n(0.0)], mode: Mode::Mul, looping: false, atom });
                                }
                                for k in b_kids {
                                    let w = self.wrap(&k);
                                    self.overlay(fm, w, Prop::Opacity, Overlay { start, dur: t.dur, ease: t.ease, keys: vec![Val::n(0.0), Val::n(1.0)], mode: Mode::Mul, looping: false, atom });
                                }
                            }
                            None => {
                                self.tween(fm, &op_key, simple(start + half, 0.0, Ease::Linear, Val::n(0.0)));
                                self.tween(fm, &pk, simple(start + half, 0.0, Ease::Linear, Val::n(1.0)));
                                self.overlay(fm, wrap.clone(), Prop::Scale, Overlay { start, dur: half, ease: e_in, keys: vec![Val::xy(1.0, 1.0), Val::xy(0.0, 1.0)], mode: Mode::Mul, looping: false, atom });
                                self.overlay(fm, pw.clone(), Prop::Scale, Overlay { start: start + half, dur: half, ease: e_out, keys: vec![Val::xy(0.0, 1.0), Val::xy(1.0, 1.0)], mode: Mode::Mul, looping: false, atom });
                            }
                        }
                    }
                    "fade" => {
                        self.tween(fm, &op_key, simple(start, t.dur, t.ease, Val::n(0.0)));
                        self.tween(fm, &pk, simple(start, t.dur, t.ease, Val::n(1.0)));
                    }
                    "morph" => {
                        self.tween(fm, &op_key, simple(start, half, t.ease, Val::n(0.0)));
                        self.tween(fm, &pk, simple(start, half, t.ease, Val::n(1.0)));
                        if let (Some(a), Some(b)) = (self.center_now(layout, target), self.center_now(layout, &partner)) {
                            let ptr = ChKey { sel: pw.clone(), prop: Prop::Translate };
                            let settled = match snap.vals.get(&ptr) { Some(Val::V(v)) => (v[0], v[1]), _ => (0.0, 0.0) };
                            self.tween(fm, &ptr, Tween { start, dur: t.dur, ease: t.ease, from: Some(Val::xy(settled.0 + a.x - b.x, settled.1 + a.y - b.y)), to: Val::xy(settled.0, settled.1), atom });
                            handled.insert(ptr);
                        }
                    }
                    other => {
                        return Err(CompileError {
                            message: format!("unknown swap via '{}': use flip, fade or morph", other),
                            span: fm.atoms[atom].span.clone(),
                        })
                    }
                }
                handled.insert(op_key.clone());
                handled.insert(pk);
            }
            MotionVerb::Fly { subject, from, .. } => {
                let dest = s.partners.get(i).cloned().unwrap_or_default();
                self.fly(subject, from.as_ref(), &dest, target, start, t, atom, snap, layout, fm)?;
            }
            MotionVerb::Effect { name, .. } => {
                self.effect(&name.node, &s.opts, target, start, t, atom, false, fm)?;
            }
            MotionVerb::Loop { effect, .. } => {
                self.effect(&effect.node, &s.opts, target, start, t, atom, true, fm)?;
            }
            MotionVerb::Count(_) => {
                self.count(s, target, start, t, atom, before, snap, fm, &mut handled);
            }
            _ => {}
        }

        // Everything else that changed follows the statement's timing.
        for k in changed {
            if handled.contains(k) {
                continue;
            }
            let to = snap.vals.get(k).cloned().unwrap_or_else(|| match k.prop {
                Prop::Opacity => Val::n(0.0),
                _ => Val::S(String::new()),
            });
            // Hidden things need no travel: land them before they show.
            self.tween(fm, k, simple(start, t.dur, t.ease, to));
        }
        Ok(())
    }

    fn is_descendant(&self, id: &str, ancestor: &str) -> bool {
        let mut cur = self.elems.get(id).and_then(|e| e.parent.clone());
        while let Some(p) = cur {
            if p == ancestor {
                return true;
            }
            cur = self.elems.get(&p).and_then(|e| e.parent.clone());
        }
        false
    }

    fn center_before(&self, before: &Snap, id: &str) -> Option<Point> {
        let base = self.base_center(id)?;
        let own = match before.vals.get(&ChKey { sel: self.wrap(id), prop: Prop::Translate }) {
            Some(Val::V(v)) => (v[0], v[1]),
            _ => (0.0, 0.0),
        };
        let anc = self.ancestors_translate(before, id);
        Some(Point::new(base.x + own.0 + anc.0, base.y + own.1 + anc.1))
    }

    #[allow(clippy::too_many_arguments)]
    fn fly(
        &mut self,
        subject: &FlySubject,
        from: Option<&Spanned<String>>,
        to_id: &str,
        target: &str,
        start: f64,
        t: &Timing,
        atom: usize,
        snap: &Snap,
        layout: &LayoutResult,
        fm: &mut FrameMotion,
    ) -> Result<(), CompileError> {
        let span = fm.atoms[atom].span.clone();
        let err = |m: String| CompileError { message: m, span: span.clone() };
        let dst_el = layout
            .get_element_by_name(to_id)
            .ok_or_else(|| err(format!("fly ... to {}: not an element", to_id)))?;
        let dst = dst_el.bounds.center();
        let me = layout
            .get_element_by_name(target)
            .ok_or_else(|| err(format!("fly: '{}' is not an element", target)))?;
        let base_c = self.base_center(target).unwrap_or(me.bounds.center());
        let fit = {
            let (w, h) = (me.bounds.width.max(1.0), me.bounds.height.max(1.0));
            (dst_el.bounds.width / w).min(dst_el.bounds.height / h)
        };
        let (hook, src, anc) = match subject {
            FlySubject::Ghost(_) => {
                let suffix = format!("{}", self.aux.ghosts.len());
                // A snapshot of what shows now: parts still hidden stay behind.
                let hidden: Vec<String> = self
                    .elems
                    .keys()
                    .filter(|id| *id != target && self.is_descendant(id, target))
                    .filter(|id| {
                        // Hidden itself, or inside a hidden part of the source.
                        let hid = |x: &str| {
                            matches!(snap.vals.get(&ChKey { sel: self.wrap(x), prop: Prop::Opacity }), Some(v) if v.close(&Val::n(0.0)))
                        };
                        let mut cur = Some(id.to_string());
                        while let Some(x) = cur.filter(|x| x != target) {
                            if hid(&x) {
                                return true;
                            }
                            cur = self.elems.get(&x).and_then(|e| e.parent.clone());
                        }
                        false
                    })
                    .cloned()
                    .collect();
                self.aux.ghosts.push((suffix.clone(), target.to_string(), hidden));
                let hook = format!(".aighost-{}{}", self.scope, suffix);
                // The ghost starts wherever its source is drawn right now.
                let src = self.center_before(snap, target).unwrap_or(base_c);
                (hook, src, (0.0, 0.0))
            }
            FlySubject::Proxy(_) => {
                let src = match from {
                    Some(f) => {
                        let id = f.node.replace('.', "_");
                        layout
                            .get_element_by_name(&id)
                            .map(|e| e.bounds.center())
                            .ok_or_else(|| err(format!("fly ... from {}: '{}' is not an element", f.node, f.node)))?
                    }
                    None => self.center_before(snap, target).unwrap_or(base_c),
                };
                (self.wrap(target), src, self.ancestors_translate(snap, target))
            }
        };
        let opts = self.current_opts.clone();
        // A snapshot (ghost) shrinks into its target; a traveller (a request
        // chip, a card) keeps its size unless told otherwise.
        let s_end = opt_number(&opts, "scale").unwrap_or(match subject {
            FlySubject::Ghost(_) => fit,
            FlySubject::Proxy(_) => 1.0,
        });
        let s_start = opt_number(&opts, "from_scale").unwrap_or(1.0);
        let arc = opt_number(&opts, "arc").unwrap_or(0.0);
        let (ax, ay) = (src.x - base_c.x - anc.0, src.y - base_c.y - anc.1);
        let (bx, by) = (dst.x - base_c.x - anc.0, dst.y - base_c.y - anc.1);
        let keys: Vec<Val> = if arc.abs() > 1e-9 {
            // Quadratic arc bowing to the left of travel (up, for left-to-right).
            let (mx, my) = ((ax + bx) / 2.0, (ay + by) / 2.0);
            let (dx, dy) = (bx - ax, by - ay);
            let len = (dx * dx + dy * dy).sqrt().max(1.0);
            let (nx, ny) = (dy / len, -dx / len);
            let (cx, cy) = (mx + nx * arc * len, my + ny * arc * len);
            (0..=16)
                .map(|i| {
                    let u = i as f64 / 16.0;
                    let v = 1.0 - u;
                    Val::xy(v * v * ax + 2.0 * v * u * cx + u * u * bx, v * v * ay + 2.0 * v * u * cy + u * u * by)
                })
                .collect()
        } else {
            vec![Val::xy(ax, ay), Val::xy(bx, by)]
        };
        let ov = |keys: Vec<Val>, ease: Ease| Overlay { start, dur: t.dur, ease, keys, mode: Mode::Abs, looping: false, atom };
        self.overlay(fm, hook.clone(), Prop::Translate, ov(keys, t.ease));
        self.overlay(fm, hook.clone(), Prop::Scale, ov(vec![Val::xy(s_start, s_start), Val::xy(s_end, s_end)], t.ease));
        // Visible for the whole trip, gone as it lands.
        let mut op: Vec<Val> = vec![Val::n(1.0); 9];
        op.push(Val::n(0.0));
        self.overlay(fm, hook, Prop::Opacity, ov(op, Ease::Linear));
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn effect(
        &mut self,
        name: &str,
        opts: &[Spanned<MotionOpt>],
        target: &str,
        start: f64,
        t: &Timing,
        atom: usize,
        looping: bool,
        fm: &mut FrameMotion,
    ) -> Result<(), CompileError> {
        let d = opt_number(opts, "distance").unwrap_or(self.input.tokens.distance);
        let dur = if looping { opt_number(opts, "period").unwrap_or(t.dur.max(1.2)) } else { t.dur };
        let wrap = self.wrap(target);
        let s = self.scope.clone();
        let ov = |keys: Vec<Val>, mode: Mode, ease: Ease| Overlay { start, dur, ease, keys, mode, looping, atom };
        let colour = match opt(opts, "color").map(|v| &v.node) {
            Some(MotionValue::Style(sv)) => crate::layout::types::ResolvedStyles::color_to_css(sv),
            Some(MotionValue::Name(n)) => Some(n.clone()),
            Some(MotionValue::Str(n)) => Some(n.clone()),
            _ => None,
        };
        match name {
            "pulse" => {
                let k = opt_number(opts, "scale").unwrap_or(1.15);
                self.overlay(fm, wrap, Prop::Scale, ov(vec![Val::xy(1.0, 1.0), Val::xy(k, k), Val::xy(1.0, 1.0)], Mode::Mul, t.ease));
            }
            "shake" => {
                let keys = [0.0, -1.0, 0.8, -0.5, 0.25, 0.0].iter().map(|f| Val::xy(f * d, 0.0)).collect();
                self.overlay(fm, wrap, Prop::Translate, ov(keys, Mode::Add, Ease::Linear));
            }
            "nudge" => {
                let (ux, uy) = match opt_name(opts, "direction").unwrap_or("right") {
                    "left" => (-1.0, 0.0),
                    "up" => (0.0, -1.0),
                    "down" => (0.0, 1.0),
                    _ => (1.0, 0.0),
                };
                self.overlay(fm, wrap, Prop::Translate, ov(vec![Val::xy(0.0, 0.0), Val::xy(ux * d, uy * d), Val::xy(0.0, 0.0)], Mode::Add, t.ease));
            }
            "flash" => {
                let kind = format!("flash:{}", colour.unwrap_or_else(|| "#ffffff".into()));
                self.aux.overlays.entry(target.to_string()).or_default().insert(kind);
                self.overlay(fm, format!(".aiflash-{}{}", s, target), Prop::Opacity, ov(vec![Val::n(0.0), Val::n(0.9), Val::n(0.0)], Mode::Abs, Ease::Linear));
            }
            "highlight" => {
                let kind = format!("highlight:{}", colour.unwrap_or_else(|| "var(--accent-1, #2346D8)".into()));
                self.aux.overlays.entry(target.to_string()).or_default().insert(kind);
                self.overlay(fm, format!(".aihl-{}{}", s, target), Prop::Opacity, ov(vec![Val::n(0.0), Val::n(0.3), Val::n(0.3), Val::n(0.0)], Mode::Abs, t.ease));
            }
            "ping" => {
                let kind = format!("ping:{}", colour.unwrap_or_else(|| "currentColor".into()));
                self.aux.overlays.entry(target.to_string()).or_default().insert(kind);
                let hook = format!(".aiping-{}{}", s, target);
                self.overlay(fm, hook.clone(), Prop::Opacity, ov(vec![Val::n(0.9), Val::n(0.0)], Mode::Abs, t.ease));
                let k = opt_number(opts, "scale").unwrap_or(1.8);
                self.overlay(fm, hook, Prop::Scale, ov(vec![Val::xy(1.0, 1.0), Val::xy(k, k)], Mode::Abs, t.ease));
            }
            "accent" => self.accent(opts, target, start, t, atom, fm)?,
            other => {
                return Err(CompileError {
                    message: format!("unknown effect '{}': use accent, pulse, shake, flash, ping, highlight or nudge", other),
                    span: fm.atoms[atom].span.clone(),
                })
            }
        }
        Ok(())
    }

    /// `accent x [tone: attention | error | ok, style: auto | underline |
    /// ring | outline | wiggle, hold: step]`: "look here". The style follows
    /// the element (a label or row is underlined, a small thing ringed, a
    /// panel outlined); the tone says why (a role colour; `error` adds a "!"
    /// badge). It leaves nothing behind and never moves anything; with
    /// `hold: step` it stays until the next click.
    fn accent(
        &mut self,
        opts: &[Spanned<MotionOpt>],
        target: &str,
        start: f64,
        t: &Timing,
        atom: usize,
        fm: &mut FrameMotion,
    ) -> Result<(), CompileError> {
        let span = fm.atoms[atom].span.clone();
        let err = |m: String| CompileError { message: m, span: span.clone() };
        let info = self.elems.get(target).ok_or_else(|| err(format!("accent: '{}' is not an element", target)))?;
        let (b, text_like) = (info.base, info.text_like);
        let tone = opt_name(opts, "tone").unwrap_or("attention");
        let colour = match tone {
            "attention" => "var(--role-warn, #E8A33D)",
            "error" => "var(--role-error, #D64545)",
            "ok" => "var(--role-ok, #2E9E5B)",
            other => return Err(err(format!("accent: tone '{}': use attention, error or ok", other))),
        };
        let style = match opt_name(opts, "style").unwrap_or("auto") {
            "auto" => {
                if text_like && b.height <= 60.0 && b.width >= 2.5 * b.height {
                    "underline"
                } else if b.width.max(b.height) <= 80.0 {
                    "ring"
                } else {
                    "outline"
                }
            }
            s @ ("underline" | "ring" | "outline" | "wiggle") => s,
            other => return Err(err(format!("accent: style '{}': use auto, underline, ring, outline or wiggle", other))),
        };
        let hold = opt_name(opts, "hold").is_some_and(|h| h == "step");
        // Long enough to be seen: the default effect time is a blink.
        let dur = t.dur;
        let entr = (dur * 0.3).min(0.35);
        let s = self.scope.clone();
        let mut nodes: Vec<(&str, &str)> = Vec::new();
        match style {
            "wiggle" => {
                let wrap = self.wrap(target);
                let keys = [0.0, -5.0, 4.0, -3.0, 2.0, 0.0].iter().map(|a| Val::n(*a)).collect();
                self.overlay(fm, wrap, Prop::Rotate, Overlay { start, dur: dur.min(0.7), ease: Ease::Linear, keys, mode: Mode::Add, looping: false, atom });
            }
            other => nodes.push((other, "aiac")),
        }
        if tone == "error" {
            nodes.push(("badge", "aiac"));
        }
        for (what, prefix) in nodes {
            let node_colour = if what == "badge" { "var(--role-error, #D64545)" } else { colour };
            // One mark per element, style and tone: two accents on the same
            // thing in different tones each keep their own colour.
            self.aux.overlays.entry(target.to_string()).or_default().insert(format!("accent-{}-{}:{}", what, tone, node_colour));
            let hook = format!(".{}{}-{}{}-{}", prefix, &what[..1], s, target, tone);
            // Opacity: in, hold, out; or in and held while the step is shown.
            if hold {
                self.overlay(fm, hook.clone(), Prop::Opacity, Overlay { start, dur: entr, ease: Ease::Linear, keys: vec![Val::n(0.0), Val::n(1.0)], mode: Mode::Abs, looping: false, atom });
                self.overlay(fm, hook.clone(), Prop::Opacity, Overlay { start: start + entr, dur: 1.0, ease: Ease::Linear, keys: vec![Val::n(1.0), Val::n(1.0)], mode: Mode::Abs, looping: true, atom });
            } else {
                let keys = vec![Val::n(0.0), Val::n(1.0), Val::n(1.0), Val::n(1.0), Val::n(0.0)];
                self.overlay(fm, hook.clone(), Prop::Opacity, Overlay { start, dur, ease: Ease::Linear, keys, mode: Mode::Abs, looping: false, atom });
            }
            // How it arrives: an underline draws from the left, a ring and an
            // outline settle onto the element, a badge pops.
            let arrive: Vec<Val> = match what {
                "underline" => vec![Val::xy(0.0, 1.0), Val::xy(1.0, 1.0)],
                "ring" => vec![Val::xy(1.35, 1.35), Val::xy(1.0, 1.0)],
                "outline" => vec![Val::xy(1.06, 1.06), Val::xy(1.0, 1.0)],
                _ => vec![Val::xy(0.0, 0.0), Val::xy(1.2, 1.2), Val::xy(1.0, 1.0)],
            };
            self.overlay(fm, hook, Prop::Scale, Overlay { start, dur: entr, ease: t.ease, keys: arrive, mode: Mode::Abs, looping: false, atom });
        }
        Ok(())
    }

    /// The words an element shows in a snapshot (from its text variants).
    fn text_in(&self, snap: &Snap, id: &str) -> Option<String> {
        let base = self.elems.get(id).and_then(|e| e.base_text.clone());
        if let Some(variants) = self.input.text_variants.get(id) {
            for (n, t) in variants.iter().enumerate() {
                let k = ChKey { sel: format!(".aitxt-{}{}-v{}", self.scope, id, n), prop: Prop::Opacity };
                if snap.vals.get(&k).is_some_and(|v| v.close(&Val::n(1.0))) {
                    return Some(t.clone());
                }
            }
        }
        base
    }

    #[allow(clippy::too_many_arguments)]
    fn count(
        &mut self,
        s: &MotionStmt,
        target: &str,
        start: f64,
        t: &Timing,
        atom: usize,
        before: &Snap,
        snap: &Snap,
        fm: &mut FrameMotion,
        handled: &mut BTreeSet<ChKey>,
    ) {
        let Some(to) = opt_number(&s.opts, "to") else { return };
        let fmt = match opt(&s.opts, "format").map(|v| &v.node) {
            Some(MotionValue::Str(f)) => f.clone(),
            _ => "{}".to_string(),
        };
        let from = opt_number(&s.opts, "from")
            .or_else(|| self.text_in(before, target).as_deref().and_then(super::parse_leading_number))
            .unwrap_or(0.0);
        let decimals = fmt.contains(".1") || fmt.contains(".2");
        self.aux.tickers.insert(target.to_string());
        let hook = format!(".aitick-{}{}", self.scope, target);
        // Digits step in time with the ease: sample the eased value at even
        // times, so the browser (which only swaps text) and the native sampler
        // agree without either knowing about easing.
        let steps = ((t.dur * 30.0).ceil() as usize).clamp(2, 90);
        let keys: Vec<Val> = (0..=steps)
            .map(|i| {
                let p = t.ease.eval(i as f64 / steps as f64);
                let v = from + (to - from) * p;
                let v = if decimals { v } else { v.round() };
                Val::S(super::format_count(v, &fmt))
            })
            .collect();
        self.overlay(fm, hook.clone(), Prop::Text, Overlay { start, dur: t.dur, ease: Ease::Linear, keys, mode: Mode::Step, looping: false, atom });
        self.overlay(fm, hook, Prop::Opacity, Overlay { start, dur: t.dur, ease: Ease::Linear, keys: vec![Val::n(1.0), Val::n(1.0)], mode: Mode::Abs, looping: false, atom });
        // The settled wordings: the old one leaves as the ticker takes over,
        // the new one arrives as it finishes.
        let prefix = format!(".aitxt-{}{}", self.scope, target);
        let keys: Vec<ChKey> = snap
            .vals
            .keys()
            .filter(|k| k.sel.starts_with(&prefix) && k.prop == Prop::Opacity)
            .cloned()
            .collect();
        for k in keys {
            let was = before.vals.get(&k).cloned();
            let now = snap.vals.get(&k).cloned().unwrap_or(Val::n(0.0));
            if was.as_ref().is_some_and(|w| w.close(&now)) {
                continue;
            }
            let on = now.close(&Val::n(1.0));
            let at = if on { start + t.dur } else { start };
            self.tween(fm, &k, Tween { start: at, dur: 0.0, ease: Ease::Linear, from: None, to: now, atom });
            handled.insert(k);
        }
    }
}

fn became_visible_conn(changed: &[ChKey], before: &Snap, scope: &str, id: &str) -> bool {
    let k = ChKey { sel: format!(".conn-{}{}", scope, id), prop: Prop::Opacity };
    changed.contains(&k) && before.vals.get(&k).is_some_and(|v| v.close(&Val::n(0.0)))
}

fn wipe_dir(v: Option<&MotionValue>) -> String {
    match v {
        Some(MotionValue::Call(_, args)) => match args.first().map(|a| &a.node) {
            Some(MotionValue::Name(d)) => d.clone(),
            _ => "right".into(),
        },
        _ => "right".into(),
    }
}

fn opposite(d: &str) -> String {
    match d {
        "left" => "right",
        "right" => "left",
        "up" => "down",
        _ => "up",
    }
    .to_string()
}

/// Inset that hides everything, for a wipe that *reveals toward* `dir`.
fn wipe_hidden(dir: &str) -> Val {
    // inset(top right bottom left)
    match dir {
        "left" => Val::V(vec![0.0, 0.0, 0.0, 100.0]),
        "up" => Val::V(vec![100.0, 0.0, 0.0, 0.0]),
        "down" => Val::V(vec![0.0, 0.0, 100.0, 0.0]),
        _ => Val::V(vec![0.0, 100.0, 0.0, 0.0]),
    }
}

/// `show a [enter: pop]` for the timeline.
fn describe_verb(s: &MotionStmt) -> String {
    let mut v = verb_name(&s.verb).to_string();
    if let Some(l) = opt_name(&s.opts, "layout") {
        return format!("use layout {}", l);
    }
    match &s.verb {
        MotionVerb::Effect { name, .. } => v = name.node.clone(),
        MotionVerb::Loop { effect, .. } => v = format!("loop {}", effect.node),
        _ => {}
    }
    let extras: Vec<String> = s
        .opts
        .iter()
        .filter(|o| matches!(o.node.key.node.as_str(), "enter" | "exit" | "swap" | "via" | "to" | "from"))
        .map(|o| format!("{}: {}", o.node.key.node, value_text(&o.node.value.node)))
        .collect();
    if extras.is_empty() {
        v
    } else {
        format!("{} [{}]", v, extras.join(", "))
    }
}

pub fn value_text(v: &MotionValue) -> String {
    match v {
        MotionValue::Number(n) => format!("{}", n),
        MotionValue::Percent(p) => format!("{}%", p * 100.0),
        MotionValue::Name(n) => n.clone(),
        MotionValue::Str(s) => format!("\"{}\"", s),
        MotionValue::Call(f, a) => format!(
            "{}({})",
            f,
            a.iter().map(|x| value_text(&x.node)).collect::<Vec<_>>().join(", ")
        ),
        MotionValue::Vertex(n) => format!("vertex {}", n),
        MotionValue::Style(_) => "…".into(),
    }
}

/// The keyframes (in order), for callers that only have a document.
pub fn keyframes(doc: &Document) -> Vec<&KeyframeDecl> {
    crate::layout::keyframe::extract_keyframes(doc)
}
