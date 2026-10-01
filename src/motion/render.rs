//! Output of compiled motion: renderer hooks, the embedded track manifest,
//! settled-state CSS, native sampling (`--at`), and the `--timeline` table.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::layout::types::LayoutResult;

use super::compile::{ChKey, Curve, Mode, Motion, Prop, Val};
use super::tokens::MotionTokens;

/// What the SVG renderer needs to give motion something to move.
#[derive(Debug, Clone, Default)]
pub struct MotionHooks {
    pub origins: HashMap<String, (f64, f64)>,
    pub drawables: BTreeMap<String, bool>,
    pub heads: BTreeSet<String>,
    pub head_d: HashMap<String, String>,
    initial: HashMap<ChKey, Val>,
    pub camera: bool,
    pub scope: String,
    pub conn_variants: std::collections::HashSet<(String, String)>,
    /// Per element: overlay nodes ("flash:#fff", "ping:colour", "highlight:colour").
    pub overlays: BTreeMap<String, BTreeSet<String>>,
    /// Ghost copies: (hook suffix, source element).
    pub ghosts: Vec<(String, String, Vec<String>)>,
    /// Text elements that get a counter ticker node.
    pub tickers: BTreeSet<String>,
}

impl MotionHooks {
    pub fn from_motion(m: &Motion, base: &LayoutResult) -> Self {
        let initial = m
            .channels
            .iter()
            .enumerate()
            .map(|(i, k)| (k.clone(), m.settled[1.min(m.settled.len() - 1)][i].clone()))
            .collect();
        MotionHooks {
            origins: m.aux.origins.clone(),
            drawables: m.aux.drawables.clone(),
            heads: m.aux.heads.clone(),
            head_d: m
                .aux
                .heads
                .iter()
                .filter_map(|h| super::compile::head_d(base, h).map(|d| (h.clone(), d)))
                .collect(),
            initial,
            camera: m.aux.camera,
            scope: m.scope.clone(),
            conn_variants: m.aux.conn_variants.clone(),
            overlays: m.aux.overlays.clone(),
            ghosts: m.aux.ghosts.clone(),
            tickers: m.aux.tickers.clone(),
        }
    }

    fn initial_num(&self, sel: String, prop: Prop, default: f64) -> f64 {
        match self.initial.get(&ChKey { sel, prop }) {
            Some(Val::V(v)) => v[0],
            _ => default,
        }
    }

    /// Frame-0 dash offset of a drawable's mask (so the no-JS view is frame 0).
    pub fn initial_offset(&self, id: &str) -> f64 {
        self.initial_num(format!(".aimask-{}{}", self.scope, id), Prop::DashOffset, 0.0)
    }

    pub fn initial_head(&self, id: &str) -> f64 {
        self.initial_num(format!(".aihead-{}{}", self.scope, id), Prop::Opacity, 1.0)
    }
}

/// Channels the legacy per-frame CSS does not cover; their settled states
/// are emitted as `.frame-<name>` rules too, so a host that flips frame
/// classes (the reveal filter) still lands every frame correctly.
fn legacy_uncovered(k: &ChKey) -> bool {
    matches!(k.prop, Prop::DashOffset | Prop::Scale | Prop::ClipPath)
        || k.sel.starts_with(".aihead-")
        || k.sel.starts_with(".aicam-")
        || k.sel.starts_with(".aimask-")
        || k.sel.starts_with(".aibase-")
        || k.sel.starts_with(".aivar-")
        || (k.prop == Prop::D && k.sel.starts_with(".aid-"))
}

