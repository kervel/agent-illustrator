# Agent Illustrator Skill - Design Process

Create diagrams with Agent Illustrator DSL. Output raw AIL code only.

## When to Use AIL vs Raw SVG

AIL is for **diagrams** (boxes, arrows, architecture, flows). For **free-form icons or illustrations**
(a car, a robot, a logo), use **raw SVG** instead — the design phases and iteration workflow
below still apply, but output `<svg>` with `<path d="...">` for direct coordinate control.

## BEFORE YOU START (MANDATORY)

You MUST complete these steps before writing any AIL code:

1. Run `agent-illustrator --grammar` — read the FULL grammar specification
2. Run `agent-illustrator --examples` — study ALL annotated examples
3. Plan your iteration: you will render, check, and refine multiple times

The syntax reference below is a SUMMARY. The grammar has the complete specification.
Do NOT start writing AIL until you have read both --grammar and --examples.

For diagrams with more than 8 elements, you MUST use constraint-based positioning
(as shown in Example 6 of --examples). Do NOT use nested row/col for the main layout
of complex diagrams — it creates overlapping elements and bad connection routing.

## Design Process

Follow these phases IN ORDER. Write each phase as a block comment before proceeding.

### Phase 1: INTENT
```
/* INTENT
   What is being communicated?
   Who is the audience?
   What should they understand after seeing this?
*/
```

### Phase 2: GLOBAL DESIGN (Metaphor)
```
/* GLOBAL DESIGN
   What visual metaphor captures the essence?
   What shape should the whole diagram evoke?
   What are the major visual groupings?
*/
```

### Phase 3: LAYOUT PLAN
```
/* LAYOUT PLAN
   What goes where spatially?
   How are elements grouped?
   What is the reading flow (left-right, top-bottom, circular)?
   Sketch the structure: row/col nesting
*/
```

### Phase 4: AIL FEATURE MAPPING
```
/* AIL FEATURES
   Elements:
     [name] → [shape: rect|circle|ellipse] because [reason]

   Connections:
     [from → to]: [routing: default|direct|curved] because [reason]

   Visual encoding:
     [what] → [color/size/style] because [reason]
*/
```

### Phase 5: DETAIL NOTES
```
/* DETAILS
   Edge cases or special considerations
   Labels that need attention
   Any constraints or assumptions
*/
```

### Phase 6: IMPLEMENTATION
Write the AIL code based on your design.

---

## Required Iteration Workflow

Every diagram requires multiple iterations. Follow this cycle:

1. Write initial AIL code
2. Render: `agent-illustrator file.ail > output.svg`
3. Convert to PNG: `google-chrome --headless --screenshot=output.png --window-size=1200,900 file://$(pwd)/output.svg`
4. Check the PNG visually — look for overlaps, misalignment, routing issues
5. Fix issues in AIL code
6. Repeat from step 2

Use a phased approach for complex diagrams:
- **Phase 1 — Components**: Test each template/component in isolation.
- **Phase 2 — Layout**: Position components. Fix spacing, alignment, grouping.
- **Phase 3 — Connections & Labels**: Add connections and labels. Fix routing overlaps.

IMPORTANT: Do NOT use ImageMagick `convert` or `rsvg-convert` — they don't support CSS variables. Chrome headless is required.

---

## AIL Syntax Quick Reference

The `--grammar` output is the authoritative syntax reference. Below are the most commonly confused points.

### Key syntax rules
1. Modifiers go in `[brackets]` AFTER keyword, BEFORE `{`
2. Group names are identifiers: `group pipeline` NOT `group "pipeline"`
3. Forbidden element names: `left`, `right`, `top`, `bottom`, `x`, `y`, `width`, `height`
4. Text syntax: `text "content" name` — content string BEFORE the name

### Constraints
```
constrain a.center_x = b.center_x        // align centers
constrain a.bottom = b.top - 10          // 10px gap
constrain a.center_x = midpoint(b, c)    // center between two elements
```

### Shape and routing selection

| Represents | Shape |
|------------|-------|
| Process, action, step | rect |
| Data, storage, document | ellipse |
| State, event, node | circle |

| Situation | Routing |
|-----------|---------|
| Sequential flow | default (orthogonal) |
| Feedback, loop-back | curved |
| Crossing another path | curved |
| Shortcut, skip | direct |

---

## Template Best Practices

- **Lead extensions**: Add short rects extending from shape edges as anchor points. See Example 5 in `--examples`.
- **Constraints over coordinates**: Express spatial relationships as constraints, not hardcoded `x`/`y` values.
- **Export sparingly**: Use `export` to expose internal elements. Access as `instance_element` (e.g., `c1_body`).
- **Label placement**: Offset labels above/below elements with constraints to avoid connection overlap. Keep connection labels short (1-2 words).
- **Template composition**: Templates can instantiate other templates. Internal elements stay with the instance when constrained.
- **Test in isolation**: Before integrating a template, test it standalone in a minimal file. One component per test image.

