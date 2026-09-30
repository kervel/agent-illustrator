# Animation Skill

Create keyframe animations in Agent Illustrator. Read `--skill` first for general AIL usage.

## When to Use

Use this sub-skill when creating **multi-frame animated diagrams** — sequences where
elements appear, disappear, or transform over time. Not needed for static diagrams.

## Your first scene in 10 minutes

A whole scene: a stage, one component, keyframes timed by events. Copy it,
change it, and check it with the four commands below. The rest of this skill
is the reference.

```ail
/* STORYBOARD
     start   a server and three laptops
     clone   the links draw; each laptop gets the history when its link arrives
     push    Ben commits; the commit travels up to the server
     pull    and down to Anna and Chris: everyone has the same history
*/

// The stage: the picture's exact bounds. Leaving it is a lint error.
rect stage [width: 1200, height: 675, fill: role-backdrop, stroke: none, canvas: true]
constrain stage.left = 0
constrain stage.top = 0

// A component: a window with a header bar, and a history of commits.
template "machine" (title: "Laptop") {
    rect bg [width: 260, height: 130, fill: role-surface, stroke: role-ink, stroke_width: 3, corner_radius: 14]
    rect head [width: 260, height: 42, fill: role-surface-2, stroke: none, clip: bg,
               label: title, align: start, font_size: 20, font_weight: 700]
    row hist [gap: 24, align: center] {
        circle c1 [size: 20, fill: role-surface, stroke: role-primary, stroke_width: 5]
        circle c2 [size: 20, fill: role-surface, stroke: role-primary, stroke_width: 5]
        circle c3 [size: 20, fill: role-surface, stroke: role-primary, stroke_width: 5, appears: later]
    }
    constrain head.top = bg.top
    constrain head.left = bg.left
    constrain hist.center_x = bg.center_x
    constrain hist.center_y = bg.center_y + 20
}

machine server [title: "Server"]
row laptops [gap: 80] {
    machine anna [title: "Anna", hist.appears: later]
    machine ben [title: "Ben", hist.appears: later]
    machine chris [title: "Chris", hist.appears: later]
}
constrain server.center_x = stage.center_x
constrain server.top = stage.top + 130
constrain laptops.center_x = stage.center_x
constrain laptops.top = server.bottom + 150

server.bottom -> anna.top as l_anna [stroke: role-rule, stroke_width: 4, appears: later]
server.bottom -> ben.top as l_ben [stroke: role-rule, stroke_width: 4, appears: later]
server.bottom -> chris.top as l_chris [stroke: role-rule, stroke_width: 4, appears: later]

keyframe "start" [title: "Everyone has the full history"] {
    show server [enter: pop]
    when server shown { show laptops.* [enter: rise, stagger: 0.1] }
}

keyframe "clone" {
    show l_anna, l_ben, l_chris [enter: draw, stagger: 0.1]
    when l_anna shown { show anna.hist [from: server.hist] }
    when l_ben shown { show ben.hist [from: server.hist] }
    when l_chris shown { show chris.hist [from: server.hist] }
}

keyframe "push" {
    show ben.c3 [enter: pop]
    when ben.c3 shown { show server.c3 [from: ben.c3, duration: slow] }
}

keyframe "pull" {
    show anna.c3, chris.c3 [from: server.c3, duration: slow, stagger: 0.1]
}
```

```bash
agent-illustrator --lint first.ail      # what to fix: overlaps, text that doesn't fit, timing
agent-illustrator --states first.ail    # per click step: what shows, hides, changes (and how long)
agent-illustrator --serve first.ail     # live preview in a browser: arrow keys step, reloads on save
agent-illustrator --animate first.ail > first.svg   # the result, with a click-to-step player
```

What to notice:
- **Names, not coordinates.** `constrain laptops.top = server.bottom + 150`;
  the row spaces the laptops; `ben.c3` is the part `c3` of instance `ben`.
- **Say it once at the element.** `appears: later` means "not there at the
  start"; the verb that brings it on says when and how (`show … [from: …]`
  flies it in from another element). `hist.appears: later` does that for one
  part of one instance.
