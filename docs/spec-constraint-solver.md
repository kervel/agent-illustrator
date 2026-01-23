# Constraint-Based Layout System

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

## Syntax Proposals

### Option A: Extended align syntax

```
// Current
align a.left = b.left

// Extended with offset
align a.center_x = b.center_x + 40

// Extended with midpoint
align a.center = midpoint(b, c)

// Contains constraint
align container contains a, b, c [padding: 20]
```

### Option B: Separate constraint block

```
constraints {
    a.center_x = (b.center_x + c.center_x) / 2
    a.center_y = (b.center_y + c.center_y) / 2
    op1 contains mjA1, mjB1 [padding: 20]
}
```

### Option C: Inline modifiers

```
ellipse op1 [center_between: mjA1 mjB1, contains: mjA1 mjB1]
```

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

Options:
1. **Kasuari** (`kasuari`): Actively maintained Cassowary implementation, used by Ratatui
2. **Custom solver**: Simple Gaussian elimination for linear systems
3. **Z3** (overkill): Full SMT solver

Kasuari is the pragmatic choice - it's the maintained fork of cassowary-rs (last updated 8 years ago),
actively used in production by Ratatui for terminal UI layout.

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
align op1.center_x = midpoint(mjA1.center_x, mjB1.center_x)
align op1.center_y = midpoint(mtrackA.center_y, mtrackB.center_y)
```

Or with contains:
```
ellipse op1 [contains: mjA1 mjB1, padding: 30]
```

## Open Questions

1. **Syntax**: Which option (A, B, C) or combination?
2. **Error reporting**: How to explain unsatisfiable constraints to users?
3. **Performance**: Is Cassowary fast enough for large diagrams?
4. **Nested AILs**: How do imported diagram constraints interact with parent constraints?

## References

- [Cassowary algorithm](https://constraints.cs.washington.edu/cassowary/)
- [kasuari](https://github.com/ratatui/kasuari) - Actively maintained Rust implementation
- [Ratatui layout docs](https://docs.rs/ratatui/latest/ratatui/layout/index.html) - Example usage in production
- [Apple Auto Layout Guide](https://developer.apple.com/library/archive/documentation/UserExperience/Conceptual/AutolayoutPG/)
