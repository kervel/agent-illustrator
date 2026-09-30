# Changelog

## v0.2.19

- `--doc` knows more words: aliases (`move`, `pivot`, `stagger`, `timing`,
  `highlight`, `z-order`, `overlap`, `caption`, ...) and new sections
  (`layering`, `overlaps`, `captions`); an unknown topic suggests the closest
  ones. The brief's topic list comes from the same table.
- Brief and guide fixes from review: stage wording, later declarations draw
  on top, generated instances (`b* [items: ...]`), `move x to y` centres (use
  a `point` beside), `then` waits for everything, artwork hinges.
- `--frame` out of range says how keyframe indices relate to `--states` steps.
- Lint: a label on a card no longer "straddles" the edge of something the
  card itself covers (paint order is taken into account).
- An artwork's own drawing no longer shows as `x._art` in messages.
- Fix: the readability test suite did not compile since v0.2.14, so CI was red
  and those tests did not run; two broken tests repaired.

## v0.2.18

- `--skill-brief`: the whole method on one page (about 830 words against
  about 16,000 in the full guides): the loop with the steps agents skip
  (lint, --states, looking at PNGs), one complete scene, placing things,
  motion idioms, output.
- `--doc TOPIC`: one section of the guides at a time (`--doc tables`, `--doc
  accent`, `--doc verify`, ...); an unknown topic lists them all.
- Tests keep them honest: the brief's scene lints clean, every topic it names
  resolves to a section, and the brief stays under 1200 words.

## v0.2.17

- Fix: two accents on the same element in different tones (`[tone: error]`
  then `[tone: ok]` a step later) shared one mark, so the earlier one showed
  the later one's colour. Each style and tone now has its own mark. Covered
  by the playback-equivalence test.

## v0.2.16

- Fix: a part of SVG artwork turned about the wrong point. Part overrides
  (`barrier gate [arm.pivot: left]`) now reach SVG-file templates as they do
  inline ones, and static frames and stills rotate an embedded part in page
  space about its pivot (the same point as the player; no skew when the
  artwork is scaled unevenly). A merge gate's arm now swings up from its post.
- Part overrides (`part.key: value`) are no longer reported as unknown
  modifiers.

## v0.2.15

- Fix: the first keyframe applied all its changes before it started, so what
  it does later (a `set` behind `when mr shown`, a transform after `then`)
  never animated. Frame 0 now sets the scene only with what happens at its
  start; the rest plays.
- Fix: an accent's marks settled visible after they played, so a held accent
  (and any accent) showed again in later steps. They settle hidden, like
  flash, highlight and ping.
- Accent defaults from real use: only text-like things (text, rows, code
  lines, label-only boxes) are underlined, a filled or bordered box is
  outlined; the error "!" badge sits just outside the right edge, level with
  the element (it says which row it means).

## v0.2.14

- `accent x`: one verb for "look here". The style follows the element (a
  label, row or code line is underlined, something small ringed, a panel
  outlined; `style:` overrides, `wiggle` too); the tone says why
  (`attention` / `error` with a "!" badge / `ok`, as role colours); `hold:
  step` keeps it until the next click. Drawn over the element, moves nothing,
  leaves nothing behind; the no-JS picture is unchanged, and it plays the same
  in the player, the CSS loop and PNG stills. An accent on something hidden
  is linted.

## v0.2.13

- `table`: a small datagrid as one element (`columns:`, `rows:` as lists or
  records with `keys:`, `widths:`, `font_size:`, `mono:`). Square flush cells,
  header band, rules, theme roles; rows are parts (`orders.row[2]`,
  `orders.rows[2..4]`, cells `orders.r2c1`) for highlight, transform and
  appears (G24). Value lists may hold lists (`[["a", "b"], ["c", "d"]]`).
- `corner_radius: 0` is written out, so a stylesheet's default rounding no
  longer wins over an explicit square corner (G25).
