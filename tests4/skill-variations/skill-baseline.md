# Agent Illustrator Skill (Baseline)

Create diagrams with Agent Illustrator DSL. Output raw AIL code only (no markdown).

## Quick Reference

SHAPES: rect, circle, ellipse, text "content"
LAYOUTS: row { }, col { }, group { }
CONNECTIONS: a -> b, a <- b, a <-> b, a -- b
MODIFIERS: [key: value, ...] after element name

## Core Patterns

```
row { circle start  rect process  circle end }
start -> process
process -> end
```

```
rect box [fill: steelblue, size: 50]
circle node [fill: red, size: 30]
```

```
col {
    text "Title" [font_size: 18]
    row {
        rect svc [label: "Service"]
        ellipse db [label: "DB"]
    }
}
```

## Layout Planning

Match visual intent to layout pattern:

| Intent | Layout Pattern |
|--------|----------------|
| Cycle/loop | 2x2 grid with circular arrows |
| Infinity/8 | Two 2x2 grids side-by-side, cross-connections |
| Flow | Single row or col with arrows |
| Hub-spoke | Central shape, surrounding row/col |

**Key insight**: Use arrow paths to suggest curves, not actual curved shapes.

## Rules

1. Names are identifiers: letters, numbers, underscore (e.g., `myShape`, `item_1`)
2. Reserved words cannot be names: left, right, top, bottom, x, y, width, height
3. Shapes in layouts auto-position; connections reference shapes by name

Write "LAYOUT: [intent] → [pattern]" then code.
