# Changelog

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
