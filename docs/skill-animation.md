# Animation Skill

Create keyframe animations in Agent Illustrator. Read `--skill` first for general AIL usage.

## When to Use

Use this sub-skill when creating **multi-frame animated diagrams** — sequences where
elements appear, disappear, or transform over time. Not needed for static diagrams.

---

## Part 1: Animation Harness (MANDATORY)

This workflow is **not optional**. Skipping steps produces bad animations.

### Phase 1: Storyboard

Before writing ANY code, write a storyboard as a block comment:

```
/* STORYBOARD
   Frame 1 "name": What is visible? What just happened?
   Frame 2 "name": What appears? What disappears? What moves?
   ...
   Frame N "name": Final state.

   STORY ARC: What narrative does this tell?
   PACING: Which frames need emphasis (longer dwell)?
*/
```

Each frame must tell a clear step in the story. If a frame doesn't advance
the narrative, cut it. If a transition is confusing, add an intermediate frame.

### Phase 2: Element Census

List ALL elements that will exist in the animation. For each element, specify:
- Is it always visible, or does it appear/disappear?
- Does it transform (move, rotate, change color) between frames?
- What connections does it participate in?

```
/* ELEMENT CENSUS
   PERSISTENT (always visible):
     cli, llm, tool — the actors
   TRANSIENT (appear/disappear):
     request_msg — visible in frames 2-3
     response_msg — visible in frames 4-5
   CONNECTIONS:
     cli.right -> llm.left as send_req — visible in frame 2
*/
```

### Phase 3: Build Incrementally

Do NOT write the full animation at once. Build frame by frame:

1. Write all PERSISTENT elements + constraints. Render. Verify layout.
2. Add TRANSIENT elements for frame 1. Position them. Render with `--frame 0` (0-indexed).
3. Add keyframe blocks one at a time. After each, render that frame and check.
4. Only after all frames render correctly, test `--animate` for the full sequence.

### Phase 4: Lint Pass (MANDATORY)

Run `--lint` and fix ALL warnings before proceeding. The linter is
keyframe-aware: things never visible at the same time are not reported as
colliding, and each defect is reported once, naming the frames it occurs in.

```bash
agent-illustrator --lint file.ail 2>&1 | grep '^lint:'
agent-illustrator --lint --lint-exclude redundant-constant file.ail   # narrow it
```

**Fix these categories immediately:**
- `alignment:` — near-horizontal/vertical connections off by a few pixels.
  Fix by constraining positions to match (e.g., `constrain a.center_y = b.center_y`).
- `connection:` — arrows crossing unrelated elements. Re-route or reposition actors.
- `redundant-constant:` — repeated magic numbers. Use element references instead.
- `reducible-bend:` — unnecessary bends in connections. Align elements to simplify paths.

A warning tagged `[frame: x]` occurs only in that frame — render it and look.
An untagged one is present in every frame.

Then read `--timeline`: every statement, when it starts, how long, and what it
moves from where to where. It is the cheapest way to catch a beat in the wrong
place, before any image exists.

### Phase 5: Frame-by-Frame Verification (MANDATORY)

After writing all keyframes, render EVERY frame as a static image and check each one.
Delegate this to a subagent to avoid bloating the main context with image data:

```bash
# Render every frame in one command: out/00-startup.svg, out/01-request.svg, ...
agent-illustrator file.ail --frames-to-dir out/

for svg in out/*.svg; do
  google-chrome --headless --screenshot="${svg%.svg}.png" --window-size=2400,1800 \
    "file://$(pwd)/$svg"
done
```

IMPORTANT: Use headless Chrome, NOT rsvg-convert. rsvg-convert does not support
CSS custom properties (var(--color)), so all themed colors render as black.

For the motion *between* frames, render contact sheets
(`--frames-strip <frame>`: 0/25/50/75/100% in one SVG) or single stills
(`--frame N --at 50%`).

The subagent should check each frame PNG for:
- Correct elements visible (not too many, not too few)
- No orphaned connections (arrow visible but target hidden)
- No overlapping transient elements
- Labels readable

Return a text-only PASS/FAIL report per frame (no images in main context).

### Phase 6: Evaluator Round (MANDATORY — NOT OPTIONAL)

After frame-by-frame verification passes, spawn a review subagent. The subagent
reviews the **full animation** by viewing all frame PNGs in sequence.

Subagent prompt:

