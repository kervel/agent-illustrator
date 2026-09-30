AGENT ILLUSTRATOR GRAMMAR
=========================

SHAPES
------
rect [name] [modifiers]      Rectangle (default 60x40)
circle [name] [modifiers]    Circle
ellipse [name] [modifiers]   Ellipse
text "content" [name] [mod]  Text element
path [name] [mod] { ... }    Custom shape with vertices/arcs
point name                   An invisible place with no size, for constraints to
                             name (`point hub; constrain hub.center = docs.center`)
callout [name] [mod]         Annotation pill with a triangular pointer
                             [pointer: up|down|left|right] (default down)
                             auto-sizes to its label; exposes a `tip` anchor at
                             the pointer apex. Aim it:
                                 constrain tag.tip = box.top - 4
                                 tag.tip -> box [routing: direct]

PATH COMMANDS (inside path { ... })
-----------------------------------
vertex name [x: N, y: N]               Define point (relative to path origin)
line_to name [x: N, y: N]              Straight line to point
arc_to name [x: N, y: N, ...]          Arc to point
curve_to name [via: elem, x: N, y: N]  Quadratic Bezier (via = external element as control point)
close                                   Close path to first vertex

Arc modifiers:
    radius: <number>              Arc radius (default: auto from bulge)
    bulge: <number>               Arc curvature factor (default: 0.414)
    sweep: clockwise|cw           Arc direction (default)
    sweep: counterclockwise|ccw
    large_arc: true|false         Use major arc (default: false)

LAYOUTS
-------
row [name] [mod] { ... }     Horizontal arrangement
col [name] [mod] { ... }     Vertical arrangement
group [name] [mod] { ... }   Column layout (constrain every element to override)
stack [name] [mod] { ... }   Overlap children centered within largest child
grid [name] [mod] { ... }    Regular lattice of cells

Grid modifiers:
    cols: <n>, rows: <n>          Lattice size (rows optional; inferred otherwise)
    gap: <n>                      Space between cells
    cell_width: <n>, cell_height: <n>   Cell size (children without an explicit
                                  size inherit it; explicit size centers in cell)
    col_labels: ["a","b",...]     Text labels above each column
    row_labels: ["a","b",...]     Text labels in a left gutter for each row
Grid children:
    rect [at: [row, col], ...]    Place a child in a cell (0-indexed). Children
                                  without `at:` fill row-major. Unoccupied cells
                                  stay empty (transparent) — sparse/triangular ok.
Grid cell addressing (in constrain / connections / contains):
    grid.cell(row, col)           Resolves to that cell's box, e.g.
                                  constrain tag.tip = heat.cell(1,1).top - 4
                                  constrain hl contains g.cell(1,0), g.cell(1,5)  // highlight a row

CONNECTIONS
-----------
a -> b [mod]                Directed arrow from a to b
a -> b -> c [mod]           Chained connections (modifiers apply to last segment)
a <- b [mod]                Directed arrow from b to a
a <-> b [mod]               Bidirectional arrow
a -- b [mod]                Undirected line
a.anchor -> b.anchor        Connect via custom anchors (see ANCHORS)
a -> b as my_conn [mod]     Named connection (referenceable in keyframes)

Connection modifiers:
    routing: orthogonal     Right-angle path (default)
    routing: direct         Straight diagonal line
    routing: curved         Smooth cubic Bezier curve
    via: element            Route curve through element's center
    label: "text"           Add label (at midpoint or curve apex)
    label_at: <number>      Label position along path (0.0=start, 1.0=end, default 0.5)
    label_offset: <number>  Perpendicular distance from path to label (default 10)

