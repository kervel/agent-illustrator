# Agent Illustrator Skill - Design Process

Create diagrams with Agent Illustrator DSL. Output raw AIL code only.

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
- **Phase 1 — Components**: Get each template/component right in isolation. Create a minimal test file per component and render it alone.
- **Phase 2 — Layout**: Position components in the overall diagram. Fix spacing, alignment, grouping.
- **Phase 3 — Connections & Labels**: Add connections, labels, annotations. Fix routing overlaps.

IMPORTANT: Do NOT use ImageMagick `convert` or `rsvg-convert` for PNG conversion — they don't support CSS variables. Chrome headless is required.

---

## AIL Syntax Reference

### Shapes
```
rect name [fill: color, label: "text"]
circle name [fill: color, size: N]
ellipse name [fill: color, label: "text"]
text "content" name [font_size: N]
```

### Layouts
```
row name [gap: N] { children }
col name [gap: N] { children }
group name { children }
stack name { children }  // overlapping elements
```

### Connections
```
a -> b                    // orthogonal (right-angle) path
a -> b -> c               // chained connections
a -> b [routing: direct]  // straight diagonal line
a -> b [routing: curved]  // smooth curve
a <-> b                   // bidirectional
a -- b                    // undirected
a -> b [label: "text"]    // labeled connection
a.top -> b.bottom         // connect via built-in anchors
a.feet -> b.feet          // connect via custom anchors (see Anchors)
a -> b [routing: curved, via: ctrl]  // curve through control point
```

### Modifiers
- `fill: color` - background color
- `stroke: color` - border color
- `stroke_width: N` - border thickness
- `label: "text"` - text inside shape
- `size: N` - width=height for square/circle
- `width: N`, `height: N` - explicit dimensions
- `gap: N` - spacing between children in layouts
- `x: N`, `y: N` - explicit position (overrides layout)
- `opacity: 0.0-1.0` - transparency

### Explicit Positioning
Use `x` and `y` modifiers to override layout positions:
```
row container {
  rect a [width: 50, height: 50]
  rect b [width: 50, height: 50, x: 200, y: 100]  // overrides row position
  rect c [width: 50, height: 50]
}
```

### Constraints
Fine-tune positions after automatic layout:
```
constrain a.center_x = b.center_x        // align centers
constrain a.bottom = b.top - 10          // 10px gap
constrain a.center_x = midpoint(b, c)    // center between two elements
```

### Templates (Reusable Components)
```
template "icon" {
  col [gap: 4] {
    circle head [size: 20, fill: #f0f0f0]
    rect body [width: 30, height: 40, fill: #e0e0e0]
  }
  // Custom anchors for semantic connection points
  anchor top [position: head.top - 4, direction: up]
  anchor bottom [position: body.bottom + 4, direction: down]
}

// Instantiate and connect via anchors:
icon alice
icon bob
alice.bottom -> bob.top [routing: curved]
```

### Custom Paths
For arbitrary shapes:
```
path arrow [fill: #333] {
  vertex a [x: 0, y: 10]
  line_to b [x: 20, y: 10]
  line_to c [x: 20, y: 0]
  line_to d [x: 30, y: 15]
  line_to e [x: 20, y: 30]
  line_to f [x: 20, y: 20]
  line_to g [x: 0, y: 20]
  close
}
```

Path commands:
- `vertex name [x: N, y: N]` - starting point
- `line_to name [x: N, y: N]` - straight line
- `arc_to name [x: N, y: N, radius: R]` - circular arc
- `curve_to name [x: N, y: N, via: point]` - quadratic curve
- `close` - close the path

---

## Rules

1. Modifiers go in `[brackets]` AFTER keyword, BEFORE `{`
2. Group names are identifiers: `group pipeline` NOT `group "pipeline"`
3. Forbidden as names: left, right, top, bottom, x, y, width, height
4. Text syntax: `text "content" name` (content BEFORE name)

---

## Shape Selection Principles

| Represents | Consider |
|------------|----------|
| Process, action, step | rect |
| Data, storage, document | ellipse |
| State, event, node | circle |

## Connection Routing Principles

| Situation | Consider |
|-----------|----------|
| Sequential flow | default (orthogonal) |
| Feedback, return, loop-back | curved |
| Crossing another path | curved |
| Shortcut, skip | direct |

---

## Template Best Practices

### Lead Extensions for Clean Connections
Add short rectangular "leads" extending from shapes to provide cleaner anchor points:
```
rect left_lead [width: 10, height: 2, fill: foreground-1, stroke: none]
constrain left_lead.right = body.left
constrain left_lead.center_y = body.center_y
anchor left_conn [position: left_lead.left, direction: left]
```

