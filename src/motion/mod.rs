//! First-class motion: timed keyframe statements compiled to explicit tracks.
//!
//! Pipeline: the parser builds a motion tree per keyframe; [`expand`] resolves
//! macros and selectors and derives each keyframe's flat state operations;
//! after layout, [`compile`] replays those operations one statement at a time,
//! attributes every state change to the statement that caused it, and emits
//! per-channel tracks. The browser player and the native `--at` sampler both
//! consume the same tracks.

pub mod compile;
pub mod ease;
pub mod expand;
pub mod geom;
pub mod lint;
pub mod render;
pub mod storyboard;
pub mod tokens;

use crate::parser::ast::{
    DrawTo, Identifier, KeyframeOp, MotionNode, MotionOpt, MotionValue, MotionVerb, PinTo,
    Selector, Spanned, StyleKey, StyleModifier, StyleValue,
};

/// Internal option: the `caption:` part of each target of a show/hide
/// ("" for none), paired by position.
pub const CAPTIONS_KEY: &str = "__captions";

/// Option keys that time a statement rather than change state.
pub const TIMING_KEYS: &[&str] = &[
    "delay", "duration", "ease", "stagger", "order", "from", "swap", "enter", "exit", "jitter",
];

pub fn is_timing_key(k: &str) -> bool {
    TIMING_KEYS.contains(&k)
}

/// The transient one-shot effects.
pub const EFFECTS: &[&str] = &["pulse", "shake", "flash", "ping", "highlight", "nudge", "accent", "mark", "unmark"];

/// `mark x [style: s, tone: t]` as state: "<style>-<tone>".
pub fn mark_kind(opts: &[Spanned<MotionOpt>]) -> String {
    format!("{}-{}", opt_name(opts, "style").unwrap_or("auto"), opt_name(opts, "tone").unwrap_or("attention"))
}

/// Find an option by key.
pub fn opt<'a>(opts: &'a [Spanned<MotionOpt>], key: &str) -> Option<&'a Spanned<MotionValue>> {
    opts.iter().rev().find(|o| o.node.key.node == key).map(|o| &o.node.value)
}

pub fn opt_number(opts: &[Spanned<MotionOpt>], key: &str) -> Option<f64> {
    match opt(opts, key).map(|v| &v.node) {
        Some(MotionValue::Number(n)) => Some(*n),
        Some(MotionValue::Percent(p)) => Some(*p),
        _ => None,
    }
}

pub fn opt_name<'a>(opts: &'a [Spanned<MotionOpt>], key: &str) -> Option<&'a str> {
    match opt(opts, key).map(|v| &v.node) {
        Some(MotionValue::Name(n)) => Some(n.as_str()),
        Some(MotionValue::Call(n, _)) => Some(n.as_str()),
        _ => None,
    }
}

/// Parse a `to:` style value into a draw target.
pub fn draw_to(v: &MotionValue) -> Option<DrawTo> {
    match v {
        MotionValue::Percent(p) => Some(DrawTo::Fraction(p.clamp(0.0, 1.0))),
        MotionValue::Vertex(n) => Some(DrawTo::Vertex(*n)),
        MotionValue::Name(n) => Some(DrawTo::Element(n.replace('.', "_"))),
        MotionValue::Number(n) if (0.0..=1.0).contains(n) => Some(DrawTo::Fraction(*n)),
        _ => None,
    }
}