STYLE MODIFIERS
---------------
Modifiers go in brackets after the element name:
    rect mybox [fill: blue, stroke: #333, stroke_width: 2]

Common modifiers:
    fill: <color>           Solid fill color
    fill: <pattern>         Pattern fill: hatch, cross_hatch, dots, grid
                            e.g. hatch(accent-1), dots(accent-1, background-light)
    fill: gradient(a, b)         Linear gradient a->b (top->bottom)
    fill: gradient(a, b, deg)    Linear gradient at angle (0=down, 90=right)
    fill: radial_gradient(a, b)  Radial gradient, center a -> edge b
    fill_opacity: <0..1>    Alpha for the fill only (keeps the fill color)
    stroke: <color>         Border color
    stroke_width: <number>  Border thickness
    stroke_opacity: <0..1>  Alpha for the stroke only
    stroke_dasharray: "6,3"  Dash pattern (SVG dasharray); also keywords
                             dashed (= 8,4) and dotted (= 2,2)
    opacity: <0..1>         Alpha for the whole element
    size: <number>          Width and height (square/circle)
    width: <number>         Explicit width
    height: <number>        Explicit height
    gap: <number>           Space between children (layouts)
    label: "text"           Words on the shape. Accepts inline markup:
                              <br>              line break (\n works too)
                              <b>..</b>         bold
                              <i>..</i>         italic
                              <small>..</small> smaller
                              <span fill=accent-dark>..</span>  coloured run
                            A bare `<` is literal text, so "a < b" is fine.
                            The box grows to fit; with an explicit width the
                            text wraps to it instead of overflowing.
    label_position: <where> inside (default) | above | below | left | right
                            An outside label is part of the element's bounds,
                            so layouts reserve space for it.
    label_offset: <number>  Gap from the shape's edge for an outside label
                            (default 6)
    align: start|center|end Inside: text position in its box. above/below:
                            which edge the label aligns to. left/right:
                            top|center|bottom (same spellings). Also the
                            cross-axis alignment of a row's/column's children
                            (left/center/right accepted; default start)
    caption_of: <element>   On a `text` element: attach it to a subject. It is
                            placed the way that subject's own label would be
                            (label_position / align / label_offset) and moves
                            whenever the subject moves or resizes, in every
                            frame. Use it for a caption that has to change
                            between frames, which a shape's own label cannot do
                            without also changing the box. A `constrain` on a
                            captioned element is overridden — --lint says so.
    label_fill: <color>     Colour of the label text (fill colours the shape)
    font_size: <number>     Size of the label / text in px (default 14; the
                            SVG root says font-size="14", so a stylesheet
                            cannot silently change what layout measured)
    font_weight: <n|bold>   Weight of the label / text (layout measures it)
    font_family: mono|sans|serif|"Name"   Face of the label / text
    max_width: <number>     Label keeps its natural width up to this, then wraps
    corner_radius: <number> Rounded corners (rect)
    text "{param}" name     In a template: the text says a parameter's value
    rotation: <degrees>     Rotate element (clockwise)
    class: <name>           Custom CSS class (for external styling)
    z_order: <number>       Render order for groups (higher = on top)
    routing: direct         Diagonal line (vs default orthogonal)
    routing: curved         Smooth curve (for loops, crossings)

COLORS
------
Hex:      #ff0000, #f00
Named:    red, blue, green, steelblue
Symbolic: always with a number or shade (there is no bare `foreground`):
          foreground-1..3, background-1..3, accent-1..3, secondary-1..3,
          text-1..3, and -light / -dark of each (accent-dark, text-light)
          status-success, status-warning, status-error
          — and any other token the active stylesheet defines
Roles:    what a colour is FOR. Reusable scenes and libraries use only these, so
          the same file renders in any theme; a stylesheet maps them
          (`--role-primary: var(--secondary-1);`):
          role-primary, role-secondary (the two main line/accent colours),
          role-ink, role-muted (text), role-rule (dividers, idle lines),
          role-surface, role-surface-2 (cards, panels), role-backdrop (stage),
          role-ok, role-error, role-warn (+ -soft tints: role-ok-soft ...),
          role-series-1..4 (+ -soft): people or categories told apart by colour
Code:     code-keyword, code-string, code-comment, code-number, code-function,
          code-key, code-plain, code-ln, code-bg, code-frame, code-title(-bg),
          code-add, code-del (+ -bg, -strong), code-mark-bg: defaults are roles

CONSTRAINTS
-----------
constrain a.left = b.left              Align left edges
constrain a.center_x = b.center_x      Center horizontally
constrain a.top = b.bottom + 20        Position with offset
constrain a.width = 100                Fixed dimension
constrain a.center_x = midpoint(b, c)  Center between two elements
constrain bg contains a, b [padding: 10]   Auto-size container
constrain a.center_x = 50 as a_home    Name a constraint
disable a_home                         Release a named constraint (top level
                                       or inside a keyframe)
enable a_home                          Reactivate it
layout beside { constrain ... }        A named alternative placement; a keyframe
                                       switches with `use layout beside` and back
                                       with `use layout default` (the declared one).
                                       Its constraints replace declared ones on the
                                       same elements and axis.

Restating a constraint on the same element+property overrides the earlier one,
whatever is on the right-hand side; --lint reports the override. Use `disable`
when you need to release a pin without replacing it — e.g. a file that
composes a shared part it does not own.

Contains: container grows to surround listed elements with padding.
          Container width/height become flexible; position may shift.

Properties: left, right, top, bottom, center_x, center_y, width, height,
            center (both axes: `constrain ring.center = st3.dot.center`)

TEMPLATES
---------
Inline templates:
    template "mytemplate" { ... }        Define reusable group (quoted name)
    mytemplate instance_name [params]    Instantiate template (unquoted)

File-based templates:
    template "icon" from "path/to/file.svg"     Import SVG file (embedded)
    template "photo" from "path/to/file.png"    Import raster image (referenced)

SVG files are embedded directly (content parsed, dimensions from viewBox).
File SVG templates auto-trim their viewBox to the artwork's content bbox (so anchors
hug the drawing and connectors are consistent). Disable per instance with `[trim: false]`.
Raster images (PNG, JPG, JPEG, GIF, WebP, BMP) are referenced by path.
The SVG viewer loads raster images at render time.

