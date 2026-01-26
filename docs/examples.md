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

EXAMPLE 9: Feedback loops with cross-connections
------------------------------------------------
row main [gap: 100] {
  col human_loop [gap: 18] {
    rect evaluate [width: 125, height: 45, fill: #e3f2fd, label: "Evaluate"]
    rect spot [width: 125, height: 45, fill: #e3f2fd, label: "Spot Patterns"]
    rect improve [width: 125, height: 45, fill: #bbdefb, label: "Tune Feedback"]
    rect assign [width: 125, height: 45, fill: #e3f2fd, label: "Assign Task"]
  }
  col agent_loop [gap: 12] {
    rect task [width: 110, height: 42, fill: #fff3e0, label: "Task"]
    rect execute [width: 110, height: 42, fill: #fff3e0, label: "Execute"]
    rect feedback [width: 110, height: 52, fill: #ffcc80, label: "Feedback"]
    rect result [width: 110, height: 42, fill: #fff3e0, label: "Result"]
  }
}

// Internal flows
evaluate -> spot -> improve -> assign
assign -> evaluate [routing: curved]
task -> execute -> feedback
feedback -> result [label: "pass"]
feedback -> task [label: "retry", routing: curved]

// Cross connections
assign -> task [label: "task"]
result -> evaluate [label: "result"]
improve -> feedback [label: "tunes"]

Two side-by-side iteration cycles with connections between them.
Use curved routing for loop-back arrows.

EXAMPLE 10: Reusable templates with paths
-----------------------------------------
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
}

row [gap: 24] {
  person alice
  person bob
}

Templates define reusable components. Use paths for custom shapes,
stack for overlapping elements, and constraints for alignment.

EXAMPLE 11: Explicit positioning with x/y
-----------------------------------------
row container {
  rect a [width: 50, height: 50, fill: #e3f2fd]
  rect b [width: 50, height: 50, fill: #bbdefb, x: 200, y: 100]
  rect c [width: 50, height: 50, fill: #90caf9]
}

The x and y modifiers override automatic layout positions.
Element 'b' is placed at absolute coordinates (200, 100)
while 'a' and 'c' follow normal row positioning.
