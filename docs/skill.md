# Agent Illustrator Skill

Create diagrams with Agent Illustrator.

## Sub-Skills

For specialized tasks, read the relevant sub-skill BEFORE starting:

- **`agent-illustrator --skill-animation`** — Keyframe animations with storyboarding,
  frame-by-frame verification, and mandatory evaluator rounds. Use when creating
  multi-frame animated sequences.
- **`agent-illustrator --skill-find-clipart`** — Finding and integrating open-source
  SVG clipart from the web. Use when diagrams need visual richness beyond basic shapes.
- **`agent-illustrator --skill-styling`** — CSS styling, color tokens, modern visual
  patterns (borderless cards, shadows, transitions), pattern/gradient fills
  (`fill: hatch(...)`, `fill: gradient(...)`), and dashed strokes. Use when diagrams need polish.

## Reach For These First

Check here before writing coordinates or a run of `constrain` lines.

| What you want | Use | Never |
|---------------|-----|-------|
| A box around existing elements | `constrain bg contains a, b [padding: 30]` | Hand-computed `width`/`height` |
| Anything on a lattice: matrix, table, calendar, aligned number columns | `grid` + `[at: [row, col]]`, cells as `g.cell(r, c)` | `constrain` lines with a hand-picked pitch |
| Text that must not jump when its wording changes | leave it auto-sized, add `align: start` | guessing a `width` for the longest wording |
| One caption across a walkthrough | `transform cap [label: "..."]` per keyframe | one text element per step |
| Children lined up on a container's cross axis | `align: center` / `end` on the container | per-child `constrain` |
| Checking every frame | `--frames-to-dir out/` | a shell loop over frame names |

`grid` is not only for heatmaps: it is the general alignment scaffold. Empty
cells are fine — place content by coordinate, address cells as `g.cell(r, c)`.

## When to Use AIL vs Raw SVG

AIL is for **diagrams** (boxes, arrows, architecture, flows). For **free-form icons or
illustrations** (a car, a robot, a logo), use **raw SVG** — the design phases and iteration
workflow below still apply, but output `<path d="...">` for direct coordinate control.

## BEFORE YOU START (MANDATORY)

Before writing any AIL code:

1. Run `agent-illustrator --grammar` — the full, authoritative syntax spec
2. Run `agent-illustrator --examples` — study ALL annotated examples
3. Plan your iteration: you will render, check, and refine multiple times

The reference below is a summary; do NOT start until you have read both.

Above 8 elements, position with constraints (Example 6 in --examples). Nested
row/col for a complex main layout produces overlaps and bad routing.

---

## Part 1: Process

### Design Phases

Follow these phases IN ORDER. Write each phase as a block comment before proceeding.

#### Phase 1: INTENT
```
/* INTENT
   What is being communicated?
   Who is the audience?
   What should they understand after seeing this?
*/
```

#### Phase 2: GLOBAL DESIGN (Metaphor)
```
/* GLOBAL DESIGN
   What visual metaphor captures the essence?
   What shape should the whole diagram evoke?
   What are the major visual groupings?
*/
```

#### Phase 3: LAYOUT PLAN
```
/* LAYOUT PLAN
   What goes where spatially?
   How are elements grouped?
   What is the reading flow (left-right, top-bottom, circular)?
   Sketch the structure: row/col nesting
*/
```

#### Phase 4: AIL FEATURE MAPPING
```
/* AIL FEATURES
   Elements:
     [name] → [shape: rect|circle|ellipse] because [reason]

   Connections (ALWAYS use explicit anchors like .bottom → .top):
     [from.anchor → to.anchor]: [routing: default|direct|curved] because [reason]

   Visual encoding:
     [what] → [color/size/style] because [reason]
*/
```

#### Phase 5: DETAIL NOTES
```
/* DETAILS
   Edge cases or special considerations
   Labels that need attention
   Any constraints or assumptions
*/
```

#### Phase 6: IMPLEMENTATION
Write the AIL code based on your design.

### Iteration Workflow