Raster images require explicit dimensions:
    photo avatar [width: 60, height: 60]

All file-based templates support modifiers like width, height, rotation:
    icon logo [width: 100, height: 100, rotation: 45]

SVG files with ids are components. Every drawable element with an `id` is a part
of each instance, laid out where the artwork has it: `d.sheet`, `d.fold`,
`d.bar3` work in constraints, anchors, `overlaps:` and every motion verb (a
`transform d.bar3 [stroke: role-error]` recolours it). Parts nest as the file
nests them (moving `d.bars` moves `d.bar3`). Colours in the file may use CSS
variables: theme tokens follow the deck (`fill="var(--role-surface, #fff)"`),
any other variable is an instance argument:
    template "docx" from "assets/docx.svg"   <!-- stroke="var(--b1, #ccc)" inside -->
    docx d [b1: role-primary]                 d.bar1 drawn in the deck's primary
    circle q [overlaps: d]                    constrain q.center_x = d.sheet.left
Ids are prefixed per instance (gradients and clip paths keep working); `width:`
scales the whole instance. XML comments in the file must not contain `--`.

Wrap file templates in inline templates to add anchors:
    template "avatar_img" from "avatar.png"
    template "person_card" (name: "Person") {
        avatar_img photo [width: 60, height: 60]
        rect label_bg [fill: none, label: name]
        anchor top_conn [position: photo.top, direction: up]
    }

ANCHORS
-------
Custom connection points on elements (especially useful in templates).

anchor name [position: elem.property, direction: dir]

Position uses element properties: top, bottom, left, right, center_x, center_y
Direction: up, down, left, right (controls curve perpendicular entry)
Offset supported: elem.property + 10 or elem.property - 5

Example in a template:
    anchor crown [position: head.top - 4, direction: up]
    anchor feet [position: torso.bottom + 4, direction: down]

Connect using dot notation:
    alice.crown -> bob.crown [routing: curved]

Built-in anchors on all shapes: top, bottom, left, right, center

KEYFRAMES
---------
Declarative animation: control visibility and transforms across frames.
Elements are laid out globally; keyframes describe temporal changes.

keyframe "name" {
    show element1, element2          Make elements visible
    hide element3, connection_name   Make elements/connections invisible
    transform element4 [rotation: 45, fill: red]   Per-frame overrides
    disable a_home                   Deactivate a named constraint (this frame on)
    constrain element4.center_x = 300   Add a constraint (this frame on)
    enable a_home                    Reactivate a disabled named constraint
}

