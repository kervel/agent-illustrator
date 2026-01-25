# Research: Complex Shapes with Arcs and Curves

## Overview

This document captures research decisions for implementing the `path` shape type that allows defining custom shapes with straight and curved segments.

## Key Decisions

### Decision 1: Path Syntax Model

**Decision:** Use a procedural path-building syntax inside a block, similar to SVG path commands but with named vertices instead of coordinates.

**Rationale:**
- Named vertices align with the project's "semantic over geometric" principle
- Block-based syntax matches existing layout/group patterns in the grammar
- Implicit vertex creation reduces verbosity while explicit declaration allows positioning hints

**Alternatives Considered:**
1. **Point-list syntax** (`path "box" [(0,0), (100,0), (100,50), (0,50)]`) - Rejected: requires coordinate calculations, violates LLM-friendliness goal
2. **Declarative edge list** (`edges: a--b, b~c, c--d`) - Rejected: harder to specify arc parameters per segment
3. **Template-based presets** (`rounded_rect "box" [radius: 10]`) - Partial adoption: use as shorthand on top of path syntax

### Decision 2: Arc Representation

**Decision:** Support two arc specification methods:
1. **Radius-based**: `arc_to target [radius: 20, sweep: clockwise]` - for circular arcs
2. **Bulge-based**: `arc_to target [bulge: 0.3]` - for intuitive curvature (0=straight, 1=semicircle)

**Rationale:**
- Radius is familiar from SVG/CSS but requires understanding of arc geometry
- Bulge factor is more intuitive for LLMs (simple 0-1 scale)
- Both are well-documented in CAD/graphics literature

**Alternatives Considered:**
1. **Bezier control points** - Rejected: requires coordinate calculations, v2 consideration
2. **Angle specification** (`arc_to target [angle: 45deg]`) - Deferred: complex to compute arc from angle alone
3. **Named arc types** (`arc_to target [type: quarter_circle]`) - Could be added as convenience keywords

### Decision 3: Vertex Positioning Model

**Decision:** All vertex positions are relative to shape origin (0,0). The first vertex defaults to origin; subsequent vertices require position hints.

**Rationale:**
- Aligns with clarification session answer
- Maintains separation: user defines shape geometry, layout engine positions the shape
- Relative coordinates prevent coordinate drift across shape editing

**Implementation Details:**
- `vertex name` at origin if first, otherwise requires position
- `vertex name [x: 10, y: 20]` - offset from origin
- `vertex name [right: 30, down: 15]` - directional syntax (sugar for x/y)
- `line_to vert [x: 50, y: 0]` - implicit vertex creation

### Decision 4: Degenerate Path Handling

**Decision:** Paths with fewer than 3 vertices are valid and render as points (1 vertex) or line segments (2 vertices).

**Rationale:**
- Aligns with clarification session answer
- Allows using path syntax for lines when arc segments are needed
- Consistent with "fail fast" principle - invalid vertex count is caught, but simple shapes are allowed

### Decision 5: Integration with Existing Shape System

**Decision:** `path` becomes a new ShapeType variant, using the same modifier system as existing shapes.

**Rationale:**
- Minimal disruption to existing code
- Reuses existing style modifier infrastructure
- Path shapes participate in layouts like other shapes

**AST Impact:**
- New `ShapeType::Path { ... }` variant
- Path-specific types: `Vertex`, `PathSegment`, `ArcParams`, `ClosePath`
- No changes to Statement enum (path is still a shape)

### Decision 6: Grammar Extension Strategy

**Decision:** Add path-specific keywords as new productions, minimal conflict with existing keywords.

**New Keywords:**
- `path` - shape type
- `vertex` - vertex declaration
- `line_to` - straight segment
- `arc_to` - curved segment
- `close` - close path (straight)
- `close_arc` - close path (curved)

**New Modifiers:**
- `radius` - arc radius
- `bulge` - arc curvature factor
- `sweep` - arc direction (clockwise/counterclockwise)
- `rounded` - corner rounding shorthand

**Rationale:**
- Keywords are distinctive and unlikely to conflict with user identifiers
- Modifiers follow existing key:value pattern
- Sweep direction uses familiar clockwise/counterclockwise terminology

## Technical Notes

### SVG Output Mapping

Path shapes will render to SVG `<path>` elements:
- `line_to` → `L x,y` command
- `arc_to` → `A rx,ry rotation large-arc-flag sweep-flag x,y`
- `close` → `Z`
- `close_arc` → `A ... Z`

Bulge factor conversion:
```
radius = distance(p1, p2) / (2 * sin(2 * atan(bulge)))
```

### Layout Engine Considerations

Path shapes need bounding box calculation from vertices for layout participation. When explicit width/height modifiers are provided, use those; otherwise compute from vertex extrema.

Auto-sizing algorithm:
1. Collect all vertex positions (resolving relative references)
2. Find min/max x and y coordinates
3. Add padding for stroke width
4. Return bounding box

## Open Questions (Deferred)

1. **Compound paths with holes**: Multiple contours in a single shape (v2)
2. **Bezier curves**: Cubic/quadratic beziers for more complex curves (v2)
3. **Path morphing/animation**: Interpolating between paths (out of scope)
4. **Path text**: Text along a path (out of scope for this feature)
