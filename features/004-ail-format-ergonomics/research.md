# Research: AIL Format Ergonomics

## Overview

Research findings for implementing AIL format improvements: better defaults, unified labels, and cross-hierarchy alignment.

---

## R1: Default Styles Analysis

### Question
What are the current defaults and what should better defaults look like?

### Findings

**Current State:**
- `LayoutConfig` has default sizes (rect: 80x30, circle: radius 25, ellipse: 80x45)
- `ResolvedStyles::with_defaults()` provides: fill #f0f0f0, stroke #333333, stroke_width 2, font_size 14
- However, these defaults are only applied by the renderer explicitly, not automatically

**Proposed Improvement:**
The defaults are already reasonable. The issue is ensuring they're consistently applied when no modifiers present. This is mostly an implementation concern in the layout engine and renderer, not a language change.

### Decision
Keep current default values. Ensure renderer/layout engine apply them consistently when modifiers are absent.

### Rationale
Existing defaults are sensible. No need to change values, just ensure consistent application.

---

## R2: Label Unification Strategy

### Question
How should labels be unified as shapes rather than special constructs?

### Findings

**Current Label Systems:**
1. `[label: "text"]` modifier on shapes/connections - works via StyleKey::Label
2. `label { ... }` block inside groups - works via Statement::Label wrapper
3. `label: <stmt>` inline inside groups - also via Statement::Label wrapper

**Problems with Current Approach:**
- Labels are a special-cased statement type
- Label content is arbitrary (any statement), but typically just text
- Groups have no other way to have a "header" or "title"

**Options Considered:**

**Option A: Remove `label` keyword entirely**
- Use `role: label` modifier on any shape
- Layout engine looks for `role: label` children to position specially
- Pros: Simpler, uses existing shape system
- Cons: Breaking change for existing files

**Option B: Keep both, deprecate `label` keyword**
- `label { }` continues working with deprecation warning
- New `role: label` modifier is the preferred approach
- Pros: Backward compatible
- Cons: Two ways to do same thing

**Option C: Redefine `label` as syntax sugar**
- `label { text "Foo" }` becomes sugar for `text "Foo" [role: label]`
- Parser transforms one to the other
- Pros: Clean migration path, backward compatible
- Cons: Adds parsing complexity

### Decision
**Option B**: Keep both, deprecate `label` keyword. Add `role: label` modifier support.

### Rationale
- Maintains backward compatibility (existing .ail files continue working)
- Clear migration path for users
- Deprecation warning educates users about new syntax
- Simpler implementation than Option C

---

## R3: Connection Label as Shape Reference

### Question
How should connection labels accept shapes instead of just text?

### Findings

**Current Implementation:**
- Connection labels are just `[label: "text"]` modifiers
- StyleKey::Label with StyleValue::String
- Layout engine creates LabelLayout from text

**Proposed Enhancement:**
The `label` modifier on connections should accept:
1. String literal (legacy): `[label: "HTTP"]` → auto-wrap in text shape
2. Identifier reference: `[label: my_label]` → use existing shape `my_label`
3. Inline shape: `[label: { text "HTTP" [fill: blue] }]` → define shape inline

**Implementation Approach:**
- Extend StyleValue to include ShapeRef variant
- Parser recognizes identifier or inline block as label value
- Layout engine resolves shape reference to get text/position

### Decision
Extend StyleValue to support shape references. Inline shape syntax deferred to future if needed.

### Rationale
- Reference by identifier is simpler and covers most use cases
- Inline shape syntax adds parser complexity
- Users can define the label shape separately then reference it

---

## R4: Cross-Hierarchy Alignment Syntax

### Question
What syntax and semantics for aligning elements across hierarchies?

### Findings

**Requirements:**
- Align elements from different groups
- Support horizontal (left, center, right) and vertical (top, center, bottom) alignment
- Position offset relative to alignment when both specified

**Syntax Options:**

**Option A: Separate `align` statement**
```
align header_left.horizontal_center = header_right.horizontal_center
```

**Option B: Extend `place` constraint**
```
place header_left aligned-with header_right horizontal-center
```

**Option C: Alignment as modifier**
```
rect header_left [align_to: header_right.horizontal_center]
```

**Analysis:**
- Option A is clearest and most explicit
- Option B conflates positioning and alignment
- Option C embeds alignment in shape definition

### Decision
**Option A**: Separate `align` statement.

### Rationale
- Clear separation of concerns
- Follows "explicit over implicit" principle
- Easy to understand what the statement does
- Can chain multiple elements: `align a.top = b.top = c.top`

---

## R5: Element Path Syntax

### Question
How to reference nested elements?

### Findings

**Requirement:**
Reference elements inside groups: `group1.subgroup.element`

**Considerations:**
- Elements have optional names (identifiers)
- Paths need to resolve through group hierarchy
- Anonymous elements can't be referenced by path

**Implementation:**
- Paths are dot-separated identifiers
- Resolution walks from document root through named containers
- Error if any path segment doesn't resolve

**Syntax:**
```
ElementPath = Identifier ("." Identifier)*
```

### Decision
Dot-separated path syntax with resolution from document root.

### Rationale
- Familiar syntax (similar to object property access)
- Unambiguous resolution
- Error messages can point to failing segment

---

## R6: Position Relative to Alignment

### Question
How should position and alignment interact?

### Findings

**Current Behavior:**
- `place` constraints are absolute positioning hints
- No alignment system exists

**Proposed Behavior:**
- Alignment runs first (adjusts positions)
- Position constraints then apply as offsets
- `place a [x: 10]` after `align a.left = b.right` → a positioned 10px right of alignment

**Implementation:**
- Layout engine: compute initial layout
- Alignment pass: apply alignment constraints
- Position offset pass: apply place constraints as deltas

### Decision
Sequential application: alignment first, then position as offset.

### Rationale
- Clear mental model
- Predictable behavior
- Position becomes "fine-tuning" after alignment

---

## R7: AST Changes Required

### Question
What AST modifications are needed?

### Findings

**New Types Needed:**
1. `AlignmentDecl` - alignment constraint statement
2. `ElementPath` - dot-separated path to element
3. `Edge` - alignment edge enum (Left, HorizontalCenter, Right, Top, VerticalCenter, Bottom)
4. `Role` - shape role enum (Label, Content, etc.)

**Modified Types:**
1. `StyleKey` - add `Role` variant
2. `StyleValue` - add `ShapeRef` variant for label references
3. `Statement` - add `Alignment(AlignmentDecl)` variant

**Grammar Changes:**
1. Add `align` keyword
2. Add `.` operator for paths
3. Add edge keywords (left, right, horizontal_center, etc.)

### Decision
Proceed with AST additions as listed.

---

## Summary of Decisions

| Item | Decision |
|------|----------|
| Defaults | Keep current values, ensure consistent application |
| Label unification | Option B: Both syntaxes, deprecate old |
| Connection labels | Extend to support shape references |
| Alignment syntax | Separate `align` statement |
| Element paths | Dot-separated from root |
| Position + alignment | Sequential: align first, position as offset |
| AST changes | New types for alignment, extend StyleKey/Value |

---

*Created: 2026-01-23*
