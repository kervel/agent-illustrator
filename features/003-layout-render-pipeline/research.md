# Research: Layout and Render Pipeline

## Overview

Research findings for feature 003 - Layout and Render Pipeline implementation.

---

## Research Task 1: Layout Engine Approach

### Question
Should we use an existing layout library (per Constitution Principle 6: "Do not reinvent the wheel") or build a custom layout engine?

### Decision
**Hybrid approach**: Build a minimal custom layout engine with potential Graphviz integration for complex cases.

### Rationale
1. **Constitution Principle 6** suggests leveraging existing tools where possible
2. **Graphviz** (via `gv` or `layout` Rust crates) excels at graph layout but:
   - Adds external dependency (C library)
   - May be overkill for simple row/column layouts
   - Doesn't directly support our semantic constraints
3. **Custom engine** provides:
   - Direct control over semantic layout (row, column, grid, stack)
   - Constraint resolution tailored to our DSL
   - Pure Rust (no C dependencies per tech-stack.md)

### Alternatives Considered
| Option | Pros | Cons |
|--------|------|------|
| Graphviz only | Mature, handles complex graphs | C dependency, doesn't fit row/column model |
| Custom only | Full control, pure Rust | More work, may miss edge cases |
| Hybrid | Best of both | Complexity in abstraction |

### Implementation Strategy
Start with custom layout engine for:
- Row/column/grid/stack layouts (straightforward algorithms)
- Simple constraint resolution
- Basic connection routing

Consider Graphviz integration later for:
- Complex graph layouts
- Automatic optimal placement when no layout hints given

---

## Research Task 2: SVG Generation Approach

### Question
Use the `svg` crate or generate SVG strings directly?

### Decision
**Direct XML string generation** with a thin abstraction layer.

### Rationale
1. SVG structure is simple and well-understood
2. `svg` crate adds dependency for minimal benefit
3. Direct generation gives full control over:
   - CSS class output (important per tech-stack.md: "extensive use of standardized classes")
   - Formatted, human-readable output
   - Exact attribute ordering and whitespace

### Code Pattern
```rust
struct SvgBuilder {
    elements: Vec<SvgElement>,
    defs: Vec<SvgDef>,
}

impl SvgBuilder {
    fn add_rect(&mut self, bbox: &BoundingBox, styles: &ResolvedStyles) { ... }
    fn add_path(&mut self, path: &ConnectionPath, styles: &ResolvedStyles) { ... }
    fn to_string(&self) -> String { ... }
}
```

---

## Research Task 3: Connection Routing Algorithm

### Question
What routing algorithm should we use for connections?

### Decision
**Orthogonal routing with simple obstacle avoidance** for initial implementation.

### Rationale
1. Orthogonal (horizontal/vertical only) routes are:
   - Clean and professional-looking
   - Predictable for LLMs
   - Simpler to implement
2. Full obstacle avoidance (A* pathfinding) is deferred:
   - Added complexity
   - May not be needed for typical diagrams
   - Can be added incrementally

### Algorithm
1. Determine edge attachment points based on relative positions
2. Create L-shaped or Z-shaped routes (1-2 bends)
3. For simple cases: direct line or single bend
4. Defer complex multi-obstacle routing

---

## Research Task 4: Constraint Solver Approach

### Question
How to resolve position constraints (`place X below Y`)?

### Decision
**Two-pass layout with topological constraint resolution**.

### Rationale
1. First pass: compute default positions from layout containers
2. Second pass: apply constraints as position adjustments
3. Conflict detection: check if constraint satisfaction is geometrically possible

### Algorithm
```
1. Build constraint graph (subject -> anchor relationships)
2. Topological sort to find dependency order
3. For each constraint in order:
   a. Get anchor's final position
   b. Compute subject's new position based on relation
   c. Check for conflicts with already-placed elements
   d. If conflict: return error identifying constraints
4. Return adjusted positions
```

### Conflict Detection
Constraints conflict when:
- Circular dependencies exist (A below B, B below A)
- Spatial impossibility (A right-of B AND A left-of B)
- Overlapping regions from multiple constraints

---

## Research Task 5: Default Sizing Strategy

### Question
How should default element sizes be determined?

### Decision
**Content-aware defaults with configurable overrides**.

### Rationale
1. Labels should fit within shapes
2. Approximate text width: ~8 units per character
3. Default dimensions:
   - Rectangle: 100x50 (fits ~10 char labels comfortably)
   - Circle: 60 diameter
   - Padding: 10 units internal, 20 units between elements

### Configuration
```rust
struct LayoutConfig {
    default_rect_size: (f64, f64),  // (100.0, 50.0)
    default_circle_radius: f64,     // 30.0
    element_spacing: f64,           // 20.0
    padding: f64,                   // 10.0
    char_width_estimate: f64,       // 8.0
}
```

---

## Research Task 6: Identifier Validation

### Question
When should undefined identifier references be caught?

### Decision
**Layout-time validation** (per clarification session).

### Rationale
1. Parser remains simple and context-free
2. Layout engine has full document context
3. Error messages can include suggestions (did you mean "server1"?)

### Implementation
```rust
fn validate_references(doc: &Document) -> Result<(), LayoutError> {
    let defined: HashSet<&str> = collect_defined_identifiers(doc);
    for conn in connections(doc) {
        if !defined.contains(conn.from.as_str()) {
            return Err(LayoutError::UndefinedIdentifier {
                name: conn.from.clone(),
                span: conn.from.span,
                suggestions: find_similar(&defined, conn.from.as_str()),
            });
        }
        // ... same for conn.to
    }
    Ok(())
}
```

---

## Summary

| Decision | Choice | Confidence |
|----------|--------|------------|
| Layout engine | Custom (hybrid-ready) | High |
| SVG generation | Direct string | High |
| Connection routing | Orthogonal, simple | High |
| Constraint solver | Two-pass topological | High |
| Default sizing | Content-aware | Medium |
| Reference validation | Layout-time | High (per clarification) |

---

*Created: 2026-01-23*