- **Time by what things wait for.** `when l_anna shown { … }` starts when that
  link has drawn in; `when ben.c3 shown` when the commit has popped up.
- **A header bar is clipped to its frame** (`clip: bg`), so it follows the
  rounded corners and never covers the border.
- **Each keyframe is one click.** `[title: …]` goes to a deck host's header.

A fuller version of this scene, with a caption per step:
`examples/motion/git-history.ail`.

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
`<img>`). For a GitHub README use `--animate-css`: the same choreography as
a self-playing pure-CSS loop (each keyframe plays, holds 1.4s, and the story
restarts), sampled from the native renderer, so it plays inside an `<img>`.
Only `count` (rewritten digits) is not carried.

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

### Beats: say what a thing waits for

Statements in a block start together. Time the rest **by events**, so a block
starts when what it depends on happens, whatever that is retimed to:
```
keyframe "main" {
    draw main_line [to: c.dot, duration: slow]
    when main_line reaches a.dot { station(a) }   // the line passes the station
    when main_line reaches b.dot { station(b) }
    when l1.dot shown + 0.1 { ... }               // an entrance ended (+/- a nudge)
    when ring arrives { flash folder }            // a move or flight ended
    when card hidden { use layout default }       // an exit ended
    beat open { show mr [enter: pop] }            // a named group ...
    after open + 0.2 { set tests passed }         // ... and when it ends
    then { pulse merged.dot }                     // when everything before it has ended
    at 1.2 { show caption }                       // 1.2s after the keyframe started
}
keyframe "next_step" [auto, after: 0.3] { ... }   // plays by itself 0.3s after the previous one
```
Every event is a moment something **finishes**: `reaches` when the line's
drawing passes the element's centre, `shown` when the latest `show` of it in this
keyframe has finished entering, `hidden` when its latest exit has finished,
`arrives` when its latest `move`/`fly` ends, or its `show [from: …]` flight
lands, or a transform that moves, turns or resizes it ends (if nothing does,
when the latest layout change such as `use layout beside` has played), `after name` when
everything in `beat name` has ended. `+ 0.1` / `- 0.15` shift from that moment.
Only statements earlier in the same keyframe count. `--timeline` prints each
event with its resolved time (`when merged shown - 0.15 (= 1.20s, starts
1.05s)`) above the statements it starts.

Use `when`/`beat`/`then` for coupling and `at` for "roughly then". `after 0.3 {
... }` (a number) still works but counts from the previous beat's *start*, so a
row of them is a sum nobody wrote down; `--lint` reports two or more in a row. A
station that pops before its line reaches it is reported too (`when <line>
reaches <station>` fixes it). A keyframe's length is derived, never declared.

### Entrances and exits

```
show x [enter: fade | pop | rise | drop | grow | wipe(left|right|up|down) | draw]
show copy [from: original]        // appears at original's place and size, travels home
hide x [exit: fade | shrink | fall | lift | wipe(...)]
motion [enter: pop, exit: fade]   // diagram-wide defaults (default: rise / fade)
```
An outline around other elements (a dashed frame) enters with `[enter: draw]`
or fades: popping or growing it scales its border through its contents, which
reads as stray lines (`--lint` says so). Showing the parts of a container that
is itself still hidden does nothing (`--lint` says that too).