- An empty label is not a label for the label-overlap lint (G23).
- PNG: labels sit where the browser puts them (the vertical centring of
  `dominant-baseline: middle` is applied as a shift; text sat a few pixels
  high before).

## v0.2.12

- Fix: a flying ghost of SVG artwork could glue two attributes together
  (`stroke="…"stroke-width="3"`) where the artwork file broke a tag across
  lines: invalid XML, so standalone SVG and `--png` failed (G22). New test:
  every frame, mid-motion still and animation of the example scenes parses as
  XML.
- The "hidden the whole keyframe" lint no longer fires on something shown
  (and hidden again) within the frame, or flown as a traveller (G20).
- `fly X from A to B` keeps the traveller's size by default; only a
  `fly ghost(...)` snapshot shrinks to fit its target (G21).

## v0.2.11

- The static binary is the whole loop: the Linux release binaries are now
  static musl builds (x86_64 and aarch64), checked static in CI, and CI runs
  lint, --states, every frame and a mid-motion still to PNG in a bare Alpine
  container without network. README: a one-line install.
- `--png` is a plain switch and `-o FILE` names the output (SVG or PNG); with
  `--frames-to-dir` it writes one PNG per frame. (`--png FILE` swallowed the
  input file name.)
- `pivot: left | right | top | bottom | top_left | ...`: what rotation and
  scaling turn about (a lid's hinge), in the player, static frames and PNGs
  alike (G7).
- PNG notes only when a font is really missing (a web @import of a font that
  is bundled anyway is not worth a note).
- Docs: "Verifying without a browser" in --skill and --skill-animation.

## v0.2.10

From three Kubernetes scenes written from scratch (tests/examples):
- Named connections in a template body are parts of each instance
  (`acme.w_front`); two instances no longer clash (G9).
- `{param}` is filled in inside any string modifier (`label: "release:
  {rname}"`), not only in `text "{title}"` (G14).
- Showing the parts of a container that is still hidden is linted (it did
  nothing, silently) (G6).
- PNG: `@font-face` WOFF/WOFF2 data URIs are unpacked and registered under
  the stylesheet's family name (the Kapernikov Avenir renders); a symbol
  fallback font is bundled (✓ ✕ ★ ⚠ …, a 143 KB Noto Sans Symbols 2 subset);
  characters no font has are reported (G13, G18).
- `set x default` reverts what a state did through a selector (`meter.*`),
  and `[prop: initial]` resolves for several targets at once (G17).
- Lists wherever one selector was taken: `move a, b, needs.* to box [stagger:
  0.07]`, `transform m1, m2 [...]`, `fly ghost(a, b) to box` / `fly ghost(a),
  ghost(b) to box`; one staggered statement is one gesture for the busy-beat
  lint (G1, G15, G4).
- `point` works inside templates (G2).
- `when x arrives` also covers a `show x [from: y]` flight and a transform
  that moves, turns or resizes x (G3, G10).
- `overlaps:` on a template instance applies to its parts (G5).
- The overlap lint ignores a container whose parts are all still hidden (G8).
- The station lint ignores a connection between two parts of the element
  shown, and reports the time a flight lands (G12).
- Monospace labels wrap by the width that sized them: a code block's longest
  line no longer wraps (it showed in yaml blocks) (G11).
- Crowded-row lint only for mixed rows: a row of identical things is what a
  row is for (G16).
- An embedded SVG's own drawing is named `x__art` / `x__self` and shown as its
  part (no more `on_test.art.art`) (G19).
- New lint: an outline around other elements that pops or grows in scales its
  border through them (looks like stray lines); use `enter: draw`.
- Docs: objects (box, laptop, server, person, cloud, document, database)
  are best embedded SVG artwork, not built from rects; the swarm idiom;
  what `arrives` covers.

## v0.2.9

- Fix (v0.2.8 regression): `through:` lines disappeared behind a slide's
  stage in every git scene (the README images, the no-JS/GitLab view, frames).
  A through-line is no longer sent to the back: it keeps its place and moves
  only just under the first sibling that holds one of its stations. A test
  pins "over the stage, under its stations".
- `--png [FILE]` (with `--frame`, `--at`, `--frames-strip`, `--frames-to-dir`)
  and `--scale`: PNG output without a browser. The SVG's CSS (custom
  properties, class rules, individual transforms, `pathLength` dashes) is
  flattened and rendered with resvg, with Overpass and Overpass Mono bundled
  (OFL) and `@font-face` data URIs from the stylesheet; a missing font is
  reported. First version: static musl builds, prebuilt binaries and docs
  follow.
