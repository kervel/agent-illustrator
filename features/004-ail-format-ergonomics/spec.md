---
parent_branch: main
feature_number: "004"
status: In Progress
created_at: 2026-01-23T00:00:00+00:00
---

# Feature: AIL Format Ergonomics

## Overview

Improve the Agent Illustrator Language (AIL) format to be more ergonomic: better defaults so fewer modifiers are needed, unify label handling, and enable cross-hierarchy element alignment. These changes reduce verbosity while increasing the language's expressiveness.

**Key Design Goals:**
- Reduce token count for common patterns (AI agents pay per-token)
- Eliminate special-case constructs where general mechanisms suffice
- Enable precise control when needed without forcing complexity on simple cases

## Clarifications

### Session 2026-01-23
- Q: Should group labels be special? → A: No. A label is just a shape. The current `label { ... }` or `label:` construct for groups should be removed in favor of placing any shape element in the first position or using a dedicated naming/reference pattern.
- Q: How should connector labels work? → A: Connector labels should accept a shape reference instead of just text. This allows styled text, icons, or even complex label content.
- Q: Can elements align across hierarchy boundaries? → A: Yes. We want `align` statements that can reference any element by path/id regardless of hierarchy position.
- Q: What about position + alignment? → A: When both an alignment and a position are specified, the position is treated as relative to the alignment anchor.

## User Scenarios

### Scenario 1: Simple Diagram Without Excessive Modifiers
An AI agent generates a basic server-client diagram. With better defaults, common styling and sizing choices are implicit.

**Before (verbose):**
```
circle server [fill: #f0f0f0, stroke: #333333, stroke_width: 2, size: 30]
circle client [fill: #f0f0f0, stroke: #333333, stroke_width: 2, size: 30]
server -> client [stroke: #333333, stroke_width: 2]
```

**After (with defaults):**
```
circle server
circle client
server -> client
```

**Acceptance Criteria:**
- Default styles are sensible and produce readable output
- Explicit modifiers override defaults where specified
- Default sizes for shapes are proportionate and consistent

### Scenario 2: Group with Custom Label Shape
An agent creates a group with a styled label using standard shape syntax, not a special label construct.

**Before (special construct):**
```
group mygroup {
    label { text "Section A" [font_size: 18, fill: red] }
    row content { rect a; rect b }
}
```

**After (unified shape approach):**
```
group mygroup {
    text "Section A" [font_size: 18, fill: red, role: label]
    row content { rect a; rect b }
}
```

Or with automatic first-text-is-label convention:
```
group mygroup [label_shape: first_text] {
    text "Section A" [font_size: 18, fill: red]
    row content { rect a; rect b }
}
```

**Acceptance Criteria:**
- Groups no longer require special `label` syntax
- Any shape can serve as a label with appropriate modifier
- Backward compatibility: existing `label { }` syntax produces deprecation warning but works

### Scenario 3: Connection with Complex Label
An agent creates a connection with a label that includes styling or is an icon.

**Before (text-only):**
```
server -> client [label: "HTTP"]
```

**After (shape reference):**
```
server -> client [label: lbl]
text "HTTP" lbl [fill: blue, font_size: 10]
```

Or inline:
```
server -> client [label: { text "HTTP" [fill: blue] }]
```

**Acceptance Criteria:**
- Connection labels can reference any shape by identifier
- Connection labels can be inline shape declarations
- Shape-based labels allow full styling control

### Scenario 4: Cross-Hierarchy Alignment
An agent needs to align elements from different groups horizontally or vertically.

**Example:**
```
group left_panel {
    rect header_left [width: 100]
    rect content_left
}
group right_panel {
    rect header_right [width: 100]
    rect content_right
}
// Align headers across groups
align header_left.horizontal_center = header_right.horizontal_center
align content_left.top = content_right.top
```

**Acceptance Criteria:**
- Elements can be referenced across group hierarchies
- Horizontal alignment options: left, center (horizontal_center), right
- Vertical alignment options: top, center (vertical_center), bottom
- Multiple elements can be aligned in a single statement

### Scenario 5: Position Relative to Alignment
An agent specifies an element's alignment anchor, then adds an offset.

**Example:**
```
rect anchor
rect offset_element
align offset_element.left = anchor.right
// offset_element is aligned to anchor's right edge
// then shifted 10px further right
place offset_element [x: 10]  // relative to alignment
```

**Acceptance Criteria:**
- When alignment is specified, position values are offsets from the aligned position
- If no alignment, position values are absolute (current behavior)
- Clear error messages when alignment targets don't exist

## Functional Requirements

### FR-1: Sensible Default Styles
The renderer must apply sensible defaults so minimal DSL produces usable output.