Every diagram requires multiple iterations. Follow this cycle:

1. Write initial AIL code
2. Render: `agent-illustrator file.ail > output.svg`
3. Convert to PNG: `google-chrome --headless --screenshot=output.png --window-size=2400,1800 file://$(pwd)/output.svg`
4. Check the PNG visually — look for overlaps, misalignment, routing issues
5. Fix issues in AIL code
6. Repeat from step 2

Text has no measurable size until it is rendered, so "does this label fit, does
that box clear the line above" is guesswork. `--debug` draws every element's box
— text included — which answers it in one render instead of three.

Use a phased approach for complex diagrams:
- **Phase 1 — Components**: Test each template/component in isolation.
- **Phase 2 — Layout**: Position components. Fix spacing, alignment, grouping.
- **Phase 3 — Connections & Labels**: Add connections and labels. Fix routing overlaps.

IMPORTANT: Do NOT use ImageMagick `convert` or `rsvg-convert` — they don't support CSS variables. Chrome headless is required.

### Self-Assessment Checklist

After each render, verify ALL of these. If any fail, fix and re-render:

1. Run `agent-illustrator --lint diagram.ail`. The warnings are there to prevent common mistakes, but can occasionally have false positives.
2. Visual check (render the svg to png)
2.1 No overlapping elements or labels
2.2 Connections don't route through text
2.3 Background containers surround their content
2.4 All labels readable at rendered size
2.5 No excessive whitespace gaps
2.6 All connections go to correct elements
2.7 Elements are at least 60x35px

### Adversarial Review (MANDATORY before declaring done)

Review only works unbiased, so a subagent is preferred. It is expensive — do it last.

**Option A — Subagent review (preferred):** Render to PNG, then spawn a subagent with the PNG path and the original prompt. It MUST Read the image itself — describing the diagram reintroduces your bias.

Use this exact prompt for the subagent:

> Review this rendered diagram against the original prompt. List defects as terse bullets:
> - One line per defect: `[CERTAIN/POSSIBLE] category: description`
> - Categories: overlap, containment, label, connection, alignment, color, spacing, missing-element
> - CERTAIN = clearly wrong. POSSIBLE = might be wrong but could be intentional.
> - Only report what you can actually see. Do not speculate about intent.
> - If no defects found, say "CLEAN".

Fix every CERTAIN defect; verify each POSSIBLE one before fixing or dismissing. Substantial feedback means a full new iteration.

**Option B — Self-review (ONLY if subagents unavailable):** Describe every element and its spatial relationships, then compare that to the actual image. Mismatches are bugs. Element by element, ask "what's wrong with THIS one?" — gaps, detached parts, overlapping labels, misaligned edges.

Not done until the review is clean.

---

## Part 2: Reference

### AIL Syntax Quick Reference

The `--grammar` output is the authoritative syntax reference. Below are the most commonly confused points.

#### Key syntax rules
1. Modifiers go in `[brackets]` AFTER keyword, BEFORE `{`
2. Group names are identifiers: `group pipeline` NOT `group "pipeline"`
3. Forbidden element names: `left`, `right`, `top`, `bottom`, `x`, `y`, `width`, `height`
4. Text syntax: `text "content" name` — content string BEFORE the name

#### Constraints
```
constrain a.center_x = b.center_x        // align centers
constrain a.bottom = b.top - 10          // 10px gap
constrain a.center_x = midpoint(b, c)    // center between two elements
```

#### Shape and routing selection

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
| Shortcut, skip | direct (only if nearly axis-aligned) |

**Direct routing warning:** `routing: direct` looks good when the connection is nearly horizontal or vertical. Steep diagonals (30-60°) look ugly when mixed with orthogonal/curved connections. Prefer orthogonal or curved for those.

#### Opacity modifiers

Three alpha modifiers, all `0..1` (clamped):
- `fill_opacity` — fades the fill only, **keeping the color**. Use for heatmaps: one hue, varying `fill_opacity` per cell, instead of pre-computing a blended hex per cell.
- `stroke_opacity` — fades the stroke only.
- `opacity` — fades the whole element.

