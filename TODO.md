# Agent Illustrator — TODO

Collected from real agent usage feedback (2026-02-04 IoT edge architecture experiment).

## High Priority

### ~~`--lint` mode for machine-verifiable diagram validation~~ DONE
Implemented. Checks: sibling overlap, contains violation, label overlap, connection crossing.
Heuristics: skips opacity<1.0 zones, contains targets, text-on-shape.
Exit code 1 on warnings, structured stderr output.

### ~~`stroke_dasharray` fixes~~ DONE
Keyword mapping (`dashed`→`"8,4"`, `dotted`→`"2,2"`) was already working.
Added `stroke_dasharray` to connection rendering.

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

### ~~Crossing detection warnings~~ DONE
Covered by `--lint` mode.

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
