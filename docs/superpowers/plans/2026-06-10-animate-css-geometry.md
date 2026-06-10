# Geometry in `--animate-css` — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `--animate-css` animates geometry (element position/size/rotation, connection paths) in addition to visibility, so a CSS-only SVG embedded as an `<img>` (README) shows the same motion as the JS `--animate` preview. Then render the token-prediction README example with `--animate-css`.

**Architecture:** `generate_animate_css` already builds per-element/per-connection opacity timelines and emits step-end `@keyframes`. Add geometry timelines (transform/size/`d`/crossfade) emitted as separate **smooth** `@keyframes` (hold-then-ease per frame), layered as a second animation on the same selectors. Share the per-frame transform-string and `d`-string builders with `generate_keyframe_css` (DRY).

**Tech Stack:** Rust, SVG/CSS output. Tests via `cargo test`. Branch off `main`.

**Spec:** `docs/superpowers/specs/2026-06-10-animate-css-geometry-design.md`

---

## File Structure

- `src/renderer/svg.rs` — extract `frame_transform_css(diff)` + make `connection_path_d` and a new `build_conn_meta(result)` `pub(crate)`; `generate_keyframe_css` uses `frame_transform_css`.
- `src/lib.rs` — `generate_animate_css` gains a `conn_meta` param + geometry `@keyframes`; build `conn_meta` at the call site.
- `tests/animate_css_geometry.rs` — integration tests.
- `examples/render-all.sh` — `token-prediction` → `--animate-css`.
- `examples/token-prediction.svg` — re-rendered (animated).
- `docs/skill-animation.md` — note.

---

## Task 1: DRY — shared per-frame string builders

**Files:**
- Modify: `src/renderer/svg.rs`

- [ ] **Step 1: Extract `frame_transform_css`** — add a `pub(crate)` free fn in `src/renderer/svg.rs` that returns the position+rotation transform string for an element diff (None when neither moves):

```rust
/// The `transform` value (translate + rotate) for an element diff's position/rotation,
/// shared by the frame-class CSS and the @keyframes animator. None when neither changes.
pub(crate) fn frame_transform_css(tx: Option<f64>, ty: Option<f64>, rotation: Option<f64>) -> Option<String> {
    let mut parts = Vec::new();
    if tx.is_some() || ty.is_some() {
        parts.push(format!("translate({}px, {}px)", tx.unwrap_or(0.0), ty.unwrap_or(0.0)));
    }
    if let Some(rot) = rotation {
        parts.push(format!("rotate({}deg)", rot));
    }
    if parts.is_empty() { None } else { Some(parts.join(" ")) }
}
```

- [ ] **Step 2: Use it in `generate_keyframe_css`** — replace the inline `xf` block (the `let mut xf = Vec::new(); if diff.tx.is_some() … rotate … .kf-{} { transform }`) with:

```rust
            if let Some(t) = frame_transform_css(diff.tx, diff.ty, diff.rotation) {
                css.push_str(&format!("  .kf-{} {{ transform: {}; }}\n", elem_id, t));
            }
```

- [ ] **Step 3: Make the connection helpers reusable** — change `fn connection_path_d(` to `pub(crate) fn connection_path_d(`. Extract the `conn_meta` builder (currently inline in `render_svg_with_keyframes`) into a `pub(crate)` fn and call it from there:

```rust
pub(crate) fn build_conn_meta(result: &LayoutResult) -> std::collections::HashMap<String, (RoutingMode, bool, f64)> {
    result.connections.iter().enumerate().map(|(i, c)| {
        let id = c.name.as_ref().map(|n| n.0.clone()).unwrap_or_else(|| format!("idx{}", i));
        let marker = matches!(c.direction, ConnectionDirection::Forward | ConnectionDirection::Bidirectional);
        (id, (c.routing_mode, marker, c.styles.stroke_width.unwrap_or(2.0)))
    }).collect()
}
```

In `render_svg_with_keyframes`, replace the inline `let conn_meta = … .collect();` with `let conn_meta = build_conn_meta(result);`.

- [ ] **Step 4: Verify no behavior change**

Run: `cargo build 2>&1 | grep -E "error" | head` (none), `cargo test 2>&1 | grep -cE "test result: ok"` (all), `cargo test --test svg_regression 2>&1 | grep "test result"`. Connection/keyframe output is byte-identical (pure refactor).

- [ ] **Step 5: Commit**

