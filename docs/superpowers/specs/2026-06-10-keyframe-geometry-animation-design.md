# Keyframe Position & Size Animation — Design

**Date:** 2026-06-10
**Status:** Approved (brainstorming) — pending implementation plan

## Problem

`keyframe { transform elem [...] }` currently supports visual overrides (opacity,
fill, stroke, rotation), but elements cannot meaningfully move or resize between
frames as a *smooth animation*. Two gaps:

1. **No motion.** Geometry diffs (`x`, `y`, `width`, `height`) are already computed
   and emitted as per-frame CSS (`#id { x: Npx; width: Npx }` under `.frame-<name>`),
   but no `transition:` rule is emitted, so frame switches jump instantly.
2. **No persistence (latent bug).** The skill doc (gotcha #3) claims "Transforms in
   frame N carry forward," but `compute_frame_states` rebuilds the per-frame
   `transforms` map fresh each keyframe — only `hidden_elements` is cumulative. So
   *all* transform overrides (visual and geometry) snap back the moment a later frame
   doesn't restate them. Documented behavior ≠ implemented behavior.

3. **Constraints fight motion.** "Constraints always solved" is an invariant, so the
   solver re-runs every frame and pulls elements back to their frame-0 constraint
   positions — blocking most intended animation. There is no way to change the active
   constraint set per frame, and constraints cannot even be named to be referenced.

We also lack ergonomic geometry keys (`dx`, `dy`, `scale`).

## Goal

Animate an element's position and size across keyframes, rendered as smooth CSS
transitions. The deck steps frames via a `frame-<name>` class on the SVG root; CSS
transitions provide the motion. Geometry transforms persist forward like visual ones.

### Acceptance scenario

An input box of token-chips that, on a keyframe, grows wider and recenters, while a
separate "predicted token" chip positioned below an LLM box moves up into the box's
new right-hand slot — both motions smooth. Plus a static reset case (move/scale back
via `--frame`).

**Critical:** it must be the *same* `rect` element that animates to new dimensions
(`transform box [width: N, x: N]` tweening) — not a cross-fade between two
differently-sized boxes. Because the box holds chips, animating the box must carry
its contents so the whole input cluster grows and recenters as a unit.

## Confirmed technical fact

SVG geometry attributes (`x`, `y`, `width`, `height`) are exposed as animatable CSS
properties in Chrome and Firefox (and modern Safari). A base
`#box { transition: x .8s, width .8s, ... }` plus a `.frame-2 #box { x: 120px;
width: 360px }` rule makes the same rect grow and reposition smoothly, strokes
staying crisp. (`transform: scale` was rejected — it distorts strokes and content.)

## Design

### 1. New transform keys

Add to `StyleKey` (`src/parser/ast.rs`) and the modifier-block grammar
(`src/parser/grammar.rs` / lexer): `Dx`, `Dy`, `Scale`. `X`, `Y`, `Width`, `Height`,
`Rotation` already parse. New keys are meaningful inside keyframe `transform` blocks;
elsewhere they are ignored (no error) for now.

### 2. Geometry application — `apply_transform_to_element` (src/layout/keyframe.rs)

Extend to handle the new keys, applied in this order against the element's frame-0
(base) bounds:

1. **Absolutes:** `x`, `y`, `width`, `height` set `bounds.*` directly (existing).
2. **Deltas:** `dx`, `dy` → `bounds.x += dx`, `bounds.y += dy`. Delta is relative to
   the frame-0 laid-out position (the cloned base), NOT the previous frame.
3. **Scale (about center):** `scale: s` → `new_w = w*s; bounds.x -= (new_w-w)/2;
   bounds.width = new_w` (and likewise height), keeping the element's center fixed.

This path serves both `--animate` (diffing) and static `--frame` rendering (reset).

### 3. Cumulative transforms — `compute_frame_states` (src/layout/keyframe.rs)

Make `transforms` cumulative, mirroring `hidden_elements`:

- Carry each element's accumulated transform modifiers forward across frames.
- A later keyframe's `transform` on the same element **merges per-property** over the
  carried-forward set (e.g. frame1 `[width: 360]`, frame2 `[x: 120]` → frame2
  effective `{width: 360, x: 120}`).
- To reset a property, restate it explicitly (`dx: 0`, or the original absolute),
  consistent with existing opacity guidance.

This realizes the documented persistence for *all* transforms. It is a behavior
change to existing visual transforms (they become persistent) — approved, and it
aligns code with the skill doc.

### 4. Smooth transitions, CSS-controlled — `generate_keyframe_css` (src/renderer/svg.rs)

Emit two **low-specificity, overridable** default rules in the keyframe CSS:

```css
.kf-animatable {
  transition: x .5s ease, y .5s ease, width .5s ease, height .5s ease,
              rotate .5s ease, fill .5s ease, stroke .5s ease;
}
.kf-fade { transition: opacity .5s ease; }
```

- `kf-animatable` is added to elements that have any geometry/color diff in any frame.
- `kf-fade` is added to the visibility wrapper groups (both the `kf-hidden kf-{id}`
  groups and the visible-start `kf-{id}` groups from the prior hide/show fix), so
  show/hide cross-fades (opacity tweens, per decision).
- Single-class specificity means a user's `--stylesheet-css` (layered *after* the
  generated CSS) can override duration/easing — "controlled by CSS." Default is
  `.5s ease`.
- These base rules are emitted even under `--no-frame-css` (they are hooks, not
  per-frame rules), so an external deck still gets motion.

### 5. Renderer class hooks — `render_svg_with_keyframes` / `render_element_*`

