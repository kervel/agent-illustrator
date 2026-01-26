---
parent_branch: main
feature_number: 009
status: In Progress
created_at: 2025-01-26T22:15:00+01:00
---

# Feature 009: Anchor Support for Shape Connections

## Overview

Add named anchor points to shapes that allow connectors to attach at specific, predictable positions. Currently, connections attach to shape centers or bounding box edges automatically. This feature enables explicit control over connection attachment points using syntax consistent with the existing constraint system.

Anchors provide:
- **Simple shapes** (rect, ellipse, circle): 4 anchors at edge midpoints (top, bottom, left, right)
- **Path shapes**: 8 anchors at edge midpoints + corners (top, bottom, left, right, top_left, top_right, bottom_left, bottom_right)
- **Templates**: Custom named anchors defined by the template author

## User Scenarios

### Scenario 1: Basic Shape Anchors
A user creates a flowchart where arrows must connect at specific sides of boxes.

**Current behavior**: `box_a -> box_b` connects center-to-center, with the renderer choosing attachment points.

**With anchors**: `box_a.right -> box_b.left` explicitly connects the right side of box_a to the left side of box_b.

### Scenario 2: Loop-back Connections
A user wants a feedback arrow that exits the top of a shape and enters the top of another shape (for visual clarity in a cycle diagram).

**Example**: `assign.top -> evaluate.top [routing: curved, via: via_point]`

### Scenario 3: Template with Custom Anchors
A user creates a "server" template with anchors for "input" (left), "output" (right), and "status" (top).

```
template "server" {
  rect body [width: 80, height: 60, label: "Server"]
  anchor input [position: body.left]
  anchor output [position: body.right]
  anchor status [position: body.top]
}

server app_server
server db_server

app_server.output -> db_server.input
monitor -> app_server.status
```

### Scenario 4: Person Template with Semantic Anchors
The existing person template could define anchors like "hand_left", "hand_right", "head_top" for connecting speech bubbles or interactions.

## Functional Requirements

### FR1: Built-in Anchors for Simple Shapes
- **FR1.1**: Rect, ellipse, and circle shapes automatically have 4 anchors: `top`, `bottom`, `left`, `right`
- **FR1.2**: Anchor positions are computed as the midpoint of each bounding box edge
- **FR1.3**: Anchors are accessible via dot notation: `shape_name.anchor_name`

### FR2: Built-in Anchors for Path Shapes
- **FR2.1**: Path shapes automatically have 8 anchors: `top`, `bottom`, `left`, `right`, `top_left`, `top_right`, `bottom_left`, `bottom_right`
- **FR2.2**: Edge anchors (top, bottom, left, right) are at bounding box edge midpoints
- **FR2.3**: Corner anchors are at bounding box corners

### FR3: Custom Anchors in Templates
- **FR3.1**: Templates can define custom anchors using an `anchor` statement
- **FR3.2**: Anchor positions are defined using constraint-like syntax referencing template elements
- **FR3.3**: Template anchors are accessible on instances: `instance_name.anchor_name`
- **FR3.4**: Custom anchors can have arbitrary names (following identifier rules)
- **FR3.5**: Templates can define anchors relative to any internal element's properties

### FR4: Connection Syntax with Anchors
- **FR4.1**: Connection syntax supports anchors: `source.anchor -> target.anchor [modifiers]`
- **FR4.2**: Mixed syntax is allowed: `source.anchor -> target` (anchor to auto-detect)
- **FR4.3**: Existing connection syntax remains valid: `source -> target` (both auto-detect)
- **FR4.4**: Invalid anchor names produce clear error messages with suggestions

### FR5: Anchor Declaration Syntax
- **FR5.1**: Anchors in templates use: `anchor name [position: element.property]`
- **FR5.2**: Position can reference any valid constraint property (left, right, top, bottom, center_x, center_y, etc.)
- **FR5.3**: Position can use expressions: `anchor mid_upper [position: body.top + 10]`

### FR6: Layout Container Anchor Propagation
- **FR6.1**: Layout containers (row, col, stack) with names inherit anchors from their bounding box (4 anchors)
- **FR6.2**: Nested element anchors are accessible via path: `container.element.anchor`

## Success Criteria

- Users can specify exact connection attachment points without workarounds
- Connection routing produces cleaner diagrams when anchors are specified
- Template authors can expose meaningful connection points for their templates
- Existing diagrams without anchor syntax continue to work unchanged
- Error messages for invalid anchors help users discover valid anchor names
- The feedback-loops example can be rewritten to use anchors instead of invisible via points for cleaner syntax

## Key Entities

### Anchor
- **name**: Identifier for the anchor (e.g., "top", "left", "input")
- **position**: Computed (x, y) coordinate relative to the owning shape
- **direction**: The outward normal direction at this anchor (connectors should approach/leave perpendicular to the shape)
- **owner**: Reference to the shape or template instance that owns this anchor

### AnchorReference
- **element**: The shape or template instance being referenced
- **anchor_name**: The name of the anchor on that element

### BuiltinAnchorSet
- **simple_shape_anchors**: [top, bottom, left, right]
- **path_anchors**: [top, bottom, left, right, top_left, top_right, bottom_left, bottom_right]

## Assumptions

- Anchor names follow the same identifier rules as shape names (alphanumeric + underscore)
- Built-in anchor names are reserved and cannot be overridden by custom anchors in templates
- Anchor positions are computed after layout resolution (they depend on final element positions)
- When both source and target have anchors specified, the connection routing respects both attachment points and directions
- Connectors should approach/leave anchors perpendicular to the shape edge (following the anchor's direction)
- For orthogonal routing: this is strictly enforced; the first/last segment must align with anchor direction
- For curved routing: direction is a soft guide (not enough degrees of freedom to enforce strictly)
- For direct routing: direction is not enforced (straight line)
- The existing auto-detection logic remains as fallback when anchors are not specified
- Corner anchors for paths are useful for diagonal connections and complex routing scenarios
- **Template anchors are shapes from layouting perspective**: Custom anchors in templates behave like invisible reference points that participate in the constraint system. They can be positioned using constraints just like any other element.
- Numeric offsets in anchor expressions follow existing constraint conventions: positive X = rightward, positive Y = downward
