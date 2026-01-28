---
parent_branch: 009-mosfet-driver-example
feature_number: 011
status: In Progress
created_at: 2026-01-28T16:30:00+01:00
---

# Feature: Anchor-Based Constraints

## Overview

Extend the constraint system to allow referencing anchor positions in constraint expressions. Currently, constraints can only reference element bounding box properties (`.left`, `.right`, `.top`, `.bottom`, `.center_x`, `.center_y`). This feature adds the ability to reference anchor positions, enabling precise alignment of elements with connection points that are offset from element centers.

**Use Case**: In electronic schematics, a MOSFET's drain anchor is offset from the body center. To align a flyback diode with the drain, users currently cannot write:
```ail
constrain d_flyback.center_x = q_main.drain_x  // Not currently supported
```

Instead, they must manually position elements or accept misalignment.

## Problem Statement

### Current Constraint Capabilities

Constraints can reference element properties:
```ail
constrain foo.left = bar.right + 10
constrain foo.center_x = bar.center_x
constrain foo.center_y = midpoint(a, b)
```

The available properties are: `.left`, `.right`, `.top`, `.bottom`, `.center_x`, `.center_y`, `.horizontal_center`, `.vertical_center`.

### Current Anchor Capabilities

Anchors define named connection points with position and direction:
```ail
anchor gate [position: gate_lead.left, direction: left]
anchor drain [position: drain_lead.top, direction: up]
```

Anchors can be used in connections:
```ail
q1.drain -> q2.gate [routing: curved]
```

### The Gap

There is no way to reference anchor positions in constraints. This prevents:
- Aligning elements to specific connection points
- Creating symmetry based on anchor positions
- Precise layout where visual connection points matter

## Proposed Solution

### Syntax Extension

Allow constraint expressions to reference anchor coordinates using underscore suffix notation:

```ail
constrain foo.center_x = bar.anchor_name_x
constrain foo.center_y = bar.anchor_name_y
```

Where:
- `bar` is the template instance name
- `anchor_name` is the name of an anchor defined in the template
- `_x` and `_y` suffixes access the x and y coordinates of the anchor position

This syntax is consistent with existing properties like `center_x` and `center_y`.

### Interaction with Rotation

When a template instance has rotation applied (Feature 010), anchor positions are transformed to global coordinates during Phase 2 (Rotation Application). Constraint references to anchors use these transformed positions.

This means `bar.anchor_name_x` returns the post-rotation x-coordinate of the anchor.

## User Scenarios

### Scenario 1: Align Flyback Diode with MOSFET Drain

A user creates a MOSFET driver circuit with a flyback diode that should be precisely aligned with the MOSFET's drain connection.

```ail
nmos q_main
diode d_flyback

// Align diode anode horizontally with MOSFET drain anchor
constrain d_flyback.center_x = q_main.drain_x
constrain d_flyback.center_y = q_main.center_y
```

The diode is positioned so its center aligns with the x-coordinate of the drain anchor, not the body center.

### Scenario 2: Symmetric Component Placement

A user creates a circuit with two LEDs that should be symmetrically placed around a central anchor.

```ail
template "driver" {
    rect body [...]
    anchor output [position: body.right + 10, direction: right]
}

driver drv
led led_top
led led_bottom

// Place LEDs symmetrically around the driver output
constrain led_top.center_x = drv.output_x + 20
constrain led_bottom.center_x = drv.output_x + 20
constrain led_top.bottom = drv.output_y - 10
constrain led_bottom.top = drv.output_y + 10
```

### Scenario 3: Rotated Template Anchor Reference

A user aligns an element to an anchor of a rotated template.

```ail
resistor r1 [rotation: 90]  // Vertical resistor
rect junction

// Align junction with the (rotated) left connector anchor
constrain junction.center_x = r1.left_conn_x
constrain junction.center_y = r1.left_conn_y
```

The junction aligns with the post-rotation position of `left_conn`, which points upward after 90° rotation.

## Functional Requirements

### FR1: Anchor Coordinate Access Syntax

Constraints must support `instance.anchorname_x` and `instance.anchorname_y` syntax.

- `instance.anchorname_x` returns the x-coordinate of the anchor named "anchorname"
- `instance.anchorname_y` returns the y-coordinate of the anchor named "anchorname"
- The `_x` and `_y` suffixes are appended to the anchor name (e.g., `drain_x`, `left_conn_y`)

