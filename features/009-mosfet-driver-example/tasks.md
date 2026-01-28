# Tasks: MOSFET Driver Example with Skill Documentation Improvements

<!-- Tech Stack Validation: PASSED -->
<!-- Validated against: .specswarm/tech-stack.md -->
<!-- No prohibited technologies found -->
<!-- No new technologies introduced -->

## Overview

| Attribute | Value |
|-----------|-------|
| Feature | 009-mosfet-driver-example |
| Total Tasks | 17 |
| Parallel Opportunities | 6 tasks in Phase 2 |
| Primary Deliverables | `examples/mosfet-driver.ail`, updated skill documentation |

---

## Phase 1: Setup

No setup required - using existing `agent-illustrator` on main branch.

---

## Phase 2: Component Templates (Parallelizable)

**Goal**: Create all reusable electronic component templates in a single AIL file.

**File**: `examples/mosfet-driver.ail`

### T001: Create Resistor Template [P] ✅

**Description**: Create a resistor template with left/right anchors and configurable value parameter.

**File**: `examples/mosfet-driver.ail`

**Details**:
```ail
template "resistor" (value: "R") {
  rect body [width: 40, height: 16, fill: none, stroke: foreground, label: value]
  anchor left [position: body.left, direction: left]
  anchor right [position: body.right, direction: right]
}
```

**Acceptance**: Template instantiates with different values, anchors connect correctly.

---

### T002: Create N-Channel MOSFET Template [P] ✅

**Description**: Create MOSFET template with gate/drain/source anchors using path primitives.

**File**: `examples/mosfet-driver.ail`

**Details**:
- Simplified MOSFET symbol (vertical channel, gate line)
- Three semantic anchors: gate, drain, source
- Anchors have correct direction hints for routing

**Acceptance**: Can connect to gate from left, drain from top, source from bottom.

---

### T003: Create LED Template [P] ✅

**Description**: Create LED template with anode/cathode anchors and configurable color.

**File**: `examples/mosfet-driver.ail`

**Details**:
```ail
template "led" (color: accent-1) {
  // Triangle body (simplified diode symbol)
  // Emission indicator (small lines)
  anchor anode [position: ..., direction: up]
  anchor cathode [position: ..., direction: down]
}
```

**Acceptance**: Instantiates with different colors, vertical connection points work.

---

### T004: Create GPIO Pin Template [P] ✅

**Description**: Create GPIO pin template with output anchor and label parameter.

**File**: `examples/mosfet-driver.ail`

**Details**:
```ail
template "gpio_pin" (label: "GPIO") {
  rect body [width: 60, height: 24, fill: accent-light, stroke: accent-dark, label: label]
  anchor output [position: body.right, direction: right]
}
```

**Acceptance**: Clean labeling, output anchor connects to the right.

---

### T005: Create Power Rail Template [P] ✅

**Description**: Create power symbol template with voltage label and rail anchor.

**File**: `examples/mosfet-driver.ail`

**Details**:
- Horizontal line with voltage text above
- Single anchor pointing down (rail)
- Voltage parameter for label

**Acceptance**: Shows voltage label, anchor connects downward.

---

### T006: Create Ground Symbol Template [P] ✅

**Description**: Create ground symbol template with standard decreasing-width lines.

**File**: `examples/mosfet-driver.ail`

**Details**:
- Stack of three horizontal lines (decreasing width)
- Single anchor pointing up (gnd)

**Acceptance**: Standard ground appearance, anchor connects upward.

---

### ✅ CHECKPOINT: All Templates Complete

**Verify**: Each template compiles individually. Test with simple instantiation.

```bash
# Render templates-only version
cargo run --quiet -- examples/mosfet-driver.ail > /tmp/templates-test.svg
google-chrome --headless --screenshot=/tmp/templates-test.png --window-size=800,600 file:///tmp/templates-test.svg
# View /tmp/templates-test.png
```

---

## Phase 3: Circuit Assembly

**Goal**: Assemble complete MOSFET driver circuit using templates.

**Depends on**: Phase 2 completion

### T007: Create Circuit Layout Structure ✅

**Description**: Set up the overall circuit layout using row/col containers.

**File**: `examples/mosfet-driver.ail`

**Details**:
- Vertical column for overall circuit (power top, ground bottom)
- Nested rows for horizontal sections
- Instantiate all templates with appropriate parameters

**Layout**:
```
col circuit {
    power_5v [voltage: "+5V"]
    // load section (resistor + LED)
    // driver section (GPIO + gate resistor + MOSFET + pulldown)
    ground gnd
}
```

**Acceptance**: All components visible, roughly correct positions.

---

### T008: Wire Driver Section ✅

**Description**: Connect GPIO → gate resistor → MOSFET gate, and source → pulldown → ground.

**File**: `examples/mosfet-driver.ail`

**Details**:
```ail
gpio.output -> r_gate.left
r_gate.right -> q1.gate
q1.source -> r_pulldown.left
r_pulldown.right -> gnd.gnd
```

