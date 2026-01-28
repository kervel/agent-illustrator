# Research: MOSFET Driver Example

## 1. Template System Capabilities

### Current State (Verified)

**Template Definition Syntax:**
```ail
template "name" (param: default_value) {
  // elements
  // constraints
  // anchors
  export element1, element2  // optional: expose internal elements
}
```

**Template Instantiation:**
```ail
name instance_name [param: value]
```

**Evidence**: `railway-topology-templated.ail` demonstrates:
- Parameterized templates with `(fill: accent-1)` syntax
- Parameter usage within template via `fill: fill`
- Template instantiation with parameter override: `tracks micro_tracks [fill: accent-1]`
- Export of internal elements for external constraints
- Cross-template constraint references: `meso_tracks_a1.center_x`

**Limitation Noted**: Exported elements use underscore naming convention (`instance_element`), not dot notation (`instance.element`) for external reference.

### Decision: Template Parameter Pattern

**Use**: Simple parameter syntax `(param: default)` for customization
**Rationale**: Proven pattern in existing examples
**Alternative Considered**: Property-based syntax `[param: value]` - but this is for modifiers, not template parameters

---

## 2. Anchor System Capabilities

### Current State (Verified)

**Custom Anchor Definition:**
```ail
anchor name [position: element.property +/- offset, direction: dir]
```

- `position`: Uses element properties (`top`, `bottom`, `left`, `right`, `center_x`, `center_y`)
- `direction`: Controls curve perpendicular entry (`up`, `down`, `left`, `right`)
- Offset supported: `head.top - 4`

**Anchor Usage:**
```ail
instance.anchor_name -> other_instance.anchor_name [routing: curved]
```

**Evidence**: `person.ail` demonstrates:
- Custom anchors with position and direction
- Cross-instance anchor connections
- Via points for curve control

**Built-in Anchors**: All shapes have `top`, `bottom`, `left`, `right`, `center`

### Decision: Anchor Strategy for Electronic Components

**Use**: Custom anchors for semantic connection points (Gate, Drain, Source, Anode, Cathode)
**Rationale**: Makes circuit connections readable (`mosfet.gate` vs `mosfet.left`)
**Alternative Considered**: Built-in anchors only - rejected because less semantic

---

## 3. Skill Documentation Analysis

### Current Documentation Structure

1. **External SKILL.md** (Kapernikov addendum):
   - Running instructions
   - REQUIRED sections for grammar/examples fetch
   - REQUIRED iterative workflow
   - Integration with Kapernikov templates

2. **Built-in `--skill`**:
   - Design process phases (INTENT, GLOBAL DESIGN, etc.)
   - Syntax reference
   - No explicit "fetch grammar first" instruction
   - No explicit iteration workflow

### Gap Analysis

| Aspect | External SKILL.md | Built-in --skill | Gap |
|--------|-------------------|------------------|-----|
| Grammar fetch instruction | ✅ REQUIRED section | ❌ Missing | Critical |
| Examples fetch instruction | ✅ REQUIRED section | ❌ Missing | Critical |
| Iterative workflow | ✅ REQUIRED section | ❌ Missing | Critical |
| Design phases | ❌ Not present | ✅ Good | Minor |
| Syntax reference | ❌ "Fetch grammar" | ✅ Good | None |
| Template patterns | ❌ Mentioned, not detailed | ✅ Basic example | Medium |
| Common pitfalls | ❌ Not present | ❌ Not present | Medium |

### Key Observation

**The built-in `--skill` output does NOT tell agents to fetch `--grammar` and `--examples` first.**

This is a critical gap: an agent using only `--skill` will:
1. See the syntax reference in `--skill`
2. Assume it's complete
3. Try to write AIL without seeing the full grammar or examples
4. Make syntax errors or miss features

### Decision: Documentation Improvement Strategy

**Primary Change**: Add explicit "BEFORE YOU START" section to `--skill` output requiring:
1. Fetch `--grammar` for complete syntax
2. Fetch `--examples` for patterns
3. Follow iterative workflow

**Rationale**: Agents tend to start working with whatever information they have. Making requirements explicit and FIRST in the document increases compliance.

**Alternative Considered**: Merge all content into one `--skill` output - rejected because it would be too long and grammar/examples serve different purposes.

---

## 4. Agent Failure Mode Hypothesis

Based on documentation analysis, predicted failure modes:

### Predicted Failures

1. **Syntax Errors**
   - Cause: Agent doesn't fetch grammar, guesses syntax
   - Expected: Wrong modifier names, incorrect connection syntax
   - Fix: Mandatory grammar fetch

2. **No Iteration**
   - Cause: Agent produces AIL and declares done without checking
   - Expected: Layout issues, overlapping labels
   - Fix: Mandatory iteration workflow

3. **No Template Usage**
   - Cause: Agent doesn't see template examples in `--skill`
   - Expected: Duplicated code instead of templates
   - Fix: Template patterns section with clear examples

4. **Wrong Path for Shapes**
   - Cause: Agent tries to draw complex shapes with coordinates
   - Expected: Broken or malformed path elements
   - Fix: "Don't do this" section explaining AIL is semantic, not geometric

