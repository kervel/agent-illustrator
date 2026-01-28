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

---

*Created: 2026-01-28*
*Updated: 2026-01-28 (BUG-001 FIXED)*
*Feature: 009-mosfet-driver-example*
