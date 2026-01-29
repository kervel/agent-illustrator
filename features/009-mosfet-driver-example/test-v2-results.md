# V2 Skill Update — Agent Test Results

**Date**: 2026-01-28
**Skill version**: V2 (V1 + layout strategy section, "What Does NOT Exist" list, self-assessment checklist, semantic color guidance, background container pattern)
**Test prompt**: Same K8s architecture as V1

---

## Changes from V1 to V2

Added to `docs/skill.md`:
1. **Layout Strategy** section — when to use row/col vs constraint-based positioning, with code examples
2. **Background containers** pattern — how to use `stack` with manually-sized rect for namespace boundaries
3. **Colors** section — prefer semantic palette names, list of available names
4. **Self-Assessment Checklist** — 6-point checklist agents must verify after each render
5. **"What Does NOT Exist"** — explicit list of non-features (contains, padding, label on text, border, align, margin, percentages)
6. **Pitfall #9** — "Don't overclaim quality without checking"

---

## Test 3: Claude V2 (Sonnet, `--dangerously-skip-permissions -p`)

### Process Observations

| Criterion | V1 Result | V2 Result | Improved? |
|-----------|-----------|-----------|-----------|
| Fetched `--skill` | UNCLEAR | YES (created files) | YES |
| Fetched `--grammar` | NO | UNCLEAR (but used correct syntax) | PARTIAL |
| Fetched `--examples` | NO | UNCLEAR | PARTIAL |
| Followed design phases | YES | YES | SAME |
| Used templates | YES (2) | YES (pod template) | SAME |
| Iterated (render → check) | **NO** | **YES** — created PNG file! | **YES** |
| Valid AIL on first attempt | YES | YES | SAME |
| Stopped appropriately | NO (overclaimed) | Better — rendered first | **YES** |

### Key Improvement: Claude V2 iterated!

The most significant change: Claude V2 created a `.png` file in the test directory, meaning it **actually rendered and checked** the output. This is the first time Claude followed the iteration workflow. The V2 self-assessment checklist and "don't overclaim" pitfall likely drove this.

### Output Quality

- **Compiles**: Yes
- **Layout**: Better than V1 but still messy:
  - Production namespace boundary (blue rect) visible at top
  - Monitoring namespace (orange rect) visible on right — properly separate
  - BUT: services show as labels ("API Server Deployment", "Background Worker Deployment") without visible boxes — they're just text inside the prod namespace
  - Pods appear below the namespace boundary, not inside it
  - "Monitoring Namespace" label appears far below the diagram (orphaned)
  - Large empty whitespace in the production namespace area
- **Color coding**: Better — uses distinct colors for different domains (blue ingress, green pods, purple data stores, orange monitoring)
- **Namespace separation**: Production and monitoring are visually distinct
- **Connections**: Orange curved monitoring connections are visible and labeled

### Remaining Issues

1. **Service/Deployment boxes invisible** — only labels appear, actual service rects may be hidden behind namespace background
2. **Pods outside namespace boundary** — the background rect doesn't extend to cover pods
3. **"Monitoring Namespace" orphaned** — appears as a lone label far below the main diagram
4. **No lead extensions on templates** — pod template still uses basic rect without leads
5. **Excessive whitespace** — large gaps in production namespace, very tall diagram

---

## Test 4: Codex V2 (GPT-5.2, `--full-auto`)

### Process Observations

| Criterion | V1 Result | V2 Result | Improved? |
|-----------|-----------|-----------|-----------|
| Fetched `--skill` | YES | YES | SAME |
| Fetched `--grammar` | YES | YES | SAME |
| Fetched `--examples` | YES | YES | SAME |
| Followed design phases | YES | YES | SAME |
| Used templates | YES (3) | YES (4: pod, service, deployment, database) | **YES** |
| Iterated (render → check) | PARTIALLY (compile-only) | PARTIALLY (compile-only, Chrome blocked) | SAME |
| Valid AIL on first attempt | NO (2 errors) | NO (anchor name errors, then fixed) | SAME |
| Stopped appropriately | YES | YES | SAME |

### Output Quality

- **Compiles**: Yes (after iteration)
- **Layout**: Better than V1:
  - Production namespace has visible beige background
  - Monitoring namespace visible on right with blue background
  - Components roughly grouped correctly
  - BUT: some pods appear as dark black rectangles (accent-dark fill too dark)
  - Overlapping elements (Worker Deployment stacks on API content)
  - Detached pod groups at bottom left