```bash
git add src/renderer/svg.rs
git commit -m "refactor(svg): share frame_transform_css + build_conn_meta + pub(crate) connection_path_d"
```

---

## Task 2: Element geometry in `--animate-css`

**Files:**
- Modify: `src/lib.rs` (`generate_animate_css` + its call site ~L538)
- Test: `tests/animate_css_geometry.rs` (new)

- [ ] **Step 1: Write the failing test** — create `tests/animate_css_geometry.rs`:

```rust
use agent_illustrator::{render_with_config, RenderConfig};

const SRC: &str = r#"
rect box [width: 100, height: 50, label: "B"]
rect other [width: 100, height: 50]
constrain box.center_x = 150
constrain box.center_y = 100
constrain other.center_x = 450
constrain other.center_y = 100
box.right -> other.left as feed
keyframe "idle" {}
keyframe "grow" { transform box [width: 260] }
keyframe "move" { transform box [dx: 40] }
"#;

fn animate_css(src: &str) -> String {
    let mut cfg = RenderConfig::new();
    cfg.animate_css = true; // adjust to the real field/setter (grep `animate_css` in lib.rs)
    render_with_config(src, cfg).expect("render")
}

#[test]
fn animate_css_emits_element_transform_keyframes() {
    let svg = animate_css(SRC);
    // box moves (dx) → a transform keyframe with translate, plus a geometry animation.
    assert!(svg.contains("transform: translate("),
        "expected transform keyframe for moved element, got:\n{}", svg);
}

#[test]
fn animate_css_emits_element_size_keyframes() {
    let svg = animate_css(SRC);
    assert!(svg.contains("width: 260px"),
        "expected width keyframe for grown element, got:\n{}", svg);
}

#[test]
fn animate_css_geometry_is_smooth_visibility_is_step() {
    // an element that both hides/shows AND moves carries two animations: a step-end
    // visibility one and a smooth geometry one.
    let svg = animate_css(SRC);
    // feed connection / box geometry uses a non-step (ease) animation somewhere
    assert!(svg.contains("ease infinite") || svg.contains("ease-in-out infinite"),
        "expected a smooth (ease) geometry animation, got:\n{}", svg);
    assert!(svg.contains("step-end infinite"),
        "visibility should still be step-end, got:\n{}", svg);
}
```

NOTE: verify how `animate_css` is set on `RenderConfig` (grep `animate_css` / `with_animate`); use the real field/builder. Keep the assertions.

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test animate_css_geometry 2>&1 | tail -20`
Expected: FAIL — only opacity keyframes emitted today.

- [ ] **Step 3a: Add the `conn_meta` param + build it at the call site** — in `src/lib.rs`, change the call (~L538):

```rust
        } else if config.animate_css {
            let conn_meta = renderer::svg::build_conn_meta(&result);
            let css = generate_animate_css(&frame_states, &frame_diffs, &conn_meta);
            if let Some(pos) = svg.rfind("</style>") {
                svg.insert_str(pos, &css);
            }
        }
