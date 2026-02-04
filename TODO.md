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

### ~~`label_position` / `label_offset` on connections~~ DONE
Connection labels always sit at the midpoint, causing collisions when paths cross.
- `label_position: 0.3` — implemented as `label_at`
- `label_offset: 15` — perpendicular offset from the line

## Medium Priority

### Orthogonal routing merge control
Fan-in/fan-out connections share a vertical/horizontal trunk line with no control
over where it sits. Options:
- `merge_x: 200` / `merge_y: 150` on connection groups
- Junction dots at merge points

### ~~Crossing detection warnings~~ DONE
Covered by `--lint` mode.

### Lint: warn on steep diagonal direct routing
`routing: direct` looks fine when nearly axis-aligned but ugly at steep angles
(30-60°) when mixed with orthogonal/curved connections. Lint could warn when the
angle exceeds ~15° from horizontal or vertical.

### ~~`label_side` on connections~~ SUPERSEDED
Tangent-relative label offsets (v0.1.12) make `left`/`right` mean perpendicular-left/right
for any path geometry. No separate `label_side` needed.

### Skill doc too long for constrained contexts
Agents with long prior context skip steps in the skill doc. Consider:
- Splitting into a short "checklist" section and a separate reference
- Moving examples/grammar to appendix sections the agent can fetch on demand
- Identifying which steps get skipped most and making them more prominent

## Low Priority / Won't Do

### z-index control
Declaration order already determines draw order and is documented. Not worth adding
explicit z-index — it would complicate the mental model for no real benefit.

### Cubic Bezier curves (`cubic_to`)
Lower priority, significant parser+renderer effort. Quadratic (`curve_to`) covers
most use cases.