- The document icons of git-copies have a really cut corner (no backdrop
  triangle); docs say artwork must never paint the background to fake a hole.

## v0.2.8

- `clip: bg`: a part is drawn only inside another element's shape, within its
  stroke and following its rounded corners. A window's header bar is a plain
  rect that never covers the border and leaves no corner notch. Code blocks'
  title bars use it; git-history's machines and git-snapshots' folder too.
- `padding: 0` on a row/column: children flush with its edge (default 5).
- A `through:` line runs under the stations it passes through, whatever the
  declaration order; `z_order:` now works on any shape among its siblings.
- The station lint counts `show x [from: y]` as arriving when it lands.
- Push/pull recipe (and git-history): the commit itself flies and stays while
  the line grows to meet it, instead of a ghost that lands and a dot that pops.
- `--states`: each step's duration (marked slow past 2.5s a frame), and a note
  when the content sits off-centre on the stage or uses under half of it.
- `jitter: rotate(4)` (degrees) / `move(6)` (px) documented; a bare number
  means rotate.
- --skill-animation opens with "your first scene in 10 minutes", a complete
  scene checked by a test.

## v0.2.7

- `--animate-css` now plays the compiled choreography (draws, pops, flights,
  flips, swaps, layout moves) as one looping pure-CSS timeline, sampled from
  the native renderer and reduced to the keys interpolation needs (50–160 KB
  for the git scenes). It plays inside an `<img>`, so a README can show it.
  Before, it was a step-by-step slideshow of the settled frames.
- A labelled shape is auto-sized for the widest wording a keyframe gives it
  (`transform cmd [label: "git commit"]` no longer overflows a box sized for
  "git clone").
- README leads with the git scenes (self-playing), a scene's motion source and
  its `--states`, then a curated diagram gallery. examples/ holds showcases
  only; feature fixtures moved to tests/fixtures. examples/renders.txt lists
  every committed image; render-all.sh renders from it and a test fails when
  a committed image is stale.

## v0.2.6

From a newcomer's scene (git-history: clone, commit, push, pull), now in
examples/motion.

- Station spacing no longer misfires on plain dots: a component's other parts
  count as a station's name only when it holds a single station of the line,
  dots a row already spaces are left alone, and room-for-names and `spread:
  even` are weaker than any constraint the author writes (they never move a
  component off its pin). Also makes `hub.center_x`/`hub.top` on a
  standalone instance reliable again.
- `midpoint(a.bottom, b.top)`: the middle of a gap between edges. A `(` in a
  constraint says there is no arithmetic and points at midpoint.
- Instance part overrides: `machine anna [hist.appears: later, bg.fill:
  role-ok]`; `appears:` (and `opacity:`) on a nested instance now apply.
- The line-before-station lint only counts what a line is declared to pass: a
  `through:` path's stations, a connection's end (fix: `when l shown`). A
  caption sitting on a link is not a station.
- Lint messages name parts with dots (`hub.bg.left`, not `hub_bg.left`).
- `ail:motion/git` macros time with `delay:` and `then`; docs say how to time
  inside a macro, and that rows/grids are for regular arrangements.

## v0.2.5

Readability: a scene says what it means, so an author (or an agent) can read
the motion section as the storyboard.

- Timing by events: `when main_line reaches a.dot { ... }`, `when x shown`,
  `when x hidden`, `when x arrives` (+/- a nudge), and named beats (`beat open
  { ... }` + `after open + 0.2 { ... }`). `then` and `at` stay; `after 0.3`
  still works, and two or more in a row are linted (each counts from the one
  before). A station that appears before its line reaches it is linted.