**Acceptance**: Connections render without overlap, routing is sensible.

---

### T009: Wire Load Section ✅

**Description**: Connect MOSFET drain → LED → current-limiting resistor → power rail.

**File**: `examples/mosfet-driver.ail`

**Details**:
```ail
q1.drain -> status_led.cathode
status_led.anode -> r_limit.right
r_limit.left -> power_5v.rail
```

**Acceptance**: Connections render correctly, LED oriented properly.

---

### T010: Apply Layout Constraints ✅

**Description**: Add constraints for proper alignment across sections.

**File**: `examples/mosfet-driver.ail`

**Details**:
- Align power rail and ground horizontally centered
- Align MOSFET with load section
- Ensure adequate spacing between voltage domains

**Acceptance**: Professional schematic appearance, no overlapping labels.

---

### T011: Render and Iterate on Layout ✅

**Description**: Render the complete circuit, identify visual issues, refine.

**Process**:
1. Render SVG: `cargo run --quiet -- examples/mosfet-driver.ail > /tmp/mosfet.svg`
2. Convert to PNG: `google-chrome --headless --screenshot=/tmp/mosfet.png ...`
3. View and identify issues
4. Fix issues in AIL
5. Repeat until acceptable

**Acceptance**: Circuit renders cleanly with:
- [x] No overlapping labels
- [x] Clear signal flow
- [x] Proper power/ground positioning
- [x] At least one template used multiple times (resistor - used 3x)

---

### ✅ CHECKPOINT: Circuit Complete

**Verify SC-1 (Example Validity)**:
- [x] `mosfet-driver.ail` compiles without errors
- [x] SVG renders all components correctly
- [x] Circuit is technically accurate

**Verify SC-2 (Template Reusability)**:
- [x] Resistor template used 3x (gate, pulldown, current-limit)
- [x] Parameters customize each instance (10kΩ, 10kΩ, 220Ω)
- [x] Anchors work across instances

---

## Phase 4: Baseline Agent Test

**Goal**: Establish baseline for agent success with current documentation.

### T012: Run Baseline Agent Test

**Description**: Launch fresh agent with current skill documentation and capture results.

**Process**:
1. Create empty test directory: `mkdir /tmp/agent-test-baseline && cd /tmp/agent-test-baseline`
2. Copy skill documentation: `cp /path/to/SKILL.md .`
3. Launch agent:
   ```bash
   claude --dangerously-skip-permissions
   # or
   codex e
   ```
4. Provide prompt: "Using the skill documentation, draw a MOSFET driver circuit with LED indicator that's 3.3V and 5V compatible"
5. Observe and document:
   - Did agent fetch `--grammar`?
   - Did agent fetch `--examples`?
   - Did agent iterate (render → check → refine)?
   - What syntax errors occurred?
   - Did agent stop at the right time? (too early? kept going unnecessarily? or good judgment?)
   - Quality of final result
6. Save session transcript to `features/009-mosfet-driver-example/baseline-test-transcript.md`

**Acceptance**: Documented baseline metrics for comparison.

---

## Phase 5: Documentation Improvements

**Goal**: Update skill documentation to address gaps found in baseline test.

**Depends on**: T012 (baseline test results)

### T013: Update Built-in Skill Documentation

**Description**: Modify `src/docs/skill.md` to include mandatory requirements.

**File**: `src/docs/skill.md`

**Changes**:
1. Add "BEFORE YOU START" section at the TOP:
   ```markdown
   ## BEFORE YOU START (MANDATORY)

   You MUST complete these steps before writing any AIL code:

   1. Fetch the grammar: `agent-illustrator --grammar`
   2. Fetch the examples: `agent-illustrator --examples`
   3. Understand: Multiple iterations are REQUIRED

   The syntax reference below is INCOMPLETE. The grammar has the full specification.
   ```

2. Add "Iteration Workflow" section:
   ```markdown
   ## Required Iteration Workflow

   Every diagram requires iteration:
   1. Write initial AIL
   2. Render: `agent-illustrator file.ail > out.svg`
   3. Convert: `google-chrome --headless --screenshot=out.png ...`
   4. CHECK the PNG visually
   5. Identify issues (overlaps, routing, spacing)
   6. Fix and repeat from step 2

   Expect 2-4 iterations for a good result.
   ```

3. Add "When to Use Templates" section (if not present)

4. Add "Common Pitfalls" section:
   ```markdown
   ## Common Pitfalls

   - DON'T guess syntax - fetch --grammar first
   - DON'T skip visual verification - always check the PNG
   - DON'T use raw coordinates for complex shapes - use templates
   - DON'T declare done without iteration
   ```

**Acceptance**: Documentation contains all required sections.

---

### T014: Update External Skill Documentation

**Description**: Ensure Kapernikov addendum SKILL.md is consistent with built-in changes.

