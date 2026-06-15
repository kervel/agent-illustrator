# Pattern & Gradient Fills (+ stroke-dash docs) — Design

**Date:** 2026-06-15
**Status:** Approved (brainstorming) — ready for implementation plan

## Problem

AIL fills are solid colors only. There is no way to fill a shape with a
texture (hatching, dots) or a gradient. SVG supports both via `<pattern>` and
`<linearGradient>`/`<radialGradient>` defs, but AIL exposes none of that.

Separately, dashed/dotted strokes **already work** (`stroke_dasharray: "6,3"`,
or keywords `dashed`/`dotted`) but are **undocumented** in the agent-facing
docs (`grammar.md` STYLE MODIFIERS, `skill-styling.md`), so agents can't
discover them. This is a docs-only fix, bundled here.

## Goals

1. Named semantic pattern fills: `hatch`, `cross_hatch`, `dots`, `grid`.
2. Gradient fills: linear (`gradient`) and radial (`radial-gradient`).
3. Document the existing stroke-dash support so agents discover it.

## Non-Goals

- Arbitrary user-supplied SVG pattern markup (conflicts with semantic-over-geometric).
- Gradient strokes, textured strokes (dashed already covers stroke variety).
- More than the four named patterns above (YAGNI; extend later if needed).

## DSL Syntax

`fill:` accepts function-style values in addition to solid colors:

```
rect a [fill: hatch]                              # default colors
rect b [fill: hatch(accent-1)]                    # lines in accent-1
rect c [fill: dots(accent-1, background-light)]   # fg dots over bg
rect d [fill: gradient(accent-light, accent-dark)]    # vertical linear
rect e [fill: gradient(blue, white, 90)]              # angled, degrees
rect f [fill: radial_gradient(white, accent-1)]
```

- **Patterns:** `hatch`, `cross_hatch`, `dots`, `grid`.
  - Args: `name`, `name(fg)`, or `name(fg, bg)`.
  - Defaults: `fg = foreground-2`, `bg = transparent`.
- **Gradients:** `gradient(from, to)`, `gradient(from, to, angleDeg)`,
  `radial_gradient(from, to)`.
  - `angleDeg`: 0 = top→bottom (default), 90 = left→right, 45 = diagonal.
  - Radial ignores angle.

Colors inside the parens reuse the existing color parser, so symbolic tokens
(`accent-1`, `background-light`), hex (`#f00`), and named colors all work and
resolve to `var(--token)` the same way solid fills do.

## Architecture (Approach A — additive field + render-time registration)

### Parser (`src/parser/grammar.rs`)
Add a function-call style value: `ident ( arg, ... )` where args are existing
style values (colors / numbers). Produces a new `StyleValue` variant, e.g.
`StyleValue::Call { name: String, args: Vec<Spanned<StyleValue>> }`. Solid-color
`fill` parsing is unchanged.

### AST (`src/parser/ast.rs`)
Add the `StyleValue::Call` variant.

### Style model (`src/layout/types.rs`)
- Add `fill_pattern: Option<FillSpec>` to `ResolvedStyles` (additive — mirrors
  how `stroke_dasharray` is its own field). Solid `fill: Option<String>` stays.
- Define:
  ```rust
  pub enum FillSpec {
      Pattern { kind: PatternKind, fg: String, bg: String },
      Gradient { kind: GradientKind, from: String, to: String, angle: f64 },
  }
  pub enum PatternKind { Hatch, CrossHatch, Dots, Grid }
  pub enum GradientKind { Linear, Radial }
  ```
  `fg`/`bg`/`from`/`to` are already CSS color strings (`var(--token)` or hex/named),
  reusing `ResolvedStyles::color_to_css`.
- In `from_modifiers`, when `fill`'s value is a `Call`, validate name + arg
  count, build the `FillSpec`, and set `fill_pattern` (leave `fill` = None).

### Renderer (`src/renderer/svg.rs`)
- `SvgBuilder::register_fill(&mut self, &ResolvedStyles) -> Option<String>`:
  if `fill_pattern` is `Some`, generate a **deterministic content-based id**
  from kind + colors + angle (e.g. `pat-hatch-accent1`, `grad-bluewhite-90`),
  push the corresponding `<pattern>`/`<linearGradient>`/`<radialGradient>` def
  into `self.defs` **once** (dedup: skip if an id is already present), and
  return `Some("url(#<prefix>id)")`. Returns `None` for solid fills.
- Shape renderers (`add_rect`, `add_circle`, `add_ellipse`, `add_polygon`,
  `add_path`) call `register_fill` and pass the resulting override into
  `format_styles` (add an optional `fill_override: Option<&str>` param, or
  precompute and stash). Solid path → `register_fill` returns `None` → unchanged.
- Ids must respect the existing `prefix()` scheme used for the arrow marker so
  multi-diagram embedding stays collision-free.

**Determinism note:** ids are derived purely from content (kind/colors/angle),
never from HashMap iteration order. This dedupes identical fills and keeps
output byte-stable across builds — consistent with the project's prior
HashMap-nondeterminism fixes.

### Pattern/gradient def geometry
- `hatch`: diagonal lines (`<path>` strokes) in a tile, `patternUnits="userSpaceOnUse"`.
- `cross-hatch`: two diagonal line sets.
- `dots`: a `<circle>` per tile.
- `grid`: horizontal + vertical lines.
- Tile size and stroke width: fixed small constants (e.g. 8px tile, 1px line);
  `bg` rendered as a `<rect>` behind the pattern marks (omitted when `transparent`).
- Linear gradient: `x1/y1/x2/y2` computed from `angle`. Two `<stop>`s.
- Radial gradient: default center/radius, two `<stop>`s.

## Error Handling

Fail at parse/resolve with a clear, spanned message:
- Unknown function name → list valid: `hatch, cross_hatch, dots, grid, gradient, radial_gradient`.
- Gradient with fewer than 2 color stops → error.
- Pattern with more than 2 args, or gradient with more than 3 → error.
- Non-color where a color is expected (e.g. angle in color slot) → error.

## Testing

- **Parser:** each form — `hatch`, `hatch(c)`, `dots(c1,c2)`, `gradient(a,b)`,
  `gradient(a,b,90)`, `radial-gradient(a,b)`; plus error cases (bad name, too
  many args, gradient with 1 stop).
- **Resolution:** AST `Call` → correct `FillSpec` with defaults applied.
- **Renderer:** def emitted once; two shapes with identical fill share one def
  (dedup); id is deterministic across runs; shape carries `fill="url(#...)"`;
  solid fills emit unchanged `fill="..."`.
- **Example:** add one `.ail` in `examples/` exercising patterns + gradients,
  rendered by the pre-commit `render-all.sh`.

## Docs

- `docs/grammar.md`: extend STYLE MODIFIERS / COLORS with pattern + gradient
  fill syntax; add `stroke_dasharray` + `dashed`/`dotted` keywords.
- `docs/skill-styling.md`: new section "Pattern & Gradient Fills" with examples
  and guidance (when a texture conveys meaning vs. decoration); document dashed
  strokes.
- `docs/skill.md`: one-line cross-reference to the new styling capability.