/// Class-flip CSS for channels the legacy frame CSS does not know.
pub fn settled_css(m: &Motion, frame_names: &[String], prefix_scope: &str) -> String {
    let _ = prefix_scope;
    let mut css = String::from("/* motion: settled states (auto-generated) */\n");
    css.push_str("svg[data-ail-player] * { transition: none !important; }\n");
    // Without a player or a frame class (a no-JS host, print, an <img>) the
    // picture is frame 0 as it ends: where frame 0 moved things, how far it
    // drew lines. `.frame-*` rules and the player's inline styles override.
    if m.settled.len() > 1 {
        for (c, k) in m.channels.iter().enumerate() {
            if k.prop == Prop::Text {
                continue;
            }
            css.push_str(&format!("{} {{ {}: {}; }}\n", k.sel, k.prop.css(), k.prop.format(&m.settled[1][c])));
        }
    }
    for (i, name) in frame_names.iter().enumerate() {
        let mut rules = String::new();
        for (c, k) in m.channels.iter().enumerate() {
            if !legacy_uncovered(k) || k.prop == Prop::Text {
                continue;
            }
            rules.push_str(&format!(
                "  {} {{ {}: {}; }}\n",
                k.sel,
                k.prop.css(),
                k.prop.format(&m.settled[i + 1][c])
            ));
        }
        if !rules.is_empty() {
            css.push_str(&format!(".frame-{} {{\n{}}}\n", name, rules));
        }
    }
    css
}

/// The whole story as plain CSS: one looping timeline (each keyframe plays,
/// then holds; `[auto]` frames follow after their own pause), sampled with
/// the native sampler every 1/30 s and reduced per channel to the keys
/// linear interpolation needs. No script: it plays inside an `<img>` (a
/// README). Text rewrites (`count`) are not carried; swaps are.
pub fn film_css(m: &Motion, tokens: &MotionTokens, hold: f64, lead: f64, end_hold: f64) -> String {
    const DT: f64 = 1.0 / 30.0;
    // (global time, frame, local time)
    let mut times: Vec<(f64, usize, f64)> = Vec::new();
    let mut g = lead;
    if !m.frames.is_empty() {
        times.push((0.0, 0, 0.0));
    }
    for (fi, f) in m.frames.iter().enumerate() {
        let n = (f.duration / DT).ceil().max(1.0) as usize;
        for k in 0..=n {
            let t = (k as f64 * DT).min(f.duration);
            times.push((g + t, fi, t));
        }
        g += f.duration;
        let pause = match m.frames.get(fi + 1) {
            Some(next) => next.auto.unwrap_or(hold),
            None => end_hold,
        };
        g += pause;
        // Held at the frame's end until the next one starts.
        times.push((g - 0.001, fi, f.duration + 1e-6));
    }
    let total = g.max(0.1);
    let samples: Vec<Vec<Val>> = times.iter().map(|(_, f, t)| m.sample(*f, *t, tokens)).collect();
    let tol = |p: Prop| match p {
        Prop::Translate | Prop::Width | Prop::Height | Prop::StrokeWidth => 0.25,
        Prop::Rotate => 0.2,
        Prop::ClipPath => 0.2,
        _ => 0.004,
    };
    let mut css = String::from("/* motion: the whole story as CSS (auto-generated) */\n");
    // One `animation:` list per selector: a second rule would replace the
    // first (a pop animates opacity and scale on the same wrapper).
    let mut by_sel: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for (c, k) in m.channels.iter().enumerate() {
        if k.prop == Prop::Text {
            continue;
        }
        let seq: Vec<(f64, &Val)> = times.iter().zip(&samples).map(|((g, _, _), v)| (*g, &v[c])).collect();
        let keep = reduce(&seq, tol(k.prop));
        let name = format!("ailfilm{}", c);
        css.push_str(&format!("@keyframes {} {{", name));
        for i in keep {
            let (t, v) = seq[i];
            css.push_str(&format!(" {}% {{ {}: {}; }}", num(t / total * 100.0), k.prop.css(), k.prop.format(v)));
        }
        css.push_str(" }\n");
        by_sel.entry(k.sel.as_str()).or_default().push(format!("{} {}s linear infinite", name, num(total)));
    }
    // A label's text variants (`.aitxt-<id>-base`, `-v0`) are also `text`
    // children of the element's wrapper, which `.kf-<id> > text` (the label
    // colour) animates too: one element, two rules, and only one
    // `animation:` can win. Each variant gets both lists, and wins.
    let text_anims: BTreeMap<String, Vec<String>> = by_sel
        .iter()
        .filter_map(|(sel, a)| {
            let id = super::compile::label_sel_id(sel)?.strip_prefix(".kf-")?;
            Some((id.to_string(), a.clone()))
        })
        .collect();
    for (sel, anims) in &by_sel {
        let mut anims = anims.clone();
        let mut sel = sel.to_string();
        if let Some(rest) = sel.strip_prefix(".aitxt-") {
            if let Some((id, _variant)) = rest.rsplit_once('-') {
                if let Some(extra) = text_anims.get(id) {
                    anims.extend(extra.iter().cloned());
                    sel = format!("{}{}", sel, sel);
                }
            }
        }
        css.push_str(&format!("{} {{ animation: {}; }}\n", sel, anims.join(", ")));
    }
    css.push_str("@media (prefers-reduced-motion: reduce) { * { animation-play-state: paused !important; } }\n");
    css
}

