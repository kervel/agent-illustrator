AGENT ILLUSTRATOR EXAMPLES
==========================

EXAMPLE 1: System Architecture
------------------------------
col main {
    text "System Architecture" title [font_size: 20]
    row components {
        col frontend {
            rect ui [label: "UI"]
            rect api [label: "API"]
        }
        col backend {
            rect service [label: "Service"]
            rect db [label: "Database", fill: orange]
        }
    }
}
api.right -> service.left
service.bottom -> db.top

Nested layouts: col contains row contains two cols.
Connections cross layout boundaries automatically.

EXAMPLE 2: Styleable feedback loops with via points
----------------------------------------------------
// Invisible via points control curve shape
circle human_via [size: 1, fill: none, stroke: none]
circle agent_via [size: 1, fill: none, stroke: none]

col diagram [gap: 40] {
  col human_section [gap: 8] {
    row [gap: 8] {
      text "HUMAN" [font_size: 14, fill: accent-dark]
      text "— slow · persistent" [font_size: 11, fill: text-3]
    }
    row human_loop [gap: 20] {
      rect assign [width: 120, height: 50, fill: accent-light, stroke: accent-dark, stroke_width: 2, label: "Assign Task"]
      rect tune [width: 120, height: 50, fill: accent-light, stroke: accent-dark, stroke_width: 2, label: "Tune Feedback"]
      rect spot [width: 120, height: 50, fill: accent-light, stroke: accent-dark, stroke_width: 2, label: "Spot Patterns"]
      rect evaluate [width: 120, height: 50, fill: accent-light, stroke: accent-dark, stroke_width: 2, label: "Evaluate"]
    }
  }
  col agent_section [gap: 8] {
    row [gap: 8] {
      text "AGENT" [font_size: 14, fill: secondary-dark]
      text "— fast · ephemeral" [font_size: 11, fill: text-3]
    }
    row agent_loop [gap: 20] {
      rect task [width: 120, height: 50, fill: secondary-light, stroke: secondary-dark, stroke_width: 2, label: "Task"]
      rect execute [width: 120, height: 50, fill: secondary-light, stroke: secondary-dark, stroke_width: 2, label: "Execute"]
      rect check [width: 120, height: 50, fill: secondary-light, stroke: secondary-dark, stroke_width: 2, label: "Feedback"]
      rect result [width: 120, height: 50, fill: secondary-light, stroke: secondary-dark, stroke_width: 2, label: "Result"]
    }
  }
}

// Position via points for curve control
constrain human_via.center_x = midpoint(assign, evaluate)
constrain human_via.center_y = assign.top - 50
constrain agent_via.center_x = midpoint(task, result)
constrain agent_via.center_y = result.bottom + 50

// Human flow (right to left)
evaluate.left -> spot.right -> tune.left -> assign.right [stroke: accent-dark, stroke_width: 3]
assign.top -> evaluate.top [stroke: accent-dark, stroke_width: 3, routing: curved, via: human_via]

// Agent flow (left to right)
task.right -> execute.left -> check.right -> result.left [stroke: secondary-dark, stroke_width: 3]
result.bottom -> task.bottom [stroke: secondary-dark, stroke_width: 3, routing: curved, via: agent_via]

// Cross connections
assign.bottom -> task.top [stroke: foreground-3, stroke_width: 1]
result.top -> evaluate.bottom [stroke: foreground-3, stroke_width: 1]
tune.bottom -> check.top [stroke: accent-1, stroke_width: 2, routing: curved, label: "tunes"]

Styleable with symbolic colors (accent-dark, secondary-light, etc).
Via points control curve bulge. Anchors (.top, .bottom) for precise connections.
Use --stylesheet to apply different color themes.

