# Implementation Plan: Arbitrary Rotation of Any Shape

**Feature:** 006-arbitrary-rotation
**Spec:** [spec.md](./spec.md)
**Created:** 2026-01-24

## Technical Context

- **Language:** Rust 2021 edition
- **Parser:** chumsky 1.0.0-alpha.7 with logos 0.14 lexer
- **Rendering:** Custom SVG builder
- **No external dependencies required:** This feature adds a new modifier keyword and SVG transform attribute

## Constitution Check

No `.specswarm/constitution.md` exists. Proceeding without constitution gates.

## Implementation Strategy

This is a straightforward feature that threads through three layers:

1. **Parser layer:** Add `Rotation` variant to `StyleKey` enum
2. **Layout layer:** Add `rotation` field to `ResolvedStyles`, pass it through layout engine
3. **Renderer layer:** Output SVG `transform="rotate(angle cx cy)"` attribute

The feature is designed as rendering-only (per spec FR3), so the layout engine and constraint solver remain unmodified.

## Detailed Changes

### Phase 1: AST and Parser (P0)

#### File: `src/parser/ast.rs`

Add `Rotation` variant to the `StyleKey` enum:

```rust
pub enum StyleKey {
    // ... existing variants ...
    /// Rotation angle in degrees (clockwise positive)
    Rotation,
    Custom(String),
}
```

#### File: `src/parser/grammar.rs`

Add "rotation" to the style key match (around line 88-105):

```rust
let style_key = choice((
    // ... existing patterns ...
    identifier.map(|id| {
        let key = match id.node.as_str() {
            // ... existing keys ...
            "rotation" => StyleKey::Rotation,
            other => StyleKey::Custom(other.to_string()),
        };
        Spanned::new(key, id.span)
    }),
));
```

### Phase 2: Layout Types (P0)

#### File: `src/layout/types.rs`

Add `rotation` field to `ResolvedStyles`:

```rust
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ResolvedStyles {
    pub fill: Option<String>,
    pub stroke: Option<String>,
    pub stroke_width: Option<f64>,
    pub stroke_dasharray: Option<String>,
    pub opacity: Option<f64>,
    pub font_size: Option<f64>,
    pub css_classes: Vec<String>,
    /// Rotation angle in degrees (clockwise positive, 0 = no rotation)
    pub rotation: Option<f64>,
}
```

Update `from_modifiers()` to extract rotation:

```rust
StyleKey::Rotation => {
    if let StyleValue::Number { value, .. } = &modifier.node.value.node {
        styles.rotation = Some(*value);
    }
}
```

Update `merge()` to handle rotation:

```rust
rotation: other.rotation.or(self.rotation),
```

Update `with_defaults()` to include rotation (set to `None` since 0 is the default):

```rust
rotation: None,
```

### Phase 3: SVG Renderer (P1)

#### File: `src/renderer/svg.rs`

Modify `render_element()` to apply rotation transform when present.

For shapes with rotation, wrap them in a `<g>` element with a `transform` attribute, or apply the transform directly to the shape.

**Approach A (Direct transform on shapes):** Add transform attribute directly to each shape.

**Approach B (Wrapper group):** Wrap rotated shapes in `<g transform="...">`.

Given SVG semantics and the existing pattern of `start_group_with_transform()`, Approach B is cleaner and already supported.

However, for simplicity and to avoid changing element structure, we'll use **Approach A**: add transform attribute directly to shapes.

Modify `add_rect`, `add_circle`, `add_ellipse`, `add_polygon`, `add_line`, `add_text_element` to accept an optional transform parameter, OR compute transform in `render_element` and pass it via the styles string.

**Simplest approach:** Add transform as part of the styles string in `format_styles()`.

Update `format_styles()`:

```rust
fn format_styles_with_rotation(styles: &ResolvedStyles, center: Point) -> String {
    let mut parts = vec![];
    // ... existing style formatting ...

    // Add rotation transform if present
    if let Some(rotation) = styles.rotation {
        if rotation.abs() > f64::EPSILON {
            parts.push(format!(
                r#" transform="rotate({} {} {})""#,
                rotation, center.x, center.y
            ));
        }
    }
    parts.join("")
}
```

