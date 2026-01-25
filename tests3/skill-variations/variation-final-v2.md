# Final Variation v2: Practical Layout Patterns

## Layout Planning

Match **visual intent** to layout pattern:

| Intent | Layout Pattern |
|--------|----------------|
| Cycle/loop | 2x2 grid with circular arrows |
| Infinity/8 | Two 2x2 grids side-by-side, cross-connections |
| Flow | Single row or col with arrows |
| Hub-spoke | Central shape, surrounding row/col |

**Key insight**: Use arrow paths to suggest curves, not actual curved shapes.

### Example: Infinity Loop with 2x2 Grids

```
row {
  col {
    row { rect a [fill: blue]  rect b [fill: blue] }
    row { rect d [fill: blue]  rect c [fill: blue] }
  }
  col {
    row { rect e [fill: green]  rect f [fill: green] }
    row { rect h [fill: green]  rect g [fill: green] }
  }
}
a -> b
b -> c
c -> d
d -> a
e -> f
f -> g
g -> h
h -> e
d -> e [routing: direct]
h -> a [routing: direct]
```

This creates two loops connected diagonally.

Write "LAYOUT: [intent] → [pattern]" then code.
