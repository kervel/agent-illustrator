# Keyframe Position & Size Animation — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Animate element position and size across keyframes as smooth CSS transitions, with persistent transforms and per-keyframe constraint control so the solver stops fighting motion.

**Architecture:** Position/rotation animate as `transform: translate()/rotate()` on the per-element `kf-{id}` wrapper group (which already contains shape + label, so labels follow); size animates as `width`/`height` geometry props on the inner shape; opacity on the wrapper. Transforms become cumulative (persist forward). Named constraints plus keyframe `constrain`/`disable`/`enable` ops change the per-frame active constraint set, which `resolve_frame_layout` re-solves.

**Tech Stack:** Rust, `chumsky` parser, `logos` lexer, `kasuari`/cassowary constraint solver, SVG/CSS output. Tests via `cargo test`.

**Spec:** `docs/superpowers/specs/2026-06-10-keyframe-geometry-animation-design.md`

---

## File Structure

**Phase A — geometry inputs, persistence, transitions:**
- `src/parser/ast.rs` — add `Dx`, `Dy`, `Scale` to `StyleKey`. (The `ElementDiff` position-field rename lives in `keyframe.rs`, below.)
- `src/parser/grammar.rs` — map `"dx"`/`"dy"`/`"scale"` to the new `StyleKey`s.
- `src/layout/keyframe.rs` — `apply_transform_to_element` handles dx/dy/scale (ordered); `compute_frame_states` makes transforms cumulative; `ElementDiff` position fields become translate deltas (`tx`/`ty`); `diff_element` computes deltas.
- `src/renderer/svg.rs` — `generate_keyframe_css` emits `transform: translate/rotate` on `.kf-{id}` and size on `#{id}`, plus `.kf-anim`/`.ai-shape` transition rules; `start_visibility_group`/`start_kf_class_group` add `kf-anim`; widen `kf_referenced`.

**Phase B — per-keyframe constraint control:**
- `src/parser/lexer.rs` — add `Disable`, `Enable` tokens.
- `src/parser/ast.rs` — add `name: Option<Spanned<Identifier>>` to `ConstrainDecl`; add `Constrain`/`Disable`/`Enable` variants to `KeyframeOp`.
- `src/parser/grammar.rs` — `constrain <expr> as <name>`; keyframe `constrain`/`disable`/`enable` ops.
- `src/layout/keyframe.rs` — `FrameState` gains `added_constraints` + `disabled_constraints`; `compute_frame_states` accumulates them; `resolve_frame_layout` builds the per-frame active constraint set.

**Phase C — acceptance + docs:**
- `examples/token-prediction.ail` — acceptance scenario.
- `docs/grammar.md`, `docs/skill-animation.md` — document new syntax.

**Tests:**
- `tests/keyframe_geometry_animation.rs` — integration (render → assert CSS).
- Unit tests inline in `src/layout/keyframe.rs` and `src/parser/grammar.rs` test modules.

---

## Phase A: Geometry inputs, persistence, transitions

### Task A1: Add `dx`/`dy`/`scale` transform keys

**Files:**
- Modify: `src/parser/ast.rs` (StyleKey enum, ~L370)
- Modify: `src/parser/grammar.rs` (style_key match, ~L168)
- Test: `src/parser/grammar.rs` (test module)

- [ ] **Step 1: Write the failing test** — append to the `#[cfg(test)] mod tests` block in `src/parser/grammar.rs` (find it with `grep -n "mod tests" src/parser/grammar.rs`):

```rust
#[test]
fn parses_dx_dy_scale_transform_keys() {
    use crate::parser::ast::{Statement, KeyframeOp, StyleKey};
    let src = r#"
rect box [width: 10, height: 10]
keyframe "k" { transform box [dx: 5, dy: -3, scale: 2] }
"#;
    let doc = crate::parse(src).expect("parse ok");
    let kf = doc.statements.iter().find_map(|s| match &s.node {
        Statement::Keyframe(k) => Some(k),
        _ => None,
    }).expect("keyframe present");
    let modifiers = kf.operations.iter().find_map(|op| match &op.node {
        KeyframeOp::Transform { modifiers, .. } => Some(modifiers),
        _ => None,
    }).expect("transform op");
    let keys: Vec<&StyleKey> = modifiers.iter().map(|m| &m.node.key.node).collect();
    assert!(keys.contains(&&StyleKey::Dx), "dx should map to StyleKey::Dx, got {:?}", keys);
    assert!(keys.contains(&&StyleKey::Dy), "dy should map to StyleKey::Dy");
    assert!(keys.contains(&&StyleKey::Scale), "scale should map to StyleKey::Scale");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib parses_dx_dy_scale_transform_keys 2>&1 | tail -20`
Expected: FAIL — `dx`/`dy`/`scale` currently become `StyleKey::Custom(...)`, so `contains(&&StyleKey::Dx)` is false.

- [ ] **Step 3: Add the enum variants** — in `src/parser/ast.rs`, inside `pub enum StyleKey`, immediately before `Custom(String),`:

```rust
    /// Delta-X: position offset relative to the laid-out position (keyframe transforms)
    Dx,
    /// Delta-Y: position offset relative to the laid-out position (keyframe transforms)
    Dy,
    /// Uniform scale about the element's center (keyframe transforms)
    Scale,
    Custom(String),
```

- [ ] **Step 4: Map the keywords** — in `src/parser/grammar.rs`, inside the `style_key` identifier match (the block starting `"fill" => StyleKey::Fill,`), add before `other => StyleKey::Custom(...)`:

```rust
                "dx" => StyleKey::Dx,
                "dy" => StyleKey::Dy,
                "scale" => StyleKey::Scale,
                other => StyleKey::Custom(other.to_string()),
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test --lib parses_dx_dy_scale_transform_keys 2>&1 | tail -20`
Expected: PASS. (If you hit a non-exhaustive match error elsewhere, it will be in `src/parser/grammar.rs` template_instance — its match already has a `_ => return None` arm, so no change needed.)

- [ ] **Step 6: Commit**

```bash
git add src/parser/ast.rs src/parser/grammar.rs
git commit -m "feat(keyframe): add dx/dy/scale transform style keys"
```

---

### Task A2: Apply `dx`/`dy`/`scale` geometry in ordered passes

**Files:**
- Modify: `src/layout/keyframe.rs` (`apply_transform_to_element`, ~L367-421)
- Test: `src/layout/keyframe.rs` (test module, ~L476)

