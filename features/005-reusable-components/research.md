# Research: Reusable Components

## Research Questions & Decisions

### RQ-1: How to distinguish component instances from shapes during parsing?

**Question**: The syntax `person "alice"` looks identical to `rect "mybox"`. How does the parser know `person` is a component and not an unknown shape type?

**Research**: Examined existing parser architecture in `grammar.rs`. The parser uses chumsky combinators with a `choice()` that checks shape keywords first.

**Decision**: Two-pass parsing approach.

**Rationale**:
1. First pass collects all `ComponentDecl` statements and builds a set of component names
2. Second pass uses this set to distinguish component instances from unknown identifiers
3. This aligns with how the existing parser handles reserved keywords

**Alternative Considered**: Single-pass with backtracking
- Would require parser context threading
- chumsky supports this but adds complexity
- Two-pass is cleaner and matches semantic validation phase

**Implementation**:
```rust
// Phase 1: Parse document, treating unknown identifiers as errors
// Phase 2: Collect component names from ComponentDecl statements
// Phase 3: Re-parse or transform, resolving component instances
```

### RQ-2: SVG dimension extraction approach

**Question**: How to reliably extract width/height from SVG files without a full XML parser?

**Research**: Examined SVG specification and common patterns:
- `<svg width="100" height="50">` - explicit dimensions
- `<svg viewBox="0 0 100 50">` - implicit dimensions
- `<svg width="100%" viewBox="0 0 100 50">` - mixed
- Units: px, em, %, pt, etc.

**Decision**: Regex-based extraction for common cases, fallback to default.

**Rationale**:
1. Full XML parsing (e.g., quick-xml crate) adds dependency
2. Most icon SVGs use simple patterns
3. Graceful degradation with default 100x100 if parsing fails
4. Can upgrade to full parser later if needed

**Implementation**:
```rust
fn extract_svg_dimensions(content: &str) -> SvgInfo {
    // Try viewBox first (most reliable)
    if let Some(vb) = extract_viewbox(content) {
        return SvgInfo { view_box: Some(vb), .. };
    }

    // Try explicit width/height
    let width = extract_numeric_attr(content, "width");
    let height = extract_numeric_attr(content, "height");

    SvgInfo { width, height, view_box: None, content: content.to_string() }
}

fn extract_viewbox(content: &str) -> Option<(f64, f64, f64, f64)> {
    // Regex: viewBox\s*=\s*["']([^"']+)["']
    // Parse "minX minY width height"
}
```

### RQ-3: Namespace prefixing strategy for AIL components

**Question**: How to prefix internal element names to avoid collisions?

**Research**: Examined existing `ElementPath` in ast.rs:
- Already supports dot-notation: `group1.item.child`
- Used for alignment: `align a.left = group1.item.left`

**Decision**: Use instance name as prefix with dot separator.

**Rationale**:
1. Consistent with existing path notation
2. Clear debugging (paths visible in SVG ids)
3. Enables existing ElementPath for export resolution

**Example**:
```
// server-rack.ail
rect server1
rect server2
export server1

// main.ail
component "rack" from "server-rack.ail"
rack "r1"  // Creates: r1.server1, r1.server2
rack "r2"  // Creates: r2.server1, r2.server2

// Only r1.server1 and r2.server1 are externally targetable
```

### RQ-4: Handling nested component imports

**Question**: When component A imports B which imports C, how to manage:
1. Namespace prefixing
2. Export propagation
3. Circular detection

**Decision**: Recursive expansion with accumulated prefix.

**Implementation**:
```rust
fn expand_instance(
    &self,
    inst: &ComponentInstance,
    prefix: &str,  // Accumulated prefix from parent instances
) -> Vec<Statement> {
    let new_prefix = if prefix.is_empty() {
        inst.instance_name.0.clone()
    } else {
        format!("{}.{}", prefix, inst.instance_name.0)
    };

    // Recursively expand, passing new_prefix
}
```

**Export Behavior**:
- Exports are component-local
- When B exports `x`, and A instantiates B as `b1`:
  - `b1.x` is targetable from A
  - If A re-exports `b1.x`, it becomes available to A's importers

### RQ-5: Style propagation to SVG components

**Question**: How do `[fill: blue]` style modifiers affect embedded SVG content?

**Research**: SVG CSS cascade rules:
- Inline styles override CSS
- Presentation attributes have lower specificity
- `currentColor` keyword inherits from parent

**Decision**: Wrapper group with CSS variables.

**Rationale**:
1. Non-destructive (doesn't modify source SVG)
2. SVGs using `currentColor` or CSS variables work automatically
3. Explicit styles in SVG still override (expected behavior)

**Implementation**:
```rust
// Render SVG embed:
builder.start_group(id, &classes);
// Set CSS custom properties for common styles
if let Some(fill) = &instance.fill {
    builder.add_style_attr(format!("--fill: {}", fill));
}
// Embed SVG content
builder.add_raw_svg(content);
builder.end_group();
```

**Limitation**: SVGs not using CSS variables or currentColor won't respond to style overrides. This is acceptable and documented.

### RQ-6: Error recovery in component parsing

**Question**: Should parsing continue after component errors (missing file, syntax error)?

**Decision**: Collect all errors, fail at validation phase.

**Rationale**:
1. Better user experience (see all errors at once)
2. Matches existing ariadne error reporting pattern
3. Parser produces partial AST for IDE integration

**Implementation**:
- ImportResolver returns `Result<ResolvedComponent, ImportError>`
- Validation phase collects all ImportErrors
- ariadne renders multi-file error spans

## Performance Considerations

### Component Caching

**Observation**: Same component may be imported multiple times (directly or transitively).

**Decision**: ImportResolver caches resolved components by canonical path.

**Benefit**: O(1) lookup for repeated imports, single parse per file.

### Large SVG Handling

**Observation**: Some SVGs (icons, diagrams) can be large (100KB+).

**Decision**: Lazy content loading with streaming.

**Future Enhancement**: Could intern common SVG content, but premature optimization for now.

## Compatibility Notes

### SVG Subset Assumptions

The implementation assumes SVGs are:
1. Static (no JavaScript, SMIL animation)
2. Self-contained (no external references like `<use href="...">`)
3. UTF-8 encoded
4. Valid XML (well-formed)

### AIL Version Compatibility

Component AIL files use the same grammar version as the importing file. No version negotiation needed for v1.