> Review this animation sequence. The frames are shown in order.
> The original intent was: [paste storyboard STORY ARC here]
>
> For each frame, score PASS/FAIL:
> 1. VISIBILITY — Correct elements shown/hidden for this story beat
> 2. LAYOUT — No overlaps, readable labels, good spacing
> 3. CONTINUITY — Transition from previous frame makes visual sense
> 4. NARRATIVE — Frame advances the story clearly
>
> Verdict: ALL frames must PASS all criteria. List specific fixes needed.

If the evaluator finds issues, fix them and re-run from Phase 4 (lint).
Maximum 3 evaluator rounds. If still failing, present issues to user.

---

## Part 2: Animation Patterns

### Story-Driven Animations

Good animations tell a story. Each frame is a "scene" with:
- **Entry**: New elements appear (connections + data envelopes)
- **Focus**: The active interaction is highlighted
- **Exit**: Previous scene elements fade or hide

Bad animations just toggle visibility randomly. Plan narrative flow.

### Message Envelopes

For protocol/interaction animations, use "message envelopes" — labeled rects
that represent data in transit:

```
rect msg [width: 180, height: 60, fill: accent-light, stroke: accent-1,
          stroke_width: 2, opacity: 0.3, label: "POST /api/chat"]
```

Position envelopes BETWEEN the actors they travel between:
```
constrain msg.center_x = midpoint(sender, receiver)
constrain msg.center_y = sender.center_y
```

### One Caption, Not Ten Text Elements

A narration line that changes per frame is ONE text element rewritten by each
keyframe — not one element per step:

```
text "the client sends a request" caption [align: start]
constrain caption.center_x = stage.center_x
constrain caption.y = stage.bottom + 24

keyframe "arrive" { transform caption [label: "the server receives it"] }
keyframe "reply"  { transform caption [label: "and answers"] }
```

Leave the box auto-sized: a caption a keyframe rewrites is laid out for the
longest wording it ever takes, so it never resizes — and never shifts —
mid-animation. `align: start` keeps the text against its left edge. Ten stacked
text elements would cost their own constraints, their own show/hide rules, and a
pile of lint noise.

### Named Connections for Keyframe Control

ALWAYS use named connections (`as name`) when the connection's visibility
changes across frames:

```
cli.right -> llm.left as send_request [stroke: accent-1, stroke_width: 3]

keyframe "request" {
    show send_request    // Can reference by name
}
```

### Visual Hierarchy Across Frames

Use transforms to draw attention to the active part of the story:

```
keyframe "tool_execution" {
    transform cli [opacity: 0.4]      // Dim inactive actors
    transform tool [opacity: 1.0]     // Full opacity for active actor
    show exec_arrow, exec_result
}
```

### Clipart and Rich Visuals

For animations that need to look like a real product or tell a compelling story,
don't settle for plain rectangles. Use `--skill-find-clipart` to find and embed
SVG clipart for actors (people, servers, terminals, etc.). File-based SVG templates
bring the animation to life.

### `--animate` vs `--animate-css`

`--animate` embeds the motion player: full choreography, autoplaying, click or
arrow keys to step, only when the SVG is opened directly (JS does not run in an
`<img>`). For a GitHub README use `--animate-css`: a self-cycling pure-CSS
animation of the settled frames (visibility and geometry, not the timed motion).

### Geometry Animation (Position & Size)

Say *where* something goes by name, never by a number. Numbers break the moment
a label is translated or the layout shifts; names keep working.

```
keyframe "merge" {
    move chip to slot_b                    // chip travels to slot_b's centre
    transform box [width: 340]             // sizes are fine as numbers
}
keyframe "reset" {
    move chip home                         // back to where the layout put it
}
```

- `move x to y` / `move x home` / `move x along path` move an element (and its
  label) by name. `fly ghost(x) to y` sends a copy instead (see Travel below).
- **Size** (`width`/`height`/`scale`) tweens via the shape's geometry; `scale` is
  about the centre.
- `x`/`y`/`dx`/`dy` in a `transform` still work in a plain state keyframe (show/
  hide/transform only), but lint warns; in a keyframe that uses motion (beats,
  timing options, draw/fly/move/effects) they are an error (`motion-coordinate`).

To make the solver place dependents relative to a moved element, change the active
constraints in the keyframe: name the original constraint and `disable` it, then add
a new one *relative to another element*:

```
constrain chip.center_y = lane_a.center_y as chip_home
keyframe "merge" {
    disable chip_home
    constrain chip.center_y = lane_b.center_y   // re-pin by name; chip tweens there
}
keyframe "reset" {
    enable chip_home                     // restore the original pin; chip tweens back
}
```

Without this, the always-solved constraints pull elements back to their frame-0
positions. Children constrained relative to a resized/moved parent cascade to new
solved positions and each tweens (the box-of-chips grows and recenters as a unit).
(`constrain chip.center_y = 120` inside a keyframe also works, but lint warns: it
pins to a coordinate.)

**Under-constrained elements.** Each frame is re-solved from the frame-0 base layout,
so an element with no active constraint (e.g. you `disable`d its pin without re-pinning,
and no other constraint references it) holds its frame-0 laid-out position. There is no
"undefined position" case — the base layout is always the fallback — so no lint is
needed for it. To leave an element where a *previous* keyframe moved it, keep that
keyframe's `constrain` (or transform) active; cumulative state carries it forward.

### Connections follow automatically

Connections re-anchor to their endpoints every keyframe — move or resize an element and
any connection touching it follows on its own (no extra constraints, no naming required).
It slides smoothly when the route keeps its shape (Chrome/Safari) and crossfades when the
route reshapes; either way it lands correct at each keyframe. (Firefox holds a sliding
connector at its start route; reshaped/crossfaded ones animate everywhere.)

---

## Part 2b: Motion (timing, choreography, verbs)

Keyframes say *what* each frame looks like; motion statements say *how the
picture gets there*. Everything below compiles to explicit tracks after layout:
the browser player and the native renderer play the same tracks, so a frame
reached by jumping looks exactly like the same frame reached by playing.
**Motion statements never contain coordinates** — they name elements, anchors,
percentages and vertices, and survive relabelling and relayout.

### Motion principles

1. **One focal movement per beat**, with at most one supporting movement.
   (`--lint` reports a *busy beat*: more than 2 flights/moves/draws/swaps at once.)
2. **Appear with a little overshoot, move with in-out, leave faster than you
   arrive.** The defaults do this: `show` rises in (`settle`), `pop` overshoots,
   `move`/`fly`/`draw` glide, `hide` fades in `fast`.
3. **Backward is instant.** Stepping back or jumping lands on the exact state;
   only forward plays.
4. **Build the story from simple states**; a line that grows (`draw`) is the main
   device for "this happened next".
5. **Keep a beat under 2.5s** (`--lint` reports a *slow beat*). Split long beats,
   and let a follow-on keyframe play on its own with `[auto]`.

### Timing: every statement takes `[delay, duration, ease, stagger, order]`

```
show title [enter: rise, duration: slow, ease: settle, delay: 0.1]
show docs.* [enter: pop, stagger: 0.08, order: random]   // start|end|center|random (seeded)
```
Durations: seconds or `fast | normal | slow`. Eases: `pop` (overshoot), `settle`
(power3-out), `glide` (in-out), `snap`, `in`, `linear`. Both are **theme tokens**:
retime a whole deck in its stylesheet CSS —
`:root { --ail-motion-normal: .4s; --ail-ease-pop: cubic-bezier(.3,1.8,.6,1); }`.

### Beats

Statements in a block start together. Beats sequence them:
```
keyframe "commit" {
    flash folder                                  // starts at 0
    then { fly ghost(folder) to st1.dot }         // when everything before it has ended
    after 0.15 { show st1.nm [enter: rise] }      // 0.15s after the previous beat STARTED
    at 1.2 { pulse st1.dot }                      // 1.2s after the keyframe started
}
keyframe "next_step" [auto, after: 0.3] { ... }   // plays by itself 0.3s after the previous one
```
`after` chains off the previous beat's *start* (that is how beats overlap);
`then` waits for everything before it; `at` is absolute. A keyframe's length is
derived, never declared.

### Entrances and exits

```
show x [enter: fade | pop | rise | drop | grow | wipe(left|right|up|down) | draw]
show copy [from: original]        // appears at original's place and size, travels home
hide x [exit: fade | shrink | fall | lift | wipe(...)]
motion [enter: pop, exit: fade]   // diagram-wide defaults (default: rise / fade)
```
Declare where an element enters instead of hiding it in the first frame:
`rect ring [..., appears: go_back]` (hidden until keyframe `go_back`, where it
enters with the default preset unless that keyframe says `show ring [...]`), or
`appears: later` (hidden until some statement — a `swap`, a macro — shows it).
Frame 0's own `hide`s apply before it plays; its `show`s are entrances.