But this requires passing the center point. Alternative: compute center in `render_element` and pass to a modified format function.

**Final approach:** Create a helper function that wraps shape rendering with rotation when needed:

```rust
fn render_shape_with_rotation(
    element: &ElementLayout,
    builder: &mut SvgBuilder,
    render_fn: impl FnOnce(&mut SvgBuilder),
) {
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

This approach:
- Wraps rotated shapes in a group with transform
- Leaves non-rotated shapes unchanged
- Works for all shape types uniformly

#### Special Case: SvgEmbed (Template Instances)

Template instances render as `SvgEmbed` shapes which already use a `transform` for positioning and scaling:

```rust
let transform = format!(
    "translate({}, {}) scale({}, {})",
    element.bounds.x, element.bounds.y, scale_x, scale_y
);
```

For rotated template instances, we need to **compose** the transforms. SVG transform operations are applied right-to-left, so:

```rust
// rotate around center, then scale, then translate
let transform = if let Some(rotation) = element.styles.rotation {
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
};
```

The rotation is applied last (first in the string) around the scaled center point.

### Phase 4: Tests (P1)

#### File: `tests/integration_tests.rs`

Add integration tests:

1. **test_rotation_rect** - Rectangle with rotation modifier
2. **test_rotation_negative** - Negative rotation angle
3. **test_rotation_text** - Text with rotation
4. **test_rotation_in_layout** - Rotated shape inside layout container
5. **test_rotation_with_connection** - Connection to rotated shape
6. **test_rotation_template_instance** - Rotated template instantiation

#### File: `src/layout/types.rs` (tests module)

Add unit test for `ResolvedStyles::from_modifiers()` with rotation.

## Task Breakdown

| # | Task | File(s) | Depends On |
|---|------|---------|------------|
| 1 | Add `StyleKey::Rotation` variant | `src/parser/ast.rs` | - |
| 2 | Add "rotation" to grammar style key match | `src/parser/grammar.rs` | 1 |
| 3 | Add `rotation` field to `ResolvedStyles` | `src/layout/types.rs` | 1 |
| 4 | Extract rotation in `from_modifiers()` | `src/layout/types.rs` | 3 |
| 5 | Handle rotation in `merge()` and `with_defaults()` | `src/layout/types.rs` | 3 |
| 6 | Add `render_shape_with_rotation` helper | `src/renderer/svg.rs` | 3 |
| 7 | Apply rotation wrapper in `render_element` | `src/renderer/svg.rs` | 6 |
| 7a | Handle rotation for SvgEmbed (template instances) | `src/renderer/svg.rs` | 6 |
| 8 | Add parser unit test | `src/parser/grammar.rs` | 2 |
| 9 | Add ResolvedStyles unit test | `src/layout/types.rs` | 4 |
| 10 | Add integration tests | `tests/integration_tests.rs` | 7 |

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| SVG transform affects child elements unexpectedly | Low | Low | Use wrapper groups only for leaf shapes |
| Connection routing breaks with rotated shapes | Low | Low | Spec explicitly says connections use unrotated bounds |
| Text rotation readability issues | Low | Low | User responsibility; behavior documented |

## Validation Criteria

- [ ] `cargo build` succeeds
- [ ] `cargo test` passes (all existing + new tests)
- [ ] `cargo clippy` has no warnings
- [ ] Example with rotated shapes renders correctly
- [ ] Non-rotated shapes render identically to before (regression test)

## Files Modified

1. `src/parser/ast.rs` - Add StyleKey::Rotation
2. `src/parser/grammar.rs` - Recognize "rotation" keyword
3. `src/layout/types.rs` - Add rotation to ResolvedStyles
4. `src/renderer/svg.rs` - Apply SVG transform
5. `tests/integration_tests.rs` - Add test cases
