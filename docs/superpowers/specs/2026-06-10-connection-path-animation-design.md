# Connections Follow Moving Endpoints — Design

**Date:** 2026-06-10
**Status:** Approved (brainstorming) — pending implementation plan
**Builds on:** 2026-06-10-keyframe-geometry-animation-design.md (shipped)

## Problem

In a keyframe animation, when an element's geometry changes (via a `transform` or a
constraint change), elements constrained relative to it already follow and tween. But a
**connection** anchored to that element does not: e.g. `box.right -> other.left` with a
keyframe `transform box [width: 260]` keeps starting at the box's *old* right edge and
cuts through the widened box. The invariant "constraints satisfied at all times" is
violated for connections.

## Why it was deferred (not a deliberate choice)

- The original keyframe diff engine (`647b27d`) only did show/hide, so `ConnectionDiff`
  was born with a single field, `opacity`. Connections could appear/disappear, not move.
- Per-frame transforms + connection re-routing arrived later (`192f8b3`), wired for the
  **static** `--frame N` path, which re-renders a whole frame from scratch — there the
  re-routed connection paths *are* used and are correct.
- The **animated** path (`--animate`, one SVG flipping frames via a `frame-<name>` class)
  was never taught to emit connection geometry — only opacity. Element geometry got wired
  into the diff/CSS (then and in recent work); connection geometry was the hard 20% left
  behind, because animating an SVG path `d` in CSS has no portable cross-browser form.

So `resolve_frame_layout` already re-routes connections correctly each frame; the
animated diff engine throws the result away. This design stops throwing it away.

## Confirmed current behavior

- Element dependents follow: a chip `constrain chip.center_x = box.center_x + 120`
  emits `.kf-chip { transform: translate(80px, 0) }` when the box grows. ✓
- Connection does not: the only output for `box.right -> other.left` is the static
  base path `d="M200 100 L392.8 100"` (start at the old right edge); nothing per-frame
  touches it. ✗

## Design

### 1. `ConnectionDiff` carries geometry

```rust
pub struct ConnectionDiff {
    pub opacity: Option<f64>,
    pub path: Option<Vec<Point>>,   // re-routed path when it differs from frame 0
}
```

A connection can have both in one frame (it moved *and* its visibility changed).

### 2. `compute_frame_diffs` computes path diffs

When a frame is re-solved (`solved_result` present), match each solved connection to its
base connection **by routing-order index** — `route_connections` iterates the document
deterministically, so `base.connections[i]` and `solved.connections[i]` are the same
connection. For each whose solved `path` differs from the base `path` beyond an epsilon
(any point off by > 0.1px), record `path: Some(solved_path)`. Visibility (opacity) diffs
continue as today; the two merge into one `ConnectionDiff` per connection.

### 3. Connection identity for CSS targeting

Every connection that animates needs a stable class. Named connections already carry
`conn-<name>`. Unnamed connections get a synthetic `conn-<idx>` (routing-order index)
assigned at render time and used as the diff key — so **every** connection on a moving
endpoint follows, with no naming required by the author.

### 4. Rendering — per-frame `d` + transition

- The path's `d` **attribute** stays the frame-0 route (correct at frame 0, and correct
  everywhere when no frame class is active).
- Extract the `d`-string builder (marker pullback + routing-mode switch, currently inline
  in `add_connection_path`) into a shared `connection_path_d(path, routing_mode,
  marker_end, stroke_width) -> String`. `add_connection_path` calls it; so does the CSS
  emitter, so the per-frame target shape matches the rendered shape exactly.
- For each later frame where a connection's path changed, emit:
  `.frame-<name> .conn-<id> { d: path("<d-string>"); }`
- Add one default base rule in `generate_keyframe_css` (and the `--no-frame-css` branch):
  `.ai-connection { transition: d 0.5s ease, opacity 0.5s ease; }`
- The arrowhead marker rides the path end automatically (markers attach to the path),
  so it follows for free.

### 5. Determinism / anti-flicker (the other reason it was static)

Rendering each connection once was also, in effect, a **deflicker guard**: the
constraint solver has had nondeterminism (HashMap iteration order — see the project
notes on sorted iteration, target/reference strength, and FIXED width when `Right` is
targeted alone). Re-solving per frame and emitting raw connection paths could make a
connection that *shouldn't* move jitter between frames as the solver returns different
-but-valid solutions.

Mitigations (both already in place / cheap):

1. **Epsilon gate.** A connection path diff is emitted only when a point moves > 0.1px
   vs frame 0 — the same threshold `diff_element` uses for element geometry. Sub-pixel
   solver noise produces no diff, so a static connection stays on its frame-0 attribute
   path and cannot flicker. (Element diffs already rely on this; connections inherit it.)
2. **Solver determinism work is done.** The sorted-iteration / target-vs-reference /
   fixed-width fixes make a given active constraint set solve to the same answer every
   time, so a frame whose cumulative constraints are unchanged re-solves identically and
   yields no spurious diff.

If genuinely large nondeterministic deltas ever appear for a connection, the fix is to
constrain the offending element more tightly (the determinism playbook), not to re-freeze
all connections — freezing is what broke endpoint-following in the first place.

### 6. Behavior & caveats

- **Chrome/Safari:** the path morphs smoothly when segment counts match across frames
  (e.g. a straight or fixed-shape connector); snaps-but-correct when the route's *shape*
  changes (different segment count — CSS can't interpolate those).
- **Firefox:** lacks CSS `d` as an animatable property, so the path holds at its frame-0
  attribute route (degraded, accepted). All other browsers honoring CSS `d` are correct
  at each keyframe.
- Endpoints already re-anchor at routing time (`resolve_anchor` reads live bounds) — no
  routing change needed.

### 7. Transform keys stay (decided)

`x/y/dx/dy/width/height/scale` already feed the solver (apply to bounds → re-solve →
cascade, now including connections). They are concise sugar over a constraint pin and are
kept; no deprecation. (Resolves the perceived "transform/solve split" — there is none for
elements, and connections are fixed here.)

## Testing

New tests (`tests/connection_animation.rs` + unit where useful):

1. **Transform moves endpoint** — `box [width:100]`, `box.right -> other.left as feed`,
   keyframe `transform box [width: 260]`: the `grow` frame emits
   `.conn-feed { d: path("M360 ...") }` (start at the new right edge, ~360), and `#box`
   still gets `width: 260px`.
2. **Constraint change moves endpoint** — same connection, but
   `constrain box.width = 100 as w0` then keyframe `{ disable w0; constrain box.width = 260 }`:
   `.conn-feed` path diff emitted to the new edge. (The constraint-only path.)
3. **Unnamed connection follows** — an unnamed connection on a moving endpoint gets a
   `conn-<idx>` class and a per-frame `d`.
4. **Transition rule present** — `.ai-connection { transition: d 0.5s ease, ... }`.
5. **Regression** — element dependents still follow (chip relative to box), and
   show/hide of connections still works.
6. **No flicker** — a connection between two *static* elements, in a keyframe that moves
   an unrelated element, emits **no** `d` diff for that connection (epsilon gate holds it
   on its frame-0 path). Guards against solver-noise jitter.

## Docs

- `--skill-animation` / `--grammar`: a short note that connections automatically follow
  moving/resized endpoints and tween (Chrome/Safari smooth, Firefox snaps to the
  end-state route); name a connection only if you also show/hide it.

## Non-goals

- Smooth path morphing across route-shape changes (different segment counts) — snaps.
- Firefox smooth path tween (no CSS `d`); correctness there is best-effort frame-0 hold.
- Animating connection `stroke`/`marker` geometry beyond what `d` + existing styles give.
