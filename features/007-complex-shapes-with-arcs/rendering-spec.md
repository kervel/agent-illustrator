# Path Rendering Specification

## Overview

Convert `PathDecl` AST nodes into SVG `<path>` elements with proper `d` attribute strings.

## Input

A `PathDecl` contains:
- `name`: Optional identifier
- `body.commands`: Vec of `PathCommand`:
  - `Vertex(VertexDecl)`: Named vertex with optional position `[x: N, y: N]`
  - `LineTo(LineToDecl)`: Line to target vertex with optional position
  - `ArcTo(ArcToDecl)`: Arc to target vertex with position and `ArcParams`
  - `Close`: Close path back to start with straight line
- `modifiers`: Style modifiers (fill, stroke, etc.)

`ArcParams`:
- `Radius { radius, sweep }`: Arc with given radius and sweep direction
- `Bulge(f64)`: Arc curvature as bulge factor (tan(θ/4))

## Output

SVG `<path>` element:
```svg
<path id="name" class="shape path" d="M x0 y0 L x1 y1 A rx ry 0 0 1 x2 y2 Z" style="..."/>
```

## SVG Path Commands

| Command | Meaning | Example |
|---------|---------|---------|
| M x y | Move to (start) | M 0 0 |
| L x y | Line to | L 50 0 |
| A rx ry rot large sweep x y | Arc to | A 10 10 0 0 1 60 10 |
| Z | Close path | Z |

## Algorithm

### Phase 1: Resolve Vertex Positions

1. Create vertex map: `HashMap<String, Point>`
2. Process commands in order:
   - `Vertex`: Add to map with position (default 0,0 if not specified)
   - `LineTo`: If position given, add target to map; otherwise must exist
   - `ArcTo`: Same as LineTo
3. If any referenced vertex missing, render as error placeholder

### Phase 2: Generate SVG Path String

1. Find starting vertex (first Vertex command or first referenced vertex)
2. Output `M startX startY`
3. For each subsequent command:
   - `LineTo`: Output `L targetX targetY`
   - `ArcTo`: Calculate arc and output `A rx ry 0 large sweep targetX targetY`
   - `Close`: Output `Z`

### Arc Calculation

**From Bulge:**
```
bulge = tan(θ/4) where θ is the arc angle
- bulge = 0 → straight line
- bulge = 1 → semicircle
- bulge = 0.414 ≈ tan(π/8) → quarter circle (45°)
- negative bulge → curve on opposite side

Given start (x1,y1), end (x2,y2), bulge:
1. chord = distance(start, end)
2. sagitta = |bulge| * chord / 2
3. radius = (chord² + 4*sagitta²) / (8*sagitta)
4. large-arc-flag = 0 (always small arc for bulge)
5. sweep-flag = 1 if bulge > 0, else 0
```

**From Radius:**
```
Given start (x1,y1), end (x2,y2), radius, sweep:
1. chord = distance(start, end)
2. If chord > 2*radius, clamp radius = chord/2 (semicircle)
3. large-arc-flag = 0 (always use smaller arc)
4. sweep-flag = 1 if Clockwise, 0 if Counterclockwise
```

## Edge Cases

| Case | Behavior |
|------|----------|
| Empty path | Render nothing |
| Single vertex | Render as point (small circle) |
| Two vertices, no segments | Render line between them |
| Missing vertex reference | Log warning, skip segment |
| Zero radius arc | Render as line |
| Unclosed path | Leave unclosed (no implicit close) |

## Positioning

Path vertices are relative to the element's bounding box origin:
- `element.bounds.x` + vertex.x
- `element.bounds.y` + vertex.y

The bounding box is computed by the layout engine from all vertex positions.

## Tests

1. Triangle (3 vertices + close)
2. Arrow with curved back (line + arc + close)
3. Rounded rectangle (lines + arcs)
4. Single vertex (point)
5. Unclosed path
6. Arc with different sweep directions
