# Clipart viewBox Auto-Trim — Design

**Date:** 2026-06-10
**Status:** Approved (brainstorming) — pending implementation plan

## Problem

A file-based SVG clipart's element bounds are taken from its **viewBox** (see
`parse_svg_dimensions` → `def.svg_dimensions` in `src/template/registry.rs`), and anchors
(`.left/.right/.top/.bottom`) come from those bounds via `AnchorSet::simple_shape`. But
clipart from different authors has different empty **margins** between the viewBox edge
and the actual drawing. So connectors anchored to `.right` of a tight-margin icon touch
the art, while the same anchor on a fat-margin icon floats far away — inconsistent,
ugly connectors with no author error.

## Goal

Make a clipart element's box equal the **artwork's tight content bounding box**, so the
margin is ~0 and identical for every icon. The connector gap then comes uniformly from
the routing/anchor offset. Automatic and on by default; safe when the SVG can't be
fully parsed.

## Design

### Content bbox replaces the viewBox (the crux)

At SVG-template load, compute the artwork's content bounding box and use **that** in
place of the raw viewBox everywhere the viewBox is used today:

- as `svg_dimensions` (drives element sizing aspect + anchor box), and
- as the **embed viewport**: when the clipart content is embedded, the emitted `viewBox`
  is set to the content bbox (`minx miny w h`) so the artwork fills the element rect and
  the anchors hug the drawing.

This keeps everything coherent — anchors and rendered art share one box.

### Computing the bbox — `usvg`

Add the `usvg` crate (pinned). Parse the SVG and read the content bbox
(`tree.root().abs_bounding_box()`, in viewBox coordinates). `usvg` is the parser behind
`resvg`, the de-facto Rust static-SVG engine; it handles shape/path/transform clipart —
the common case — well. It does not run scripts/animation, and text bboxes need fonts.

### Graceful degradation (required)

Trimming is **best-effort**; it must never break a render or silently clip art:

1. **Parse error** → keep the raw viewBox (today's behavior).
2. **Degenerate bbox** (empty, non-finite, or implausibly small — < ~5% of viewBox area,
   the signal that usvg dropped elements it didn't understand) → keep the raw viewBox.
   This guards against *under*-parsing producing a too-tight box that clips the drawing.
3. **Clamp** the content bbox inside the viewBox — never larger.

### Author control

- **Default on** for file-based SVG templates (margin consistency is the whole point).
- `[trim: false]` on the template definition or an instance forces the raw viewBox for
  any icon that trims wrong — an author is never blocked on a code change. (`[trim: true]`
  is also accepted/explicit.)

### Scope notes

- Aspect ratio: if the content bbox aspect differs from the instance `[width/height]`,
  the art stretches — same as today with the viewBox; trimming only changes the reference
  box, introduces no new stretching.
- Inline SVG templates and raster templates are unaffected.

## Testing

- **Fat-margin icon trims:** an SVG whose drawing occupies a small centered region of a
  large viewBox — after load, `svg_dimensions`/embed viewBox equal the content bbox, and
  an anchor (`.right`) sits at the drawing edge, not the viewBox edge.
- **Degenerate → fallback:** an SVG usvg can't meaningfully bbox (or whose bbox is
  near-empty) keeps its raw viewBox; render succeeds.
- **`[trim: false]`:** forces the raw viewBox even when a content bbox is available.
- **Clamp:** a content bbox computed larger than the viewBox is clamped.
- **Connector consistency (integration):** two icons with very different margins, each
  with a connector to a shared target, produce connector start points equidistant from
  their drawings (within tolerance).

## Docs

- `--skill-find-clipart` 2b: change from "crop the viewBox by hand" to "this is automatic
  (viewBox is trimmed to the artwork on import); only crop manually or pass `[trim: false]`
  if auto-trim degrades for a specific icon."
- `--grammar`: note the `trim` modifier on file SVG templates (default on).

## Non-goals

- Perfect bbox for every conceivable SVG (text-without-fonts, exotic filters) — those
  degrade to the viewBox.
- Trimming inline/raster templates.
- Adding a uniform decorative padding (zero-margin + routing gap already gives
  consistency; a padding knob can come later if wanted).