Keyframe names become CSS classes (`.frame-<name>`), so they must be letters,
digits, '-' or '_' and must not start with a digit. A name with a space is
rejected rather than renamed, because a renamed frame is one you could not then
pass to --frame.

Keyframes are cumulative: each frame builds on the previous frame's state.
Transforms (visual AND geometry) persist forward, merged per-property; to reset
a property, restate it (e.g. dx: 0).
Without keyframes, all elements are visible (backward compatible).
Named connections (a -> b as name) can be referenced in show/hide.
Referencing nonexistent elements is a hard error.
Connections automatically follow moving/resized endpoints across keyframes (CSS d:
morph where the route shape is unchanged, opacity crossfade where it reshapes).

Transform geometry keys (inside keyframe transform [...]):
    x: N, y: N         Absolute target position
    dx: N, dy: N       Offset relative to the laid-out (frame-0) position
    width: N, height: N   Absolute target size
    scale: N           Uniform scale about the element's center
    rotation: N        Rotation in degrees
Other transform keys: fill, stroke, stroke_width, stroke_dasharray, opacity,
align, label_fill, and label
(rewrites the element's words for that frame -- a text element's content or any
element's label). The box never changes size: auto-sized text is laid out for
the longest wording any frame gives it, and an explicit width is kept as
written (--lint reports text that does not fit it).
Position + rotation animate via a transform on the element's wrapper group (so the
label rides along); size animates via the shape's width/height.

    A key that cannot be animated (z_order, font_size, routing, ...) is
    reported by --lint rather than silently ignored. stroke_dasharray only
    interpolates between patterns with the same dash count, so dashed -> solid
    is written as a zero-length pattern ("0,0"), not `none`, if a smooth
    transition is wanted.

Motion statements (full guide: --skill-animation):
    show a, b [enter: pop|rise|drop|fade|grow|wipe(dir)|draw, from: other,
               delay, duration, ease, stagger, order: start|end|center|random, jitter: 4]
    hide a [exit: fade|shrink|fall|lift|wipe(dir)]
    when line reaches a.dot { ... }   when a line being drawn passes a.dot
    when a shown | hidden | arrives + 0.1 { ... }   when a's latest entrance / exit /
                        move (or layout change) in this keyframe has FINISHED;
                        --timeline prints each event's resolved time
    beat name { ... }   a named group;  after name + 0.1 { ... }  when it has ended
    then { ... }        next beat: when everything before it has ended
    at 1.0 { ... }      1.0s after the keyframe started
    after 0.2 { ... }   0.2s after the previous beat started (a row of these is
                        linted: time by events instead)
    keyframe "k" [auto, after: 0.3] { ... }   plays by itself after the previous one
    draw line [to: elem | 60% | vertex 2]   undraw line [to: ...]
    fly ghost(a) to b, c [scale, arc]   fly proxy from a to b
    move a to b   move a home   move a along path [to: b]
    pulse | shake | nudge | flash | ping | highlight  a [...]   loop a [pulse]
    count total [to: 14900, format: "€ {:,}"]
    transform cap [label: "...", swap: roll|fade|cut]
    swap a -> b [via: flip|fade|morph, flip: member]
    camera focus a [zoom: 1.4]   camera reset
    use layout beside [duration: slow]   use layout default
    set review done [swap: fade]   enter a state the component's template declares:
        template "check" (...) { ...  state done { show tick; transform txt [label: ok] } }
        (motion statements naming its parts; `self` is the instance; what other
        states change and `done` does not goes back to the template's look;
        `set x default` undoes them all). One element: state status broken { ... }
    transform a [fill: initial]   back to the declared value
    motion [title: s.heading, title_swap: roll]   the heading shows each keyframe's title
    motion name(p: element|group|path|anchor|number|text) { ... }   name(args)
    motion [enter: pop, exit: fade]     diagram-wide defaults
    import "file.ail" | "ail:motion/git"   templates + motion macros
