# Variation B: Shape Vocabulary

## Layout Planning

For complex diagrams, match the **visual intent**:

| Intent | Approach |
|--------|----------|
| Cycle/loop | Items around circle(s), curved arrows |
| Flow | row/col with directional arrows |
| Infinity/8 | Two touching circles, items on each |
| Hub-spoke | Central node, radiating connections |
| Layers | Stacked rows with vertical arrows |

**Think beyond row/col** - use `path` with `arc_to` to draw curves, position `circle` shapes to form visual patterns.

Write "LAYOUT: [intent] → [approach]" before coding.
