AGENT ILLUSTRATOR EXAMPLES
==========================

EXAMPLE 1: Simple shapes in a row
---------------------------------
row {
    rect client [fill: steelblue]
    rect server [fill: green]
}
client -> server [label: "request"]

Creates two rectangles side-by-side with a labeled arrow.

EXAMPLE 2: Nested layout
------------------------
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
api -> service
service -> db

Vertical layout containing a title and a 2x2 grid of components.

EXAMPLE 3: Styling connections
------------------------------
rect a [size: 40]
rect b [size: 40]
rect c [size: 40]

row { a  b  c }

a -> b [stroke: green, stroke_width: 3]
b -> c [routing: direct, stroke: red]
a <-> c [stroke_dasharray: "4,2"]

Three shapes with different connection styles: thick green arrow,
diagonal red arrow, and dashed bidirectional arrow.

EXAMPLE 4: Chained connections
------------------------------
row { rect a  rect b  rect c  rect d }

a -> b -> c -> d [stroke: blue]

Chained connections - the modifier applies to the last segment only.
Each segment becomes a separate connection: a->b, b->c, c->d.

EXAMPLE 5: Constraints for alignment
------------------------------------
rect header [width: 200, height: 30]
rect body [width: 200, height: 100]
rect footer [width: 200, height: 30]

constrain header.bottom = body.top
constrain body.bottom = footer.top
constrain header.center_x = body.center_x
constrain body.center_x = footer.center_x

Three rectangles stacked vertically and centered.

EXAMPLE 6: Groups with labels
-----------------------------
group server {
    text "Web Server" [role: label, font_size: 14]
    col {
        rect nginx [size: 30, label: "nginx"]
        rect app [size: 30, label: "app"]
    }
}

A labeled group containing two stacked components.

EXAMPLE 7: Curved connections (loops)
-------------------------------------
row [gap: 20] {
    col [gap: 10] {
        rect plan [fill: lightblue, label: "Plan"]
        rect code [fill: lightblue, label: "Code"]
        rect build [fill: lightblue, label: "Build"]
        rect test [fill: lightblue, label: "Test"]
    }
}

plan -> code
code -> build
build -> test
test -> plan [routing: curved]

Use curved routing for loop-back connections or when paths would cross.

EXAMPLE 8: Custom shapes with paths
-----------------------------------
path "arrow" [fill: steelblue] {
    vertex a
    line_to b [x: 60, y: 15]
    line_to c [x: 30, y: 0]
    line_to d [x: 30, y: 10]
    line_to e [x: 0, y: 10]
    line_to f [x: 0, y: 20]
    line_to g [x: 30, y: 20]
    line_to h [x: 30, y: 30]
    close
}

A custom arrow shape. Paths let you define any polygon with
straight lines (line_to) or curves (arc_to with radius/bulge).
