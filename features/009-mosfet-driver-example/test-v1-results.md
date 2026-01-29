# V1 Skill Update — Agent Test Results

**Date**: 2026-01-28
**Skill version**: V1 (added BEFORE YOU START, iteration workflow, template best practices, common pitfalls, Example 5)
**Test prompt**: Kubernetes deployment architecture (ingress, 2 services with pods, PostgreSQL+PVC, Redis, Prometheus+Grafana, namespace boundaries)

---

## Test 1: Claude (Sonnet, `--dangerously-skip-permissions -p`)

### Process Observations

| Criterion | Result | Notes |
|-----------|--------|-------|
| Fetched `--skill` | **UNCLEAR** | The transcript output was minimal — Claude may have used CLAUDE.md instructions directly rather than fetching --skill first |
| Fetched `--grammar` | **NO** | No evidence in output of grammar fetch |
| Fetched `--examples` | **NO** | No evidence in output of examples fetch |
| Followed design phases | **YES** | All 6 phases present as block comments |
| Used templates | **YES** | `pod` and `service` templates with anchors |
| Iterated (render → check → fix) | **NO** | Produced AIL and declared done without rendering/checking |
| Valid AIL on first attempt | **YES** | Compiles without errors |
| Stopped appropriately | **NO** | Stopped after first attempt without visual verification |

### Output Quality

- **Compiles**: Yes
- **Layout**: Poor — components overlapping, namespace boundaries don't contain their content, Grafana far away from monitoring boundary, massive vertical space between monitoring namespace and Grafana
- **Templates**: Only 2 templates (pod, service) — not deployment. Pods have anchors but no lead extensions
- **Color coding**: Good — different colors for namespaces, data stores, cache
- **Connections**: Present but routing is messy due to poor layout
- **Labels**: Present on most connections

### Key Failures

1. **Did not fetch `--grammar` or `--examples`** — the BEFORE YOU START section was ignored or not seen
2. **Did not iterate** — wrote AIL and declared done, never rendered to PNG
3. **Used hardcoded hex colors** instead of semantic palette names (foreground-1, accent-1, etc.)
4. **Namespace boundaries as explicit large rects** — doesn't scale, doesn't auto-contain content
5. **Claims "clean, professional" result** without having checked — overconfident self-assessment

### AIL Patterns Used

- `col main_layout [gap: 80]` as root container
- `group` for namespace boundaries (with explicit `rect` for visual boundary)
- Templates with `anchor` definitions
- `constrain` for namespace label positioning
- `routing: curved` for monitoring connections

---

## Test 2: Codex (GPT-5.2, `--full-auto`)

### Process Observations

| Criterion | Result | Notes |
|-----------|--------|-------|
| Fetched `--skill` | **YES** | First action after reading CLAUDE.md |
| Fetched `--grammar` | **YES** | Immediately after --skill, following BEFORE YOU START |
| Fetched `--examples` | **YES** | Immediately after --grammar |
| Followed design phases | **YES** | All phases present as block comments |
| Used templates | **YES** | `pod`, `service`, `deployment` templates — all with lead extensions and anchors |
| Iterated (render → check → fix) | **PARTIALLY** | Compiled AIL, hit parse errors, fixed them iteratively. Could not render PNG (Chrome sandbox issue in codex env) |
| Valid AIL on first attempt | **NO** | Two errors: (1) `contains` constraint (invented syntax), (2) `text` with `label:` modifier instead of content-before-name |
| Stopped appropriately | **YES** | Acknowledged inability to do visual check and offered to iterate further |

### Output Quality

- **Compiles**: Yes (after 2 fix iterations)
- **Layout**: Better structure than Claude — uses `stack` for namespace background + content overlay, `row`/`col` for grouping
- **Templates**: 3 templates (pod, service, deployment) — all use lead extensions and anchors per Example 5
- **Color coding**: Good — soft blues for production, amber for monitoring, distinct colors for data stores
- **Connections**: Clean routing with labeled connections
- **Labels**: Present, clear
- **Namespace boundaries**: Uses `stack` with background rect + content overlay — smarter approach

### Key Failures

