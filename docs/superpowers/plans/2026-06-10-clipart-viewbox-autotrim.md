# Clipart viewBox Auto-Trim — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Trim each file-based SVG clipart's element box to the artwork's tight content bounding box (via `usvg`), so anchors hug the drawing and connectors are consistent — on by default, with graceful fallback to the raw viewBox and a `[trim: false]` escape.

**Architecture:** At template load, compute the content bbox `(minX,minY,w,h)` with `usvg` and cache it alongside the raw viewBox dims. The resolver picks trimmed-vs-raw per the `trim` modifier (default on) and bakes a content offset into the `SvgEmbed` shape; the renderer prepends `translate(-minX,-minY)` so the drawing fills the element rect. Any parse error / degenerate bbox falls back to the raw viewBox.

**Tech Stack:** Rust, `usvg` (new dep), SVG output. Tests via `cargo test`. Branch off `main`.

**Spec:** `docs/superpowers/specs/2026-06-10-clipart-viewbox-autotrim-design.md`

---

## File Structure

- `Cargo.toml` — add `usvg` (pinned).
- `src/template/registry.rs` — `svg_content_bbox()` helper; `TemplateDefinition` gains `svg_trimmed: Option<(f64,f64,f64,f64)>`; `load_svg_template` computes it.
- `src/parser/ast.rs` — `ShapeType::SvgEmbed` gains `offset_x: f64, offset_y: f64`.
- `src/template/resolver.rs` — pick trimmed vs raw per `[trim]` modifier (default true); set intrinsic dims + offset.
- `src/renderer/svg.rs` — `SvgEmbed` transform prepends `translate(-offset_x,-offset_y)`.
- `tests/clipart_trim.rs` — integration tests.
- `docs/skill-find-clipart.md`, `docs/grammar.md` — doc updates.

---

## Task 1: usvg content-bbox helper (with fallback semantics)

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/template/registry.rs`
- Test: `src/template/registry.rs` test module

- [ ] **Step 1: Add the dependency**

In `Cargo.toml` under `[dependencies]`, add (pin to whatever the registry resolves; verify the bbox API in Step 3):

```toml
usvg = "0.45"
```

Run: `cargo build 2>&1 | tail -3` to fetch and confirm it resolves. If 0.45 fails to build, try the latest 0.4x and note the version.

- [ ] **Step 2: Write the failing test** — append to the `#[cfg(test)] mod tests` in `src/template/registry.rs`:

```rust
#[test]
fn content_bbox_trims_fat_margin() {
    // 100x100 viewBox, drawing is a 20x20 rect centered at (40,40)..(60,60).
    let svg = r#"<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg"><rect x="40" y="40" width="20" height="20" fill="black"/></svg>"#;
    let bbox = svg_content_bbox(svg, 100.0, 100.0).expect("bbox computed");
    let (x, y, w, h) = bbox;
    assert!((x - 40.0).abs() < 1.0, "min_x ~40, got {}", x);
    assert!((y - 40.0).abs() < 1.0, "min_y ~40, got {}", y);
    assert!((w - 20.0).abs() < 1.0, "w ~20, got {}", w);
    assert!((h - 20.0).abs() < 1.0, "h ~20, got {}", h);
}

#[test]
fn content_bbox_fallback_on_unparseable() {
    // Not valid SVG → None (caller keeps raw viewBox).
    assert!(svg_content_bbox("not an svg", 100.0, 100.0).is_none());
}

#[test]
fn content_bbox_rejects_degenerate_tiny() {
    // Drawing area < 5% of viewBox → None (guards against usvg dropping elements).
    let svg = r#"<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg"><rect x="0" y="0" width="2" height="2" fill="black"/></svg>"#;
    assert!(svg_content_bbox(svg, 100.0, 100.0).is_none(), "2x2 in 100x100 is <5% area");
}
```

- [ ] **Step 3: Run to verify failure**

Run: `cargo test --lib content_bbox 2>&1 | tail -15`
Expected: FAIL to compile — `svg_content_bbox` not defined.

