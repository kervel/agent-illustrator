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

/// The value the player shows on a channel at time t of frame f.
fn played(m: &serde_json::Value, f: usize, c: usize, t: f64) -> Option<Vec<f64>> {
    let mut value = numbers(m["settled"][f][c].as_str()?)?;
    for a in m["anims"][f].as_array()? {
        if a["c"].as_u64()? as usize != c || a.get("loop").and_then(|l| l.as_bool()).unwrap_or(false) {
            continue;
        }
        let (start, dur) = (a["t"].as_f64()?, a["d"].as_f64()?.max(0.001));
        if t < start {
            continue; // before its delay: no effect (fill is forwards only)
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
        for i in 1..=12 {
            let t = d * i as f64 / 13.0;
            let mut c = config(&dir);
            c.frame = Some(f.to_string());
            c.at = Some(format!("{}s", t));
            let native = sampled(&render_with_config(&src, c).unwrap());
            for (ci, (sel, prop)) in channels.iter().enumerate() {
                if prop == "text" || prop == "clip-path" {
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
