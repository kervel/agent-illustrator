# Skill Iteration Analysis - Tests Round 4

## SVG Outputs Available

View the rendered diagrams in `tests4/svg/`:

| Test | Claude (Fixed) | Codex |
|------|----------------|-------|
| DevOps Baseline | `01-devops-baseline-claude-fixed.svg` | `01-devops-baseline-codex-fixed.svg` |
| DevOps Structured | `01-devops-structured-claude-fixed.svg` | `01-devops-structured-codex.svg` |
| ML Pipeline | `02-structured-claude-fixed.svg` | `02-structured-codex.svg` |
| Hub-Spoke | `03-structured-claude-fixed.svg` | `03-structured-codex.svg` |
| Git Branching | `04-structured-claude-fixed.svg` | `04-structured-codex-fixed.svg` |
| State Machine | `05-structured-claude-fixed.svg` | `05-structured-codex.svg` |

---

**Date**: 2026-01-26
**Focus**: Testing structured reasoning approach vs baseline

## Test Matrix

| Prompt | Skill | Claude (Haiku) | Codex (GPT-5.2) |
|--------|-------|----------------|-----------------|
| DevOps Cycle | Baseline | FAIL: `LAYOUT:` line | FAIL: `LAYOUT:` line |
| DevOps Cycle | Structured | FAIL: `gap:` statement | **OK** ✓ |
| ML Pipeline | Structured | FAIL: `group "Name"` | **OK** ✓ |
| Hub-Spoke | Structured | FAIL: `group "Name"` | **OK** ✓ |
| Git Branching | Structured | FAIL: `group "Name"` | FAIL: `text name [...]` |
| State Machine | Structured | FAIL: `group "Name"` | **OK** ✓ |

**Pass Rates:**
- Claude: 0/6 (0%)
- Codex: 4/6 (67%)

## Error Categories

### 1. `LAYOUT:` Output Literally (Both agents, Baseline skill)

**Evidence**: Both agents output `LAYOUT: Infinity/8 → ...` as actual code.

**Root cause**: Skill says "Write LAYOUT: [intent] → [pattern] then code" but agents interpret this as output format, not a comment/thinking step.

**Fix options**:
- A) Change to: "First write a comment: `// LAYOUT: ...`"
- B) Remove the instruction entirely and rely on structured reasoning comments
- C) Make it clearer this is internal thinking, not output

### 2. `group "Name"` Syntax Error (Claude only)

**Evidence**: Claude consistently writes `group "Name" { }` with quoted strings.

**Root cause**: Skill shows `group { }` pattern but Claude infers quoted names like `text "content"`. The syntax is actually `group name { }` like `rect name`.

**Fix options**:
- A) Add explicit example: `group my_group { ... }`
- B) Add rule: "Group names are identifiers, not strings"

### 3. `gap:` as Statement (Claude, one instance)

**Evidence**: `col { ... gap: 20 }` - gap as a statement inside layout.

**Root cause**: Confusion between modifier syntax `[gap: 20]` and statement syntax.

**Fix options**:
- A) Add explicit example showing modifiers on layouts: `row [gap: 20] { ... }`
- B) Add rule: "Modifiers go in brackets AFTER the keyword, BEFORE the braces"

### 4. `text name [label]` Syntax Error (Codex, one instance)

**Evidence**: `text spacer_hotfix [label: ""]` - using text with identifier name.

**Root cause**: `text "content"` requires a string, not identifier. Used incorrectly as spacer.

**Fix options**:
- A) Document that `text` requires a quoted string content
- B) Add a spacer pattern or invisible element for alignment

### 5. Missing Routing for Infinity Pattern (Claude, baseline)

**Evidence**: Claude's baseline DevOps output lacks `[routing: direct]` for cross-connections.

**Root cause**: Didn't understand that the infinity pattern requires diagonal lines via `routing: direct`.

**Fix**: Structured reasoning skill fixed this for Codex. Claude's syntax errors masked layout quality.

## Plan Quality Analysis

### DevOps Cycle - Comparing Planning Quality

| Aspect | Claude (Haiku) | Codex (GPT-5.2) |
|--------|----------------|-----------------|
| **Visual Intent** | "Infinity symbol (∞) with two interconnected loops" | "Infinity (two loops side-by-side)" |
| **Strategy** | "Two vertical 2x2 grids positioned side-by-side" | "Two 2x2 grids side-by-side; connect diagonally with direct routing" |
| **Structure** | `col { row { left_loop right_loop } }` | `row { col { row row } col { row row } }` |
| **Cross-connections noted?** | ✓ Yes (mentioned) | ✓ Yes (explicit `direct routing`) |
| **Actually implemented routing?** | ✗ No (missing `[routing: direct]`) | ✓ Yes |