- [ ] **Step 4: Implement the helper** — add to `src/template/registry.rs` (module level). VERIFY the bbox API for the resolved `usvg` version — recent usvg exposes the laid-out tree root's absolute bbox; the call is one of `tree.root().abs_bounding_box()` (returns `usvg::Rect`, fields `.x() .y() .width() .height()`) or `tree.root().bounding_box()`. Adjust the three lines that read the rect to match; keep the surrounding fallback logic:

```rust
/// Tight content bounding box (min_x, min_y, w, h) of an SVG's artwork, in viewBox
/// coordinates. Returns None when the SVG can't be parsed or the bbox is degenerate /
/// implausible (caller falls back to the raw viewBox). Best-effort: never panics.
pub fn svg_content_bbox(svg: &str, viewbox_w: f64, viewbox_h: f64) -> Option<(f64, f64, f64, f64)> {
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg, &opt).ok()?;

    // --- VERSION-SENSITIVE: read the laid-out content bbox of the root group ---
    let bb = tree.root().abs_bounding_box();
    let (mut x, mut y, mut w, mut h) = (bb.x() as f64, bb.y() as f64, bb.width() as f64, bb.height() as f64);
    // ---------------------------------------------------------------------------

    if !(x.is_finite() && y.is_finite() && w.is_finite() && h.is_finite()) || w <= 0.0 || h <= 0.0 {
        return None;
    }
    // Clamp inside the viewBox (never larger; strokes can spill slightly).
    let x1 = (x).max(0.0);
    let y1 = (y).max(0.0);
    let x2 = (x + w).min(viewbox_w);
    let y2 = (y + h).min(viewbox_h);
    x = x1; y = y1; w = (x2 - x1).max(0.0); h = (y2 - y1).max(0.0);
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    // Reject implausibly small artwork (usvg likely dropped elements it couldn't
    // parse): falling back to the viewBox is the safe direction.
    let vb_area = viewbox_w * viewbox_h;
    if vb_area > 0.0 && (w * h) / vb_area < 0.05 {
        return None;
    }
    Some((x, y, w, h))
}
```

- [ ] **Step 5: Run to verify passing**

