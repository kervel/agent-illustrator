# Connections Follow Moving Endpoints — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make connections follow a moving/resized endpoint across keyframes — slide (CSS `d:` morph) when the route keeps its shape, crossfade when it reshapes — landing correct at each keyframe, with no author work and no flicker.

**Architecture:** `resolve_frame_layout` already re-routes connections per frame; we stop discarding the result. `compute_frame_diffs` records the re-routed path (epsilon-gated) and whether it's morphable; `generate_keyframe_css` emits a per-frame `d:` morph or an opacity crossfade between path variants; a default `.ai-connection { transition: d, opacity }` drives the motion.

**Tech Stack:** Rust, SVG/CSS output. Tests via `cargo test`. Branch off `main`.

**Spec:** `docs/superpowers/specs/2026-06-10-connection-path-animation-design.md`

---

## File Structure

- `src/layout/keyframe.rs` — `ConnectionDiff` gains `path` + `morphable`; `compute_frame_diffs` computes connection path diffs by routing-order index (named → `conn-<name>` identity, unnamed → `conn-idx<N>`), epsilon-gated.
- `src/renderer/svg.rs` — extract `connection_path_d(...)` from `add_connection_path`; assign a stable `conn-<id>` class to every connection in the keyframe render loop; emit crossfade variant `<path>`s for non-morphable diffs; `generate_keyframe_css` emits `d:` morph rules, crossfade opacity rules, and the `.ai-connection` transition base rule.
- `tests/connection_animation.rs` — new integration tests.
- `docs/grammar.md`, `docs/skill-animation.md` — one note each.

**Identity rule (used identically on both sides):** for a connection at routing-order index `i`, its identity is `conn.name` if present, else `format!("idx{}", i)`; its CSS class token is `conn-<identity>`. `route_connections` iterates the document deterministically, so `base.connections[i]` and `solved.connections[i]` are the same connection.

---

## Phase 1: Morphable path animation (the slide)

### Task 1: `ConnectionDiff` carries path + morphable; compute it

**Files:**
- Modify: `src/layout/keyframe.rs` (`ConnectionDiff` ~L68, `compute_frame_diffs` connection block ~L255)
- Test: `src/layout/keyframe.rs` test module

- [ ] **Step 1: Write the failing unit test** — append to `mod tests` in `src/layout/keyframe.rs`:

```rust
#[test]
fn connection_path_diff_recorded_when_endpoint_moves() {
    use crate::parser::parse;
    use crate::layout::{layout_document, config::LayoutConfig};
    let doc = parse(r#"
rect box [width: 100, height: 50]
rect other [width: 100, height: 50]
constrain box.center_x = 150
constrain box.center_y = 100
constrain other.center_x = 450
constrain other.center_y = 100
box.right -> other.left as feed
keyframe "idle" {}
keyframe "grow" { transform box [width: 260] }
"#).expect("parse");
    let cfg = LayoutConfig::default();
    let result = layout_document(&doc, &cfg).expect("layout");
    let states = compute_frame_states(&extract_keyframes(&doc));
    let diffs = compute_frame_diffs(&result, &states, &doc, &cfg);
    // The "grow" frame must record a moved path for connection "feed".
    let grow = diffs.iter().find(|f| f.name == "grow").expect("grow frame");
    let cd = grow.connection_diffs.get("feed").expect("feed conn diff in grow");
    assert!(cd.path.is_some(), "feed should have a re-routed path diff, got {:?}", cd);
}
```