### Lines that grow

```
path track [through: [st1.dot, st2.dot, st3.dot], drawn: 0, stroke: accent-1, stroke_width: 14, fill: none]
path branch [through: [c.dot, l1.dot, l2.dot, m.dot], routing: metro, ...]   // 45-degree runs
draw track [to: st2.dot]      // or [to: 60%], [to: vertex 2]; `draw track` = all the way
undraw track [to: st1.dot]
a -> b as feed
show feed [enter: draw]       // a connection that draws itself in
```
A line is drawn as declared (`drawn: 0 | 60% | <element on it>`, default fully),
and a later `draw` never changes how it starts. Drawing is cumulative across
frames. Dashes survive (a mask does the drawing), an arrowhead appears as the tip
arrives. Drawing to something that is not on the line is a compile error.
`path … [through: […]]` is routed after layout through the centres it names, so
stations stay on the line whatever moves; `extend`, `extend_start`, `extend_end`
run it past its ends.

List every station in `through:`, not just the ends. Neighbouring stations
that sit side by side are then kept far enough apart for their names (the
engine pushes the later one along and whatever is placed off it follows), and
`spread: even` puts the stations of a flat run at equal steps. Pin the first
station and the height of a run; leave the rest free. A station pinned too close,
or packed in a `pack: tight` row, is reported by lint rather than moved.

### Travel

```
fly ghost(folder) to st1.dot [scale: 0.25, arc: 0.15]   // a copy flies and vanishes; default scale fits the target
fly ghost(hub.d4) to anna.d4, chris.d4 [stagger: 0.1]   // one copy per destination
fly token from a to b                                    // a real (hidden) element as the traveller
move ring to st2.dot            // persistent: dependents re-solve around it
move d0 home                    // release a `move`
move train along track [to: st3.dot]
```

### One-shot effects (leave nothing behind)

`pulse x [scale: 1.3]`, `shake x`, `nudge x [direction: up]`, `flash x`,
`ping x`, `highlight x [color: accent-1]`; ambient: `loop x [pulse, period: 1.2]`
(runs while the frame is on screen). `jitter: 4` on a `show` gives each target a
small seeded tilt (`jitter: move(6)` shifts instead) — a pile, not a table.

### Text and numbers

```
transform cap [label: "Approved by Ben", swap: roll | fade | cut]
count total [to: 14900, format: "€ {:,}"]     // {} {:,} {:.1} {:,.2}
swap docs.* -> codes.* [via: flip, flip: pg, stagger: 0.07]   // flip only member `pg`, crossfade the rest
swap ok -> bad [via: fade | flip | morph]
```

### Selecting many things

`group.*` (its named children, in order), `.class`, `a.b.c` (a part of a nested
component), `all except title, stage`. Paired selectors pair by index
(`swap docs.* -> codes.*`).

### Camera

`camera focus card [zoom: 1.4]` / `camera reset` (persistent, animated).

### Reuse: motion macros

```
import "ail:motion/git"            // built-in library; or import "deck.ail" (templates + macros)
motion commit(src: element, station: element, track: path) {
    snapshot(src, station)
    after 0.6 { draw track [to: station.dot, duration: fast] }
}
keyframe "second" { commit(folder, st2, track) }
```
Parameters are typed (`element | group | path | anchor | number | text`) and
checked; macros expand before compilation, so they seek, lint and render like
anything else. There is no raw-SVG/JS escape hatch: extend through macros, or ask
for a core primitive.

### Tooling

