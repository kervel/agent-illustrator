//! What the browser player plays equals what the native renderer samples.
//!
//! The seek tests compare settled states and `p.at` freezes; neither plays
//! the exported animations as a host does. A flip's opening half was exported
//! as 0 -> 0 (combined with the previous overlay's edge-on value) and played
//! as a blank for 250ms per page while every still looked right.
//!
//! This replays the manifest's animations with the Web Animations rules the
//! player relies on (created in order, a later one overrides an earlier one,
//! `fill: forwards`, delays honoured) and compares every numeric channel with
//! the native `--at` sample at the same instant, for scene steps with a flip,
//! with `show [from:]`, with a draw, and with lines opening and closing.

use agent_illustrator::motion::ease::Ease;
use agent_illustrator::{render_with_config, RenderConfig};
use std::collections::HashMap;
use std::path::Path;

fn config(dir: &Path) -> RenderConfig {
    let css = std::fs::read_to_string(dir.join("git-deck.css")).unwrap();
    RenderConfig::new().with_custom_css(css).with_template_base_path(dir.to_path_buf())
}

fn manifest(svg: &str) -> serde_json::Value {
    let start = svg.find("class=\"ail-motion\"><![CDATA[").expect("manifest") + "class=\"ail-motion\"><![CDATA[".len();
    let end = svg[start..].find("]]></script>").unwrap() + start;
    serde_json::from_str(&svg[start..end]).unwrap()
}

/// `SEL { prop: value !important; }` rules of a native `--at` still.
fn sampled(svg: &str) -> HashMap<(String, String), String> {
    let mut out = HashMap::new();
    for line in svg.lines() {
        let l = line.trim();
        let Some(open) = l.find(" { ") else { continue };
        if !l.ends_with("!important; }") {
            continue;
        }
        let sel = l[..open].to_string();
        let body = &l[open + 3..l.len() - "!important; }".len()];
        if let Some((p, v)) = body.split_once(": ") {
            out.insert((sel, p.trim().to_string()), v.trim().to_string());
        }
    }
    out
}

fn numbers(v: &str) -> Option<Vec<f64>> {
    let v = v.trim();
    if v == "none" {
        return None;
    }
    let mut out = Vec::new();
    for tok in v.split(|c: char| c.is_whitespace() || c == ',' || c == '(' || c == ')') {
        let t = tok.trim_end_matches("px").trim_end_matches("deg").trim_end_matches('%');
        if t.is_empty() || t == "inset" || t == "x" {
            continue;
        }
        out.push(t.parse::<f64>().ok()?);
    }
    (!out.is_empty()).then_some(out)
}

/// The wrapper opacity channel of the element a channel belongs to.
fn wrapper_opacity(m: &serde_json::Value, c: usize) -> Option<usize> {
    let sel = m["channels"][c][0].as_str()?;
    let id = sel.strip_prefix(".kfp-").or_else(|| sel.strip_prefix(".kf-"))?;
    let wrap = format!(".kf-{}", id);
    m["channels"].as_array()?.iter().position(|x| x[0].as_str() == Some(&wrap) && x[1].as_str() == Some("opacity"))
}

/// Wrapper opacity channels of the channel's element and its ancestors, by
/// the part naming (`d0_pg` is a part of `d0`).
fn ancestors_and_self(m: &serde_json::Value, c: usize) -> Vec<usize> {
    let sel = m["channels"][c][0].as_str().unwrap_or_default();
    let Some(id) = sel.strip_prefix(".kfp-").or_else(|| sel.strip_prefix(".kf-")) else { return vec![] };
    let chans = m["channels"].as_array().unwrap();
    let mut out: Vec<usize> = wrapper_opacity(m, c).into_iter().collect();
    let mut cur = format!(".kf-{}", id);
    while let Some(i) = cur.rfind('_') {
        cur.truncate(i);
        if let Some(p) = chans.iter().position(|x| x[0].as_str() == Some(&cur) && x[1].as_str() == Some("opacity")) {
            out.push(p);
        }
    }
    out
}

/// Hidden when frame f starts (its wrapper at opacity 0).
fn hidden_at_start(m: &serde_json::Value, f: usize, c: usize) -> bool {
    wrapper_opacity(m, c)
        .and_then(|o| numbers(m["settled"][f][o].as_str()?))
        .is_some_and(|v| v[0] == 0.0)
}

