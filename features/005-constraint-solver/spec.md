# Constraint-Based Layout System

## Clarifications

### Session 2026-01-23
- Q: Which syntax approach for constraints? → A: New `constrain` keyword (replaces `align`, handles equality and inequality)
- Q: How to handle unsatisfiable constraints? → A: Hard error with detailed conflict report
- Q: Expected diagram scale? → A: Medium (50-500 elements), solve time <1 second acceptable
- Q: Support inequality constraints? → A: Yes, full support for min/max width, height, gaps
- Q: How do containers interact with constraints? → A: Containers generate constraints (syntactic sugar)

## Problem Statement

The current procedural layout approach processes layout operations in a fixed order:
1. Initial layout (row/col/stack)
2. Alignments
3. Position offsets

This breaks down when:
- **Dependency chains**: Element A aligns to B, but B was aligned with offset to C
- **Computed values**: Aligning to the center of something whose width depends on contents
- **Cross-hierarchy references**: Aligning elements from different groups/containers
- **Nested AILs** (future): Imported diagrams with their own internal constraints

The result: Users must manually calculate pixel offsets, violating the core principle of "semantic over geometric."

## Proposed Solution

Replace the procedural layout with a **constraint solver** that:
1. Collects all layout relationships as constraints
2. Solves for values satisfying all constraints simultaneously
3. Handles dependency chains automatically
4. Detects unsatisfiable/conflicting constraints

## Constraint Types

### Position Constraints

```
// Absolute positioning
element.x = 100
element.y = 50

// Relative positioning (offsets)
element.x = other.x + 20
element.y = other.bottom + 10
```

### Alignment Constraints

```
// Edge alignment
a.left = b.left
a.right = b.right
a.top = b.top
a.bottom = b.bottom

// Center alignment
a.center_x = b.center_x
a.center_y = b.center_y

// Multi-element alignment (chain)
a.left = b.left = c.left
```

### Computed Center Constraints

```
// Center between two elements
a.center_x = (b.center_x + c.center_x) / 2
a.center_y = (b.center_y + c.center_y) / 2

// Or using midpoint syntax
a.center = midpoint(b, c)
```

### Container Constraints

```
// Element must contain others (with optional padding)
container.left <= child.left - padding
container.right >= child.right + padding
container.top <= child.top - padding
container.bottom >= child.bottom + padding
```

### Size Constraints

```
// Fixed size
element.width = 100
element.height = 50

// Relative size
a.width = b.width
a.height = b.height * 2

// Minimum/maximum
element.width >= 50
element.height <= 200
```

## Syntax (Decision: `constrain` keyword)

Single `constrain` keyword replaces `align` and handles all constraint types:

```
// Equality (replaces align)
constrain a.left = b.left
constrain a.center_x = b.center_x + 40

// Midpoint
constrain a.center = midpoint(b, c)

// Inequalities
constrain a.width >= 50
constrain a.height <= b.height

// Containment
constrain container contains a, b, c [padding: 20]
```

**Breaking change**: `align` keyword is removed, replaced entirely by `constrain`.

Layout containers (`row`, `col`, `stack`, `grid`) become syntactic sugar that generate constraints internally.

## Implementation Approach

### Phase 1: Constraint Collection

Convert existing layout operations to constraints:

```rust
enum Constraint {
    // a.prop = b.prop + offset
    Equal {
        left: Variable,
        right: Variable,
        offset: f64,
    },
    // a.prop = (b.prop + c.prop) / 2
    Midpoint {
        target: Variable,
        a: Variable,
        b: Variable,
    },
    // a.prop >= value
    GreaterOrEqual {
        variable: Variable,
        value: f64,
    },
    // a.prop <= value
    LessOrEqual {
        variable: Variable,
        value: f64,
    },
}

struct Variable {
    element_id: String,
    property: Property, // X, Y, Width, Height, Left, Right, Top, Bottom, CenterX, CenterY
}
```

### Phase 2: Constraint Solver

**Initial choice**: Kasuari (`kasuari`) - actively maintained Cassowary implementation used by Ratatui.

**Fallback options** (if Kasuari proves unsuitable):
1. **Custom linear solver**: Gaussian elimination for simple equality systems
2. **z3** via `z3-sys`: Full SMT solver (heavier dependency but very capable)

#### Decision Gate: When to Abandon Kasuari

Stop trying Kasuari and switch if ANY of these occur within first 2 days of implementation:
- Cannot express midpoint constraints (a = (b + c) / 2) without manual variable substitution
- Cannot handle our inequality constraints (>=, <=) for contains/min-max
- API is too low-level, requiring >100 lines of boilerplate per constraint type
- Performance is >1 second for 100 elements
- Documentation/examples are insufficient to understand the model

**Spike task**: Before full implementation, write a standalone test that:
1. Creates 10 variables (x, y for 5 elements)
2. Adds equality constraints (align)
3. Adds midpoint constraint
4. Adds inequality constraint (contains)
5. Solves and extracts values

If the spike fails or takes >4 hours, evaluate alternatives before proceeding.

### Phase 3: Layout Pipeline Refactor

```rust
fn layout(doc: &Document) -> LayoutResult {
    // 1. Create solver
    let mut solver = Solver::new();

    // 2. Add intrinsic constraints (shape sizes, text measurements)
    add_intrinsic_constraints(&mut solver, doc);

    // 3. Add layout constraints (row/col/stack/grid)
    add_layout_constraints(&mut solver, doc);

    // 4. Add user constraints (align, place)
    add_user_constraints(&mut solver, doc);

    // 5. Solve
    let solution = solver.solve()?;

    // 6. Extract positions
    build_layout_result(solution)
}
```

## Migration Path

1. **Keep existing syntax working**: Current `align` and `place` statements compile to constraints
2. **Add new constraint features incrementally**: midpoint, contains, etc.
3. **Deprecate nothing initially**: Old examples continue to work

## Example: Railway Topology with Constraints

Current (procedural, broken):
```
ellipse op1 [...]
place op1 [x: -66]  // Magic number from trial and error
```

With constraints:
```
ellipse op1 [...]
constrain op1.center_x = midpoint(mjA1.center_x, mjB1.center_x)
constrain op1.center_y = midpoint(mtrackA.center_y, mtrackB.center_y)
```

Or with contains:
```
ellipse op1 [...]
constrain op1 contains mjA1, mjB1 [padding: 30]
```

## Error Handling

When constraints are unsatisfiable or conflict, compilation fails with a detailed error report:

```
error: Unsatisfiable constraints
  --> example.ail:15:1
   |
15 | align a.left = b.left
   | ^^^^^^^^^^^^^^^^^^^^^ constraint 1
...
18 | align a.left = c.right + 100
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ constraint 2 conflicts
   |
   = note: a.left cannot simultaneously equal b.left (50) and c.right + 100 (200)
   = help: remove one constraint or adjust offsets
```

## Open Questions (Remaining)

1. **Nested AILs**: How do imported diagram constraints interact with parent constraints? (Deferred to nested AIL feature)
2. **Kasuari fit**: Will Kasuari handle our constraint types? (Resolved by spike task)

## References

- [Cassowary algorithm](https://constraints.cs.washington.edu/cassowary/)
- [kasuari](https://github.com/ratatui/kasuari) - Actively maintained Rust implementation
- [Ratatui layout docs](https://docs.rs/ratatui/latest/ratatui/layout/index.html) - Example usage in production
- [Apple Auto Layout Guide](https://developer.apple.com/library/archive/documentation/UserExperience/Conceptual/AutolayoutPG/)