```
rect cell [fill: secondary-1, fill_opacity: 0.7]
```

#### Callout (annotation pin)

`callout tag [label: "= subject", pointer: down]` draws a rounded pill with a
triangular pointer. `pointer:` is `up|down|left|right`. It auto-sizes to the
label and exposes a `tip` anchor at the pointer apex. Aim it at a target:

```
callout tag [label: "= subject", pointer: down, stroke: accent-1, fill: background-1]
constrain tag.tip = box.top - 4        // point-constraint: moves the callout
tag.tip -> box [routing: direct]       // or aim with a connection
```

Callouts are exempt from overlap lint — they are meant to sit over what they annotate.

#### Text alignment

`align: start|center|end` (or `left`/`center`/`right`): where text sits in its
own box, and which edge a container's children line up on. Default is `start`.

```
text "caption" cap [width: 400, align: start]    // left-aligned in a 400px box
rect r [width: 200, label: "title", align: left] // label inset from the border
col steps [gap: 10, align: center]               // children centred
row legend [gap: 8, align: end]                  // children share a bottom edge
```

An auto-sized text box is exactly as wide as its words, so a box positioned by
its centre moves when the wording changes. A caption rewritten by keyframes is
sized up front for the longest wording it ever takes, so leave it auto-sized —
only add `width` when you want a specific box, and the linter will tell you the
px if the text does not fit it.

#### Grid (any row/column alignment: matrix, table, heatmap, number columns)

`grid g [cols: 6, rows: 6, gap: 5, cell_width: 56, cell_height: 56]` lays a
regular lattice. Place children by coordinate with `[at: [row, col]]` (0-indexed);
unplaced children fill row-major; empty cells stay empty, so sparse or triangular
grids are trivial. Children without a size inherit the cell size.

Address any cell as `g.cell(row, col)` in `constrain`/connections — that is what
makes a grid an alignment scaffold rather than just a picture: draw a frame
around a block of cells, aim a callout at one, connect two. `col_labels: [...]`
/ `row_labels: [...]` add aligned text gutters. For a heatmap, use one hue with
per-cell `fill_opacity`. See `--examples`.

### Layout Strategy

#### DEFAULT: Constraint-based positioning

For >8 elements, wrap in a `group` and position everything with `constrain`. `group` uses column layout by default — constrain every element to override. Unconstrained elements fall back to column stacking. See Example 6 in `--examples`.

#### ALTERNATIVE: Row/col for simple diagrams (≤8 elements)

WARNING: Nested `row`/`col` breaks down with cross-group connections. Switch to constraints if elements overlap.

#### Sizing heuristics
- Components: ~120-150px wide, ~50px tall
- Gaps: ~40-60px horizontal, ~60-80px vertical
- Background containers: add ~60px padding beyond content on each side
- Minimum readable element: 60x35px, font_size 10

#### Via-point routing
Use invisible elements as curve control points:
```
circle via_pt [size: 1, opacity: 0]
constrain via_pt.center_x = midpoint(source, target)
constrain via_pt.center_y = source.center_y - 40
source.bottom -> target.top [routing: curved, via: via_pt]
```
Keep via-points 30-60px from the connection line. Too far = huge loops.

#### Fitting a box around elements (`contains`)

Any box that should wrap existing elements — a background zone, a frame, a
highlight — is sized by `contains`, never by hand:
```
rect bg [fill: accent-light, stroke: accent-dark, opacity: 0.3]
constrain bg contains svc1, svc2, svc3 [padding: 30]
```
The box grows to fit everything listed, plus the padding. Declare backgrounds
FIRST in a `group` so they render behind the foreground.

`contains` frees **both** dimensions, so it cannot draw a line: a `height: 3`
rule told to contain a row of cells comes back 52px tall. For a line, constrain
the two edges you care about and leave the height alone:

```
rect rule [height: 3, fill: foreground-1]
constrain rule.left = g.cell(1, 1).left
constrain rule.right = g.cell(1, 3).right
constrain rule.top = g.cell(1, 3).bottom + 4
```

