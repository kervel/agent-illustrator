# V5 Skill Update — Final Test Results

**Date**: 2026-01-29
**Skill version**: V5 (V4 + label placement guidance, template composition, enhanced self-assessment checklist)

---

## Changes from V4 to V5

1. **Label placement guidance**: Offset labels above/below elements, keep connection labels short
2. **Template composition example**: Shows deployment template containing pods with internal constraints
3. **Enhanced self-assessment**: Added "Labels don't overlap" and "Elements reasonably sized (min 60x35px)" checks

---

## Test 9: Claude V5 (Sonnet) — Layout Quality: 4/5

**Good:**
- Two clear namespace zones (blue production, orange monitoring)
- All components visible: nginx Ingress, API Server, Background Worker, PostgreSQL, Redis Cache, PVC, Prometheus, Grafana
- Pods shown as blue circles inside deployment containers
- Labeled connections: HTTP, jobs, cache, writes, read/write, scrape, metrics, query
- Namespace backgrounds properly contain most content
- Compact layout, readable labels

**Remaining:**
- PVC slightly below production background
- Some "scrape"/"metrics" labels crowd the right edge
- Pods as circles rather than rectangles

## Test 10: Codex V5 (GPT-5.2) — Layout Quality: 3.5/5

**Good:**
- Two namespace zones with visible backgrounds
- All major components present and labeled
- Ingress → Services → Deployments → Data stores flow clear
- Pods visible with labels near their deployments
- Templates used (pod, deployment_3, deployment_2, service_box, data_store, postgres_with_pvc)

**Remaining:**
- One pod detached far below the main diagram
- Some vertical whitespace gaps
- Deployment-pod connection not perfectly contained

---

## CUMULATIVE PROGRESS: V1 → V5

### Claude (Sonnet) — Layout Quality Trend

| Version | Score | Key Change |
|---------|-------|-----------|
| V1 | 1/5 | Overlapping mess, no iteration |
| V2 | 2/5 | First iteration (self-assessment checklist) |
| V3 | 3/5 | Better layout, color coding works |
| V4 | 4/5 | Constraint-based layout, namespace containment |
| V5 | 4/5 | Consistent, labels improved slightly |

### Codex (GPT-5.2) — Layout Quality Trend

| Version | Score | Key Change |
|---------|-------|-----------|
| V1 | 2/5 | Fetched docs, but invented syntax |
| V2 | 2.5/5 | No invented syntax ("What Does NOT Exist" worked) |
| V3 | 2.5/5 | Constraint-based (Example 6), but messy via-points |
| V4 | 3/5 | Better sizing, pods still detach |
| V5 | 3.5/5 | Templates with composition, most pods contained |

### Process Behavior Trend

| Behavior | V1 Cl | V5 Cl | V1 Cx | V5 Cx |
|----------|-------|-------|-------|-------|
| Fetches --skill | ? | YES | YES | YES |
| Fetches --grammar | NO | YES | YES | YES |
| Fetches --examples | NO | YES | YES | YES |
| Follows design phases | YES | YES | YES | YES |
| Uses templates | Basic | w/leads | w/leads | w/composition |
| Constraint-based | NO | **YES** | NO | **YES** |
| Iterates (render→check) | NO | **YES** | Partial | YES |
| Stops appropriately | NO | **YES** | YES | YES |
| Invented bad syntax | NO | NO | YES(×1) | NO |

---

## What the Skill Updates Fixed (V1→V5)

### ✅ Fully Fixed
1. **Agents now fetch grammar+examples before starting** — BEFORE YOU START section
2. **Agents iterate and render** — Iteration Workflow + Self-Assessment Checklist
3. **No invented syntax** — "What Does NOT Exist" section
4. **Constraint-based layout for complex diagrams** — Example 6 + mandatory guidance
5. **Correct color usage** — Color rules (light for fill, dark for stroke)
6. **Accurate self-assessment** — Agents no longer overclaim quality

### ⚠️ Partially Fixed
7. **Namespace containment** — Works for Claude, partially for Codex (some elements escape)
8. **Template quality** — Both use templates with anchors; Codex also uses composition
9. **Label overlap** — Reduced but not eliminated in dense areas

### ❌ Not Fixed (inherent limitations)
10. **Connection routing in dense areas** — Orthogonal router creates overlapping paths when many connections converge
11. **Pod representation** — Both agents struggle to embed pods visually inside deployments
12. **Chrome headless in sandboxed environments** — Codex can never do visual verification

---

## Skill Documentation Changes Summary (V1→V5)

### docs/skill.md additions:
1. BEFORE YOU START (mandatory grammar+examples fetch + constraint mandate)
2. Required Iteration Workflow (render→check→fix cycle, phased approach)
3. Layout Strategy (constraint-based DEFAULT, row/col ALTERNATIVE, background containers)
4. Sizing Heuristics (pixel sizes for components, gaps, backgrounds)
5. Via-point Routing (placement guidance)
6. Colors (light for fill, dark for stroke, never dark as fill)
7. Template Best Practices (lead extensions, label placement, template composition, testing in isolation)
8. Self-Assessment Checklist (8 items including labels and element sizing)
9. What Does NOT Exist (contains, padding, label on text, border, align, margin, percentages)
10. Common Pitfalls expanded to 9 items

### docs/examples.md additions:
- Example 5: Complex templates with internal constraints, leads, anchors (from mosfet-driver)
- Example 6: Constraint-based layout for multi-domain diagrams (templates, backgrounds, connections)

---

## Files

Test outputs:
- `/tmp/agent-test-v{1-5}-claude/` — Claude test directories
- `/tmp/agent-test-v{1-5}-codex/` — Codex test directories
- Each contains: `k8s-architecture.ail`, `final.png`, `final.svg`

Skill docs:
- `docs/skill.md` — Updated (V5)
- `docs/examples.md` — Updated (V5)

Test analysis:
- `features/009-mosfet-driver-example/test-v1-results.md`
- `features/009-mosfet-driver-example/test-v2-results.md`
- `features/009-mosfet-driver-example/test-v3-results.md`
- `features/009-mosfet-driver-example/test-v4-results.md`
- `features/009-mosfet-driver-example/test-v5-results.md` (this file)

---

*Created: 2026-01-29*
*Feature: 009-mosfet-driver-example*
