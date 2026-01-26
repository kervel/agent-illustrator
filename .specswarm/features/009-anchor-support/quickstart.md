# Quickstart: Anchor Support Implementation

## Overview

This guide helps you get started implementing Feature 009: Anchor Support for Shape Connections.

---

## Key Concepts

### What are Anchors?

Anchors are named attachment points on shapes where connectors can attach:

```
         top
          ↓
    ┌─────┼─────┐
    │           │
left│   RECT    │right
    │           │
    └─────┼─────┘
          ↑
        bottom
```

### Anchor Properties

Each anchor has:
- **Position**: (x, y) coordinate on the shape boundary
- **Direction**: Outward normal (connectors approach perpendicular to shape)

### Syntax Examples

```ail
// Basic anchor usage
box_a.right -> box_b.left

// Mixed (one anchor, one auto-detect)
box_a.right -> box_b

// Template with custom anchors
template "server" {
  rect body [width: 80, height: 60]
  anchor input [position: body.left]
  anchor output [position: body.right]
}

server app
server db
app.output -> db.input
```

---

## Implementation Order

### Phase 1: AST & Parser
**Files**: `src/parser/ast.rs`, `src/parser/grammar.rs`

1. Add `AnchorReference` struct to AST
2. Update `ConnectionDecl` to use `AnchorReference`
3. Add `AnchorDecl` for template anchor declarations
4. Parse `element.anchor` dot notation

**Test**: `box_a.right -> box_b.left` parses correctly

### Phase 2: Built-in Anchors
**Files**: `src/layout/types.rs`, `src/layout/engine.rs`

1. Add `Anchor`, `AnchorSet`, `AnchorDirection` types
2. Implement `AnchorSet::simple_shape()` and `AnchorSet::path_shape()`
3. Compute anchors in layout pass after bounding boxes resolved

**Test**: Rect element has `top`, `bottom`, `left`, `right` anchors

### Phase 3: Template Anchors
**Files**: `src/template/registry.rs`, `src/template/resolver.rs`

1. Store `AnchorDecl` in template definition
2. Resolve anchor positions during template expansion
3. Make template anchors accessible on instances

**Test**: `server1.input` resolves to template-defined anchor

### Phase 4: Connection Routing
**Files**: `src/layout/routing.rs`, `src/layout/engine.rs`

1. Resolve `AnchorReference` to `ResolvedAnchor`
2. Update `route_connection()` to use anchor positions and directions
3. Ensure orthogonal routing starts/ends perpendicular to anchor direction

**Test**: `box_a.right -> box_b.left` routes with horizontal exit/entry

### Phase 5: Nested Access (Optional)
**Files**: `src/parser/grammar.rs`, `src/layout/engine.rs`

1. Parse multi-level paths: `container.element.anchor`
2. Resolve nested element references

**Test**: `diagram.box_a.top` works

### Phase 6: Integration
**Files**: `examples/`, `tests/`

1. Update `feedback-loops.ail` to use anchors
2. Add `anchors-demo.ail` example
3. Verify backward compatibility

---

## Critical Code Locations

| Component | File | Key Function/Type |
|-----------|------|-------------------|
| Connection AST | `src/parser/ast.rs` | `ConnectionDecl` |
| Connection parsing | `src/parser/grammar.rs` | `connection()` |
| Layout types | `src/layout/types.rs` | `ElementLayout`, `ConnectionLayout` |
| Routing | `src/layout/routing.rs` | `route_connection()`, `best_edges()` |
| Endpoint calc | `src/layout/routing.rs` | `attachment_point()`, `boundary_point_toward()` |
| Template storage | `src/template/registry.rs` | `Template` struct |
| Template expansion | `src/template/resolver.rs` | `resolve_template()` |

---

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_simple_shape_anchors() {
    let bounds = BoundingBox::new(0.0, 0.0, 100.0, 50.0);
    let anchors = AnchorSet::simple_shape(&bounds);

    assert_eq!(anchors.get("top").unwrap().position, Point::new(50.0, 0.0));
    assert_eq!(anchors.get("bottom").unwrap().position, Point::new(50.0, 50.0));
    assert_eq!(anchors.get("left").unwrap().position, Point::new(0.0, 25.0));
    assert_eq!(anchors.get("right").unwrap().position, Point::new(100.0, 25.0));
}

#[test]
fn test_anchor_directions() {
    let bounds = BoundingBox::new(0.0, 0.0, 100.0, 50.0);
    let anchors = AnchorSet::simple_shape(&bounds);

    assert_eq!(anchors.get("top").unwrap().direction, AnchorDirection::Up);
    assert_eq!(anchors.get("right").unwrap().direction, AnchorDirection::Right);
}
```

### Integration Tests

```rust
#[test]
fn test_anchor_connection_parsing() {
    let input = "rect a\nrect b\na.right -> b.left";
    let ast = parse(input).unwrap();
    // Verify connection has anchor references
}

#[test]
fn test_anchor_connection_routing() {
    let input = r#"
        rect a [width: 100, height: 50]
        rect b [width: 100, height: 50]
        constrain b.left = a.right + 50
        a.right -> b.left
    "#;
    let result = render(input).unwrap();
    // Verify connection starts at (100, 25) and ends at (150, 25)
}
```

---

## Common Pitfalls

1. **Backward Compatibility**: Ensure `a -> b` (no anchors) still works
2. **Timing**: Anchors depend on bounding boxes - compute after layout resolution
3. **Template Scope**: Template anchors reference template-internal elements only
4. **Reserved Names**: Don't allow custom anchors named `top`/`bottom`/`left`/`right`

---

## Quick Reference: Anchor Directions

| Anchor | Direction | Angle |
|--------|-----------|-------|
| top | Up | 270° |
| bottom | Down | 90° |
| left | Left | 180° |
| right | Right | 0° |
| top_left | Diagonal | 225° |
| top_right | Diagonal | 315° |
| bottom_left | Diagonal | 135° |
| bottom_right | Diagonal | 45° |