EXAMPLE 3: Templates with anchors and curved connections
--------------------------------------------------------
template "person" {
  col [gap: 6] {
    stack head_stack {
      circle head [size: 18, fill: #f2c9a0, stroke: #333]
      path hair [fill: #2b1b0e] {
        vertex a [x: 0, y: 6]
        arc_to b [x: 18, y: 6, radius: 9]
        line_to c [x: 13, y: 4]
        line_to d [x: 9, y: 6]
        line_to e [x: 6, y: 5]
        close
      }
    }
    path torso [fill: #4a6fa5, stroke: #333] {
      vertex a [x: 0, y: 6]
      arc_to b [x: 26, y: 6, radius: 13]
      line_to c [x: 26, y: 14]
      line_to d [x: 0, y: 14]
      close
    }
  }
  constrain head.center_x = torso.center_x
  constrain hair.center_x = head.center_x
  constrain head.bottom = torso.top

  // Custom anchors with direction for perpendicular curve entry
  anchor crown [position: head.top - 4, direction: up]
  anchor feet [position: torso.bottom + 4, direction: down]
}

person alice
person bob

constrain bob.left = alice.right + 80
constrain bob.vertical_center = alice.vertical_center

// Invisible via points control curve shape
circle bottom_via [size: 1, fill: none, stroke: none]
circle top_via [size: 1, fill: none, stroke: none]

constrain bottom_via.center_x = midpoint(alice, bob)
constrain bottom_via.center_y = alice.bottom + 30
constrain top_via.center_x = midpoint(alice, bob)
constrain top_via.center_y = alice.top - 30

// S-curved connections using custom anchors
alice.feet -> bob.feet [routing: curved, via: bottom_via, label: "request"]
bob.crown -> alice.crown [routing: curved, via: top_via, label: "response"]

Templates with custom anchors for semantic connection points.
Via points control curve shape, anchor directions ensure
perpendicular entry. Labels auto-position at curve apex.

EXAMPLE 4: Explicit positioning with x/y
----------------------------------------
row container {
  rect a [width: 50, height: 50, fill: #e3f2fd]
  rect b [width: 50, height: 50, fill: #bbdefb, x: 200, y: 100]
  rect c [width: 50, height: 50, fill: #90caf9]
}

The x and y modifiers override automatic layout positions.
Element 'b' is placed at absolute coordinates (200, 100)
while 'a' and 'c' follow normal row positioning.

EXAMPLE 5: Complex templates with internal constraints, leads, and anchors
--------------------------------------------------------------------------
// Templates can have rich internal structure using paths, constraints,
// and lead extensions for clean connection points.

// Simple template: resistor with lead extensions
template "resistor" (value: "R") {
    rect body [width: 40, height: 16, fill: none, stroke: foreground-1, stroke_width: 2, label: value, font_size: 10]

    // Lead extensions: short bars that extend from the shape edges
    rect left_lead [width: 10, height: 2, fill: foreground-1, stroke: none]
    rect right_lead [width: 10, height: 2, fill: foreground-1, stroke: none]
    constrain left_lead.right = body.left
    constrain left_lead.center_y = body.center_y
    constrain right_lead.left = body.right
    constrain right_lead.center_y = body.center_y

    // Anchors at lead tips (not body edges) for cleaner routing
    anchor left_conn [position: left_lead.left, direction: left]
    anchor right_conn [position: right_lead.right, direction: right]
}

// Complex template: LED with path-based triangle and emission arrows
template "led" (color: accent-1) {
    path triangle [fill: none, stroke: foreground-1, stroke_width: 2] {
        vertex tl [x: 0, y: 0]
        line_to tr [x: 20, y: 0]
        line_to tip [x: 10, y: 18]
        close
    }

    rect cathode_bar [width: 24, height: 2, fill: foreground-1, stroke: none]
    constrain cathode_bar.center_x = triangle.center_x
    constrain cathode_bar.top = triangle.bottom + 3

    // Vertical leads extending away from the symbol
    rect anode_lead [width: 2, height: 15, fill: foreground-1, stroke: none]
    constrain anode_lead.center_x = triangle.center_x
    constrain anode_lead.bottom = triangle.top

    rect cathode_lead [width: 2, height: 15, fill: foreground-1, stroke: none]
    constrain cathode_lead.center_x = cathode_bar.center_x
    constrain cathode_lead.top = cathode_bar.bottom

    // Emission arrows using parameterized color
    path arrow1 [fill: none, stroke: color, stroke_width: 1.5] {
        vertex a [x: 0, y: 8]
        line_to b [x: 10, y: 0]
    }
    constrain arrow1.left = triangle.right + 4
    constrain arrow1.top = triangle.top + 2

    anchor anode [position: anode_lead.top, direction: up]
    anchor cathode [position: cathode_lead.bottom, direction: down]
}

// Ground symbol: stacked horizontal lines decreasing in width
template "ground_sym" {
    rect line1 [width: 40, height: 3, fill: foreground-2, stroke: none]
    rect line2 [width: 26, height: 3, fill: foreground-2, stroke: none]
    rect line3 [width: 12, height: 3, fill: foreground-2, stroke: none]
    constrain line2.top = line1.bottom + 4
    constrain line3.top = line2.bottom + 4
    constrain line2.center_x = line1.center_x
    constrain line3.center_x = line1.center_x
    anchor conn [position: line1.top - 4, direction: up]
}

// Instantiate templates with parameters
resistor r1 [value: "10kΩ"]
resistor r2 [value: "220Ω", rotation: 90]   // Rotation works on templates
led status [color: green]
ground_sym gnd1

// Position with constraints (not row/col — this is free-form layout)
constrain r1.center_x = 100
constrain r1.center_y = 100

constrain r2.center_x = r1.center_x
constrain r2.top = r1.bottom + 40

constrain status.anode_x = r2.right_conn_x
constrain status.top = r2.bottom + 30

constrain gnd1.conn_x = status.cathode_x
constrain gnd1.top = status.bottom + 30

// Connect via semantic anchors with undirected connections
r1.right_conn -- r2.left_conn [stroke: accent-1, stroke_width: 2]
r2.right_conn -- status.anode [stroke: accent-1, stroke_width: 1.5]
status.cathode -- gnd1.conn [stroke: foreground-2, stroke_width: 1.5]

Complex templates use internal constraints to position sub-elements
relative to each other. Lead extensions provide clean anchor points
away from the shape body. The `path` element draws arbitrary shapes
with vertex/line_to/arc_to/close commands. Parameters like `color`
and `value` customize each instance. Rotation works on template
instances. Use `--` for undirected connections (schematic style).

EXAMPLE 6: Constraint-based layout for complex multi-domain diagrams
--------------------------------------------------------------------
// For diagrams with 10+ elements and cross-group connections,
// use constraint-based positioning instead of nested row/col.
// This gives you full control over element placement.

// === TEMPLATES ===

template "service_box" (name: "Service") {
    rect body [width: 130, height: 50, fill: accent-light, stroke: accent-dark, stroke_width: 2, label: name]
    rect left_lead [width: 8, height: 2, fill: accent-dark, stroke: none]
    rect right_lead [width: 8, height: 2, fill: accent-dark, stroke: none]
    constrain left_lead.right = body.left
    constrain left_lead.center_y = body.center_y
    constrain right_lead.left = body.right
    constrain right_lead.center_y = body.center_y
    anchor left_conn [position: left_lead.left, direction: left]
    anchor right_conn [position: right_lead.right, direction: right]
    anchor top_conn [position: body.top - 4, direction: up]
    anchor bottom_conn [position: body.bottom + 4, direction: down]
}

template "data_store" (name: "DB") {
    ellipse body [width: 130, height: 55, fill: secondary-light, stroke: secondary-dark, stroke_width: 2, label: name]
    anchor top_conn [position: body.top - 4, direction: up]
    anchor bottom_conn [position: body.bottom + 4, direction: down]
    anchor left_conn [position: body.left - 4, direction: left]
    anchor right_conn [position: body.right + 4, direction: right]
}

// === ELEMENTS ===
// Wrap everything in a group and constrain every element.
// Group uses column layout by default — unconstrained elements
// fall back to column stacking. Declare backgrounds FIRST.

group diagram {
    // Background rects (drawn first = behind everything else; or give
    // them z_order: -1 to declare them anywhere, see EXAMPLE 11)
    rect prod_bg [width: 550, height: 200, fill: accent-light, stroke: accent-dark, stroke_width: 2, opacity: 0.25,
                  label: "Production Zone", label_position: above, align: start, label_fill: accent-dark]
    rect data_bg [width: 550, height: 120, fill: secondary-light, stroke: secondary-dark, stroke_width: 2, opacity: 0.25,
                  label: "Data Layer", label_position: above, align: start, label_fill: secondary-dark]

    // Entry point
    rect gateway [width: 140, height: 50, fill: foreground-3, stroke: foreground-1, stroke_width: 2, label: "API Gateway"]

    // Production services
    service_box auth [name: "Auth Service"]
    service_box orders [name: "Order Service"]
    service_box notify [name: "Notification"]

    // Data stores
    data_store users_db [name: "Users DB"]
    data_store orders_db [name: "Orders DB"]
    data_store cache [name: "Redis Cache"]
}

// === LAYOUT VIA CONSTRAINTS ===
// Use absolute coordinates. Backgrounds declared first render behind.

// Backgrounds (y positions chosen to surround their content)
constrain prod_bg.center_x = 300
constrain prod_bg.center_y = 170

constrain data_bg.center_x = 300
constrain data_bg.center_y = 370

// Zone titles ride on their background rect. `label_position: above` keeps
// the title clear of the box and inside the canvas, so there is no offset
// to guess and nothing to re-derive when a zone moves or resizes.

// Gateway at top center
constrain gateway.center_x = 300
constrain gateway.center_y = 40

// Services row at y=170 (inside prod_bg)
constrain auth.center_x = 140
constrain auth.center_y = 170
constrain orders.center_x = 300
constrain orders.center_y = 170
constrain notify.center_x = 460
constrain notify.center_y = 170

// Data stores row at y=370 (inside data_bg)
constrain users_db.center_x = 140
constrain users_db.center_y = 370
constrain orders_db.center_x = 300
constrain orders_db.center_y = 370
constrain cache.center_x = 460
constrain cache.center_y = 370

// === CONNECTIONS ===

// Gateway to services
gateway.bottom -> auth.top_conn [label: "auth"]
gateway.bottom -> orders.top_conn [label: "orders"]

// Service to service
orders.right_conn -> notify.left_conn [label: "events"]

// Services to data
auth.bottom_conn -> users_db.top_conn [label: "read/write"]
orders.bottom_conn -> orders_db.top_conn [label: "read/write"]
orders.bottom_conn -> cache.top_conn [routing: curved, label: "cache"]

Constraint-based layout: each element is positioned with explicit
constraints rather than relying on row/col nesting. This approach
gives full control over spacing, alignment, and grouping. Background
rects with opacity create visual zones. Use `midpoint()` to center
between elements. Templates with lead extensions provide clean anchors.
This is the RECOMMENDED approach for complex diagrams.

EXAMPLE 7: Heatmap with fill_opacity
------------------------------------
// One hue, varying fill_opacity per cell — no need to pre-compute a
// blended hex per cell. Intensity reads as alpha over the same color.
grid heat [cols: 3] {
    rect a1 [fill: secondary-1, fill_opacity: 0.2]
    rect a2 [fill: secondary-1, fill_opacity: 0.4]
    rect a3 [fill: secondary-1, fill_opacity: 0.6]
    rect b1 [fill: secondary-1, fill_opacity: 0.4]
    rect b2 [fill: secondary-1, fill_opacity: 0.6]
    rect b3 [fill: secondary-1, fill_opacity: 0.8]
    rect c1 [fill: secondary-1, fill_opacity: 0.6]
    rect c2 [fill: secondary-1, fill_opacity: 0.8]
    rect c3 [fill: secondary-1, fill_opacity: 1.0]
}

`fill_opacity` keeps the color token and emits an SVG `fill-opacity`
attribute. `stroke_opacity` and `opacity` (whole element) work the same
way. All three are clamped to the 0..1 range.

EXAMPLE 8: Attention heatmap with labels and a callout
------------------------------------------------------
// A labelled lower-triangular attention matrix: one hue, per-cell
// fill_opacity for intensity, row/col labels in gutters, and a callout
// pinned to a cell. Declare only the cells you want — the rest stay empty.
grid heat [cols: 6, rows: 6, gap: 5, cell_width: 56, cell_height: 56,
           col_labels: ["The","cat","sat","because","it","was"],
           row_labels: ["The","cat","sat","because","it","was"]] {
    rect [at: [0,0], fill: secondary-1, fill_opacity: 1.00]
    rect [at: [1,0], fill: secondary-1, fill_opacity: 0.30]
    rect [at: [1,1], fill: secondary-1, fill_opacity: 0.70]
    rect [at: [2,0], fill: secondary-1, fill_opacity: 0.20]
    rect [at: [2,1], fill: secondary-1, fill_opacity: 0.40]
    rect [at: [2,2], fill: secondary-1, fill_opacity: 0.90]
    // ... only the lower-triangular cells ...
}
callout tag [label: "= subject", pointer: down, stroke: accent-1, fill: background-1]
constrain tag.tip = heat.cell(1,1).top - 4

The grid lays cells on a regular lattice; `[at: [row, col]]` places each
cell (0-indexed, sparse). `grid.cell(row, col)` addresses a cell's box in
constraints, and a callout's `tip` anchor (the pointer apex) is aimed at it
with a point-constraint. Callouts auto-size to their label and may overlap
the content they annotate.

EXAMPLE 9: Grid as an alignment scaffold (not a heatmap)
--------------------------------------------------------
// A grid is the general answer to "these should line up in rows and
// columns" — a worked sum, a table, a calendar. Cells carry the layout;
// cell refs position the rule and frame a block. No hand-computed pitch,
// no per-digit constraints.
grid digits [cols: 4, rows: 3, gap_x: 6, gap_y: 12, cell_width: 38, cell_height: 44,
             col_labels: ["1000", "100", "10", "1"]] {
    text "2" [at: [0, 0], align: center]
    text "4" [at: [0, 1], align: center]
    text "7" [at: [0, 2], align: center]
    text "1" [at: [0, 3], align: center]
    text "+" [at: [1, 0], align: center]
    text "3" [at: [1, 2], align: center]
    text "9" [at: [1, 3], align: center]
    text "2" [at: [2, 2], align: center]
    text "8" [at: [2, 3], align: center]
}

// The rule sits IN the gap between the rows, not on the boundary.
rect rule [height: 3, fill: foreground-1, stroke: none]
constrain rule.left = digits.cell(1, 0).left
constrain rule.right = digits.cell(1, 3).right
constrain rule.top = digits.cell(1, 3).bottom + 2

rect carry [fill: accent-light, stroke: accent-dark, opacity: 0.4]
constrain carry contains digits.cell(0, 3), digits.cell(2, 3) [padding: 2]

`align: center` centres each digit in its cell, so the columns line up
whatever the glyph widths. The highlight is never sized by hand:
`contains` over two cell refs spans the block between them.

Three details that make this lint clean. The grid has a gap between rows
(`gap_y: 12`), so the rule has somewhere to live: with no gap the cells touch
and a rule on a row boundary necessarily overlaps a neighbouring row. The
columns stay tight (`gap_x: 6`); `gap` alone would set both. The rule is
positioned by its edges (`.left` / `.right` / `.top`), not by `contains`,
which would free its height and turn 3px into a whole cell. And the
highlight is drawn see-through, so the digits resting on it are not
collisions.

EXAMPLE 10: One caption across a keyframe walkthrough
-----------------------------------------------------
// `transform <element> [label: "..."]` rewrites an element's words for a
// frame, so one caption follows the steps instead of one text element
// per step plus its constraints and show/hide rules.
row stage [gap: 60] {
    rect client [width: 120, height: 50, label: "client"]
    rect server [width: 120, height: 50, label: "server"]
}
client.right -> server.left as call

text "the client sends a request" caption [align: start]
constrain caption.center_x = stage.center_x
constrain caption.y = stage.bottom + 24

keyframe "send" {
    hide call
}
keyframe "arrive" {
    show call
    transform caption [label: "the server receives it"]
}
keyframe "reply" {
    transform caption [label: "and answers", fill: accent-dark]
}

The caption is auto-sized: it is laid out for the longest wording any
frame gives it, so it never resizes and the text holds its position from
frame to frame. `--frames-to-dir out/` renders them all; `--animate`
plays them back.


EXAMPLE 11: Layering with z_order (under, over, and changing layer)
-------------------------------------------------------------------
// Declaration order first: later is drawn on top. z_order is for what
// order cannot say.
template "doc" (title: "design.md") {
    rect page [width: 200, height: 120, fill: background-1, stroke: foreground-1, stroke_width: 2,
               label: title, label_position: inside]
    // A part over things OUTSIDE its instance: scene(N). A plain z_order
    // on a part only orders it among the parts of its own instance.
    circle badge [size: 34, fill: accent-dark, stroke: none, z_order: scene(1)]
    constrain badge.center_x = page.right
    constrain badge.center_y = page.top
}
doc spec [title: "design.md"]
// Declared after the document, to keep the file in reading order, yet
// drawn under it: z_order: -1 (siblings default to 0).
rect berg [width: 260, height: 160, fill: secondary-light, stroke: none, z_order: -1, overlaps: spec]
rect server [width: 160, height: 160, fill: foreground-3, stroke: foreground-1, stroke_width: 2, label: "server"]
rect card [width: 110, height: 60, fill: accent-light, stroke: accent-dark, stroke_width: 2, label: "card", z_order: 1]
point behind
constrain berg.center_x = spec.page.center_x
constrain berg.top = spec.page.center_y
constrain server.left = spec.page.right + 200
constrain server.top = spec.page.top
constrain card.left = spec.page.right + 40
constrain card.top = server.bottom + 40
constrain behind.center_x = server.center_x
constrain behind.center_y = server.center_y

keyframe "start" { }
// A layer change rides on the move: by default it happens where it is
// not seen (just before the card reaches the server; z_at: start|mid|end
// to choose yourself).
keyframe "hide" { move card to behind [z_order: -1] }
keyframe "back" { move card home [z_order: 1] }

Static `z_order: N` orders siblings (higher on top); `scene(N)` lifts a
template part out of its instance's layer; `move ... [z_order: N]` changes
layer mid-story without a visible pop. Details: `--doc layering`.

MULTI-LINE CARDS
================

A card with a title and detail lines is ONE rect. Reach for `label:` markup
before you reach for a `text` element and a pair of constraints — a separate
text element has to be positioned by hand, does not move when the box moves,
and is what `--lint`'s `label` category exists to talk you out of.

row cards [gap: 34] {
    rect changeset [width: 290, fill: accent-light, stroke: accent-dark, stroke_width: 2,
                    label: "<b>Changeset</b><br><small>temporal_mode: correction</small><br><small>event_ref -> an event</small>"]

    rect note [width: 240, fill: background-1, stroke: foreground-2,
               label: "an explanation long enough that it has to wrap, which it does because the width is explicit"]

    rect chip [width: 90, height: 40, fill: secondary-light, stroke: secondary-dark,
               label: "based_on", label_position: below, align: end]
}

    <br>                  line break
    <b>..</b>             bold
    <i>..</i>             italic
    <small>..</small>     smaller
    <span fill=accent-dark>..</span>   coloured run

The box grows to fit the label. With an explicit `width:` the text wraps to
it and the box grows taller instead of spilling out of the sides. A bare `<`
is literal text, so "a < b" and "Vec<String>" are safe.

`label_position:` puts the label outside the box — `above`, `below`, `left`,
`right` — and `align:` picks the edge it lines up on. An outside label is part
of the element's bounds, so a row or column reserves space for it and a
caption never collides with the next element along.


CAPTIONS THAT FOLLOW THEIR SUBJECT
==================================

A shape's own `label:` cannot differ between frames. A caption that has to
change is a `text` element, and `caption_of:` places it the way that shape's
label would be placed — so it follows when the subject moves or resizes, in
every frame, instead of needing a constraint per position.

row bars [gap: 60] {
    rect v1 [width: 220, height: 48, label: "V1"]
    rect v2 [width: 150, height: 48, label: "V2"]
}
text "confirmed" v1_state [caption_of: v1, label_position: below, align: end,
                           fill: status-success]
text "supersedes V1" v1_note [caption_of: v1, label_position: below, align: end,
                              label_offset: 22]

`label_position`, `align` and `label_offset` mean exactly what they mean for a
shape's own label. Stack captions by giving each the same subject with a bigger
`label_offset` — nothing can be constrained TO a captioned element, because its
position is only known after the solve.

The win is not the line count. Every place a keyframe moves a bar, it no longer
has to remember to move the word underneath it: three scenario files in a real
deck lost a named constraint and the `disable` that went with it.


EXAMPLE 12: Sequence diagram from one list of messages (rows by id, kinds as data, bars, loop/alt, self-message)
----------------------------------------------------------------------------------------------------------------
No dedicated construct, and no hand-written message lines: `let msgs = [...]` is
the conversation, one record per message. The `life` template makes one
invisible row per message on every lifeline (`each t by id in ticks`), named by
the message id, so `api.p[query]` is the row of "query" on API. One `each ...
where m.from != m.to` draws every arrow across its row, numbered from its place
in the list (`"{i+1}: {m.label}"`); a second list, `looks`, says how each kind
looks, read by an outer `each k in looks` (`where m.kind == k.kind`). A
self-message (`where m.from == m.to`) runs from the top of its row to the bottom
through two points beside it. Activation bars come from a third list; frames
(loop, alt with a `split` for its else), the note and the destroy cross name rows
by message id. Auth's lifeline ends early with `until: auth.p[ok]`; the template's
default `(until: rows)` names its own part, so the others need nothing.
Adding a message = adding one record. `label_mask: 4` cuts the line behind a
label that sits on a message; keep the label shorter than its arrow. Full file:
examples/sequence-diagram.ail.

```
// A sequence diagram drawn from one list of messages.
//
// `msgs` is the conversation, in order: one record per message, one row per record on every
// lifeline. A message's `id` names its row (`api.p[query]`), so the bars, frames and the note
// below say which message they mean, not which row number. To add a message, add a record:
// the arrows, rows and numbers follow, and nothing else changes.
//   from, to   the two lifelines (from == to is a self-message)
//   kind       how the arrow looks: one of the kinds in `looks`
//   label      the text on the arrow; its number is its place in the list
let msgs = [
  {id: login,    from: cl,   to: api,  kind: call,  label: "login(user)"},
  {id: verify,   from: api,  to: auth, kind: call,  label: "verify"},
  {id: ok,       from: auth, to: api,  kind: reply, label: "ok"},
  {id: validate, from: api,  to: api,  kind: call,  label: "validate cart"},
  {id: query,    from: api,  to: db,   kind: call,  label: "query"},
  {id: rows,     from: db,   to: api,  kind: reply, label: "rows"},
  {id: ok200,    from: api,  to: cl,   kind: reply, label: "200 OK"},
  {id: conflict, from: api,  to: cl,   kind: reply, label: "409"},
  {id: audit,    from: api,  to: db,   kind: async, label: "audit(event)"},
]
let looks = [
  {kind: call,  head: arrow, dash: none},     // solid, filled head
  {kind: reply, head: open,  dash: dashed},   // dashed, open head
  {kind: async, head: open,  dash: none},     // solid, open head
]
// activation bars: on which lifeline, from which message to which
let bars = [
  {on: api,  first: login,  last: conflict},
  {on: auth, first: verify, last: ok},
  {on: db,   first: query,  last: rows},
]

// An actor: a head and a dashed lifeline over one invisible row per tick, named by the
// tick's id (`p[login]`). The lifeline runs to the bottom of `until`, by default its own rows.
template "life" (ttl: "", ticks: [], until: rows, step: 48, w: 110) {
  col l [gap: 30, padding: 0, align: center] {
    rect hd [label: ttl, width: w, height: 40]
    col rows [gap: 8, padding: 0] {
      each t by id in ticks { rect p [width: 2, height: step, stroke: none, fill: none] }
    }
  }
  hd.bottom -- until.bottom as lifeline [routing: direct, stroke_dasharray: "4,3", stroke: role-muted, stroke_width: 1]
}
// A square frame around the rows from `first` to `last` (loop, alt), and a line that splits a
// frame below the row `after` (the `else` of an alt).
template "frame" (first: "", last: "", ttl: "") {
  rect box [corner_radius: 0, fill: none, stroke: role-muted, overlaps: all]
  constrain box contains first, last [padding_x: 110, padding_y: 0]
  text ttl title [font_size: 11, fill: role-muted, overlaps: all]
  constrain title.left = box.left + 6
  constrain title.top = box.top + 4
}
template "split" (of: "", after: "", ttl: "") {
  rect rule [width: 10.., height: 1, fill: role-muted, stroke: none, overlaps: all]
  constrain rule.left = of.box.left
  constrain rule.right = of.box.right
  constrain rule.center_y = after.bottom + 4
  text ttl title [font_size: 11, fill: role-muted, overlaps: all]
  constrain title.left = rule.left + 6
  constrain title.top = rule.bottom + 3
}

row actors [gap: 100, align: top] {
  life cl   [ttl: "Client",   ticks: msgs]
  life api  [ttl: "API",      ticks: msgs]
  life auth [ttl: "Auth",     ticks: msgs, until: auth.p[ok]]   // destroyed after "ok"
  life db   [ttl: "Database", ticks: msgs]
}

// Messages: an arrow across its own row; a self-message goes out and back within the row.
// `label_mask` cuts the line behind the label, so keep a label shorter than its arrow.
each k in looks {
  each m, i in msgs where m.kind == k.kind and m.from != m.to {
    m.from.p[m.id] -> m.to.p[m.id] [routing: direct, to_end: k.head, stroke_dasharray: k.dash,
                                    label: "{i+1}: {m.label}", label_offset: 0, label_mask: 4]
  }
  each m, i in msgs where m.kind == k.kind and m.from == m.to {
    point out
    point back
    constrain out.center_x = m.from.p[m.id].center_x + 40
    constrain out.center_y = m.from.p[m.id].top
    constrain back.center_x = out.center_x
    constrain back.center_y = m.from.p[m.id].bottom
    m.from.p[m.id].top -> m.to.p[m.id].bottom [routing: direct, via: [out, back], to_end: k.head, stroke_dasharray: k.dash]
    text "{i+1}: {m.label}" self_t [font_size: 11, fill: role-muted]
    constrain self_t.left = out.right + 6
    constrain self_t.center_y = m.from.p[m.id].center_y
  }
}

each b in bars {
  rect bar [width: 10, corner_radius: 0, fill: role-surface-2, stroke: role-muted, overlaps: all]
  constrain bar.center_x = b.on.p[b.first].center_x
  constrain bar.top = b.on.p[b.first].center_y - 4
  constrain bar.bottom = b.on.p[b.last].center_y + 4
}

// fragments, a note and a destroy cross, each pointing at the message it is about
frame loop_f [first: api.p[query], last: db.p[rows], ttl: "loop [each item]"]
frame alt_f [first: cl.p[ok200], last: api.p[conflict], ttl: "alt [in stock]"]
split alt_else [of: alt_f, after: cl.p[ok200], ttl: "[out of stock]"]
rect note1 [label: "read-only<br>replica", width: 96, height: 36, corner_radius: 0, fill: role-warn-soft, stroke: role-muted, font_size: 11, overlaps: all]
constrain note1.center = db.p[validate].center
rect x1 [width: 16, height: 2, rotation: 45, fill: role-muted, stroke: none, overlaps: all]
rect x2 [width: 16, height: 2, rotation: -45, fill: role-muted, stroke: none, overlaps: all]
constrain x1.center_x = auth.p[ok].center_x
constrain x1.center_y = auth.p[ok].bottom
constrain x2.center = x1.center
```


EXAMPLE: First-class motion (commits on a timeline)
---------------------------------------------------
Self-contained: the built-in library brings the macros AND templates they fit
(`git_station`, `git_file`; its full text is at the end of --skill-animation).
Renders as is with --animate; lint clean.

import "ail:motion/git"

group folder {
    rect folder_bg [fill: background-1, stroke: foreground-1, stroke_width: 2, corner_radius: 10]
    col files [gap: 0] {
        git_file f_login [name: "login.py", value: "84 lines"]
        git_file f_cart [name: "cart.py", value: "57 lines"]
    }
}
constrain folder_bg contains files [padding: 8]

path track [through: [c1.dot, c2.dot], drawn: 0, stroke: accent-1, stroke_width: 12, fill: none]
row history [gap: 60, align: dot] {
    git_station c1 [name: "Login page", meta: "anna · Mon"]
    git_station c2 [name: "Fix cart", meta: "ben · Tue"]
}
constrain history.left = folder.right + 80
constrain c1.dot.center_y = folder.center_y

keyframe "start" { }
keyframe "first" { commit(folder, c1, track) }     // flash, fly a copy, pop the station, grow the line
keyframe "second" {
    change(f_cart, "58 lines")                      // the value rolls, the row glows
    then { commit(folder, c2, track) }
}

Bigger scenes (slides, branches, a merge request, going back in history) are in
the repository: examples/motion/git-copies.ail, git-snapshots.ail, git-branches.ail.

No coordinates in any motion statement: stations, lines and flights are found by
name, so relabelling or moving things never touches the choreography. Check it
with --timeline, --frame N --at 50%, and --frames-strip N.

EXAMPLE: Traffic on a wire (request, response, a token's loop)
--------------------------------------------------------------
What travels rides the arrow it travels on (`along:`). Two arrows between the
same pair are drawn as two lanes, so request and reply never overlap.

```
app.right -> llm.left as request [label: "whole log"]
llm.left -> app.right as reply [label: "one message"]   // second lane, label outside

keyframe "call" {
    show request [enter: draw]
    when request shown { fly ghost(log) to llm [along: request] }   // the whole log goes in
}
keyframe "answer" {
    show reply [enter: draw]
    // ONE message comes back over the wire, at its own size, and lands in the log
    when reply shown { show m3 [from: llm, along: reply, scale: 1, ease: glide] }
}

keyframe "fail" {
    move token along run                                  // to the end of the arrow
    when token arrives { shake check }
    when check shaken { move token home [along: retry] }  // a lap
}
keyframe "pass" {
    move token along run
    when token arrives { pulse check }
    when check pulsed { move token along run, out }       // through the check and out
}
```
Full scenes: examples/motion/request-response.ail and examples/state-machine.ail.

EXAMPLE: Line ends (UML relations, crow's foot)
-----------------------------------------------
`from_end` / `to_end` choose the decoration at each end of any connection:
arrow, open, triangle, diamond, diamond-filled, crow, circle, none. They
override what the operator implies and combine with dashed lines.

```
// one record per row; a record field can carry an end value or a dash style
each r in [
    {a: "Dog",    b: "Animal",    fe: "none",    te: "triangle", dash: none,   means: "inheritance"},
    {a: "Garage", b: "Car",       fe: "diamond", te: "none",     dash: none,   means: "aggregation"},
    {a: "Client", b: "Service",   fe: "none",    te: "open",     dash: dashed, means: "dependency"},
    {a: "Plug",   b: "Socket",    fe: "circle",  te: "circle",   dash: none,   means: "socket"},
] {
    rect a [label: r.a]   rect b [label: r.b]
    a -- b [from_end: r.fe, to_end: r.te, stroke_dasharray: r.dash, label: r.means]
}
```
Full scene: examples/line-ends.ail.

## Cross-functional flowchart: lanes from records, nodes that place themselves

Lanes are one `lane` per record (`{id: am, name: "Account manager"}`), named by
the id with `each r by id` (`lanes_am`, reached as `lanes[am]`). A lane's body has `width: 200..`, a
size range, so it stretches to the last step; a fixed part would move its whole
lane instead (`--doc sizes`). Steps are invisible points `step[k]`. Every node
template takes `name:`, `lane:` and `at:` and constrains itself onto the lane's
centre line and the step, and every node kind declares the same anchors `l r t b`.

```
template "lane" (name: "", across: "", h: 130) {
    rect hd [width: 36, height: h, corner_radius: 0, fill: role-surface-2, stroke: role-muted]
    text "{name}" title [rotation: -90, font_size: 13, font_weight: bold, overlaps: hd]
    rect body [width: 200.., height: h, corner_radius: 0, fill: none, stroke: role-muted, overlaps: all]
    constrain title.center = hd.center
    constrain body.left = hd.right
    constrain body.top = hd.top
    constrain body.right = across.right + 50
}
template "task" (name: "", lane: "", at: "") {
    rect box [label: name, width: 130, height: 56, corner_radius: 8, fill: role-series-1-soft, font_size: 12]
    constrain box.center_y = lane.center_y
    constrain box.center_x = at.center_x
    anchor l [position: box.left, direction: left]
    anchor r [position: box.right, direction: right]
    anchor t [position: box.top, direction: up]
    anchor b [position: box.bottom, direction: down]
}

col pool [gap: 0, padding: 0, align: hd] {
    each r by id in [{id: cust, name: "Customer"}, {id: am, name: "Account manager"}] {
        lane lanes [name: r.name, across: steps]
    }
}
row steps [gap: 150] {
    each k in [1, 2, 3] { point step }
}
constrain steps.left = lanes[cust].body.left + 60
constrain steps.center_y = pool.top

task ask  [name: "Ask for a quote", lane: lanes[cust], at: step[1]]
task make [name: "Prepare quote",   lane: lanes[am],   at: step[2]]
task sign [name: "Sign",            lane: lanes[cust], at: step[3]]
ask.b  -> make.l
make.r -> sign.b
```
Full scene, with gateways, events and a message flow: examples/cross-functional.ail.

## Class diagram: lists and `each` make the compartments

A class box is a template that takes lists (`attrs`, `ops`); `each` writes one
row per item, so an empty list leaves no compartment. Rows are plain rects,
each compartment's outline is a stroke-only rect that `contains` its rows, and
the `shell` rect gives the rounded corners (inner rects are `clip: shell`).
The template declares anchors for the box, each compartment and each row
(`entry.r_a_2` is the right edge of attribute 2), so a relation can leave from
the exact row. `from_label`/`to_label` carry the multiplicities and `label` the
relationship's name. Elements made inside an `each` are numbered `a_row_1`,
`a_row_2`, ... (`--doc each`).

```
template "cls" (ttl: "", stereo: [], attrs: [], changed: [], added: [], later: [], ops: [],
                w: 220, rh: 20, hfill: secondary-light, edge: foreground-2, radius: 8) {
    col box [gap: 0, padding: 0] {
        col head [gap: 0, padding: 0] {
            each s in stereo {
                rect st_row [width: w, height: 16, label: s, font_size: 11, fill: hfill, stroke: none, clip: shell, label_fill: role-muted]
            }
            rect hd [width: w, height: 28, label: ttl, font_weight: bold, fill: hfill, stroke: none, clip: shell]
        }
        col at [gap: 0, padding: 0] {
            each a in attrs {
                rect a_row [width: w, height: rh, label: a, align: start, font_size: 12, fill: background-1, stroke: none, clip: shell]
                anchor l_a [position: a_row.left, direction: left]
                anchor r_a [position: a_row.right, direction: right]
            }
            each a in changed {
                rect c_row [width: w, height: rh, label: a, align: start, font_size: 12, fill: role-warn-soft, stroke: none, clip: shell]
            }
            each a in added {
                rect n_row [width: w, height: rh, label: a, align: start, font_size: 12, fill: role-ok-soft, stroke: none, clip: shell]
            }
            each a in later {
                rect l_row [width: w, height: rh, label: a, align: start, font_size: 12, fill: role-surface-2, stroke: none, clip: shell, label_fill: role-muted]
            }
        }
        col op [gap: 0, padding: 0] {
            each o in ops {
                rect o_row [width: w, height: rh, label: o, align: start, font_size: 12, fill: background-1, stroke: none, clip: shell]
                anchor l_o [position: o_row.left, direction: left]
                anchor r_o [position: o_row.right, direction: right]
            }
        }
    }
    rect hd_box [fill: none, stroke: edge, corner_radius: 0, clip: shell]
    constrain hd_box contains head
    rect at_box [fill: none, stroke: edge, corner_radius: 0, clip: shell]
    rect op_box [fill: none, stroke: edge, corner_radius: 0, clip: shell]
    rect shell [fill: none, stroke: edge, corner_radius: radius]
    constrain at_box contains at
    constrain op_box contains op
    constrain shell contains box
    anchor t_box [position: shell.top, direction: up]
    anchor b_box [position: shell.bottom, direction: down]
    anchor l_hd [position: hd.left, direction: left]
    anchor r_hd [position: hd.right, direction: right]
    anchor l_at [position: at_box.left, direction: left]
    anchor r_at [position: at_box.right, direction: right]
    anchor l_op [position: op_box.left, direction: left]
    anchor r_op [position: op_box.right, direction: right]
}
row r1 [gap: 140, align: top] {
    cls entry [ttl: "TimeEntry", attrs: ["date: date", "hours: decimal"], changed: ["status: EntryStatus"], ops: ["+ approve()"]]
    cls invoice [ttl: "Invoice", attrs: ["number: str", "total: decimal", "paid: bool"], ops: ["+ pay()"]]
}
entry.r_a_2 -- invoice.l_a_2 [from_label: "*", to_label: "0..1", label: "billed on", bundle: apart]
```
The full scene, with all anchors and relations, is `examples/class-diagram.ail`.

Tight rows: give rows a fixed `rh` (20) and write ONE short attribute per row (`name: Type`);
without a height each row gets airy label padding, and a long text with `height: 22` trips
label-overflow. To mark rows (new / changed / later) add lists `changed`, `added`, `later`
after `attrs`, each `each` writing rows with a tint (`role-warn-soft`, `role-ok-soft`, grey);
`hfill` tints the title bar. `each` iterates plain strings, so group rows by status.
Lay out: boxes joined title-to-title need `constrain a.top = b.top`; top-to-bottom joins need
equal `center_x`; no line may cross a box in between. `examples/class-diagram.ail` has the full
template (status lists, row anchors, legend).

## Repeating with `each`

```
each n, i in ["Open", "Save", "Quit"] {        // top level: literal list
    rect item [label: "{i}. {n}", width: 120, height: 24]   // item_1, item_2, item_3
}
template "menu" (items: []) {                   // inside a template: a list parameter
    col m [gap: 4] { each it in items { rect entry [label: it, width: 120, height: 24] } }
}
menu file [items: ["New", "Open"]]              // file.entry_1, file.entry_2
```

Items written as names stand for elements wherever a name goes (connection ends,
`constrain`, `contains`, `via:`). A server sending data to three clients, the middle one
failing:

```
rect srv [label: "Server", width: 130, height: 70, fill: role-primary, label_fill: role-surface]
col clients [gap: 34, align: start] {
    rect c1 [label: "Client 1", width: 130, height: 44, fill: role-ok-soft]
    rect c2 [label: "Client 2<br><i>timeout</i>", width: 130, height: 54, fill: role-error-soft, stroke: role-error, stroke_width: 2]
    rect c3 [label: "Client 3", width: 130, height: 44, fill: role-ok-soft]
}
constrain clients.left = srv.right + 200
constrain srv.center_y = clients.center_y

each c in [c1, c3] {                            // the healthy clients: one arrow each
    srv.right -> c.left [routing: direct, label: "data", stroke: role-ok]
}
srv.right -> c2.left [routing: direct, label: "no reply", stroke: role-error, stroke_dasharray: dashed, to_end: open]
```

Items may be records: `t.from`, `t.on` are fields, usable as names, values and in strings.
`let` names a list for the whole file, so several blocks read one table; `where` picks the
items a block is for. A state machine from its transition table, with one running instance
animated (`examples/state-machine.ail`):

```
let tr = [
    {from: idle,    to: playing, on: "play"},
    {from: playing, to: paused,  on: "pause"},
    {from: paused,  to: playing, on: "resume"},
    {from: playing, to: idle,    on: "stop"},
    {from: playing, to: failed,  on: "error"},
    {from: failed.left, to: idle.bottom, on: "reset"},   // a field may name a side
    {from: playing, to: playing, on: "seek"},            // a self-transition
]
each t in tr where t.from != t.to {          // between two states: one arrow
    t.from -> t.to as arc [label: t.on]       // `as arc` names them arc_1, arc_2, ... for motion
}
each t in tr where t.from == t.to {          // to the same state: a loop over its top
    point up_l
    point up_r
    constrain up_l.center_x = t.from.center_x - 22
    constrain up_r.center_x = t.from.center_x + 22
    constrain up_l.center_y = t.from.top - 34
    constrain up_r.center_y = up_l.center_y
    t.from.top -> t.to.top as arc [via: [up_l, up_r], routing: curved, label: t.on]   // arc_7
}
col legend [gap: 4, align: start] {          // the same list again: one row per event
    text "Events" legend_title [font_size: 14, font_weight: 700, align: start]
    each t, i in tr { text "{i+1}. {t.on}" ev [font_size: 13, align: start] }
}
```

`where` keeps positions: the self-loop is `arc_7` because it is item 7 of `tr`, so names
and `{i+1}` agree across every block over the same list.

`stage[k]` names copy k of what an `each` made (`stage[2]` is `stage_2`); inside an `each`
the index may be computed (`stage[k+1]`, `p[i+1]`, `p[m.row]`). A pipeline whose boxes come
from one `each` and whose hand-off arrows come from another (`examples/pipeline.ail`):

```
row stages [gap: 70, align: center] {
    each s in [{name: "Build", tool: "cargo build"}, {name: "Test", tool: "cargo test"},
               {name: "Package", tool: "docker build"}] {
        rect stage [label: "<b>{s.name}</b><br>{s.tool}", width: 120, height: 56]
    }
}
each k in [1, 2] { stage[k].right -> stage[k+1].left [routing: direct] }   // copies count from 1
stage[2].top -> stage[1].top [routing: curved, label: "fails"]               // one copy by number
```

## Optional parts with `if`

`if COND { ... } else { ... }` keeps one body and drops the other before layout; a dropped
body takes no space, so a row closes up. The condition reads template parameters (defaults
included) and `each` variables, like `where`. A support-ticket flow whose AI steps carry a
badge and whose checked steps are followed by a review gate (`examples/support-flow.ail`):

```
template "step" (name: "", who: "", ai: false) {
    rect box [label: "<b>{name}</b><br>{who}", width: 150, height: 64, fill: role-surface-2, font_size: 12]
    if ai {                                       // only on steps with ai: true
        rect badge [label: "AI", width: 30, height: 18, corner_radius: 9, fill: role-primary,
                    label_fill: role-surface, stroke: none, font_size: 10, overlaps: box]
        constrain badge.right = box.right - 6     // its constraints go inside the `if` too
        constrain badge.top = box.top + 6
    }
}
let steps = [
    {id: receive,  name: "Receive",     who: "mail gateway",  ai: false, check: false, next: classify},
    {id: classify, name: "Classify",    who: "triage agent",  ai: true,  check: true,  next: draft},
    {id: draft,    name: "Draft reply", who: "support agent", ai: true,  check: true,  next: send},
    {id: send,     name: "Send",        who: "helpdesk",      ai: false, check: false, next: none},
]
row flow [gap: 36, align: center] {
    each s by id in steps {
        step st [name: s.name, who: s.who, ai: s.ai]
        if s.check { rect gate [label: "review", width: 64, height: 28, corner_radius: 14] }
    }
}
each s by id in steps where s.next != none {      // `where`: which items; `if`: which part
    if s.check {
        st[s.id].right -> gate[s.id].left [routing: direct]
        gate[s.id].right -> st[s.next].left [routing: direct]
    } else {
        st[s.id].right -> st[s.next].left [routing: direct]
    }
}
```

`if ai` holds for `true` and fails for `false`; `if m.kind == call and m.from != m.to` compares.
Referring to `st[receive].badge` from outside is an error: "`st_receive.badge` only exists when
`ai` is true".
