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
