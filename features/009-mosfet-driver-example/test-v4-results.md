# V4 Skill Update — Agent Test Results

**Date**: 2026-01-29
**Skill version**: V4 (V3 + mandatory constraint-based for >8 elements in BEFORE YOU START, via-point guidance, sizing heuristics)

---

## Changes from V3 to V4

1. **BEFORE YOU START hardened**: "For diagrams with >8 elements, you MUST use constraint-based positioning. Do NOT use nested row/col for the main layout."
2. **Sizing heuristic**: Rough pixel sizes for components, gaps, and backgrounds
3. **Via-point routing guidance**: Keep via-points close to midpoint, 30-60px offset

---

## Test 7: Claude V4 (Sonnet)

### Layout Quality: 4/5 (up from 3/5 in V3!)

**What works well:**
- TWO clear namespace backgrounds: blue "Production Namespace" and gray "Monitoring Namespace"
- Namespace backgrounds ACTUALLY CONTAIN their content (first time!)
- Services visible as ellipses (API Service, Worker Svc) with leads
- Individual pods labeled (api-1, api-2, api-3, worker-1, worker-2)
- Data stores as distinct shapes (ellipse PostgreSQL, circle Redis, rect PVC)
- Prometheus and Grafana in separate monitoring zone
- Connection labels: HTTP, metrics, query, cache, jobs
- Constraint-based positioning used throughout

**Remaining issues:**
- Label overlap in dense center area (API/Worker labels collide)
- Monitoring connections ("metrics" labels) crowd the bottom of production zone
- Pods are circles (unusual for K8s) — should be rectangles
- Some connections route through text

### Process: Iterated (PNG created), constraint-based, templates with leads

---

## Test 8: Codex V4 (GPT-5.2)

### Layout Quality: 3/5 (up from 2.5/5 in V3)

**What works well:**
- Clear namespace backgrounds
- Services and deployments visible with labels
- Monitoring cleanly separate
- Constraint-based layout
- Templates used for services, deployments, data stores

**Remaining issues:**
- Pods detached from their deployments (appear as tiny boxes far below diagram)
- Excessive whitespace below main content
- Some overlapping in the production zone center

### Process: Fetched all docs, iterated on parse errors, constraint-based

---

## Cumulative Progress V1→V4

| Metric | V1 Cl | V2 Cl | V3 Cl | V4 Cl | V1 Cx | V2 Cx | V3 Cx | V4 Cx |
|--------|-------|-------|-------|-------|-------|-------|-------|-------|
| Layout (1-5) | 1 | 2 | 3 | **4** | 2 | 2.5 | 2.5 | **3** |
| Iterated? | NO | YES | YES | YES | Part | Part | YES | YES |
| Constraint-based | NO | NO | Part | **YES** | NO | NO | YES | YES |
| Templates w/leads | NO | NO | NO | YES | YES | YES | YES | YES |
| NS backgrounds | Bad | Bad | Part | **GOOD** | Part | Part | Part | Better |
| Color correct | NO | NO | YES | YES | NO | Part | YES | YES |
| Invented syntax | NO | NO | NO | NO | YES | NO | NO | NO |

**Key wins from V4:**
- Claude finally uses full constraint-based layout → namespace backgrounds work
- Claude produces readable, structured diagrams with proper containment
- Both agents consistently iterate and verify

---

## V5 Focus

The main remaining issues:
1. **Label overlap in dense areas** — skill docs could add guidance on avoiding crowded label placement
2. **Codex pods detach from deployments** — the deployment template might define pods as separate elements outside the template body
3. **Minor**: pod shapes (circles vs rectangles), connection routing through text

For V5, focus on:
- Add "Label placement" guidance: avoid placing labels where connections route, use offset constraints
- Add note about template composition: internal elements stay with the template instance

---

*Created: 2026-01-29*
*Feature: 009-mosfet-driver-example*
