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

### Residual flip flash on a GPU desktop (parked, 2026-09-30)
Frank still sees an occasional brief flash mid-way through the opening half of
`swap a -> b [via: flip]` (examples/motion/git-copies.ail, "code" step) in
desktop Chrome with a GPU (Intel Iris Xe, Mesa, Wayland), on v0.2.3.
Not reproduced so far:
- headless Chrome, every painted frame (CDP screencast, ~17ms apart): 0 flashes
- headed, GPU-accelerated Chrome (ANGLE / Mesa Iris Xe, same machine), CDP
  screencast: 0 in 98 frames
- 3x Playwright recordVideo at 25fps: 0
The detector (a slot's content box >10px larger than in both neighbouring
frames) catches the v0.2.2 flash but only sees box size: a brightness or
stale-layer flash would slip past it.
Hypotheses:
- compositor hand-off of the individual `scale` property on an SVG `<g>`
  mid-animation: try animating a CSS `transform` (or the SVG `transform`
  attribute) instead of `scale`
- a cancel/commit or re-settle of channels while other staggered animations
  still run
- the name crossfade (`d*.name` / `k*.name` opacity) overlapping the opening half
Next step: ask Frank for a phone video of the flash.

## Low Priority / Won't Do

### z-index control
Declaration order already determines draw order and is documented. Not worth adding
explicit z-index — it would complicate the mental model for no real benefit.

### Cubic Bezier curves (`cubic_to`)
Lower priority, significant parser+renderer effort. Quadratic (`curve_to`) covers
most use cases.