**Requirement:** Shapes render with visible, readable defaults without explicit styling.
**Default Values:**
- `fill`: Light gray (#f0f0f0) for closed shapes, none for lines
- `stroke`: Dark gray (#333333)
- `stroke_width`: 2
- `font_size`: 14
- `size`: Shape-type appropriate (rect: 80x50, circle: 30, etc.)

**Testable Criterion:** Parsing `rect mybox` and rendering produces a visible rectangle with default styling.

### FR-2: Default Shape Sizes
Shapes must have appropriate default dimensions.

**Requirement:** Each shape type has a default size that is proportionate and usable.
**Defaults:**
- `rect`: 80w x 50h
- `circle`: 30 diameter
- `ellipse`: 60w x 40h
- `text`: intrinsic to content

**Testable Criterion:** `circle c1` without size modifier renders with diameter 30.

### FR-3: Unified Label Handling for Groups
Remove special `label` construct; labels are shapes with a role modifier.

**Requirement:** Groups use regular shapes as labels via `role: label` modifier or configuration.
**Testable Criteria:**
- `group g { text "Title" [role: label]; rect a }` positions text as group label
- Old `label { }` syntax works but emits deprecation warning
- Labels position above/around the group content (configurable)

### FR-4: Shape-Based Connection Labels
Connection labels accept shape references instead of plain text.

**Requirement:** The `label` modifier on connections can reference a shape or contain inline shape declaration.
**Testable Criteria:**
- `a -> b [label: mylabel]` uses shape `mylabel` as the label
- `a -> b [label: { text "Hi" [fill: red] }]` uses inline styled text
- Legacy `[label: "text"]` syntax continues working (auto-wraps in text shape)

### FR-5: Cross-Hierarchy Alignment Syntax
New `align` statement enables alignment between any elements.

**Requirement:** Elements can be aligned on horizontal or vertical axes regardless of hierarchy.
**Syntax:**
```
align <element>.<edge> = <element>.<edge>
```
**Edge options:**
- Horizontal: `left`, `horizontal_center`, `right`
- Vertical: `top`, `vertical_center`, `bottom`

**Testable Criterion:** `align a.horizontal_center = b.horizontal_center` centers element `a` horizontally relative to `b`.

### FR-6: Multi-Element Alignment
Align multiple elements in one statement.

**Requirement:** Alignment can chain multiple elements.
**Syntax:**
```
align <e1>.<edge> = <e2>.<edge> = <e3>.<edge>
```

**Testable Criterion:** `align a.top = b.top = c.top` aligns all three elements' tops to the same y-coordinate.

### FR-7: Position Relative to Alignment
When alignment and position both specified, position is offset from alignment.

**Requirement:** Position modifiers/constraints become relative after alignment.
**Testable Criteria:**
- Without alignment: `place a [x: 100]` positions at x=100 (absolute)
- With alignment: after `align a.left = b.right`, `place a [x: 10]` positions a at b.right + 10

### FR-8: Element Path References
Enable referencing nested elements via path syntax.

**Requirement:** Dotted path syntax for deep element references.
**Syntax:** `group1.subgroup.element`

**Testable Criterion:** `align g1.content.header.left = g2.sidebar.top_element.left` resolves and aligns correctly.

### FR-9: Backward Compatibility
Existing AIL files should continue working.

**Requirement:** All current syntax remains valid; new features are additive.
**Testable Criteria:**
- Existing `[label: "text"]` on shapes/connections works
- Existing `label { }` on groups works (with deprecation warning)
- Existing `place` constraints work unchanged when no alignment present

## Success Criteria

1. **Reduced Verbosity**: Common patterns require 50% fewer tokens than current syntax
2. **First-Attempt Correctness**: Defaults produce usable output; AI agents need not specify styling for simple diagrams
3. **Unified Model**: Labels use the same shape system as everything else
4. **Precise Control**: Cross-hierarchy alignment enables layouts not currently possible
5. **Backward Compatible**: All existing .ail files parse and render correctly

## Key Entities

### AlignmentConstraint
A constraint that aligns edges of two or more elements.

**Fields:**
- `elements`: List of (element_path, edge) pairs
- `edge_type`: Horizontal or Vertical

### ElementPath
A dotted path reference to an element, potentially nested.

**Example:** `"group1.row.item"` → resolves to element `item` inside `row` inside `group1`

### Edge
An alignment edge on an element's bounding box.

**Variants:**
- `Left`, `HorizontalCenter`, `Right` (horizontal axis)
- `Top`, `VerticalCenter`, `Bottom` (vertical axis)

### LabelRole (modifier value)
Marks a shape as serving the label role for its parent container.

## Assumptions

1. **Element IDs are unique within scope**: Paths resolve unambiguously
2. **Layout runs before alignment**: Alignment adjusts positions computed by layout
3. **Alignment is constraint-based**: May require iterative solving for complex cases
4. **Defaults are overridable**: Explicit modifiers always take precedence

## Technical Boundaries

This feature covers:
- Grammar extensions for `align` statements
- Grammar changes for shape-based labels
- AST types for alignment constraints and element paths
- Layout engine changes to apply alignment after initial layout
- Default value configuration in renderer

This feature does NOT cover:
- Animation or transition between aligned states
- Automatic label positioning algorithms (just position at role)
- Constraint solver for circular alignment dependencies (error case)
