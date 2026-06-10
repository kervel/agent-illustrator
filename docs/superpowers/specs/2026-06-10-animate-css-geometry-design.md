# Geometry in `--animate-css` — Design

**Date:** 2026-06-10
**Status:** Approved (brainstorming) — pending implementation plan

## Problem

Two animation outputs exist:
- `--animate` (JS): toggles a `frame-<name>` class on the SVG root, so the full
  frame-class CSS (`generate_keyframe_css`) applies — geometry (transform/width/`d`) +
  transitions. Looks perfect (confirmed: the `/tmp/token-loop-preview.svg` JS preview).
- `--animate-css` (`generate_animate_css`): self-cycling `@keyframes`, **opacity only**.
  No JS needed — the only mode that animates when an SVG is embedded as an `<img>`
  (e.g. a GitHub README), where JS does not run.

So a geometry-driven example (token-prediction: box grows, tokens slide in, the feed
connector follows) cannot be shown animated in the README — `--animate-css` would only
blink token visibility at frame-0 positions, which looks broken.

## Goal

Extend `--animate-css` to animate **geometry** (element position/size/rotation and
connection paths) in addition to visibility, so a CSS-only SVG embedded as an `<img>`
shows the same motion as the JS `--animate` preview. Then render the token-prediction
README example with `--animate-css`.

## Design

### Geometry timelines (in `generate_animate_css`)

The frame diffs already carry per-frame geometry: `ElementDiff { tx, ty, width, height,
rotation, opacity, fill, stroke }` and `ConnectionDiff { opacity, path, morphable }`.
Build per-element / per-connection timelines across the `n` frames and emit `@keyframes`:

- **Element position + rotation** → `transform: translate(tx,ty) rotate(rot)` on the
  wrapper `.kf-{id}` (identity when a frame has no diff). Smooth.
- **Element size** → `width/height` on the shape `#{id}` (base dims when no diff). Smooth.
- **Element fill/stroke** → on `#{id}` if present. Smooth.
- **Connection path, morphable** → `d: path(...)` on `.conn-{id}` (via `connection_path_d`,
  needs `conn_meta`). Smooth.
- **Connection path, non-morphable (reshape)** → crossfade: opacity timelines for the
  base path `.conn-{id}-base` (0 in reshaping frames) and each variant
  `.conn-{id}-f{frame}` (1 only in its frame). Stepped opacity (the variants already
  render in `render_svg_with_keyframes`).
- **Visibility** → opacity on `.kf-{id}` / `.conn-{id}` (existing). Stepped.

### Timing — smooth geometry, stepped visibility

A single CSS `animation` shorthand can list several animations, each with its own
timing function. Per element/connection we emit up to two animations on a selector:
`animation: kf-vis-{id} {dur}s step-end infinite, kf-geo-{id} {dur}s ease infinite`.
Visibility/crossfade keyframes use `step-end` (instant); geometry keyframes use `ease`
(interpolate between frame stops → the slide/grow). Where a selector only needs one,
emit one.

### DRY — share the per-frame string builders

`generate_keyframe_css` (frame-class) and `generate_animate_css` (`@keyframes`) now emit
the same geometry two ways. Factor the per-frame **element transform string**
(`translate(tx,ty) rotate(rot)`) and the **connection `d` string** (already
`connection_path_d`) into shared helpers used by both, so the two paths can't drift.

### Wiring

- `generate_animate_css` gains a `conn_meta: &HashMap<String,(RoutingMode,bool,f64)>`
  parameter (same map `render_svg_with_keyframes` builds for `generate_keyframe_css`),
  to rebuild connection `d` strings.
- Fix its connection selector from `.ai-connection.conn-{id}` to `.conn-{id}` for
  consistency with `generate_keyframe_css` (so labels toggle too).
- `examples/render-all.sh`: add `token-prediction` to `extra_flags_for` → `--animate-css`.

## Testing

- **Element transform keyframes:** a `--animate-css` render of a frame that moves an
  element emits a `@keyframes` with `transform: translate(...)` and the element carries a
  geometry animation (not just opacity).
- **Element size keyframes:** a width-growing element emits `width:` keyframes on `#id`.
- **Morphable connection:** a connection whose endpoint moves emits `d: path(...)`
  keyframes.
- **Crossfade connection:** a reshaping connection emits variant opacity keyframes
  (`.conn-{id}-f{frame}` toggled).
- **Timing:** geometry animation uses a smooth function, visibility uses `step-end`
  (assert both animation names present on the wrapper).
- **Regression:** existing opacity-only `--animate-css` (e.g. agentic-loop-story) still
  cycles visibility; `--animate` (JS) unchanged.
- **token-prediction README render** stays green in `render-all.sh`.

## Docs

- `--skill-animation`: note that `--animate-css` now animates geometry (position, size,
  connection paths), not just visibility — so geometry animations work when the SVG is
  embedded as an image (README/`<img>`), while `--animate` (JS) is for direct viewing.

## Non-goals

- Changing the JS `--animate` path (already complete via frame-class CSS).
- New easing/duration controls (reuse the existing `frame_duration`; CSS override still
  possible via `--stylesheet-css`).
- Animating properties beyond what the frame diffs already capture.