---

## Layout Strategy

### DEFAULT: Constraint-based positioning

For >8 elements, wrap in a `group` and position everything with `constrain`. `group` uses column layout by default — constrain every element to override. Unconstrained elements fall back to column stacking. See Example 6 in `--examples`.

### ALTERNATIVE: Row/col for simple diagrams (≤8 elements)

WARNING: Nested `row`/`col` breaks down with cross-group connections. Switch to constraints if elements overlap.

### Sizing heuristics
- Components: ~120-150px wide, ~50px tall
- Gaps: ~40-60px horizontal, ~60-80px vertical
- Background containers: add ~60px padding beyond content on each side
- Minimum readable element: 60x35px, font_size 10

### Via-point routing
Use invisible elements as curve control points:
```
circle via_pt [size: 1, opacity: 0]
constrain via_pt.center_x = midpoint(source, target)
constrain via_pt.center_y = source.center_y - 40
source.anchor -> target.anchor [routing: curved, via: via_pt]
```
Keep via-points 30-60px from the connection line. Too far = huge loops.

### Background containers

No auto-sizing exists. Place a manually-sized rect behind content:
```
rect bg [width: 500, height: 350, fill: accent-light, stroke: accent-dark, opacity: 0.3]
// Position content inside bg with constraints
constrain svc.center_x = bg.center_x
constrain svc.center_y = bg.center_y
```
Declare backgrounds FIRST in a `group` so they render behind foreground elements. After rendering, verify the boundary surrounds all elements.

---

## Colors

Use semantic palette colors. NEVER use `*-dark` as a fill — it renders near-black.

| Purpose | Use |
|---------|-----|
| Fills/backgrounds | `accent-light`, `secondary-light` |
| Moderate fills | `accent-1`, `accent-2` |
| Strokes/borders | `accent-dark`, `secondary-dark` |
| Primary lines/text | `foreground-1` |
| Secondary lines | `foreground-2`, `foreground-3` |

Available: `foreground-1`, `foreground-2`, `foreground-3`, `accent-1`, `accent-2`, `accent-light`, `accent-dark`, `secondary-light`, `secondary-dark`, `text-1`, `text-2`, `text-3`.

---

## Self-Assessment Checklist

After each render, verify ALL of these. If any fail, fix and re-render:

1. No overlapping elements or labels
2. Connections don't route through text
3. Background containers surround their content
4. All labels readable at rendered size
5. No excessive whitespace gaps
6. All connections go to correct elements
7. Elements are at least 60x35px

### Adversarial Review (MANDATORY before declaring done)

**Option A — Subagent review (preferred):** Spawn a separate agent with the rendered PNG and the original prompt. Its only job: "List every visual defect. Be harsh." Fix every issue it finds, re-render, and re-submit until it returns clean.

**Option B — Self-review (if subagents unavailable):** Describe every element and its spatial relationships in text. Then compare that description to the actual image. Mismatches are bugs. Go element-by-element: for each one, ask "what's wrong with THIS one?" Look for gaps, detached parts, overlapping labels, misaligned edges.

Do NOT declare done until the adversarial review passes clean.

---

## What Does NOT Exist

Do not attempt to use these — they will waste iteration cycles:

- `contains` constraint — no auto-sizing of containers
- `padding`, `margin`, `border`, `align` modifiers — use `constrain`, `gap`, `stroke`
- `label` on `text` elements — use `text "content" name`, not `text name [label: "content"]`
- Percentage-based sizing — all sizes are in pixels

---

## Common Pitfalls

1. **Don't guess syntax** — fetch `--grammar` first.
2. **Don't skip visual verification** — render to PNG and check every time.
3. **Use exact color names** — `foreground-1` not `foreground`.
4. **Don't over-constrain** — constraining both edges AND size on the same axis conflicts.
5. **Avoid reserved names** — `left`, `right`, `top`, `bottom`, `x`, `y`, `width`, `height`.
6. **Constraint coords are local** — property refs use pre-rotation coordinates.
7. **Path vertices are local** — coordinates start from (0,0). Use `constrain path.left = X` / `constrain path.top = Y` to position the path in the diagram.
8. **Use `path` for complex shapes** — not overlapping rectangles.
9. **Consistent visual style** — decide stroke-only vs filled before creating templates.
10. **Don't overclaim quality** — compile success ≠ good diagram. Always check visually.

---

## More Information

Run `agent-illustrator --examples` for annotated examples.
Run `agent-illustrator --grammar` for the full syntax reference.
