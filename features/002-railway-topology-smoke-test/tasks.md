# Tasks: Railway Topology Smoke Test

<!-- Tech Stack Validation: PASSED -->
<!-- No new technologies required - document creation only -->
<!-- Validated: No code changes, uses existing Feature 001 parser -->

## Feature Summary

Create the first reference DSL source document (`examples/railway-topology.ail`) for the Agent Illustrator language, describing a railway topology diagram at three abstraction levels.

**Output**: Single file `examples/railway-topology.ail`
**Validation**: Parser acceptance via `cargo test`

---

## User Story Mapping

| Story | Description | Priority |
|-------|-------------|----------|
| US1 | DSL Document Parses Successfully | P1 (Core) |
| US2 | Hierarchical Structure is Expressed | P1 |
| US3 | Track Geometry is Described (Micro) | P2 |
| US4 | Operational Points are Defined (Meso) | P2 |
| US5 | Macro Graph is Abstracted | P2 |

---

## Phase 1: Setup

### T001: Create examples directory and initial file [X]
**File**: `examples/railway-topology.ail`
**Story**: Setup (all stories depend on this)

Create the examples directory and initial DSL file with header comments.

```bash
mkdir -p examples
```

Create `examples/railway-topology.ail`:
```
// Railway Topology - Three Level Abstraction
// Smoke test for Agent Illustrator DSL
// Feature 002: First reference document

// Document structure:
// - Micro level: Track geometry (lines, switches)
// - Meso level: Operational points (OP regions)
// - Macro level: Network graph (nodes, edges)
```

**Acceptance**: File exists with header comments

---

## Phase 2: Document Structure [US1, US2]

### T002: Create column layout wrapper and three groups [X]
**File**: `examples/railway-topology.ail`
**Story**: US1 (Parse Success), US2 (Hierarchical Structure)

Add the main document structure with column layout and three empty groups.

```
col diagram {
    group micro [label: "Micro"] {
        // Track geometry - to be filled
    }

    group meso [label: "Meso"] {
        // Operational points - to be filled
    }

    group macro [label: "Macro"] {
        // Network graph - to be filled
    }
}
```

**Acceptance**:
- Document parses without error
- AST contains `col` layout with 3 `group` children
- Each group has label modifier

**Validation**:
```bash
cargo test --lib
```

---

## Phase 3: Micro Level - Track Geometry [US3]

### T003: Add track lines to micro group [X]
**File**: `examples/railway-topology.ail`
**Story**: US3 (Track Geometry)
**Depends on**: T002

Add horizontal track lines inside the micro group. Use `rect` shapes styled as track lines.

```
group micro [label: "Micro"] {
    // Main tracks (horizontal lines)
    row tracks {
        rect track1 [fill: blue, class: "track"]
        rect track2 [fill: blue, class: "track"]
        rect track3 [fill: blue, class: "track"]
        rect track4 [fill: blue, class: "track"]
    }
}
```

**Acceptance**:
- Document parses without error
- Micro group contains track shape declarations
- Track shapes have blue fill style

### T004: Add switch connections between tracks [X]
**File**: `examples/railway-topology.ail`
**Story**: US3 (Track Geometry)
**Depends on**: T003

Add connections representing switches/junctions between tracks.

```
    // Switches connecting tracks
    track1 -- track2 [class: "switch"]
    track2 -- track3 [class: "switch"]
    track3 -- track4 [class: "switch"]
```

**Acceptance**:
- Document parses without error
- AST contains connection nodes between track elements

**Checkpoint US3**: Micro level complete - track geometry expressed

---

## Phase 4: Meso Level - Operational Points [US4]

### T005: Add operational point regions to meso group [X]
**File**: `examples/railway-topology.ail`
**Story**: US4 (Operational Points)
**Depends on**: T002

Add semi-transparent circular regions for OP1 and OP2 inside the meso group.

```
group meso [label: "Meso"] {
    // Track geometry passes through (simplified representation)
    row tracks {
        rect meso_track1 [fill: blue, class: "track"]
        rect meso_track2 [fill: blue, class: "track"]
        rect meso_track3 [fill: blue, class: "track"]
    }

    // Operational Point regions (overlaid)
    ellipse op1 [fill: green, opacity: 0.3, label: "OP1"]
    ellipse op2 [fill: green, opacity: 0.3, label: "OP2"]
}
```

**Acceptance**:
- Document parses without error
- Meso group contains ellipse shapes with opacity style
- OP shapes have labels

**Checkpoint US4**: Meso level complete - operational points defined

---

## Phase 5: Macro Level - Network Graph [US5]

