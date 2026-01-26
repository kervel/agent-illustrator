# Agent Illustrator Skill v4 - Deep Reasoning

Create diagrams with Agent Illustrator DSL. Output raw AIL code only (no markdown).

## MANDATORY: Think Through Every Step

You MUST include detailed reasoning comments. The quality of your diagram depends on thorough planning.

```
/* STEP 1: UNDERSTAND THE REQUEST
   What is being asked for?
   What visual metaphor should readers see?
   What are the key elements and their relationships?
*/

/* STEP 2: CHOOSE LAYOUT STRATEGY
   What overall shape approximates the visual intent?
   - Cycle: 2x2 grid with circular arrows
   - Infinity: Two 2x2 grids side-by-side
   - Linear: Single row or col
   - Hub-spoke: Central element with surrounding elements
   - Tree: Nested cols with row siblings

   My choice: [describe why]
   Sketch: [nested structure like "row { col { row row } col { row row } }"]
*/

/* STEP 3: MAP ELEMENTS TO AIL
   For each element, specify:
   - Shape type (rect, circle, ellipse, text)
   - Identifier name (no quotes, no reserved words)
   - Key modifiers (fill, label, size)

   [element1] → rect name1 [fill: color, label: "text"]
   [element2] → ...
*/

/* STEP 4: PLAN CONNECTIONS
   List all arrows needed:
   - Sequential flow: a -> b
   - Bidirectional: a <-> b
   - Diagonal/cross: a -> b [routing: direct]

   Which connections need [routing: direct]?
*/

// STEP 5: IMPLEMENT
[actual AIL code here]
```

## Syntax Reference

```
rect name [fill: color, label: "text", size: N]
circle name [fill: color, size: N]
text "quoted content" [font_size: N]

row [gap: N] { children }
col [gap: N] { children }
group name { children }  ← identifier, NOT quoted string

a -> b [label: "text", routing: direct]
a <-> b
```

## Critical Rules

1. **Modifiers**: Always `[in brackets]`, always BEFORE `{` brace
2. **Group names**: Identifiers only: `group my_group` NOT `group "my group"`
3. **Forbidden identifiers**: left, right, top, bottom, x, y, width, height
4. **Diagonal lines**: Use `[routing: direct]` for any non-orthogonal connection

## Example: State Machine

```
/* STEP 1: UNDERSTAND
   Order state machine with happy path, cancel path, and failure path.
   Visual: Main flow horizontal, branches going down.
   Elements: Pending, Processing, Shipped, Delivered, Cancelled, Failed
*/

/* STEP 2: LAYOUT STRATEGY
   Main flow as top row, branch states below.
   Choice: row { main_col  branch_col }
   Sketch: row { col { pending processing shipped delivered } col { cancelled failed } }
*/

/* STEP 3: ELEMENT MAPPING
   Pending → rect pending [label: "Pending", fill: gray]
   Processing → rect processing [label: "Processing"]
   Shipped → rect shipped [label: "Shipped"]
   Delivered → rect delivered [label: "Delivered", fill: green]
   Cancelled → rect cancelled [label: "Cancelled", fill: red]
   Failed → rect failed [label: "Failed", fill: orange]
*/

/* STEP 4: CONNECTIONS
   Happy path: pending -> processing -> shipped -> delivered
   Cancel: pending -> cancelled [routing: direct]
   Failure: processing -> failed [routing: direct]
*/

row {
  col {
    rect pending [label: "Pending", fill: gray]
    rect processing [label: "Processing"]
    rect shipped [label: "Shipped"]
    rect delivered [label: "Delivered", fill: green]
  }
  col {
    rect cancelled [label: "Cancelled", fill: red]
    rect failed [label: "Failed", fill: orange]
  }
}

pending -> processing
processing -> shipped
shipped -> delivered
pending -> cancelled [routing: direct]
processing -> failed [routing: direct]
```
