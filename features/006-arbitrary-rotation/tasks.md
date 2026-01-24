# Tasks: Arbitrary Rotation of Any Shape

**Feature:** 006-arbitrary-rotation
**Generated:** 2026-01-24
**Spec:** [spec.md](./spec.md)
**Plan:** [plan.md](./plan.md)

---

## Overview

This feature adds rotation support to all shape types in the agent-illustrator DSL. The implementation threads through three layers: parser, layout types, and SVG renderer.

**Total Tasks:** 11
**Parallelizable:** 4 (marked with [P])
**Estimated Phases:** 3

---

## Phase 1: Parser Layer (Foundational)

These tasks must complete before any rendering work can begin.

### T001: Add StyleKey::Rotation variant to AST

**File:** `src/parser/ast.rs`
**Priority:** P0
**Depends on:** None

Add the `Rotation` variant to the `StyleKey` enum, before `Custom(String)`:

```rust
/// Rotation angle in degrees (clockwise positive)
Rotation,
```

Insert after `StrokeDasharray` and before `Custom(String)` in the enum definition (around line 281).

**Acceptance:** `cargo build` succeeds with new variant.

---

### T002: Add "rotation" keyword recognition to grammar

**File:** `src/parser/grammar.rs`
**Priority:** P0
**Depends on:** T001

Add `"rotation"` to the style key match block (around line 88-105):

```rust
"rotation" => StyleKey::Rotation,
```

Insert in alphabetical order with the other keys.

**Acceptance:** Parsing `rect foo [rotation: 45]` produces AST with `StyleKey::Rotation`.

---

### CHECKPOINT: Parser Layer Complete
- [ ] T001 complete
- [ ] T002 complete
- [ ] `cargo build` succeeds
- [ ] Can parse `[rotation: 45]` modifier

---

## Phase 2: Layout Types (Foundational)

These tasks enable rotation values to flow through the layout system.

### T003: Add rotation field to ResolvedStyles struct [P]

**File:** `src/layout/types.rs`
**Priority:** P0
**Depends on:** T001

Add `rotation` field to `ResolvedStyles` struct (around line 108-116):

```rust
/// Rotation angle in degrees (clockwise positive, 0 = no rotation)
pub rotation: Option<f64>,
```

Add after `css_classes` field.

**Acceptance:** Struct compiles with new field.

---

### T004: Extract rotation in from_modifiers()

**File:** `src/layout/types.rs`
**Priority:** P0
**Depends on:** T003

Add rotation extraction in `from_modifiers()` method. Add a new match arm in the match block (around line 185-195):

```rust
StyleKey::Rotation => {
    if let StyleValue::Number { value, .. } = &modifier.node.value.node {
        styles.rotation = Some(*value);
    }
}
```

**Acceptance:** `ResolvedStyles::from_modifiers()` correctly extracts rotation values.

---

### T005: Handle rotation in merge() and with_defaults()

**File:** `src/layout/types.rs`
**Priority:** P0
**Depends on:** T003

Update `merge()` method (around line 230-246) to include:

```rust
rotation: other.rotation.or(self.rotation),
```

Update `with_defaults()` method (around line 120-130) to include:

```rust
rotation: None,
```

**Acceptance:** Rotation merges correctly; defaults don't override explicit values.

---

### CHECKPOINT: Layout Types Complete
- [ ] T003-T005 complete
- [ ] `cargo build` succeeds
- [ ] Rotation flows from AST through layout types

---

## Phase 3: SVG Renderer (Core Feature)

These tasks implement the actual rotation rendering.

### T006: Add render_shape_with_rotation helper function

**File:** `src/renderer/svg.rs`
**Priority:** P1
**Depends on:** T003

Add a helper function before `render_element()` (around line 545):

```rust
/// Wrap shape rendering with rotation transform if needed
fn render_shape_with_rotation<F>(
    element: &ElementLayout,
    builder: &mut SvgBuilder,
    render_fn: F,
)
where
    F: FnOnce(&mut SvgBuilder),
{
    if let Some(rotation) = element.styles.rotation {
        if rotation.abs() > f64::EPSILON {
            let center = element.bounds.center();
            let transform = format!("rotate({} {} {})", rotation, center.x, center.y);
            builder.start_group_with_transform(None, &[], &transform);
            render_fn(builder);
            builder.end_group();
        } else {
            render_fn(builder);
        }
    } else {
        render_fn(builder);
    }
}
```