/// The samples a channel needs: its ends, every change of a non-numeric
/// value, and the points where straight lines between kept samples would
/// stray more than `tol` from the samples between them.
fn reduce(seq: &[(f64, &Val)], tol: f64) -> Vec<usize> {
    let n = seq.len();
    if n <= 2 {
        return (0..n).collect();
    }
    let nums = |v: &Val| match v {
        Val::V(x) => Some(x.clone()),
        Val::S(_) => None,
    };
    let mut keep = vec![0];
    let mut a = 0;
    let mut j = 2;
    while j < n {
        // Can a straight line from a to j stand for everything between?
        let ok = match (nums(seq[a].1), nums(seq[j].1)) {
            (Some(va), Some(vj)) if va.len() == vj.len() => (a + 1..j).all(|i| match nums(seq[i].1) {
                Some(vi) if vi.len() == va.len() => {
                    let (ta, tj, ti) = (seq[a].0, seq[j].0, seq[i].0);
                    let u = if tj > ta { (ti - ta) / (tj - ta) } else { 0.0 };
                    va.iter().zip(&vj).zip(&vi).all(|((x, y), z)| (x + (y - x) * u - z).abs() <= tol)
                }
                _ => false,
            }),
            // Strings: only while nothing changes.
            _ => (a + 1..=j).all(|i| seq[i].1 == seq[a].1),
        };
        if ok {
            j += 1;
        } else {
            a = j - 1;
            keep.push(a);
            j = a + 2;
        }
    }
    if *keep.last().unwrap() != n - 1 {
        keep.push(n - 1);
    }
    keep
}

/// A static style block that pins every channel to its value at (frame, t).
pub fn sampled_css(m: &Motion, f: usize, t: f64, tokens: &MotionTokens) -> String {
    let vals = m.sample(f, t, tokens);
    let mut css = String::from("/* motion: sampled still (auto-generated) */\n* { transition: none !important; }\n");
    for (c, k) in m.channels.iter().enumerate() {
        if k.prop == Prop::Text {
            continue;
        }
        css.push_str(&format!(
            "{} {{ {}: {} !important; }}\n",
            k.sel,
            k.prop.css(),
            k.prop.format(&vals[c])
        ));
    }
    css
}

/// Text channels at (frame, t), applied by rewriting node text.
pub fn sampled_texts(m: &Motion, f: usize, t: f64, tokens: &MotionTokens) -> Vec<(String, String)> {
    let vals = m.sample(f, t, tokens);
    m.channels
        .iter()
        .enumerate()
        .filter(|(_, k)| k.prop == Prop::Text)
        .map(|(c, k)| {
            let s = match &vals[c] {
                Val::S(s) => s.clone(),
                Val::V(v) => format!("{}", v[0]),
            };
            (k.sel.clone(), s)
        })
        .collect()
}

fn js_str(s: &str) -> String {
    serde_like_escape(s)
}

fn serde_like_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '<' => out.push_str("\\u003c"),
            '>' => out.push_str("\\u003e"),
            '&' => out.push_str("\\u0026"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn num(x: f64) -> String {
    let y = (x * 10000.0).round() / 10000.0;
    format!("{}", y)
}

const SAMPLE_HZ: f64 = 60.0;

/// One Web Animations segment of a channel.
struct Anim {
    delay: f64,
    dur: f64,
    easing: String,
    keys: Vec<(f64, Val)>,
    looping: bool,
}

/// The element a channel selector belongs to (`.kfp-<scope>d0_pg` -> d0_pg).
fn channel_element<'a>(sel: &'a str, scope: &str) -> Option<&'a str> {
    let rest = sel.strip_prefix('.')?;
    let (_, after) = rest.split_once('-')?;
    let id = after.strip_prefix(scope)?;
    // Text variants: `.aitxt-<scope><id>-base`, `-v0`.
    Some(id.rsplit_once("-base").map(|(a, _)| a).or_else(|| {
        id.rsplit_once("-v").filter(|(_, n)| n.chars().all(|c| c.is_ascii_digit())).map(|(a, _)| a)
    }).unwrap_or(id))
}