/// Format a counter value: `{}`, `{:,}` (thousands), `{:.1}`, `{:,.2}`.
pub fn format_count(value: f64, fmt: &str) -> String {
    let Some(open) = fmt.find('{') else {
        return fmt.to_string();
    };
    let Some(close_rel) = fmt[open..].find('}') else {
        return fmt.to_string();
    };
    let close = open + close_rel;
    let spec = &fmt[open + 1..close];
    let spec = spec.strip_prefix(':').unwrap_or(spec);
    let thousands = spec.contains(',');
    let decimals = spec
        .split('.')
        .nth(1)
        .and_then(|d| d.trim_end_matches(|c: char| !c.is_ascii_digit()).parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = format!("{:.*}", decimals, value);
    if thousands {
        let (int, frac) = match body.find('.') {
            Some(i) => (body[..i].to_string(), body[i..].to_string()),
            None => (body.clone(), String::new()),
        };
        let neg = int.starts_with('-');
        let digits: Vec<char> = int.trim_start_matches('-').chars().collect();
        let mut out = String::new();
        for (i, c) in digits.iter().enumerate() {
            if i > 0 && (digits.len() - i) % 3 == 0 {
                out.push(',');
            }
            out.push(*c);
        }
        body = format!("{}{}{}", if neg { "-" } else { "" }, out, frac);
    }
    format!("{}{}{}", &fmt[..open], body, &fmt[close + 1..])
}

/// The first number in a piece of text (a counter's starting value).
pub fn parse_leading_number(text: &str) -> Option<f64> {
    let mut num = String::new();
    let mut started = false;
    for c in text.chars() {
        if c.is_ascii_digit() || (started && c == '.') || (!started && c == '-') {
            num.push(c);
            started = started || c.is_ascii_digit();
        } else if started && c == ',' {
            continue;
        } else if started {
            break;
        } else {
            num.clear();
        }
    }
    num.parse().ok()
}

fn ident(name: &str, span: &crate::parser::ast::Span) -> Spanned<Identifier> {
    Spanned::new(Identifier::new(name), span.clone())
}

/// The state operations one statement causes, for its resolved targets
/// (`partners` is the right-hand side of a swap). Transient verbs (fly,
/// effects, loops) change no state and return nothing.
pub fn stmt_ops(
    verb: &MotionVerb,
    opts: &[Spanned<MotionOpt>],
    targets: &[Spanned<Identifier>],
    partners: &[Spanned<Identifier>],
) -> Vec<KeyframeOp> {
    let mut ops = base_stmt_ops(verb, opts, targets, partners);
    ops.extend(jitter_ops(opts, targets));
    // `caption:` parts come and go with their element.
    if let Some(MotionValue::List(items)) = opt(opts, CAPTIONS_KEY).map(|v| &v.node) {
        let ids: Vec<Spanned<Identifier>> = items
            .iter()
            .filter_map(|x| match &x.node {
                MotionValue::Name(n) if !n.is_empty() => Some(Spanned::new(Identifier::new(n.as_str()), x.span.clone())),
                _ => None,
            })
            .collect();
        if !ids.is_empty() {
            match verb {
                MotionVerb::Show(_) => ops.push(KeyframeOp::Show(ids)),
                MotionVerb::Hide(_) => ops.push(KeyframeOp::Hide(ids)),
                _ => {}
            }
        }
    }
    ops
}

/// A deterministic value in [-1, 1] for an element (and a salt).
pub fn seeded_unit(id: &str, salt: u64) -> f64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ salt;
    for b in id.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
    h ^= h >> 33;
    (h % 20001) as f64 / 10000.0 - 1.0
}

/// `jitter: 4` / `jitter: rotate(4)` / `jitter: move(6)`: a small, seeded,
/// lasting disorder per target, so a grid of copies looks like a mess rather
/// than a table. Seeded by the element's name: every render and every seek
/// agrees.
fn jitter_ops(opts: &[Spanned<MotionOpt>], targets: &[Spanned<Identifier>]) -> Vec<KeyframeOp> {
    let Some(v) = opt(opts, "jitter") else { return vec![] };
    let (kind, amount) = match &v.node {
        MotionValue::Number(n) => ("rotate", *n),
        MotionValue::Call(k, args) => match args.first().map(|a| &a.node) {
            Some(MotionValue::Number(n)) => (k.as_str(), *n),
            _ => return vec![],
        },
        _ => return vec![],
    };
    targets
        .iter()
        .map(|t| {
            let span = t.span.clone();
            let m = |key: StyleKey, value: f64| {
                Spanned::new(
                    StyleModifier {
                        key: Spanned::new(key, span.clone()),
                        value: Spanned::new(StyleValue::Number { value, unit: None }, span.clone()),
                    },
                    span.clone(),
                )
            };
            let modifiers = match kind {
                "move" => vec![
                    m(StyleKey::Dx, seeded_unit(&t.node.0, 1) * amount),
                    m(StyleKey::Dy, seeded_unit(&t.node.0, 2) * amount),
                ],
                _ => vec![m(StyleKey::Rotation, seeded_unit(&t.node.0, 0) * amount)],
            };
            KeyframeOp::Transform { target: t.clone(), modifiers }
        })
        .collect()
}