### Test Plan

1. Run baseline test with current documentation (capture failures)
2. Update documentation addressing each failure mode
3. Re-test and measure improvement
4. Iterate until agent demonstrates accurate self-assessment (stops when result is good)

---

## 5. Electronic Schematic Design Notes

### MOSFET Driver Circuit (Reference)

```
                 +5V
                  │
                  R2 (current limiting)
                  │
                 LED
                  │
    GPIO ──R1──┬──D (Drain)
               │
               G (Gate)
               │
               S (Source)
               │
    GND ───R3──┴──────────────────── GND
         (pull-down)
```

### Components Needed

| Component | Symbol | Parameters | Anchors |
|-----------|--------|------------|---------|
| Resistor | Zig-zag or rectangle | value (e.g., "10kΩ") | left, right |
| N-MOSFET | Standard symbol | - | gate, drain, source |
| LED | Triangle + line | color | anode, cathode |
| GPIO Pin | Rectangle | label | output |
| Power Rail | Text + line | voltage | - |
| Ground | Standard symbol | - | - |

### Layout Strategy

- Vertical flow: VCC at top, GND at bottom
- Horizontal flow: Signal left-to-right (GPIO → MOSFET → Load)
- Use `col` for vertical stacking, `row` for horizontal grouping
- Constraints for alignment across groups

---

## 6. Decisions Summary

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Template parameter syntax | `(param: default)` | Proven in existing examples |
| Anchor naming | Semantic names (gate, drain, source) | Readability |
| Documentation fix location | Both `--skill` and external SKILL.md | Consistency |
| Primary doc change | "BEFORE YOU START" section | Agents read top-first |
| Test methodology | Fresh agent with `--dangerously-skip-permissions` | Real iteration capability |

---

## 7. Bugs Discovered During Implementation

### BUG-001: Constraints Inside Templates Fail for Certain Elements [FIXED]

**Severity**: Medium (workaround available) → **FIXED**

**Description**: When using `constrain` statements inside templates to position elements relative to other elements (especially paths), the constraint solver sometimes places elements at incorrect coordinates (often near origin).

**Observed**:
- `constrain cathode_bar.center_x = body.center_x` placed cathode_bar at x=90 when body was at x=247
- `constrain line2.center_x = line1.center_x` in ground template placed line2/line3 far from line1

**Root Cause**: Template children were being double-shifted:
1. When the template instance (e.g., `gnd`) was shifted by 175px, its children were also moved via `shift_element_and_children`
2. BUT the children (e.g., `gnd_line2`, `gnd_line3`) were ALSO in the external constraint solver separately
3. So they got shifted a SECOND time based on their own solver solution, moving them back towards the origin

**Fix Applied**: Modified `is_internal_constraint()` in `src/layout/engine.rs` to recognize that template-internal `constrain` statements should be treated as INTERNAL constraints (PASS 1) rather than external (PASS 2). The key insight is that template children share a common parent prefix (e.g., `gnd_line1` and `gnd_line2` both have prefix `gnd`).

**Code Change**: Added `elements_share_parent_prefix()` helper function that checks if two element IDs share a common underscore-delimited prefix. If both sides of a `UserDefined` constraint share a prefix, it's treated as internal.

**Important**: The prefix must be extracted using the FIRST underscore, not the last. Element names like `q1_gate_ext` have instance prefix `q1`, not `q1_gate`. Using `rfind('_')` incorrectly gave `q1_gate`, breaking the sibling detection.

**Regression Test Added**: `test_template_internal_constraints_centering` in `src/layout/engine.rs` verifies that template children stay properly centered when the template instance is moved.

### BUG-002: Rotation Modifier Doesn't Work on Template Instances

**Severity**: Medium (workaround available)

**Description**: The `rotation` modifier works on simple shapes but has no effect on template instances.

**Reproduction**:
```ail
template "resistor" { rect body [...] }
resistor r1 [value: "10k"]
resistor r2 [value: "20k", rotation: 90]  // No effect - r2 is still horizontal
```

**Workaround**: Create separate templates for different orientations, or adjust layout to avoid needing rotation.

**Status**: NOT FIXED - needs investigation

---

## 8. Lessons Learned (Skill Documentation Candidates)

These lessons are **general-purpose** (SC-5 compliant - not specific to electronic schematics):

### LL-1: Isolate Components When Debugging Templates

**Problem**: When debugging a template within a large diagram, issues get lost in the big picture.

**Solution**: Create a minimal test file with just the component being debugged:
```bash
cat > /tmp/test-component.ail << 'EOF'
template "my_component" { ... }
my_component instance1
EOF
cargo run -- /tmp/test-component.ail > test.svg
```

**Rationale**: Isolated testing makes issues immediately obvious and iteration faster.

### LL-2: Consistent Design Language Within Diagrams

**Problem**: Mixing different visual styles creates unprofessional-looking diagrams.