Selectors: a, a.b.c (nested parts), group.*, .class, all except a, b

CODE BLOCKS
    code c [lang: python, source: "def f():\n    return 1"]   highlighted, numbered lines
    code c [file: "cart.py", lines: "3-8"]                   from a file (lang from its extension)
    code d [diff: "-    return s\n+    return round(s, 2)"]  -/+ rows, tinted, changed part bold
    options: title: "cart.py · merged" (markup ok), line_numbers: false, font_size: 17,
             marks: "1:role-series-2-soft, 8:role-warn-soft", frame: false (no panel),
             min_chars: 40, appears: later
    Languages: python, js/ts, css, sh, json, yaml, rust, go, java, c, html, sql, ... (syntect)
    Lines are parts: c.line4 (also c.line[4]); c.lines[8..12] selects a range.
    transform c.line1 [source: "def total(cart, coupon=None):", swap: roll]  re-highlights
    remove c.lines[8..12]            hide and close the room they took (hide [exit: collapse])
    show c.line8 [enter: expand]     open it again
    insert c after line 7 [name: conflict, tint: role-warn-soft, source: "<<<<<<< anna\n..."]
                                     new unnumbered lines c.conflict1..N (class conflict),
                                     collapsed until this statement opens them
    Width is measured in monospace: --ail-mono-advance: 0.616 in the stylesheet
    for a face wider than the usual 0.6em (Overpass Mono).

Declarations that serve motion:
    [appears: keyframe | later]   hidden until then; enters there
    path p [through: [a.dot, b.dot], routing: metro, drawn: 0|60%|elem, extend: 40]
        stations side by side on the line get room for their names (the line
        stretches); list every station. spread: even  -> equal steps on a flat run
    rect stage [..., canvas: true]   the picture's bounds; leaving it is reported
    label wrapping: max_width: 240    rows of components: [align: member, pack: tight]
    many at once: doc d* [fname: ["a", "b", "c"]]  ->  d0, d1, d2
                  (templates and plain shapes alike; list-valued arguments zip
                  by index: rect l* [width: [88, 70], fill: [red, blue]])
                  several arguments per instance: one record each, not
                  parallel lists: code k* [items: [{fname: "a.py", c1: role-ok},
                                                   {fname: "b.js", c1: role-primary}]]
                  constrain k*.center = d*.center   one constraint per index
                  (k0 on d0, ...); the lists must be the same length
    keyframe flags: keyframe "k" [auto, after: 0.3] plays on by itself after
                  the previous one; [no_resolve] keeps the previous frame's
                  layout (no constraint re-solve) for a pure style change;
                  [title: "...", note: "..."] host metadata: a deck's header
                  from this step on, and its speaker notes (player: p.meta(k))
    collapsed: true   no room and hidden until `show x [enter: expand]`
    overlaps: card    sits on card on purpose (a corner badge): no overlap finding

Names: every element, named connection and keyframe name must be unique. Part b
of instance a is `a.b` in source and `a_b` internally, so naming something a_b
yourself clashes with it; any clash is an error that locates both declarations.

Named constraints & per-keyframe control:
    constrain a.center_x = 50 as a_home   Name a constraint (handle for disable/enable)
    Inside a keyframe: constrain <expr> (adds; overrides any earlier constraint on the
    same element+property), disable <name>, enable <name>. Without this, the
    always-solved constraints pull elements back to their frame-0 positions.
    An element left with no active constraint holds its frame-0 laid-out position
    (each frame re-solves from the base layout); enable <name> restores a disabled pin.

