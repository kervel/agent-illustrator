---
parent_branch: main
feature_number: 009
status: In Progress
created_at: 2026-01-28T10:00:00+01:00
---

# Feature: MOSFET Driver Example with Skill Documentation Improvements

## Overview

Create a non-trivial electronic schematic example (MOSFET driver with LED status indicator, 3.3V/5V tolerant) that serves dual purposes:

1. **Demonstrate complex template usage** - Reusable electronic component templates (resistor, MOSFET, LED, GPIO pin) that can be instantiated multiple times with different parameters
2. **Stress-test and improve the skill documentation** - Use the process of building this example to identify bugs in the tool and gaps in the skill instructions that cause agents to produce suboptimal results

The ultimate goal is to improve both the tool functionality and the skill documentation such that a coding agent, when given a similar electronic schematic task using the skill, produces satisfactory results with minimal human intervention.

## User Scenarios

### Scenario 1: Agent Creates Electronic Schematic from Scratch

**Actor**: AI coding agent using the skill documentation

**Flow**:
1. Agent receives request: "Draw a MOSFET driver circuit with LED indicator that's 3.3V and 5V compatible"
2. Agent reads the skill documentation
3. Agent fetches grammar and examples from `--grammar` and `--examples`
4. Agent creates initial AIL code with electronic component templates
5. Agent iterates on the design (render → check → refine) until satisfactory
6. Result: Correct, readable schematic on first few attempts

**Expected Outcome**: The skill documentation guides the agent to:
- Mandatory grammar/examples fetch before starting
- Use of templates for reusable components
- Iterative refinement workflow
- Proper constraint usage for circuit layout

### Scenario 2: Automated Skill Documentation Testing

**Actor**: Fresh AI agent instance (Claude or Codex) with tool access

**Flow**:
1. Launch fresh agent in isolated environment with tool permissions:
   - `claude --dangerously-skip-permissions` in empty directory, or
   - `codex e` session
2. Provide agent with:
   - The skill documentation (SKILL.md)
   - The test prompt: "Draw a MOSFET driver circuit with LED indicator that's 3.3V and 5V compatible"
3. Agent works autonomously: reads skill → fetches grammar/examples → creates AIL → iterates
4. Human observes the session transcript for:
   - Did agent fetch grammar and examples before starting?
   - Did agent follow iterative workflow (render → check → refine)?
   - How many iterations to acceptable result?
   - What errors did agent make and why?
5. Document gaps and update skill documentation
6. Repeat test with fresh agent to validate improvements

**Expected Outcome**:
- Documented list of skill documentation gaps with fixes
- Measurable improvement in agent success rate across test runs
- Final skill documentation that reliably guides agents to success

## Functional Requirements

### FR-1: Electronic Component Templates

The example shall include reusable templates for:

- **Resistor template**: Horizontal zig-zag or rectangle symbol with value label, configurable resistance value
- **N-channel MOSFET template**: Gate/Drain/Source terminals with standard symbol, terminal anchors for connections
- **LED template**: Diode symbol with light emission indicator, configurable color
- **GPIO pin template**: Rectangle with pin number/name, input/output indicator

Each template shall:
- Accept parameters for customization (value, color, label)
- Provide named anchors for connection points
- Use semantic color palette references where appropriate

### FR-2: Circuit Topology

The example schematic shall demonstrate:

- **Power rails**: 3.3V and 5V supply lines with ground
- **MOSFET driver section**: GPIO → resistor → MOSFET gate, with pull-down resistor
- **Load section**: MOSFET drain → LED → current limiting resistor → power supply
- **Voltage tolerance indication**: Visual or textual indication of which parts are 3.3V vs 5V tolerant

Circuit connections shall use:
- Orthogonal routing for power rails
- Direct routing for signal paths where appropriate
- Clear labeling of voltage domains

### FR-3: Layout Quality

The rendered schematic shall exhibit:

- Standard schematic flow (power at top, ground at bottom, signal flow left-to-right)
- No overlapping labels or components
- Adequate spacing between components
- Clear distinction between power, ground, and signal connections

### FR-4: Skill Documentation Improvements

Based on findings during development, the skill documentation shall be updated to include:

- **Mandatory fetch instruction**: Explicit requirement to run `--grammar` and `--examples` before creating any diagram (not just mentioned as available)
- **Iterative workflow requirement**: Clear instruction that multiple iterations are expected and how to perform them
- **Template usage patterns**: When and how to use templates for reusable components
- **Common pitfalls**: Known errors agents make and how to avoid them

## Success Criteria

### SC-1: Example Validity

- The MOSFET driver example compiles without errors using `agent-illustrator`
- The rendered SVG displays all components correctly positioned
- The circuit is technically accurate (correct connections for a real MOSFET driver)

### SC-2: Template Reusability

- At least one component template is instantiated multiple times (e.g., two resistors with different values)
- Template parameters correctly customize each instance
- Connection anchors work correctly across template instances

### SC-3: Documentation Effectiveness (Primary Goal)

- The updated skill documentation causes agents to:
  - Always fetch grammar and examples before starting
  - Follow the iterative render-check-refine workflow
  - Produce syntactically valid AIL on first attempt
  - Have an accurate sense of when to stop iterating (recognizes when result is good vs needs more work)

### SC-4: Bug Discovery and Resolution

- Any bugs discovered during development are documented
- Critical bugs (preventing basic usage) are fixed or have documented workarounds
- Non-critical bugs are logged in the todo list for future work

### SC-5: Skill Documentation Use-Case Independence

- Skill documentation improvements must be **general-purpose**, not tailored to electronic schematics
- **PROHIBITED**: Adding advice like "when drawing transistors, ensure X" or "for electronic components, use Y"
- **ALLOWED**: General advice like "use debug flags to verify element positioning" or "check rendered output when templates have complex internal structure"
- The improved skill documentation should help agents create ANY diagram type better, not just circuits
- This prevents overfitting the documentation to the test case

## Key Entities

- **Electronic component template**: Reusable AIL template representing a circuit element
- **Template anchor**: Named connection point on a template for wiring
- **Circuit topology**: The arrangement and connections of components
- **Skill documentation**: The SKILL.md file that guides agents in using the tool
- **Iterative workflow**: The render → check → refine cycle for producing quality diagrams

## Assumptions

- The current template system supports parameterized templates (confirmed by railway-topology-templated.ail)
- The anchor system supports template-level anchors with direction hints (confirmed by person.ail)
- The constraint solver handles cross-template alignment constraints
- Chrome headless is available for PNG conversion (SVG uses CSS variables)
- The skill addendum approach (separate file with additional instructions) is the current pattern for improving agent guidance
- Fresh agent testing uses `claude --dangerously-skip-permissions` or `codex e` to allow full tool access for iteration
- Test agents run in isolated directories without access to the agent-illustrator source code (they only have the skill documentation)

## Out of Scope

- Actual electronic simulation or verification
- Complex multi-page schematics
- PCB layout or physical design considerations
- Interactive schematic editing
- Component libraries beyond what's needed for this example