```bash
agent-illustrator f.ail --timeline              # the choreography as a table: when, what, from -> to
agent-illustrator f.ail --frame 2 --at 50%      # a still mid-motion (also 0.35s, 350ms)
agent-illustrator f.ail --frames-strip 2 > s.svg  # contact sheet: 0/25/50/75/100%
agent-illustrator f.ail --lint                  # includes motion lints; exits 1 on errors only
agent-illustrator f.ail --lint-strict           # also exits 1 on warnings
agent-illustrator f.ail --timeline-json         # the manifest (tracks) a host can inspect
agent-illustrator --player-js > player.js       # the player, for hosts inlining the SVG
agent-illustrator f.ail --serve                  # live preview: arrow keys step, scrubber, reload on save
agent-illustrator f.ail --film 3 > film.html     # step 3 sampled every 50ms by the real player
agent-illustrator f.ail --crop-to-content 24 --animate   # picture = what the frames show (for embedding)
agent-illustrator f.ail --stylesheet-css theme.css --stylesheet-css deck.css   # layered, later wins
```
`--serve` and `--film` run the browser player itself: use them for what a host
will really show (flicker, a jump between steps); `--at` and `--frames-strip`
are the native renderer, which the golden tests check against.
Review `--timeline` before rendering anything: it answers "does this land in
the right beat" without images. Motion lints: motion on something hidden the whole
frame, a flight to a hidden target, a connection whose end is hidden, slow beat,
busy beat, and numeric coordinates in keyframes (`motion-coordinate`, an error).

### The player (hosts)

Every animated SVG embeds its manifest; `--animate` also embeds the player and
autoplays. To drive it yourself:
```js
const p = ail.player(svgElement);   // shows frame 0, settled
p.nextStep(); p.prevStep();         // one click: plays a frame and any [auto] frames after it
p.goToStep(k);                      // instant: the end of step k (fragments shown = k)
p.steps; p.step;                    // each step's first frame; the current step
p.goTo(i|name); p.next(); p.prev(); // frame by frame, if you really need it
p.on('step', k => ...); p.on('frame', (i, name) => ...); p.at(i, seconds)
```
A *step* is what one click shows. Hosts that advance on clicks (a slide deck
with fragments) use steps and never need to know which frames are `[auto]`;
`agent-illustrator --list-steps file.ail` prints them, one line per step.
Hosts drive the player; they never mutate the SVG. Without any player (no JS, an
`<img>`, print) the SVG shows frame 0 as it ends. Printing with a player shows
the last frame; `prefers-reduced-motion` jumps instead of playing. Hosts that
flip a `frame-<name>` class on the SVG root still get every frame's settled state.

### The stage

`rect stage [width: 1600, height: 900, canvas: true]` makes the stage the
picture's exact bounds. Anything leaving it in any frame is reported
(`canvas-overflow`, a lint error, and a warning on every render). Let labels wrap
instead of growing: `max_width: 240`. A row of components can pack by their
parts (`row r [gap: 40, align: dot, pack: tight]`) so labels alternating above
and below interleave.

### Themes: roles, not colours

A scene that uses only roles (`role-primary`, `role-ink`, `role-surface`,
`role-ok-soft`, `role-series-1` ...; see --grammar COLORS) renders unedited in
any theme: the theme's stylesheet maps each role once (`--role-primary:
var(--secondary-1);`). `ail:motion/git` uses roles only. Pick palette slots
(`accent-1`) only in a one-off diagram.

### Code on a slide

```
code merged [lang: python, title: "cart.py · merged", marks: "8:role-series-1-soft",
             source: "def total(cart):\n    ...\n    return 0 if total > 40 else 4.95"]