**One idiom:** declare at the element that it is not there from the start
(`appears: later`), and let the verb that brings it on say when and how (`show
ring [enter: fade]`, a `swap`, a macro, a `set`). `appears: go_back` (hidden
until that keyframe, entering with the default preset) is for elements nothing
else shows; writing both `appears: go_back` and a `show` in go_back is reported.
Frame 0's own `hide`s apply before it plays; its `show`s are entrances.
A part of a component that enters later for *some* instances only is said at
the instance: `machine anna [title: "Anna", hist.appears: later]` (any
`part.modifier: value` overrides that part of that instance; `hist.d4.fill:`
reaches into a nested component). `appears:` on a nested instance in a
template body applies to every instance.

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
fly ghost(folder) to st1.dot [scale: 0.25, arc: 0.15]   // a copy flies and vanishes; by default it shrinks to fit the target
fly ghost(hub.d4) to anna.d4, chris.d4 [stagger: 0.1]   // one copy per destination
fly token from a to b                                    // a real (hidden) element travels, at its own size
move ring to st2.dot            // persistent: dependents re-solve around it
move d0 home                    // release a `move`
move train along track [to: st3.dot]
// `move x to y` centres x ON y. To stand next to it, move to a point beside it:
//   point at_row3
//   constrain at_row3.left = t.row3.right + 30
//   constrain at_row3.center_y = t.row3.center_y
//   move robot to at_row3
```

### Look here: `accent`

The one verb for "this is what we are talking about":
```
accent orders.row[2]                     // a row or a label: underlined
accent api.dot                           // something small: ringed
accent panel                             // a panel: outlined
accent step3 [tone: error]               // why: attention (default) | error (adds a "!") | ok
accent code.line[4] [hold: step]         // stays until the next click (while you talk)
accent icon [style: wiggle]              // or underline | ring | outline
```
The style follows the element: text and rows (things with no fill or border
of their own) are underlined, something small (up to 80px) is ringed, any
other box is outlined; the tone picks a role colour. An accent lasts 1.2s:
a `then` after it waits for it (use `when x shown` to go on sooner). It is drawn over
the element, moves nothing and leaves nothing behind; stills (`--at`, PNG)
show it while it plays. Prefer it to building emphasis from pulse +
highlight + a colour change.

### One-shot effects (leave nothing behind)

`pulse x [scale: 1.3]`, `shake x`, `nudge x [direction: up]`, `flash x`,
`ping x`, `highlight x [color: accent-1]`; ambient: `loop x [pulse, period: 1.2]`
(runs while the frame is on screen). `jitter: rotate(4)` on a `show` gives each
target a small seeded tilt of up to 4 degrees (`jitter: move(6)` shifts by up
to 6px instead; a bare `jitter: 4` means rotate) — a pile, not a table.

### Text and numbers

```
transform cap [label: "Approved by Ben", swap: roll | fade | cut]
count total [to: 14900, format: "€ {:,}"]     // {} {:,} {:.1} {:,.2}
swap docs.* -> codes.* [via: flip, flip: pg, stagger: 0.07]   // flip only member `pg`, crossfade the rest
swap ok -> bad [via: fade | flip | morph]
```
A swap turns one element into another *where it is*: place the partner on it
(`constrain bad.center = ok.center`; for lists `constrain k*.pg.center =
d*.pg.center` pairs them by index), or `--lint` reports it. With `flip: pg`
the flipped part is what must line up. One element that
changes wording and colour is simpler as a transform: `transform status [label:
"● broken", stroke: role-error, label_fill: role-error, swap: fade]` and back
with `[label: initial, stroke: initial, label_fill: initial]` (or give it
states, below).

Marks in labels: the bundled fonts (used by `--png` and for any viewer
without the right font) have these, so they look the same everywhere:
✓ ✔ ✕ ✗ × → ← ↑ ↓ ↔ ↕ ↺ ➜ ➤ · • … – — ★ ☆ ● ○ ■ □ ▲ ▶ ◀ ▼ ◆ ◇ ♥ ⚠ ⚡ ☁ ☐ ☑ ☒ ✉ ✎ ✚ € £ ° ± ≈ ≠ ≤ ≥ ∞ √ π « ».
Others (↻ ⇒ ↩ ⚙ ⏳, emoji) may show as boxes; `--png` names them.

### Layouts that change

A few elements placed differently for a while (a file moving aside for a card)
is a named layout, not a pair of disable/enable statements:
```
constrain merged.bg.center_x = s.stage.center_x      // the layout as declared: `default`
layout beside {
    constrain merged.bg.left = s.stage.left + 272
}
keyframe "conflict" { use layout beside; ... }
keyframe "choose"   { ...; at 0.75 { use layout default } }
```
`use layout X` takes timing options like any verb; what depends on the moved
elements follows. A layout's constraints replace the declared ones on the same
elements and axis (x, y, width, height).

### Components with states

A component that reports (a check, a status, a badge) declares its looks once,
in its template; keyframes only say which one:
```
template "check" (says: "check", then_says: "") {
    rect bg [fill: role-warn, fill_opacity: 0.28, stroke: none, opacity: 0]
    row item { circle tick [..., appears: later]  rect txt [label: says, ...] }
    constrain bg contains item [padding: 4]
    state passed   { transform self [opacity: 1]; show tick [enter: pop] }
    state asked    { transform self [opacity: 1]; transform bg [opacity: 1] }
    state approved { transform self [opacity: 1]; transform txt [label: then_says, swap: fade]
                     after 0.1 { show tick [enter: pop] } }
}
keyframe "review" { set tests passed; when tests.tick shown { set review asked } }
keyframe "fixed"  { set review approved [duration: fast] }
```
A state is motion statements naming the component's parts (`self` is the
instance), with the template's parameters. `set x s [opts]` gives every
statement the options, and whatever another state of the component changes that
`s` does not goes back to how the template draws it (`asked` → `approved` clears
the wash); `set x default` undoes every state. A single element gets states at
top level:
```
rect status [label: "● checkout works", stroke: role-ok, label_fill: role-ok, ...]
state status broken { transform self [label: "● checkout broken", stroke: role-error,
                                      label_fill: role-error, swap: fade] }
