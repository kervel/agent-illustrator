---
parent_branch: 009-mosfet-driver-example
feature_number: 010
status: In Progress
created_at: 2026-01-28T14:30:00+01:00
---

# Feature: Local/Global Constraint Solver Separation

## Clarifications

### Session 2026-01-28
- Q: When external constraint references exported element (e.g., `c1_body.right`), local or global coords? → A: Property references (`.left`, `.right`, etc.) refer to the axis-aligned bounding box AFTER rotation. For a rotated element, `.right` is the rightmost x-coordinate of the post-rotation bounding box, not the original right edge.
- Q: How handle top-level elements with rotation? → A: Same as templates in principle (local solve, rotate, global solve). Simple shapes have nothing to solve locally, but paths with vertices may need investigation.
- Q: Can external constraints reference path vertices? → A: Defer investigation to planning phase. Unknown if currently supported.
- Q: How to compute bounding box of complex paths (arcs, curves) after rotation? → A: Use "loose bounds" approach: compute unrotated AABB, rotate its 4 corners, take AABB of those corners. Simpler than finding mathematical extrema, consistent with CSS/SVG behavior.
- Q: What should happen when the local solver encounters an unsolvable constraint within a template? → A: Fail fast - abort entire render with clear error message pointing to the template.
- Q: How should the rotation center be determined for template instances? → A: Use the geometric center of the bounding box enclosing all child elements (child bounds center).

## Overview

Refactor the constraint solver from a single-pass global system to a two-phase architecture: local solvers handle template-internal constraints first, then rotation transformations are applied, and finally a global solver handles external constraints. This enables proper template rotation support where anchors, via points, and external constraint references all work correctly with rotated coordinates.

Currently, template rotation (Feature 006) only applies SVG transforms at render time - the layout engine operates entirely on pre-rotation coordinates. This causes:
- Anchors to point to wrong positions after rotation
- Via points in curves to reference pre-rotation element centers
- External constraints referencing rotated template children to break after rotation

## Problem Statement

### Current Architecture (Single-Pass)

1. All constraints (internal + external) collected together
2. Single constraint solver processes everything
3. Anchor positions calculated from solved bounds
4. Connection routing uses anchor positions
5. SVG render applies rotation transforms visually

### Why This Breaks Rotation

When `resistor r2 [rotation: 90]` is specified:
- The solver positions `r2_body` at coordinates (x, y)
- Rotation transform visually moves the element to a new position
- But anchors still reference the pre-rotation (x, y) position
- Connections attach to the wrong location
- External constraints like `foo.left = r2_body.right + 10` are violated after rotation

### Current Workaround (Being Removed)

BUG-001 was partially addressed with a prefix-based hack (`elements_share_parent_prefix()`) to detect template-internal constraints. This hack:
- Adds complexity to the constraint solver
- Doesn't solve the rotation problem
- Creates confusion about constraint ownership
- Will be replaced by the clean local/global separation

## Proposed Solution

### Two-Phase Solver Architecture

**Phase 1: Local Solving**
- For each template instance, run a local constraint solver
- Solve only internal constraints (constraints between elements within the same template)
- Results are in the template's local coordinate system
- Store local bounds and anchor positions

**Phase 2: Rotation Application**
- For template instances with `rotation` modifier
- Transform all child element bounds around the instance center
- Transform anchor positions and directions
- Update bounds to reflect rotated bounding box

**Phase 3: Global Solving**
- Run global constraint solver
- Solve external constraints (constraints between different templates, or between templates and top-level elements)
- Uses rotated bounds from Phase 2
- Positions template instances relative to each other

**Phase 4: Connection Routing**
- Route connections using transformed anchor positions
- Via points reference rotated element centers
- All coordinates are in final global space

## User Scenarios

### Scenario 1: Vertical Resistor in Circuit

A user creates a circuit diagram where some resistors need to be horizontal and others vertical.

```ail
template "resistor" (value: "R") {
    rect body [width: 40, height: 16, ...]
    anchor left_conn [position: body.left, direction: left]
    anchor right_conn [position: body.right, direction: right]
}

resistor r_horizontal [value: "10k"]
resistor r_vertical [value: "20k", rotation: 90]

constrain r_vertical.top = r_horizontal.bottom + 50
r_horizontal.right_conn -- r_vertical.left_conn
```

The vertical resistor appears rotated 90 degrees, and the connection attaches to the correct (rotated) anchor positions.

### Scenario 2: External Constraint to Rotated Child

A user aligns an external element to an exported child of a rotated template.

```ail
template "component" {
    rect body [width: 80, height: 40]
    export body
}

component c1 [rotation: 45]
rect label

constrain label.left = c1_body.right + 10
```

The label aligns 10px to the right of the **axis-aligned bounding box** of the rotated `c1_body`. For a 45° rotation of an 80x40 box, the bounding box is approximately 85x85, and `.right` refers to the rightmost x-coordinate of this enclosing box.

### Scenario 3: Curve with Via Point in Rotated Template

A user creates a curve that passes through a control point inside a rotated template.

```ail
template "waypoint" {
    circle marker [size: 6]
}

rect start
rect end
waypoint ctrl [rotation: 30]

start -> end [routing: curved, via: ctrl_marker]
```

The curve passes through the rotated position of `ctrl_marker`.

## Functional Requirements

