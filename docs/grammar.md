AGENT ILLUSTRATOR GRAMMAR
=========================

SHAPES
------
rect [name] [modifiers]      Rectangle (default 60x40)
circle [name] [modifiers]    Circle
ellipse [name] [modifiers]   Ellipse
text "content" [name] [mod]  Text element
path [name] [mod] { ... }    Custom shape with vertices/arcs
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
    fill: <color>           Fill color
    fill_opacity: <0..1>    Alpha for the fill only (keeps the fill color)
    stroke: <color>         Border color
    stroke_width: <number>  Border thickness
    stroke_opacity: <0..1>  Alpha for the stroke only
    opacity: <0..1>         Alpha for the whole element
    size: <number>          Width and height (square/circle)
    width: <number>         Explicit width
    height: <number>        Explicit height
    gap: <number>           Space between children (layouts)
    label: "text"           Add label to shape
    rotation: <degrees>     Rotate element (clockwise)
    class: <name>           Custom CSS class (for external styling)
    z_order: <number>       Render order for groups (higher = on top)
    routing: direct         Diagonal line (vs default orthogonal)
    routing: curved         Smooth curve (for loops, crossings)

COLORS
------
Hex:      #ff0000, #f00
Named:    red, blue, green, steelblue
Symbolic: foreground, background, accent, text
          foreground-1, accent-dark, text-light

CONSTRAINTS
-----------
constrain a.left = b.left              Align left edges
constrain a.center_x = b.center_x      Center horizontally
constrain a.top = b.bottom + 20        Position with offset
constrain a.width = 100                Fixed dimension
constrain a.center_x = midpoint(b, c)  Center between two elements
constrain bg contains a, b [padding: 10]   Auto-size container

Contains: container grows to surround listed elements with padding.
          Container width/height become flexible; position may shift.

Properties: left, right, top, bottom, center_x, center_y, width, height

TEMPLATES
---------
Inline templates:
    template "mytemplate" { ... }        Define reusable group (quoted name)
    mytemplate instance_name [params]    Instantiate template (unquoted)

File-based templates:
    template "icon" from "path/to/file.svg"     Import SVG file (embedded)
    template "photo" from "path/to/file.png"    Import raster image (referenced)

SVG files are embedded directly (content parsed, dimensions from viewBox).
Raster images (PNG, JPG, JPEG, GIF, WebP, BMP) are referenced by path.
The SVG viewer loads raster images at render time.

Raster images require explicit dimensions:
    photo avatar [width: 60, height: 60]

All file-based templates support modifiers like width, height, rotation:
    icon logo [width: 100, height: 100, rotation: 45]

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
}

Keyframes are cumulative: each frame builds on the previous frame's state.
Without keyframes, all elements are visible (backward compatible).
Named connections (a -> b as name) can be referenced in show/hide.
Referencing nonexistent elements is a hard error.

CLI flags:
    --frame N          Render single frame as static SVG (by index or name)
    --animate          Embed minimal JS for self-contained animated playback

SVG output:
    data-frames="frame1,frame2,..."    Frame names on SVG root
    .frame-<name> { ... }              CSS classes with per-frame diffs
    Elements hidden in frame 0 get inline opacity="0"

RESERVED IDENTIFIERS
--------------------
Cannot use as element names: left, right, top, bottom, x, y, width, height

EXAMPLES
--------
Basic shapes:
    rect server [fill: steelblue, label: "Server"]
    circle node [fill: gold, size: 30]

Layout:
    row [gap: 20] {
        rect a [label: "A"]
        rect b [label: "B"]
    }

Connections:
    a -> b                    // default orthogonal routing
    b -> c [routing: curved]  // smooth curve
    a -> b -> c -> d          // chained connections

Run --examples for more detailed patterns.
