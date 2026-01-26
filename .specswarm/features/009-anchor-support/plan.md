# Implementation Plan: Anchor Support for Shape Connections

## Technical Context

| Component | Technology | Notes |
|-----------|------------|-------|
| Language | Rust 2021 edition | Existing codebase |
| Parser | chumsky + logos | Pattern-based grammar |
| Layout | kasuari constraint solver | Cassowary-based |
| Renderer | SVG output | Direct XML generation |

**Dependencies**: No new dependencies required. Extends existing AST and routing infrastructure.

---

## Constitution Check

| Principle | Compliance | Notes |
|-----------|------------|-------|
| 1. Semantic Over Geometric | ✅ Pass | Anchors are named semantic references (`.top`, `.left`), not coordinates |
| 2. First-Attempt Correctness | ✅ Pass | Built-in anchors "just work" for common shapes |
| 3. Explicit Over Implicit | ✅ Pass | Users explicitly specify anchor names; auto-detect remains as fallback |
| 4. Fail Fast, Fail Clearly | ✅ Pass | Invalid anchor names produce errors with valid alternatives |
| 5. Composability | ✅ Pass | Anchors compose with connections, constraints, templates |
| 6. Don't Reinvent Wheel | ✅ Pass | Leverages existing ConstraintProperty system for anchor computation |

---

## Tech Stack Compliance Report

### ✅ Approved Technologies
- Rust (existing)
- chumsky parser (existing)
- logos lexer (existing)
- ariadne diagnostics (existing)
- kasuari constraint solver (existing)

### ➕ New Technologies
*None - this feature uses only existing stack*

### ⚠️ Conflicting Technologies
*None detected*

### ❌ Prohibited Technologies
*None used*

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Parser Layer                                │
├─────────────────────────────────────────────────────────────────────┤
│  ast.rs: Add AnchorReference to ConnectionDecl                      │
│  ast.rs: Add AnchorDecl statement for templates                     │
│  grammar.rs: Parse shape.anchor dot notation                        │
│  grammar.rs: Parse `anchor name [position: ...]` in templates       │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         Layout Layer                                │
├─────────────────────────────────────────────────────────────────────┤
│  types.rs: Add Anchor struct and AnchorSet                          │
│  engine.rs: Compute built-in anchors for shapes                     │
│  engine.rs: Register template-defined anchors                       │
│  routing.rs: Use anchor positions for connection endpoints          │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        Renderer Layer                               │
├─────────────────────────────────────────────────────────────────────┤
│  svg.rs: Render connections starting/ending at anchor points        │
│  (minimal changes - uses existing path rendering)                   │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Data Model

### New AST Types

```rust
/// Reference to an anchor on an element
pub struct AnchorReference {
    pub element: Spanned<Identifier>,    // The shape/template instance
    pub anchor: Option<Spanned<String>>, // The anchor name (None = auto-detect)
}

/// Anchor declaration in a template
pub struct AnchorDecl {
    pub name: Spanned<Identifier>,
    pub position: AnchorPosition,
    pub direction: Option<AnchorDirectionSpec>,  // NEW: optional explicit direction
}

/// How an anchor position is specified
pub enum AnchorPosition {
    PropertyRef {
        element: Identifier,
        property: ConstraintProperty,
    },
    Expression {
        element: Identifier,
        property: ConstraintProperty,
        offset: f64,
    },
}

/// Direction specification for template anchors
pub enum AnchorDirectionSpec {
    Cardinal(CardinalDirection),  // up, down, left, right
    Angle(f64),                   // explicit angle in degrees
}

pub enum CardinalDirection {
    Up, Down, Left, Right,
}
```

### Updated ConnectionDecl

```rust
pub struct ConnectionDecl {
    pub from: AnchorReference,           // CHANGED: was Spanned<Identifier>
    pub to: AnchorReference,             // CHANGED: was Spanned<Identifier>
    pub direction: ConnectionDirection,
    pub modifiers: Vec<Spanned<StyleModifier>>,
}
```

