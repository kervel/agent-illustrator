# Implementation Plan: Railway Topology Smoke Test

## Technical Context

### Feature Summary
Create the first reference DSL source document for the Agent Illustrator language, describing a railway topology diagram at three abstraction levels (Micro, Meso, Macro).

### Dependencies
- **Feature 001**: Grammar and AST (COMPLETE - merged to main)
  - Parser available via `agent_illustrator::parser::parse()`
  - All required constructs implemented (shapes, connections, layouts, groups, constraints, styles)

### Available DSL Constructs (from Feature 001)

| Construct | Syntax | Purpose |
|-----------|--------|---------|
| Shapes | `rect name`, `circle name`, `ellipse name` | Geometric primitives |
| Icons | `icon "type" name` | Semantic shapes |
| Connections | `a -> b`, `a -- b` | Relationships with direction |
| Layouts | `row { }`, `col { }`, `grid { }`, `stack { }` | Arrangement containers |
| Groups | `group name { }` | Semantic grouping |
| Constraints | `place x right-of y` | Relative positioning |
| Styles | `[fill: color, opacity: 0.5, label: "text"]` | Visual modifiers |

### Tech Stack Compliance Report

**No new technologies required** - this feature creates a `.ail` document file using the existing grammar. No code changes needed.

---

## Constitution Check

| Principle | Status | Notes |
|-----------|--------|-------|
| 1. Semantic Over Geometric | PASS | DSL describes relationships, not coordinates |
| 2. First-Attempt Correctness | PASS | Document must parse correctly on first run |
| 3. Explicit Over Implicit | PASS | All elements explicitly declared |
| 4. Fail Fast, Fail Clearly | N/A | Feature creates document, not code |
| 5. Composability | PASS | Uses groups and layouts for composition |
| 6. Don't Reinvent Wheel | PASS | Uses existing Feature 001 parser |

---

## Implementation Strategy

### Approach
This feature creates a single DSL document file. The implementation is:
1. Map reference image elements to DSL constructs
2. Write the document incrementally, testing parsing after each section
3. Validate all acceptance criteria via parser

### Output Artifact
- `examples/railway-topology.ail` - The DSL source document

---

## Phase 0: Element Mapping (Research)

### Reference Image → DSL Mapping

| Visual Element | DSL Construct | Notes |
|----------------|---------------|-------|
| Micro level label | Text/label modifier | `[label: "Micro"]` |
| Meso level label | Text/label modifier | `[label: "Meso"]` |
| Macro level label | Text/label modifier | `[label: "Macro"]` |
| Track lines | `rect` shapes in a group | Horizontal lines with connections |
| Switch/junction | Connection between tracks | `track1 -- track2` |
| OP1 region (meso) | `circle` or `ellipse` with opacity | `circle op1 [fill: green, opacity: 0.3]` |
| OP2 region (meso) | `circle` or `ellipse` with opacity | `circle op2 [fill: green, opacity: 0.3]` |
| OP1 node (macro) | `ellipse` shape | `ellipse macro_op1 [fill: green, label: "OP1"]` |
| OP2 node (macro) | `ellipse` shape | `ellipse macro_op2 [fill: green, label: "OP2"]` |
| Line sections | Connections with labels | `macro_op1 -- macro_op2 [label: "Line Section"]` |
| Aggregation arrows | Connections between groups | `micro -> meso [label: "Aggregation"]` |
| Three-level layout | `col` container | Vertical arrangement |

---

## Phase 1: Document Structure

### Document Skeleton

```
// Railway Topology - Three Level Abstraction
// Smoke test for Agent Illustrator DSL

col diagram {
    // Level 1: Micro (track geometry)
    group micro [label: "Micro"] {
        ...tracks and switches...
    }

    // Aggregation arrow
    micro -> meso [label: "Aggregation", style: gray]

    // Level 2: Meso (operational points)
    group meso [label: "Meso"] {
        ...tracks with OP regions overlaid...
    }

    // Aggregation arrow
    meso -> macro [label: "Aggregation", style: gray]

    // Level 3: Macro (network graph)
    group macro [label: "Macro"] {
        ...nodes and edges only...
    }
}
```

---

## Implementation Tasks

### Task 1: Create examples directory and file
- Create `examples/` directory if not exists
- Create `examples/railway-topology.ail` with header comment

### Task 2: Implement Micro Level
Write the track geometry section:
- 4-6 horizontal track lines
- Switch connections between tracks
- Group with "Micro" label

**Acceptance**: Section parses without error

### Task 3: Implement Meso Level
Write the operational points section:
- Track geometry (can reference or duplicate micro)
- Two semi-transparent circle/ellipse regions for OP1 and OP2
- Labels for each OP

**Acceptance**: Section parses without error

### Task 4: Implement Macro Level
Write the abstract graph section:
- Two ellipse nodes (OP1, OP2)
- Connections labeled "Line Section"
- No track detail

**Acceptance**: Section parses without error

### Task 5: Add Aggregation Arrows
Connect the three levels:
- `micro -> meso` with "Aggregation" label
- `meso -> macro` with "Aggregation" label
- Gray color style

**Acceptance**: Connections parse without error

### Task 6: Wrap in Column Layout
Combine all sections in a `col` layout for vertical arrangement

**Acceptance**: Complete document parses without error

### Task 7: Validate and Refine
- Run parser on complete document
- Check AST has all expected elements
- Adjust styling for clarity
- Ensure under 100 lines

**Acceptance**: All success criteria met

---

## Verification Plan

### Parse Test
```bash
# Run from project root
cargo test --lib -- --nocapture
# Or create a simple test that parses the file
```

### Manual Verification Checklist
- [ ] Document parses without errors
- [ ] AST contains 3 groups (micro, meso, macro)
- [ ] AST contains aggregation connections
- [ ] Track elements present in micro group
- [ ] OP regions present in meso group
- [ ] Graph nodes present in macro group
- [ ] Document is under 100 lines
- [ ] Comments explain structure

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Grammar doesn't support needed construct | Low | High | Feature 001 covers all needs |
| Parser errors unclear | Low | Medium | Feature 001 has ariadne diagnostics |
| Document too verbose | Medium | Low | Iterate on compactness |

---

## Notes

- This is a **document creation** task, not a code implementation task
- No Rust code will be written (beyond optional test)
- Success is measured by parser acceptance and AST completeness
- Visual verification deferred to rendering feature

---

*Plan created: 2026-01-23*