### FR2: Anchor Resolution Order

Anchor positions must be resolved before constraint evaluation.

- For non-rotated templates: anchor positions calculated from local element bounds
- For rotated templates: anchor positions transformed after local solve (per Feature 010)
- Constraint solver receives final anchor coordinates

### FR3: Cross-Template Anchor References

Constraints can reference anchors from any template instance in scope.

- Both template instances and top-level elements with custom anchors
- Exported anchors are not required (anchors are always accessible)
- Error if anchor name does not exist on the referenced instance

### FR4: Error Handling

Clear error messages for invalid anchor references.

- Error: "Unknown anchor 'xyz' on instance 'foo'" if anchor doesn't exist
- Error: "Cannot reference anchor on non-template element 'bar'" if bar is a simple shape without anchors
- Error: "Missing coordinate selector - use 'foo.anchor_x' or 'foo.anchor_y'" if bare anchor used

### FR5: Parser Extension

The constraint parser must be extended to recognize anchor references via `ConstraintProperty::from_str()`.

Current property parsing:
```rust
match s {
    "center_x" => Some(Self::CenterX),
    "left" => Some(Self::Left),
    // etc.
    _ => None,
}
```

Extended parsing (underscore suffix pattern):
```rust
match s {
    "center_x" => Some(Self::CenterX),
    "left" => Some(Self::Left),
    // etc.
    _ if s.ends_with("_x") => {
        let anchor_name = &s[..s.len() - 2];
        Some(Self::AnchorX(anchor_name.to_string()))
    }
    _ if s.ends_with("_y") => {
        let anchor_name = &s[..s.len() - 2];
        Some(Self::AnchorY(anchor_name.to_string()))
    }
    _ => None,
}
```

This approach requires no grammar changes—anchor properties are recognized as a fallback when the property string ends with `_x` or `_y` and isn't a built-in property.

### FR6: Backward Compatibility

Existing constraint syntax must continue to work unchanged.

- All current constraint patterns remain valid
- No changes to existing element property references
- Anchor references are additive, not replacing

### FR7: Template Self-Reference

Within a template, anchors can be referenced using the standard syntax with anchor name suffixes.

```ail
template "component" {
    rect body [...]
    anchor output [position: body.right, direction: right]

    // Reference own anchor using component name (after resolution)
    // This works because template children are prefixed with instance name
    constrain internal_element.left = output_x + 5
}
```

Note: Since templates don't know their instance name at definition time, internal anchor references may need special handling. This can be addressed by treating unprefixed anchor properties as local references during constraint collection within templates.

## Success Criteria

- Constraints can reference anchor positions using `instance.anchor_x` and `instance.anchor_y` syntax
- Anchor references work with both non-rotated and rotated template instances
- Anchor references on rotated templates use post-rotation coordinates (integration with Feature 010 two-phase solver)
- Clear error messages for invalid anchor references (unknown anchor, wrong element type, missing coordinate)
- Parser correctly distinguishes between element properties (`.left`, `.center_x`) and anchor references (`.anchor_name_x`)
- Existing constraint syntax continues to work unchanged
- The `person-rotation.ail` example continues to render correctly (regression)
- The MOSFET driver example can be updated to use anchor-based alignment for the flyback diode

## Key Entities

### AnchorReference

A reference to an anchor position within a constraint expression, containing:
- Instance identifier
- Anchor name
- Coordinate selector (x or y)

### ResolvedAnchorPosition

The computed position of an anchor after local solving and optional rotation transformation.

## Assumptions

- Anchors are always defined with an unambiguous position (via element properties or explicit coordinates)
- Anchor names are unique within a template (enforced by existing anchor definition logic)
- The `.x` and `.y` notation is unambiguous because no element property uses these names
- Performance impact of anchor resolution is negligible (anchors are already computed for connection routing)

## Dependencies

- **Feature 010 (Local/Global Solver Separation)**: Anchor transformation for rotated templates
- **Feature 005 (Constraint Solver)**: This feature extends the constraint parser and solver

## Out of Scope

- Anchors on simple shapes (rect, circle, etc.) - only templates define custom anchors
- Anchor direction in constraints (only position is accessible)
- Mathematical operations on anchor references (e.g., `midpoint(a.anchor1, b.anchor2)`) - may be added later
- Dynamic anchor creation based on constraints