/// The value the player shows on a channel at time t of frame f.
fn played(m: &serde_json::Value, f: usize, c: usize, t: f64) -> Option<Vec<f64>> {
    let mut value = numbers(m["settled"][f][c].as_str()?)?;
    let mut first = true;
    for a in m["anims"][f].as_array()? {
        if a["c"].as_u64()? as usize != c || a.get("loop").and_then(|l| l.as_bool()).unwrap_or(false) {
            continue;
        }
        let backfill = first && hidden_at_start(m, f, c);
        first = false;
        let (start, dur) = (a["t"].as_f64()?, a["d"].as_f64()?.max(0.001));
        if t < start {
            // Before its delay: no effect, unless the player back-fills it
            // (the first animation of something hidden when the frame starts).
            if backfill {
                value = numbers(a["k"][0][1].as_str()?)?;
            }
            continue;
        }
        // A 1ms export of an instant change is a step.
        let p = if dur <= 0.002 { 1.0 } else { ((t - start) / dur).clamp(0.0, 1.0) };
        let e = Ease::parse_css(a["e"].as_str()?).unwrap_or(Ease::Linear).eval(p);
        let keys = a["k"].as_array()?;
        // The segment the eased progress falls in; an overshooting ease
        // (`pop`, e > 1) extrapolates the last segment, as browsers do.
        let n = keys.len();
        let mut seg = n - 2;
        for i in 0..n - 1 {
            if e <= keys[i + 1][0].as_f64()? {
                seg = i;
                break;
            }
        }
        let (o0, o1) = (keys[seg][0].as_f64()?, keys[seg + 1][0].as_f64()?);
        let (a0, a1) = (numbers(keys[seg][1].as_str()?)?, numbers(keys[seg + 1][1].as_str()?)?);
        let q = if o1 > o0 { (e - o0) / (o1 - o0) } else { 1.0 };
        let v: Vec<f64> = a0.iter().zip(&a1).map(|(x, y)| x + (y - x) * q).collect();
        value = v;
    }
    Some(value)
}

