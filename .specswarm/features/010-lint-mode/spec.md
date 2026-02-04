---
parent_branch: main
feature_number: 010
status: In Progress
created_at: 2026-02-04T00:00:00+01:00
---

# Feature: `--lint` Mode for Machine-Verifiable Diagram Validation

## Overview

Agent Illustrator diagrams are currently validated by spawning an LLM subagent to visually review rendered PNGs. This is slow (~40-60 seconds), expensive (~20k tokens), and produces 40-50% false positives on mechanical defects like overlapping elements or labels outside containers.

The `--lint` flag adds deterministic, instant validation that catches mechanical defects the constraint solver and renderer already have data for. This eliminates the need for LLM review of objective layout problems, leaving the adversarial review subagent responsible only for subjective quality (spacing, readability, aesthetics).

## Clarifications

### Session 2026-02-04
- Q: Should constraint violations become warnings (best-effort) with --lint? → A: No. Constraint violations remain hard errors. --lint only reports geometric defects on diagrams that already render successfully.
- Q: How should lint reference anonymous (unnamed) elements? → A: Use positional path in AST, e.g. `<child #3 of group_a>`.
- Q: Should lint ignore sub-pixel overlaps to avoid solver rounding noise? → A: No tolerance. Near-overlaps still look bad in the rendered output.

## User Scenarios

### Scenario 1: Agent checks diagram during iteration loop
An AI agent renders a diagram and runs `agent-illustrator --lint diagram.ail`. Lint reports two overlapping sibling elements on stderr. The agent fixes the layout and re-runs lint. No warnings — the agent proceeds to adversarial review for subjective quality only.

### Scenario 2: Human author validates before committing
A human writes an AIL file and runs `agent-illustrator --lint diagram.ail` before committing. Lint catches a label that overflows its container. The author adjusts sizes and re-runs. Clean exit — they commit.

### Scenario 3: CI pipeline gate
A CI pipeline runs `--lint` on all `.ail` files in the repo. If any file has lint warnings, the pipeline fails with a non-zero exit code and the warnings in stderr.

### Scenario 4: Lint combined with rendering
The author runs `agent-illustrator --lint diagram.ail > output.svg`. The SVG is still produced, but lint warnings appear on stderr. A non-zero exit code signals that the output has known defects.

## Functional Requirements

### FR1: CLI flag
- `--lint` flag accepted alongside existing flags (`-d`, `-t`, `-s`, etc.)
- `--lint` does NOT suppress SVG output — the diagram is still rendered normally to stdout
- Lint warnings go to stderr
- Exit code is non-zero if any lint warnings are emitted, zero if clean

### FR2: Bounding box overlap detection

The challenge: AIL diagrams commonly use "zone" or "background" rectangles as visual grouping — large, semi-transparent shapes that intentionally overlap everything placed on top of them. Elements related via `contains` constraints are also siblings that intentionally overlap. Naively checking all sibling pairs would produce massive false positives.

**Heuristic for identifying intentional overlap:**
- Skip any element pair where one element is a `contains` target (its bounds are constrained to wrap other elements)
- Skip any element pair where one element has `opacity < 1.0` (visual convention for background/zone elements)
- Skip any `text` element overlapping a non-text element (labels placed on top of shapes are intentional)

**What to check:**
- After filtering, check remaining sibling pairs (elements sharing the same parent container) for bounding box intersection
- Report each overlap with the IDs of both elements and the overlap dimensions
- Do not report overlaps between elements in different containers

### FR3: `contains` constraint satisfaction
- For every `contains` constraint in the source, verify the container's final bounds fully enclose each contained element's bounds (plus declared padding)
- Report any contained element that extends beyond the container
- Include the overflow direction and amount
- This replaces the generic "elements outside container" check — since `contains` is the explicit containment declaration in AIL, only check what was explicitly declared

### FR4: Label overlap detection
- Check label-vs-label overlaps: element labels against other element labels, connection labels against element labels
- Estimate label bounding box using the existing heuristic (text length * 7px wide, 14px tall)
- Do NOT check labels against their own element or against background/zone elements (opacity < 1.0)
- Report the overlapping label texts

### FR5: Connection-element intersection detection
- For each connection path segment (line between consecutive waypoints), check whether it passes through any non-endpoint element's bounding box
- Only check elements that are neither the source nor target of the connection
- Skip elements with `opacity < 1.0` (connections routinely cross zone backgrounds)
- Report the connection (source → target) and the element it crosses

### FR6: Warning output format
- Each warning is one line on stderr
- Format: `lint: <category>: <description>`
- Categories: `overlap`, `containment`, `label`, `connection`
- Elements are referenced by their ID if named, or by AST path if anonymous (e.g., `<child #3 of group_a>`)
- Example: `lint: overlap: elements "server" and "database" overlap by 12x8px`
- Example: `lint: containment: element "icon" extends 15px past right edge of container "header"`
- Example: `lint: connection: connection server→client crosses element "firewall"`
- Example: `lint: overlap: elements "box_a" and <child #2 of main> overlap by 5x20px`
- Summary line at end: `lint: N warning(s)` or `lint: clean`
- No overlap tolerance — all geometric violations are reported regardless of size

## Success Criteria

1. Lint completes in under 100ms for diagrams with up to 50 elements (deterministic, no LLM call)
2. Zero false positives on mechanical checks — every reported defect is a real geometric violation
3. Agents using `--lint` before adversarial review reduce subagent review iterations by at least 50%
4. All four check categories (overlap, containment, label, connection) are implemented and independently testable

## Key Entities

- **ElementLayout**: Positioned element with bounding box, label, children
- **ConnectionLayout**: Routed connection with waypoint path and optional label
- **BoundingBox**: x, y, width, height rectangle with intersection math
- **ConstraintSource**: Origin and description of a constraint for error reporting
- **LabelLayout**: Label text, position, and anchor alignment

## Assumptions

- **Opacity as background signal**: Elements with `opacity < 1.0` are treated as background/zone elements and excluded from overlap and intersection checks. This matches the established AIL convention where zone backgrounds use `opacity: 0.2` or similar. If a future diagram uses opacity for non-background purposes, the heuristic may suppress valid warnings — this is an acceptable tradeoff given the high false-positive rate of the alternative.
- **`contains` implies intentional overlap**: Elements involved in a `contains` constraint are expected to overlap their container. These pairs are excluded from overlap detection.
- **Label bounding box estimation** (7px/char width, 14px height) is sufficient for overlap detection. Exact text measurement would require font metrics not available in the renderer.
- **Axis-aligned bounding boxes**: Rotated elements use their axis-aligned bounding box, which may over-report overlaps for heavily rotated elements. This is acceptable — over-reporting is better than missing real overlaps.
- **Straight-line path segments**: Connection path segments are checked as straight lines between waypoints. Curved connections (Bezier) use their control polygon, not the actual curve. This may slightly under-report crossings for curves.
- **Lint runs post-layout**: The lint pass runs after constraint solving and connection routing but before SVG rendering, using the same `LayoutResult` data the renderer uses.
