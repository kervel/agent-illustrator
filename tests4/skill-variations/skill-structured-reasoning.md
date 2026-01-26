# Agent Illustrator Skill (Structured Reasoning)

Create diagrams with Agent Illustrator DSL. Output raw AIL code only (no markdown).

## MANDATORY: Reasoning First

Before writing any shapes, you MUST include block comments with your reasoning:

```
/* SPECIFICATION
   What: [one-line description of what diagram shows]
   Elements: [list the key elements/nodes]
   Relationships: [how elements connect]
*/

/* LAYOUT PLAN
   Visual shape: [what overall shape should viewer see?]
   Strategy: [how rows/cols will approximate this shape]
   Structure: [nested layout skeleton, e.g., "col { row { A B } row { C D } }"]
*/

/* ELEMENT MAPPING
   [element] → [AIL shape] [modifiers]
   [element] → [AIL shape] [modifiers]
   ...
*/

// Now write the AIL code:
[actual code here]
```

## Quick Reference

SHAPES: rect, circle, ellipse, text "content"
LAYOUTS: row { }, col { }, group { }
CONNECTIONS: a -> b, a <- b, a <-> b, a -- b
MODIFIERS: [fill: color, label: "text", size: N, gap: N, routing: direct]

## Layout Patterns

| Visual Intent | Layout Pattern |
|---------------|----------------|
| Cycle/loop | 2x2 grid with circular arrows |
| Infinity/8 | Two 2x2 grids side-by-side, diagonal cross-connections |
| Linear flow | Single row or col with arrows |
| Hub-spoke | Central shape in group, spokes in surrounding row/col |
| Tree | Nested cols with rows for siblings |

**Key insight**: Arrow paths suggest curves. Use `[routing: direct]` for diagonal lines.

## Rules

1. Identifiers: letters, numbers, underscore (e.g., `myShape`, `item_1`)
2. FORBIDDEN as names: left, right, top, bottom, x, y, width, height
3. Shape types required: always write `rect name` not just `name`

## Example: Infinity Loop

```
/* SPECIFICATION
   What: DevOps infinity loop
   Elements: Plan, Code, Build, Test, Release, Deploy, Operate, Monitor
   Relationships: Dev loop (left), Ops loop (right), cross-connections
*/

/* LAYOUT PLAN
   Visual shape: infinity symbol (∞)
   Strategy: two 2x2 grids side-by-side, arrows form loops within each
   Structure: row { col { row row } col { row row } }
*/

/* ELEMENT MAPPING
   Plan → rect [fill: blue, label]
   Code → rect [fill: blue, label]
   ... (all 8 elements as rects with labels)
*/

row {
  col {
    row { rect plan [fill: steelblue, label: "Plan"]  rect code [fill: steelblue, label: "Code"] }
    row { rect test [fill: steelblue, label: "Test"]  rect build [fill: steelblue, label: "Build"] }
  }
  col {
    row { rect release [fill: green, label: "Release"]  rect deploy [fill: green, label: "Deploy"] }
    row { rect monitor [fill: green, label: "Monitor"]  rect operate [fill: green, label: "Operate"] }
  }
}
plan -> code
code -> build
build -> test
test -> plan
release -> deploy
deploy -> operate
operate -> monitor
monitor -> release
test -> release [routing: direct]
monitor -> plan [routing: direct]
```
