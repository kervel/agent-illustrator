# Changelog

## Unreleased

### Motion (new)
- Motion is first-class: `keyframe` blocks take motion statements (`show`, `hide`,
  `transform`, `draw`, `fly`, `move … along`, effects, `loop`, `count`, `swap`,
  `camera`) with `then` / `after N` / `at N` beats, selectors with `stagger`,
  timing tokens and macros (`motion name(...) { }`, `import "ail:motion/git"`).
- `--animate` embeds a motion manifest and a dependency-free WAAPI player;
  `--frame N --at T` renders a mid-motion still from the same tracks;
  `--timeline`, `--timeline-json`, `--frames-strip`, `--player-js`.
- Motion lints: numeric coordinates in motion, slow or busy beats, motion on
  hidden elements. `--lint` exits 1 only on errors; `--lint-strict` on any finding.

### Layout
- Lines routed `through:` stations give side-by-side stations room for their
  names (the line stretches); `spread: even` spaces a flat run evenly.
- Nested templates resolve and are solved innermost first; dotted paths
  (`a.b.c`) work everywhere.
- `contains` works inside templates; grid cells centre template instances;
  rows/columns re-flow after constraints; `pack: tight`; `align: <member>`;
  `canvas: true`; `max_width:` label wrapping (also for labels a keyframe swaps in);
  `caption_of`; `text "{param}"`; list-driven instances `name*`.
- Monospace labels are measured exactly (0.6em per character).

### Behaviour changes that move existing diagrams
- **Unsized text now renders at 14px.** Layout always measured text without a
  `font_size` at 14px, but the SVG did not say so and browsers drew it at 16px:
  about 14% wider than every box and viewBox. The root `<svg>` now carries
  `font-size="14"`. Diagrams that rely on the default size render smaller text,
  and text no longer runs out of its box or off the image.
- `a.center = b.center` aligns both axes (it aligned only x).
- Template instances in a grid are centred in their cells.
- Nested constraints are solved innermost first, and a constraint on a child of
  an instance moves the whole instance.
- **Every name must be unique, and a clash is an error.** Part `b` of instance
  `a` is named `a_b` internally (`a.b` in source), so an element you name `a_b`
  yourself, two declarations of one name, a connection named like an element, or
  two keyframes with one name used to collapse silently into one (the other was
  gone from the render). All of these now fail with both declarations located.
  Diagrams that relied on the collapse must rename one.
- Unanchored orthogonal connections pick an edge pair; vertical routes are
  preferred when the vertical gap is larger.
- Measured on a 24-diagram deck (before the font-size fix): 21 pixel-identical,
  3 shifted by 5-10px (spacing only).

### Lint
- New: text cut off at the edge of the image (every diagram, not only on a
  canvas); two texts touching on one line; a `through:` line running under a box
  it does not pass through; stations on a line closer than their names need;
  a keyframe's swapped label overflowing its box (wrapped as rendered).
- Less noise: a canvas or backdrop panel is not an obstacle; a ring or
  translucent wash around something is not a collision; S-routes that are the
  only clean route (or part of a fan-out) no longer ask for anchors.