keyframe "broke"   { set status broken [duration: fast] }
keyframe "go_back" { set status default [swap: fade] }
``` Name parameters so they do not shadow words used as values (a
parameter called `later` would rewrite `appears: later`). Outside templates,
`transform x [fill: initial]` returns a property to the declared value.

### Hinges

`rect lid [..., pivot: left]` turns about its left edge: `transform lid
[rotation: -28]` opens it like a lid (also `right`, `top`, `bottom`,
`top_left`, ...). The pivot is where pops and pulses scale from too. On a part
of SVG artwork, set it at the instance: `barrier gate [arm.pivot: left]`, then
`transform gate.arm [rotation: -80]` swings the arm up from its post.

### Layering and overlaps on purpose

What is declared later is drawn on top. Declare backgrounds (bands, lanes, a
big shape things sit on) *before* what sits on them, or they cover it.
`z_order: 1` lifts one element above its siblings; a line `through:` stations
runs under them by itself.

Things that sit on something on purpose (a badge on a document's corner, a
chip on an environment box, a tag on a frame's border) say so, so the lint
stops reporting them: `circle badge [..., overlaps: doc]`, or on an instance
`machine m [overlaps: env.bg]`.

### Captions

A caption follows its subject: `text "Hotfix" nm [caption_of: dot,
label_position: below]` (or `above`, `left`, `right`). Place the subject, not
the caption; do not constrain other things against a `caption_of` element
(the lint says so).

### Many things at once

Many similar instances from one line: `boxart b* [items: [{tint: role-ok},
{tint: role-error}]]` makes `b0`, `b1`, … (inside a `row` or `grid` they line
up); `constrain k*.center = d*.center` pairs two such lists by index.

A swarm is one statement: `move needs.* to box [stagger: 0.07]`,
`move a, b, c to box`, `fly ghost(a, b, c) to box` (or `fly ghost(a),
ghost(b) to box`), `transform m1, m2, m3 [fill: role-error]`. The busy-beat
lint counts one statement as one gesture, however many targets it staggers.

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
    draw track [to: station.dot, duration: fast, delay: 0.6]
}
keyframe "second" { commit(folder, st2, track) }
```
Parameters are typed (`element | group | path | anchor | number | text`) and
checked; macros expand before compilation, so they seek, lint and render like
anything else. Inside a macro, time with `then`, events and `delay:` (all
relative to where it is called); `at` counts from the keyframe start. There is
no raw-SVG/JS escape hatch: extend through macros, or ask for a core primitive.

### Verifying without a browser

The binary alone is the whole loop; nothing else needs to be installed:

```bash
agent-illustrator --lint scene.ail                          # what to fix (exit 1 on errors)
agent-illustrator --states scene.ail                        # per click step: what shows, hides, changes
agent-illustrator --timeline scene.ail                      # when everything happens
agent-illustrator --frames-to-dir out --png scene.ail       # every step's end state as PNG
agent-illustrator --frame 2 --at 50% --png -o mid.png scene.ail   # a still, mid-motion
agent-illustrator --frames-strip 2 --png -o strip.png scene.ail   # 0/25/50/75/100% of step 2
agent-illustrator --frame 2 --png --scale 3 -o zoom.png scene.ail # zoom in on details
```

Look at the PNGs: they are rendered from the same tracks the browser player
plays. Fonts: Overpass, Overpass Mono and a symbol font are built in; a
stylesheet's `@font-face` data URIs (TTF/OTF/WOFF/WOFF2) are used; a font or
character that is not available is reported. A browser (`--serve`, `--film`)
is extra assurance, never required.

### Tooling

```bash
agent-illustrator f.ail --states                # the storyboard: per click step what is visible, what changes
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
Check `--states` against the STORYBOARD comment first (it answers "is the right
thing on screen at each step", including what `appears:`, macros and `set` do),
then `--timeline` for "does this land in the right beat", both without images. Motion lints: motion on something hidden the whole
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

### Windows and panels

A box with a header bar (a machine, a folder, a file window): draw the bar as
a plain rect and clip it to the frame, so it follows the frame's rounded
corners and never covers its border:
```
template "machine" (title: "Laptop") {
    rect bg [width: 300, height: 150, fill: role-surface, stroke: role-ink, stroke_width: 3, corner_radius: 16]
    rect head [width: 300, height: 48, fill: role-surface-2, stroke: none, clip: bg,
               label: title, align: start]
    constrain head.top = bg.top
    constrain head.left = bg.left
}
```
In a `col` of rows under a header, `col body [gap: 0, padding: 0]` puts them
flush with the frame. Code blocks' title bars do this already.

### Tables on a slide

A small datagrid is one element, not rows of rects fighting the defaults:
```
table orders [columns: ["#", "customer", "status"],
              rows: [["41", "Stroopwafels BV", "shipped"], ["42", "Acme Bikes", "open"]],
              widths: [60, 210, 114], font_size: 18, mono: [0]]