/// Whether the channel's element, or an ancestor, is at opacity 0 at time t
/// of frame fi. An element's own opacity channel is its visibility, not
/// something that must hold while hidden.
fn hidden_at(m: &Motion, fi: usize, sel: &str, prop: Prop, t: f64, tokens: &MotionTokens) -> bool {
    let Some(id) = channel_element(sel, &m.scope) else { return false };
    let own_opacity = prop == Prop::Opacity && sel == format!(".kf-{}{}", m.scope, id);
    let mut cur = if own_opacity { m.aux.parents.get(id).cloned().flatten() } else { Some(id.to_string()) };
    while let Some(e) = cur {
        let wrap = format!(".kf-{}{}", m.scope, e);
        if let Some(c) = m.channels.iter().position(|k| k.sel == wrap && k.prop == Prop::Opacity) {
            let v = match m.frames[fi].curves.get(&c) {
                Some(curve) => curve.value(&m.settled[fi][c], t, tokens),
                None => m.settled[fi][c].clone(),
            };
            if let Val::V(x) = &v {
                if x.first().is_some_and(|o| *o <= 1e-6) {
                    return true;
                }
            }
        }
        cur = m.aux.parents.get(&e).cloned().flatten();
    }
    false
}

/// `hidden(t)`: the channel's element (or an ancestor) is invisible at t.
fn export_curve(curve: &Curve, init: &Val, tokens: &MotionTokens, hidden: &dyn Fn(f64) -> bool) -> Vec<Anim> {
    #[derive(Clone, Copy)]
    enum Seg {
        T(usize),
        O(usize),
    }
    let mut segs: Vec<(f64, f64, Seg)> = Vec::new();
    for (i, tw) in curve.tweens.iter().enumerate() {
        segs.push((tw.start, tw.start + tw.dur.max(0.0), Seg::T(i)));
    }
    for (i, ov) in curve.overlays.iter().enumerate() {
        if !ov.looping {
            segs.push((ov.start, ov.start + ov.dur.max(0.0), Seg::O(i)));
        }
    }
    segs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then(a.1.partial_cmp(&b.1).unwrap()));
    // Cluster strictly overlapping segments.
    let mut clusters: Vec<(f64, f64, Vec<Seg>)> = Vec::new();
    for (a, b, sg) in segs {
        match clusters.last_mut() {
            Some(c) if a < c.1 - 1e-9 => {
                c.1 = c.1.max(b);
                c.2.push(sg);
            }
            _ => clusters.push((a, b, vec![sg])),
        }
    }
    let mut out = Vec::new();
    for (a, b, members) in clusters {
        let dur = (b - a).max(0.001);
        if members.len() == 1 {
            match members[0] {
                Seg::T(i) => {
                    let tw = &curve.tweens[i];
                    let from = curve.value(init, a - 1e-4, tokens);
                    let from = tw.from.clone().unwrap_or(from);
                    out.push(Anim {
                        delay: a,
                        dur,
                        easing: tw.ease.css(),
                        keys: vec![(0.0, from), (1.0, tw.to.clone())],
                        looping: false,
                    });
                    continue;
                }
                Seg::O(i) => {
                    let ov = &curve.overlays[i];
                    // An overlay shapes the tweens' value, not what another
                    // overlay left behind: a flip's opening half follows its
                    // edge-on hold (scale 0), and combining with that turned
                    // the whole opening into 0 -> 0 in the player.
                    let tweens_only = Curve { tweens: curve.tweens.clone(), overlays: Vec::new() };
                    let base = tweens_only.value(init, a + 1e-6, tokens);
                    let n = ov.keys.len().max(2) - 1;
                    let keys = ov
                        .keys
                        .iter()
                        .enumerate()
                        .map(|(j, k)| (j as f64 / n as f64, combine_pub(&base, k, ov.mode)))
                        .collect::<Vec<_>>();
                    let mut keys = keys;
                    if keys.len() == 1 {
                        keys.push((1.0, keys[0].1.clone()));
                    }
                    out.push(Anim { delay: a, dur, easing: ov.ease.css(), keys, looping: false });
                    // After a transient the base value returns, unless the
                    // element is hidden by then: nothing about an element
                    // changes at the instant it disappears (a paint between
                    // the two showed the old flip page, full size, for one
                    // frame). The frame's end settles it atomically.
                    if hidden(b + 1e-6) {
                        continue;
                    }
                    let after = curve.value(init, b + 1e-6, tokens);
                    out.push(Anim {
                        delay: b,
                        dur: 0.001,
                        easing: "linear".into(),
                        keys: vec![(0.0, after.clone()), (1.0, after)],
                        looping: false,
                    });
                    continue;
                }
            }
        }
        // Overlapping: sample.
        let steps = ((dur * SAMPLE_HZ).ceil() as usize).max(2);
        // Once hidden for good (for the rest of this cluster), hold the last
        // visible value, as above. Something hidden that appears later must
        // keep tracking, so it is in place the moment it shows.
        let ts: Vec<f64> = (0..=steps).map(|j| a + dur * j as f64 / steps as f64).collect();
        let mut gone = vec![false; ts.len()];
        let mut all_hidden = true;
        for j in (0..ts.len()).rev() {
            all_hidden = all_hidden && hidden(ts[j]);
            gone[j] = all_hidden;
        }
        let mut last: Option<Val> = None;
        let keys = ts
            .iter()
            .enumerate()
            .map(|(j, t)| {
                let v = match (&last, gone[j]) {
                    (Some(prev), true) => prev.clone(),
                    _ => curve.value(init, *t, tokens),
                };
                last = Some(v.clone());
                (j as f64 / steps as f64, v)
            })
            .collect();
        out.push(Anim { delay: a, dur, easing: "linear".into(), keys, looping: false });
        // Nothing steps at the instant (or while) the element is hidden.
        if hidden(b + 1e-6) {
            continue;
        }
        let after = curve.value(init, b + 1e-6, tokens);
        out.push(Anim {
            delay: b,
            dur: 0.001,
            easing: "linear".into(),
            keys: vec![(0.0, after.clone()), (1.0, after)],
            looping: false,
        });
    }
    for ov in curve.overlays.iter().filter(|o| o.looping) {
        let base = curve.value(init, ov.start - 1e-9, tokens);
        let n = ov.keys.len().max(2) - 1;
        let keys = ov
            .keys
            .iter()
            .enumerate()
            .map(|(j, k)| (j as f64 / n as f64, combine_pub(&base, k, ov.mode)))
            .collect();
        out.push(Anim { delay: ov.start, dur: ov.dur.max(0.001), easing: ov.ease.css(), keys, looping: true });
    }
    out
}

