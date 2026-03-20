---
parent_branch: main
feature_number: 011
status: In Progress
created_at: 2026-03-20T00:00:00+01:00
---

# Feature: Keyframe Animation System

## Overview

Agent Illustrator can produce static diagrams that are correct on the first attempt, but there is no way to create animated sequences — step-by-step visual narratives like "request flows to API, response comes back, tool executes." The z_order and CSS class features (shipped in v0.1.18+) enable external CSS animations on SVG groups, but agents cannot verify animations: they can only take screenshots of static frames.

The keyframe system adds a declarative animation primitive to the language. All elements are laid out globally with the existing constraint system. A separate `keyframe` section describes temporal visibility and per-frame transforms. The renderer produces a single SVG where each element exists once, with generated CSS that controls position/visibility per frame. User-authored CSS handles transitions between frames.

This design ensures: (1) agents can verify each frame as a static snapshot, (2) the linter can check per-frame visibility to avoid false positive overlap warnings, and (3) smooth CSS transitions eliminate jarring jumps between frames.

## Clarifications

### Session 2026-03-20
- Q: Should connections to transformed elements auto-reroute, or only if explicitly mentioned? → A: Auto-reroute always. Any connection touching a transformed element gets rerouted, even if the connection isn't mentioned in the keyframe.
- Q: What happens when a keyframe references a nonexistent element? → A: Hard error. Parse/compile error, refuse to render.
- Q: Can transform and hide be combined on the same element in one keyframe? → A: Yes, both apply. Transform sets position/style, hide sets opacity to 0. Both participate in the solver. CSS transitions can animate slide-then-fade.
- Q: Should the SVG include a built-in frame-switching mechanism? → A: Opt-in embedded JS. A `--animate` flag embeds a minimal inline script for click/timer playback. Without the flag, bare SVG with frame classes only.

## Design Decisions from Brainstorming

These decisions were made during the initial design conversation and should be treated as requirements:

1. **Separation of layout and animation** — Keyframes live in a separate section from element declarations. They reference layout elements by name, they do not contain element definitions.

2. **Default visibility** — Without keyframes, everything is visible (backward compatible). The first keyframe typically hides most things to set up the opening scene.

3. **Cumulative keyframes** — Each keyframe applies diffs on top of the previous frame's state. This captures animation intent naturally ("and then this appears") without repeating the full visible set each time.

4. **Per-keyframe constraint solving** — Each keyframe gets its own full constraint resolution pass. If a box is rotated in a keyframe, arrows leading to it get re-routed correctly. Elements not explicitly mentioned in a keyframe stay pinned to their global positions to prevent unintended relayout.

5. **Named connections** — Connections can be named so keyframes can reference them (e.g., `cli.right -> llm.left as req_arrow`). This also enables connection styling and constraining by name beyond keyframes.

6. **Diff-based CSS output** — The renderer solves layout per keyframe, diffs each against frame 0 (the base), and emits CSS overrides only for properties that changed. Elements exist once in the SVG body at their base positions. CSS classes on a wrapper element drive frame switching.

## User Scenarios

### Scenario 1: Agent creates animated agentic loop

An AI agent receives the prompt "draw the agentic loop as an animation." It writes an AIL file with actors (CLI, LLM, Tool, User) laid out with global constraints. Five keyframes progressively show/hide message envelopes and connection arrows. The agent renders and verifies each frame as a static snapshot. The user adds a CSS file with `transition: all 0.5s` for smooth playback.

### Scenario 2: Agent verifies individual frames

The agent runs `agent-illustrator --frame 2 diagram.ail` to render frame 2 as a static SVG. It takes a screenshot and checks that the right elements are visible and positioned correctly. It repeats for each frame. This workflow requires no animation — each frame is a standalone image.

### Scenario 3: Linter checks per-frame overlap

The linter runs overlap detection per keyframe. In frame 1, elements A and B occupy the same space — but A is hidden, so no warning. In frame 3, both are visible and overlapping — the linter flags it. This eliminates false positives from elements that never coexist visually.

### Scenario 4: Presentation with CSS transitions

A user creates a technical presentation where each keyframe is a build step on a slide. CSS transitions smoothly animate position changes between frames. A button or timer advances frames by toggling a CSS class on the SVG wrapper. The AIL file is the single source of truth; the CSS is minimal glue.

### Scenario 5: Backward compatibility

A user with existing AIL files (no keyframes) sees no change in behavior. All elements are visible, the SVG output is identical. The keyframe system is purely additive.

## Functional Requirements

### FR1: Named connections

