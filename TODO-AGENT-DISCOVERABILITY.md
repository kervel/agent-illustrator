# Agent Discoverability Improvements

## Test Results Summary

| Iteration | Pass Rate | Key Fix |
|-----------|-----------|---------|
| 1 | 92% (11/12) | N/A (baseline) |
| 2 | 100% (12/12) | Added reserved keywords warning |

Prioritized list based on evidence from 24 subagent tests across 2 iterations.

## P0: Critical (Blocking Issues)

### 1. ✅ Document Reserved Keywords in Skill (DONE)
**Evidence**: Test 06 failed because subagent used `col left { }` - "left" is a reserved keyword.
**Fix**: Added to `--skill` output: "Reserved words cannot be names: left, right, top, bottom, x, y, width, height"
**Result**: Iteration 2 achieved 100% pass rate (Test 06 used `left_col` instead of `left`)

### 2. Improve Parse Error Messages
**Evidence**: Error was "ExpectedFound { expected: [something else...], found: Some(Left) }" - not helpful.
**Fix**: In parser error handling, detect reserved keyword usage and emit: "Cannot use 'left' as identifier - it's a reserved keyword for constraints"
**Effort**: Medium (parser changes)

## P1: High Priority (Improves Success Rate)

### 3. Unify Path Naming Syntax
**Evidence**: `path "name" { }` requires quotes, `rect name` doesn't - inconsistent.
**Options**:
- A) Make path accept unquoted names like `path myshape { }`
- B) Document the difference clearly in skill
**Effort**: Medium (A) or Small (B)

### 4. Add Identifier Rules to Skill
**Evidence**: Test 11 subagent initially tried quoted identifiers in connections.
**Fix**: Add to skill:
```
## Naming Rules
- Names are unquoted identifiers: start with letter/underscore, contain alphanumeric/underscore
- Example: rect myShape [fill: blue] ✓
- Not: rect "my shape" [fill: blue] ✗
```
**Effort**: Small

### 5. Add "Output Format" Note to Skill
**Evidence**: Some subagents wrapped output in markdown code blocks.
**Fix**: Add at top of skill: "Output raw AIL code only. No markdown, no explanations."
**Effort**: Trivial

## P2: Medium Priority (Quality Improvements)

### 6. Document Connection Label Position
**Evidence**: Feature exists but not in skill.
**Fix**: Add to Common Modifiers table: `label_position | label_position: center | Label placement (left/center/right)`
**Effort**: Small

### 7. Add Constraint Examples to Skill
**Evidence**: Constraints only in `--grammar`, not in `--skill`.
**Fix**: Add a simple constraint pattern example.
**Effort**: Small

### 8. Add "Common Pitfalls" Section to Skill
**Fix**: Create section with:
- Reserved keywords
- Quoting rules
- Layout nesting patterns
**Effort**: Small

## P3: Low Priority (Nice to Have)

### 9. Interactive Error Recovery Suggestions
**Example**: "Did you mean 'leftcol' instead of 'left'? 'left' is reserved."
**Effort**: Large (requires semantic analysis)

### 10. Add More Test Prompts
- Test constraint system
- Test templates
- Test arc_to in paths
- Test edge cases (empty layouts, etc.)
**Effort**: Medium

---

## Quick Wins (Can Do Now)

1. ✅ Add reserved keywords note to `--skill` (DONE)
2. ✅ Add identifier naming rules to `--skill` (DONE)
3. Add label_position to modifiers table (2 min)
4. Strengthen "no markdown" instruction (haiku still wraps in ```)

## New Issue from Iteration 2

### Markdown Code Block Wrapping
**Evidence**: Tests 05, 07 (haiku model) wrapped output in ``` blocks despite "Output raw AIL code only (no markdown)" instruction
**Impact**: Low (easily stripped in post-processing)
**Fix**: Make instruction more emphatic or accept as model-dependent behavior

## Summary

| Priority | Count | Estimated Effort |
|----------|-------|------------------|
| P0 | 2 | Small + Medium |
| P1 | 3 | Small × 3 |
| P2 | 3 | Small × 3 |
| P3 | 2 | Medium + Large |

Completing P0 + P1 would likely achieve 98%+ subagent success rate.