### Layout Types

```rust
/// Direction a connector should approach/leave an anchor
/// Connectors should arrive/depart perpendicular to the shape at this anchor
pub enum AnchorDirection {
    Up,         // Connector approaches from above (for bottom anchors)
    Down,       // Connector approaches from below (for top anchors)
    Left,       // Connector approaches from the left (for right anchors)
    Right,      // Connector approaches from the right (for left anchors)
    Angle(f64), // Custom angle in degrees (for template-defined anchors)
}

/// Computed anchor with resolved position and direction
pub struct Anchor {
    pub name: String,
    pub position: Point,
    pub direction: AnchorDirection,  // NEW: outward normal direction
}

/// Set of anchors for an element
pub struct AnchorSet {
    anchors: HashMap<String, Anchor>,  // CHANGED: stores Anchor, not just Point
}

impl AnchorSet {
    pub fn simple_shape(bounds: &BoundingBox) -> Self;  // top, bottom, left, right
    pub fn path_shape(bounds: &BoundingBox) -> Self;    // + corners
    pub fn from_custom(anchors: Vec<Anchor>) -> Self;   // template-defined
}
```

**Anchor Direction Semantics**:
- The direction is the **outward normal** at the anchor point
- Connectors should arrive/depart along this direction (perpendicular to shape edge)
- For `top` anchor: direction is `Up` (connector comes from above)
- For `bottom` anchor: direction is `Down` (connector comes from below)
- For corner anchors (e.g., `top_left`): direction is diagonal outward
- For template anchors: direction can be explicit (`up`, `down`, `left`, `right`) or `Angle(degrees)`

**Built-in Anchor Directions**:

| Anchor | Direction | Angle (degrees) |
|--------|-----------|-----------------|
| top | Up | 270° |
| bottom | Down | 90° |
| left | Left | 180° |
| right | Right | 0° |
| top_left | Angle(-135°) | 225° |
| top_right | Angle(-45°) | 315° |
| bottom_left | Angle(135°) | 135° |
| bottom_right | Angle(45°) | 45° |

**Routing Behavior**:
- For **orthogonal** routing: First/last segment must be perpendicular to anchor
- For **direct** routing: Line approaches anchor from the direction indicated
- For **curved** routing: Relaxed - control points influence curve, but anchor direction is a soft guide (not enough degrees of freedom to enforce strictly)
```

---

## Implementation Phases

### Phase 1: AST and Parser Extensions

**Goal**: Parse anchor reference syntax in connections and anchor declarations in templates.

**Files Modified**:
- `src/parser/ast.rs`: Add `AnchorReference`, `AnchorDecl`, `AnchorPosition`
- `src/parser/grammar.rs`: Parse `element.anchor` syntax
- `src/parser/grammar.rs`: Parse `anchor name [position: ...]` statement

**Connection Syntax Parsing**:
```
Current: shape_a -> shape_b [modifiers]
New:     shape_a.anchor -> shape_b.anchor [modifiers]
         shape_a.anchor -> shape_b [modifiers]      // mixed
         shape_a -> shape_b [modifiers]             // unchanged (backward compat)
