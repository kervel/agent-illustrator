# V3 Skill Update — Agent Test Results

**Date**: 2026-01-29
**Skill version**: V3 (V2 + Example 6 constraint-based layout, restructured Layout Strategy, color usage rules)
**Test prompt**: Same K8s architecture

---

## Changes from V2 to V3

1. **Example 6 added**: Full constraint-based layout example with templates, background containers, and connections
2. **Layout Strategy restructured**: Constraint-based is now the DEFAULT recommendation, row/col is ALTERNATIVE
3. **Color usage rules**: Light colors for fill, dark colors for stroke, never use *-dark as fill
4. **Background container pattern**: Uses `group` wrapper with backgrounds declared first, all elements positioned via absolute constraints

---

## Test 5: Claude V3 (Sonnet)

### Layout Quality: 3/5 (up from 2/5 in V2)

**Improvements:**
- Services visible as distinct shapes (API Service, Worker Svc)
- Pods visible as labeled boxes ("API", "Work")
- Production namespace has visible blue background
- Monitoring namespace separate on right with orange background
- Color coding: green PostgreSQL, pink Redis, orange Prometheus/Grafana
- More compact, less wasted space

**Remaining issues:**
- Namespace background doesn't extend to cover data stores (PostgreSQL/Redis partially outside)
- Some label overlapping in dense areas
- PVC box squeezed next to PostgreSQL

### Process:
- Iterated: YES (created PNG file)
- Valid AIL: YES
- Templates: YES (pod, deployment, service, data_store, monitoring)
- Constraint-based layout: PARTIAL (mixed row/col + constraints)

---

## Test 6: Codex V3 (GPT-5.2)

### Layout Quality: 2.5/5 (slight regression from V2 due to messy connections)

**Improvements:**
- Used constraint-based positioning (followed Example 6!)
- Two clear namespace backgrounds (blue production, beige monitoring)
- Templates with lead extensions
- Used via-points for connection routing (advanced technique)

**Remaining issues:**
- Monitoring connections are huge orange loops (via-points created arcs too large)
- Pods render as tiny circles (3 small dots) — barely visible
- Excessive whitespace at bottom (where via-point loops extend to)
- Data stores and deployments overlap in the production zone

### Process:
- Fetched skill/grammar/examples: YES
- Iterated: YES (fixed parse errors, added via-points for routing)
- Valid AIL: YES (after iterations)
- Used via-points: YES (but routing is too aggressive)

---

## V1→V2→V3 Trend

| Metric | V1 Claude | V2 Claude | V3 Claude | V1 Codex | V2 Codex | V3 Codex |
|--------|-----------|-----------|-----------|----------|----------|----------|
| Layout quality (1-5) | 1 | 2 | **3** | 2 | 2.5 | 2.5 |
| Iterated? | NO | YES | YES | Partial | Partial | YES |
| Templates w/ leads | NO | NO | NO | YES | YES | YES |
| Constraint-based | NO | NO | Partial | NO | NO | **YES** |
| Color usage correct | NO | NO | **YES** | NO | Partial | YES |
| Invented bad syntax | NO | NO | NO | YES | NO | NO |

---

## Root Causes for V4

### Claude: still mixes row/col with constraints
Claude adopted more constraint usage but still nests elements in row/col containers. The skill doc says constraint-based is DEFAULT but Claude reaches for the familiar pattern first. The templates are good but the overall layout is still row/col dominant.

**V4 fix**: In the BEFORE YOU START section, add: "For diagrams with >8 elements, use constraint-based positioning as shown in Example 6. Do NOT nest elements in row/col for the main layout."

### Codex: via-point routing creates huge loops
Codex tried to use via-points for monitoring connections but the via-points were placed too far from the targets, creating massive arc loops. The skill docs don't explain how to position via-points effectively.

**V4 fix**: Add via-point guidance: "Place via-points close to the midpoint between source and target. Too far away creates huge loops."

### Both: pods are too small or inconsistently sized
Claude renders pods as small labeled boxes, Codex as tiny circles. Neither is ideal.

**V4 fix**: In Example 6 or template best practices, add sizing guidance: "Give all elements readable minimum sizes. A pod/component should be at least 60x35 pixels."

### Both: namespace background sizing is guesswork
Both agents struggle to size background rects to contain their content. Without auto-sizing, they need to guess pixel dimensions and often guess wrong.

**V4 fix**: Add heuristic: "To size a background container, count the elements inside. Each element is ~150px wide with ~30px gap. Height is ~200-300px for 2-3 rows. Start larger than you think, then shrink after visual check."

---

*Created: 2026-01-29*
*Feature: 009-mosfet-driver-example*
