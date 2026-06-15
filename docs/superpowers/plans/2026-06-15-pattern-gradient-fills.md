# Pattern & Gradient Fills Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add named pattern fills (`hatch`, `cross_hatch`, `dots`, `grid`) and gradient fills (`gradient`, `radial_gradient`) to AIL, plus document the already-working dashed-stroke support.

**Architecture:** Approach A — additive. A new `StyleValue::Call` AST variant carries function-style fill values parsed in the grammar (with name/arity validation there). `ResolvedStyles` gains an additive `fill_pattern: Option<FillSpec>` field built in `from_modifiers`. At render time `SvgBuilder::register_fill` emits a deterministic, content-addressed `<pattern>`/`<linearGradient>`/`<radialGradient>` def once and returns a `url(#id)` that overrides the shape's `fill` attribute. Solid fills are untouched.

**Tech Stack:** Rust, chumsky parser combinators, SVG output. Tests via `cargo test`. Examples re-rendered by `examples/render-all.sh` (pre-commit hook).

---

## File Structure

- `src/parser/ast.rs` — add `StyleValue::Call { name, args }` variant.
- `src/parser/grammar.rs` — parse + validate function-call style values.
- `src/layout/types.rs` — `FillSpec`/`PatternKind`/`GradientKind` enums, `fill_pattern` field, `from_modifiers`/`merge`/`with_defaults` updates, color-arg resolution.
- `src/renderer/svg.rs` — `fill_def_ids` field, `register_fill`, `build_fill_def`, `format_styles` override param + call site.
- `examples/pattern-fills.ail` — new example.
- `docs/grammar.md`, `docs/skill-styling.md`, `docs/skill.md` — docs (incl. stroke dash).

---

## Task 1: AST — `StyleValue::Call` variant

**Files:**
- Modify: `src/parser/ast.rs:391-405` (the `StyleValue` enum)

- [ ] **Step 1: Add the variant**

In `src/parser/ast.rs`, add a `Call` variant to the `StyleValue` enum (after `List`):

```rust
/// Style values
#[derive(Debug, Clone, PartialEq)]
pub enum StyleValue {
    Color(ColorValue),
    Number {
        value: f64,
        unit: Option<String>,
    },
    String(String),
    Keyword(String),
    /// Identifier reference (for `[label: my_shape]` syntax)
    Identifier(Identifier),
    /// List of identifiers (for `[via: c1, c2, c3]` syntax - Feature 008)
    IdentifierList(Vec<Identifier>),
    /// Bracketed list of values (e.g. `at: [1, 0]`, `col_labels: ["a", "b"]`)
    List(Vec<Spanned<StyleValue>>),
    /// Function-style value: `hatch(accent-1)`, `gradient(a, b, 90)`.
    /// Used for pattern/gradient fills. `name` is the function name; `args`
    /// are color/number atoms.
    Call {
        name: String,
        args: Vec<Spanned<StyleValue>>,
    },
}
```

- [ ] **Step 2: Build to verify it compiles**

Run: `cargo build`
Expected: PASS. (Match arms over `StyleValue` elsewhere use a catch-all `_` or already cover relevant variants; if the compiler reports a non-exhaustive match, add a `StyleValue::Call { .. } => {}` arm that mirrors the neighboring no-op arm. Note any such site in the commit.)

- [ ] **Step 3: Commit**

```bash
git add src/parser/ast.rs
git commit -m "feat(ast): add StyleValue::Call variant for fill functions"
```

---

## Task 2: Grammar — parse and validate function-call fill values

**Files:**
- Modify: `src/parser/grammar.rs:325-336` (after `value_atom` / `value_list`, before `style_value`)
- Test: `src/parser/grammar.rs` (in the existing `#[cfg(test)] mod tests` at the bottom of the file)

The grammar validates the function name and argument shape here (mirrors how `symbolic_color` validates via `try_map`), so `from_modifiers` in Task 3 can stay infallible.

- [ ] **Step 1: Write the failing tests**

Find the test module in `src/parser/grammar.rs` (search for `mod tests`). Add these tests. Use whatever parse helper the neighboring tests use — search the module for an existing `fn parse_*` helper that returns the parsed program/statements; if tests assert on a `Document`/`Program`, follow that exact pattern. The assertions below assume a helper `parse_ok(src) -> Document` that panics on parse error and a `parse_err(src) -> bool` returning true when parsing fails. If the module names them differently, adapt the call but keep the assertions.