- Connections support an optional name via `as` syntax: `a.right -> b.left as my_arrow [stroke: red]`
- Named connections can be referenced in keyframes by name
- Named connections can be referenced in constraints by name
- Unnamed connections continue to work as before

### FR2: Keyframe declaration syntax

- Keyframes are declared after all layout elements and constraints
- Syntax: `keyframe "name" { ... }`
- Each keyframe has a unique name (string)
- Keyframes are ordered: first declared = frame 0, second = frame 1, etc.
- Keyframe bodies contain operations: `show`, `hide`, `transform`

### FR3: Keyframe operations

- `show <element_name>` — make an element visible (opacity 1, or reverting a previous hide)
- `show <connection_name>` — make a named connection visible
- `hide <element_name>` — make an element invisible (opacity 0)
- `hide <connection_name>` — make a named connection invisible
- `transform <element_name> [modifier: value, ...]` — apply per-frame style/position overrides
- Transform modifiers: any existing style modifier (rotation, fill, opacity, x, y, etc.)
- Show/hide accept comma-separated lists: `hide a, b, c`
- Transform and hide may be combined on the same element in one keyframe: transform sets position/style, hide sets opacity to 0. Both participate in the solver. This enables CSS-animated "slide out then disappear" effects.
- Referencing a nonexistent element or connection name in any keyframe operation is a hard error (parse/compile failure, no SVG output)

### FR4: Cumulative state model

- Before any keyframes, all elements and connections are visible (default state)
- Frame 0 = default state + operations from keyframe 0
- Frame N = Frame N-1 + operations from keyframe N
- The linter computes the cumulative visible set per frame for overlap detection

### FR5: Per-keyframe constraint solving

- Each keyframe gets a full constraint resolution pass
- Transform operations that affect position (x, y) or geometry (rotation, width, height) participate in the solver
- Elements not mentioned in the keyframe are pinned to their position from the previous frame
- All connections (named and unnamed) are rerouted per-frame using each frame's solved positions, regardless of whether the connection is mentioned in the keyframe
- This ensures arrows always point correctly when any connected element moves or rotates

### FR6: Diff-based SVG output

- The SVG body contains all elements at their frame-0 solved positions
- Hidden elements in frame 0 get `opacity: 0` in inline style
- The renderer generates CSS classes for each frame (e.g., `.frame-startup`, `.frame-request`)
- Each frame class contains only the property diffs from frame 0
- Properties diffed: x, y, width, height, rotation (as transform), opacity, fill, stroke
- A data attribute on the SVG root lists frame names in order: `data-frames="startup,request,response"`

### FR7: Static frame rendering

- `--frame N` or `--frame "name"` CLI flag renders a single frame as a static SVG
- The output is a normal SVG with all elements at their frame-N positions and visibility
- No animation CSS is emitted
- This enables agent verification of individual frames

### FR8: Linter keyframe awareness

- Overlap detection runs per visible frame, not globally
- Elements hidden in a given frame are excluded from overlap checks for that frame
- Lint output identifies which frame(s) each warning applies to
- `--lint --frame N` checks only frame N

### FR9: Opt-in embedded playback

- `--animate` CLI flag embeds a minimal inline `<script>` in the SVG for self-contained playback
- The script cycles through frames on a timer (default ~2s per frame) and/or on click
- Without `--animate`, the SVG contains only frame classes and data attributes — no JS
- `--animate` and `--frame` are mutually exclusive (error if both specified)

## Key Entities

- **Keyframe** — A named temporal state with show/hide/transform operations
- **Named Connection** — A connection with an identifier, referenceable in keyframes and constraints
- **Frame State** — The cumulative visibility and transform state at a given keyframe
- **Frame Diff** — The CSS property changes between a frame's solved layout and the base (frame 0)

## Success Criteria

- An AI agent can produce a 5-frame animated diagram that is correct on the first attempt, verified by rendering each frame individually
- The linter produces zero false positive overlap warnings on diagrams with keyframes where hidden elements would otherwise overlap
- Existing AIL files without keyframes produce identical output (no behavioral change)
- The generated SVG plays as a smooth animation when a user adds CSS transitions targeting the generated frame classes
- Each frame can be rendered as a standalone static SVG for verification

## Assumptions

- Five to ten keyframes per diagram is the typical range; performance of N solver passes is acceptable for N < 50
- CSS transitions (user-authored) are sufficient for smooth inter-frame animation; the built-in `--animate` JS is a convenience, not a requirement
- Frame switching is external by default; `--animate` provides an opt-in embedded alternative
- The `as` keyword for naming connections does not conflict with existing syntax
- Per-keyframe constraint solving reuses the existing solver infrastructure with minimal modification (pinning unchanged elements)
