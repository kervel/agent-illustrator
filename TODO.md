# Agent Illustrator — TODO

Collected from real agent usage feedback (2026-02-04 IoT edge architecture experiment).

## High Priority

### `--lint` mode for machine-verifiable diagram validation
Deterministic checks that eliminate the need for LLM review of mechanical defects.
Report defects to stderr as structured warnings, exit non-zero if any found.

Checks to implement:
- Constraint violations (unsatisfied after solving)
- Bounding box overlaps between sibling elements
- Connection lines intersecting non-endpoint elements
- Labels overlapping other labels or elements
- Elements outside their declared container

This would make the adversarial review subagent only responsible for subjective
layout quality (spacing, readability, aesthetic balance) — the mechanical stuff
becomes free, instant, and zero false positives.

Needs a SpecSwarm spec — touches layout, routing, and renderer.

### `stroke_dasharray` fixes
Already implemented for shapes (`stroke_dasharray: "4,2"`) but has issues:
- Keyword mapping broken: `"dashed"` outputs literally instead of `"8,4"`
- Not applied to connections (only shapes)
Fix both — should be quick.

### `label_position` / `label_offset` on connections
Connection labels always sit at the midpoint, causing collisions when paths cross.
- `label_position: 0.3` — fraction along the path (0.0 = start, 1.0 = end)
- `label_offset: 15` — perpendicular offset from the line
Would eliminate a common iteration sink.

## Medium Priority

### Orthogonal routing merge control
Fan-in/fan-out connections share a vertical/horizontal trunk line with no control
over where it sits. Options:
- `merge_x: 200` / `merge_y: 150` on connection groups
- Junction dots at merge points

### Crossing detection warnings
When `routing: direct` lines intersect non-endpoint elements, emit a stderr warning:
`warning: connection a→b crosses element c`. Helps agents catch issues without rendering.
Don't try to auto-adjust — just warn.

### `label_side` on connections
Place connection label above or below the line instead of centered on it.
Syntax: `a -> b [label: "data", label_side: above]`
Ties into the label_position work.

## Low Priority / Won't Do

### z-index control
Declaration order already determines draw order and is documented. Not worth adding
explicit z-index — it would complicate the mental model for no real benefit.

### Cubic Bezier curves (`cubic_to`)
Lower priority, significant parser+renderer effort. Quadratic (`curve_to`) covers
most use cases.