**Verdict**: Codex's plan was more actionable - it mentioned `direct routing` in the plan AND implemented it. Claude understood the concept but didn't translate to code.

### Hub-Spoke - Comparing Planning Quality

| Aspect | Claude (Haiku) | Codex (GPT-5.2) |
|--------|----------------|-----------------|
| **Hub Shape** | circle (gold) | rect (no color specified) |
| **Layout Strategy** | "radial pattern around it" | "central hub and surrounding row/col" |
| **Structure** | Vague ("group with central hub") | `group { col { row row row } }` |
| **Bidirectional** | ✓ Noted in mapping | ✓ Noted |

**Verdict**: Claude had better visual thinking (circle for hub, gold color) but vaguer structure. Codex had clearer structure but less visual distinction.

### Pattern: Planning → Implementation Gap

| Issue | Claude | Codex |
|-------|--------|-------|
| Plan mentions feature but code omits it | Common | Rare |
| Structure in plan matches code | Often wrong | Usually matches |
| Syntax errors in implementation | Frequent | Rare |
| Visual creativity (colors, shapes) | Better | Basic |

## Key Findings

### Structured Reasoning Helps Significantly

Codex with structured reasoning produced:
- Correct 2-grid infinity layout (DevOps)
- Proper cross-connections with `[routing: direct]`
- Appropriate use of patterns from the table

The mandatory comment structure forced planning before coding.

### Claude Has Syntax Modeling Issues

Claude consistently:
- Quoted group names (incorrect)
- Sometimes confused modifier vs statement syntax
- May benefit from more explicit syntax examples

### Codex Has Better Syntax Adherence

Codex almost always used correct syntax when following the structured reasoning approach. Only one failure (text as spacer).

## Recommendations for Skill Improvement

### High Priority

1. **Remove or comment-ify LAYOUT instruction**
   ```diff
   - Write "LAYOUT: [intent] → [pattern]" then code.
   + // Internal planning: identify pattern from table, then implement
   ```

2. **Add explicit group syntax example**
   ```
   group pipeline {
     row { rect a  rect b }
   }
   ```

3. **Add modifier placement example**
   ```
   row [gap: 20] {   // ← modifier goes here, before brace
     rect a
     rect b
   }
   ```

### Medium Priority

4. **Clarify text shape syntax**
   ```
   text "Title"           // ← requires quoted string
   rect name [label: "x"] // ← identifier name, label is separate
   ```

5. **Add "what NOT to do" examples**
   ```
   WRONG: group "Name" { }    → RIGHT: group name { }
   WRONG: col { gap: 20 }     → RIGHT: col [gap: 20] { }
   WRONG: LAYOUT: Infinity    → Use // comment or omit
   ```

### Low Priority

6. **Consider adding spacer/invisible element** for complex alignment needs

## v5 Feature-Rich Skill Results

After creating skill-v5-feature-rich.md that explicitly encourages shape variety and curves:

### Codex DevOps (v5):
- ✓ Used **circles** for dev/ops hubs
- ✓ Used **curves** for feedback loops (`[routing: curved]`)
- ✓ Used **colors** consistently (steelblue, lightgreen, gold)
- ✓ Specified gap values

### Codex ML Pipeline (v5):
- ✓ Used **ellipse** for Raw Data (data storage)
- ✓ Used **circle** for Data Collection hub
- ✓ Used **curves** for feedback loops
- ✓ Distinct colors for data vs model stages

**Key insight**: Explicit guidelines about WHEN to use each shape type (circles for hubs, ellipses for storage, curves for feedback) dramatically improved feature usage.

## Renderer Issues Noted

### 1. Default Colors Missing
Default fill color is not applied when agents omit colors. Shapes render as black boxes with black text.

**Fix needed**: In `src/layout/types.rs`, change `from_modifiers` to merge with `with_defaults()` rather than starting from `Self::default()`.

### 2. Labels Overflow Shapes
Shapes don't auto-size to fit their labels. Default rect (80x30) is too small for labels like "Feature Engineering".

**Fix needed**: Use `estimate_label_width()` in `src/layout/engine.rs` to calculate minimum shape width based on label text, then use `max(specified_width, label_width + padding)`.

## Next Steps

1. Update skill with fixes above
2. Re-run tests with both agents
3. Consider testing with Claude Sonnet (may have better syntax adherence than Haiku)
4. If issues persist, consider parser improvements (better error messages for common mistakes)
5. **Fix renderer defaults** for fill/stroke when not specified