**Solution**: Before creating templates, decide on a consistent visual language:
- Stroke-only vs filled shapes (pick one, stick to it)
- Consistent stroke widths (e.g., 2px for primary, 1.5px for secondary)
- Proportional sizing between related components
- Consistent use of color semantics

### LL-3: Use `path` for Complex Shapes

**Problem**: Approximating shapes with multiple rectangles is fragile and often misaligned.

**Solution**: Use the `path` element with vertices for complex shapes:
```ail
path triangle [fill: none, stroke: foreground-1, stroke_width: 2] {
    vertex tl [x: 0, y: 0]
    line_to tr [x: 20, y: 0]
    line_to tip [x: 10, y: 18]
    close
}
```

**Note**: Avoid reserved keywords as vertex names (`left`, `right`, `top`, `bottom`, etc.). Use abbreviations like `tl`, `tr`, `bl`, `br`.

### LL-3b: Test One Component Per Image

**Problem**: Testing multiple components in one "all components" image leads to the same trap as debugging the full diagram - issues get lost.

**Solution**: Create separate test files for each component:
```bash
# Test resistor
echo 'template "resistor" {...} resistor r1' > /tmp/test-resistor.ail
# Test LED
echo 'template "led" {...} led d1' > /tmp/test-led.ail
# Test each in isolation
```

**Rationale**: One component per image means one thing to focus on.

### LL-4: Color References Must Be Exact

**Problem**: Using undefined colors like `foreground` (instead of `foreground-1`) causes silent rendering failures (now fixed with validation).

**Solution**: Always use exact color names from the palette. Run `agent-illustrator` without arguments to see error if color is invalid:
```
Error: Unknown color 'foreground'. Did you mean one of: foreground-1, foreground-2?
```

### LL-5: Lead Extensions Improve Connections

**Problem**: Anchors placed directly on shape edges can cause awkward connection routing.

**Solution**: Add short "lead" rectangles extending from shapes to provide cleaner connection points:
```ail
rect left_lead [width: 10, height: 2, fill: foreground-1]
constrain left_lead.right = body.left
anchor left_conn [position: left_lead.left, direction: left]
```

### LL-6: Constraint Coordinates Are Always Local

**Problem**: When a template has rotation, it's unclear whether property references like `.left` refer to pre-rotation or post-rotation coordinates.

**Design Decision**: Property references ALWAYS use local (pre-rotation) coordinates.

**Rationale**:
- If you rotate 44°, your constraints still work
- If you then change to 46°, your constraints still work
- You don't have to rewrite constraints when changing rotation angle
- Rotation is a presentation concern, not a constraint concern

**Example**:
```ail
resistor r2 [rotation: 90]
constrain foo.left = r2_body.right + 10
```

Here `r2_body.right` refers to the right edge in local (pre-rotation) space, even though visually the element is rotated.

### LL-7: External Constraints Can Reference Exported Elements

**Problem**: Template elements are "internal" but sometimes external code needs to reference them.

**Pattern**: Use `export` to expose internal elements, then reference them with the `instance_element` naming convention:

```ail
template "component" {
    rect body [...]
    export body
}
component c1
constrain foo.left = c1_body.right + 10  // c1_body = instance name + underscore + exported name
```

**Caution**: This couples external layout to internal template structure. Use sparingly.

### LL-8: Template Rotation Requires Solver Separation

**Problem**: Template rotation (Feature 006) was designed as render-only, but this breaks:
- Anchor positions (point to pre-rotation coordinates)
- Via points in curves (reference pre-rotation element centers)
- External constraints (violated after visual rotation)

**Solution**: Feature 010 introduces local/global solver separation:
1. Local solver: template-internal constraints
2. Rotation applied to local results
3. Global solver: external constraints with rotated coordinates

**Workaround Until 010**: Avoid rotation on templates with external connections. Create separate templates for different orientations.

### LL-9: Local-Then-Global Optimization Workflow

**Problem**: When optimizing a complex diagram, trying to fix everything at once leads to confusion and missed issues.

**Solution**: Use a two-phase optimization approach:

1. **Local Optimization Phase**: Optimize each component/template in isolation
   - Test each template with a minimal single-instance file
   - Fix internal layout, proportions, and anchor positions
   - Verify the component looks correct standalone

2. **Global Optimization Phase**: Optimize the overall diagram layout
   - Position components relative to each other
   - Adjust spacing and alignment
   - Fix connection routing issues

**Rationale**: Local issues are easier to spot and fix in isolation. Once all components are individually correct, global layout problems become clear.

### LL-10: Don't Work Around Bugs - Fix Them

**Problem**: When encountering a bug, the temptation is to work around it (e.g., create horizontal and vertical resistor templates instead of using rotation).

**Principle**: Workarounds accumulate technical debt and mask design issues. If a feature should work but doesn't, investigate and fix the root cause.

**Example**: BUG-002 (rotation on templates) led to discovering that the entire solver architecture needs refactoring (Feature 010). A workaround would have hidden this architectural issue.

---

*Created: 2026-01-28*
*Updated: 2026-01-28 (added Lessons Learned LL-6 through LL-9 from solver discussion)*
*Feature: 009-mosfet-driver-example*