- [ ] **Step 1: Write the failing test** — append to `mod tests` in `src/layout/keyframe.rs`:

```rust
#[test]
fn scale_grows_about_center() {
    use crate::layout::types::{ElementLayout, BoundingBox};
    use crate::parser::ast::{StyleModifier, StyleKey, StyleValue};
    use crate::parser::Spanned;

    fn modi(key: StyleKey, v: f64) -> Spanned<StyleModifier> {
        Spanned::new(StyleModifier {
            key: Spanned::new(key, 0..0),
            value: Spanned::new(StyleValue::Number { value: v, unit: None }, 0..0),
        }, 0..0)
    }

    let mut elem = ElementLayout::default();
    elem.id = Some(crate::parser::ast::Identifier("e".into()));
    elem.bounds = BoundingBox { x: 100.0, y: 100.0, width: 200.0, height: 100.0 };
    // center is (200, 150)

    let mods = vec![modi(StyleKey::Scale, 2.0)];
    super::apply_transform_to_element(std::slice::from_mut(&mut elem), "e", &mods);

    // width/height doubled, center unchanged
    assert!((elem.bounds.width - 400.0).abs() < 0.001, "width {}", elem.bounds.width);
    assert!((elem.bounds.height - 200.0).abs() < 0.001, "height {}", elem.bounds.height);
    assert!((elem.bounds.x - 0.0).abs() < 0.001, "x {}", elem.bounds.x);   // 100 - (400-200)/2
    assert!((elem.bounds.y - 50.0).abs() < 0.001, "y {}", elem.bounds.y);  // 100 - (200-100)/2
}

#[test]
fn dx_dy_offset_from_base_after_absolute() {
    use crate::layout::types::{ElementLayout, BoundingBox};
    use crate::parser::ast::{StyleModifier, StyleKey, StyleValue};
    use crate::parser::Spanned;
    fn modi(key: StyleKey, v: f64) -> Spanned<StyleModifier> {
        Spanned::new(StyleModifier {
            key: Spanned::new(key, 0..0),
            value: Spanned::new(StyleValue::Number { value: v, unit: None }, 0..0),
        }, 0..0)
    }
    let mut elem = ElementLayout::default();
    elem.id = Some(crate::parser::ast::Identifier("e".into()));
    elem.bounds = BoundingBox { x: 10.0, y: 10.0, width: 5.0, height: 5.0 };
    // absolute x=100 then dx=5 => 105, regardless of modifier order
    let mods = vec![modi(StyleKey::Dx, 5.0), modi(StyleKey::X, 100.0)];
    super::apply_transform_to_element(std::slice::from_mut(&mut elem), "e", &mods);
    assert!((elem.bounds.x - 105.0).abs() < 0.001, "x {}", elem.bounds.x);
}
```

NOTE: if `ElementLayout` has no `Default`, construct it the way other tests in this file do (grep `ElementLayout {` in tests/`src/layout`). Adjust the two constructions accordingly; keep the assertions identical.

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --lib scale_grows_about_center dx_dy_offset_from_base_after_absolute 2>&1 | tail -25`
Expected: FAIL — `Scale`/`Dx`/`Dy` currently hit the `_ => {}` arm, so bounds are unchanged.

- [ ] **Step 3: Rewrite `apply_transform_to_element`** — replace the body of the `if elem.id … == target_id` block (the `for modifier in modifiers { match … }` loop) with a call to a new ordered helper, and add the helper. Full replacement:

```rust
fn apply_transform_to_element(
    elements: &mut [ElementLayout],
    target_id: &str,
    modifiers: &[crate::parser::ast::Spanned<crate::parser::ast::StyleModifier>],
) {
    for elem in elements.iter_mut() {
        if elem.id.as_ref().map_or(false, |id| id.0 == target_id) {
            apply_modifiers_ordered(elem, modifiers);
            return;
        }
        apply_transform_to_element(&mut elem.children, target_id, modifiers);
    }
}

/// Apply transform modifiers in a fixed order against the element's base bounds:
/// 1) absolutes + visual, 2) dx/dy deltas, 3) scale about center.
fn apply_modifiers_ordered(
    elem: &mut ElementLayout,
    modifiers: &[crate::parser::ast::Spanned<crate::parser::ast::StyleModifier>],
) {
    use crate::parser::ast::{StyleKey, StyleValue};
    let num = |v: &StyleValue| -> Option<f64> {
        if let StyleValue::Number { value, .. } = v { Some(*value) } else { None }
    };

    // Pass 1: absolutes + visual
    for m in modifiers {
        match &m.node.key.node {
            StyleKey::Rotation => { if let Some(v) = num(&m.node.value.node) { elem.styles.rotation = Some(v); } }
            StyleKey::Fill => { elem.styles.fill = ResolvedStyles::color_to_css(&m.node.value.node); }
            StyleKey::Stroke => { elem.styles.stroke = ResolvedStyles::color_to_css(&m.node.value.node); }
            StyleKey::Opacity => { if let Some(v) = num(&m.node.value.node) { elem.styles.opacity = Some(v); } }
            StyleKey::Width => { if let Some(v) = num(&m.node.value.node) { elem.bounds.width = v; } }
            StyleKey::Height => { if let Some(v) = num(&m.node.value.node) { elem.bounds.height = v; } }
            StyleKey::X => { if let Some(v) = num(&m.node.value.node) { elem.bounds.x = v; } }
            StyleKey::Y => { if let Some(v) = num(&m.node.value.node) { elem.bounds.y = v; } }
            _ => {}
        }
    }
    // Pass 2: deltas
    for m in modifiers {
        match &m.node.key.node {
            StyleKey::Dx => { if let Some(v) = num(&m.node.value.node) { elem.bounds.x += v; } }
            StyleKey::Dy => { if let Some(v) = num(&m.node.value.node) { elem.bounds.y += v; } }
            _ => {}
        }
    }
    // Pass 3: scale about center
    for m in modifiers {
        if let StyleKey::Scale = &m.node.key.node {
            if let Some(s) = num(&m.node.value.node) {
                let nw = elem.bounds.width * s;
                let nh = elem.bounds.height * s;
                elem.bounds.x -= (nw - elem.bounds.width) / 2.0;
                elem.bounds.y -= (nh - elem.bounds.height) / 2.0;
                elem.bounds.width = nw;
                elem.bounds.height = nh;
            }
        }
    }
}
```

- [ ] **Step 4: Run to verify passing**

Run: `cargo test --lib scale_grows_about_center dx_dy_offset_from_base_after_absolute 2>&1 | tail -25`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/layout/keyframe.rs
git commit -m "feat(keyframe): apply dx/dy/scale geometry in ordered passes"
```