fn check(scene: &str, frames: &[usize]) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/motion");
    let src = std::fs::read_to_string(dir.join(format!("{scene}.ail"))).unwrap();
    let mut cfg = config(&dir);
    cfg.animate = true;
    let m = manifest(&render_with_config(&src, cfg).unwrap());
    let channels: Vec<(String, String)> = m["channels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| (c[0].as_str().unwrap().to_string(), c[1].as_str().unwrap().to_string()))
        .collect();
    let mut failures = Vec::new();
    for &f in frames {
        let d = m["frames"][f]["duration"].as_f64().unwrap();
        // Evenly through the step, and on both sides of every start: a
        // visibility step and a geometry hold that begin together must
        // agree on the first frame too.
        let mut times: Vec<f64> = (1..=12).map(|i| d * i as f64 / 13.0).collect();
        // Both sides of every start: a visibility step and a geometry hold
        // that begin together must agree from the first frame.
        let mut edges: Vec<f64> = m["anims"][f].as_array().unwrap().iter().map(|a| a["t"].as_f64().unwrap()).collect();
        edges.sort_by(|a, b| a.partial_cmp(b).unwrap());
        edges.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        for t0 in edges {
            times.extend([t0 - 0.0005, t0 + 0.0005]);
        }
        times.retain(|t| *t > 0.0 && *t < d);
        times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        // One compile, every sample.
        let mut c = config(&dir);
        c.frame = Some(f.to_string());
        c.sample_at = times.clone();
        let all = render_with_config(&src, c).unwrap();
        let blocks: Vec<&str> = all.split("/* t=").skip(1).collect();
        assert_eq!(blocks.len(), times.len());
        for (t, block) in times.iter().copied().zip(blocks) {
            let native = sampled(block);
            for (ci, (sel, prop)) in channels.iter().enumerate() {
                if prop == "text" || prop == "clip-path" {
                    continue;
                }
                // What cannot be seen cannot flash: compare while it and
                // every ancestor (a part `d0_pg` is inside `d0`) is visible
                // (1/200 opacity, the last instant of a fade, is not).
                let visible = ancestors_and_self(&m, ci).iter().all(|o| {
                    played(&m, f, *o, t).is_none_or(|v| v[0] > 0.005)
                });
                if !visible {
                    continue;
                }
                let (Some(n), Some(p)) = (native.get(&(sel.clone(), prop.clone())).and_then(|v| numbers(v)), played(&m, f, ci, t))
                else {
                    continue;
                };
                // Overlapping tracks are exported sampled at 60 Hz: allow
                // that much interpolation error, not a missing motion.
                let tol = match prop.as_str() {
                    "translate" | "width" | "height" => 2.0,
                    _ => 0.05,
                };
                if n.len() == p.len() && n.iter().zip(&p).any(|(a, b)| (a - b).abs() > tol) {
                    failures.push(format!("{scene} frame {f} t={t:.3} {sel} {prop}: native {n:?}, played {p:?}"));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{} mismatches:\n{}", failures.len(), failures.iter().take(12).cloned().collect::<Vec<_>>().join("\n"));
}

#[test]
fn a_flip_plays_both_halves() {
    check("git-copies", &[2]);
}

#[test]
fn show_from_and_moves_play_as_sampled() {
    check("git-copies", &[1]);
}

#[test]
fn draws_play_as_sampled() {
    check("git-branches", &[0, 5]);
}

#[test]
fn lines_opening_and_closing_play_as_sampled() {
    check("git-merge", &[2, 3]);
}

#[test]
fn a_flip_is_never_blank_except_at_its_turn() {
    // The page turning away plus the page turning in: some width is always
    // showing, except within a frame of the edge-on instant.
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/motion");
    let src = std::fs::read_to_string(dir.join("git-copies.ail")).unwrap();
    let mut cfg = config(&dir);
    cfg.animate = true;
    let m = manifest(&render_with_config(&src, cfg).unwrap());
    let ch = |suffix: &str| {
        m["channels"].as_array().unwrap().iter().position(|c| {
            c[0].as_str().unwrap().ends_with(suffix) && c[1].as_str().unwrap() == "scale"
        })
    };
    let (a, b) = (ch("-d0_pg").unwrap(), ch("-k0_pg").unwrap());
    // d0's flip: starts after the badges go (0.15s), lasts the swap's duration.
    let (start, dur): (f64, f64) = (0.15, 0.5);
    let mid = start + dur / 2.0;
    let mut t = start;
    while t <= start + dur {
        if (t - mid).abs() > 0.016 {
            let w = played(&m, 2, a, t).unwrap()[0] + played(&m, 2, b, t).unwrap()[0];
            assert!(w >= 0.2, "at t={t:.3} the flip shows {w:.3} of a page");
        }
        t += 0.005;
    }
}

#[test]
fn nothing_changes_at_the_instant_something_is_hidden() {
    // An element's appearance must never change in the same instant as its
    // visibility: at the end of a flip the old page was hidden while its
    // turned-away hold reset to full width, and a paint between the two
    // showed it, full size, for one frame.
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/motion");
    let mut bad = Vec::new();
    for scene in ["git-copies", "git-snapshots", "git-branches", "git-merge"] {
        let src = std::fs::read_to_string(dir.join(format!("{scene}.ail"))).unwrap();
        let mut cfg = config(&dir);
        cfg.animate = true;
        let m = manifest(&render_with_config(&src, cfg).unwrap());
        let chans = m["channels"].as_array().unwrap();
        for (f, anims) in m["anims"].as_array().unwrap().iter().enumerate() {
            let anims = anims.as_array().unwrap();
            for hide in anims {
                let c = hide["c"].as_u64().unwrap() as usize;
                let (sel, prop) = (chans[c][0].as_str().unwrap(), chans[c][1].as_str().unwrap());
                let last = hide["k"].as_array().unwrap().last().unwrap()[1].as_str().unwrap();
                if !(prop == "opacity" && sel.starts_with(".kf-") && last == "0" && hide["d"].as_f64().unwrap() <= 0.002) {
                    continue;
                }
                let t0 = hide["t"].as_f64().unwrap();
                let id = &sel[".kf-".len()..];
                for a in anims {
                    let ac = a["c"].as_u64().unwrap() as usize;
                    if ac == c || (a["t"].as_f64().unwrap() - t0).abs() > 0.002 || a["d"].as_f64().unwrap() > 0.002 {
                        continue;
                    }
                    let asel = chans[ac][0].as_str().unwrap();
                    let aid = asel.split_once('-').map(|x| x.1).unwrap_or("");
                    if aid == id || aid.starts_with(&format!("{id}_")) {
                        bad.push(format!("{scene} frame {f}: {asel} {} steps at {t0} as {sel} hides", chans[ac][1]));
                    }
                }
            }
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}