```

(Verify the path to `build_conn_meta` — it's `pub(crate)` in `renderer::svg`; import or fully-qualify. `RoutingMode` is `layout::...`; the param type mirrors `generate_keyframe_css`.)

Update `generate_animate_css`'s signature:

```rust
fn generate_animate_css(
    frame_states: &[layout::keyframe::FrameState],
    frame_diffs: &[layout::keyframe::FrameLayout],
    conn_meta: &std::collections::HashMap<String, (renderer::svg::RoutingMode, bool, f64)>,
) -> String {
```

(Use whatever `RoutingMode` path resolves — likely `crate::layout::types::RoutingMode`. Match the type `build_conn_meta` returns; adjust the alias.)

- [ ] **Step 3b: Emit element geometry keyframes** — after the existing element opacity loop in `generate_animate_css`, add a geometry pass. Add a smooth-keyframes helper at module level in `src/lib.rs`:

```rust
/// Emit a @keyframes body for a per-frame value timeline, holding each frame's value
/// then easing to the next (mimics the JS frame-class + transition feel). `vals[i]` is
/// the full CSS declaration value for frame i (e.g. "translate(40px, 0px)"); None = use
/// `identity`. Returns None if every frame equals `identity` (nothing to animate).
fn smooth_keyframes_body(prop: &str, vals: &[Option<String>], identity: &str, pct_per_frame: f64) -> Option<String> {
    if vals.iter().all(|v| v.as_deref().unwrap_or(identity) == identity) {
        return None;
    }
    let val = |i: usize| vals[i].clone().unwrap_or_else(|| identity.to_string());
    let trans = pct_per_frame * 0.3; // ~30% of the frame window eases, the rest holds
    let mut body = String::new();
    body.push_str(&format!("  0% {{ {}: {}; }}\n", prop, val(0)));
    for i in 1..vals.len() {
        let start = i as f64 * pct_per_frame;
        // hold previous value into the frame start, then ease to this frame's value
        body.push_str(&format!("  {:.2}% {{ {}: {}; }}\n", start, prop, val(i - 1)));
        body.push_str(&format!("  {:.2}% {{ {}: {}; }}\n", (start + trans).min(100.0), prop, val(i)));
    }
    body.push_str(&format!("  100% {{ {}: {}; }}\n", prop, val(vals.len() - 1)));
    Some(body)
}
```

Then build per-element transform and size timelines from `frame_diffs` and emit them. After the element opacity loop:

```rust
    // Element geometry: transform (wrapper) + size (shape), animated smoothly.
    // Collect per-frame values keyed by element id.
    let mut xf_tl: std::collections::BTreeMap<String, Vec<Option<String>>> = std::collections::BTreeMap::new();
    let mut size_tl: std::collections::BTreeMap<String, Vec<Option<String>>> = std::collections::BTreeMap::new();
    for (i, diff) in frame_diffs.iter().enumerate() {
        for (id, d) in &diff.element_diffs {
            if let Some(t) = renderer::svg::frame_transform_css(d.tx, d.ty, d.rotation) {
                xf_tl.entry(id.clone()).or_insert_with(|| vec![None; n])[i] = Some(t);
            }
            let mut sz = Vec::new();
            if let Some(w) = d.width { sz.push(format!("width: {}px", w)); }
            if let Some(h) = d.height { sz.push(format!("height: {}px", h)); }
            // size is emitted as separate width/height keyframes below; track raw dims
            if d.width.is_some() {
                size_tl.entry(format!("{}\u{1}w", id)).or_insert_with(|| vec![None; n])[i] = Some(format!("{}px", d.width.unwrap()));
            }
            if d.height.is_some() {
                size_tl.entry(format!("{}\u{1}h", id)).or_insert_with(|| vec![None; n])[i] = Some(format!("{}px", d.height.unwrap()));
            }
            let _ = sz;
        }
    }
    for (id, vals) in &xf_tl {
        if let Some(body) = smooth_keyframes_body("transform", vals, "translate(0px, 0px)", pct_per_frame) {
            let anim = format!("kf-geo-{}", id);
            css.push_str(&format!("@keyframes {} {{\n{}}}\n", anim, body));
            css.push_str(&format!(".kf-{} {{ animation: {} {:.1}s ease infinite; }}\n", id, anim, total_duration));
        }
    }
    for (key, vals) in &size_tl {
        // key is "<id>\u{1}w" or "<id>\u{1}h"
        let (id, prop) = { let mut it = key.split('\u{1}'); (it.next().unwrap(), if key.ends_with('w') { "width" } else { "height" }); };
        // identity = base dim is unknown here; use the first defined value as the from-anchor
        // so the element rests at base until its first change. Simpler: animate only when
        // present; CSS leaves the attribute value when the keyframe has no stop.
        if let Some(body) = smooth_keyframes_body(prop, vals, "", pct_per_frame) {
            let anim = format!("kf-{}-{}", prop, id);
            css.push_str(&format!("@keyframes {} {{\n{}}}\n", anim, body));
            css.push_str(&format!("#{} {{ animation: {} {:.1}s ease infinite; }}\n", id, anim, total_duration));
        }
    }
```

NOTE: the `identity` for width/height is the element's base attribute value (not 0). `smooth_keyframes_body` with `identity=""` would emit literal `width: ;` for None frames — WRONG. Fix in implementation: for size, the "from" value at frames with no diff must be the element's **base width/height**. Easiest: look up the base dim from the layout result (pass element base sizes into `generate_animate_css`, or thread a `base_dims: &HashMap<String,(f64,f64)>`), and use it as `identity` per element. Implement that: build `base_dims` from `result.root_elements` (recursively, id → (w,h)) in `lib.rs` and pass it in; in the size loop use the element's base dim string as `identity`. Adjust the helper call accordingly. (The transform identity `translate(0px,0px)` is correct as-is — base position needs no offset.)

- [ ] **Step 4: Run to verify passing**

Run: `cargo test --test animate_css_geometry 2>&1 | tail -20`
Expected: the transform + size + smooth/step tests PASS.

- [ ] **Step 5: Full suite + clippy**

Run: `cargo test 2>&1 | grep -cE "test result: ok"`, `cargo clippy --all-targets 2>&1 | grep -cE "^warning: "` (0).

- [ ] **Step 6: Commit**

```bash
git add src/lib.rs tests/animate_css_geometry.rs
git commit -m "feat(animate-css): animate element transform + size (smooth), keep visibility stepped"
```

---

## Task 3: Connection geometry in `--animate-css`

**Files:**
- Modify: `src/lib.rs` (`generate_animate_css` connection passes)
- Test: `tests/animate_css_geometry.rs`

- [ ] **Step 1: Write the failing tests** — append:

```rust
#[test]
fn animate_css_morphable_connection_d_keyframes() {
    let svg = animate_css(SRC); // feed: straight box.right->other.left, box widens (morphable)
    assert!(svg.contains("d: path("),
        "expected d: keyframes for the following connection, got:\n{}", svg);
}

#[test]
fn animate_css_reshaping_connection_crossfades() {
    // orthogonal route that reshapes when the endpoint shifts → variant crossfade
    let svg = animate_css(r#"
rect box [width: 80, height: 40]
rect other [width: 80, height: 40]
constrain box.center_x = 120
constrain box.center_y = 100
constrain other.center_x = 120
constrain other.center_y = 320
box.bottom -> other.top as link
keyframe "idle" {}
keyframe "shift" { transform box [dx: 220] }
"#);
    // crossfade variant class participates in an opacity animation
    assert!(svg.contains("conn-link-fshift"),
        "expected crossfade variant for reshaping route, got:\n{}", svg);
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test animate_css_geometry animate_css_morphable_connection_d_keyframes animate_css_reshaping_connection_crossfades 2>&1 | tail -20`
Expected: FAIL.

- [ ] **Step 3a: Fix the connection visibility selector** — in `generate_animate_css`'s connection opacity loop, change `.ai-connection.conn-{}` to `.conn-{}` (consistency with `generate_keyframe_css`; toggles labels too).

- [ ] **Step 3b: Emit morphable `d` keyframes + crossfade variant opacity** — after the connection opacity loop:

```rust
    // Connection geometry: morphable → smooth d: morph; reshaping → crossfade variants.
    let mut d_tl: std::collections::BTreeMap<String, Vec<Option<String>>> = std::collections::BTreeMap::new();
    // variant_tl: key = "<id>\u{1}<frame>", value = opacity timeline (1 only in that frame)
    let mut variant_tl: std::collections::BTreeMap<String, Vec<f64>> = std::collections::BTreeMap::new();
    let mut base_fade: std::collections::BTreeMap<String, Vec<f64>> = std::collections::BTreeMap::new();
    for (i, diff) in frame_diffs.iter().enumerate() {
        for (id, d) in &diff.connection_diffs {
            if let Some(pts) = &d.path {
                if d.morphable {
                    if let Some((routing, marker, sw)) = conn_meta.get(id) {
                        let ds = renderer::svg::connection_path_d(pts, *routing, *marker, *sw);
                        d_tl.entry(id.clone()).or_insert_with(|| vec![None; n])[i] = Some(format!("path(\"{}\")", ds));
                    }
                } else {
                    base_fade.entry(id.clone()).or_insert_with(|| vec![1.0; n])[i] = 0.0;
                    variant_tl.entry(format!("{}\u{1}{}", id, frame_diffs[i].name)).or_insert_with(|| vec![0.0; n])[i] = 1.0;
                }
            }
        }
    }
    for (id, vals) in &d_tl {
        if let Some(body) = smooth_keyframes_body("d", vals, "", pct_per_frame) {
            // identity "" → the connection rests at its attribute `d` until first change;
            // like size, anchor the from-value: use the first defined d as identity.
            let anim = format!("kf-d-{}", id);
            css.push_str(&format!("@keyframes {} {{\n{}}}\n", anim, body));
            css.push_str(&format!(".conn-{} {{ animation: {} {:.1}s ease infinite; }}\n", id, anim, total_duration));
        }
    }
    // base path fades out in reshaping frames; each variant fades in for its frame (stepped)
    for (id, tl) in &base_fade {
        let anim = format!("kf-basefade-{}", id);
        css.push_str(&step_opacity_keyframes(&anim, tl, pct_per_frame, n));
        css.push_str(&format!(".conn-{}-base {{ animation: {} {:.1}s step-end infinite; }}\n", id, anim, total_duration));
    }
    for (key, tl) in &variant_tl {
        let mut it = key.split('\u{1}'); let id = it.next().unwrap(); let frame = it.next().unwrap();
        let anim = format!("kf-variant-{}-{}", id, frame);
        css.push_str(&step_opacity_keyframes(&anim, tl, pct_per_frame, n));
        css.push_str(&format!(".conn-{}-f{} {{ animation: {} {:.1}s step-end infinite; }}\n", id, frame, anim, total_duration));
    }
```

Factor the existing opacity-keyframe emission (used in the element/connection opacity loops) into `fn step_opacity_keyframes(anim_name, timeline, pct_per_frame, n) -> String` and reuse it for base_fade/variant_tl. (For the `d` identity-anchoring, mirror the size fix: use the connection's frame-0 `d` as the `identity`; compute it from the base connection path via `conn_meta` + the base `result` — thread base `d` strings in, or accept the first-defined value as the rest value. Implement whichever is simpler and assert the test.)

- [ ] **Step 4: Run to verify passing**

Run: `cargo test --test animate_css_geometry 2>&1 | tail -20` — all PASS.

- [ ] **Step 5: Full suite + clippy + examples**

Run: `cargo test 2>&1 | grep -cE "test result: ok"`, `cargo clippy --all-targets 2>&1 | grep -cE "^warning: "` (0), `bash examples/render-all.sh 2>&1 | grep -i fail || echo ok`. agentic-loop-story (`--animate-css`, visibility) must still animate.

- [ ] **Step 6: Commit**

```bash
git add src/lib.rs tests/animate_css_geometry.rs
git commit -m "feat(animate-css): connection d-morph + reshaping crossfade in CSS-only mode"
```

---

## Task 4: Animate the README example

**Files:**
- Modify: `examples/render-all.sh`
- Regenerate: `examples/token-prediction.svg`

- [ ] **Step 1: Flag token-prediction for --animate-css** — in `examples/render-all.sh` `extra_flags_for`:

```bash
extra_flags_for() {
    case "$1" in
        agentic-loop-story)   echo "--animate-css" ;;
        token-prediction)     echo "--animate-css" ;;
        *)                    echo "" ;;
    esac
}
```

- [ ] **Step 2: Re-render + sanity check**

Run: `bash examples/render-all.sh 2>&1 | grep -i token`
Run: `grep -cE "@keyframes kf-(geo|width|d)-" examples/token-prediction.svg` — expect non-zero (geometry keyframes present).
Open `examples/token-prediction.svg` (file:// or browser) — the box grows, tokens slide in, the feed connector follows, looping. (Matches the JS preview, now CSS-only.)

- [ ] **Step 3: Commit**

```bash
git add examples/render-all.sh examples/token-prediction.svg
git commit -m "docs(examples): render token-prediction with --animate-css (animated in README)"
```

---

## Task 5: Docs

**Files:**
- Modify: `docs/skill-animation.md`

- [ ] **Step 1** — in the CSS Transitions / animation section, add:

```markdown
### --animate vs --animate-css