CLI flags:
    --version          Which binary this is. Worth checking when a documented
                       modifier seems to do nothing: one newer than the binary
                       reads as unknown and is silently dropped.
    --frame N          Render single frame as static SVG (by index or name)
    --frames-to-dir D  Every frame as D/NN-name.svg (--list-frames: just names)
    --list-steps       One line per click step: its frame, then its [auto] frames
    --frame N --at T   A still T into frame N (0.35s, 350ms, 50%)
    --frames-strip N   Contact sheet of frame N at 0/25/50/75/100%
    --states           The storyboard: per click step what is visible (a matrix)
                       and what changes (entrances, exits, transforms, moves,
                       draws, layouts); check it against the STORYBOARD comment
    --timeline         The compiled choreography as a table (--timeline-json: tracks)
    --animate          Embed the motion player (autoplay; click/arrows step)
    --player-js        Print the player for hosts that inline the SVG
    --lint-strict      Like --lint, but warnings fail too (errors always do)
    --stylesheet-css F Repeat to layer stylesheets in order (theme, then deck): later wins
    --crop-to-content [PAD]  The picture is what the frames show (union of all
                       frames, + PAD, default 24), not the stage: for embedding
    --serve [PORT]     Live preview (default 8420): the real player, arrow keys
                       step, a scrubber, reload on save
    --film STEP        HTML page: that click step sampled every 50ms (--film-every)
                       by the browser player, for spotting flicker and jumps

SVG output:
    data-frames="frame1,frame2,..."    Frame names on SVG root
    .frame-<name> { ... }              CSS classes with per-frame diffs
    Elements hidden in frame 0 get inline opacity="0"
    .kf-anim / .ai-shape transition rules are emitted by default (0.5s ease) so
    position/size/opacity tween smoothly; override via --stylesheet-css.

RESERVED IDENTIFIERS
--------------------
Cannot use as element names: left, right, top, bottom, x, y, width, height,
and the keywords: rect, circle, ellipse, polygon, line, path, text, icon,
callout, row, col, grid, stack, group, label, template, anchor, ... (a name
that is a keyword is a parse error that says so).

EXAMPLES
--------
Basic shapes:
    rect server [fill: steelblue, label: "Server"]
    circle node [fill: gold, size: 30]

Labels:
    rect b [label: "Cache"]                        // centred inside
    rect b [label: "Cache", align: end]            // inside, right-aligned
    rect b [label: "based_on", label_position: below, align: end]
    rect card [label: "<b>title</b><br>second line<br><small>note</small>"]
    rect card [width: 200, label: "a long sentence that wraps to the width"]

Captions that follow their subject:
    rect v1 [width: 300, height: 56]
    constrain v1.center_x = 400
    text "confirmed" cap [caption_of: v1, label_position: below, align: end]

Layout:
    row [gap: 20] {
        rect a [label: "A"]
        rect b [label: "B"]
    }

Connections:
    a -> b                    // default orthogonal routing
    b -> c [routing: curved]  // smooth curve
    a -> b -> c -> d          // chained connections

LINT
----
`--lint` reports likely defects and exits 1 when it finds an error (text cut
off, content leaving the canvas, an unknown modifier, a label overflowing its
box, a coordinate in motion); warnings alone exit 0. `--lint-strict` exits 1 on
any finding, so a build can gate on it. `--lint-categories` lists the
categories: overlap, containment, label, connection, alignment,
redundant-constant, reducible-bend, missing-anchor, contrast, steep-direct,
crowded-layout, over-constrained, label-overflow, unknown-modifier,
overridden-constraint, canvas-overflow, motion, motion-coordinate.

What the newer checks say and what to do:
    "... is cut off"            text runs past the image edge: wrap (max_width),
                                shorten, or move it inward
    "labels ... touch"          two texts on one line < 6px apart: add a gap or wrap
    "line L runs under X"       a through: line passes behind a box it does not
                                stop at: move the box or route the line
    "stations ... names need"   neighbouring stations too close for their names:
                                leave their x free (the line stretches), raise the
                                row gap, or shorten a name
    "two elements are both named"  (error) rename one; part b of instance a is a_b

Two `overlap` findings are worth knowing by name. Text that stops within 2px
of a visible edge is reported as "grazes" — at that distance the glyphs read as
struck through, and neither an overlap nor a label check sees it because the
boxes do not intersect. And a long thin element (20:1 or worse) that crosses
what it spans is NOT reported: a rule over a bar, or a pin on an axis, is the
picture working.

With keyframes, collision checks re-solve each frame, so a warning names the
frames the defect actually appears in.

Run --examples for more detailed patterns.