**Acceptance:** Helper function compiles.

---

### T007: Apply rotation wrapper in render_element for basic shapes

**File:** `src/renderer/svg.rs`
**Priority:** P1
**Depends on:** T006

Wrap each shape rendering call in `render_element()` with the rotation helper. For each shape type (Rectangle, Circle, Ellipse, Polygon, Line, Icon, Text), replace the direct builder call with:

```rust
ElementType::Shape(ShapeType::Rectangle) => {
    render_shape_with_rotation(element, builder, |b| {
        b.add_rect(
            id,
            element.bounds.x,
            element.bounds.y,
            element.bounds.width,
            element.bounds.height,
            &classes,
            &styles,
        );
    });
}
```

Apply similar pattern to: Circle, Ellipse, Polygon, Line, Icon, Text shapes.

**Acceptance:** Rotated shapes render with `<g transform="rotate(...)">` wrapper.

---

### T008: Handle rotation for SvgEmbed (template instances)

**File:** `src/renderer/svg.rs`
**Priority:** P1
**Depends on:** T006

Modify the `SvgEmbed` case in `render_element()` (around line 654-686) to compose rotation into the existing transform:

```rust
ElementType::Shape(ShapeType::SvgEmbed {
    content,
    intrinsic_width,
    intrinsic_height,
}) => {
    // ... existing prefix and embed_classes code ...

    let scale_x = intrinsic_width
        .map(|w| element.bounds.width / w)
        .unwrap_or(1.0);
    let scale_y = intrinsic_height
        .map(|h| element.bounds.height / h)
        .unwrap_or(1.0);

    // Compose transform with rotation if present
    let transform = if let Some(rotation) = element.styles.rotation {
        if rotation.abs() > f64::EPSILON {
            let cx = intrinsic_width.unwrap_or(element.bounds.width) / 2.0;
            let cy = intrinsic_height.unwrap_or(element.bounds.height) / 2.0;
            format!(
                "translate({}, {}) scale({}, {}) rotate({} {} {})",
                element.bounds.x, element.bounds.y, scale_x, scale_y, rotation, cx, cy
            )
        } else {
            format!(
                "translate({}, {}) scale({}, {})",
                element.bounds.x, element.bounds.y, scale_x, scale_y
            )
        }
    } else {
        format!(
            "translate({}, {}) scale({}, {})",
            element.bounds.x, element.bounds.y, scale_x, scale_y
        )
    };

    builder.start_group_with_transform(id, &embed_classes, &transform);
    // ... rest unchanged ...
}
```

**Acceptance:** Template instances with rotation render correctly.

---

### CHECKPOINT: Renderer Complete
- [ ] T006-T008 complete
- [ ] `cargo build` succeeds
- [ ] Rotated shapes produce correct SVG output
- [ ] Template instances with rotation work

---

## Phase 4: Tests

### T009: Add parser unit test for rotation [P]

**File:** `src/parser/grammar.rs`
**Priority:** P1
**Depends on:** T002

Add test in the tests module at the bottom of the file:

```rust
#[test]
fn test_parse_rotation_modifier() {
    let input = r#"rect box [rotation: 45]"#;
    let doc = parse(input).expect("should parse");
    assert_eq!(doc.statements.len(), 1);

    if let Statement::Shape(shape) = &doc.statements[0].node {
        assert_eq!(shape.modifiers.len(), 1);
        assert!(matches!(
            shape.modifiers[0].node.key.node,
            StyleKey::Rotation
        ));
        if let StyleValue::Number { value, .. } = &shape.modifiers[0].node.value.node {
            assert!((value - 45.0).abs() < f64::EPSILON);
        } else {
            panic!("Expected number value");
        }
    } else {
        panic!("Expected shape statement");
    }
}
```

**Acceptance:** Test passes.

---

### T010: Add ResolvedStyles unit test for rotation [P]

**File:** `src/layout/types.rs`
**Priority:** P1
**Depends on:** T004

Add test in the tests module at the bottom of the file:

```rust
#[test]
fn test_resolved_styles_rotation() {
    use crate::parser::ast::{StyleKey, StyleModifier, StyleValue, Spanned};

    let modifiers = vec![
        Spanned::new(
            StyleModifier {
                key: Spanned::new(StyleKey::Rotation, 0..8),
                value: Spanned::new(StyleValue::Number { value: 45.0, unit: None }, 10..12),
            },
            0..12,
        ),
    ];

    let styles = ResolvedStyles::from_modifiers(&modifiers);
    assert_eq!(styles.rotation, Some(45.0));
}
```

**Acceptance:** Test passes.

---

### T011: Add integration tests for rotation [P]

**File:** `tests/integration_tests.rs`
**Priority:** P1
**Depends on:** T007, T008

Add integration tests at the end of the file:

```rust
#[test]
fn test_rotation_rect() {
    let input = r#"rect box [rotation: 45]"#;
    let result = agent_illustrator::render(input).expect("should render");
    assert!(result.contains("transform="));
    assert!(result.contains("rotate(45"));
}

#[test]
fn test_rotation_negative() {
    let input = r#"rect box [rotation: -30]"#;
    let result = agent_illustrator::render(input).expect("should render");
    assert!(result.contains("rotate(-30"));
}

#[test]
fn test_rotation_text() {
    let input = r#"text label [rotation: 90] "Hello""#;
    let result = agent_illustrator::render(input).expect("should render");
    assert!(result.contains("rotate(90"));
}

#[test]
fn test_rotation_in_layout() {
    let input = r#"
        row container {
            rect a [rotation: 15]
            rect b
        }
    "#;
    let result = agent_illustrator::render(input).expect("should render");
    assert!(result.contains("rotate(15"));
    // Verify non-rotated shape has no transform wrapper
    assert!(result.matches("rotate(").count() == 1);
}

#[test]
fn test_rotation_with_connection() {
    let input = r#"
        rect a [rotation: 45]
        rect b [x: 100]
        a -> b
    "#;
    let result = agent_illustrator::render(input).expect("should render");
    assert!(result.contains("rotate(45"));
    // Connection should still render
    assert!(result.contains("ai-connection"));
}

#[test]
fn test_rotation_zero_no_transform() {
    let input = r#"rect box [rotation: 0]"#;
    let result = agent_illustrator::render(input).expect("should render");
    // Zero rotation should not add a transform wrapper
    assert!(!result.contains("rotate(0"));
}
```

**Acceptance:** All tests pass.

---

### CHECKPOINT: Tests Complete
- [ ] T009-T011 complete
- [ ] `cargo test` passes (all 155+ existing tests + new tests)
- [ ] No regressions in existing functionality

---

## Final Validation

- [ ] `cargo build` succeeds
- [ ] `cargo test` passes
- [ ] `cargo clippy` has no warnings
- [ ] Create example file with rotated shapes and verify visual output
- [ ] Non-rotated shapes render identically to before (regression check)

---

## Dependency Graph

```
T001 (AST)
  ├── T002 (Grammar) ────────────────────────────────────────┐
  │     └── T009 (Parser test) [P]                           │
  │                                                          │
  └── T003 (ResolvedStyles) ─────────────────────────────────┤
        ├── T004 (from_modifiers)                            │
        │     └── T010 (Styles test) [P]                     │
        ├── T005 (merge/defaults)                            │
        │                                                    │
        └── T006 (rotation helper) ──────────────────────────┤
              ├── T007 (apply to shapes) ────────────────────┤
              │     └── T011 (Integration tests) [P] ────────┘
              └── T008 (SvgEmbed) ───────────────────────────┘
```

---

## Parallel Execution Strategy

**Wave 1:** T001 (foundation)
**Wave 2:** T002, T003 (can run in parallel - different files)
**Wave 3:** T004, T005 (same file, sequential)
**Wave 4:** T006 (depends on T003)
**Wave 5:** T007, T008 (can run in parallel - different parts of same file, but better sequential)
**Wave 6:** T009, T010, T011 (all tests can run in parallel)

**Optimal execution order:** T001 → T002+T003 → T004 → T005 → T006 → T007 → T008 → T009+T010+T011

---

## Notes

- This is a rendering-only feature; layout engine and constraint solver are unchanged
- Connections attach to unrotated bounding boxes (per spec FR4)
- Zero rotation (or epsilon-close) should not add a transform wrapper
- Template instances require composed transforms (translate + scale + rotate)
