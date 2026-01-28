# Implementation Plan: MOSFET Driver Example with Skill Documentation Improvements

## Technical Context

| Aspect | Value |
|--------|-------|
| Language | Rust (stable, edition 2021) |
| Primary Output | AIL source code (`.ail` files) |
| Secondary Output | Updated skill documentation (SKILL.md) |
| Runtime | `agent-illustrator` CLI tool |
| Testing | Fresh agent instances (`claude --dangerously-skip-permissions`, `codex e`) |

### Dependencies

- `agent-illustrator` CLI (current main branch)
- Chrome headless (for PNG conversion)
- nix (for reproducible execution in isolated tests)

### Key Files

| File | Purpose |
|------|---------|
| `examples/mosfet-driver.ail` | The example schematic |
| `src/docs/skill.md` | Built-in skill documentation (via `--skill`) |
| `src/docs/examples.md` | Built-in examples (via `--examples`) |
| External SKILL.md | Kapernikov addendum skill file |

---

## Constitution Check

### Principle Alignment

| Principle | Status | Notes |
|-----------|--------|-------|
| 1. Semantic Over Geometric | ✅ Aligned | Electronic components as semantic templates, not coordinate drawings |
| 2. First-Attempt Correctness | ✅ Primary Goal | Documentation improvements aim to enable first-attempt success |
| 3. Explicit Over Implicit | ✅ Aligned | Skill documentation will make requirements explicit (mandatory grammar fetch) |
| 4. Fail Fast, Fail Clearly | ✅ Aligned | Testing methodology catches failures early |
| 5. Composability | ✅ Aligned | Component templates compose into circuits |
| 6. Do not reinvent the wheel | ✅ Aligned | Using existing template/anchor system |

### Quality Standards

| Standard | Compliance |
|----------|------------|
| Test coverage | N/A (documentation/example feature) |
| Clippy/fmt | Will apply to any Rust changes |
| Documentation | Primary deliverable |

---

## Tech Stack Compliance Report

### ✅ Approved Technologies
- Rust (core language)
- SVG output format
- Chrome headless (PNG conversion)
- nix (reproducible builds)

### No New Technologies Required
This feature creates AIL examples and documentation - no new dependencies.

---

## Phase 0: Research

### Research Tasks

1. **Template Parameter Syntax**
   - Verify current template parameter passing works as expected
   - Test with existing `railway-topology-templated.ail`

2. **Anchor System Capabilities**
   - Verify custom anchors with direction hints
   - Test cross-template anchor connections

3. **Agent Failure Modes**
   - Run initial test with fresh agent using current skill documentation
   - Document where agent fails and why

### Research Output → `research.md`

---

## Phase 1: Design

### Deliverables

1. **Component Templates Design** (`data-model.md`)
   - Resistor template specification
   - MOSFET template specification
   - LED template specification
   - GPIO pin template specification

2. **Circuit Layout Design** (`data-model.md`)
   - Power rail structure
   - Signal flow arrangement
   - Voltage domain visual encoding

3. **Skill Documentation Gap Analysis** (`research.md`)
   - Current documentation review
   - Agent test results analysis
   - Proposed improvements

---

## Phase 2: Implementation Tasks

### Task Group A: Create Electronic Component Templates

**A1: Resistor Template**
- Create template with left/right anchors
- Zig-zag or rectangle body
- Value label parameter
- Test: Instantiate twice with different values

**A2: N-Channel MOSFET Template**
- Create template with Gate/Drain/Source anchors
- Standard symbol shape
- Test: Verify anchor positions work for connections

**A3: LED Template**
- Create template with anode/cathode anchors
- Diode symbol with emission indicator
- Color parameter for LED color
- Test: Instantiate with different colors

**A4: GPIO Pin Template**
- Create template with output anchor
- Rectangle with label
- Pin name/number parameter
- Test: Verify clean labeling

### Task Group B: Create Circuit Schematic

**B1: Power Rails and Ground**
- Create 3.3V and 5V supply indicators
- Create ground reference
- Standard schematic positioning (VCC top, GND bottom)

