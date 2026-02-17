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
    // Background rects (drawn first = behind everything else)
    rect prod_bg [width: 550, height: 200, fill: accent-light, stroke: accent-dark, stroke_width: 2, opacity: 0.25]
    text "Production Zone" prod_label [font_size: 14, fill: accent-dark]
    rect data_bg [width: 550, height: 120, fill: secondary-light, stroke: secondary-dark, stroke_width: 2, opacity: 0.25]
    text "Data Layer" data_label [font_size: 14, fill: secondary-dark]

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
constrain prod_label.center_x = 300
constrain prod_label.center_y = 80

constrain data_bg.center_x = 300
constrain data_bg.center_y = 370
constrain data_label.center_x = 300
constrain data_label.center_y = 320

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