### Use Constraints, Not Hardcoded Coordinates
Express spatial relationships as constraints rather than computing coordinates manually:
```
// GOOD: constraint expresses intent
constrain label.center_x = body.center_x
constrain label.bottom = body.top - 8

// BAD: fragile manually-computed position
text "Label" label [x: 47, y: 12]
```

### Export Internal Elements Sparingly
Use `export` to expose internal template elements for external constraints:
```
template "component" {
    rect body [...]
    export body
}
component c1
constrain foo.left = c1_body.right + 10  // instance_element naming
```

### Label Placement
Avoid placing labels where connections will route. Use constraints to offset labels:
```
// Place label above the element, not on the connection path
text "API Server" api_label [font_size: 11, fill: text-1]
constrain api_label.center_x = api.center_x
constrain api_label.bottom = api.top - 6
```
For connection labels, keep them short (1-2 words). Long labels overlap with nearby elements.

### Template Composition
Templates can instantiate other templates. Internal elements stay with the template instance:
```
template "deployment" (name: "Deploy", pod_count: 3) {
    rect header [width: 180, height: 30, fill: accent-light, stroke: accent-dark, label: name]
    row pods [gap: 8] {
        pod p1
        pod p2
    }
    constrain pods.top = header.bottom + 8
    constrain pods.center_x = header.center_x
    anchor top_conn [position: header.top - 4, direction: up]
    anchor bottom_conn [position: pods.bottom + 4, direction: down]
}
```
When you instantiate `deployment`, the pods are part of the instance — they move with it when you constrain the deployment's position.

### Test Templates in Isolation
Before integrating a template into a large diagram, test it standalone:
```bash
echo 'template "mycomp" { ... } mycomp test1' > /tmp/test-comp.ail
agent-illustrator /tmp/test-comp.ail > /tmp/test-comp.svg
```
One component per test image. Issues are obvious in isolation, invisible in a crowded diagram.

---

## Layout Strategy

### DEFAULT: Constraint-based positioning

For any diagram with more than a few elements, use `constrain` to position elements explicitly. This gives you full control and avoids layout engine surprises:
```
// Declare elements with sizes
rect svc_a [width: 120, height: 50, fill: accent-light, stroke: accent-dark, label: "Service A"]
rect svc_b [width: 120, height: 50, fill: accent-light, stroke: accent-dark, label: "Service B"]
rect db [width: 120, height: 50, fill: secondary-light, stroke: secondary-dark, label: "Database"]

// Position with constraints — you control the layout
constrain svc_a.center_x = 150
constrain svc_a.center_y = 80
constrain svc_b.center_x = 350
constrain svc_b.center_y = 80
constrain db.center_x = midpoint(svc_a, svc_b)
constrain db.top = svc_a.bottom + 60

svc_a -> db [label: "queries"]
svc_b -> db [label: "queries"]
```

This is the recommended approach. See Example 6 in `--examples` for a full constraint-based diagram.

### ALTERNATIVE: Row/col nesting for simple diagrams

For simple diagrams with ≤8 elements and no cross-group connections, nested layouts work:
```
col main {
    row top { rect a  rect b }
    row bottom { rect c  rect d }
}
a -> c
b -> d
```

WARNING: Nested `row`/`col` breaks down with complex diagrams. If elements overlap or connections route badly, switch to constraint-based positioning.

### Sizing heuristic for constraint-based layouts

When positioning elements manually, use these rough sizes:
- Each component: ~120-150px wide, ~50px tall
- Horizontal gap between components: ~40-60px
- Vertical gap between rows: ~60-80px
- Background container: add ~60px padding on each side beyond the content
- Minimum readable text: font_size 10, minimum element width 60px

Example: 3 components in a row need ~3×140 + 2×50 = 520px width. With 60px padding on each side = 640px background width.

### Via-point routing

Use invisible `circle` elements as via-points for curved connections:
```
circle via_pt [size: 1, opacity: 0]
constrain via_pt.center_x = midpoint(source, target)
constrain via_pt.center_y = source.center_y - 40  // arc above

source.right_conn -> target.left_conn [routing: curved, via: via_pt]
```
Keep via-points CLOSE to the midpoint between source and target. Placing them too far away creates huge loops. A good distance is 30-60px above/below the connection line.

### Background containers (namespace boundaries, visual groups)