- Named layouts: `layout beside { constrain ... }`, switched with `use layout
  beside` / `use layout default` (with timing options). Replaces pairs of
  named constraints disabled and enabled by hand.
- Component states: `state done { ... }` in a template (motion statements on
  its parts, with its parameters), entered with `set review done [opts]`;
  what another state changes and this one does not goes back to the
  template's look. `transform x [fill: initial]` returns a property to its
  declared value.
- States on one element: `state status broken { ... }` at top level; `set x
  default` returns any component to its declared look.
- Constraints over starred lists: `constrain k*.pg.center = d*.pg.center`
  (one per index).
- `--timeline` prints each `when ...` with its resolved time, and a layout
  switch as `use layout beside`. Events are documented as the moment
  something finishes; `arrives` also covers a layout change.
- Records for starred instances: `k* [items: [{fname: "a.py", c1: role-ok},
  ...]]` instead of parallel lists.
- `point hub`: an invisible place with no size (instead of a 1x1 rect).
- One title: `motion [title: s.heading, title_swap: roll]` makes the heading
  show each keyframe's `[title: ...]`.
- `--states`: the storyboard per click step: a visibility matrix, then what
  each step shows, hides, transforms, moves, draws and which layout it uses.
- `transform x [fill: initial]` passes the colour lint.
- An instance's `opacity:` (`check tests [opacity: 0.3]`) now applies to the
  instance; it was dropped, so the merge request's checks were never faint.
- Every motion scene is gated fully lint clean under git-deck.css and
  kapernikov.css in the test suite.
- Lints: `appears: <frame>` plus a `show` in that frame (say it once: `appears:
  later` at the element, the verb for how); `swap a -> b` where b is not where
  a is (for `flip: part`, that part).
- The four git scenes are rewritten with these (no summed offsets, no
  enable/disable pairs, no helper rects, no parallel lists), lint clean.

## v0.2.4

- No more one-frame flash of the old page at the end of a flip (seen on a GPU
  desktop Chrome). The outgoing page was hidden in the same instant its
  turned-away hold ended and its width reset to full (its name came back
  too); a paint between the two showed the old page, full size, under the new
  one. The rule now holds both ways: nothing about an element changes at the
  instant it becomes hidden, or while it is hidden (a transient's reset is
  left to the frame's atomic settle); v0.2.3 did the showing side.
- Tests: the playback check sees an element as hidden when an ancestor is,
  and a new test asserts that no step starts on an element or its parts at
  the instant it is hidden, in all four example scenes.

## v0.2.3

- No more one-frame flash of a full-size page at the start of a flip. When a
  frame starts playing, anything hidden takes its first animation's starting
  value at once (`fill: both` on that channel's first animation): an element
  never becomes visible before its geometry is in place. The incoming page of
  a flip held scale 1 until its zero-width hold began, and a paint that showed
  the opacity step before the hold was a full-size flash.
- `RenderConfig.sample_at`: many `--at` samples from one compile.
- The playback test samples both sides of every animation start, compares only
  what is visible, and runs in about 5 seconds.

## v0.2.2

- The browser player now plays the opening half of `swap a -> b [via: flip]`.
  The export combined that half with the value the previous overlay (the
  edge-on hold, scale 0) left behind, so it animated 0 -> 0: every page was
  blank for the second half of its flip and appeared at full width at the end.
  Native stills were right, so the seek tests could not see it.
- A flip eases as one turn: the closing half takes the ease-in form of the
  statement's ease, the opening half its ease-out form, so edge-on is an
  instant.
- New test: the manifest's animations are replayed with the Web Animations
  rules the player relies on and compared with the native sampler over time
  (a flip, `show [from:]`, draws, lines opening and closing), plus a check that
  a flip always shows at least 20% of a page outside its edge-on instant.

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