Compute a `kf_animatable: HashSet<String>` (ids with any non-opacity diff in any
frame) alongside the existing `kf_referenced` set, thread it to the element renderer,
and add the `kf-animatable` class to those shapes. Add `kf-fade` to wrapper groups in
`start_visibility_group` / `start_kf_class_group`.

### 6. Named constraints & per-keyframe constraint control (the unblock)

**Why this is required, not deferrable.** With "constraints always solved" as the
invariant, the solver re-runs every frame and yanks elements back to their frame-0
constraint positions — actively *fighting* any transform or intended motion. The only
way to change where the solver places something in a later frame is to change the
active constraint set for that frame. Without this, most animations are blocked by
constraints. (Hand-computing absolute coordinates against a box that itself just moved
is the anti-pattern we reject — it breaks semantic-over-geometric.)

**Named constraints.** `ConstrainDecl` gains an optional name:
`constrain <expr> as <name>` (grammar + AST; today `ConstrainDecl { expr }` has no
name). Names are the handle for releasing/toggling — you can't release what you can't
name.

**Keyframe constraint ops.** A `keyframe { }` block may now contain, alongside
`show`/`hide`/`transform`:

- `constrain <expr> [as <name>]` — activate a constraint from this frame forward
  (cumulative). If it targets the same `(element, target-property)` as an
  earlier-active constraint, it **overrides** that one (per-property last-wins, like
  transform merge) — so re-pinning "just works" without an explicit release.
- `disable <name>` — deactivate a named constraint from this frame forward (the
  "release" / free-without-repin case).
- `enable <name>` — reactivate a previously disabled named constraint.

**State + solve.** `FrameState` gains cumulative `added_constraints` and a
`disabled_constraints: HashSet<String>` set (carried forward like `hidden_elements`).
`resolve_frame_layout` builds the per-frame active constraint set:

```
active = (base_constraints ∪ added_constraints)
         minus those named in disabled_constraints
         minus those overridden by a later constraint on the same (element, property)
```

then re-solves with it. The existing `rewrite_constraints_for_transforms` (which drops
a geometry-transformed element's own positioning constraints so the transform wins) is
retained as the implicit override path for `transform`; explicit
`constrain`/`disable`/`enable` is the general path. Both feed the same active-set
computation. Target-property identity uses the existing `get_constraint_lhs_element` /
`get_constraint_target_var` helpers.

### 7. Groups carry contents via the solver cascade

No new group machinery. A keyframe re-solves constraints (§6), so chips constrained
relative to the box re-position to their new solved spots; each cascaded child gets its
own geometry diff → `kf-animatable` → tweens independently. Children must be
constrained to follow the parent (relative constraints, or `contains` plus
row/relative positioning). Auto-layout containers are out of scope for this iteration.

### 8. Acceptance scenario, realized

- `transform box [width: 360, x: 120]` — same rect grows + recenters, tweening via
  geometry-prop transition. Its own frame-0 positioning constraint is overridden by the
  transform (existing rewrite path).
- Chips inside follow via the constraint cascade (§7).
- The separate predicted-token chip — constrained `below LLM` (named, e.g.
  `as chip_home`) in frame 0 — moves into the box's new right slot *semantically*:
  ```
  keyframe "merge" {
      disable chip_home
      constrain chip.center_x = box.right - 30
      constrain chip.center_y = box.center_y
  }
  ```
  Re-solve places it relative to the moved box; it tweens. No hand-typed coordinates.
- Static reset: a later keyframe (or `--frame` on an earlier one) restates base
  geometry/constraints; with cumulative state, reset is explicit.

## Testing

New tests (likely `tests/keyframe_geometry_animation.rs` + unit tests in
`src/layout/keyframe.rs`):

1. **Geometry diff CSS** — `transform [x, y]` and `transform [width, height]` emit the
   right `#id { x:…; y:…; width:…; height:… }` per frame.
2. **dx/dy** — delta from base produces the expected absolute target.
3. **scale** — `scale: 2` doubles width/height and keeps center fixed (x/y shift).
4. **Transition emission** — `.kf-animatable` and `.kf-fade` rules present with
   default `.5s ease`; animatable elements carry the `kf-animatable` class.
5. **Cumulative persistence** — a transform in frame 1 still applies in frame 2 when
   frame 2 doesn't restate it; a per-property override in frame 2 merges correctly.
6. **Group cascade** — a child constrained to a transformed parent gets its own
   geometry diff (follows the parent), and carries `kf-animatable`.
7. **Named constraints parse** — `constrain <expr> as <name>` parses and the name is
   captured on `ConstrainDecl`.
8. **Per-keyframe constraint control** — `disable <name>` in a keyframe removes that
   constraint from the active set for that frame forward; a keyframe `constrain` on the
   same `(element, property)` overrides the earlier one; `enable <name>` restores it.
   The chip scenario (disable home constraint, add box-slot constraint) produces the
   chip's new solved position as a geometry diff.

Full suite + `examples/render-all.sh` must stay green (CSS-var-ordering diffs aside).

## Docs

- `docs/grammar.md` (`--grammar`): document `dx`, `dy`, `scale` transform keys;
  `constrain <expr> as <name>` naming; and keyframe `constrain`/`disable`/`enable` ops.
- `docs/skill-animation.md` (`--skill-animation`): document geometry transform
  options, the transition mechanism (and CSS override), per-keyframe constraint control
  (named constraints + disable/enable + override), and that persistence is now
  actually implemented (update/clarify gotcha #3).

## Non-goals (this iteration)

- Group `transform: translate/scale` on the `<g>` itself (rejected: scale distorts;
  cascade is the chosen path).
- Per-keyframe transition duration in the grammar (control via CSS instead).
- Auto-layout container animation.
