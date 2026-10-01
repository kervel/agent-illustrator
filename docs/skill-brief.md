# Agent Illustrator — the short version

AIL describes a picture by meaning: shapes, components and how they relate.
The engine does the layout, animation, checking and rendering. You never write
coordinates. This page is enough for most tasks. Fetch a detail with
`agent-illustrator --doc <topic>`; the topics are listed at the end. The full
guides are `--skill` (diagrams) and `--skill-animation` (motion).

## The loop (do every step, every time)

1. Write a `STORYBOARD` comment first: one line per click step.
2. Write the scene. Declare everything once, then keyframes.
3. `agent-illustrator --lint scene.ail`. Fix every finding, or accept an
   intended overlap explicitly with `overlaps:`.
4. `agent-illustrator --states scene.ail`. Compare it with your storyboard.
5. Look at it: `agent-illustrator --frames-to-dir out --png scene.ail`, and
   a mid-motion still with `--frame clone --at 0.6s --png -o mid.png` (at can
   also be `50%`). Look at the PNGs before you say it is done. `--frame` takes
   a keyframe name (best) or index; `--states` numbers click steps, and an
   `[auto]` keyframe joins the step before it.

## A whole scene

```ail
/* STORYBOARD
     start   a server and two laptops
     clone   the links draw; each laptop gets the history as its link arrives
*/
rect stage [width: 1000, height: 560, fill: role-backdrop, stroke: none, canvas: true]
constrain stage.left = 0
constrain stage.top = 0

template "machine" (title: "Laptop") {
    rect bg [width: 240, height: 120, fill: role-surface, stroke: role-ink, stroke_width: 3, corner_radius: 14]
    rect head [width: 240, height: 40, fill: role-surface-2, stroke: none, clip: bg,
               label: title, align: start, font_size: 20, font_weight: 700]
    row hist [gap: 22, align: center] {
        circle c1 [size: 20, fill: role-surface, stroke: role-primary, stroke_width: 5]
        circle c2 [size: 20, fill: role-surface, stroke: role-primary, stroke_width: 5]
    }
    constrain head.top = bg.top
    constrain head.left = bg.left
    constrain hist.center_x = bg.center_x
    constrain hist.center_y = bg.center_y + 18
}

machine server [title: "Server"]
row laptops [gap: 100] {
    machine anna [title: "Anna", hist.appears: later]
    machine ben [title: "Ben", hist.appears: later]
}
constrain server.center_x = stage.center_x
constrain server.top = stage.top + 110
constrain laptops.center_x = stage.center_x
constrain laptops.top = server.bottom + 100

server.bottom -> anna.top as l_anna [stroke: role-rule, stroke_width: 4, appears: later]
server.bottom -> ben.top as l_ben [stroke: role-rule, stroke_width: 4, appears: later]

keyframe "start" [title: "Everyone has the full history"] {
    show server [enter: pop]
    when server shown { show laptops.* [enter: rise, stagger: 0.1] }
}
keyframe "clone" {
    show l_anna, l_ben [enter: draw, stagger: 0.1]
    when l_anna shown { show anna.hist [from: server.hist] }
    when l_ben shown { show ben.hist [from: server.hist] }
}
```

## Placing things

- `row`, `col`, `grid` for regular arrangements, with `gap:`, `align:` and
  `padding:`. Use constraints for relations:
  `constrain card.left = file.right + 70`,
  `constrain cap.center_y = midpoint(a.bottom, b.top)`.
- `template "x" (param: default) { ... }` makes a component. Parts are `inst.part`.
  Set one part per instance with `machine anna [hist.appears: later]`.
- `point hub` is an invisible anchor. `clip: bg` draws a part inside its frame.
- Stage: give every scene one, `rect stage [..., canvas: true]`. It is the
  picture's frame; anything drawn outside it is a lint error.
- Later declarations draw on top. Declare backgrounds (bands, lanes, a big
  shape) before what sits on them. An intended overlap (a badge on a corner,
  a chip on a box) needs `overlaps: other`.
- Captions: `text "Hotfix" nm [caption_of: dot, label_position: below]`.
  Don't constrain other things against a caption.
- Many alike from one line: `box b* [items: [{tint: role-ok}, {tint: role-warn}]]`
  gives `b0`, `b1`, … (in a `row`/`grid` they line up).
- A few words are reserved (`line`, `row`, `label`, `left`, `top`, …); the
  parse error says so.
- Colours are roles: `role-primary`, `role-ink`, `role-surface`,
  `role-ok`/`-error`/`-warn` (with `-soft` variants), `role-rule`.
- Objects (a laptop, a person, a box, a database) are SVG artwork with ids:
  `template "box" from "box.svg"`, with parts `b.lid`. Don't build them from rects.
- Built-ins: `code c [lang: python, source: "..."]` (lines `c.line[4]`) and
  `table t [columns: [...], rows: [[...]]]` (rows `t.row[2]`).

## Motion

- Each `keyframe` is one click. `[title: "..."]` goes to the deck header.
  Keep a keyframe under about 2.5s. Split a long one with a follow-on
  `keyframe "more" [auto] { ... }` that plays by itself.
- Say at the element that it is not there at the start (`appears: later`).
  The verb that brings it on says when and how:
  `show x [enter: pop | rise | fade | draw]`, `show x [from: y]`. Put
  `appears: later` on the thing you `show`, not on its container.
- Time by what things wait for:
  - `when line reaches st.dot { ... }`
  - `when x shown { ... }`, `when x arrives + 0.3 { ... }`,
    `when x accented { ... }` (they fire when that finishes; `+`/`-` shifts
    them; x already on screen counts as shown at the start)
  - `then { ... }` waits for EVERYTHING before it, a 1.2s accent or a slow
    move included (to go on sooner use `when x shown`)
  - `at 0.4 { ... }` (from the start of the keyframe)

  Avoid chains of `after 0.3`.
- Verbs:
  - `hide`, `move x to y`, `fly ghost(x) to y`, `draw line [to: st.dot]`
  - `move x to y` centres x on y. To stand beside y, move to a `point`
    placed beside it.
  - `transform x [fill: role-ok, label: "...", swap: fade]`
  - `swap a -> b [via: flip]`
  - lists work: `move a, b, needs.* to box [stagger: 0.07, order: random]`
- "Look here": `accent x`. Use `[tone: error]` for a problem and
  `[hold: step]` to keep it until the next click.
- States live in the template:
  `state done { transform txt [label: "ok"]; show tick }`. Enter one with
  `set inst done`; go back with `set inst default`.
- Layouts that change: `layout beside { constrain ... }`, then
  `use layout beside` and later `use layout default`.
- Hinges: `rect lid [pivot: left]` + `transform lid [rotation: -30]`. For
  artwork: `barrier gate [arm.pivot: left]` + `transform gate.arm [rotation: -80]`.

## Output

- `--animate`: a click-to-step player.
- `--animate-css`: a self-playing loop that works in an `<img>` or a README.
- `--png -o f.png`, `--frame N`, `--at T`, `--scale 2`: pictures, no browser needed.
- `--serve`: a live preview.

## Fetch on demand: `agent-illustrator --doc <topic>`

Topics: {TOPICS}. Other words work too (move, pivot, stagger, timing,
highlight, z-order, overlap, caption, symbols); a near miss gets a suggestion.