- **Templates**: 4 templates with lead extensions (learned from Example 5!)
- **Color coding**: Used semantic colors (accent-dark) — but accent-dark renders as near-black
- **No `contains` constraint** — correctly avoided this after reading "What Does NOT Exist"!

### Key Improvement: No invented syntax

V1 Codex invented `constrain X contains Y [padding: 20]`. V2 Codex correctly avoided this after reading the "What Does NOT Exist" section. This is a direct win from the V2 skill update.

### Key Improvement: Lead extensions from examples

V2 Codex used lead extensions on all templates — directly from Example 5 in the updated `--examples`. This shows the examples are being read and applied.

### Remaining Issues

1. **accent-dark renders as near-black** — pods are illegible black rectangles. Need to document that accent-dark is a dark color, not suitable for fill
2. **Layout still messy** — overlapping elements, detached groups
3. **Chrome sandbox blocked** — can't do visual verification in Codex environment
4. **Anchor naming iteration** — had to fix anchor names that were reserved words (used `up`, `down` initially)

---

## V1 → V2 Comparative Summary

| Criterion | Claude V1 | Claude V2 | Codex V1 | Codex V2 |
|-----------|-----------|-----------|----------|----------|
| Fetched grammar+examples | NO | PARTIAL | YES | YES |
| Used design phases | YES | YES | YES | YES |
| Templates with leads | NO | NO | YES (from Ex5) | YES (from Ex5) |
| Visual iteration | **NO** | **YES** | No (env) | No (env) |
| First-attempt valid AIL | YES | YES | NO | NO |
| Invented non-existent syntax | NO | NO | YES (contains) | **NO** |
| Self-assessment accuracy | Poor | Better | Good | Good |
| Layout quality (1-5) | 1 | 2 | 2 | 2.5 |

### What improved (V2 wins):
1. **Claude now iterates** — renders to PNG and checks (self-assessment checklist worked)
2. **Codex no longer invents syntax** — "What Does NOT Exist" section worked
3. **Both use more templates** with better anchor patterns
4. **Claude no longer overclaims** — acknowledges limitations

### What didn't improve:
1. **Layout quality still poor** — both agents produce messy layouts with overlapping elements
2. **Neither agent uses constraint-based positioning** for complex diagrams despite the new "Layout Strategy" section
3. **Grammar/examples fetch still inconsistent** for Claude
4. **Semantic color confusion** — accent-dark is too dark for fills

---

## Root Cause Analysis for Remaining Issues

### Why layout is still poor

Both agents default to nested `row`/`col` layouts. The "Layout Strategy" section explains when to use constraints instead, but agents still reach for the familiar pattern. The constraint-based approach requires more cognitive effort.

**Possible V3 fix**: Make the examples more prominent — add Example 6 showing a constraint-based complex layout (not nested row/col). Or move the constraint approach to the BEGINNING of the layout section, making it the primary recommendation for complex diagrams.

### Why Claude doesn't reliably fetch grammar/examples

The `-p` flag runs Claude in print mode where tool calling behavior differs from interactive mode. The CLAUDE.md file tells it to start with `--skill`, but the agent may process the CLAUDE.md instructions differently in print mode.

**Possible V3 fix**: Simplify the entry point — put the most critical instructions in the prompt itself, not just in CLAUDE.md. Or restructure `--skill` to inline the most critical grammar details.

### Why semantic colors cause problems

`accent-dark` renders as very dark (near-black) in the default stylesheet. Agents see "accent-dark" as a semantic name and use it for backgrounds, but it's designed for strokes/borders, not fills.

**Possible V3 fix**: Add color usage guidance: "Use `*-light` colors for backgrounds/fills. Use `*-dark` colors for strokes/borders. `foreground-1` is for primary lines."

---

## Recommendations for V3

1. **Add Example 6**: constraint-based complex layout (NOT row/col) showing 6+ elements positioned with `constrain`
2. **Color usage rules**: light colors for fill, dark colors for stroke
3. **Move constraint layout advice BEFORE row/col** in the Layout Strategy section
4. **Add reserved anchor name list**: document that `up`, `down`, `left`, `right` cannot be anchor names (or can they? need to check grammar)

---

## Files

- `/tmp/agent-test-v2-claude/k8s-architecture.ail` — Claude V2 output (176 lines)
- `/tmp/agent-test-v2-claude/final.png` — Claude V2 rendered PNG
- `/tmp/agent-test-v2-codex/k8s-architecture.ail` — Codex V2 output (174 lines)
- `/tmp/agent-test-v2-codex/output.png` — Codex V2 rendered PNG

---

*Created: 2026-01-28*
*Feature: 009-mosfet-driver-example*
