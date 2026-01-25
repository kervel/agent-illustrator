---
parent_branch: main
feature_number: 008
status: In Progress
created_at: 2026-01-25T14:30:00+01:00
---

# Feature: Curved Paths and Connectors

## Overview

Add support for smooth curves in both custom path shapes and connectors. Rather than implementing a custom Bezier or B-spline engine, this feature leverages SVG's native quadratic Bezier curves (Q command) by introducing "steering vertices" - invisible control points that shape the curve. These steering vertices can be positioned using all existing positioning capabilities (constraints, relative coordinates, rows/columns), making curve definition consistent with the rest of the language.

The key insight is that SVG's quadratic Bezier curves use a control point between endpoints, which maps naturally to "steering vertices" that authors place semantically. This provides smooth, predictable curves without coordinate-level complexity.

## Clarifications

### Session 2026-01-25

- Q: How should the system behave when a `via` reference points to a non-existent or unpositioned steering vertex? → A: Compile error - fail at parse/compile time with clear error message
- Q: For connectors with multiple `via` points (e.g., `[via: v1, v2]`), should this produce multiple quadratic segments or a single cubic Bezier? → A: Chain quadratics - each via point creates a Q segment that joins smoothly at intermediate points
- Q: For path shapes, should `curve_to` require an explicit `via` parameter, or allow implicit auto-generated control points like connectors? → A: Optional via - allow `[via: x]` or omit for auto-generated control point

## User Scenarios

### Scenario 1: Creating a Curved Path Shape

An author wants to draw a smooth curved shape (like a wave or flowing line).

**Flow:**
1. Author defines a path with vertices and curve segments
2. Author specifies steering vertices between endpoints to control curve shape
3. Steering vertices can be positioned using existing mechanisms (x/y offsets, constraints, relative positioning)
4. The system renders smooth curves through the vertices

**Example usage:**
```
path "wave" wave_shape {
    vertex start [x: 0, y: 50]
    curve_to mid [via: control1]
    curve_to end [via: control2]
}

// Steering vertices positioned using existing capabilities
place control1 [x: 25, y: 0]
place control2 [x: 75, y: 100]
```

### Scenario 2: Curved Connectors Between Shapes

An author wants to connect two shapes with a smooth curve instead of straight or orthogonal lines.

**Flow:**
1. Author creates connection between two shapes
2. Author specifies curved routing mode
3. Optionally, author places steering vertices to control curve shape
4. Without explicit steering, system provides reasonable default curve

**Example usage:**
```
circle a [size: 30]
circle b [size: 30]
place b right-of a [x: 100]

// Simple curved connector with auto-generated control point
a -> b [routing: curved]

// Or with explicit steering vertex
a -> b [routing: curved, via: my_control]
place my_control above a [y: 50]
```

### Scenario 3: Multi-Segment Curved Paths

An author wants to create a path with multiple curved segments sharing continuous tangent direction.

**Flow:**
1. Author defines path with multiple curve segments
2. Each segment has its own steering vertex
3. Curves connect smoothly at shared vertices

### Scenario 4: Mixed Straight and Curved Segments

An author combines straight lines and curves in a single path or connector route.

**Flow:**
1. Author uses `line_to` for straight segments
2. Author uses `curve_to` for curved segments
3. The system renders the mixed path correctly

## Functional Requirements

### FR1: Curve Path Command

The path syntax must support a new `curve_to` command that creates a quadratic Bezier curve segment.

**Requirements:**
- `curve_to` takes a target vertex and an optional `via` parameter specifying the steering vertex
- When `via` is omitted, system auto-generates a control point (same logic as curved connectors)
- When `via` is specified, the steering vertex must exist and be positioned; otherwise compile error
- The curve is rendered using SVG's Q (quadratic Bezier) command
- Steering vertices are not rendered visibly

### FR2: Curved Connector Routing Mode

Connectors must support a `curved` routing mode.

