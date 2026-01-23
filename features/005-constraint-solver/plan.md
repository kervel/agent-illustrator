# Implementation Plan: Constraint-Based Layout System

**Feature**: 005-constraint-solver
**Branch**: `feature/constraint-solver`
**Spec**: [spec.md](./spec.md)

---

## Technical Context

| Component | Choice | Notes |
|-----------|--------|-------|
| Language | Rust (Edition 2021) | Existing codebase |
| Constraint Solver | kasuari 0.4.x | Actively maintained Cassowary implementation |
| Parser | chumsky + logos | Already in use |
| Error Reporting | ariadne | Already in use |

### Key Dependencies to Add
- `kasuari = "0.4"` - Cassowary constraint solver

---

## Constitution Check

| Principle | Compliance | Notes |
|-----------|------------|-------|
| 1. Semantic Over Geometric | **PASS** | Users declare relationships (align, constrain), not coordinates |
| 2. First-Attempt Correctness | **PASS** | Solver handles dependency chains automatically |
| 3. Explicit Over Implicit | **PASS** | New `constrain` keyword makes constraints explicit |
| 4. Fail Fast, Fail Clearly | **PASS** | Unsatisfiable constraints produce detailed errors |
| 5. Composability | **PASS** | Constraints compose naturally with layout containers |
| 6. Don't Reinvent the Wheel | **PASS** | Using kasuari (Cassowary) instead of custom solver |

---

## Implementation Phases

### Phase 0: Spike - Validate Kasuari Fitness

**Goal**: Verify kasuari can express our constraint types before committing to full implementation.

**Deliverables**:
- Standalone test in `src/layout/solver_spike.rs` demonstrating:
  1. Equality constraints: `a.left = b.left`
  2. Offset equality: `a.left = b.left + 20`
  3. Inequality constraints: `a.width >= 50`
  4. Midpoint (computed via expression): `a.center_x = (b.center_x + c.center_x) / 2`
  5. Containment via inequalities

**Gate**: If spike takes >4 hours or cannot express these constraints, evaluate z3 or custom solver.

---

### Phase 1: Core Solver Integration

**Goal**: Integrate kasuari into the layout engine, replacing procedural alignment.

**Tasks**:

1. **Add kasuari dependency**
   - Update `Cargo.toml`
   - Create `src/layout/solver.rs` module

2. **Define constraint data structures**
   ```rust
   // src/layout/solver.rs
   pub enum LayoutConstraint {
       // a.prop = b.prop + offset
       Equal { left: LayoutVariable, right: LayoutVariable, offset: f64 },
       // a.prop = constant
       Fixed { variable: LayoutVariable, value: f64 },
       // a.prop >= value
       GreaterOrEqual { variable: LayoutVariable, value: f64 },
       // a.prop <= value
       LessOrEqual { variable: LayoutVariable, value: f64 },
       // a.prop = (b.prop + c.prop) / 2
       Midpoint { target: LayoutVariable, a: LayoutVariable, b: LayoutVariable },
   }

   pub struct LayoutVariable {
       pub element_id: String,
       pub property: LayoutProperty,
   }

   pub enum LayoutProperty {
       X, Y, Width, Height,
       Left, Right, Top, Bottom,
       CenterX, CenterY,
   }
   ```

3. **Implement constraint collector**
   - Traverse AST to collect all constraints
   - Convert layout containers (row/col) into gap constraints
   - Convert `align` statements into equality constraints

4. **Implement solver wrapper**
   ```rust
   pub struct ConstraintSolver {
       solver: kasuari::Solver,
       variables: HashMap<LayoutVariable, kasuari::Variable>,
   }

   impl ConstraintSolver {
       pub fn new() -> Self;
       pub fn add_constraint(&mut self, c: LayoutConstraint) -> Result<(), SolverError>;
       pub fn solve(&mut self) -> Result<Solution, SolverError>;
   }
   ```

5. **Wire into layout pipeline**
   - Modify `layout::compute()` to use solver
   - Remove procedural alignment code (can be phased)

**Tests**:
- Unit tests for constraint translation
- Integration tests with existing examples
- Snapshot tests comparing old vs new layout output

---

### Phase 2: AST Extension for `constrain` Keyword

**Goal**: Add new syntax while keeping `align` working.

**Tasks**:

