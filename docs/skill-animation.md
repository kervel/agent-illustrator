# Animation Skill

Create keyframe animations in Agent Illustrator. Read `--skill` first for general AIL usage.

## When to Use

Use this sub-skill when creating **multi-frame animated diagrams** — sequences where
elements appear, disappear, or transform over time. Not needed for static diagrams.

---

## Part 1: Animation Harness (MANDATORY)

This workflow is **not optional**. Skipping steps produces bad animations.

### Phase 1: Storyboard

Before writing ANY code, write a storyboard as a block comment:

```
/* STORYBOARD
   Frame 1 "name": What is visible? What just happened?
   Frame 2 "name": What appears? What disappears? What moves?
   ...
   Frame N "name": Final state.

   STORY ARC: What narrative does this tell?
   PACING: Which frames need emphasis (longer dwell)?
*/
```

Each frame must tell a clear step in the story. If a frame doesn't advance
the narrative, cut it. If a transition is confusing, add an intermediate frame.

### Phase 2: Element Census

List ALL elements that will exist in the animation. For each element, specify:
- Is it always visible, or does it appear/disappear?
- Does it transform (move, rotate, change color) between frames?
- What connections does it participate in?

```
/* ELEMENT CENSUS
   PERSISTENT (always visible):
     cli, llm, tool — the actors
   TRANSIENT (appear/disappear):
     request_msg — visible in frames 2-3
     response_msg — visible in frames 4-5
   CONNECTIONS:
     cli.right -> llm.left as send_req — visible in frame 2
*/
```

### Phase 3: Build Incrementally

Do NOT write the full animation at once. Build frame by frame:

1. Write all PERSISTENT elements + constraints. Render. Verify layout.
2. Add TRANSIENT elements for frame 1. Position them. Render with `--frame 0` (0-indexed).
3. Add keyframe blocks one at a time. After each, render that frame and check.
4. Only after all frames render correctly, test `--animate` for the full sequence.

### Phase 4: Lint Pass (MANDATORY)

Run `--lint` and fix ALL warnings before proceeding. Overlap warnings between
elements that are never visible simultaneously are false positives (the linter
is not yet keyframe-aware), but all other warnings MUST be fixed:

```bash
agent-illustrator --lint file.ail 2>&1 | grep '^lint:'
```

**Fix these categories immediately:**
- `alignment:` — near-horizontal/vertical connections off by a few pixels.
  Fix by constraining positions to match (e.g., `constrain a.center_y = b.center_y`).
- `connection:` — arrows crossing unrelated elements. Re-route or reposition actors.
- `redundant-constant:` — repeated magic numbers. Use element references instead.
- `reducible-bend:` — unnecessary bends in connections. Align elements to simplify paths.

**Safe to ignore:** `overlap:` warnings between transient elements that occupy the
same position in different keyframes (they are never visible simultaneously).

### Phase 5: Frame-by-Frame Verification (MANDATORY)

After writing all keyframes, render EVERY frame as a static image and check each one.
Delegate this to a subagent to avoid bloating the main context with image data:

```bash
# Render each frame individually
for frame in "startup" "request" "tool_call" "execute" "respond"; do
  agent-illustrator file.ail --frame "$frame" > "frame_${frame}.svg"
  google-chrome --headless --screenshot="frame_${frame}.png" --window-size=2400,1800 \
    "file://$(pwd)/frame_${frame}.svg"
done
```

IMPORTANT: Use headless Chrome, NOT rsvg-convert. rsvg-convert does not support
CSS custom properties (var(--color)), so all themed colors render as black.

The subagent should check each frame PNG for:
- Correct elements visible (not too many, not too few)
- No orphaned connections (arrow visible but target hidden)
- No overlapping transient elements
- Labels readable

Return a text-only PASS/FAIL report per frame (no images in main context).

### Phase 6: Evaluator Round (MANDATORY — NOT OPTIONAL)

After frame-by-frame verification passes, spawn a review subagent. The subagent
reviews the **full animation** by viewing all frame PNGs in sequence.

Subagent prompt:

> Review this animation sequence. The frames are shown in order.
> The original intent was: [paste storyboard STORY ARC here]
>
> For each frame, score PASS/FAIL:
> 1. VISIBILITY — Correct elements shown/hidden for this story beat
> 2. LAYOUT — No overlaps, readable labels, good spacing
> 3. CONTINUITY — Transition from previous frame makes visual sense
> 4. NARRATIVE — Frame advances the story clearly
>
> Verdict: ALL frames must PASS all criteria. List specific fixes needed.

If the evaluator finds issues, fix them and re-run from Phase 4 (lint).
Maximum 3 evaluator rounds. If still failing, present issues to user.

---

## Part 2: Animation Patterns

### Story-Driven Animations

Good animations tell a story. Each frame is a "scene" with:
- **Entry**: New elements appear (connections + data envelopes)
- **Focus**: The active interaction is highlighted
- **Exit**: Previous scene elements fade or hide

Bad animations just toggle visibility randomly. Plan narrative flow.

### Message Envelopes

For protocol/interaction animations, use "message envelopes" — labeled rects
that represent data in transit:

```
rect msg [width: 180, height: 60, fill: accent-light, stroke: accent-1,
          stroke_width: 2, opacity: 0.3, label: "POST /api/chat"]
```

Position envelopes BETWEEN the actors they travel between:
```
constrain msg.center_x = midpoint(sender, receiver)
constrain msg.center_y = sender.center_y
```

### Named Connections for Keyframe Control

ALWAYS use named connections (`as name`) when the connection's visibility
changes across frames:

```
cli.right -> llm.left as send_request [stroke: accent-1, stroke_width: 3]

keyframe "request" {
    show send_request    // Can reference by name
}
```

### Visual Hierarchy Across Frames

Use transforms to draw attention to the active part of the story:

```
keyframe "tool_execution" {
    transform cli [opacity: 0.4]      // Dim inactive actors
    transform tool [opacity: 1.0]     // Full opacity for active actor
    show exec_arrow, exec_result
}
```

### Clipart and Rich Visuals

For animations that need to look like a real product or tell a compelling story,
don't settle for plain rectangles. Use `--skill-find-clipart` to find and embed
SVG clipart for actors (people, servers, terminals, etc.). File-based SVG templates
bring the animation to life.

### CSS Transitions

For smooth playback, add a CSS file with transitions:

```css
svg * {
    transition: opacity 0.5s ease-in-out;
}
```

Apply with: `agent-illustrator file.ail --animate --stylesheet-css transitions.css`

---

## Part 3: Gotchas

1. **Orphaned connections** — If you hide element A but keep `A -> B` visible,
   the arrow renders to nowhere. Always hide connections when hiding their endpoints.
2. **Cumulative keyframes** — Each frame builds on the previous. If you `hide X` in
   frame 2, X stays hidden in frames 3+ unless you `show X` again.
3. **Transform persistence** — Transforms in frame N carry forward. To reset opacity
   to 1.0 in a later frame, you must explicitly `transform elem [opacity: 1.0]`.
4. **Frame naming** — Use descriptive names ("user_sends_prompt", not "frame3").
   Names appear in the animation player UI.
5. **Element count** — Animations tend to need many elements (persistent + transient
   for each frame). Use constraint-based layout, not row/col.
6. **Frame indexing** — `--frame N` is 0-indexed. Frame 0 is the first keyframe,
   frame 1 is the second, etc. There is no implicit "base" frame before keyframe 0.
7. **ViewBox consistency** — Hidden elements are omitted from the SVG (not just
   invisible), so frames with fewer visible elements can produce a smaller viewBox,
   causing the diagram to "jump" between frames. Fix: add an invisible canvas rect
   that spans the full desired area and is NEVER hidden:
   ```
   rect canvas [width: 740, height: 420, fill: white, stroke: none, opacity: 0.01]
   constrain canvas.center_x = 370
   constrain canvas.center_y = 210
   ```
   Use `opacity: 0.01` (not 0) — zero-opacity elements may be optimized away.
   Keep the canvas outside all keyframe `hide` directives.
8. **Delegate image verification to subagents** — Viewing rendered PNGs in the main
   conversation bloats context rapidly. Spawn a subagent to render frames, inspect
   the images, and return a text-only PASS/FAIL report.