**Requirements:**
- `[routing: curved]` creates a smooth curved connection
- Default behavior: system calculates a reasonable control point offset perpendicular to the direct path
- Explicit control: `[via: vertex_name]` specifies a steering vertex for precise curve shaping
- Multiple `via` points create chained quadratic segments: `[via: v1, v2]` produces start→v1→v2→end as smooth-joined Q commands
- Invalid `via` references (non-existent or unpositioned) cause compile-time error

### FR3: Steering Vertex Positioning

Steering vertices must support all existing positioning mechanisms.

**Requirements:**
- Steering vertices can be positioned with absolute coordinates: `[x: N, y: N]`
- Steering vertices can be positioned relatively: `place control above shape [y: 20]`
- Steering vertices can participate in constraints: `constrain control.x = midpoint(a, b)`
- Steering vertices can be placed in layout containers (row, col) for systematic positioning

### FR4: Default Curve Behavior

When `[routing: curved]` is used without explicit steering, the system generates a sensible default.

**Requirements:**
- Default control point is offset perpendicular to the line connecting endpoints
- Default offset distance is proportional to the connection length (reasonable factor to be determined during implementation)
- The curve direction follows a predictable convention (e.g., curves "outward" from crowded areas)

### FR5: SVG Path Output

Curved paths and connectors must render to valid SVG.

**Requirements:**
- Path curves render using SVG `Q` command (quadratic Bezier)
- Multi-segment curves can use `T` command (smooth quadratic) where appropriate
- Output is valid SVG that renders correctly in standard SVG viewers

### FR6: Invisible Steering Elements

Steering vertices used in curves are not rendered.

**Requirements:**
- Steering vertices declared in paths or referenced via `[via:]` are invisible by default
- Steering vertices do not affect bounding box calculations for their parent path/connector
- Steering vertices can optionally be made visible for debugging purposes

## Success Criteria

1. **Curve accuracy**: Curves pass through endpoints with smooth tangent continuity at join points
2. **First-time success rate**: An AI agent can produce correctly curved illustrations 90%+ of the time without iteration
3. **Positioning consistency**: Steering vertices behave identically to regular elements for positioning purposes
4. **Rendering correctness**: All curved output renders correctly in major browsers and SVG viewers
5. **Language ergonomics**: Authors can specify simple curves with minimal syntax (single `via` parameter)
6. **Complex curve support**: Authors can create multi-point spline-like curves by chaining `curve_to` commands

## Key Entities

### CurveToDecl
A path command representing a quadratic Bezier curve segment.
- **target**: The endpoint vertex name
- **via**: The steering vertex name (control point)
- **position**: Optional position modifiers for the target

### Steering Vertex
An invisible element used to shape curves.
- Can be declared inline in path body or referenced externally
- Supports all positioning mechanisms
- Does not render visibly
- Multiple steering vertices can be chained for complex curves

### CurvedRouting
A connection routing mode using smooth curves.
- Default behavior with auto-generated control point
- Optional explicit steering via `[via:]` modifier
- Supports multiple control points for S-curves

## Edge Cases & Error Handling

- **Missing via reference**: If `[via: foo]` references a non-existent element, emit compile error: "Steering vertex 'foo' not found"
- **Unpositioned via reference**: If the referenced element exists but has no resolved position, emit compile error: "Steering vertex 'foo' has no position"
- **Empty via list**: `[via:]` with no arguments is a syntax error
- **Self-referential via**: A curve cannot use itself as a steering vertex; compile error if detected

## Assumptions

1. **SVG quadratic Bezier is sufficient**: Most illustration use cases are served well by quadratic Bezier curves (Q command). Multiple via points chain as quadratic segments rather than upgrading to cubic Beziers.

2. **Steering vertices are named elements**: Steering vertices follow the same naming and referencing conventions as other elements in the language.

3. **Default curve offset**: A perpendicular offset of approximately 25-30% of the connection length provides visually pleasing default curves.

4. **Tangent continuity optional**: While smooth joins are desirable, enforcing G1 continuity at all join points is not required for initial implementation.

5. **No closed curve smoothing**: The `close` command in paths creates a straight line to start vertex; automatic smoothing of closed curves is out of scope.

6. **Steering vertex syntax**: The `via` keyword clearly indicates curve control points, matching common terminology in vector graphics.
