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
text "content" [font_size: N]
```

### Layouts
```
row [gap: N] { children }
col [gap: N] { children }
group name { children }
```

### Connections
```
a -> b                    // orthogonal (right-angle) path
a -> b -> c               // chained connections
a -> b [routing: direct]  // straight diagonal line
a -> b [routing: curved]  // smooth curve
a <-> b                   // bidirectional
a -- b                    // undirected
```

### Modifiers
- `fill: color` - background color
- `stroke: color` - border color
- `label: "text"` - text inside shape
- `size: N` - width=height for square/circle
- `width: N`, `height: N` - explicit dimensions
- `gap: N` - spacing between children in layouts

### More Details
Run `agent-illustrator --examples` for annotated examples.
Run `agent-illustrator --grammar` for the full syntax reference.

---

## Rules

1. Modifiers go in `[brackets]` AFTER keyword, BEFORE `{`
2. Group names are identifiers: `group pipeline` NOT `group "pipeline"`
3. Forbidden as names: left, right, top, bottom, x, y, width, height

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