```rust
#[test]
fn parse_fill_hatch_no_args() {
    // `fill: hatch` parses as a bare identifier (Call requires parens);
    // it becomes a pattern in resolution (Task 3), not here.
    let doc = parse_ok("rect a [fill: hatch]\n");
    // Just assert it parses without error.
    let _ = doc;
}

#[test]
fn parse_fill_hatch_with_color() {
    let doc = parse_ok("rect a [fill: hatch(accent-1)]\n");
    let _ = doc;
}

#[test]
fn parse_fill_gradient_three_args() {
    let doc = parse_ok("rect a [fill: gradient(blue, white, 90)]\n");
    let _ = doc;
}

#[test]
fn parse_fill_radial_gradient() {
    let doc = parse_ok("rect a [fill: radial_gradient(white, accent-1)]\n");
    let _ = doc;
}

#[test]
fn parse_fill_unknown_function_errors() {
    assert!(parse_err("rect a [fill: bogus(blue, white)]\n"));
}

#[test]
fn parse_fill_gradient_one_stop_errors() {
    assert!(parse_err("rect a [fill: gradient(blue)]\n"));
}

#[test]
fn parse_fill_pattern_too_many_args_errors() {
    assert!(parse_err("rect a [fill: hatch(a, b, c)]\n"));
}
```

If no `parse_ok`/`parse_err` helpers exist, add these minimal helpers at the top of the test module, adapting the parser entry point to the one the other tests call (search the module for the function used to parse, e.g. `parse_document`, `program_parser().parse(...)`):

