# Layout & Render Pipeline Iteration Procedure

## Part 1: Iteration Procedure for Agents

### Overview

The iteration cycle consists of a **supervisor agent** that judges results and manages work, and **worker subagents** that implement changes. The supervisor never implements - it only evaluates and delegates.

### Supervisor Responsibilities

1. **Render and capture output** - Generate PNG from current SVG
2. **Compare against reference** - Visual comparison with target image
3. **Score the result** - Quantitative assessment (1-10 scale)
4. **Identify gaps** - List specific differences
5. **Prioritize improvements** - Add/update todo items
6. **Delegate to subagent** - Assign specific, scoped tasks

### Comparison Method

```bash
# 1. Render current state
cargo run --quiet -- examples/railway-topology.ail > /tmp/railway-topology.svg
convert /tmp/railway-topology.svg /tmp/railway-topology.png

# 2. View both images
# Use Read tool on /tmp/railway-topology.png
# Use Read tool on /tmp/reference.png (or fetch reference URL)
```

### Scoring Rubric

| Score | Criteria |
|-------|----------|
| 1-2   | Broken output, missing elements, layout completely wrong |
| 3-4   | Basic structure visible but major issues (wrong positions, missing connections) |
| 5-6   | Correct structure, elements present, but visual quality lacking |
| 7-8   | Good visual match, minor positioning or styling differences |
| 9-10  | Near-identical to reference, production ready |

### What to Evaluate

For each comparison, check these aspects in order:

1. **Structure** (most important)
   - Are all major sections present?
   - Is the hierarchy correct (parent-child relationships)?
   - Are connections between correct elements?

2. **Layout**
   - Vertical/horizontal flow correct?
   - Elements properly aligned?
   - Spacing reasonable?

3. **Shapes**
   - Correct shape types (rect, ellipse, line)?
   - Appropriate sizes?
   - Proper aspect ratios?

4. **Connections**
   - Arrow directions correct?
   - Routing sensible (not crossing elements)?
   - Labels positioned correctly?

5. **Styling**
   - Colors match?
   - Opacity/transparency correct?
   - Stroke widths appropriate?

6. **Advanced features** (lowest priority)
   - Curved paths (if reference has them)
   - Gradients or effects
   - Fine visual polish

### Delegation Rules

When delegating to a subagent:

1. **One concern per task** - Don't mix "fix routing" with "add new primitive"
2. **Provide context** - Include the specific problem and expected outcome
3. **Scope boundaries** - Specify which files can be modified
4. **Acceptance criteria** - Define how to verify the task is complete

Example delegation prompt:
```
Task: Fix vertical connection routing
Problem: Arrows between vertically stacked elements go diagonal instead of down-then-across
Files to modify: src/layout/routing.rs
Acceptance: Connections from micro->meso and meso->macro should use S-shaped paths that go down first
Verify by: Render railway-topology.ail and check arrow paths in SVG
```

### Iteration Loop

```
WHILE score < target_score:
    1. Render current output
    2. Compare with reference
    3. Score (1-10)
    4. IF score >= target: DONE
    5. Identify top 1-3 gaps
    6. Create/update todo items for gaps
    7. Select highest-priority incomplete todo
    8. Delegate to subagent with specific instructions
    9. Wait for subagent completion
    10. Run tests (cargo test)
    11. IF tests fail: delegate fix to subagent
    12. CONTINUE
```

### When to Stop

- Target score reached
- Remaining gaps require new DSL primitives (architectural change)
- Diminishing returns (3+ iterations with <0.5 score improvement)
- All feasible todos completed

---

## Part 2: Current Todo List

### Completed Items ✓

- [x] Add `line` shape primitive to DSL
- [x] Implement basic layout engine (row, column, grid, stack)
- [x] Implement SVG renderer with CSS classes
- [x] Add connection routing with arrow markers
- [x] Fix stack layout centering for overlay effects
- [x] Move group labels to left side
- [x] Fix connection fill (was black, now transparent)
- [x] Improve edge selection for vertical stacking
- [x] Implement S-shaped routing for misaligned vertical connections
- [x] Adjust default sizes for compact layout

### High Priority (Structural)

- [ ] **Add `path` primitive for curved lines**
  - Scope: lexer, grammar, AST, layout engine, renderer
  - Enables: Wavy/bezier tracks like in reference
  - Complexity: High (new primitive end-to-end)

- [ ] **Support size modifiers on shapes**
  - Example: `line track [width: 200]` or `ellipse op [size: 120x60]`
  - Scope: grammar, layout engine
  - Enables: Per-element size control without global config changes

### Medium Priority (Visual Quality)

- [ ] **Improve Meso ellipse overlap**
  - Currently: Ellipses side-by-side with small gap
  - Target: Ellipses should overlap ~20% like in reference
  - Scope: AIL file or add overlap parameter to row layout

- [ ] **Add "Line Section" labels to Macro level**
  - Reference shows diagonal lines with "Line Section" text
  - Scope: AIL file, possibly label positioning in renderer

- [ ] **Vertical arrow styling**
  - Current: S-shaped routing works but arrow heads point right
  - Target: Arrow heads should point down at the end
  - Scope: SVG marker orientation in renderer

### Low Priority (Polish)

- [ ] **Connection label positioning**
  - Currently: Labels appear at path midpoint
  - Target: Labels should be offset to not overlap the line
  - Scope: routing.rs label calculation

- [ ] **Add stroke style support**
  - Enable: `[stroke_style: dashed]` for different line styles
  - Scope: lexer, grammar, renderer

- [ ] **Grid layout improvements**
  - Current: Basic grid exists but untested
  - Target: Proper column/row spanning, auto-sizing

### Blocked (Requires Architecture Decision)

- [ ] **Curved/bezier path primitive**
  - Blocked by: Need to design path syntax
  - Options:
    1. `path points: [(0,0), (50,25), (100,0)]` - explicit coordinates
    2. `curve from: a to: b bend: 0.5` - semantic curve between elements
    3. `wave amplitude: 10 frequency: 3` - parametric wave pattern
  - Recommendation: Option 2 (semantic) aligns with project philosophy

- [ ] **Diagonal line support**
  - Blocked by: Lines currently only horizontal
  - Options:
    1. Add angle parameter: `line [angle: 45]`
    2. Add endpoint parameters: `line from: (0,0) to: (100,50)`
    3. Use connections between invisible anchor points
  - Recommendation: Option 1 for simplicity

### Test Coverage Needed

- [ ] Integration test: render railway-topology.ail and verify SVG structure
- [ ] Test: stack layout with different-sized children
- [ ] Test: connection routing between misaligned elements
- [ ] Test: label positioning doesn't overflow viewbox

---

## Appendix: Reference Comparison Checklist

When comparing output to reference, check each item:

```
[ ] Micro level has multiple parallel track lines
[ ] Micro label on left side
[ ] Arrow from Micro to Meso pointing down
[ ] "Aggregation" text on arrow
[ ] Meso level has track lines visible
[ ] Meso has two overlapping ellipses (OP1, OP2)
[ ] Ellipses are semi-transparent (tracks visible through)
[ ] Meso label on left side
[ ] Arrow from Meso to Macro pointing down
[ ] "Aggregation" text on arrow
[ ] Macro level shows simplified graph
[ ] Macro has OP1 and OP2 ellipses
[ ] Macro has "Line Section" connections
[ ] Macro label on left side
[ ] Overall vertical flow top-to-bottom
[ ] Colors: blue tracks, green ellipses
```

Current status: 12/16 items passing (75%)