---

### Task A3: Make transforms cumulative (persistence)

**Files:**
- Modify: `src/layout/keyframe.rs` (`compute_frame_states`, ~L92-137)
- Test: `src/layout/keyframe.rs` (test module)

- [ ] **Step 1: Write the failing test** — append to `mod tests`:

```rust
#[test]
fn transforms_persist_and_merge_forward() {
    use crate::parser::ast::{StyleKey, StyleValue};
    let kf1 = make_keyframe("a", vec![
        KeyframeOp::Transform {
            target: make_id("box"),
            modifiers: vec![Spanned::new(StyleModifier {
                key: Spanned::new(StyleKey::Width, 0..0),
                value: Spanned::new(StyleValue::Number { value: 360.0, unit: None }, 0..0),
            }, 0..0)],
        },
    ]);
    let kf2 = make_keyframe("b", vec![
        KeyframeOp::Transform {
            target: make_id("box"),
            modifiers: vec![Spanned::new(StyleModifier {
                key: Spanned::new(StyleKey::X, 0..0),
                value: Spanned::new(StyleValue::Number { value: 120.0, unit: None }, 0..0),
            }, 0..0)],
        },
    ]);
    let states = compute_frame_states(&[&kf1, &kf2]);
    // Frame b must still carry the width from frame a, plus its own x.
    let box_mods = states[1].transforms.get("box").expect("box transformed in frame b");
    let keys: Vec<&StyleKey> = box_mods.iter().map(|m| &m.node.key.node).collect();
    assert!(keys.contains(&&StyleKey::Width), "width persists into frame b, got {:?}", keys);
    assert!(keys.contains(&&StyleKey::X), "x added in frame b");
}
```

The test module already imports `StyleModifier`? If not, add `use crate::parser::ast::StyleModifier;` to the test `use` block (it already imports `crate::parser::ast::*`, so this is covered).

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --lib transforms_persist_and_merge_forward 2>&1 | tail -20`
Expected: FAIL — frame b's `transforms` only contains `x` (transforms are rebuilt per frame today).

- [ ] **Step 3: Make transforms cumulative** — in `compute_frame_states`, replace the per-frame transform collection (the block that builds a fresh `let mut transforms = HashMap::new();` from `kf.operations`) with a cumulative map declared before the loop and merged per-property. The full edited function:

```rust
pub fn compute_frame_states(keyframes: &[&KeyframeDecl]) -> Vec<FrameState> {
    let mut frames = Vec::with_capacity(keyframes.len());
    let mut hidden_elements: HashSet<String> = HashSet::new();
    let mut hidden_connections: HashSet<String> = HashSet::new();
    // Cumulative transforms: element id -> merged modifiers (later keys override earlier).
    let mut cumulative_transforms: HashMap<String, Vec<crate::parser::ast::Spanned<crate::parser::ast::StyleModifier>>> = HashMap::new();

    for kf in keyframes {
        for op in &kf.operations {
            match &op.node {
                KeyframeOp::Show(targets) => {
                    for target in targets {
                        hidden_elements.remove(&target.node.0);
                        hidden_connections.remove(&target.node.0);
                    }
                }
                KeyframeOp::Hide(targets) => {
                    for target in targets {
                        hidden_elements.insert(target.node.0.clone());
                        hidden_connections.insert(target.node.0.clone());
                    }
                }
                KeyframeOp::Transform { target, modifiers } => {
                    let entry = cumulative_transforms.entry(target.node.0.clone()).or_default();
                    for m in modifiers {
                        // per-property override: drop any earlier modifier with the same key
                        entry.retain(|existing| existing.node.key.node != m.node.key.node);
                        entry.push(m.clone());
                    }
                }
            }
        }

        frames.push(FrameState {
            name: kf.name.node.clone(),
            hidden_elements: hidden_elements.clone(),
            hidden_connections: hidden_connections.clone(),
            transforms: cumulative_transforms.clone(),
            no_resolve: kf.no_resolve,
        });
    }

    frames
}
```

(`StyleKey` derives `PartialEq`, so `existing.node.key.node != m.node.key.node` compiles. This replaces the old separate Show/Hide loop and the fresh-per-frame transform loop with one combined loop.)

- [ ] **Step 4: Run to verify passing**

Run: `cargo test --lib transforms_persist_and_merge_forward 2>&1 | tail -20`
Expected: PASS. Also run the existing `test_cumulative_show_hide` to confirm no regression:
Run: `cargo test --lib test_cumulative_show_hide 2>&1 | tail -10` → PASS.

- [ ] **Step 5: Commit**

```bash
git add src/layout/keyframe.rs
git commit -m "feat(keyframe): make transforms cumulative (persist forward, per-property merge)"
```

---

### Task A4: Position as wrapper translate, size on shape, + transition CSS

**Files:**
- Modify: `src/layout/keyframe.rs` (`ElementDiff` struct ~L48, `is_empty` ~L66, `diff_element` ~L424)
- Modify: `src/renderer/svg.rs` (`generate_keyframe_css` ~L796, no-frame-css branch ~L678, `start_visibility_group`/`start_kf_class_group`, `kf_referenced` computation)
- Test: `tests/keyframe_geometry_animation.rs` (new)

- [ ] **Step 1: Write the failing integration test** — create `tests/keyframe_geometry_animation.rs`:

```rust
//! Integration tests for keyframe position/size animation (geometry channel).
use agent_illustrator::render;

#[test]
fn position_emits_translate_on_wrapper_group() {
    // box moves right+down by a delta in frame "move".
    let src = r#"
rect box [width: 100, height: 50, label: "B"]
constrain box.center_x = 100
constrain box.center_y = 100
keyframe "idle" {}
keyframe "move" { transform box [dx: 60, dy: 40] }
"#;
    let svg = render(src).expect("render ok");
    // position animates as a transform translate on the wrapper, NOT as #box { x: ... }
    assert!(svg.contains(".kf-box { transform: translate(60px, 40px)"),
        "expected translate on .kf-box, got:\n{}", svg);
    assert!(!svg.contains("#box { x:"),
        "position must NOT be emitted as a geometry x prop, got:\n{}", svg);
}

