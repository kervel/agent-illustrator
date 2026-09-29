//! `--serve`: a live preview. `--film`: a step, frame by frame.
//!
//! Both run the real player in the browser: what you see is what a deck
//! host will show. No dependencies: a tiny HTTP/1.1 loop on std::net.

use agent_illustrator::{render_with_config, RenderConfig};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// The stylesheets, joined in order (later wins), `@import`s first.
pub fn load_css(paths: &[PathBuf]) -> Result<Option<String>, String> {
    if paths.is_empty() {
        return Ok(None);
    }
    let mut css = String::new();
    for path in paths {
        let c = std::fs::read_to_string(path).map_err(|e| format!("Error loading CSS '{}': {}", path.display(), e))?;
        css.push_str(&format!("/* {} */\n", path.display()));
        css.push_str(&c);
        css.push('\n');
    }
    let (imports, rules): (Vec<&str>, Vec<&str>) = css.lines().partition(|l| l.trim_start().starts_with("@import"));
    Ok(Some(format!("{}\n{}", imports.join("\n"), rules.join("\n"))))
}

fn render(input: &Path, css: &[PathBuf], base: &RenderConfig) -> Result<String, String> {
    let source = std::fs::read_to_string(input).map_err(|e| format!("{}: {}", input.display(), e))?;
    let mut config = base.clone();
    config.custom_css = load_css(css)?;
    config.animate = true;
    render_with_config(&source, config).map_err(|e| e.to_string())
}

/// Changes to the input or a stylesheet (or a file next to the input: an
/// imported component, an SVG template) reload the page.
fn version(input: &Path, css: &[PathBuf]) -> String {
    let mtime = |p: &Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    let mut latest: Option<SystemTime> = None;
    let mut consider = |t: Option<SystemTime>| {
        if let Some(t) = t {
            latest = Some(latest.map_or(t, |l| l.max(t)));
        }
    };
    consider(mtime(input));
    for c in css {
        consider(mtime(c));
    }
    if let Some(dir) = input.parent().filter(|d| !d.as_os_str().is_empty()).or(Some(Path::new("."))) {
        for entry in walk(dir, 2) {
            consider(mtime(&entry));
        }
    }
    latest
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_millis().to_string())
        .unwrap_or_default()
}

fn walk(dir: &Path, depth: usize) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else { return out };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            if depth > 0 {
                out.extend(walk(&p, depth - 1));
            }
        } else if p.extension().is_some_and(|x| x == "ail" || x == "svg" || x == "css") {
            out.push(p);
        }
    }
    out
}

