# Quickstart: Curved Paths and Connectors

## Usage Examples

### 1. Simple Curved Connector

Connect two shapes with a smooth curve:

```ail
circle a [size: 30]
circle b [size: 30]
place b right-of a [x: 100]

// Auto-generated curve (gentle arc)
a -> b [routing: curved]
```

### 2. Controlled Curve via Steering Vertex

Use a steering vertex to control curve shape:

```ail
circle a [size: 30]
circle b [size: 30]
place b right-of a [x: 100]

// Invisible steering element
rect control [size: 0]
place control above a [y: 50, x: 50]

// Curve bends toward control point
a -> b [routing: curved, via: control]
```

### 3. S-Curve with Multiple Via Points

Create complex curves using multiple control points:

```ail
circle start [size: 20]
circle end [size: 20]
place end right-of start [x: 200]

// Two steering points create S-curve
rect c1 [size: 0]
rect c2 [size: 0]
place c1 above start [x: 60, y: 40]
place c2 below start [x: 140, y: -40]

start -> end [routing: curved, via: c1, c2]
```

### 4. Curved Path Shape

Define custom shapes with curved segments:

```ail
path "wave" my_wave {
    vertex start [x: 0, y: 50]
    curve_to mid [via: top_control]
    curve_to end [via: bottom_control]
}

rect top_control [size: 0]
rect bottom_control [size: 0]
place top_control [x: 50, y: 0]
place bottom_control [x: 100, y: 100]
```

### 5. Auto-Generated Curve in Path

Let the system calculate control points:

```ail
path "simple_arc" arc_shape {
    vertex a [x: 0, y: 0]
    curve_to b [x: 100, y: 0]  // No via = auto-generated curve
}
```

### 6. Mixed Straight and Curved

Combine line and curve commands:

```ail
path "mixed" shape {
    vertex a [x: 0, y: 0]
    line_to b [x: 50, y: 0]
    curve_to c [via: control, x: 100, y: 50]
    line_to d [x: 100, y: 100]
}

rect control [size: 0]
place control [x: 80, y: 10]
```

## Key Concepts

### Steering Vertices
- Invisible elements that shape curves
- Position using any existing mechanism (place, constraints, rows)
- Reference by name with `[via: name]`

### Default Curves
- When `via` is omitted, system generates a gentle curve
- Control point offset perpendicular to chord at 25% distance
- Predictable, pleasant default for simple cases

### Chained Curves
- Multiple via points create smooth-joined segments
- Each segment transitions smoothly to the next
- Uses SVG's smooth quadratic (T) command internally

### Error Handling
- Invalid via references produce compile errors
- Clear messages: "Steering vertex 'foo' not found"
- Fail fast - no silent degradation

## Migration from Existing Connectors

| Before | After |
|--------|-------|
| `a -> b` | `a -> b` (unchanged, orthogonal default) |
| `a -> b [routing: direct]` | `a -> b [routing: direct]` (unchanged) |
| N/A | `a -> b [routing: curved]` (NEW) |
| N/A | `a -> b [routing: curved, via: c]` (NEW) |