1. **Extend AST** (`src/parser/ast.rs`)
   ```rust
   pub enum ConstraintExpr {
       // a.left = b.left
       Equal { left: PropertyRef, right: PropertyRef },
       // a.left = b.left + 20
       EqualWithOffset { left: PropertyRef, right: PropertyRef, offset: f64 },
       // a.center = midpoint(b, c)
       Midpoint { target: PropertyRef, a: Spanned<Identifier>, b: Spanned<Identifier> },
       // a.width >= 50
       GreaterOrEqual { left: PropertyRef, value: f64 },
       // a.width <= 100
       LessOrEqual { left: PropertyRef, value: f64 },
       // container contains a, b, c [padding: 20]
       Contains { container: Spanned<Identifier>, elements: Vec<Spanned<Identifier>>, padding: Option<f64> },
   }

   pub struct PropertyRef {
       pub element: Spanned<ElementPath>,
       pub property: Spanned<LayoutProperty>,
   }
   ```

2. **Extend lexer** (`src/parser/lexer.rs`)
   - Add `Constrain` keyword token
   - Add `Midpoint` keyword token
   - Add `Contains` keyword token
   - Add comparison operators: `>=`, `<=`

3. **Extend parser** (`src/parser/grammar.rs`)
   - Parse `constrain` statements
   - Parse property references: `element.property`
   - Parse constraint expressions

4. **Update grammar.ebnf**
   - Document new syntax

**Tests**:
- Parser tests for all new syntax forms
- Error message tests for malformed constraints

---

### Phase 3: Container Constraint Generation

**Goal**: Make layout containers (row, col, stack, grid) generate constraints internally.

**Tasks**:

1. **Row/Column constraint generation**
   - Adjacent element gaps: `elem[i+1].left = elem[i].right + gap`
   - Alignment within container (vertical center for row, horizontal for col)

2. **Stack constraint generation**
   - All elements share same position
   - Size expands to fit largest

3. **Grid constraint generation**
   - Row/column alignment constraints
   - Cell size constraints

4. **Migrate existing tests**
   - All existing layout tests must pass with constraint-based engine

---

### Phase 4: Error Handling for Unsatisfiable Constraints

**Goal**: Provide actionable error messages when constraints conflict.

**Tasks**:

1. **Detect unsatisfiable systems**
   - Catch kasuari's constraint failures
   - Map back to source spans

2. **Generate helpful error messages**
   ```
   error: Unsatisfiable constraints
     --> example.ail:15:1
      |
   15 | constrain a.left = b.left
      | ^^^^^^^^^^^^^^^^^^^^^^^^^ constraint 1
   ...
   18 | constrain a.left = c.right + 100
      | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ constraint 2 conflicts
      |
      = note: a.left cannot equal both b.left (50) and c.right + 100 (200)
      = help: remove one constraint or adjust offsets
   ```

3. **Track constraint provenance**
   - Store source span with each internal constraint
   - Report full chain for derived constraints (from containers)

---

### Phase 5: Migration and Deprecation

**Goal**: Smooth transition from `align` to `constrain`.

**Tasks**:

1. **Keep `align` working** (converts to constraints internally)
   - `align a.left = b.left` → `constrain a.left = b.left`
   - Emit deprecation warning in parse output

2. **Update examples**
   - Convert railway example to use `constrain`
   - Document migration path

3. **Update documentation**
   - Grammar.ebnf reflects new syntax
   - README shows constraint examples

---

## Tech Stack Compliance Report

### Approved Technologies (already in stack)
- Rust (Edition 2021)
- chumsky (parser)
- logos (lexer)
- ariadne (error reporting)
- thiserror (error types)

### New Technologies (to be added)
- **kasuari 0.4.x**
  - Purpose: Cassowary constraint solving
  - No conflicts detected
  - Pure Rust, no C dependencies
  - Actively maintained by Ratatui team

### Prohibited Technologies
- None applicable to this feature

---

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| Kasuari doesn't support our constraint types | Phase 0 spike validates before commitment |
| Performance regression on large diagrams | Benchmark with 100+ element test; Cassowary is O(n) |
| Breaking existing examples | All existing tests must pass before merge |
| Constraint error messages are unclear | Phase 4 focuses on error quality |

---

## Definition of Done

- [ ] Phase 0 spike demonstrates kasuari fitness
- [ ] Solver integration passes all existing layout tests
- [ ] New `constrain` syntax parses and executes
- [ ] Layout containers generate constraints internally
- [ ] Unsatisfiable constraints produce helpful errors
- [ ] `align` syntax still works (with deprecation warning)
- [ ] Railway example works with constraints
- [ ] Grammar.ebnf updated
- [ ] No clippy warnings, `cargo fmt` clean

---

## Open Questions

1. **Constraint strength**: Should users be able to specify soft vs hard constraints? (Deferred - start with all constraints as REQUIRED)

2. **Debugging output**: Should there be a `--debug-constraints` flag to dump the constraint system? (Nice-to-have)

---

*Created: 2026-01-23*