const PAGE: &str = r#"<!doctype html>
<html><head><meta charset="utf-8"><title>ail serve</title>
<style>
  body { margin: 0; background: #1e2127; color: #d7dae0; font: 14px/1.4 system-ui, sans-serif; }
  #bar { display: flex; gap: 14px; align-items: center; padding: 10px 16px; background: #282c34; }
  #bar button { font: inherit; padding: 4px 12px; }
  #scrub { flex: 1; }
  #title { font-weight: 600; }
  #err { color: #ff7b72; white-space: pre-wrap; font-family: ui-monospace, monospace; padding: 0 16px; }
  #stage { display: flex; justify-content: center; padding: 16px; }
  #stage svg { max-width: 96vw; max-height: 82vh; height: auto; background: #fff; }
  kbd { background: #3a3f4b; padding: 1px 6px; border-radius: 4px; }
</style></head>
<body>
<div id="bar">
  <button id="prev">&#9664;</button><span id="info">&nbsp;</span><button id="next">&#9654;</button>
  <span id="title"></span>
  <input id="scrub" type="range" min="0" max="1" step="0.001" value="1"><span id="t"></span>
  <span><kbd>&larr;</kbd> <kbd>&rarr;</kbd> step &middot; scrub plays the current frame</span>
</div>
<div id="err"></div>
<div id="stage"></div>
<script src="/player.js"></script>
<script>
let p = null, step = 0, ver = null;
const $ = (id) => document.getElementById(id);
function show() {
  if (!p) return;
  const m = p.meta(p.step);
  $('info').textContent = 'step ' + (p.step + 1) + '/' + p.steps.length + ' · ' + p.frames[p.frame];
  $('title').textContent = m.title || '';
  $('scrub').value = 1; $('t').textContent = '';
}
async function load() {
  const r = await fetch('/svg');
  const txt = await r.text();
  if (!r.ok) { $('err').textContent = txt; return; }
  $('err').textContent = '';
  $('stage').innerHTML = txt;
  const svg = $('stage').querySelector('svg');
  if (!svg.querySelector('script.ail-motion')) { p = null; $('info').textContent = '(no keyframes)'; return; }
  p = ail.player(svg);
  p.on('step', () => { step = p.step; setTimeout(show, 0); });
  p.goToStep(Math.min(step, p.steps.length - 1));
  show();
}
$('next').onclick = () => { if (p) p.nextStep(); };
$('prev').onclick = () => { if (p) p.prevStep(); };
document.addEventListener('keydown', (e) => {
  if (!p) return;
  if (e.key === 'ArrowRight' || e.key === ' ') { p.nextStep(); e.preventDefault(); }
  if (e.key === 'ArrowLeft') { p.prevStep(); e.preventDefault(); }
});
$('scrub').oninput = () => {
  if (!p) return;
  const f = p.frame, d = p.manifest.frames[f].duration, t = $('scrub').value * d;
  p.at(f, t); $('t').textContent = t.toFixed(2) + 's / ' + d.toFixed(2) + 's';
};
async function poll() {
  try {
    const v = await (await fetch('/version')).text();
    if (ver !== null && v !== ver) await load();
    ver = v;
  } catch (e) {}
  setTimeout(poll, 600);
}
load().then(poll);
</script></body></html>
"#;

pub fn serve(input: PathBuf, css: Vec<PathBuf>, base: RenderConfig, port: u16) -> Result<(), String> {
    let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|e| format!("cannot listen on port {}: {}", port, e))?;
    eprintln!(
        "serving {} on http://127.0.0.1:{}/  (arrow keys step; saves reload; Ctrl-C stops)",
        input.display(),
        port
    );
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { continue };
        let mut line = String::new();
        let mut reader = BufReader::new(&stream);
        if reader.read_line(&mut line).is_err() {
            continue;
        }
        // Drain the headers.
        let mut h = String::new();
        while reader.read_line(&mut h).is_ok_and(|n| n > 2) {
            h.clear();
        }
        let path = line.split_whitespace().nth(1).unwrap_or("/");
        let (status, ctype, body) = match path {
            "/" | "/index.html" => ("200 OK", "text/html; charset=utf-8", PAGE.to_string()),
            "/player.js" => ("200 OK", "text/javascript", agent_illustrator::motion::render::PLAYER_JS.to_string()),
            "/version" => ("200 OK", "text/plain", version(&input, &css)),
            "/svg" => match render(&input, &css, &base) {
                Ok(svg) => ("200 OK", "image/svg+xml", svg),
                Err(e) => ("500 Internal Server Error", "text/plain; charset=utf-8", e),
            },
            _ => ("404 Not Found", "text/plain", "not found".to_string()),
        };
        let _ = write!(
            stream,
            "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{}",
            status,
            ctype,
            body.len(),
            body
        );
    }
    Ok(())
}

/// A self-contained HTML page: step `step` (its frame and the `[auto]`
/// frames after it) sampled every `every` seconds by the browser player,
/// one small picture per sample. For spotting flicker and jumps.
pub fn film(svg: &str, step: usize, every: f64) -> String {
    let player = agent_illustrator::motion::render::PLAYER_JS;
    format!(
        r#"<!doctype html>
<html><head><meta charset="utf-8"><title>film: step {step}</title>
<style>
  body {{ margin: 0; background: #fff; font: 12px system-ui, sans-serif; }}
  #film {{ display: grid; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); gap: 8px; padding: 8px; }}
  figure {{ margin: 0; border: 1px solid #ddd; }}
  figure svg {{ width: 100%; height: auto; display: block; }}
  figcaption {{ padding: 2px 6px; color: #555; }}
</style></head>
<body><div id="film"></div>
<template id="src">{svg}</template>
<script>{player}</script>
<script>
(function () {{
  const src = document.getElementById('src').content.querySelector('svg');
  const probe = src.cloneNode(true);
  document.body.appendChild(probe);
  const p0 = ail.player(probe);
  const k = Math.min({step}, p0.steps.length - 1);
  const first = p0.steps[k], last = p0._stepEnd(k);
  const frames = p0.manifest.frames;
  probe.remove();
  const film = document.getElementById('film');
  for (let f = first; f <= last; f++) {{
    const d = frames[f].duration;
    for (let t = 0; t <= d + 1e-9; t += {every}) {{
      const fig = document.createElement('figure');
      const svg = src.cloneNode(true);
      fig.appendChild(svg);
      const cap = document.createElement('figcaption');
      cap.textContent = frames[f].name + '  t=' + t.toFixed(2) + 's';
      fig.appendChild(cap);
      film.appendChild(fig);
      ail.player(svg).at(f, Math.min(t, d));
    }}
  }}
}})();
</script></body></html>
"#,
        step = step,
        svg = svg,
        player = player,
        every = every
    )
}