1. **Invented `contains` constraint syntax** — `constrain prod_bg contains prod_content [padding: 20]` doesn't exist. The grammar doesn't support this. (This line is ignored at compile time, so the namespace backgrounds don't properly wrap content.)
2. **`text` syntax confusion** — initially used `text mon_note [label: "Monitoring stack"]` instead of `text "Monitoring stack" mon_note`. Fixed on iteration.
3. **Could not do visual verification** — Chrome headless failed in Codex sandbox environment. This is an environment limitation, not a skill documentation issue.
4. **Used hex colors** instead of semantic palette — same as Claude
5. **Layout still messy** — components overlapping, elements scattered, monitoring namespace not properly positioned

### AIL Patterns Used

- `stack` for background + content overlay (namespace boundaries) — creative pattern!
- `col`/`row` for hierarchical layout
- Templates with lead extensions (directly from Example 5)
- Parameterized templates (`pod_label`, `svc_label`, `deploy_label`)
- Template composition (deployment contains pods)
- `routing: curved` for monitoring, `routing: direct` for Grafana→Prometheus

---

## Comparative Summary

| Criterion | Claude (Sonnet) | Codex (GPT-5.2) |
|-----------|-----------------|------------------|
| Fetched grammar+examples | NO | YES |
| Followed BEFORE YOU START | NO | YES |
| Used design phases | YES | YES |
| Template quality | Basic (no leads) | Good (leads + composition) |
| Iterated on errors | NO | YES (2 fix cycles) |
| Visual verification | NO | Attempted (env blocked) |
| First-attempt valid AIL | YES | NO (2 errors) |
| Self-assessment accuracy | Poor (overclaimed) | Good (acknowledged limitations) |
| Final layout quality | Poor (overlapping) | Medium (structured but still messy) |

---

## Root Cause Analysis

### Why Claude didn't fetch grammar/examples

The `CLAUDE.md` in the test directory told Claude to "Start by running `agent-illustrator --skill`". Claude was run with `-p` flag (print mode), which may process the CLAUDE.md but doesn't necessarily trigger tool calls as reliably as interactive mode. Additionally, Claude (Sonnet) may have relied on the inline prompt + CLAUDE.md rather than executing commands first.

**Possible fix**: The CLAUDE.md should not be necessary — the `--skill` output itself should drive the workflow. The issue is that the agent needs to know to run `--skill` at all. For agents with a CLAUDE.md, the instructions work. For agents without, they'd need a different entry point.

### Why both agents produced poor layouts

Both agents used hex colors and nested `row`/`col` layouts. The fundamental issue is that complex diagrams with cross-cutting connections (monitoring → everything) require constraint-based positioning, not pure layout nesting. The agents correctly used `row`/`col` for grouping but the auto-layout engine doesn't handle this complexity well — or the agents didn't add enough constraints to fix overlaps.

### Why Codex invented syntax (`contains`)

Codex read the grammar but still invented a `contains` constraint that doesn't exist. This suggests:
1. The grammar is clear enough about what exists, but doesn't explicitly say "don't use what's not listed"
2. The concept of "container sizing" is a real need that AIL doesn't currently support — agents reasonably expect it to exist

---

## Recommendations for V2 Skill Update

### High Priority

1. **Add explicit "DO NOT" list to grammar/skill**: "There is no `contains` constraint. There is no auto-sizing of containers to fit children. If you need a background behind content, use `stack` with a fixed-size `rect`."

2. **Emphasize constraint-based layout for complex diagrams**: Add a section explaining that nested `row`/`col` works for simple diagrams but complex diagrams with cross-group connections need explicit `constrain` positioning (as shown in the mosfet-driver example).

3. **Add a "complex diagram" example** showing constraint-based free-form positioning (not just row/col nesting).

4. **Improve self-assessment guidance**: Add "After rendering, check: (1) Are any elements overlapping? (2) Do namespace boundaries contain their content? (3) Are connection labels readable? (4) Is there excessive whitespace?"

### Medium Priority

5. **Add semantic color guidance**: "Prefer semantic colors (foreground-1, accent-1, etc.) over hex codes. Run with `--stylesheet` for themed output."

6. **Chrome headless workaround for sandboxed environments**: Document `--no-sandbox` flag or alternative rendering approaches.

### Low Priority

7. **Template composition example**: Show a template that instantiates another template (deployment containing pods) — Codex discovered this pattern independently.

---

## Files

- `/tmp/agent-test-v1-claude/k8s-architecture.ail` — Claude's output (208 lines)
- `/tmp/agent-test-v1-claude/output.png` — Claude's rendered PNG
- `/tmp/agent-test-v1-codex/k8s-architecture.ail` — Codex's output (174 lines)
- `/tmp/agent-test-v1-codex/output.png` — Codex's rendered PNG
- `/tmp/agent-test-v1-claude/CLAUDE.md` — Test instructions given to both agents

---

*Created: 2026-01-28*
*Feature: 009-mosfet-driver-example*