**B2: Driver Section**
- GPIO → Gate resistor → MOSFET gate
- Pull-down resistor to ground
- Proper voltage domain indication

**B3: Load Section**
- MOSFET drain → LED → Current limiting resistor → 5V
- Clear connection routing

**B4: Full Circuit Integration**
- Combine all sections in `mosfet-driver.ail`
- Apply constraints for alignment
- Verify renders correctly

### Task Group C: Documentation Improvements

**C1: Baseline Agent Test**
- Run fresh agent with current skill documentation
- Document exact failures and gaps
- Capture session transcript

**C2: Update Skill Documentation**
- Add/strengthen mandatory grammar/examples fetch instruction
- Add explicit iteration workflow requirement
- Add template usage patterns section
- Add common pitfalls section

**C3: Validation Agent Test**
- Run fresh agent with updated skill documentation
- Measure improvement (iterations needed, error types)
- Document remaining issues

**C4: Iteration on Documentation**
- Address issues found in C3
- Re-test until agent demonstrates accurate self-assessment of result quality

### Task Group D: Finalization

**D1: Add Example to Built-in Examples**
- Add mosfet-driver.ail excerpt to `--examples` output
- Include annotations explaining template usage

**D2: Update Built-in Skill Documentation**
- Integrate improvements into `--skill` output
- Ensure consistency between built-in and external skill docs

**D3: Bug Documentation**
- Document any bugs discovered during development
- File issues or add to todo list as appropriate

---

## Task Dependencies

```
A1 ─┬─► B2 ─┬─► B4 ─► D1
A2 ─┤      │
A3 ─┼─► B3 ─┘
A4 ─┘

B1 ─────────► B4

C1 ─► C2 ─► C3 ─► C4 ─► D2

B4 + C4 ─► D3 (final bug documentation)
```

### Parallelizable
- A1, A2, A3, A4 (all templates independent)
- B1 (power rails independent of components)
- C1 (agent testing can start immediately)

### Sequential
- C1 → C2 → C3 → C4 (documentation iteration cycle)
- B2, B3 depend on templates (A1-A4)
- B4 depends on all circuit parts (B1, B2, B3)
- D1, D2 depend on circuit and documentation completion

---

## Acceptance Criteria (from Spec)

### SC-1: Example Validity
- [ ] `mosfet-driver.ail` compiles without errors
- [ ] SVG renders all components correctly
- [ ] Circuit connections are technically accurate

### SC-2: Template Reusability
- [ ] At least one template instantiated multiple times (resistor expected)
- [ ] Parameters correctly customize instances
- [ ] Anchors work across template instances

### SC-3: Documentation Effectiveness
- [ ] Fresh agent fetches grammar and examples before starting
- [ ] Fresh agent follows iterative workflow
- [ ] Fresh agent produces valid AIL on first attempt
- [ ] Fresh agent has accurate sense of when to stop (stops when result is good, not prematurely)

### SC-4: Bug Discovery
- [ ] All discovered bugs documented
- [ ] Critical bugs fixed or have workarounds
- [ ] Non-critical bugs logged for future work

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Template parameters don't work as expected | Low | High | Test with existing examples first |
| Agent still fails despite doc improvements | Medium | Medium | Multiple iteration cycles planned |
| Complex schematic reveals layout engine bugs | Medium | Medium | Simplify schematic if needed |
| Chrome headless not available in test env | Low | Low | Document alternative (manual browser) |

---

## Estimated Effort

| Phase | Effort |
|-------|--------|
| Phase 0: Research | Small |
| Phase 1: Design | Small |
| Task Group A (Templates) | Medium |
| Task Group B (Circuit) | Medium |
| Task Group C (Documentation) | Large (iterative) |
| Task Group D (Finalization) | Small |

**Primary effort**: Task Group C - the documentation improvement cycle requires multiple test-iterate rounds with fresh agents.

---

*Created: 2026-01-28*
*Feature: 009-mosfet-driver-example*
