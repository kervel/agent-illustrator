AGENT ILLUSTRATOR GRAMMAR
=========================

SHAPES
------
rect [name] [modifiers]      Rectangle (default 60x40)
circle [name] [modifiers]    Circle
ellipse [name] [modifiers]   Ellipse
text "content" [name] [mod]  Text element
path [name] [mod] { ... }    Custom shape with vertices/arcs

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

CONNECTIONS
-----------
a -> b [mod]           Directed arrow from a to b
a -> b -> c [mod]      Chained connections (modifiers apply to last segment)
a <- b [mod]           Directed arrow from b to a
a <-> b [mod]          Bidirectional arrow
a -- b [mod]           Undirected line
a.anchor -> b.anchor   Connect via custom anchors (see ANCHORS)

Connection modifiers:
    routing: orthogonal     Right-angle path (default)
    routing: direct         Straight diagonal line
    routing: curved         Smooth cubic Bezier curve
    via: element            Route curve through element's center
    label: "text"           Add label (at midpoint or curve apex)

STYLE MODIFIERS
---------------
Modifiers go in brackets after the element name:
    rect mybox [fill: blue, stroke: #333, stroke_width: 2]

Common modifiers:
    fill: <color>           Fill color
    stroke: <color>         Border color
    stroke_width: <number>  Border thickness
    size: <number>          Width and height (square/circle)
    width: <number>         Explicit width
    height: <number>        Explicit height
    gap: <number>           Space between children (layouts)
    label: "text"           Add label to shape
    rotation: <degrees>     Rotate element (clockwise)
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
template "mytemplate" { ... }        Define reusable group (quoted name)
mytemplate instance_name [params]    Instantiate template (unquoted)

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