fn base_stmt_ops(
    verb: &MotionVerb,
    opts: &[Spanned<MotionOpt>],
    targets: &[Spanned<Identifier>],
    partners: &[Spanned<Identifier>],
) -> Vec<KeyframeOp> {
    let all = || targets.to_vec();
    // `show x [enter: expand]` / `hide x [exit: collapse]`: the element also
    // opens to its laid-out height or closes to nothing, and what is stacked
    // below it (a code line under a code line) moves with it.
    let height = |value: StyleValue| -> Vec<KeyframeOp> {
        targets
            .iter()
            .map(|t| {
                let span = t.span.clone();
                KeyframeOp::Transform {
                    target: t.clone(),
                    modifiers: vec![Spanned::new(
                        StyleModifier {
                            key: Spanned::new(StyleKey::Height, span.clone()),
                            value: Spanned::new(value.clone(), span.clone()),
                        },
                        span,
                    )],
                }
            })
            .collect()
    };
    match verb {
        MotionVerb::Show(_) if opt_name(opts, "enter") == Some("expand") => {
            let mut ops = vec![KeyframeOp::Show(all())];
            ops.extend(height(StyleValue::Keyword("initial".into())));
            ops
        }
        MotionVerb::Hide(_) if opt_name(opts, "exit") == Some("collapse") => {
            let mut ops = vec![KeyframeOp::Hide(all())];
            ops.extend(height(StyleValue::Number { value: 0.0, unit: None }));
            ops
        }
        MotionVerb::Show(_) => vec![KeyframeOp::Show(all())],
        MotionVerb::Hide(_) => vec![KeyframeOp::Hide(all())],
        MotionVerb::Transform { modifiers, .. } => targets
            .iter()
            .map(|t| KeyframeOp::Transform { target: t.clone(), modifiers: modifiers.clone() })
            .collect(),
        MotionVerb::Constrain(d) => vec![KeyframeOp::Constrain(d.clone())],
        MotionVerb::Disable(n) => vec![KeyframeOp::Disable(n.clone())],
        MotionVerb::Enable(n) => vec![KeyframeOp::Enable(n.clone())],
        MotionVerb::Draw(_) | MotionVerb::Undraw(_) => {
            let undraw = matches!(verb, MotionVerb::Undraw(_));
            let to = opt(opts, "to")
                .and_then(|v| draw_to(&v.node))
                .unwrap_or(DrawTo::Fraction(if undraw { 0.0 } else { 1.0 }));
            targets
                .iter()
                .map(|t| KeyframeOp::Draw { target: t.clone(), to: to.clone() })
                .collect()
        }
        MotionVerb::Move { to, along, .. } => {
            let pin = match (to, along) {
                (Some(to), _) if to.node == "home" => PinTo::Home,
                (Some(to), _) => PinTo::Element(to.node.replace('.', "_")),
                (None, Some(path)) => PinTo::Along {
                    path: path.node.replace('.', "_"),
                    at: opt(opts, "to")
                        .and_then(|v| draw_to(&v.node))
                        .unwrap_or(DrawTo::Fraction(1.0)),
                },
                (None, None) => return vec![],
            };
            targets
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    // Paired destinations (`move a, b to x, y`).
                    let to = match (&pin, partners.get(i)) {
                        (PinTo::Element(_), Some(p)) => PinTo::Element(p.node.0.clone()),
                        _ => pin.clone(),
                    };
                    KeyframeOp::Pin { target: t.clone(), to }
                })
                .collect()
        }
        MotionVerb::Count(_) => {
            let Some(to) = opt_number(opts, "to") else { return vec![] };
            let fmt = match opt(opts, "format").map(|v| &v.node) {
                Some(MotionValue::Str(s)) => s.clone(),
                _ => "{}".to_string(),
            };
            let text = format_count(to, &fmt);
            targets
                .iter()
                .map(|t| {
                    let span = t.span.clone();
                    KeyframeOp::Transform {
                        target: t.clone(),
                        modifiers: vec![Spanned::new(
                            StyleModifier {
                                key: Spanned::new(StyleKey::Label, span.clone()),
                                value: Spanned::new(StyleValue::String(text.clone()), span.clone()),
                            },
                            span,
                        )],
                    }
                })
                .collect()
        }
        MotionVerb::Swap { .. } => {
            vec![KeyframeOp::Hide(all()), KeyframeOp::Show(partners.to_vec())]
        }
        MotionVerb::Camera(focus) => {
            let zoom = opt_number(opts, "zoom").unwrap_or(1.5);
            vec![KeyframeOp::Camera {
                focus: focus.as_ref().map(|f| ident(&f.node.replace('.', "_"), &f.span)),
                zoom,
            }]
        }
        MotionVerb::Effect { name, .. } if name.node == "mark" => all()
            .into_iter()
            .map(|t| KeyframeOp::Mark { target: t, kind: mark_kind(opts) })
            .collect(),
        MotionVerb::Effect { name, .. } if name.node == "unmark" => all().into_iter().map(KeyframeOp::Unmark).collect(),
        MotionVerb::Fly { .. }
        | MotionVerb::Effect { .. }
        | MotionVerb::Loop { .. }
        | MotionVerb::Call { .. }
        | MotionVerb::Insert { .. }
        | MotionVerb::UseLayout(_)
        | MotionVerb::SetState { .. } => vec![],
    }
}