### T006: Add graph nodes and edges to macro group [X]
**File**: `examples/railway-topology.ail`
**Story**: US5 (Macro Graph)
**Depends on**: T002

Add the simplified network graph with node shapes and labeled connections.

```
group macro [label: "Macro"] {
    // Graph layout
    row graph {
        // Line section to left
        rect left_section [fill: blue, label: "Line Section"]

        // OP1 node
        ellipse macro_op1 [fill: green, label: "OP1"]

        // Line section between
        rect center_section [fill: blue, label: "Line Section"]

        // OP2 node
        ellipse macro_op2 [fill: green, label: "OP2"]

        // Line section to right
        rect right_section [fill: blue, label: "Line Section"]
    }

    // Connections
    left_section -- macro_op1
    macro_op1 -- center_section
    center_section -- macro_op2
    macro_op2 -- right_section
}
```

**Acceptance**:
- Document parses without error
- Macro group contains ellipse nodes and connections
- No micro-level track detail in macro group

**Checkpoint US5**: Macro level complete - graph abstraction expressed

---

## Phase 6: Aggregation Arrows [US2]

### T007: Add aggregation arrows between levels [X]
**File**: `examples/railway-topology.ail`
**Story**: US2 (Hierarchical Structure)
**Depends on**: T002, T003, T005, T006

Add directed connections between level groups with "Aggregation" labels.

Insert between groups in the column layout:
```
col diagram {
    group micro [label: "Micro"] {
        // ... micro content ...
    }

    // Aggregation arrow: Micro → Meso
    micro -> meso [label: "Aggregation", fill: gray]

    group meso [label: "Meso"] {
        // ... meso content ...
    }

    // Aggregation arrow: Meso → Macro
    meso -> macro [label: "Aggregation", fill: gray]

    group macro [label: "Macro"] {
        // ... macro content ...
    }
}
```

**Acceptance**:
- Document parses without error
- AST contains connection nodes between groups
- Connections have "Aggregation" labels

**Checkpoint US2**: Hierarchical structure complete with aggregation flow

---

## Phase 7: Final Validation [US1]

### T008: Validate complete document and line count [X]
**File**: `examples/railway-topology.ail`
**Story**: US1 (Parse Success), All stories
**Depends on**: T007

Final validation steps:
1. Run parser on complete document
2. Verify line count < 100
3. Check AST completeness
4. Add any missing comments for clarity

**Validation Commands**:
```bash
# Parse test
cargo test --lib

# Line count check
wc -l examples/railway-topology.ail
# Must be < 100

# Optional: Add a quick parse check in Rust if needed
```

**Acceptance Criteria (All Success Criteria)**:
- [X] Parse Success: Document parses without errors
- [X] AST Completeness: All visual elements represented
- [X] DSL Clarity: Structure is readable and commented
- [X] Compactness: Under 100 lines (72 lines)
- [X] Grammar Coverage: Uses shapes, groups, connections, labels, styles, layouts

---

## Task Dependency Graph

```
T001 (Setup)
  │
  v
T002 (Structure) ─────────────────────────┐
  │                                       │
  ├──────────┬──────────┬─────────────────┤
  │          │          │                 │
  v          v          v                 │
T003 [P]   T005 [P]   T006 [P]            │
(Micro)    (Meso)     (Macro)             │
  │          │          │                 │
  v          │          │                 │
T004        │          │                 │
(Switches)  │          │                 │
  │          │          │                 │
  └──────────┴──────────┴─────────────────┤
                                          │
                                          v
                                        T007
                                   (Aggregation)
                                          │
                                          v
                                        T008
                                   (Validation)
```

**[P] = Parallelizable**: T003, T005, T006 can be done in parallel after T002

---

## Parallel Execution Guide

### Sequential Critical Path
```
T001 → T002 → T007 → T008
```

### Parallel Opportunities
After T002 completes, these can run in parallel:
- **Worker A**: T003 → T004 (Micro level)
- **Worker B**: T005 (Meso level)
- **Worker C**: T006 (Macro level)

Then converge at T007.

**Note**: Since this is a single-file feature, parallelization is conceptual. In practice, one agent writes the file incrementally.

---

## Summary

| Metric | Value |
|--------|-------|
| Total Tasks | 8 |
| Setup Tasks | 1 |
| Structure Tasks | 1 |
| Content Tasks | 5 |
| Validation Tasks | 1 |
| Parallel Opportunities | 3 (T003, T005, T006) |
| Output Files | 1 (`examples/railway-topology.ail`) |

**MVP Scope**: T001 → T002 → T008 (empty groups that parse)
**Full Scope**: All tasks (complete illustration)

---

*Tasks generated: 2026-01-23*
