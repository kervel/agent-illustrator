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
api -> service
service -> db

Nested layouts: col contains row contains two cols.
Connections cross layout boundaries automatically.

EXAMPLE 2: Feedback loops with cross-connections
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
