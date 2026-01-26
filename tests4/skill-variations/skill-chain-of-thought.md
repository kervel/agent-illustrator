# Agent Illustrator Skill (Chain of Thought)

Output raw AIL code only (no markdown blocks).

## Process

Think step-by-step using line comments before each major decision:

```
// STEP 1: What am I drawing?
// [describe the diagram purpose]

// STEP 2: What's the overall visual shape?
// [circle, linear, tree, grid, infinity, hub-spoke, etc.]

// STEP 3: How do I approximate this with row/col?
// [describe your layout strategy]

// STEP 4: What are my elements?
// [list: element_name as shape_type]

// STEP 5: What connections exist?
// [list: from -> to relationships]

// Now implementing:
[actual AIL code]
```

## Reference

**Shapes**: `rect name`, `circle name`, `ellipse name`, `text "content"`
**Layouts**: `row { }`, `col { }`, `group name { }`
**Connections**: `->`, `<-`, `<->`, `--`
**Modifiers**: `[fill: color, label: "text", size: N, gap: N, routing: direct]`

**FORBIDDEN identifiers**: left, right, top, bottom, x, y, width, height

## Layout Strategies

| Visual Shape | Strategy |
|--------------|----------|
| Loop/Cycle | 2x2 grid: `col { row { A B } row { D C } }` with A→B→C→D→A |
| Infinity | Two 2x2 grids in a row, cross-connect with `[routing: direct]` |
| Linear | Single `row { }` or `col { }` |
| Hub-spoke | Center element with spokes in surrounding rows |
| Tree | Nested `col` for depth, `row` for siblings |

## Key Insight

AIL positions elements via layouts, not coordinates. Use arrows to suggest flow direction. Use `[routing: direct]` for diagonal lines that cross the grid.