#[test]
fn size_emits_width_height_on_shape() {
    let src = r#"
rect box [width: 100, height: 50]
constrain box.center_x = 100
constrain box.center_y = 100
keyframe "idle" {}
keyframe "grow" { transform box [width: 300, height: 120] }
"#;
    let svg = render(src).expect("render ok");
    assert!(svg.contains("#box {") && svg.contains("width: 300px") && svg.contains("height: 120px"),
        "expected #box width/height on inner shape, got:\n{}", svg);
}

#[test]
fn emits_transition_rules_and_kf_anim_class() {
    let src = r#"
rect box [width: 100, height: 50]
constrain box.center_x = 100
constrain box.center_y = 100
keyframe "idle" {}
keyframe "grow" { transform box [width: 300] }
"#;
    let svg = render(src).expect("render ok");
    assert!(svg.contains(".kf-anim { transition: transform 0.5s ease, opacity 0.5s ease;"),
        "expected .kf-anim transition rule, got:\n{}", svg);
    assert!(svg.contains(".ai-shape { transition:"),
        "expected .ai-shape transition rule, got:\n{}", svg);
    // box is geometry-diffed → gets a wrapper group carrying kf-anim
    assert!(svg.contains("kf-anim"), "wrapper group should carry kf-anim, got:\n{}", svg);
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test keyframe_geometry_animation 2>&1 | tail -30`
Expected: FAIL — today position is emitted as `#box { x: ... }`, no translate, no `.kf-anim`/`.ai-shape` transition rules, and `box` (not toggled) gets no wrapper group.

- [ ] **Step 3a: Make `ElementDiff` position fields translate deltas** — in `src/layout/keyframe.rs`, change the struct (rename `x`/`y` → `tx`/`ty`, keep the rest):

```rust
pub struct ElementDiff {
    /// Translate delta X (solved.x - base.x), emitted as transform on the wrapper
    pub tx: Option<f64>,
    /// Translate delta Y (solved.y - base.y)
    pub ty: Option<f64>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub rotation: Option<f64>,
    pub opacity: Option<f64>,
    pub fill: Option<String>,
    pub stroke: Option<String>,
}
```

Update `is_empty`:

```rust
    pub fn is_empty(&self) -> bool {
        self.tx.is_none()
            && self.ty.is_none()
            && self.width.is_none()
            && self.height.is_none()
            && self.rotation.is_none()
            && self.opacity.is_none()
            && self.fill.is_none()
            && self.stroke.is_none()
    }
```

- [ ] **Step 3b: Compute deltas in `diff_element`** — replace the x/y blocks (the `if (base.bounds.x - solved.bounds.x).abs() > eps` and the y equivalent) with:

```rust
    if (base.bounds.x - solved.bounds.x).abs() > eps {
        diff.tx = Some(solved.bounds.x - base.bounds.x);
    }
    if (base.bounds.y - solved.bounds.y).abs() > eps {
        diff.ty = Some(solved.bounds.y - base.bounds.y);
    }
```

(width/height/rotation/opacity/fill/stroke blocks stay unchanged. Also fix the bug-fix merge site in `compute_frame_diffs` if it names `diff.x`/`diff.y` — grep `\.x =`/`\.y =` in this file; the merge in the visibility/transform block copies fields, update `tx`/`ty` there too.)

- [ ] **Step 3c: Emit translate + size in `generate_keyframe_css`** — in `src/renderer/svg.rs`, replace the per-element props loop (the block building `props` from `diff.x`/`diff.y`/`diff.width`/`diff.height`/`diff.rotation`/`diff.fill`/`diff.stroke` and emitting `#{id} { … }`) with two emissions — a transform on the wrapper group and size/color on the shape:

```rust
            // Position + rotation animate via transform on the wrapper group (label rides along).
            let mut xf = Vec::new();
            if diff.tx.is_some() || diff.ty.is_some() {
                xf.push(format!("translate({}px, {}px)",
                    diff.tx.unwrap_or(0.0), diff.ty.unwrap_or(0.0)));
            }
            if let Some(rot) = diff.rotation {
                xf.push(format!("rotate({}deg)", rot));
            }
            if !xf.is_empty() {
                css.push_str(&format!("  .kf-{} {{ transform: {}; }}\n", elem_id, xf.join(" ")));
            }
            // Size + color animate on the inner shape.
            let mut props = Vec::new();
            if let Some(w) = diff.width { props.push(format!("width: {}px", w)); }
            if let Some(h) = diff.height { props.push(format!("height: {}px", h)); }
            if let Some(ref fill) = diff.fill { props.push(format!("fill: {}", fill)); }
            if let Some(ref stroke) = diff.stroke { props.push(format!("stroke: {}", stroke)); }
            if !props.is_empty() {
                css.push_str(&format!("  #{} {{ {}; }}\n", elem_id, props.join("; ")));
            }
```

(The `if let Some(opacity) = diff.opacity { … .kf-{} { opacity } … }` block just above stays as-is.)

- [ ] **Step 3d: Emit the transition base rules** — in `generate_keyframe_css`, just after the line `css.push_str(".kf-hidden { opacity: 0; }\n");`, add:

```rust
    css.push_str(".kf-anim { transition: transform 0.5s ease, opacity 0.5s ease; }\n");
    css.push_str(".ai-shape { transition: width 0.5s ease, height 0.5s ease, fill 0.5s ease, stroke 0.5s ease; }\n");
```

And in the `no_frame_css` branch string (the `String::from("/* Keyframe CSS suppressed …")`), append the same two rules so external decks still get motion:

```rust
        String::from("/* Keyframe CSS suppressed (--no-frame-css) */\n.kf-hidden { opacity: 0; }\n.kf-anim { transition: transform 0.5s ease, opacity 0.5s ease; }\n.ai-shape { transition: width 0.5s ease, height 0.5s ease, fill 0.5s ease, stroke 0.5s ease; }\n")
```

- [ ] **Step 3e: Add `kf-anim` to wrapper groups** — in `src/renderer/svg.rs`, update both builder methods:

```rust
    pub fn start_visibility_group(&mut self, element_id: &str) {
        self.elements.push(format!(
            r#"{}<g class="kf-hidden kf-{} kf-anim">"#,
            self.indent_str(),
            element_id
        ));
        self.indent += 1;
    }

    pub fn start_kf_class_group(&mut self, element_id: &str) {
        self.elements.push(format!(
            r#"{}<g class="kf-{} kf-anim">"#,
            self.indent_str(),
            element_id
        ));
        self.indent += 1;
    }
```

- [ ] **Step 3f: Widen `kf_referenced` to all diffed elements** — in `render_svg_with_keyframes`, change the `kf_referenced` computation so any element with *any* diff (not just opacity) gets a wrapper group:

```rust
    let kf_referenced: std::collections::HashSet<String> = frame_diffs
        .iter()
        .flat_map(|f| f.element_diffs.iter())
        .filter(|(_, diff)| !diff.is_empty())
        .map(|(id, _)| id.clone())
        .filter(|id| !frame0_hidden.contains(id))
        .collect();
```

- [ ] **Step 4: Run to verify passing**

Run: `cargo test --test keyframe_geometry_animation 2>&1 | tail -30`
Expected: PASS (all three). Then the prior regression suite:
Run: `cargo test --test keyframe_animation_bugs 2>&1 | tail -8` → PASS.

- [ ] **Step 5: Run full suite + examples**

Run: `cargo test 2>&1 | grep -E "test result|FAILED" | head`
Expected: all `ok`. Then `bash examples/render-all.sh 2>&1 | grep -i fail || echo "examples ok"`.

- [ ] **Step 6: Commit**

```bash
git add src/layout/keyframe.rs src/renderer/svg.rs tests/keyframe_geometry_animation.rs
git commit -m "feat(keyframe): animate position via wrapper translate, size on shape, emit transitions"
```

---

## Phase B: Per-keyframe constraint control

### Task B1: Named constraints (`constrain <expr> as <name>`)

**Files:**
- Modify: `src/parser/ast.rs` (`ConstrainDecl`, ~L663)
- Modify: `src/parser/grammar.rs` (`constrain_decl`, ~L743)
- Test: `src/parser/grammar.rs` (test module)

- [ ] **Step 1: Write the failing test** — append to `mod tests` in `src/parser/grammar.rs`:

```rust
#[test]
fn parses_named_constraint() {
    use crate::parser::ast::Statement;
    let src = r#"
rect a [width: 10, height: 10]
constrain a.center_x = 50 as a_home
"#;
    let doc = crate::parse(src).expect("parse ok");
    let named = doc.statements.iter().any(|s| matches!(
        &s.node, Statement::Constrain(c) if c.name.as_ref().map(|n| n.node.0.as_str()) == Some("a_home")
    ));
    assert!(named, "constraint should be named a_home");
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --lib parses_named_constraint 2>&1 | tail -20`
Expected: FAIL to compile — `ConstrainDecl` has no `name` field.

- [ ] **Step 3a: Add the field** — in `src/parser/ast.rs`:

```rust
pub struct ConstrainDecl {
    pub expr: ConstraintExpr,
    /// Optional name (`constrain <expr> as <name>`) — handle for keyframe disable/enable.
    pub name: Option<Spanned<Identifier>>,
}
```

- [ ] **Step 3b: Parse the optional name** — in `src/parser/grammar.rs`, update `constrain_decl`:

```rust
    let constrain_decl = just(Token::Constrain)
        .ignore_then(constraint_expr)
        .then(just(Token::As).ignore_then(identifier.clone()).or_not())
        .map(|(expr, name)| ConstrainDecl { expr, name });
```

- [ ] **Step 3c: Fix other `ConstrainDecl { … }` constructions** — grep and update any struct-literal sites to include `name: None`:

Run: `grep -rn "ConstrainDecl {" src/` — for each site that is NOT the parser above, add `name: None,`. (Most code reads `c.expr` and is unaffected.)

- [ ] **Step 4: Run to verify passing**

Run: `cargo test --lib parses_named_constraint 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/parser/ast.rs src/parser/grammar.rs
git commit -m "feat(constraints): support naming constraints with 'as <name>'"
```

---

### Task B2: `disable`/`enable` tokens + keyframe constraint ops

**Files:**
- Modify: `src/parser/lexer.rs` (token enum, ~L130)
- Modify: `src/parser/ast.rs` (`KeyframeOp`, ~L281)
- Modify: `src/parser/grammar.rs` (keyframe ops, ~L1230-1264)
- Test: `src/parser/grammar.rs` (test module)

- [ ] **Step 1: Write the failing test** — append to `mod tests`:

```rust
#[test]
fn parses_keyframe_constraint_ops() {
    use crate::parser::ast::{Statement, KeyframeOp};
    let src = r#"
rect a [width: 10, height: 10]
constrain a.center_x = 10 as a_home
keyframe "k" {
    disable a_home
    constrain a.center_x = 200
    enable a_home
}
"#;
    let doc = crate::parse(src).expect("parse ok");
    let kf = doc.statements.iter().find_map(|s| match &s.node {
        Statement::Keyframe(k) => Some(k), _ => None,
    }).expect("keyframe");
    let has_disable = kf.operations.iter().any(|o| matches!(&o.node, KeyframeOp::Disable(n) if n.iter().any(|x| x.node.0 == "a_home")));
    let has_enable  = kf.operations.iter().any(|o| matches!(&o.node, KeyframeOp::Enable(n) if n.iter().any(|x| x.node.0 == "a_home")));
    let has_constr  = kf.operations.iter().any(|o| matches!(&o.node, KeyframeOp::Constrain(_)));
    assert!(has_disable, "disable parsed");
    assert!(has_enable, "enable parsed");
    assert!(has_constr, "keyframe-scoped constrain parsed");
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --lib parses_keyframe_constraint_ops 2>&1 | tail -20`
Expected: FAIL to compile — no `KeyframeOp::Disable/Enable/Constrain`, no `disable`/`enable` tokens.

- [ ] **Step 3a: Add tokens** — in `src/parser/lexer.rs`, after the `Transform` token:

```rust
    #[token("disable")]
    Disable,
    #[token("enable")]
    Enable,
```

- [ ] **Step 3b: Add `KeyframeOp` variants** — in `src/parser/ast.rs`:

```rust
pub enum KeyframeOp {
    Show(Vec<Spanned<Identifier>>),
    Hide(Vec<Spanned<Identifier>>),
    Transform {
        target: Spanned<Identifier>,
        modifiers: Vec<Spanned<StyleModifier>>,
    },
    /// Activate a constraint for this frame forward (cumulative).
    Constrain(ConstrainDecl),
    /// Deactivate named constraints from this frame forward.
    Disable(Vec<Spanned<Identifier>>),
    /// Reactivate previously disabled named constraints.
    Enable(Vec<Spanned<Identifier>>),
}
```

- [ ] **Step 3c: Parse the new keyframe ops** — in `src/parser/grammar.rs`, the keyframe-ops region (where `show_op`/`hide_op`/`transform_op` are defined, ~L1231). Add three parsers and extend the `choice`. `constrain_decl` is in scope (defined earlier in the same parser fn):

```rust
        let kf_constrain_op = constrain_decl.clone()
            .map_with(|decl, e| Spanned::new(KeyframeOp::Constrain(decl), span_range(&e.span())));

        let disable_op = just(Token::Disable)
            .ignore_then(identifier.clone().separated_by(just(Token::Comma)).at_least(1).collect::<Vec<_>>())
            .map_with(|names, e| Spanned::new(KeyframeOp::Disable(names), span_range(&e.span())));

        let enable_op = just(Token::Enable)
            .ignore_then(identifier.clone().separated_by(just(Token::Comma)).at_least(1).collect::<Vec<_>>())
            .map_with(|names, e| Spanned::new(KeyframeOp::Enable(names), span_range(&e.span())));

        let keyframe_op = choice((show_op, hide_op, transform_op, kf_constrain_op, disable_op, enable_op));
```

(Order: `kf_constrain_op` starts with `Token::Constrain`, `disable_op`/`enable_op` with their own tokens — no ambiguity with show/hide/transform.)

- [ ] **Step 3d: Make `KeyframeOp` matches exhaustive** — grep for `match` on `KeyframeOp` and add arms for the new variants where needed. The compiler will flag them; the key site is `src/layout/keyframe.rs` `compute_frame_states` (handled in Task B3) — until B3, add temporary `KeyframeOp::Constrain(_) | KeyframeOp::Disable(_) | KeyframeOp::Enable(_) => {}` arms anywhere the compiler errors so this task compiles in isolation.

- [ ] **Step 4: Run to verify passing**

Run: `cargo test --lib parses_keyframe_constraint_ops 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/parser/lexer.rs src/parser/ast.rs src/parser/grammar.rs src/layout/keyframe.rs
git commit -m "feat(keyframe): parse constrain/disable/enable ops in keyframe blocks"
```

---

### Task B3: Accumulate per-frame constraint state

**Files:**
- Modify: `src/layout/keyframe.rs` (`FrameState` ~L14, `compute_frame_states` ~L92)
- Test: `src/layout/keyframe.rs` (test module)

- [ ] **Step 1: Write the failing test** — append to `mod tests`. We assert the simplest observable (the disabled set accumulates); `added_constraints` is exercised end-to-end in Task B4:

```rust
#[test]
fn constraint_ops_accumulate_per_frame() {
    let kf = KeyframeDecl {
        name: Spanned::new("k".into(), 0..0),
        operations: vec![
            Spanned::new(KeyframeOp::Disable(vec![make_id("a_home")]), 0..0),
        ],
        no_resolve: false,
    };
    let states = compute_frame_states(&[&kf]);
    assert!(states[0].disabled_constraints.contains("a_home"));
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --lib constraint_ops_accumulate_per_frame 2>&1 | tail -20`
Expected: FAIL to compile — `FrameState` has no `disabled_constraints`.

- [ ] **Step 3a: Extend `FrameState`** — in `src/layout/keyframe.rs`:

```rust
pub struct FrameState {
    pub name: String,
    pub hidden_elements: HashSet<String>,
    pub hidden_connections: HashSet<String>,
    pub transforms: HashMap<String, Vec<crate::parser::ast::Spanned<crate::parser::ast::StyleModifier>>>,
    /// Constraints added by keyframes, active from their frame forward (cumulative).
    pub added_constraints: Vec<crate::parser::ast::ConstrainDecl>,
    /// Names of constraints disabled from this frame forward (cumulative).
    pub disabled_constraints: HashSet<String>,
    pub no_resolve: bool,
}
```

- [ ] **Step 3b: Accumulate in `compute_frame_states`** — add cumulative containers before the loop and handle the new ops inside the existing `for op in &kf.operations` match, then include them in the pushed `FrameState`:

```rust
    let mut added_constraints: Vec<crate::parser::ast::ConstrainDecl> = Vec::new();
    let mut disabled_constraints: HashSet<String> = HashSet::new();
```

Inside the op match, add arms:

```rust
                KeyframeOp::Constrain(decl) => {
                    added_constraints.push(decl.clone());
                }
                KeyframeOp::Disable(names) => {
                    for n in names { disabled_constraints.insert(n.node.0.clone()); }
                }
                KeyframeOp::Enable(names) => {
                    for n in names { disabled_constraints.remove(&n.node.0); }
                }
```

And in the `frames.push(FrameState { … })`, add:

```rust
            added_constraints: added_constraints.clone(),
            disabled_constraints: disabled_constraints.clone(),
```

(Remove any temporary catch-all arm added in Task B2 Step 3d now that all variants are handled.)

- [ ] **Step 3c: Fix other `FrameState { … }` constructions** — grep and add the two new fields:

Run: `grep -rn "FrameState {" src/` — update each literal (e.g. the test helper `make_keyframe` callers construct via `compute_frame_states`, so likely only real sites need it) with `added_constraints: Vec::new(), disabled_constraints: HashSet::new(),`.

- [ ] **Step 4: Run to verify passing**

Run: `cargo test --lib constraint_ops_accumulate_per_frame 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/layout/keyframe.rs
git commit -m "feat(keyframe): accumulate per-frame added/disabled constraints in FrameState"
```

---

### Task B4: Apply the per-frame active constraint set when re-solving

**Files:**
- Modify: `src/layout/keyframe.rs` (`resolve_frame_layout` ~L262, `rewrite_constraints_for_transforms` ~L309, add a `build_active_document` helper)
- Test: `tests/keyframe_geometry_animation.rs`

- [ ] **Step 1: Write the failing integration test** — append to `tests/keyframe_geometry_animation.rs`:

```rust
#[test]
fn disable_then_reconstrain_moves_element_semantically() {
    // chip is pinned below at center_y=300 in frame 0; in "merge" we disable that
    // pin and re-pin it to y=100. It should move ~200px up (translate ty ~ -200).
    let src = r#"
rect chip [width: 40, height: 20]
constrain chip.center_x = 100
constrain chip.center_y = 300 as chip_home
keyframe "idle" {}
keyframe "merge" {
    disable chip_home
    constrain chip.center_y = 100
}
"#;
    let svg = render(src).expect("render ok");
    // chip moves up: a negative ty translate appears on .kf-chip in the merge frame.
    assert!(svg.contains(".kf-chip { transform: translate("),
        "chip should translate after re-constrain, got:\n{}", svg);
    assert!(svg.contains("-200px") || svg.contains("-199") || svg.contains("-201"),
        "chip should move ~200px up, got:\n{}", svg);
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test keyframe_geometry_animation disable_then_reconstrain_moves_element_semantically 2>&1 | tail -30`
Expected: FAIL — keyframe `constrain`/`disable` are parsed and accumulated but not yet fed into the solver, so the chip stays at y=300 (no diff).

- [ ] **Step 3a: Replace `rewrite_constraints_for_transforms` with a fuller active-document builder** — in `src/layout/keyframe.rs`, add a function that produces the per-frame document: it (1) drops constraints whose LHS targets a geometry-transformed element (existing behavior), (2) drops constraints named in `disabled_constraints`, (3) drops base/earlier constraints overridden by an added constraint on the same `(element, property)`, then (4) appends the added constraints:

```rust
fn build_active_document(doc: &Document, state: &FrameState) -> Document {
    use crate::parser::ast::*;

    // (a) elements with geometry transforms — their own positioning constraints are dropped
    let mut geometry_transformed: HashSet<&str> = HashSet::new();
    for (elem_id, modifiers) in &state.transforms {
        let has_geometry = modifiers.iter().any(|m| matches!(
            m.node.key.node,
            StyleKey::X | StyleKey::Y | StyleKey::Width | StyleKey::Height
                | StyleKey::Dx | StyleKey::Dy | StyleKey::Scale
        ));
        if has_geometry { geometry_transformed.insert(elem_id.as_str()); }
    }

    // (b) (element, property) targets newly pinned by added constraints → override base
    let added_targets: HashSet<(String, String)> = state.added_constraints.iter()
        .filter_map(|c| constraint_target(&c.expr))
        .collect();

    let mut new_doc = doc.clone();
    new_doc.statements.retain(|stmt| {
        if let Statement::Constrain(c) = &stmt.node {
            // drop if its LHS element is geometry-transformed
            if let Some(elem) = get_constraint_lhs_element(&c.expr) {
                if geometry_transformed.contains(elem.as_str()) { return false; }
            }
            // drop if named and disabled
            if let Some(name) = &c.name {
                if state.disabled_constraints.contains(&name.node.0) { return false; }
            }
            // drop if overridden by an added constraint on the same (element, property)
            if let Some(tgt) = constraint_target(&c.expr) {
                if added_targets.contains(&tgt) { return false; }
            }
        }
        true
    });

    // (c) append the keyframe-added constraints (active from this frame forward)
    for decl in &state.added_constraints {
        new_doc.statements.push(Spanned::new(Statement::Constrain(decl.clone()), 0..0));
    }
    new_doc
}

/// (element, property-name) targeted by a constraint LHS, for override detection.
fn constraint_target(expr: &crate::parser::ast::ConstraintExpr) -> Option<(String, String)> {
    use crate::parser::ast::ConstraintExpr;
    let pr = match expr {
        ConstraintExpr::Equal { left, .. }
        | ConstraintExpr::EqualWithOffset { left, .. }
        | ConstraintExpr::Constant { left, .. }
        | ConstraintExpr::GreaterOrEqual { left, .. }
        | ConstraintExpr::LessOrEqual { left, .. } => left,
        ConstraintExpr::Midpoint { target, .. } => target,
        ConstraintExpr::Contains { container, .. } => {
            return Some((container.node.0.clone(), "contains".to_string()));
        }
    };
    Some((
        pr.element.node.leaf().0.clone(),
        format!("{:?}", pr.property.node),
    ))
}
```

(`get_constraint_lhs_element` already exists in this file. `PropertyRef` has `.element` (an `ElementPath` with `.leaf()`) and `.property`. Verify field access against the existing `get_constraint_lhs_element` body and mirror it.)

- [ ] **Step 3b: Use it in `resolve_frame_layout`** — replace the call to `rewrite_constraints_for_transforms(doc, state)` with `build_active_document(doc, state)`, and change the `if geometry_transformed.is_empty() { return doc.clone() }` early-out: now we must also re-solve when there are added/disabled constraints. Update the guard in `resolve_frame_layout` so re-solving runs whenever transforms OR constraint changes exist. In `compute_frame_diffs`, the `solved_result` is computed only `if !state.transforms.is_empty()`; change that condition to also re-solve when constraints changed:

```rust
        let needs_resolve = !state.transforms.is_empty()
            || !state.added_constraints.is_empty()
            || !state.disabled_constraints.is_empty();
        let solved_result = if needs_resolve {
            resolve_frame_layout(base_result, state, doc, config)
        } else {
            None
        };
```

And delete `rewrite_constraints_for_transforms` (now superseded) or leave it unused-removed to satisfy the linter.

- [ ] **Step 4: Run to verify passing**

Run: `cargo test --test keyframe_geometry_animation 2>&1 | tail -30`
Expected: PASS (including `disable_then_reconstrain_moves_element_semantically`).

- [ ] **Step 5: Full suite + examples**

Run: `cargo test 2>&1 | grep -E "test result|FAILED" | head`
Run: `bash examples/render-all.sh 2>&1 | grep -i fail || echo "examples ok"`
Expected: all green.

- [ ] **Step 6: Commit**

```bash
git add src/layout/keyframe.rs
git commit -m "feat(keyframe): per-frame active constraint set (add/disable/override) re-solved"
```

---

## Phase C: Acceptance scenario + docs

### Task C1: Acceptance example

**Files:**
- Create: `examples/token-prediction.ail`
- Modify: `examples/render-all.sh` (if it enumerates files explicitly — check first)

- [ ] **Step 1: Write the example** — create `examples/token-prediction.ail`:

```
/* INTENT: an input box of token chips grows wider and recenters while a
   predicted token chip moves up from below the LLM box into the box's new
   right-hand slot. Render: agent-illustrator examples/token-prediction.ail --animate */

rect canvas [width: 760, height: 420, fill: white, stroke: none, opacity: 0.01]
constrain canvas.center_x = 380
constrain canvas.center_y = 210

rect inbox [width: 240, height: 70, fill: accent-light, stroke: accent-1, stroke_width: 2]
constrain inbox.center_x = 250 as inbox_cx
constrain inbox.center_y = 120

rect tok1 [width: 50, height: 36, fill: background-2, stroke: foreground-3, label: "The"]
rect tok2 [width: 50, height: 36, fill: background-2, stroke: foreground-3, label: "cat"]
constrain tok1.center_y = 120
constrain tok2.center_y = 120
constrain tok1.center_x = 200
constrain tok2.center_x = 270

rect llm [width: 160, height: 70, fill: secondary-light, stroke: secondary-dark, stroke_width: 2, label: "LLM"]
constrain llm.center_x = 600
constrain llm.center_y = 120

rect tok3 [width: 50, height: 36, fill: secondary-2, stroke: secondary-dark, label: "sat"]
constrain tok3.center_x = 600
constrain tok3.center_y = 300 as tok3_home

keyframe "prompt" {
    hide tok3
}

keyframe "predict" {
    show tok3
}

keyframe "append" {
    // grow the input box wider + recenter; chips inside follow via constraints
    transform inbox [width: 340, dx: -50]
    disable inbox_cx
    constrain inbox.center_x = 300
    // move the predicted token up into the box's new right slot
    disable tok3_home
    constrain tok3.center_x = 360
    constrain tok3.center_y = 120
}
```

- [ ] **Step 2: Render it (manual check)**

Run: `cargo run --quiet -- examples/token-prediction.ail --animate > /tmp/token-prediction.svg && grep -c "kf-anim\|translate\|transition" /tmp/token-prediction.svg`
Expected: non-zero — transition rules, translate transforms, and `kf-anim` classes present. Eyeball `/tmp/token-prediction.svg` opens without error.

- [ ] **Step 3: Render all examples (catch regressions, auto-include new file if scripted)**

Run: `bash examples/render-all.sh 2>&1 | tail -5`
Expected: includes `token-prediction.svg` (if the script globs `examples/*.ail`; if it lists files explicitly, add the new file to it).

- [ ] **Step 4: Commit**

```bash
git add examples/token-prediction.ail examples/token-prediction.svg
git commit -m "docs(examples): token-prediction animation (grow box + move predicted token)"
```

---

### Task C2: Documentation

**Files:**
- Modify: `docs/grammar.md`
- Modify: `docs/skill-animation.md`

- [ ] **Step 1: Document grammar additions** — in `docs/grammar.md`, in the keyframe/transform section, add:

```markdown
### Keyframe transform geometry keys

Inside `keyframe { transform <id> [...] }`:

- `x: N`, `y: N` — absolute target position
- `dx: N`, `dy: N` — offset relative to the laid-out (frame-0) position
- `width: N`, `height: N` — absolute target size
- `scale: N` — uniform scale about the element's center

Geometry transforms persist forward across keyframes (per-property), like visual ones.
To reset, restate the property (e.g. `dx: 0`).

### Named constraints & per-keyframe constraint control

- `constrain <expr> as <name>` — name a constraint so a keyframe can toggle it.
- Inside a keyframe: `constrain <expr>` (add for this frame forward; overrides any
  earlier constraint on the same element+property), `disable <name>`, `enable <name>`.
```

- [ ] **Step 2: Document the animation mechanism** — in `docs/skill-animation.md`:
  - In the transform section, document the geometry keys and that position/rotation tween via a wrapper-group `transform` (labels follow) while size tweens via the shape's `width`/`height`.
  - Add to the CSS Transitions section: transitions are now emitted by default
    (`.kf-anim` for transform+opacity, `.ai-shape` for size/color, `0.5s ease`) and can
    be overridden by supplying your own CSS via `--stylesheet-css`.
  - Add a gotcha: a labeled element that *resizes* does not re-center its own label
    (label follows position, not size); resize unlabeled frames, label chips that move.
  - Update gotcha #3 to state persistence is now actually implemented and applies to
    geometry too.

Append to `docs/skill-animation.md` (after the existing CSS Transitions block):

```markdown
### Geometry animation (position & size)

`transform` inside a keyframe can move and resize elements:

```
keyframe "grow" {
    transform box [width: 340, dx: -50]   // grow wider + recenter
    transform chip [dx: 0, dy: -180]      // move up
}
```

- **Position** (`x`/`y`/`dx`/`dy`) and **rotation** tween via a `transform` on the
  element's wrapper group, so the element's label moves with it.
- **Size** (`width`/`height`/`scale`) tweens via the shape's geometry. `scale` is about
  the center.
- Transitions are emitted by default (`0.5s ease`). Override by supplying your own
  `.kf-anim` / `.ai-shape` rules via `--stylesheet-css`.

To make the solver place dependents relative to a moved element, change the active
constraints in the keyframe (`disable <name>` then `constrain ...`). Without this, the
always-solved constraints pull elements back to their frame-0 positions.
```

- [ ] **Step 3: Verify docs render via the CLI flags**

Run: `cargo run --quiet -- --grammar | grep -i "dx\|scale\|as <name>" | head` and `cargo run --quiet -- --skill-animation | grep -i "wrapper\|scale\|disable" | head`
Expected: the new content appears (these flags print the doc files).

- [ ] **Step 4: Commit**

```bash
git add docs/grammar.md docs/skill-animation.md
git commit -m "docs: document keyframe geometry transforms + per-keyframe constraints"
```

---

## Final verification

- [ ] `cargo test 2>&1 | grep -E "test result"` — all suites `ok`.
- [ ] `cargo clippy --all-targets 2>&1 | grep -E "warning|error" | head` — no new warnings (remove any dead `rewrite_constraints_for_transforms`).
- [ ] `bash examples/render-all.sh` — green; example SVG diffs limited to CSS-var ordering + the new animation rules.
- [ ] Manually open `/tmp/token-prediction.svg` (or via `--animate`) and confirm: box grows + recenters, chips ride along, predicted token moves up — all tweened.
