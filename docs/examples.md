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
evaluate -> spot -> tune -> assign [stroke: accent-dark, stroke_width: 3]
assign.top -> evaluate.top [stroke: accent-dark, stroke_width: 3, routing: curved, via: human_via]

// Agent flow (left to right)
task -> execute -> check -> result [stroke: secondary-dark, stroke_width: 3]
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