NOTE: confirm the layout entry point name/signature — grep `pub fn layout_document` (or how other tests in this file build a `LayoutResult` from a `Document`); mirror that exactly. If the helper differs, adjust the two setup lines only; keep the assertions.

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --lib connection_path_diff_recorded_when_endpoint_moves 2>&1 | tail -20`
Expected: FAIL to compile — `ConnectionDiff` has no `path` field.

- [ ] **Step 3a: Extend `ConnectionDiff`** — in `src/layout/keyframe.rs`:

```rust
/// Diff for a connection between frame N and frame 0
#[derive(Debug, Clone)]
pub struct ConnectionDiff {
    pub opacity: Option<f64>,
    /// Re-routed path when it differs from frame 0 (epsilon-gated).
    pub path: Option<Vec<crate::layout::types::Point>>,
    /// True when the re-routed path has the same segment count as frame 0
    /// (so a CSS `d:` morph can interpolate it); false → crossfade fallback.
    pub morphable: bool,
}
```

- [ ] **Step 3b: Compute connection identity + path diffs** — replace the connection-diff block in `compute_frame_diffs` (the `for name in base_connections.keys() { ... }` loop). Build identity-keyed base + solved maps and diff paths. Full replacement for that block:

```rust
        // Connection diffs: visibility (opacity) + geometry (re-routed path).
        // Identity = name if present, else idx<routing-order index>. base.connections[i]
        // and solved.connections[i] are the same connection (deterministic routing).
        let conn_identity = |i: usize, c: &ConnectionLayout| -> String {
            c.name.as_ref().map(|n| n.0.clone()).unwrap_or_else(|| format!("idx{}", i))
        };
        let solved_conns: Option<&Vec<ConnectionLayout>> =
            solved_result.as_ref().map(|s| &s.connections);

        for (i, base_conn) in base_result.connections.iter().enumerate() {
            let id = conn_identity(i, base_conn);

            // Visibility (named connections only carry hidden state).
            let mut opacity = None;
            if let Some(name) = &base_conn.name {
                let hidden0 = frame_states[0].hidden_connections.contains(&name.0);
                let hidden_now = state.hidden_connections.contains(&name.0);
                if hidden0 != hidden_now {
                    opacity = Some(if hidden_now { 0.0 } else { 1.0 });
                }
            }

            // Geometry: compare the solved (re-routed) path to the base path.
            let mut path = None;
            let mut morphable = true;
            if let Some(solved) = solved_conns {
                if let Some(sc) = solved.get(i) {
                    if paths_differ(&base_conn.path, &sc.path) {
                        morphable = base_conn.path.len() == sc.path.len()
                            && base_conn.routing_mode == sc.routing_mode;
                        path = Some(sc.path.clone());
                    }
                }
            }

            if opacity.is_some() || path.is_some() {
                connection_diffs.insert(id, ConnectionDiff { opacity, path, morphable });
            }
        }
```

Add the epsilon helper near `diff_element` in the same file:

```rust
/// True if any corresponding point differs by more than the sub-pixel threshold,
/// or the point counts differ. Sub-pixel solver noise produces no diff (anti-flicker).
fn paths_differ(a: &[crate::layout::types::Point], b: &[crate::layout::types::Point]) -> bool {
    let eps = 0.1;
    if a.len() != b.len() {
        return true;
    }
    a.iter().zip(b.iter()).any(|(p, q)| (p.x - q.x).abs() > eps || (p.y - q.y).abs() > eps)
}
```

(Remove the now-unused `base_connections` HashMap if nothing else uses it — grep `base_connections` in this function. `ConnectionLayout` is already imported at the top of keyframe.rs.)

- [ ] **Step 4: Run to verify passing**

Run: `cargo test --lib connection_path_diff_recorded_when_endpoint_moves 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 5: Fix the renderer's `ConnectionDiff` construction site (if any) + compile**

Run: `cargo build 2>&1 | grep -E "error" | head`
Any `ConnectionDiff { opacity: ... }` literal elsewhere needs `path: None, morphable: true` added. Grep `ConnectionDiff {` to find them. Expected after fixes: clean build.

- [ ] **Step 6: Commit**

```bash
git add src/layout/keyframe.rs
git commit -m "feat(keyframe): record re-routed connection path diffs (epsilon-gated)"
```

---

### Task 2: Extract `connection_path_d`; class every connection with a stable id

**Files:**
- Modify: `src/renderer/svg.rs` (`add_connection_path` ~L417, `render_connection` ~L1270, keyframe render loop ~L765)

- [ ] **Step 1: Extract the `d`-string builder** — pull the marker-pullback + routing-mode `d` construction out of `add_connection_path` into a free function, and have `add_connection_path` call it. New function (place above `add_connection_path` impl or as a module fn):

