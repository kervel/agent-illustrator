---
parent_branch: main
feature_number: 006
status: In Progress
created_at: 2026-01-24T16:05:20+01:00
---

# Feature: Arbitrary Rotation of Any Shape

## Overview

Add rotation support to all shape types in the agent-illustrator DSL, allowing users to specify rotation angles as a shape modifier. This enables rotated elements in illustrations without requiring manual coordinate calculations, staying true to the "semantic over geometric" core principle.

Currently, shapes can only be positioned axis-aligned. Adding rotation allows diagrams like tilted arrows, angled text labels, rotated icons, and diagonal elements that are common in technical illustrations.

## User Scenarios

### Scenario 1: Rotated Arrow Indicator

A user wants to create a diagram showing a diagonal arrow pointing at a specific element.

```
rect server [label: "Server"]
polygon arrow [rotation: 45]
```

The arrow polygon appears rotated 45 degrees from its default orientation.

### Scenario 2: Angled Text Label

A user creates a timeline diagram where labels need to be rotated to fit along a diagonal axis.

```
text timestamp [rotation: -30] "2024-01-01"
```

The text appears rotated 30 degrees counter-clockwise.

### Scenario 3: Rotated Icon in Layout

A user places a rotated warning icon within a layout container.

```
row warnings {
    icon warning [rotation: 15]
    text msg "Attention required"
}
```

The warning icon appears slightly tilted while remaining within the layout flow.

### Scenario 4: Ellipse with Custom Orientation

A user creates an ellipse representing an orbit that needs to be tilted.

```
ellipse orbit [width: 200, height: 80, rotation: 20]
```

The ellipse appears rotated 20 degrees from horizontal.

## Functional Requirements

### FR1: Rotation Modifier Support

All shape types must accept a `rotation` modifier that specifies the rotation angle in degrees.

- Applies to: rect, circle, ellipse, polygon, line, icon, text
- Syntax: `[rotation: <number>]`
- Value range: Any numeric value (positive or negative)
- Convention: Positive angles rotate clockwise, negative angles rotate counter-clockwise
- Default: 0 (no rotation, current behavior preserved)

### FR2: Rotation Center Point

Shapes rotate around their geometric center.

- For rectangles and ellipses: center of bounding box
- For circles: center point
- For polygons: centroid of the polygon points
- For text: center of the text bounding box
- For icons: center of the icon bounding box

### FR3: Layout Interaction

Rotation is a rendering-only property that does not affect layout calculations.

- Layout engine uses unrotated bounding boxes for positioning
- Rotated shapes may visually overflow their layout-assigned space
- This is intentional to maintain layout predictability
- Constraint references (`.left`, `.top`, etc.) refer to the unrotated bounding box

### FR4: Connection Routing

Connections to rotated shapes attach to the unrotated bounding box edges.

- This maintains compatibility with the existing routing system
- Provides predictable connection behavior
- Future enhancement could add rotated boundary support

### FR5: SVG Output

Rotation is implemented via SVG `transform` attribute.

- Output format: `transform="rotate(<angle> <cx> <cy>)"`
- The rotation applies to the shape element directly
- Preserves all other styling (fill, stroke, opacity, etc.)

### FR6: Angle Normalization

The system accepts any angle value but normalizes for rendering consistency.

- Angles outside 0-360 are mathematically equivalent (e.g., 370 = 10)
- Negative angles work as expected (-45 = 315)
- No clamping or restriction on input values

## Success Criteria

- Users can rotate any shape type using the `rotation` modifier
- Rotated shapes appear correctly oriented in the SVG output
- Existing illustrations without rotation continue to render identically
- Layout containers position elements the same regardless of rotation
- Connections to rotated shapes remain functional
- Test suite covers all shape types with various rotation angles

## Key Entities

### StyleKey::Rotation

New variant in the StyleKey enum that represents the rotation angle modifier.

### ResolvedStyles.rotation

New optional field storing the resolved rotation angle for rendering.

### SVG Transform

The rotation is expressed in SVG output as a transform attribute on shape elements.

## Assumptions

- Degrees are the natural unit for users (not radians), as they are more intuitive for non-technical users
- Clockwise positive rotation matches common graphics tools (SVG, CSS) conventions
- Rendering-only rotation (not affecting layout) is acceptable for the initial implementation - this keeps the constraint solver simple while still providing useful functionality
- Text rotation inherits the same semantics as shape rotation
- Icons can be rotated like any other shape element