### Template Best Practices

- **Lead extensions**: Add short rects extending from shape edges as anchor points. See Example 5 in `--examples`.
- **Constraints over coordinates**: Express spatial relationships as constraints, not hardcoded `x`/`y` values.
- **Export sparingly**: Use `export` to expose internal elements. Access as `instance_element` (e.g., `c1_body`).
- **Label placement**: Offset labels above/below elements with constraints to avoid connection overlap. Keep connection labels short (1-2 words).
- **Template composition**: Templates can instantiate other templates. Internal elements stay with the instance when constrained.
- **Test in isolation**: Before integrating a template, test it standalone in a minimal file. One component per test image.

### Colors

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

## Part 3: Gotchas

### What Does NOT Exist

Do not attempt to use these — they will waste iteration cycles:

- `padding`, `margin`, `border` modifiers — use `constrain`, `gap`, `stroke`
- Percentage-based sizing — all sizes are in pixels

`text name [label: "content"]` does not create text — use `text "content" name`.
(`label:` in a keyframe `transform` *does* rewrite an element's words.)

### Common Pitfalls

1. **Always specify connection anchors** — write `a.bottom -> b.top`, never `a -> b`. Explicit anchors produce much better routing.
2. **Don't guess syntax** — fetch `--grammar` first.
3. **Don't skip visual verification** — render to PNG and check every time.
4. **Use exact color names** — `foreground-1` not `foreground`.
5. **Don't over-constrain** — constraining both edges AND size on the same axis conflicts.
6. **Avoid reserved names** — `left`, `right`, `top`, `bottom`, `x`, `y`, `width`, `height`.
7. **Constraint coords are local** — property refs use pre-rotation coordinates.
8. **Path vertices are local** — coordinates start from (0,0). Use `constrain path.left = X` / `constrain path.top = Y` to position the path in the diagram.
9. **Use `path` for complex shapes** — not overlapping rectangles.
10. **Consistent visual style** — decide stroke-only vs filled before creating templates.
11. **Don't overclaim quality** — compile success ≠ good diagram. Always check visually.

---

## Keyframe Animations

Create animated sequences where elements appear/disappear across frames.

### Workflow
1. Layout all elements globally with constraints
2. Add `keyframe` blocks to control visibility per frame
3. Run `--frames-to-dir out/` to write every frame as a static SVG, then look at
   each one (`--list-frames` prints the names; `--frame <name>` renders one)
4. Use `--animate` for self-contained playback, or add external CSS transitions

### Syntax
```
a -> b as req_arrow [stroke: red]    // Named connection

keyframe "startup" {
    hide envelope, req_arrow          // Hide in this frame
}
keyframe "request" {
    show envelope, req_arrow          // Show in this frame
    transform server [rotation: 10]   // Per-frame overrides
    transform caption [label: "the server receives the request"]
}
```

### One caption for the whole walkthrough

`transform <element> [label: "..."]` rewrites an element's words for that frame
— a text element's content, or any other element's label:

```
text "the client sends a request" caption [align: start]
constrain caption.y = stage.bottom + 24

keyframe "send" { }
keyframe "receive" { transform caption [label: "the server receives it"] }
keyframe "reply"   { transform caption [label: "and answers"] }
```

One caption beats one text element per step. Leave it auto-sized — it is laid
out for the longest wording any frame gives it, so it never resizes and never
shifts; `align: start` pins the text to its left edge.

### Key Rules
- Keyframes are **cumulative**: each builds on the previous
- Without keyframes, everything is visible (backward compatible)
- References to nonexistent elements are hard errors
- The linter checks collisions per-frame: things that are never visible at the
  same time are never reported as overlapping, and a defect is reported once,
  naming the frames it occurs in
- Use `--frames-to-dir <dir>` to render every frame in one command

---

## More Information

Run `agent-illustrator --examples` for annotated examples.
Run `agent-illustrator --grammar` for the full syntax reference.
