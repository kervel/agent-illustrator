# Research: Constraint Solver Selection

**Feature**: 005-constraint-solver
**Date**: 2026-01-23

---

## Decision: Use Kasuari for Constraint Solving

### Rationale

Kasuari is the clear choice for our constraint solver:

1. **Actively maintained** - Under the Ratatui organization, recent commits as of January 2026
2. **Pure Rust** - No C/C++ dependencies (complies with tech-stack.md)
3. **Battle-tested** - Used in Ratatui's layout system, a popular TUI framework
4. **Cassowary algorithm** - Same algorithm used by Apple's Auto Layout, proven for UI constraints

### Alternatives Considered

| Option | Pros | Cons | Decision |
|--------|------|------|----------|
| **kasuari** | Active, pure Rust, production-proven | Less documentation than Apple's | **SELECTED** |
| cassowary-rs | Original implementation | Unmaintained since 2018 | Rejected |
| z3 (via z3-sys) | Full SMT solver, very powerful | Heavy dependency, C++ binding, overkill | Fallback only |
| Custom solver | Full control | Significant effort, violates Principle 6 | Rejected |

---

## Kasuari API Findings

### Key Types

- `Variable` - Represents an unknown value (e.g., `element.x`, `element.width`)
- `Solver` - Manages constraints and computes solutions
- `Expression` - Linear combinations for constraint equations
- `Strength` - Priority levels (REQUIRED, STRONG, MEDIUM, WEAK)

### Constraint Expression Syntax

Kasuari uses a "pipe bracket" notation with `WeightedRelation` enum:

```rust
use kasuari::{Variable, Solver, Strength::*, WeightedRelation::*};

let a_left = Variable::new();
let b_left = Variable::new();

// Equality: a.left = b.left
solver.add_constraint(a_left |EQ(REQUIRED)| b_left)?;

// Equality with offset: a.left = b.left + 20
solver.add_constraint(a_left |EQ(REQUIRED)| b_left + 20.0)?;

// Inequality: width >= 50
solver.add_constraint(width |GE(REQUIRED)| 50.0)?;

// Midpoint: center = (a + b) / 2
// Express as: 2*center = a + b
solver.add_constraint(2.0 * center |EQ(REQUIRED)| a + b)?;
```

### Solving and Extracting Values

```rust
// Add edit variables for values we'll update
solver.add_edit_variable(window_width, STRONG)?;
solver.suggest_value(window_width, 500.0)?;

// Get changes
for (var, value) in solver.fetch_changes() {
    // Apply new values to our layout
}
```

---

## Constraint Type Mapping

Our spec defines several constraint types. Here's how they map to kasuari:

### Position Constraints

| Our Syntax | Kasuari Expression |
|------------|-------------------|
| `a.x = 100` | `a_x \|EQ(REQUIRED)\| 100.0` |
| `a.x = b.x + 20` | `a_x \|EQ(REQUIRED)\| b_x + 20.0` |

### Alignment Constraints

| Our Syntax | Kasuari Expression |
|------------|-------------------|
| `a.left = b.left` | `a_left \|EQ(REQUIRED)\| b_left` |
| `a.center_x = b.center_x` | `a_center_x \|EQ(REQUIRED)\| b_center_x` |

### Midpoint Constraints

| Our Syntax | Kasuari Expression |
|------------|-------------------|
| `a.center_x = midpoint(b, c)` | `2.0 * a_center_x \|EQ(REQUIRED)\| b_center_x + c_center_x` |

### Containment Constraints

| Our Syntax | Kasuari Expressions |
|------------|-------------------|
| `container contains a, b [padding: 10]` | Multiple inequalities per child |

```rust
// For each child:
solver.add_constraint(container_left |LE(REQUIRED)| child_left - padding)?;
solver.add_constraint(container_right |GE(REQUIRED)| child_right + padding)?;
solver.add_constraint(container_top |LE(REQUIRED)| child_top - padding)?;
solver.add_constraint(container_bottom |GE(REQUIRED)| child_bottom + padding)?;
```

### Size Constraints

| Our Syntax | Kasuari Expression |
|------------|-------------------|
| `a.width = 100` | `a_width \|EQ(REQUIRED)\| 100.0` |
| `a.width >= 50` | `a_width \|GE(REQUIRED)\| 50.0` |
| `a.height <= 200` | `a_height \|LE(REQUIRED)\| 200.0` |

---

## Variable Naming Strategy

Each element needs up to 6 variables:
- `{id}_x`, `{id}_y` (position)
- `{id}_width`, `{id}_height` (size)

Derived values are computed from these:
- `left = x`
- `right = x + width`
- `top = y`
- `bottom = y + height`
- `center_x = x + width/2`
- `center_y = y + height/2`

---

## Performance Considerations

Cassowary is designed for UI layout with these characteristics:
- O(n) complexity for typical constraint systems
- Incremental solving (only recomputes affected variables)
- Edit variables for efficient repeated updates

For our target scale (50-500 elements), solve time should be well under 100ms.

---

## Spike Validation Plan

Before full implementation, validate with standalone test:

```rust
#[test]
fn spike_kasuari_fitness() {
    // 1. Create 5 elements (10 variables: x, y each)
    // 2. Add alignment: a.left = b.left
    // 3. Add offset: c.left = b.right + 20
    // 4. Add midpoint: d.center_x = (a.center_x + c.center_x) / 2
    // 5. Add inequality: e.width >= 50
    // 6. Solve and verify values
}
```

If this works smoothly, proceed with full implementation.

---

## References

- [Kasuari crate](https://crates.io/crates/kasuari)
- [Kasuari docs](https://docs.rs/kasuari/latest/kasuari/)
- [Kasuari GitHub](https://github.com/ratatui/kasuari)
- [Cassowary algorithm paper](https://constraints.cs.washington.edu/cassowary/)

---

*Created: 2026-01-23*