/// Visit every statement of a motion tree in source order.
pub fn for_each_stmt<'a>(nodes: &'a [Spanned<MotionNode>], f: &mut dyn FnMut(&'a Spanned<MotionNode>)) {
    for n in nodes {
        match &n.node {
            MotionNode::Stmt(_) => f(n),
            MotionNode::Then(b)
            | MotionNode::After(_, b)
            | MotionNode::At(_, b)
            | MotionNode::When(_, _, b)
            | MotionNode::Beat(_, b) => for_each_stmt(b, f),
        }
    }
}

/// Operations for a freshly parsed keyframe, before selectors and macros are
/// resolved: plain names map to themselves, anything else is left for
/// [`expand`] (which regenerates the list).
pub fn naive_operations(nodes: &[Spanned<MotionNode>]) -> Vec<Spanned<KeyframeOp>> {
    let mut out = Vec::new();
    for_each_stmt(nodes, &mut |n| {
        let MotionNode::Stmt(s) = &n.node else { return };
        let plain = |sels: &[&Spanned<Selector>]| -> Vec<Spanned<Identifier>> {
            sels.iter()
                .filter_map(|sel| match &sel.node {
                    Selector::Name(name) if !name.contains('.') => Some(ident(name, &sel.span)),
                    _ => None,
                })
                .collect()
        };
        let (t, p): (Vec<&Spanned<Selector>>, Vec<&Spanned<Selector>>) = match &s.verb {
            MotionVerb::Show(v) | MotionVerb::Hide(v) | MotionVerb::Draw(v) | MotionVerb::Undraw(v) => {
                (v.iter().collect(), vec![])
            }
            MotionVerb::Transform { target, .. } | MotionVerb::Count(target) | MotionVerb::Move { target, .. } => {
                (vec![target], vec![])
            }
            MotionVerb::Swap { from, to } => (vec![from], vec![to]),
            _ => (vec![], vec![]),
        };
        for op in stmt_ops(&s.verb, &s.opts, &plain(&t), &plain(&p)) {
            out.push(Spanned::new(op, n.span.clone()));
        }
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_counts() {
        assert_eq!(format_count(14900.0, "€ {:,}"), "€ 14,900");
        assert_eq!(format_count(58.0, "{} lines"), "58 lines");
        assert_eq!(format_count(3.14159, "{:.2}"), "3.14");
        assert_eq!(format_count(1234567.5, "{:,.1}"), "1,234,567.5");
    }

    #[test]
    fn reads_leading_numbers() {
        assert_eq!(parse_leading_number("57 lines"), Some(57.0));
        assert_eq!(parse_leading_number("€ 14,900"), Some(14900.0));
        assert_eq!(parse_leading_number("none"), None);
    }
}
