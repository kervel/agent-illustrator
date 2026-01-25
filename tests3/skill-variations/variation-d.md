# Variation D: Visual Intent + Correct Syntax

## Layout Planning

For complex diagrams, match the **visual intent**:

| Intent | Approach |
|--------|----------|
| Cycle/loop | path with arc_to forming circle, items in row/col |
| Flow | row/col with directional arrows |
| Infinity/8 | Two paths side by side, each curving back |
| Hub-spoke | Central shape, row/col of items, direct arrows |

**Drawing curves**: Use `path` with `vertex`, `line_to`, `arc_to`, `close` only.

```
path "semicircle" [fill: blue] {
    vertex a
    line_to b [x: 80, y: 0]
    arc_to c [x: 80, y: 80, radius: 40]
    line_to d [x: 0, y: 80]
    arc_to a [x: 0, y: 0, radius: 40]
}
```

Write "VISUAL: [intent] → [approach]" then code.