keyframe "conflict" {
    remove merged.line8
    at 0.1 { insert merged after line 7 [name: conflict, tint: role-warn-soft, stagger: 0.05,
             source: "<<<<<<< anna\n    return 0 if total > 40 else 4.95\n=======\n    ...\n>>>>>>> ben"] }
}
keyframe "resolve" {
    remove .conflict [stagger: 0.04, order: end]
    at 0.15 { show merged.line8 [enter: expand] }
}
```
Lines are parts (`merged.line4`, `merged.lines[3..5]`): highlight one with
`transform merged.line4 [fill: role-warn-soft]`, change one with `transform
merged.line1 [source: "def total(cart, coupon=None):", swap: roll]`. The block
grows and shrinks with its lines, and what is constrained below it follows. A diff:
`code d [diff: "-    return s\n+    return round(s, 2)", frame: false]`. Full scene:
examples/motion/git-merge.ail.

### Artwork with parts

An SVG file whose elements have ids is a component: `template "docx" from
"docx.svg"` gives every instance the parts `d.sheet`, `d.fold`, `d.bar3`, to
anchor to (`constrain q.center_x = d.sheet.left`), animate (`pulse d.fold`,
`transform d.bar3 [stroke: role-error]`) and flip (`swap a -> b [via: flip]`).
Draw colours as CSS variables with a fallback: `var(--role-ink, #111)` follows
the theme, `var(--b1, #ccc)` is an instance argument (`docx d [b1:
role-primary]`). Example artwork: examples/motion/assets/*.svg, used by
git-copies.ail.

### A deck host's header and notes

`keyframe "broke" [title: "Broke something? Go back", note: "..."]`: the title
holds from that step on, notes belong to their step. The player hands both to the
host with every step (`p.on('step', (k, m) => header.textContent = m.title)`),
forward and backward.

### Cookbook

**Snapshot to a timeline** (git commit):
```
flash folder
then { fly ghost(folder) to st1.dot [arc: 0.12] }
then { show st1.dot [enter: pop]; after 0.15 { show st1.nm [enter: rise] } }
```
(= `snapshot(folder, st1)` from `ail:motion/git`; `commit(folder, st1, track)`
also grows the line.)

**Clone to N machines**: `show anna.hist, ben.hist, chris.hist [from: hub.hist, stagger: 0.14]`
plus `show l_anna, l_ben, l_chris [enter: draw, stagger: 0.1]` for the links.

**Push / pull**:
```
show ben.hist.d4 [enter: pop]; draw ben.hist.track [to: ben.hist.d4]
then { fly ghost(ben.hist.d4) to hub.hist.d4 }
then { show hub.hist.d4 [enter: pop] }
then { fly ghost(hub.hist.d4) to anna.hist.d4, chris.hist.d4 }
then { show anna.hist.d4, chris.hist.d4 [enter: pop] }
```

**Branch and merge**: a metro path through the branch stations, `drawn: 0`,
`spread: even`; pin the first branch station (45 degrees off main) and give the
others only its height;
per step `draw branch [to: l1.dot]` then the station pops; to merge,
`show gate [enter: draw]`, `draw branch`, `draw main_line`, then pop and
`pulse` the interchange.

**Merge request checklist**: rows with a hidden tick
(`appears: later`); `transform tests [opacity: 1]` + `show tests.tick [enter: pop]`
per check; a pending review is a highlight wash
(`transform review.bg [opacity: 1]`), approval is
`transform review.txt [label: "Approved by Ben", swap: fade]` + its tick.

**Conflict chooser**: both versions side by side, the chosen one
`highlight`ed then `move`d into place, the other `hide [exit: shrink]`:
```
highlight theirs [color: status-warning]
then { move ours to merged_slot; hide theirs [exit: shrink] }
```

**Go back** (undo to an earlier state): `move ring to st2.dot`,
`transform st3 [opacity: 0.28]`, `flash folder`, and roll the values back with
`transform file.val [label: "57 lines", swap: roll]`.

The full acceptance scenes are in `examples/motion/` (git-copies, git-snapshots,
git-branches, sharing `git-deck.ail`).

---

## Part 3: Gotchas

1. **Orphaned connections** — If you hide element A but keep `A -> B` visible,
   the arrow renders to nowhere. Always hide connections when hiding their endpoints
   (`--lint` reports it).
2. **Cumulative keyframes** — Each frame builds on the previous. If you `hide X` in
   frame 2, X stays hidden in frames 3+ unless you `show X` again.
3. **Transform persistence** — Transforms in frame N carry forward (visual AND
   geometry), merged per-property. To reset a property in a later frame, restate it
   explicitly (e.g. `transform elem [opacity: 1.0]`, or `move elem home`).
3b. **Labels and resize** — Position animation moves an element's label with it (both
   live in the same wrapper group). But a *labeled* element that itself resizes
   (width/height) does not re-center its own label. Resize unlabeled frames/boxes;
   put labels on the chips that move.
4. **Frame naming** — Use descriptive names ("user_sends_prompt", not "frame3").
   Names appear in the animation player UI.
5. **Element count** — Animations tend to need many elements (persistent + transient
   for each frame). Use constraint-based layout, not row/col.
6. **Frame indexing** — `--frame N` is 0-indexed. Frame 0 is the first keyframe,
   frame 1 is the second, etc. When frame 0 plays, its `hide`s are already applied
   and its `show`s enter.
7. **ViewBox consistency** — Declare the stage: `rect stage [width: 1600, height: 900, canvas: true]`.
   The viewBox is then exactly the stage in every frame, and anything that leaves it
   is reported.
8. **Delegate image verification to subagents** — Viewing rendered PNGs in the main
   conversation bloats context rapidly. Spawn a subagent to render frames, inspect
   the images, and return a text-only PASS/FAIL report.