fn combine_pub(base: &Val, key: &Val, mode: Mode) -> Val {
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

pub const MANIFEST_VERSION: u32 = 1;

/// The schema-versioned manifest + tracks, as JSON.
pub fn manifest_json(m: &Motion, tokens: &MotionTokens, elements: &[String]) -> String {
    let mut j = String::new();
    j.push_str(&format!("{{\"v\":{},\"scope\":{},", MANIFEST_VERSION, js_str(&m.scope)));
    j.push_str("\"frames\":[");
    for (i, f) in m.frames.iter().enumerate() {
        if i > 0 {
            j.push(',');
        }
        let opt = |s: &Option<String>| s.as_deref().map(js_str).unwrap_or_else(|| "null".into());
        j.push_str(&format!(
            "{{\"name\":{},\"duration\":{},\"auto\":{},\"title\":{},\"note\":{}}}",
            js_str(&f.name),
            num(f.duration),
            f.auto.map(num).unwrap_or_else(|| "null".into()),
            opt(&f.title),
            opt(&f.note)
        ));
    }
    j.push_str("],\"elements\":[");
    j.push_str(&elements.iter().map(|e| js_str(e)).collect::<Vec<_>>().join(","));
    j.push_str("],\"channels\":[");
    for (i, k) in m.channels.iter().enumerate() {
        if i > 0 {
            j.push(',');
        }
        j.push_str(&format!("[{},{}]", js_str(&k.sel), js_str(k.prop.css())));
    }
    j.push_str("],\"settled\":[");
    for (fi, row) in m.settled.iter().enumerate() {
        if fi > 0 {
            j.push(',');
        }
        j.push('[');
        j.push_str(
            &row.iter()
                .zip(&m.channels)
                .map(|(v, k)| js_str(&k.prop.format(v)))
                .collect::<Vec<_>>()
                .join(","),
        );
        j.push(']');
    }
    j.push_str("],\"anims\":[");
    for (fi, f) in m.frames.iter().enumerate() {
        if fi > 0 {
            j.push(',');
        }
        j.push('[');
        let mut first = true;
        for (c, curve) in &f.curves {
            let k = &m.channels[*c];
            let hidden = |t: f64| hidden_at(m, fi, &k.sel, k.prop, t, tokens);
            for a in export_curve(curve, &m.settled[fi][*c], tokens, &hidden) {
                if !first {
                    j.push(',');
                }
                first = false;
                let keys = a
                    .keys
                    .iter()
                    .map(|(o, v)| format!("[{},{}]", num(*o), js_str(&k.prop.format(v))))
                    .collect::<Vec<_>>()
                    .join(",");
                j.push_str(&format!(
                    "{{\"c\":{},\"t\":{},\"d\":{},\"e\":{},\"k\":[{}]{}}}",
                    c,
                    num(a.delay),
                    num(a.dur),
                    js_str(&a.easing),
                    keys,
                    if a.looping { ",\"loop\":true" } else { "" }
                ));
            }
        }
        j.push(']');
    }
    j.push_str("]}");
    j
}

/// The dependency-free player (Web Animations API).
pub const PLAYER_JS: &str = include_str!("player.js");

/// `<script>` blocks embedding the manifest (always) and the player
/// (with `autoplay`, a self-running standalone SVG).
pub fn embed(manifest: &str, player: bool, autoplay: bool) -> String {
    let mut s = format!(
        "<script type=\"application/json\" class=\"ail-motion\"><![CDATA[{}]]></script>\n",
        manifest
    );
    if player {
        s.push_str("<script><![CDATA[\n");
        s.push_str(PLAYER_JS);
        if autoplay {
            s.push_str("\n(function(){var svg=document.currentScript&&document.currentScript.closest?document.currentScript.closest('svg'):null;if(!svg){var all=document.querySelectorAll('svg[data-frames]');svg=all[all.length-1];}if(svg&&window.ail){window.ail.player(svg,{autoplay:true});}})();\n");
        }
        s.push_str("]]></script>\n");
    }
    s
}

/// A channel selector as the author would say it: `box`, `box (shape)`, ...
fn pretty_sel(sel: &str, scope: &str, display: &HashMap<String, String>) -> String {
    let shown = pretty_sel_raw(sel, scope);
    // The id is the first word; write it the way the author does.
    let (id, rest) = match shown.find([' ', '-']) {
        Some(i) => (&shown[..i], &shown[i..]),
        None => (shown.as_str(), ""),
    };
    match display.get(id) {
        Some(d) => format!("{}{}", d, rest),
        None => shown.clone(),
    }
}

fn pretty_sel_raw(sel: &str, scope: &str) -> String {
    let strip = |p: &str| sel.strip_prefix(&format!(".{}{}", p, scope)).map(str::to_string);
    if let Some(w) = super::compile::label_sel_id(sel) {
        if let Some(id) = w.strip_prefix(&format!(".kf-{}", scope)) {
            return format!("{} (label)", id);
        }
    }
    if let Some(rest) = strip("kf-") {
        return rest;
    }
    for (p, what) in [
        ("kfp-", "shape"),
        ("aimask-", "drawn"),
        ("aihead-", "arrowhead"),
        ("aibase-", "route"),
        ("aid-", "path"),
        ("conn-", ""),
        ("aighost-", "ghost"),
        ("aiflash-", "flash"),
        ("aiping-", "ping"),
        ("aihl-", "highlight"),
        ("aitick-", "counter"),
        ("aitxt-", "text"),
        ("aivar-", "route variant"),
    ] {
        if let Some(rest) = strip(p) {
            return if what.is_empty() { rest } else { format!("{} ({})", rest, what) };
        }
    }
    if sel == format!(".aicam-{}", scope) {
        return "camera".into();
    }
    sel.to_string()
}

/// Human-readable choreography, one block per frame.
pub fn timeline_text(m: &Motion, tokens: &MotionTokens) -> String {
    let mut out = String::new();
    for (fi, f) in m.frames.iter().enumerate() {
        out.push_str(&format!(
            "frame {} \"{}\"  duration {:.2}s{}\n",
            fi,
            f.name,
            f.duration,
            match f.auto {
                Some(a) => format!("  [auto, after {:.2}s]", a),
                None => String::new(),
            }
        ));
        let mut rows: BTreeMap<usize, Vec<String>> = BTreeMap::new();
        for (c, curve) in &f.curves {
            let k = &m.channels[*c];
            let who = pretty_sel(&k.sel, &m.scope, &m.display);
            for tw in &curve.tweens {
                let from = curve.value(&m.settled[fi][*c], tw.start - 1e-4, tokens);
                let from = tw.from.clone().unwrap_or(from);
                let timing = if tw.start.abs() > 1e-9 || tw.dur.abs() > 1e-9 {
                    format!("  @{:.2}+{:.2}", tw.start, tw.dur)
                } else {
                    String::new()
                };
                rows.entry(tw.atom).or_default().push(format!(
                    "{} {}: {} -> {}{}",
                    who,
                    k.prop.css(),
                    short(&k.prop.format(&from)),
                    short(&k.prop.format(&tw.to)),
                    timing
                ));
            }
            for ov in &curve.overlays {
                rows.entry(ov.atom).or_default().push(format!(
                    "{} {}: {} {}  @{:.2}+{:.2}{}",
                    who,
                    k.prop.css(),
                    match ov.mode {
                        Mode::Abs => "",
                        Mode::Add => "+",
                        Mode::Mul => "x",
                        Mode::Step => "",
                    },
                    {
                        let all: Vec<String> = ov.keys.iter().map(|v| short(&k.prop.format(v))).collect();
                        if all.len() > 6 {
                            format!("{} | {} | ... ({} keys) ... | {}", all[0], all[1], all.len(), all[all.len() - 1])
                        } else {
                            all.join(" | ")
                        }
                    },
                    ov.start,
                    ov.dur,
                    if ov.looping { " (loop)" } else { "" }
                ));
            }
        }
        let mut quiet = 0;
        // Resolved events, each before the first statement of its block
        // (a beat's end, after its last).
        let mut shown_ev = vec![false; f.events.len()];
        for (ai, a) in f.atoms.iter().enumerate() {
            let _ = a;
            for (k, (t, what, first)) in f.events.iter().enumerate() {
                if !shown_ev[k] && *first == ai {
                    out.push_str(&format!("  {:>5.2}s  {}\n", t, what));
                    shown_ev[k] = true;
                }
            }
            let Some(r) = rows.get(&ai) else {
                quiet += 1;
                continue;
            };
            out.push_str(&format!(
                "  {:>5.2}s  {:>4.2}s  {:<7} {} {}\n",
                a.start, a.dur, a.ease_name, a.verb, a.target
            ));
            for line in r {
                out.push_str(&format!("                        {}\n", line));
            }
        }
        for (k, (t, what, _)) in f.events.iter().enumerate() {
            if !shown_ev[k] {
                out.push_str(&format!("  {:>5.2}s  {}\n", t, what));
            }
        }
        if quiet > 0 {
            out.push_str(&format!(
                "  ({} statement(s) change nothing here{})\n",
                quiet,
                if fi == 0 { ": frame 0 applies them before it starts" } else { "" }
            ));
        }
        out.push('\n');
    }
    out
}

fn short(s: &str) -> String {
    if s.chars().count() > 48 {
        let t: String = s.chars().take(45).collect();
        format!("{}...", t)
    } else {
        s.to_string()
    }
}

/// Static `--frame N`: cut each drawn connection where it is drawn to.
pub fn truncate_drawn(
    layout: &mut LayoutResult,
    state: &crate::layout::keyframe::FrameState,
    doc: &crate::parser::ast::Document,
) {
    use crate::layout::routing::RoutingMode;
    use crate::layout::types::Point;
    let (_, initial, _) = super::compile::drawable_info(doc, layout);
    let mut fracs: Vec<(String, f64)> = state
        .drawn
        .iter()
        .filter_map(|(id, to)| super::compile::resolve_draw_to(layout, id, to).map(|f| (id.clone(), f)))
        .collect();
    for (id, f) in initial {
        if !state.drawn.contains_key(&id) {
            fracs.push((id, f));
        }
    }
    for (id, frac) in fracs {
        if frac >= 0.999 {
            continue;
        }
        let Some(idx) = layout.connections.iter().position(|c| c.name.as_ref().is_some_and(|n| n.0 == id)) else {
            truncate_path_shape(layout, &id, frac);
            continue;
        };
        if frac <= 0.0005 {
            layout.connections.remove(idx);
            continue;
        }
        let rendered = super::compile::drawable_polyline(layout, &id).map(|p| p.length()).unwrap_or(0.0);
        let c = &mut layout.connections[idx];
        if matches!(c.routing_mode, RoutingMode::Curved) {
            continue;
        }
        let keep = frac * rendered;
        let mut acc = 0.0;
        let mut pts: Vec<Point> = vec![c.path[0]];
        for w in c.path.windows(2) {
            let seg = ((w[1].x - w[0].x).powi(2) + (w[1].y - w[0].y).powi(2)).sqrt();
            if acc + seg >= keep {
                let t = if seg > 0.0 { (keep - acc) / seg } else { 0.0 };
                pts.push(Point::new(w[0].x + (w[1].x - w[0].x) * t, w[0].y + (w[1].y - w[0].y) * t));
                break;
            }
            acc += seg;
            pts.push(w[1]);
        }
        c.path = pts;
        // No arrowhead until the tip arrives.
        c.direction = crate::parser::ast::ConnectionDirection::Undirected;
        c.label = None;
    }
}

/// Cut a path shape (a line routed through stations, say) at `frac` of its
/// length, as a polyline of its flattened geometry.
fn truncate_path_shape(layout: &mut LayoutResult, id: &str, frac: f64) {
    use crate::layout::types::{ElementLayout, ElementType, Point};
    use crate::parser::ast::{Identifier, LineToDecl, PathBody, PathCommand, ShapeType, Spanned, VertexDecl, VertexPosition};
    let Some(pl) = super::compile::drawable_polyline(layout, id) else { return };
    let pts: Vec<Point> = if frac <= 0.0005 {
        vec![]
    } else {
        let keep = frac * pl.length();
        let mut out = vec![pl.pts[0]];
        for i in 1..pl.pts.len() {
            if pl.cum[i] >= keep {
                let seg = pl.cum[i] - pl.cum[i - 1];
                let t = if seg > 0.0 { (keep - pl.cum[i - 1]) / seg } else { 0.0 };
                let (a, b) = (pl.pts[i - 1], pl.pts[i]);
                out.push(Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t));
                break;
            }
            out.push(pl.pts[i]);
        }
        out
    };
    fn visit(elems: &mut [ElementLayout], id: &str, pts: &[Point]) -> bool {
        for e in elems {
            if e.id.as_ref().is_some_and(|i| i.0 == id) {
                if let ElementType::Shape(ShapeType::Path(decl)) = &mut e.element_type {
                    let origin = Point::new(e.bounds.x, e.bounds.y);
                    let pos = |p: &Point| VertexPosition { x: Some(p.x - origin.x), y: Some(p.y - origin.y) };
                    let name = |i: usize| Spanned::new(Identifier::new(format!("t{}", i)), 0..0);
                    let mut commands = Vec::new();
                    for (i, p) in pts.iter().enumerate() {
                        let cmd = if i == 0 {
                            PathCommand::Vertex(VertexDecl { name: name(i), position: Some(pos(p)) })
                        } else {
                            PathCommand::LineTo(LineToDecl { target: name(i), position: Some(pos(p)) })
                        };
                        commands.push(Spanned::new(cmd, 0..0));
                    }
                    decl.body = PathBody { commands };
                    // Coordinates are relative to the box, not re-normalised.
                    e.path_normalize = false;
                }
                return true;
            }
            if visit(&mut e.children, id, pts) {
                return true;
            }
        }
        false
    }
    visit(&mut layout.root_elements, id, &pts);
}