Run: `cargo test --lib content_bbox 2>&1 | tail -15`
Expected: all three PASS. (If the fat-margin test's numbers are off by the usvg layout model, adjust tolerances; if `content_bbox_rejects_degenerate_tiny` passes only because of parse behavior, keep the 5% guard as the cause.)

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock src/template/registry.rs
git commit -m "feat(template): usvg-based content bbox helper for clipart trimming"
```

---

## Task 2: Compute & cache the trimmed bbox at load

**Files:**
- Modify: `src/template/registry.rs` (`TemplateDefinition` struct + its initializer; `load_svg_template`)
- Test: `src/template/registry.rs` test module

- [ ] **Step 1: Write the failing test** — append to `mod tests`:

```rust
#[test]
fn load_svg_template_caches_trimmed_bbox() {
    use std::io::Write;
    let dir = std::env::temp_dir().join("ail_trim_test");
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("icon.svg");
    std::fs::write(&p, r#"<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg"><rect x="40" y="40" width="20" height="20"/></svg>"#).unwrap();

    let mut reg = TemplateRegistry::new(); // adjust to the real constructor (grep `TemplateRegistry::`)
    // register a file SVG template named "icon" pointing at p, then:
    reg.load_svg_template("icon").unwrap();
    let def = reg.get("icon").unwrap();
    let (_, _, w, h) = def.svg_trimmed.expect("trimmed bbox cached");
    assert!((w - 20.0).abs() < 1.0 && (h - 20.0).abs() < 1.0);
}
```

NOTE: wire up the registration the way other registry tests do (grep `fn register`, `TemplateRegistry::` in this file's tests). If registry tests are sparse, build the `TemplateDefinition` directly and insert it, then call `load_svg_template`. Keep the assertion on `svg_trimmed`.

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --lib load_svg_template_caches_trimmed_bbox 2>&1 | tail -15`
Expected: FAIL — no `svg_trimmed` field.

- [ ] **Step 3a: Add the field** — in `TemplateDefinition` (`src/template/registry.rs`, struct at ~L55), after `svg_dimensions`:

```rust
    /// Content bbox (min_x, min_y, w, h) for trimming the viewBox margin; None if not
    /// computed or trimming would degrade (caller falls back to svg_dimensions).
    pub svg_trimmed: Option<(f64, f64, f64, f64)>,
```

In the initializer (the `TemplateDefinition { ... svg_dimensions: None, ... }` literal at ~L106), add `svg_trimmed: None,`.

- [ ] **Step 3b: Compute it in `load_svg_template`** — after `let dimensions = parse_svg_dimensions(&content);`, compute the trimmed bbox using the raw viewBox dims as the clamp reference:

```rust
        let dimensions = parse_svg_dimensions(&content);
        let trimmed = dimensions.and_then(|(w, h)| svg_content_bbox(&content, w, h));

        let def = self.templates.get_mut(name).unwrap();
        def.svg_content = Some(content);
        def.svg_dimensions = dimensions;
        def.svg_trimmed = trimmed;
```

(Replace the existing two-line `def.svg_content = …; def.svg_dimensions = …;` tail.)

- [ ] **Step 4: Run to verify passing**

Run: `cargo test --lib load_svg_template_caches_trimmed_bbox 2>&1 | tail -15`
Expected: PASS. Fix any other `TemplateDefinition { … }` literals the compiler flags (grep `TemplateDefinition {`) by adding `svg_trimmed: None,`.

- [ ] **Step 5: Commit**

```bash
git add src/template/registry.rs
git commit -m "feat(template): cache trimmed content bbox on SVG template load"
```

---

## Task 3: Apply the trim — offset on SvgEmbed + [trim] modifier + renderer

**Files:**
- Modify: `src/parser/ast.rs` (`ShapeType::SvgEmbed`)
- Modify: `src/template/resolver.rs` (`resolve_svg_template` ~L238)
- Modify: `src/renderer/svg.rs` (`SvgEmbed` render arm ~L1132)
- Test: `tests/clipart_trim.rs` (new)

- [ ] **Step 1: Write the failing integration test** — create `tests/clipart_trim.rs`. It writes a fat-margin clipart to a temp dir, references it, and checks the embed transform offsets the content (trim on by default) and that `[trim: false]` disables it:

```rust
use agent_illustrator::render_with_config;
use agent_illustrator::RenderConfig;
use std::io::Write;

fn write_clipart(dir: &std::path::Path) {
    std::fs::create_dir_all(dir.join("clipart")).unwrap();
    // drawing occupies (40,40)..(60,60) in a 100x100 viewBox (fat margin)
    let mut f = std::fs::File::create(dir.join("clipart/box.svg")).unwrap();
    f.write_all(br#"<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg"><rect x="40" y="40" width="20" height="20" fill="black"/></svg>"#).unwrap();
}

fn render_in(dir: &std::path::Path, src: &str) -> String {
    let mut cfg = RenderConfig::default();
    cfg.base_dir = Some(dir.to_path_buf()); // adjust to the real field used to resolve template paths (grep `base_dir`/`resolve_path`)
    render_with_config(src, cfg).expect("render")
}

#[test]
fn trim_on_by_default_offsets_content() {
    let dir = std::env::temp_dir().join("ail_trim_default");
    write_clipart(&dir);
    let svg = render_in(&dir, r#"
template "box_icon" from "clipart/box.svg"
box_icon icon [width: 40, height: 40]
constrain icon.center_x = 100
constrain icon.center_y = 100
"#);
    // Trimmed: the embed transform must shift the content by its bbox origin (-40,-40).
    assert!(svg.contains("translate(-40") , "expected content offset translate(-40,-40), got:\n{}", svg);
}

#[test]
fn trim_false_keeps_viewbox() {
    let dir = std::env::temp_dir().join("ail_trim_off");
    write_clipart(&dir);
    let svg = render_in(&dir, r#"
template "box_icon" from "clipart/box.svg"
box_icon icon [width: 40, height: 40, trim: false]
constrain icon.center_x = 100
constrain icon.center_y = 100
"#);
    assert!(!svg.contains("translate(-40"), "trim:false must not offset content, got:\n{}", svg);
}
```

NOTE: verify `RenderConfig`'s field for the template base directory (grep `base_dir`, `resolve_path`, how existing template/embedded-images tests set it). If rendering needs a real on-disk relative path, mirror how `tests/integration_tests.rs` exercises file templates. Keep the two assertions (offset present by default; absent with `trim:false`).

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test clipart_trim 2>&1 | tail -20`
Expected: FAIL — `SvgEmbed` has no offset; transform never contains the inner translate.

- [ ] **Step 3a: Add offset fields to `SvgEmbed`** — in `src/parser/ast.rs`, the `ShapeType::SvgEmbed` variant:

```rust
    SvgEmbed {
        content: String,
        intrinsic_width: Option<f64>,
        intrinsic_height: Option<f64>,
        /// Content bbox origin to subtract so the artwork fills the element rect
        /// (0,0 when not trimmed). 
        offset_x: f64,
        offset_y: f64,
    },
```

Fix every constructor/match of `SvgEmbed` the compiler flags (grep `SvgEmbed`): the resolver constructor (Task 3b) and the renderer arm (Task 3c) are the real ones; any match-only sites add `offset_x: _, offset_y: _` or `..`.

- [ ] **Step 3b: Pick trimmed-vs-raw in the resolver** — in `src/template/resolver.rs` `resolve_svg_template`, replace the dims/shape construction (the `let (width, height) = def.svg_dimensions.unwrap_or((100.0, 100.0));` + `ShapeType::SvgEmbed { … }`) with:

```rust
    // Trim is on by default; `[trim: false]` on the instance disables it.
    let trim_enabled = read_trim_flag(instance_modifiers);
    let ((width, height), (offset_x, offset_y)) = match (trim_enabled, def.svg_trimmed) {
        (true, Some((mx, my, w, h))) => ((w, h), (mx, my)),
        _ => (def.svg_dimensions.unwrap_or((100.0, 100.0)), (0.0, 0.0)),
    };

    let shape = ShapeDecl {
        shape_type: Spanned::new(
            ShapeType::SvgEmbed {
                content,
                intrinsic_width: Some(width),
                intrinsic_height: Some(height),
                offset_x,
                offset_y,
            },
            span.clone(),
        ),
        name: Some(Spanned::new(Identifier::new(instance_name), span.clone())),
        modifiers: instance_modifiers.to_vec(),
    };
```

Add the `read_trim_flag` helper in `resolver.rs` (module level). `trim` parses as `StyleKey::Custom("trim")` with a keyword/identifier value; default true:

```rust
/// Read `[trim: false]` from instance modifiers (default true). Accepts keyword/ident
/// `true`/`false`.
fn read_trim_flag(modifiers: &[Spanned<StyleModifier>]) -> bool {
    use crate::parser::ast::{StyleKey, StyleValue};
    for m in modifiers {
        let is_trim = matches!(&m.node.key.node, StyleKey::Custom(k) if k == "trim");
        if is_trim {
            return match &m.node.value.node {
                StyleValue::Keyword(s) | StyleValue::Identifier(crate::parser::ast::Identifier(s)) => s != "false",
                StyleValue::Number { value, .. } => *value != 0.0,
                _ => true,
            };
        }
    }
    true
}
```

(Verify how `false`/`true` lex: they may be `StyleValue::Keyword` or `Identifier`. Grep how other boolean-ish modifiers like `no_resolve`/keywords parse; adjust the match arm. The instance still keeps `trim` in its modifiers — harmless, ignored downstream as a Custom key.)

- [ ] **Step 3c: Apply the offset in the renderer** — in `src/renderer/svg.rs`, the `SvgEmbed` arm, bind the new fields and append the content-offset translate to each `transform` string so it runs first (right-most):

```rust
        ElementType::Shape(ShapeType::SvgEmbed {
            content,
            intrinsic_width,
            intrinsic_height,
            offset_x,
            offset_y,
        }) => {
```

Then change the three `format!` transforms to end with `translate(-offset_x, -offset_y)`. For the no-rotation branches:

```rust
                format!(
                    "translate({}, {}) scale({}, {}) translate({}, {})",
                    element.bounds.x, element.bounds.y, scale_x, scale_y, -offset_x, -offset_y
                )
```

and the rotation branch keeps `rotate(...)` then the offset translate:

```rust
                    format!(
                        "translate({}, {}) scale({}, {}) rotate({} {} {}) translate({}, {})",
                        element.bounds.x, element.bounds.y, scale_x, scale_y, rotation, cx, cy, -offset_x, -offset_y
                    )
```

(When `offset_x/​y == 0` the appended `translate(0, 0)` is a harmless no-op, so non-trimmed and inline-SVG embeds are unchanged. Optionally skip emitting it when both are 0 to keep output clean — preferred.)

- [ ] **Step 4: Run to verify passing**

Run: `cargo test --test clipart_trim 2>&1 | tail -20`
Expected: both PASS.

- [ ] **Step 5: Full suite + clippy + examples**

Run: `cargo test 2>&1 | grep -E "test result: ok" | wc -l` (all), `cargo clippy --all-targets 2>&1 | grep -cE "^warning: "` (0), `bash examples/render-all.sh 2>&1 | grep -i fail || echo ok`. The embedded-images example uses SVG templates — confirm it still renders; its SVGs may re-anchor slightly (expected, that's the feature).

- [ ] **Step 6: Commit**

```bash
git add src/parser/ast.rs src/template/resolver.rs src/renderer/svg.rs tests/clipart_trim.rs
git commit -m "feat(clipart): trim SVG viewBox to artwork (default on, [trim: false] escape)"
```

---

## Task 4: Docs

**Files:**
- Modify: `docs/skill-find-clipart.md` (Part 2b), `docs/grammar.md`

- [ ] **Step 1: skill-find-clipart.md** — replace the body of `### 2b. Tighten the viewBox to the artwork (scale prep)` with:

```markdown
### 2b. viewBox margin is trimmed automatically

agent-illustrator trims each file SVG template's element box to the artwork's content
bounding box on import (so anchors hug the drawing and connectors land consistently) —
you don't need to crop by hand. If a specific icon trims wrong (e.g. usvg couldn't fully
parse it, so it fell back to the raw viewBox, or the bbox came out wrong), pass
`[trim: false]` on that instance and crop it manually. The artwork's content still needs
a sensible viewBox for the fallback path.
```

- [ ] **Step 2: grammar.md** — near the template/clipart notes, add:

```markdown
File SVG templates auto-trim their viewBox to the artwork's content bbox (so anchors hug
the drawing). Disable per instance with `[trim: false]`.
```

- [ ] **Step 3: Verify + commit**

Run: `cargo run --quiet -- --skill-find-clipart 2>/dev/null | grep -i "trimmed automatically"` and `cargo run --quiet -- --grammar 2>/dev/null | grep -i "auto-trim"`.

```bash
git add docs/skill-find-clipart.md docs/grammar.md
git commit -m "docs: clipart viewBox auto-trim (default on, [trim: false])"
```

---

## Final verification

- [ ] `cargo test 2>&1 | grep -E "test result"` — all `ok`.
- [ ] `cargo clippy --all-targets 2>&1 | grep -cE "^warning: "` — `0`.
- [ ] `bash examples/render-all.sh` — green (embedded-images may re-anchor; expected).
- [ ] Manual: a real fat-margin clipart with a connector to it — the connector now meets the drawing edge, not the viewBox edge; `[trim: false]` reverts to the old behavior.