`--animate` embeds JS that flips a `frame-<name>` class — full motion, but only when the
SVG is opened directly (JS doesn't run when an SVG is embedded as an `<img>`, e.g. a
GitHub README). `--animate-css` is a self-cycling, pure-CSS animation that **does** run
in an `<img>`: it animates visibility (stepped) and geometry — element position/size and
connection paths (smoothly, with reshaping connectors cross-fading). Use `--animate-css`
for SVGs you embed as images; `--animate` for standalone interactive viewing.
```

- [ ] **Step 2: Verify + commit**

Run: `cargo run --quiet -- --skill-animation 2>/dev/null | grep -i "animate-css is a self-cycling"`

```bash
git add docs/skill-animation.md
git commit -m "docs: --animate-css now animates geometry (use it for <img>-embedded SVGs)"
```

---

## Final verification

- [ ] `cargo test 2>&1 | grep -E "test result"` — all `ok`.
- [ ] `cargo clippy --all-targets 2>&1 | grep -cE "^warning: "` — `0`.
- [ ] `bash examples/render-all.sh` — green; agentic-loop-story still animates (visibility), token-prediction now animates (geometry).
- [ ] Open `examples/token-prediction.svg` as an image — full motion loops (box grows, tokens slide in, feed connector follows), matching the JS `--animate` preview.