**File**: `/home/kervel/projects/markdown-templates/.claude-plugin/skills/kapernikov-agent-illustrator/SKILL.md`

**Changes**:
- Verify REQUIRED sections match built-in
- Add any new sections (common pitfalls, templates)
- Ensure iteration workflow is prominently featured

**Acceptance**: Both skill documents are consistent.

---

## Phase 6: Validation Agent Test

**Goal**: Verify documentation improvements work.

**Depends on**: T013, T014

### T015: Run Validation Agent Test

**Description**: Test updated documentation with fresh agent.

**Process**:
1. Create new test directory: `mkdir /tmp/agent-test-v2 && cd /tmp/agent-test-v2`
2. Copy UPDATED skill documentation
3. Launch fresh agent (same as T012)
4. Provide same prompt
5. Document:
   - Did agent fetch grammar/examples? (Expected: YES)
   - Did agent iterate? (Expected: YES)
   - Did agent stop at the right time? (not too early with broken result, not endlessly)
   - Quality of final result (acceptable schematic)
   - Syntax errors (Target: None on first attempt)
6. Save transcript to `features/009-mosfet-driver-example/validation-test-transcript.md`

**Acceptance**:
- [ ] Agent fetched grammar and examples
- [ ] Agent followed iteration workflow
- [ ] Valid AIL on first attempt
- [ ] Agent stopped when result was actually good (accurate self-assessment)

---

### T016: Iterate on Documentation (if needed)

**Description**: If T015 shows issues, refine documentation and re-test.

**Process**:
1. Analyze T015 transcript for failure causes
2. Update documentation to address each cause
3. Re-run validation test
4. Repeat until agent demonstrates good judgment about when to stop

**Acceptance**: Agent produces acceptable result AND knows when to stop (not too early, not endlessly iterating).

---

## Phase 7: Finalization

**Goal**: Complete the feature with all deliverables.

**Depends on**: T011 (circuit), T016 (documentation)

### T017: Add Example to Built-in Examples

**Description**: Add mosfet-driver excerpt to `--examples` output.

**File**: `src/docs/examples.md`

**Changes**:
Add new example section:
```markdown
EXAMPLE N: Electronic Schematic with Templates
----------------------------------------------
// Reusable component templates
template "resistor" (value: "R") {
  rect body [width: 40, height: 16, fill: none, stroke: foreground, label: value]
  anchor left [position: body.left, direction: left]
  anchor right [position: body.right, direction: right]
}

// Instantiate multiple times with different parameters
resistor r1 [value: "10kΩ"]
resistor r2 [value: "220Ω"]

// Connect via semantic anchors
r1.right -> r2.left

Templates enable reusable components. Anchors provide semantic connection points.
Use parameters to customize each instance.
```

**Acceptance**: `agent-illustrator --examples` includes the new example.

---

### ✅ FINAL CHECKPOINT

**Verify All Success Criteria**:

**SC-1: Example Validity**
- [ ] `mosfet-driver.ail` compiles without errors
- [ ] SVG renders all components correctly
- [ ] Circuit is technically accurate

**SC-2: Template Reusability**
- [ ] Resistor template instantiated 3x
- [ ] Parameters correctly customize instances
- [ ] Anchors work across instances

**SC-3: Documentation Effectiveness**
- [ ] Fresh agent fetches grammar/examples
- [ ] Fresh agent follows iteration workflow
- [ ] Valid AIL on first attempt
- [ ] Agent stops at the right time (accurate self-assessment of when result is good)

**SC-4: Bug Discovery**
- [ ] All bugs documented in research.md or issues
- [ ] Critical bugs have workarounds
- [ ] Non-critical bugs logged

---

## Dependencies

```
T001 ─┬─► T007 ─► T008 ─► T010 ─► T011
T002 ─┤         T009 ─┘
T003 ─┤
T004 ─┤
T005 ─┤
T006 ─┘

T012 ─► T013 ─► T014 ─► T015 ─► T016

T011 + T016 ─► T017
```

---

## Parallel Execution Opportunities

### Phase 2 (Templates)
All 6 template tasks can run in parallel:
```
[T001] [T002] [T003] [T004] [T005] [T006]
```

### Phase 4-5 (Testing and Documentation)
Can run in parallel with Phase 3 (Circuit Assembly):
```
Circuit: T007 → T008 → T009 → T010 → T011
Testing: T012 → T013 → T014 → T015 → T016
```

---

## Summary

| Phase | Tasks | Parallelizable |
|-------|-------|----------------|
| Phase 2: Templates | T001-T006 | Yes (6 parallel) |
| Phase 3: Circuit | T007-T011 | Sequential |
| Phase 4: Baseline Test | T012 | Independent |
| Phase 5: Documentation | T013-T016 | Sequential |
| Phase 7: Finalization | T017 | After all |

**MVP Scope**: Complete T001-T011 for working example, T012-T016 for documentation improvements.

---

*Created: 2026-01-28*
*Feature: 009-mosfet-driver-example*