keyframe "pick" { highlight orders.row[2] [color: role-primary] }
keyframe "ship" { transform orders.r2c2 [label: "shipped", swap: fade] }
```
Cells are square and flush and keep their spacing; the header band and rules
use theme roles. Rows are parts (`orders.row2`, `orders.rows[1..3]`), so they
can appear later, be highlighted, or be recoloured (`transform orders.row2
[fill: role-ok-soft]`).

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
Never paint the background colour to fake a hole (a backdrop-coloured
triangle over a page's corner): on another background it shows. Cut the
shape instead (the sheet in docx.svg is a path with its corner cut off).

**Objects are artwork, not rects.** Agent Illustrator is not an icon-drawing
tool. A recognisable object (a box, a laptop, a server, a person, a cloud, a
document, a database) is best an SVG file with ids (`template "x" from
"x.svg"`), themed with role colours. Its parts can then be animated. The
language's defaults for scene primitives (corner radius, stroke and label
defaults) are right for diagram elements, but they fight you when every shape
of one object is deliberate. A hand-built page, a header bar made of two
rects, or a box made of a rect and a lid all looked worse than the artwork
that replaced them. Building an icon from shapes isn't forbidden, it's just
not what the tool is for.

### A deck host's header and notes

`keyframe "broke" [title: "Broke something? Go back", note: "..."]`: the title
holds from that step on, notes belong to their step. The player hands both to the
host with every step (`p.on('step', (k, m) => header.textContent = m.title)`),
forward and backward. When the picture has its own heading, write the title once:
`motion [title: s.heading, title_swap: roll]` makes the heading show each
keyframe's title (it swaps where the title changes).

### Cookbook

**Snapshot to a timeline** (git commit):
```
flash folder
then { fly ghost(folder) to st1.dot [arc: 0.12] }
then { show st1.dot [enter: pop]; show st1.nm [enter: rise, delay: 0.15] }
```
(= `snapshot(folder, st1)` from `ail:motion/git`; `commit(folder, st1, track)`
also grows the line.)

**Clone to N machines**: `show anna.hist, ben.hist, chris.hist [from: hub.hist, stagger: 0.14]`
plus `show l_anna, l_ben, l_chris [enter: draw, stagger: 0.1]` for the links.

**Push / pull**: the commit itself travels and stays (a ghost that lands and
then a dot that pops reads as "arrive, vanish, reappear"); the line grows to
meet it as it lands:
```
keyframe "push" {
    show hub.hist.d4 [from: ben.hist.d4, duration: slow]
    draw hub.hist.track [to: hub.hist.d4, duration: slow]
}
keyframe "pull" {
    show anna.hist.d4, chris.hist.d4 [from: hub.hist.d4, duration: slow, stagger: 0.1]
    draw anna.hist.track [to: anna.hist.d4, duration: slow]
    at 0.1 { draw chris.hist.track [to: chris.hist.d4, duration: slow] }
}
```
(Full scene: examples/motion/git-history.ail. The station lint counts a
`[from:]` entrance as arriving when it lands.) A `fly ghost(...)` is for a
copy that leaves the original where it was and is not itself kept (a
snapshot).

**Branch and merge**: a metro path through the branch stations, `drawn: 0`,
`spread: even`; pin the first branch station (45 degrees off main) and give the
others only its height;
per step `draw branch [to: l1.dot]` then the station pops; to merge,
`show gate [enter: draw]`, `draw branch`, `draw main_line`, then pop and
`pulse` the interchange.

**Merge request checklist**: a `check` component with states (see Components
with states): `set tests passed`, `set review asked`, `set review approved`.

**Conflict chooser**: both versions side by side, the chosen one
`highlight`ed then `move`d into place, the other `hide [exit: shrink]`:
```
highlight theirs [color: status-warning]
then { move ours to merged_slot; hide theirs [exit: shrink] }
```

**Go back** (undo to an earlier state): `move ring to st2.dot`,
`transform st3 [opacity: 0.28]`, `flash folder`, and roll the values back with
`transform file.val [label: "57 lines", swap: roll]` (or `[label: initial]`).

The full acceptance scenes are in `examples/motion/` (git-copies, git-snapshots,
git-branches, git-merge, sharing `git-deck.ail`).

---

## Part 3: Gotchas

1. **Orphaned connections** — If you hide element A but keep `A -> B` visible,
   the arrow renders to nowhere. Always hide connections when hiding their endpoints
   (`--lint` reports it).
2. **Cumulative keyframes** — Each frame builds on the previous. If you `hide X` in
   frame 2, X stays hidden in frames 3+ unless you `show X` again.
3. **Transform persistence** — Transforms in frame N carry forward (visual AND
   geometry), merged per-property. To reset a property in a later frame, say so:
   `transform elem [opacity: initial]` (the declared value), or `move elem home`.
3b. **Labels and resize** — Position animation moves an element's label with it (both
   live in the same wrapper group). But a *labeled* element that itself resizes
   (width/height) does not re-center its own label. Resize unlabeled frames/boxes;
   put labels on the chips that move.
4. **Frame naming** — Use descriptive names ("user_sends_prompt", not "frame3").
   Names appear in the animation player UI.
5. **Layout** — Use `row`/`col`/`grid` for regular arrangements (a set of
   laptops, dots on a history) and constraints for relations between things
   (this card beside that file, the caption in the gap:
   `constrain cap.center_y = midpoint(hub.bottom, laptops.top)`). Animations
   need many elements; declare them all once and let keyframes show them.
6. **Frame indexing** — `--frame N` is 0-indexed. Frame 0 is the first keyframe,
   frame 1 is the second, etc. When frame 0 plays, its `hide`s are already applied
   and its `show`s enter.
7. **ViewBox consistency** — Declare the stage: `rect stage [width: 1600, height: 900, canvas: true]`.
   The viewBox is then exactly the stage in every frame, and anything that leaves it
   is reported.
8. **Delegate image verification to subagents** — Viewing rendered PNGs in the main
   conversation bloats context rapidly. Spawn a subagent to render frames, inspect
   the images, and return a text-only PASS/FAIL report.