There is NO auto-sizing of containers. To create a visual boundary around elements, place a large rect behind them:
```
// Background rect — sized manually to surround content
rect prod_bg [width: 500, height: 350, fill: accent-light, stroke: accent-dark, stroke_width: 2, opacity: 0.3]

// Content elements positioned on top
rect svc_a [width: 120, height: 50, fill: accent-1, label: "Service A"]
rect svc_b [width: 120, height: 50, fill: accent-1, label: "Service B"]

// Position content inside the background
constrain svc_a.center_x = prod_bg.center_x - 80
constrain svc_a.center_y = prod_bg.center_y
constrain svc_b.center_x = prod_bg.center_x + 80
constrain svc_b.center_y = prod_bg.center_y

// Label the group
text "Production" prod_label [font_size: 14, fill: accent-dark]
constrain prod_label.center_x = prod_bg.center_x
constrain prod_label.top = prod_bg.top + 8
```
You must manually set the background `width` and `height` large enough to contain the content. After rendering, check that the boundary actually surrounds all elements.

---

## Colors

Prefer semantic palette colors over hex codes. Semantic colors adapt to different stylesheets.

**Color usage rules:**
- Use `*-light` colors for **fills/backgrounds**: `accent-light`, `secondary-light`
- Use `*-dark` colors for **strokes/borders**: `accent-dark`, `secondary-dark`
- Use `accent-1`, `accent-2` for **moderate fills** (not too light, not too dark)
- Use `foreground-1` for **primary lines and text** (dark color)
- Use `foreground-2`, `foreground-3` for **secondary/tertiary lines** (lighter)
- NEVER use `*-dark` as a fill — it renders as near-black, making labels illegible

```
// GOOD: light fill + dark stroke
rect a [fill: accent-light, stroke: accent-dark, stroke_width: 2, label: "Service"]

// GOOD: moderate fill
rect b [fill: accent-1, stroke: accent-dark, label: "Node"]

// BAD: dark fill makes label unreadable
rect c [fill: accent-dark, label: "Can't read this"]
```

Available semantic colors: `foreground-1`, `foreground-2`, `foreground-3`, `accent-1`, `accent-2`, `accent-light`, `accent-dark`, `secondary-light`, `secondary-dark`, `text-1`, `text-2`, `text-3`.

Run `agent-illustrator` with `--stylesheet` to use different color themes.

---

## Self-Assessment Checklist

After each render-check cycle, verify:

1. **No overlapping elements** — are any shapes or labels on top of each other?
2. **Connections don't cross labels** — are connection lines routing through text?
3. **Containers contain their content** — if using background rects, do they actually surround the elements?
4. **Readable labels** — can you read all text at the rendered size?
5. **No excessive whitespace** — are there large empty gaps between elements?
6. **Connections make sense** — do all connections go to/from the correct elements?

7. **Labels don't overlap** — are any text labels on top of other labels?
8. **Elements are reasonably sized** — can you distinguish individual elements? Minimum 60x35px.

If ANY of these fail, fix the issue and re-render. Do NOT declare done until all pass.
Common fix: if labels overlap, increase spacing between elements or move labels to a different side.

---

## What Does NOT Exist

These features do NOT exist in AIL. Do not attempt to use them:

- `contains` constraint — there is no auto-sizing of containers to fit children
- `padding` modifier — there is no padding on layouts or groups
- `label` modifier on `text` elements — text content goes BEFORE the name: `text "content" name`, not `text name [label: "content"]`
- `border` modifier — use `stroke` instead
- `align` modifier — use `constrain` for alignment
- `margin` modifier — use `gap` on parent layouts or `constrain` for spacing
- Percentage-based sizing — all sizes are in pixels

---

## Common Pitfalls

1. **Don't guess syntax** — fetch `--grammar` first. The syntax reference above is a summary.
2. **Don't skip visual verification** — always render to PNG and check. Use the self-assessment checklist above.
3. **Use exact color names** — `foreground-1` not `foreground`. Invalid colors cause render failures.
4. **Don't over-constrain** — constraining both edges AND size on the same axis creates conflicts. Use either `width` + one edge constraint, or two edge constraints without explicit size.
5. **Avoid reserved names** — `left`, `right`, `top`, `bottom`, `x`, `y`, `width`, `height` cannot be element or vertex names.
6. **Constraint coordinates are local** — when using `rotation`, property references (`.left`, `.top`) always use pre-rotation coordinates. You don't need to adjust constraints when changing rotation angle.
7. **Use `path` for complex shapes** — don't approximate shapes with multiple overlapping rectangles. Use `path` with `vertex`, `line_to`, `arc_to`, and `close`.
8. **Consistent visual style** — decide on stroke-only vs filled, consistent stroke widths, and proportional sizing before creating templates.
9. **Don't overclaim quality** — always render and visually check before saying the diagram is done. An AIL file that compiles is not necessarily a good diagram.

---

## More Information

Run `agent-illustrator --examples` for annotated examples including:
- Nested layouts with cross-connections
- Templates with anchors, via points, and S-curved connections
- Constraints for precise positioning
- Complex templates with internal constraints and lead extensions

Run `agent-illustrator --grammar` for the full syntax reference.