```rust
/// Build the SVG path `d` string for a connection, including the arrow-marker
/// pullback, matching exactly what `add_connection_path` renders.
fn connection_path_d(path: &[Point], routing_mode: RoutingMode, marker_end: bool, stroke_width: f64) -> String {
    let path = if marker_end && path.len() >= 2 {
        let mut shortened = path.to_vec();
        let last_idx = shortened.len() - 1;
        let prev_idx = last_idx - 1;
        let dx = shortened[last_idx].x - shortened[prev_idx].x;
        let dy = shortened[last_idx].y - shortened[prev_idx].y;
        let len = (dx * dx + dy * dy).sqrt();
        if len > 0.001 {
            let pullback = 3.6 * stroke_width;
            shortened[last_idx].x -= dx / len * pullback;
            shortened[last_idx].y -= dy / len * pullback;
        }
        shortened
    } else {
        path.to_vec()
    };
    match routing_mode {
        RoutingMode::Curved if path.len() >= 4 => {
            let mut d = format!(
                "M{} {} C{} {} {} {} {} {}",
                path[0].x, path[0].y, path[1].x, path[1].y, path[2].x, path[2].y, path[3].x, path[3].y
            );
            for chunk in path[4..].chunks(3) {
                if chunk.len() == 3 {
                    d.push_str(&format!(" C{} {} {} {} {} {}", chunk[0].x, chunk[0].y, chunk[1].x, chunk[1].y, chunk[2].x, chunk[2].y));
                } else if chunk.len() == 2 {
                    d.push_str(&format!(" Q{} {} {} {}", chunk[0].x, chunk[0].y, chunk[1].x, chunk[1].y));
                } else if chunk.len() == 1 {
                    d.push_str(&format!(" L{} {}", chunk[0].x, chunk[0].y));
                }
            }
            d
        }
        _ => path_to_d(&path),
    }
}
```

Then in `add_connection_path`, replace the inline `let path = …; let d = match routing_mode { … };` block with:

```rust
        let d = connection_path_d(path, routing_mode, marker_end, stroke_width);
```

- [ ] **Step 2: Build/verify no behavior change**

Run: `cargo build 2>&1 | grep -E "error" | head` (expect none), then
Run: `cargo test --test svg_regression 2>&1 | grep -E "test result"` — connection `d` output is byte-identical, so regression stays green.

- [ ] **Step 3: Thread a stable connection id into `render_connection`** — change the keyframe render loop (in `render_svg_with_keyframes`) to enumerate and compute identity, and pass it down. Replace the loop:

```rust
    for (i, conn) in result.connections.iter().enumerate() {
        let id = conn.name.as_ref().map(|n| n.0.clone()).unwrap_or_else(|| format!("idx{}", i));
        let hidden0 = conn.name.as_ref().map_or(false, |n| frame0_hidden_conns.contains(&n.0));
        if hidden0 {
            let mut hidden_conn = conn.clone();
            hidden_conn.styles.opacity = Some(0.0);
            render_connection(&hidden_conn, &mut builder, Some(&id));
            continue;
        }
        render_connection(conn, &mut builder, Some(&id));
    }
```

Update `render_connection`'s signature and class logic to use the passed id (falling back to the old name-based class when `None`, e.g. for the non-keyframe `render_svg` path which calls it without an id):

```rust
fn render_connection(conn: &ConnectionLayout, builder: &mut SvgBuilder, id: Option<&str>) {
    let mut classes = conn.styles.css_classes.clone();
    let conn_class = id.map(|s| s.to_string())
        .or_else(|| conn.name.as_ref().map(|n| n.0.clone()));
    if let Some(c) = &conn_class {
        classes.push(format!("conn-{}", c));
    }
    // … rest unchanged, but use `conn_class` for the label's extra class too:
```

In the label block, replace the `conn.name`-based `extra_classes` with `conn_class`:

```rust
        let extra_classes = conn_class.as_ref().map(|c| format!("conn-{}", c)).unwrap_or_default();
```

Find every other `render_connection(` call (grep) — the non-keyframe path(s) — and pass `None`.

- [ ] **Step 4: Build + full suite**

Run: `cargo build 2>&1 | grep -E "error" | head` (none), then `cargo test 2>&1 | grep -E "test result: ok" | wc -l` (all suites ok). svg_regression may show structural diffs only if unnamed connections newly get `conn-idx<N>` — that's expected; if the structure test is byte-strict on connections, update its baseline per the test's documented refresh step.

- [ ] **Step 5: Commit**

```bash
git add src/renderer/svg.rs
git commit -m "refactor(svg): extract connection_path_d; give every connection a stable conn-<id> class"
```

---

### Task 3: Emit `d:` morph + transition; the slide

**Files:**
- Modify: `src/renderer/svg.rs` (`generate_keyframe_css` connection block ~L879, base rules ~L800, `--no-frame-css` branch ~L678)
- Test: `tests/connection_animation.rs` (new)

- [ ] **Step 1: Write failing integration tests** — create `tests/connection_animation.rs`:

```rust
use agent_illustrator::render;

const SRC_TRANSFORM: &str = r#"
rect box [width: 100, height: 50]
rect other [width: 100, height: 50]
constrain box.center_x = 150
constrain box.center_y = 100
constrain other.center_x = 450
constrain other.center_y = 100
box.right -> other.left as feed
keyframe "idle" {}
keyframe "grow" { transform box [width: 260] }
"#;

#[test]
fn connection_follows_transformed_endpoint() {
    let svg = render(SRC_TRANSFORM).expect("render");
    // grow frame: feed re-anchors to the new right edge (box.right moves 200 -> 280;
    // exact x depends on marker pullback, so just assert a per-frame d: rule exists for feed
    // with a start x well past the old 200 edge).
    assert!(svg.contains(".conn-feed { d: path("), "expected d: morph rule for feed, got:\n{}", svg);
}

#[test]
fn connection_transition_rule_present() {
    let svg = render(SRC_TRANSFORM).expect("render");
    assert!(svg.contains(".ai-connection { transition: d 0.5s ease, opacity 0.5s ease;"),
        "expected .ai-connection transition rule, got:\n{}", svg);
}

#[test]
fn connection_follows_constraint_change() {
    let svg = render(r#"
rect box [width: 100, height: 50]
rect other [width: 100, height: 50]
constrain box.left = 80
constrain box.center_y = 100
constrain box.width = 190 as w0
constrain other.center_x = 450
constrain other.center_y = 100
box.right -> other.left as feed
keyframe "idle" {}
keyframe "grow" { disable w0; constrain box.width = 300 }
"#).expect("render");
    assert!(svg.contains(".conn-feed { d: path("),
        "constraint-driven widen should move the feed connection, got:\n{}", svg);
}

#[test]
fn static_connection_does_not_flicker() {
    // feed connects two static elements; only an unrelated element moves.
    let svg = render(r#"
rect a [width: 60, height: 30]
rect b [width: 60, height: 30]
rect mover [width: 40, height: 20]
constrain a.center_x = 100
constrain a.center_y = 60
constrain b.center_x = 400
constrain b.center_y = 60
constrain mover.center_x = 250
constrain mover.center_y = 200
a.right -> b.left as feed
keyframe "idle" {}
keyframe "go" { transform mover [dx: 50] }
"#).expect("render");
    // feed never moves → no d: rule for it anywhere.
    assert!(!svg.contains(".conn-feed { d:"),
        "static connection must not get a path diff (anti-flicker), got:\n{}", svg);
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test connection_animation 2>&1 | tail -20`
Expected: `connection_follows_*` and `transition_rule_present` FAIL (no `d:`/transition emitted yet); `static_connection_does_not_flicker` likely PASSES already (nothing emits `d:`).

- [ ] **Step 3a: Emit the transition base rule** — in `generate_keyframe_css`, after the `.ai-shape { … }` line, add:

```rust
    css.push_str(".ai-connection { transition: d 0.5s ease, opacity 0.5s ease; }\n");
```

And append the same line to the `--no-frame-css` branch string in `render_svg_with_keyframes`.

- [ ] **Step 3b: Emit per-frame `d:` for morphable connection diffs** — `ConnectionDiff` stores `path: Vec<Point>` (from Task 1), not a `d` string — so the solver layer stays free of SVG concerns. `generate_keyframe_css` rebuilds the `d` string with `connection_path_d`, using per-connection routing/marker/stroke from a `conn_meta` lookup passed in (built in Step 3c). Replace the connection-diff loop in `generate_keyframe_css` with:

```rust
        for (id, diff) in &frame.connection_diffs {
            if let Some(op) = diff.opacity {
                css.push_str(&format!("  .conn-{} {{ opacity: {}; }}\n", id, op));
            }
            if let Some(pts) = &diff.path {
                if diff.morphable {
                    if let Some((routing, marker, sw)) = conn_meta.get(id) {
                        let d = connection_path_d(pts, *routing, *marker, *sw);
                        css.push_str(&format!("  .conn-{} {{ d: path(\"{}\"); }}\n", id, d));
                    }
                }
                // non-morphable handled by the crossfade branch in Task 4 Step 3b
            }
        }
```

`conn_meta: &std::collections::HashMap<String, (RoutingMode, bool, f64)>` is a new parameter on `generate_keyframe_css` (added in Step 3c).

- [ ] **Step 3c: Pass the connection meta map** — in `render_svg_with_keyframes`, before calling `generate_keyframe_css`, build:

```rust
    let conn_meta: std::collections::HashMap<String, (RoutingMode, bool, f64)> = result
        .connections
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let id = c.name.as_ref().map(|n| n.0.clone()).unwrap_or_else(|| format!("idx{}", i));
            let marker = matches!(c.direction, ConnectionDirection::Forward | ConnectionDirection::Bidirectional);
            (id, (c.routing_mode, marker, c.styles.stroke_width.unwrap_or(2.0)))
        })
        .collect();
```

and thread it into `generate_keyframe_css(frame_states, frame_diffs, &conn_meta)` (update the signature and the `no_frame_css` call site; under `no_frame_css` the function isn't called — only the base string is used — so no change there beyond the appended transition rule).

- [ ] **Step 4: Run to verify passing**

Run: `cargo test --test connection_animation 2>&1 | tail -20`
Expected: all four PASS.

- [ ] **Step 5: Full suite + clippy**

Run: `cargo test 2>&1 | grep -cE "test result: ok"` (all), `cargo clippy --all-targets 2>&1 | grep -cE "^warning: "` (0).

- [ ] **Step 6: Commit**

```bash
git add src/renderer/svg.rs tests/connection_animation.rs
git commit -m "feat(keyframe): connections slide to follow moving endpoints (CSS d: morph + transition)"
```

---

## Phase 2: Crossfade fallback (route reshapes)

### Task 4: Variant paths + opacity crossfade for non-morphable diffs

**Files:**
- Modify: `src/renderer/svg.rs` (`render_svg_with_keyframes` render loop, `generate_keyframe_css`)
- Test: `tests/connection_animation.rs`

- [ ] **Step 1: Write the failing test** — append to `tests/connection_animation.rs`. Use a connection whose route changes segment count between frames (orthogonal route that gains a bend when an endpoint moves past the other). Construct a case where base path and grow path have different point counts:

```rust
#[test]
fn connection_crossfades_when_route_reshapes() {
    // Orthogonal routing: moving the endpoint forces an extra bend → different segment
    // count → not morphable → crossfade variant emitted instead of a d: rule.
    let svg = render(r#"
rect box [width: 80, height: 40]
rect other [width: 80, height: 40]
constrain box.center_x = 120
constrain box.center_y = 100
constrain other.center_x = 120
constrain other.center_y = 320
box.bottom -> other.top as link [routing: orthogonal]
keyframe "idle" {}
keyframe "shift" { transform box [dx: 220] }
"#).expect("render");
    // Non-morphable → a variant path classed for the shift frame, toggled by opacity,
    // and NOT a `.conn-link { d: ... }` morph rule.
    let has_variant = svg.contains("conn-link-fshift");
    let has_morph = svg.contains(".conn-link { d:");
    assert!(has_variant && !has_morph,
        "reshaped route should crossfade (variant + opacity), not morph. variant={} morph={}\n{}",
        has_variant, has_morph, svg);
}
```

NOTE: verify this input actually yields different segment counts (render it and inspect); if orthogonal routing keeps the same count, adjust the geometry (e.g. move the endpoint so the router must insert/remove a bend) until base vs shift paths differ in `path.len()`. The assertion (variant emitted, no morph) is the invariant to keep.

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test connection_animation connection_crossfades_when_route_reshapes 2>&1 | tail -20`
Expected: FAIL — no variant emitted; today a non-morphable diff produces nothing (Task 3 only emits `d:` for morphable).

- [ ] **Step 3a: Render variant paths** — in `render_svg_with_keyframes`, after the main connection render loop, emit one variant `<path>` per non-morphable connection diff. For each frame in `frame_diffs`, for each `(id, diff)` where `diff.path.is_some() && !diff.morphable`, render a hidden variant path:

```rust
    for frame in frame_diffs {
        for (id, diff) in &frame.connection_diffs {
            if let Some(pts) = &diff.path {
                if !diff.morphable {
                    if let Some((routing, marker, sw)) = conn_meta.get(id) {
                        let d = connection_path_d(pts, *routing, *marker, *sw);
                        // hidden by default; shown only in its frame via CSS
                        builder.add_connection_variant(&d, id, &frame.name, *marker);
                    }
                }
            }
        }
    }
```

Add the builder method (mirrors `add_connection_path` output but with the variant classes and inline `opacity:0`, and a fixed style; reuse the connection's stroke via a passed style string if needed — minimal version uses default stroke, since the base path already carries the real style and the variant only shows during the crossfade):

```rust
    pub fn add_connection_variant(&mut self, d: &str, id: &str, frame: &str, marker_end: bool) {
        let prefix = self.prefix();
        let marker = if marker_end { format!(r#" marker-end="url(#{prefix}arrow)""#) } else { String::new() };
        self.connections.push(format!(
            r#"{}<path class="{}connection conn-{} conn-{}-f{}" d="{}" fill="none" opacity="0"{}/>"#,
            self.indent_str(), prefix, id, id, frame, d, marker
        ));
    }
```

(If preserving the connection's real stroke on the variant matters visually, pass and append the connection's `format_connection_styles` string; the minimal version above relies on the default `.ai-connection` stroke from the stylesheet.)

- [ ] **Step 3b: Emit crossfade opacity rules** — in `generate_keyframe_css`, for a non-morphable connection diff, instead of a `d:` rule emit: hide the base path and show the variant in that frame. In the connection-diff loop:

```rust
            if let Some(pts) = &diff.path {
                if diff.morphable {
                    // … existing d: emission …
                } else {
                    // crossfade: base fades out, this frame's variant fades in
                    css.push_str(&format!("  .conn-{}:not([class*=\"-f\"]) {{ opacity: 0; }}\n", id));
                    css.push_str(&format!("  .conn-{}-f{} {{ opacity: 1; }}\n", id, frame.name));
                }
            }
```

NOTE: the base path must remain visible in frames where this connection is NOT reshaped. Verify the selector only zeroes the base within the reshaped frame's `.frame-<name>` block (it is nested inside it). The `:not([class*="-f"])` targets the base path (which lacks a `-f<frame>` class) vs variants. If that selector proves brittle, give the base path an explicit `conn-<id>-base` class in `render_connection` and target `.conn-<id>-base { opacity: 0 }` instead — prefer this explicit form.

- [ ] **Step 3c (recommended): explicit base class** — to avoid the `:not()` selector, add `conn-<id>-base` to the base connection path in `render_connection` (alongside `conn-<id>`), and use `.conn-<id>-base { opacity: 0 }` in 3b.

- [ ] **Step 4: Run to verify passing**

Run: `cargo test --test connection_animation 2>&1 | tail -20`
Expected: all PASS (including crossfade).

- [ ] **Step 5: Full suite + clippy**

Run: `cargo test 2>&1 | grep -cE "test result: ok"`; `cargo clippy --all-targets 2>&1 | grep -cE "^warning: "` (0).

- [ ] **Step 6: Commit**

```bash
git add src/renderer/svg.rs tests/connection_animation.rs
git commit -m "feat(keyframe): crossfade connection paths when the route reshapes (non-morphable)"
```

---

## Phase 3: Docs

### Task 5: Document connection following

**Files:**
- Modify: `docs/skill-animation.md`, `docs/grammar.md`

- [ ] **Step 1: skill-animation.md** — in the geometry/animation section, add:

```markdown
### Connections follow automatically

Connections re-anchor to their endpoints every keyframe — move or resize an element and
any connection touching it follows on its own (no extra constraints, no naming required).
It slides smoothly when the route keeps its shape (Chrome/Safari) and crossfades when the
route reshapes; either way it's correct at each keyframe. (Firefox holds a sliding
connector at its start route; reshaped/crossfaded ones animate everywhere.)
```

- [ ] **Step 2: grammar.md** — near the keyframe/connection notes, add one line:

```markdown
Connections automatically follow moving/resized endpoints across keyframes (CSS d:
morph where the route shape is unchanged, opacity crossfade where it reshapes).
```

- [ ] **Step 3: Verify via CLI**

Run: `cargo run --quiet -- --skill-animation 2>/dev/null | grep -i "Connections follow"` and `cargo run --quiet -- --grammar 2>/dev/null | grep -i "automatically follow"`.

- [ ] **Step 4: Commit**

```bash
git add docs/skill-animation.md docs/grammar.md
git commit -m "docs: connections follow moving endpoints (slide/crossfade)"
```

---

## Final verification

- [ ] `cargo test 2>&1 | grep -E "test result"` — all `ok`.
- [ ] `cargo clippy --all-targets 2>&1 | grep -cE "^warning: "` — `0`.
- [ ] `bash examples/render-all.sh` — green.
- [ ] Manual: render the token-prediction example (or the Task-3 SRC) with `--animate`, open in Chrome, confirm the arrow's start tracks the box's right edge as it grows and the path glides (no crossing the box).
