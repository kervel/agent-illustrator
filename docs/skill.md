# Agent Illustrator Skill - Design Process

Create diagrams with Agent Illustrator DSL. Output raw AIL code only.

## Design Process

Follow these phases IN ORDER. Write each phase as a block comment before proceeding.

### Phase 1: INTENT
```
/* INTENT
   What is being communicated?
   Who is the audience?
   What should they understand after seeing this?
*/
```

### Phase 2: GLOBAL DESIGN (Metaphor)
```
/* GLOBAL DESIGN
   What visual metaphor captures the essence?
   What shape should the whole diagram evoke?
   What are the major visual groupings?
*/
```

### Phase 3: LAYOUT PLAN
```
/* LAYOUT PLAN
   What goes where spatially?
   How are elements grouped?
   What is the reading flow (left-right, top-bottom, circular)?
   Sketch the structure: row/col nesting
*/
```

### Phase 4: AIL FEATURE MAPPING
```
/* AIL FEATURES
   Elements:
     [name] → [shape: rect|circle|ellipse] because [reason]

   Connections:
     [from → to]: [routing: default|direct|curved] because [reason]

   Visual encoding:
     [what] → [color/size/style] because [reason]
*/
```

### Phase 5: DETAIL NOTES
```
/* DETAILS
   Edge cases or special considerations
   Labels that need attention
   Any constraints or assumptions
*/
```

### Phase 6: IMPLEMENTATION
Write the AIL code based on your design.

---

## AIL Syntax Reference

### Shapes
```
rect name [fill: color, label: "text"]
circle name [fill: color, size: N]
ellipse name [fill: color, label: "text"]
text "content" name [font_size: N]
```

### Layouts
```
row name [gap: N] { children }
col name [gap: N] { children }
group name { children }
stack name { children }  // overlapping elements
```

### Connections
```
a -> b                    // orthogonal (right-angle) path
a -> b -> c               // chained connections
a -> b [routing: direct]  // straight diagonal line
a -> b [routing: curved]  // smooth curve
a <-> b                   // bidirectional
a -- b                    // undirected
a -> b [label: "text"]    // labeled connection
```

### Modifiers
- `fill: color` - background color
- `stroke: color` - border color
- `stroke_width: N` - border thickness
- `label: "text"` - text inside shape
- `size: N` - width=height for square/circle
- `width: N`, `height: N` - explicit dimensions
- `gap: N` - spacing between children in layouts
- `x: N`, `y: N` - explicit position (overrides layout)
- `opacity: 0.0-1.0` - transparency

### Explicit Positioning
Use `x` and `y` modifiers to override layout positions:
```
row container {
  rect a [width: 50, height: 50]
  rect b [width: 50, height: 50, x: 200, y: 100]  // overrides row position
  rect c [width: 50, height: 50]
}
```

### Constraints
Fine-tune positions after automatic layout:
```
constrain a.center_x = b.center_x        // align centers
constrain a.bottom = b.top - 10          // 10px gap
constrain a.center_x = midpoint(b, c)    // center between two elements
```

### Templates (Reusable Components)
```
template "icon" {
  col [gap: 4] {
    circle head [size: 20, fill: #f0f0f0]
    rect body [width: 30, height: 40, fill: #e0e0e0]
  }
}

// Instantiate:
icon alice
icon bob
```

### Custom Paths
For arbitrary shapes:
```
path arrow [fill: #333] {
  vertex a [x: 0, y: 10]
  line_to b [x: 20, y: 10]
  line_to c [x: 20, y: 0]
  line_to d [x: 30, y: 15]
  line_to e [x: 20, y: 30]
  line_to f [x: 20, y: 20]
  line_to g [x: 0, y: 20]
  close
}
```

Path commands:
- `vertex name [x: N, y: N]` - starting point
- `line_to name [x: N, y: N]` - straight line
- `arc_to name [x: N, y: N, radius: R]` - circular arc
- `curve_to name [x: N, y: N, via: point]` - quadratic curve
- `close` - close the path

---

## Rules

1. Modifiers go in `[brackets]` AFTER keyword, BEFORE `{`
2. Group names are identifiers: `group pipeline` NOT `group "pipeline"`
3. Forbidden as names: left, right, top, bottom, x, y, width, height
4. Text syntax: `text "content" name` (content BEFORE name)

---

## Shape Selection Principles

| Represents | Consider |
|------------|----------|
| Process, action, step | rect |
| Data, storage, document | ellipse |
| State, event, node | circle |

## Connection Routing Principles

| Situation | Consider |
|-----------|----------|
| Sequential flow | default (orthogonal) |
| Feedback, return, loop-back | curved |
| Crossing another path | curved |
| Shortcut, skip | direct |

---

## Examples

See `examples/` folder for non-trivial examples:
- `feedback-loops.ail` - Two interconnected iteration cycles with cross-connections
- `person.ail` - Reusable template with custom path shapes (hair, torso)
- `railway-topology.ail` - Complex multi-level diagram with constraints
- `railway-topology-templated.ail` - Same diagram using templates

Run `agent-illustrator --examples` for more annotated examples.
Run `agent-illustrator --grammar` for the full syntax reference.
