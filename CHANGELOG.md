# Changelog

## v0.2.1

- Template instances pass every argument through. Arguments whose key is a
  built-in style key (`font_size`, `opacity`, `label_fill`, `align`, ...) were
  silently dropped unless they were one of fill/stroke/stroke_width/size/
  width/height/label/rotation: `code c [font_size: 24]` did nothing.
- A code block's title bar scales with its `font_size`.
- Scene 5 (examples/motion/git-merge.ail) reworked for slides: the copies side
  by side, then the merged file large (24px code) with the choice beside it;
  at 1200x675 cropped the code reads at 22px (the copies at 17px).

## v0.2.0

### Code, artwork, themes (the author/integrator round)
- `code` blocks: `code c [lang: python, source: "..." | file: "x.py", lines: "3-8"]`
  with syntax highlighting (syntect: python, js/ts, css, sh, json, yaml and the
  other Sublime syntaxes), line numbers, a title bar, `marks:` tints, and diff
  mode (`diff: "-a\n+b"`: tinted -/+ rows, the changed part marked). Lines are
  parts (`c.line4`, `c.line[4]`, `c.lines[8..12]`); `transform c.line1 [source:
  "..."]` re-highlights; `remove` / `insert c after line 7 [...]` /
  `show x [enter: expand]` open and close lines with the block and everything
  below following. Theme tokens `code-*`.
- SVG file templates with parts: every drawable element with an `id` is a part
  (`d.fold`, `d.bar3`), nested as in the file, usable in constraints, anchors
  and motion; CSS variables in the file are instance arguments; ids are
  namespaced per instance; a part's paint can be transformed.
- Colour roles (`role-primary`, `role-ink`, `role-surface`, `role-ok-soft`,
  `role-series-1`, ...) mapped by themes; `ail:motion/git` and the example scenes
  use roles only and render unedited in git-deck.css and kapernikov.css.
- Keyframe host metadata `[title: "...", note: "..."]` in the manifest; the
  player's step events carry them (`p.meta(k)`).
- `--crop-to-content [pad]`, `--stylesheet-css` repeatable (layered),
  `--serve [port]` live preview, `--film STEP` browser film strip.
- `collapsed: true`, `overlaps: <element>`, `hide [exit: collapse]`,
  `show [enter: expand]`, `--ail-mono-advance` stylesheet token.

### Fixes in this round
- A stylesheet's `@import` (web fonts) is now the first rule of the SVG's style
  block; it used to come after the generated rules and was ignored, so decks
  never got their font.
- `font_family` / `font_weight` and `corner_radius` are inline styles: a
  stylesheet's `.ai-label { font-family }` or `.ai-rect { rx }` no longer beats
  what an element asks for (mono labels came out in the brand face).
- Moving a component releases constraints that place it through a part
  (`anna.bg.right = ...`); a size-only transform keeps position constraints.
- Custom stylesheet colour values override the default palette's for colour
  decisions (light text on dark fills).


### Fixes from the discoverability review and the reveal-deck integration
- Swapped labels (`transform x [label: ..., swap: ...]`) render their variants in
  a wrapper `<g class="ai-label-variants">` that carries their colour, so a
  host rule like `[fill=dark] + .ai-label` no longer turns them white on white.
  Labels on a dark fill (judged by the fill's actual colour in the active
  palette) get class `ai-on-dark` and light text from a generated rule, the
  base text and every variant alike. Under the default stylesheet, labels on
  dark shapes are now light (e.g. the railway examples' stations).
- Output is deterministic: solved coordinates are rounded to 1e-4 px before
  anything is derived from them (the solver's hash-ordered pivoting used to
  change the last bits from run to run).
- The player has a step API (`steps`, `step`, `nextStep`, `prevStep`,
  `goToStep`, `on('step')`): a step is a click frame plus the `[auto]` frames
  after it. `--list-steps` prints them. Built-in clicks/arrows step.
- Without a player the animated SVG shows frame 0 as it ends (moves and draws
  included), for no-JS hosts, `<img>` and print.
- Lint: a `draw` that shrinks a line (usually a line declared without
  `drawn: 0`).
- Station spacing counts a station's own `label:`; the crowding hint says what
  to change (row gap vs free x).
- `f.val` names the only part of a one-shape template (it was undefined).
- A reserved word used as a name (`path line`, `rect row`) is an error at the
  name; it used to vanish silently into a second declaration.
- `ail:motion/git` ships templates its macros fit (`git_station`, `git_file`),
  uses `accent-1` instead of a deck-only colour, and is printed at the end of
  `--skill-animation`. A macro argument missing a part is explained at the call.
- Docs: geometry animation by name (`move`) instead of `dx`/`dy`; grammar lint
  section, colours, `font_size` default, `center`, keyframe flags, list
  arguments, reserved words, new CLI flags; `--skill` points to motion;
  `--examples` motion example is self-contained.
- Golden frames: every settled frame of the three example scenes is compared
  with a reviewed SVG (`tests/golden/motion`, `AIL_UPDATE_GOLDEN=1` regenerates).

## v0.1.32

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
