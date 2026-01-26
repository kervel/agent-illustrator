# Agent Illustrator Skill v2 - Explicit Syntax

Create diagrams with Agent Illustrator DSL. Output raw AIL code only (no markdown).

## Syntax Rules (CRITICAL)

```
SHAPES:     rect name [modifiers]
            circle name [modifiers]
            ellipse name [modifiers]
            text "quoted content" [modifiers]

LAYOUTS:    row [modifiers] { children }
            col [modifiers] { children }
            group name { children }     ← name is identifier, NOT quoted string

CONNECTIONS: a -> b [modifiers]
             a <- b [modifiers]
             a <-> b [modifiers]
             a -- b [modifiers]

MODIFIERS:  [fill: color, label: "text", size: N, gap: N, routing: direct]
            └─ Always in brackets, always BEFORE opening brace
```

### Common Mistakes to Avoid

```
WRONG: group "Pipeline" { }     → RIGHT: group pipeline { }
WRONG: col { gap: 20 }          → RIGHT: col [gap: 20] { }
WRONG: text spacer [label: ""]  → RIGHT: rect spacer [size: 10]
WRONG: LAYOUT: Infinity         → RIGHT: // LAYOUT: Infinity (as comment)
```

## Planning Process

Before coding, write your plan as a comment:

```
// PLAN: [visual shape] via [layout strategy]
// Elements: [list]
// Key connections: [cross-links with routing: direct]
```

Then implement the code.

## Layout Patterns

| Intent | Structure | Key Feature |
|--------|-----------|-------------|
| Cycle/loop | `col { row { A B } row { D C } }` | Arrows: A→B→C→D→A |
| Infinity | `row { col { row row } col { row row } }` | Cross-links: `[routing: direct]` |
| Linear | `row { A B C }` or `col { A B C }` | Sequential arrows |
| Hub-spoke | `col { row { top spokes } hub row { bottom spokes } }` | Bidirectional: `<->` |
| Tree | `col { parent row { children } }` | Parent→child arrows |

## FORBIDDEN Identifiers

Cannot use as shape/group names: `left`, `right`, `top`, `bottom`, `x`, `y`, `width`, `height`

## Complete Example

```
// PLAN: Hub-spoke via central row with hub, spokes above/below
// Elements: gateway (hub), user, order, payment (spokes)
// Key connections: hub <-> each spoke

col {
  row { rect user [label: "User"]  rect order [label: "Order"]  rect payment [label: "Payment"] }
  rect gateway [label: "API Gateway", fill: gold]
}

gateway <-> user
gateway <-> order
gateway <-> payment
```
