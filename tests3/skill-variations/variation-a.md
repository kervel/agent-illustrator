# Variation A: Visual Shape First

## Layout Planning

For complex diagrams, think about the **visual shape** first:

1. **What shape should the whole diagram be?**
   - Cycle/loop → arrange items around a circle
   - Flow → left-to-right or top-to-bottom
   - Hierarchy → tree structure with levels
   - Comparison → side-by-side columns

2. **Use primitives creatively**
   - `circle` for cycle points, not just decoration
   - `path` with `arc_to` to draw actual curves
   - Position items to form the intended shape

3. **Write VISUAL plan first, then code**

Examples:
- "VISUAL: infinity loop → two circles, 4 items each, arrows follow curves"
- "VISUAL: flow diagram → single row with arrows"
- "VISUAL: hub-spoke → central circle, items radiating outward"