```rust
#[allow(dead_code)]
fn parse_ok(src: &str) -> bool {
    // Replace `parse_program` with the actual entry point used by sibling tests.
    crate::parser::parse(src).is_ok()
}
#[allow(dead_code)]
fn parse_err(src: &str) -> bool {
    crate::parser::parse(src).is_err()
}
```
(Then change `let doc = parse_ok(...)` lines above to `assert!(parse_ok(...))`.)

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib parse_fill`
Expected: The error-case tests FAIL (parser currently accepts `bogus(...)` as identifier + leftover, or errors differently), and the `with_color`/`gradient` tests FAIL (function-call syntax not yet parsed — `(` after identifier is unexpected).

- [ ] **Step 3: Add the call-value parser**

In `src/parser/grammar.rs`, after the `value_list` definition (ends at line ~334) and before `let style_value = ...` (line ~336), insert:

```rust
    // Function-style values for pattern/gradient fills, e.g. `hatch(accent-1)`,
    // `gradient(blue, white, 90)`. Name + arity are validated here so that
    // style resolution can stay infallible. A bare `hatch` (no parens) is NOT
    // matched here — it falls through to `value_atom` as an identifier and is
    // interpreted as a default-colored pattern during resolution.
    let call_value = identifier
        .then(
            value_atom
                .clone()
                .separated_by(just(Token::Comma))
                .allow_trailing()
                .collect::<Vec<_>>()
                .delimited_by(just(Token::ParenOpen), just(Token::ParenClose)),
        )
        .try_map(|(name, args), span| {
            let n = name.node.0.as_str();
            let argc = args.len();
            let ok = match n {
                "hatch" | "cross_hatch" | "dots" | "grid" => argc <= 2,
                "gradient" => argc == 2 || argc == 3,
                "radial_gradient" => argc == 2,
                _ => {
                    return Err(Rich::custom(
                        span,
                        format!(
                            "unknown fill function `{}`; expected one of: \
                             hatch, cross_hatch, dots, grid, gradient, radial_gradient",
                            n
                        ),
                    ))
                }
            };
            if !ok {
                return Err(Rich::custom(
                    span,
                    format!("`{}` got {} argument(s); patterns take 0-2 colors, \
                             gradient takes 2 colors + optional angle, \
                             radial_gradient takes 2 colors", n, argc),
                ));
            }
            Ok(StyleValue::Call {
                name: name.node.0.clone(),
                args,
            })
        })
        .map_with(|v, e| Spanned::new(v, span_range(&e.span())))
        .boxed();
```

Then change the `style_value` line (currently `let style_value = choice((value_list, value_atom)).boxed();`) to put `call_value` before `value_atom`:

```rust
    let style_value = choice((value_list, call_value, value_atom)).boxed();
```

Note: `value_atom` is already `.boxed()` and cloned elsewhere, so `value_atom.clone()` is valid. `Rich` is already imported (used by `symbolic_color`); if not, add `use chumsky::error::Rich;` matching the existing import style.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib parse_fill`
Expected: PASS for all seven tests.

- [ ] **Step 5: Full parser test sweep (no regressions)**

Run: `cargo test --lib parser`
Expected: PASS. (Confirms `call_value` ordering didn't break bare identifiers / lists.)

- [ ] **Step 6: Commit**

```bash
git add src/parser/grammar.rs
git commit -m "feat(parser): parse + validate function-style fill values"
```

---

## Task 3: Style model — `FillSpec` and resolution

**Files:**
- Modify: `src/layout/types.rs:522-538` (`ResolvedStyles` struct), `:547-560` (`with_defaults`), `:566-656` (`from_modifiers`, the `StyleKey::Fill` arm), `:683-703` (`merge`)
- Test: `src/layout/types.rs` (existing test module, or add one)

- [ ] **Step 1: Write the failing tests**

In `src/layout/types.rs`, find or create the `#[cfg(test)] mod tests`. Add:

```rust
#[cfg(test)]
mod fill_spec_tests {
    use super::*;
    use crate::parser::ast::{Identifier, StyleKey, StyleModifier, StyleValue, Spanned as _};
    // NOTE: adapt the Spanned import/construction to match how other tests in
    // this file build Spanned<StyleModifier>. Search this file for an existing
    // helper that wraps a node in a default span and reuse it.

    fn span0<T>(node: T) -> crate::parser::ast::Spanned<T> {
        crate::parser::ast::Spanned::new(node, 0..0)
    }

    fn fill_modifier(value: StyleValue) -> Vec<crate::parser::ast::Spanned<StyleModifier>> {
        vec![span0(StyleModifier {
            key: span0(StyleKey::Fill),
            value: span0(value),
        })]
    }

    #[test]
    fn bare_hatch_becomes_default_pattern() {
        let mods = fill_modifier(StyleValue::Identifier(Identifier::new("hatch")));
        let s = ResolvedStyles::from_modifiers(&mods);
        match s.fill_pattern {
            Some(FillSpec::Pattern { kind, fg, bg }) => {
                assert_eq!(kind, PatternKind::Hatch);
                assert_eq!(fg, "var(--foreground-2)");
                assert_eq!(bg, "transparent");
            }
            other => panic!("expected Pattern, got {:?}", other),
        }
        assert_eq!(s.fill, None);
    }

    #[test]
    fn hatch_with_one_color() {
        let mods = fill_modifier(StyleValue::Call {
            name: "hatch".into(),
            args: vec![span0(StyleValue::Identifier(Identifier::new("red")))],
        });
        let s = ResolvedStyles::from_modifiers(&mods);
        match s.fill_pattern {
            Some(FillSpec::Pattern { fg, bg, .. }) => {
                assert_eq!(fg, "red");
                assert_eq!(bg, "transparent");
            }
            other => panic!("expected Pattern, got {:?}", other),
        }
    }

    #[test]
    fn gradient_with_angle() {
        let mods = fill_modifier(StyleValue::Call {
            name: "gradient".into(),
            args: vec![
                span0(StyleValue::Keyword("blue".into())),
                span0(StyleValue::Keyword("white".into())),
                span0(StyleValue::Number { value: 90.0, unit: None }),
            ],
        });
        let s = ResolvedStyles::from_modifiers(&mods);
        match s.fill_pattern {
            Some(FillSpec::Gradient { kind, from, to, angle }) => {
                assert_eq!(kind, GradientKind::Linear);
                assert_eq!(from, "blue");
                assert_eq!(to, "white");
                assert_eq!(angle, 90.0);
            }
            other => panic!("expected Gradient, got {:?}", other),
        }
    }

    #[test]
    fn solid_fill_unchanged() {
        let mods = fill_modifier(StyleValue::Keyword("blue".into()));
        let s = ResolvedStyles::from_modifiers(&mods);
        assert_eq!(s.fill, Some("blue".to_string()));
        assert!(s.fill_pattern.is_none());
    }
}
```

(Adapt `Spanned::new(node, 0..0)` and `Identifier::new` to the real constructors — search `src/parser/ast.rs` and existing tests for their signatures. `Identifier::new` is used in `grammar.rs`, so it exists.)

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib fill_spec_tests`
Expected: FAIL to compile — `FillSpec`, `PatternKind`, `GradientKind`, and `fill_pattern` don't exist yet.

- [ ] **Step 3: Add the enums and the field**

In `src/layout/types.rs`, above the `ResolvedStyles` struct (before line 522), add:

```rust
/// A non-solid fill: a tiling pattern or a gradient.
#[derive(Debug, Clone, PartialEq)]
pub enum FillSpec {
    Pattern {
        kind: PatternKind,
        /// CSS color string for the pattern marks (already resolved, e.g. `var(--accent-1)`).
        fg: String,
        /// CSS color string for the tile background; `transparent` when omitted.
        bg: String,
    },
    Gradient {
        kind: GradientKind,
        from: String,
        to: String,
        /// Linear gradient angle in degrees: 0 = top->bottom, 90 = left->right.
        /// Ignored for radial.
        angle: f64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternKind {
    Hatch,
    CrossHatch,
    Dots,
    Grid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradientKind {
    Linear,
    Radial,
}
```

Add the field to `ResolvedStyles` (after `pub fill: Option<String>,` at line 525):

```rust
    pub fill: Option<String>,
    /// Non-solid fill (pattern/gradient). When `Some`, takes precedence over `fill`
    /// and is rendered as an SVG `<defs>` entry referenced via `url(#id)`.
    pub fill_pattern: Option<FillSpec>,
```

In `with_defaults` (line ~548), add `fill_pattern: None,` to the struct literal (after `fill: Some(...)`).

In `merge` (line ~684), add to the returned struct literal:

```rust
            fill_pattern: other
                .fill_pattern
                .clone()
                .or_else(|| self.fill_pattern.clone()),
```

- [ ] **Step 4: Implement resolution in `from_modifiers`**

Add a `FillSpec` constructor. Place this in the `impl ResolvedStyles` block (e.g. after `color_to_css`):

```rust
    /// Build a FillSpec from a `fill:` style value, or `None` if the value is a
    /// solid color. The grammar has already validated function name + arity, so
    /// this method trusts the shape and applies defaults.
    fn fill_spec_from_value(value: &StyleValue) -> Option<FillSpec> {
        // A color argument -> CSS string, with a default fallback.
        fn color_or(args: &[Spanned<StyleValue>], idx: usize, default: &str) -> String {
            args.get(idx)
                .and_then(|a| ResolvedStyles::color_to_css(&a.node))
                .unwrap_or_else(|| default.to_string())
        }
        let (name, args): (&str, &[Spanned<StyleValue>]) = match value {
            StyleValue::Call { name, args } => (name.as_str(), args.as_slice()),
            // Bare `fill: hatch` etc. (no parens) arrives as an identifier/keyword.
            StyleValue::Identifier(id) => (id.0.as_str(), &[]),
            StyleValue::Keyword(k) => (k.as_str(), &[]),
            _ => return None,
        };
        let pattern = |kind| {
            Some(FillSpec::Pattern {
                kind,
                fg: color_or(args, 0, "var(--foreground-2)"),
                bg: color_or(args, 1, "transparent"),
            })
        };
        match name {
            "hatch" => pattern(PatternKind::Hatch),
            "cross_hatch" => pattern(PatternKind::CrossHatch),
            "dots" => pattern(PatternKind::Dots),
            "grid" => pattern(PatternKind::Grid),
            "gradient" | "radial_gradient" => {
                let kind = if name == "gradient" {
                    GradientKind::Linear
                } else {
                    GradientKind::Radial
                };
                let angle = match args.get(2).map(|a| &a.node) {
                    Some(StyleValue::Number { value, .. }) => *value,
                    _ => 0.0,
                };
                Some(FillSpec::Gradient {
                    kind,
                    from: color_or(args, 0, "var(--foreground-2)"),
                    to: color_or(args, 1, "var(--background-1)"),
                    angle,
                })
            }
            _ => None, // solid color name (e.g. "red") — not a fill function
        }
    }
```

Then change the `StyleKey::Fill` arm in `from_modifiers` (lines 571-573) from:

```rust
                StyleKey::Fill => {
                    styles.fill = Self::color_to_css(&modifier.node.value.node);
                }
```

to:

```rust
                StyleKey::Fill => {
                    if let Some(spec) = Self::fill_spec_from_value(&modifier.node.value.node) {
                        styles.fill_pattern = Some(spec);
                        styles.fill = None;
                    } else {
                        styles.fill = Self::color_to_css(&modifier.node.value.node);
                    }
                }
```

Note: `Spanned` and `StyleValue` are already imported in this file (used throughout `from_modifiers`). `PatternKind`/`GradientKind`/`FillSpec` are in the same module.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --lib fill_spec_tests`
Expected: PASS (all four).

- [ ] **Step 6: Build whole crate (catch struct-literal breakage)**

Run: `cargo build`
Expected: PASS. The explicit `ResolvedStyles { .. }` literals at `src/renderer/svg.rs:1673` and `:1695` (tests) will FAIL here because they list every field — fix them in Task 4 Step 1. `src/layout/engine.rs:1899` uses `..Default::default()` and is unaffected. If `cargo build` (non-test) passes but `cargo test` fails to compile, that's expected and handled in Task 4.

- [ ] **Step 7: Commit**

```bash
git add src/layout/types.rs
git commit -m "feat(layout): resolve pattern/gradient fills into FillSpec"
```

---

## Task 4: Renderer — emit defs and reference them

**Files:**
- Modify: `src/renderer/svg.rs:13-21` (struct fields), `:24-35` (`new`), `:980-1002` (`render_element_inner` call site), `:1386-1417` (`format_styles`), test literals at `:1673` and `:1695`
- Add: `build_fill_def` free function and `register_fill` method in `src/renderer/svg.rs`
- Test: `src/renderer/svg.rs` test module

- [ ] **Step 1: Fix the broken test struct literals first**

In `src/renderer/svg.rs`, the two `ResolvedStyles { ... }` literals (lines ~1673 and ~1695) list every field. Add `fill_pattern: None,` to each (after their `fill:` line). Also both call `format_styles(&styles)` (lines ~1685, ~1707) — update to `format_styles(&styles, None)` (the new signature comes in Step 4; do this now so the file compiles after Step 4).

- [ ] **Step 2: Write the failing tests**

Add to the `src/renderer/svg.rs` test module:

```rust
    #[test]
    fn register_fill_emits_pattern_def_once() {
        use crate::layout::{FillSpec, PatternKind};
        let mut b = SvgBuilder::new(SvgConfig::default());
        let styles = ResolvedStyles {
            fill_pattern: Some(FillSpec::Pattern {
                kind: PatternKind::Hatch,
                fg: "var(--accent-1)".into(),
                bg: "transparent".into(),
            }),
            ..Default::default()
        };
        let url1 = b.register_fill(&styles);
        let url2 = b.register_fill(&styles); // identical -> dedup
        assert!(url1.is_some());
        assert_eq!(url1, url2);
        let url = url1.unwrap();
        assert!(url.starts_with("url(#"));
        // Exactly one def emitted for two identical registrations.
        let count = b.defs.iter().filter(|d| d.contains("<pattern")).count();
        assert_eq!(count, 1);
    }

    #[test]
    fn register_fill_none_for_solid() {
        let mut b = SvgBuilder::new(SvgConfig::default());
        let styles = ResolvedStyles {
            fill: Some("blue".into()),
            ..Default::default()
        };
        assert_eq!(b.register_fill(&styles), None);
    }

    #[test]
    fn format_styles_uses_fill_override() {
        let styles = ResolvedStyles {
            fill: Some("blue".into()),
            ..Default::default()
        };
        let out = format_styles(&styles, Some("url(#grad-x)"));
        assert!(out.contains(r##"fill="url(#grad-x)""##));
        assert!(!out.contains(r##"fill="blue""##));
    }

    #[test]
    fn register_fill_deterministic_id() {
        use crate::layout::{FillSpec, GradientKind};
        let make = || {
            let mut b = SvgBuilder::new(SvgConfig::default());
            let styles = ResolvedStyles {
                fill_pattern: Some(FillSpec::Gradient {
                    kind: GradientKind::Linear,
                    from: "blue".into(),
                    to: "white".into(),
                    angle: 90.0,
                }),
                ..Default::default()
            };
            b.register_fill(&styles).unwrap()
        };
        assert_eq!(make(), make()); // id is content-derived, stable across builders
    }
```

(If `b.defs` is private and not accessible from the test module — it is in the same file/module, so it is accessible. `SvgConfig::default()` must exist; if `SvgConfig` has no `Default`, construct it the way sibling tests do — search the test module for `SvgConfig`.)

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test --lib svg`
Expected: FAIL to compile — `register_fill` and the 2-arg `format_styles` don't exist yet.

- [ ] **Step 4: Add the `fill_def_ids` field and `format_styles` override**

In `SvgBuilder` struct (line ~13), add a field after `defs`:

```rust
    defs: Vec<String>,
    /// Ids of fill pattern/gradient defs already emitted (dedup, order-stable).
    fill_def_ids: Vec<String>,
```

In `SvgBuilder::new` (line ~24), add `fill_def_ids: vec![],` to the initializer.

Change `format_styles` (line 1386) signature and fill line:

```rust
fn format_styles(styles: &ResolvedStyles, fill_override: Option<&str>) -> String {
    let mut parts = vec![];

    // Fill: a pattern/gradient url override wins; else the solid fill; else default.
    let fill = fill_override
        .or(styles.fill.as_deref())
        .unwrap_or("#f0f0f0");
    parts.push(format!(r#" fill="{}""#, fill));
    // ... rest unchanged ...
```

- [ ] **Step 5: Add `register_fill` and `build_fill_def`**

Add the method inside `impl SvgBuilder` (e.g. after `add_arrow_marker`):

```rust
    /// If `styles` carries a pattern/gradient fill, ensure its `<defs>` entry is
    /// emitted (once, deduped by content-derived id) and return `url(#id)` to use
    /// as the shape's `fill`. Returns `None` for solid fills.
    pub fn register_fill(&mut self, styles: &ResolvedStyles) -> Option<String> {
        let spec = styles.fill_pattern.as_ref()?;
        let prefix = self.prefix();
        let (id, def) = build_fill_def(&prefix, spec);
        if !self.fill_def_ids.contains(&id) {
            self.fill_def_ids.push(id.clone());
            self.defs.push(def);
        }
        Some(format!("url(#{})", id))
    }
```

Add the free function near `format_styles` (it needs `FillSpec`, `PatternKind`, `GradientKind` — add them to the `use crate::layout::{...}` import at the top of the file, line ~3):

```rust
/// Sanitize a CSS color string into an id-safe token: `var(--accent-1)` -> `accent-1`,
/// `#ff0000` -> `ff0000`, `transparent` -> `transparent`. Keeps `[a-z0-9-]`.
fn color_token(c: &str) -> String {
    let inner = c
        .trim()
        .trim_start_matches("var(")
        .trim_end_matches(')')
        .trim_start_matches("--")
        .trim_start_matches('#');
    inner
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() || ch == '-' { ch.to_ascii_lowercase() } else { '-' })
        .collect()
}

/// Build the (id, def-string) for a fill spec. `id` is fully prefixed and purely
/// content-derived so identical fills dedup and output stays byte-stable.
fn build_fill_def(prefix: &str, spec: &FillSpec) -> (String, String) {
    match spec {
        FillSpec::Pattern { kind, fg, bg } => {
            let (kname, marks): (&str, String) = match kind {
                PatternKind::Hatch => (
                    "hatch",
                    format!(
                        r#"<path d="M0,8 L8,0 M-1,1 L1,-1 M7,9 L9,7" stroke="{fg}" stroke-width="1" fill="none"/>"#
                    ),
                ),
                PatternKind::CrossHatch => (
                    "crosshatch",
                    format!(
                        r#"<path d="M0,8 L8,0 M-1,1 L1,-1 M7,9 L9,7 M0,0 L8,8 M-1,7 L1,9 M7,-1 L9,1" stroke="{fg}" stroke-width="1" fill="none"/>"#
                    ),
                ),
                PatternKind::Dots => (
                    "dots",
                    format!(r#"<circle cx="4" cy="4" r="1.5" fill="{fg}"/>"#),
                ),
                PatternKind::Grid => (
                    "grid",
                    format!(
                        r#"<path d="M0,0 H8 M0,0 V8" stroke="{fg}" stroke-width="1" fill="none"/>"#
                    ),
                ),
            };
            let id = format!("{prefix}pat-{kname}-{}-{}", color_token(fg), color_token(bg));
            let bg_rect = if bg == "transparent" {
                String::new()
            } else {
                format!(r#"<rect width="8" height="8" fill="{bg}"/>"#)
            };
            let def = format!(
                r#"<pattern id="{id}" patternUnits="userSpaceOnUse" width="8" height="8">{bg_rect}{marks}</pattern>"#
            );
            (id, def)
        }
        FillSpec::Gradient { kind, from, to, angle } => match kind {
            GradientKind::Linear => {
                // angle 0 = top->bottom, 90 = left->right (objectBoundingBox units).
                let rad = angle.to_radians();
                let dx = rad.sin();
                let dy = rad.cos();
                let x1 = 0.5 - dx / 2.0;
                let y1 = 0.5 - dy / 2.0;
                let x2 = 0.5 + dx / 2.0;
                let y2 = 0.5 + dy / 2.0;
                let id = format!(
                    "{prefix}grad-lin-{}-{}-{}",
                    color_token(from),
                    color_token(to),
                    (*angle as i64)
                );
                let def = format!(
                    r#"<linearGradient id="{id}" x1="{x1:.4}" y1="{y1:.4}" x2="{x2:.4}" y2="{y2:.4}"><stop offset="0" stop-color="{from}"/><stop offset="1" stop-color="{to}"/></linearGradient>"#
                );
                (id, def)
            }
            GradientKind::Radial => {
                let id = format!(
                    "{prefix}grad-rad-{}-{}",
                    color_token(from),
                    color_token(to)
                );
                let def = format!(
                    r#"<radialGradient id="{id}" cx="0.5" cy="0.5" r="0.5"><stop offset="0" stop-color="{from}"/><stop offset="1" stop-color="{to}"/></radialGradient>"#
                );
                (id, def)
            }
        },
    }
}
```

Update the import at the top of `src/renderer/svg.rs` (line ~3):

```rust
use crate::layout::{
    BoundingBox, ConnectionLayout, ElementLayout, ElementType, FillSpec, GradientKind,
    LayoutResult, PatternKind, Point, ResolvedStyles, RoutingMode, TextAnchor,
};
```

(Ensure `FillSpec`, `PatternKind`, `GradientKind` are re-exported from the `layout` module. Check `src/layout/mod.rs` — `ResolvedStyles` is exported there; add the three new names to the same `pub use types::{...}` line. If they are not yet exported, add them.)

- [ ] **Step 6: Wire the call site in `render_element_inner`**

Change lines 986-988 from:

```rust
    let id = element.id.as_ref().map(|i| i.0.as_str());
    let styles = format_styles(&element.styles);
    let classes = element.styles.css_classes.clone();
```

to:

```rust
    let id = element.id.as_ref().map(|i| i.0.as_str());
    let fill_override = builder.register_fill(&element.styles);
    let styles = format_styles(&element.styles, fill_override.as_deref());
    let classes = element.styles.css_classes.clone();
```

(`register_fill` borrows `builder` mutably and returns an owned `Option<String>`, so the borrow is released before the per-shape closures borrow `builder` again. No borrow conflict.)

There is also a `fill_style` computed for the `Text` shape arm (around line 1086) using `element.styles.fill` directly — leave it unchanged; patterns are not applied to text fills.

- [ ] **Step 7: Run tests to verify they pass**

Run: `cargo test --lib svg`
Expected: PASS (the four new tests + existing svg tests with updated `format_styles(&styles, None)` calls).

- [ ] **Step 8: Full test suite**

Run: `cargo test`
Expected: PASS. (If the SVG regression test compares structure, a new pattern example isn't added yet — that comes in Task 5; existing examples are unaffected since solid fills are unchanged.)

- [ ] **Step 9: Commit**

```bash
git add src/renderer/svg.rs src/layout/mod.rs
git commit -m "feat(renderer): emit pattern/gradient <defs> and reference via url()"
```

---

## Task 5: Example + documentation

**Files:**
- Create: `examples/pattern-fills.ail`
- Modify: `docs/grammar.md:76-100` (STYLE MODIFIERS + COLORS), `docs/skill-styling.md` (new section), `docs/skill.md` (cross-ref)

- [ ] **Step 1: Create the example**

Write `examples/pattern-fills.ail`. Mirror the structure/conventions of an existing simple example (open `examples/railway-topology.ail` first to match layout idioms — e.g. how rows/labels are declared). Minimal version:

```
# Pattern & gradient fill showcase
row showcase [gap: 16] {
  rect hatch_box [fill: hatch(accent-1), label: "hatch"]
  rect cross_box [fill: cross_hatch(accent-1), label: "cross_hatch"]
  rect dots_box [fill: dots(accent-1, background-light), label: "dots"]
  rect grid_box [fill: grid(foreground-2), label: "grid"]
  rect lin_box [fill: gradient(accent-light, accent-dark), label: "gradient"]
  rect rad_box [fill: radial_gradient(background-light, accent-1), label: "radial"]
}
```

Adjust shape/layout keywords to match what the grammar actually supports (verify against `docs/grammar.md` — e.g. confirm `row` and inline `label:` are valid; if the example examples use a different layout keyword, use that).

- [ ] **Step 2: Render it manually to verify it produces valid SVG**

Run: `cargo run -- examples/pattern-fills.ail -o /tmp/pattern-fills.svg` (adjust CLI flags to match the binary — check `examples/render-all.sh` for the exact invocation form, including any `--stylesheet` flag).
Expected: exits 0, `/tmp/pattern-fills.svg` exists and contains `<pattern` and `<linearGradient` and `url(#`.

- [ ] **Step 3: Verify the def is referenced and tiles render**

Run: `grep -c 'url(#' /tmp/pattern-fills.svg` (expect ≥ 6) and `grep -c '<pattern\|Gradient' /tmp/pattern-fills.svg` (expect ≥ 6, one def per distinct fill).
Open `/tmp/pattern-fills.svg` in a browser or image viewer to confirm patterns/gradients visibly render.

- [ ] **Step 4: Run the example renderer (pre-commit parity)**

Run: `bash examples/render-all.sh`
Expected: completes without error; produces `examples/pattern-fills.svg` (the committed rendered output).

- [ ] **Step 5: Document in `docs/grammar.md`**

In the STYLE MODIFIERS section (after the `fill: <color>` line, ~line 83), and add a COLORS/FILLS note. Insert:

```
    fill: <color>           Solid fill color
    fill: <pattern>         Pattern fill: hatch, cross_hatch, dots, grid
                            e.g. hatch(accent-1), dots(accent-1, background-light)
    fill: gradient(a, b)    Linear gradient a->b (top->bottom)
    fill: gradient(a,b,deg) Linear gradient at angle (0=down, 90=right)
    fill: radial_gradient(a, b)  Radial gradient, center a -> edge b
```

Also add a stroke-dash line in the same section (it is currently undocumented):

```
    stroke_dasharray: "6,3"  Dash pattern (SVG dasharray); also keywords:
                             dashed (8,4), dotted (2,2)
```

- [ ] **Step 6: Document in `docs/skill-styling.md`**

Add a new section after "Part 2: Color System":

```markdown
## Part 3: Pattern & Gradient Fills

Beyond solid colors, `fill:` accepts patterns and gradients. Use them when the
texture carries meaning (e.g. a hatched zone = "restricted", a gradient =
"transition/flow"), not as decoration.

### Patterns

    rect zone [fill: hatch]                          # default fg, transparent bg
    rect zone [fill: hatch(accent-1)]                # colored marks
    rect zone [fill: dots(accent-1, background-light)]  # marks over a bg fill

Available: `hatch`, `cross_hatch`, `dots`, `grid`.
Args: `name`, `name(fg)`, or `name(fg, bg)`. Defaults: fg=`foreground-2`, bg=`transparent`.

### Gradients

    rect bar [fill: gradient(accent-light, accent-dark)]  # vertical (top->bottom)
    rect bar [fill: gradient(blue, white, 90)]            # horizontal (left->right)
    rect bar [fill: radial_gradient(background-light, accent-1)]

`gradient(from, to[, angleDeg])` — angle 0 = top→bottom, 90 = left→right, 45 = diagonal.
`radial_gradient(from, to)` — center `from` to edge `to`.

Colors may be tokens (`accent-1`), hex (`#f00`), or named (`blue`), same as solid fills.

## Part 4: Dashed & Dotted Strokes

    rect box [stroke: accent-1, stroke_dasharray: "6,3"]   # custom dash
    rect box [stroke_dasharray: dashed]                    # = "8,4"
    rect box [stroke_dasharray: dotted]                    # = "2,2"
```

(Renumber any subsequent "Part N" headings in the file so numbering stays sequential.)

- [ ] **Step 7: Cross-reference in `docs/skill.md`**

Find where `docs/skill.md` references styling / `--skill-styling` (search for `skill-styling` or "styling"). Add one line near it:

```
- Pattern/gradient fills (`fill: hatch(...)`, `fill: gradient(...)`) and dashed
  strokes — see `--skill-styling`.
```

- [ ] **Step 8: Run full test + render once more**

Run: `cargo test && bash examples/render-all.sh`
Expected: tests PASS; renders complete without error.

- [ ] **Step 9: Commit**

```bash
git add examples/pattern-fills.ail examples/pattern-fills.svg docs/grammar.md docs/skill-styling.md docs/skill.md
git commit -m "docs(styling): document pattern/gradient fills + dashed strokes; add example"
```

---

## Self-Review Notes

- **Spec coverage:** patterns (Task 3/4), gradients incl. angle (Task 3/4), error handling (Task 2 grammar `try_map`), deterministic dedup ids (Task 4 `build_fill_def`/`register_fill`), docs incl. stroke-dash (Task 5), example + render-all (Task 5), tests at each layer (Tasks 2-4). All spec sections map to a task.
- **Type consistency:** `FillSpec`/`PatternKind`/`GradientKind` defined in Task 3, imported in Task 4. `register_fill(&ResolvedStyles) -> Option<String>` and `build_fill_def(&str, &FillSpec) -> (String, String)` used consistently. `format_styles(&ResolvedStyles, Option<&str>)` — every call site updated (render_element_inner + 2 tests).
- **Known adaptation points flagged inline:** parser test helper names, `Spanned`/`Identifier` constructors, `SvgConfig::default`, `layout::mod` re-exports, exact CLI flags in render-all.sh. These depend on local conventions the implementer must confirm against the file — each is called out at its step.
```
