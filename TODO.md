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

### ~~Sandbox: the static binary is the whole loop~~ DONE (v0.2.11)
PNG output via resvg (`--frame N [--at T] --png`, `--frames-to-dir --png`,
`--frames-strip --png`, `--scale`); bundled sans + mono fonts and `@font-face`
data URIs from stylesheets, with a clear report on fallback; CI check that the
default build is a static musl binary; prebuilt static linux x86_64/aarch64
binaries on every release with a one-line install; a "verifying without a
browser" section in --skill and --skill-animation.

### Flicker check: a maintainer tool, outside the binary (lowest priority)
The flip flashes of v0.2.2/v0.2.3 only showed in a headed GPU Chrome: CDP
`Page.startScreencast` (everyNthFrame 1) plus a per-region check (frame i
differs from i-1 and i+1 while those two agree). Build it as
`tools/flicker-check/` (Node + Playwright or a separate crate), taking an SVG
or the `--serve` page; keep the v0.2.3 regression as a known-positive fixture.
Run it when the player or the track export changes (release checklist), not in
CI. No browser dependency may enter the main binary.

### The long tail (decided, not planned)
- 3D card flip (perspective rotateY): no, by design. The native renderer and
  the player must agree frame by frame; the 2D flip does that.
- Spring/physics easing: later, as named eases (a spring is a cubic
  approximation away from `pop`).
- Per-character text (typing): later, as an `enter: type` preset on text.
- Masks/reveal wipes on arbitrary shapes: later; `wipe(dir)` covers boxes.
- Particles: no, by design (not a diagramming need; no semantic meaning).


### Built-in icon set (`ail:icons`) — deferred (Frank, 2026-09-30)
Not a bad idea, but we don't yet know what a good set is. Collect the objects
real scenes keep needing (so far: person, laptop, server, cloud, document,
code file, database, container box) and decide once the pattern is clear.

### Tight group box as a constraint subject (deferred from v0.2.5)
`constrain [merged, card].center_x = s.stage.center_x`: centre two elements
together. `contains` only emits inequalities, so a helper box is loose (the
solver stretches the box instead of moving what is inside). Needs a solver
preference that keeps the box tight. Would replace the `+ 272` in
git-merge's `layout beside`.

### Player: translate of a collapsing line at the instant it vanishes
playback_equivalence saw git-merge "conflict" line8 at t=0.499 translate
-84 (native) vs -94 (played) while its opacity is ~0 and it is clipped
shut; the test now ignores opacity < 0.005. Not visible in settled stills;
not yet checked in a GPU `--film` of git-merge step 2.


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

### ~~Skill doc too long for constrained contexts~~ DONE (v0.2.18: `--skill-brief`, `--doc TOPIC`)
Agents with long prior context skip steps in the skill doc. Consider:
- Splitting into a short "checklist" section and a separate reference
- Moving examples/grammar to appendix sections the agent can fetch on demand
- Identifying which steps get skipped most and making them more prominent

### ~~Residual flip flash on a GPU desktop~~ DONE (v0.2.4)
The mirror image of the v0.2.3 fix: at the end of a flip the old page was
hidden in the same instant its turned-away hold reset to full width; a paint
between the two showed it for one frame. Nothing about an element now steps at
(or while) it is hidden; tests/playback_equivalence.rs asserts it.

## Low Priority / Won't Do

### ~~z-index control~~ DONE (v0.2.8: `z_order:` on any shape; through-lines under their stations)
Declaration order already determines draw order and is documented. Not worth adding
explicit z-index — it would complicate the mental model for no real benefit.

### Cubic Bezier curves (`cubic_to`)
Lower priority, significant parser+renderer effort. Quadratic (`curve_to`) covers
most use cases.