```

**Anchor Declaration Parsing**:
```
anchor input [position: body.left]
anchor output [position: body.right]
anchor mid_upper [position: body.top + 10]
```

**Tests**:
- Parse `box_a.right -> box_b.left`
- Parse `box_a.right -> box_b` (mixed)
- Parse `box_a -> box_b` (backward compat)
- Parse `anchor input [position: body.left]`
- Parse `anchor mid [position: body.center_y + 20]`
- Error on `box_a.invalid_anchor`
- Error on `anchor missing_position`

---

### Phase 2: Built-in Anchor Computation

**Goal**: Compute anchors for simple shapes and paths based on bounding boxes.

**Files Modified**:
- `src/layout/types.rs`: Add `Anchor`, `AnchorSet` types
- `src/layout/engine.rs`: Add `compute_anchors()` function

**Built-in Anchor Sets**:

| Shape Type | Anchors |
|------------|---------|
| rect, ellipse, circle | top, bottom, left, right |
| path | top, bottom, left, right, top_left, top_right, bottom_left, bottom_right |
| row, col, stack (named) | top, bottom, left, right (from container bounds) |

**Anchor Position and Direction Computation**:
```rust
fn compute_simple_anchors(bounds: &BoundingBox) -> AnchorSet {
    AnchorSet::from([
        Anchor::new("top",    bounds.top_center(),    AnchorDirection::Up),
        Anchor::new("bottom", bounds.bottom_center(), AnchorDirection::Down),
        Anchor::new("left",   bounds.left_center(),   AnchorDirection::Left),
        Anchor::new("right",  bounds.right_center(),  AnchorDirection::Right),
    ])
}

fn compute_path_anchors(bounds: &BoundingBox) -> AnchorSet {
    let mut anchors = compute_simple_anchors(bounds);
    // Corner anchors have diagonal directions (outward normal)
    anchors.insert(Anchor::new("top_left",     bounds.top_left(),     AnchorDirection::Angle(225.0)));
    anchors.insert(Anchor::new("top_right",    bounds.top_right(),    AnchorDirection::Angle(315.0)));
    anchors.insert(Anchor::new("bottom_left",  bounds.bottom_left(),  AnchorDirection::Angle(135.0)));
    anchors.insert(Anchor::new("bottom_right", bounds.bottom_right(), AnchorDirection::Angle(45.0)));
    anchors
}
```

**Tests**:
- Rect anchors computed correctly
- Circle anchors computed correctly (uses bounding box)
- Path anchors include corners
- Container anchors computed from child bounds

---

### Phase 3: Template Anchor Support

**Goal**: Allow templates to define custom named anchors.

**Files Modified**:
- `src/template/registry.rs`: Store anchor definitions in Template
- `src/template/resolver.rs`: Resolve anchors during template expansion
- `src/layout/engine.rs`: Compute template anchor positions

**Template Processing**:

1. **Parse Phase**: Collect `anchor` statements into template definition
2. **Expansion Phase**: Template anchors become positioned reference points
3. **Layout Phase**: Anchor positions resolved using constraint system (they participate like shapes)

**Template Anchor as Layout Element**:
Per clarification: template anchors are shapes from layouting perspective. They:
- Participate in constraint solving
- Can be positioned with constraints
- Have their final position used as connection endpoints

**Anchor Declaration Syntax with Direction**:
```
// Basic - direction inferred from property (left property → left direction)
anchor input [position: body.left]

// Explicit cardinal direction
anchor input [position: body.left, direction: left]

// Explicit angle (degrees, 0=right, 90=down, etc.)
anchor custom_port [position: body.center_y + 10, direction: 45]
```

**Direction Inference Rules**:
When `direction` is omitted, infer from the position property:
| Position Property | Inferred Direction |
|-------------------|-------------------|
| `.left` | Left |
| `.right` | Right |
| `.top` | Up |
| `.bottom` | Down |
| `.center_x`, `.center_y`, `.center` | Down (default) |
| Expression with offset | Same as base property |

**Example Template**:
```
template "server" {
  rect body [width: 80, height: 60, label: "Server"]
  anchor input [position: body.left]           // direction: left (inferred)
  anchor output [position: body.right]         // direction: right (inferred)
  anchor status [position: body.top, direction: up]  // explicit direction
  anchor debug [position: body.bottom + 5, direction: 135]  // diagonal angle
}
```

**Tests**:
- Template with custom anchors parses
- Template instance exposes anchors: `server1.input`
- Anchor positions computed after layout
- Anchor directions inferred correctly from position
- Explicit direction overrides inference
- Error on duplicate anchor names
- Error on invalid element reference in anchor position

---

### Phase 4: Connection Routing with Anchors

**Goal**: Update connection routing to use explicit anchor positions AND directions.

**Files Modified**:
- `src/layout/routing.rs`: Accept anchor positions and directions
- `src/layout/engine.rs`: Resolve anchor references before routing

**Updated Routing Flow**:

```rust
/// Resolved anchor information for routing
pub struct ResolvedAnchor {
    pub position: Point,
    pub direction: AnchorDirection,
}