### FR1: Local Constraint Isolation

Template-internal constraints must be solved in isolation before external constraints.

- Constraints between elements sharing the same template prefix are "local"
- Local solver runs per template instance
- Local results are in template-local coordinate system (origin at template instance position)

### FR2: Rotation Transformation

After local solving, rotation must be applied to all local results.

- Transform element bounds around the template instance center
- Transform anchor positions using rotation matrix
- Transform anchor directions (left→up for 90°, etc.)
- Update bounding box using "loose bounds" algorithm (see FR2a)

### FR2a: Loose Bounding Box Computation

Post-rotation bounding boxes use the "loose bounds" approach for simplicity and consistency.

- Compute the unrotated axis-aligned bounding box (AABB)
- Rotate the 4 corners of this box around the rotation center
- Compute the AABB of the 4 rotated corner points
- This result is the post-rotation bounding box for constraint purposes

**Rationale**: Computing mathematically tight bounds for curves/arcs after rotation requires finding derivative roots - complex and slow. Loose bounds are simpler, faster, and match how CSS/SVG transforms work. The over-estimation is acceptable for layout positioning.

### FR3: Coordinate System Semantics

Property references (`.left`, `.right`, `.top`, `.bottom`, `.center_x`, `.center_y`) refer to the **axis-aligned bounding box after rotation**.

- For a rotated element, `.right` is the rightmost x-coordinate of the post-rotation bounding box
- This is NOT the original right edge transformed - it's the enclosing axis-aligned box
- Changing rotation angle MAY require adjusting constraints (the bounding box shape changes)
- This is necessary because the original `.right` edge is no longer a single x-coordinate after rotation

**Important distinction from anchors**: Anchors transform the actual point (the original anchor position rotated around the center). Property references use the axis-aligned bounding box.

### FR4: Global Constraint Resolution

External constraints operate on post-rotation axis-aligned bounding boxes.

- Template instance bounds (e.g., `r2.left`) reflect the post-rotation bounding box
- Exported element bounds (e.g., `r2_body.right`) also reflect post-rotation bounding box
- Global solver sees all coordinates in global space after rotation
- A 100x50 box rotated 90° has a 50x100 bounding box for constraint purposes

### FR5: Anchor Transformation

Anchors must be transformed for connection routing.

- Anchor positions rotated around template center
- Anchor directions rotated (e.g., `direction: left` becomes `direction: up` for 90° rotation)
- Connection routing uses transformed anchors

### FR6: Via Point Transformation

Via points for curved routing must use transformed positions.

- `via: element_name` resolves to the rotated center of the element
- Curves pass through visually correct positions

### FR7: Remove Prefix-Based Hack

The `elements_share_parent_prefix()` workaround must be removed.

- Clean separation replaces the hack
- Simplifies constraint solver code
- Removes potential edge cases in prefix matching

### FR8: Backward Compatibility

Existing AIL files without rotation must produce **byte-identical SVG output**.

- Single-phase behavior preserved when no rotation involved
- No changes to constraint syntax
- No changes to anchor or connection syntax
- All examples in `/examples/` directory must render identically before and after this refactor
- Regression test: capture SVG output of all examples before refactor, compare after refactor

## Success Criteria

- Template instances with rotation have correctly positioned anchors
- Connections to rotated templates attach at visually correct positions
- External constraints referencing rotated template children work correctly
- Via points in curves route through rotated element positions
- Changing rotation angle does not require rewriting constraints
- The `elements_share_parent_prefix()` hack is removed from the codebase
- Existing test suite passes without modification
- **All examples in `/examples/` produce byte-identical SVG output** (regression test)
- New tests cover rotation scenarios for anchors, connections, and external constraints

## Key Entities

### LocalSolverResult

Contains the solved positions of elements within a template instance, in local coordinates.

### RotationTransform

Represents a rotation to be applied to a template instance, including angle and center point.

### GlobalSolverContext

Context for the global solving phase, containing transformed bounds from all template instances.

### TransformedAnchor

An anchor that has been transformed from local to global coordinates, including rotated position and direction.

## Assumptions

- All template instances have a well-defined center point for rotation: the geometric center of the bounding box enclosing all child elements
- Rotation angles are specified in degrees (consistent with Feature 006)
- Clockwise positive rotation convention (consistent with SVG and Feature 006)
- Two-phase solving adds acceptable overhead for typical diagram sizes (50-500 elements)
- External constraints referencing non-exported elements remain an error (no change from current behavior)
- Layout containers (row, col, stack) inside templates are resolved during local solving
- Top-level rotated elements follow the same local/global pattern (trivial local phase for simple shapes)
- Unsolvable constraints within a template cause immediate failure with clear error message (fail fast)

## Investigation Required

- **Path vertex references**: Determine during planning whether external constraints can reference path vertices (e.g., `mypath.vertex_name.x`). If supported, define how rotation affects these references.

## Dependencies

- **Feature 005 (Constraint Solver)**: This feature refactors the existing solver
- **Feature 006 (Arbitrary Rotation)**: This feature extends rotation to work with templates
- **Feature 009 (BUG-001)**: The prefix-based hack being replaced was added in this feature

## Out of Scope

- Nested template rotation (templates containing rotated templates) - deferred to future feature
- Animated rotation or transitions
- Rotation interpolation or tweening
- Non-uniform scaling combined with rotation