fn route_connection(
    from_bounds: &BoundingBox,
    to_bounds: &BoundingBox,
    from_anchor: Option<ResolvedAnchor>,  // NEW: position + direction
    to_anchor: Option<ResolvedAnchor>,    // NEW: position + direction
    mode: RoutingMode,
    via_points: &[Point],
) -> Vec<Point>
```

**Direction-Aware Routing**:

For **orthogonal** routing with anchors:
```rust
// Connection must start/end perpendicular to anchor direction
// Example: from_anchor.direction = Right means first segment goes right

fn route_orthogonal_with_anchors(
    from: ResolvedAnchor,
    to: ResolvedAnchor,
) -> Vec<Point> {
    let mut path = vec![from.position];

    // First segment: extend in anchor direction
    let exit_length = 20.0; // configurable
    let exit_point = from.position + from.direction.to_vector() * exit_length;
    path.push(exit_point);

    // Route orthogonally to entry point
    let entry_length = 20.0;
    let entry_point = to.position + to.direction.to_vector() * entry_length;

    // Add intermediate segments as needed...
    // (existing orthogonal routing logic)

    path.push(entry_point);
    path.push(to.position);
    path
}
```

For **curved** routing with anchors:
- Direction provides a **hint** for control point placement
- Not strictly enforced (curves have limited degrees of freedom)
- Control point placed along exit direction from source anchor

For **direct** routing with anchors:
- Straight line, direction not used (would require bending)
- Could add small perpendicular stub at anchor if desired (optional enhancement)

**Anchor Resolution**:
```rust
fn resolve_anchor(
    element_id: &Identifier,
    anchor_name: Option<&str>,
    elements: &HashMap<String, ElementLayout>,
    anchors: &HashMap<String, AnchorSet>,
) -> Result<ResolvedAnchor, LayoutError> {
    let element = elements.get(element_id)?;
    match anchor_name {
        Some(name) => {
            let anchor_set = anchors.get(element_id)?;
            anchor_set.get(name)
                .map(|a| ResolvedAnchor { position: a.position, direction: a.direction })
                .ok_or_else(|| {
                    LayoutError::InvalidAnchor {
                        element: element_id.clone(),
                        anchor: name.to_string(),
                        valid: anchor_set.names(),
                    }
                })
        }
        None => {
            // Auto-detect: use center with direction toward target
            Ok(ResolvedAnchor {
                position: element.bounds.center(),
                direction: AnchorDirection::Down, // Or compute from target
            })
        }
    }
}
```

**Error Messages**:
```
error: Invalid anchor 'top_right' for rect shape 'box_a'
  --> example.ail:5:10
   |
 5 | box_a.top_right -> box_b
   |       ^^^^^^^^^ not a valid anchor
   |
   = help: Valid anchors for rect: top, bottom, left, right
```

**Tests**:
- Connection with both anchors uses exact points
- Connection with one anchor uses anchor + auto-detect
- Connection without anchors uses existing auto-detect
- Invalid anchor produces error with suggestions
- Anchors work with curved routing mode

---

### Phase 5: Nested Anchor Access (FR6.2)

**Goal**: Support accessing anchors through container paths: `container.element.anchor`

**Files Modified**:
- `src/parser/grammar.rs`: Parse multi-level dot notation
- `src/layout/engine.rs`: Resolve nested anchor paths

**Path Resolution**:
```rust
// Parse: container.element.anchor
// Resolve: find 'element' inside 'container', get its 'anchor'

fn resolve_anchor_path(
    path: &[Identifier],
    elements: &HashMap<String, ElementLayout>,
    anchors: &HashMap<String, AnchorSet>,
) -> Result<Point, LayoutError>
```

**Tests**:
- `diagram.box_a.top` resolves correctly
- `outer.inner.element.left` works for deep nesting
- Error on invalid path segment

---

### Phase 6: Integration, Examples, and Error Handling

**Goal**: Complete integration tests, update examples, polish error messages.

**Files Modified**:
- `examples/feedback-loops.ail`: Rewrite using anchors
- `examples/anchors-demo.ail`: New example demonstrating anchor features
- `tests/integration/`: Add anchor integration tests

**Updated feedback-loops Example**:
```ail
// Before (with invisible via points):
circle human_via [size: 1, fill: none, stroke: none]
assign -> evaluate [routing: curved, via: human_via]

// After (with anchors):
assign.top -> evaluate.top [routing: curved, via: human_via]
```

**New Example: anchors-demo.ail**:
```ail
// Basic anchor usage
rect box_a [width: 100, height: 60, label: "Box A"]
rect box_b [width: 100, height: 60, label: "Box B"]

constrain box_b.left = box_a.right + 50

box_a.right -> box_b.left

// Template with custom anchors
template "server" {
  rect body [width: 80, height: 60, label: "Server"]
  anchor input [position: body.left]
  anchor output [position: body.right]
}

server app
server db

constrain db.left = app.right + 100

app.output -> db.input
```

**Tests**:
- All existing connection tests still pass
- Anchor-based connections route correctly
- Template anchors accessible on instances
- Error messages include valid anchor suggestions
- Mixed anchor/auto-detect connections work

---

## Test Strategy

### Unit Tests
| Component | Test Coverage |
|-----------|---------------|
| Parser | Anchor reference parsing, anchor declaration parsing, backward compat |
| Layout | Anchor computation, anchor resolution, error handling |
| Routing | Anchor-based endpoint calculation |

### Integration Tests
| Test | Description |
|------|-------------|
| `anchor_basic` | Simple `box.right -> box.left` |
| `anchor_mixed` | `box.right -> box` (one explicit, one auto) |
| `anchor_template` | Template with custom anchors |
| `anchor_nested` | `container.element.anchor` path |
| `anchor_invalid` | Error message for invalid anchor |
| `anchor_backward_compat` | Existing `a -> b` syntax unchanged |

### Snapshot Tests
- SVG output for anchor-based connections
- Visual verification of anchor positions

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| AST breaking change | High | Medium | Carefully migrate ConnectionDecl; add backward-compat parsing |
| Template anchor resolution timing | Medium | Medium | Ensure anchors computed after full layout pass |
| Nested path resolution complexity | Low | Low | Limit nesting depth; clear errors for deep paths |

---

## Estimated Complexity

| Phase | Scope | Files | LOC Estimate |
|-------|-------|-------|--------------|
| Phase 1: AST/Parser | Medium | 2 | ~150 |
| Phase 2: Built-in Anchors | Small | 2 | ~100 |
| Phase 3: Template Anchors | Medium | 3 | ~120 |
| Phase 4: Routing Integration | Medium | 2 | ~100 |
| Phase 5: Nested Access | Small | 2 | ~60 |
| Phase 6: Integration | Small | 4+ | ~80 |
| **Total** | | **15** | **~610** |

---

## Success Metrics

1. **Backward Compatibility**: All existing .ail files parse and render unchanged
2. **Parser**: All anchor syntaxes parse correctly
3. **Layout**: Anchors computed accurately from bounding boxes
4. **Templates**: Custom anchors work as documented
5. **Errors**: Invalid anchors produce helpful suggestions
6. **Examples**: feedback-loops can use anchors for cleaner syntax
