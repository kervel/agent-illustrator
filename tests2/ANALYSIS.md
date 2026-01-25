# Agent Discoverability Test Analysis - Iteration 2

**Date**: 2026-01-25
**Tests Run**: 12
**Pass Rate**: 12/12 (100%) - up from 11/12 (92%) in iteration 1

## Comparison with Iteration 1

| Metric | Iteration 1 | Iteration 2 | Change |
|--------|-------------|-------------|--------|
| Pass Rate | 92% (11/12) | 100% (12/12) | +8% |
| Reserved Keyword Issues | 1 (Test 06) | 0 | Fixed |
| Markdown Wrapping | 0* | 2 (Tests 05, 07) | New issue |

*Note: Markdown wrapping may have occurred in iteration 1 but was manually fixed before saving.

## Test Results Summary

| Test | Prompt | Model | Result | Notes |
|------|--------|-------|--------|-------|
| 01 | Simple shapes (2 rects) | haiku | PASS | Clean output |
| 02 | Basic connection | sonnet | PASS | Clean output |
| 03 | Vertical stack | haiku | PASS | Clean output |
| 04 | Styled shapes | sonnet | PASS | Clean output |
| 05 | Bidirectional arrow | haiku | PASS | Output wrapped in markdown* |
| 06 | Nested layout | sonnet | PASS | Used `left_col`/`right_col` (fixed!) |
| 07 | 3-tier architecture | haiku | PASS | Output wrapped in markdown* |
| 08 | Flowchart | sonnet | PASS | Clean output |
| 09 | Custom path shape | haiku | PASS | Triangle rendered correctly |
| 10 | Labeled group | sonnet | PASS | Clean output |
| 11 | Multiple connections | haiku | PASS | Clean identifiers |
| 12 | Microservices | sonnet | PASS | Complex layout worked |

*Markdown was stripped before saving to .ail files.

## Key Improvements from Iteration 1

### Fixed: Reserved Keywords as Identifiers
- **Before**: Test 06 used `col left { }` which failed to parse
- **After**: Test 06 used `col left_col { }` avoiding reserved keywords
- **Root cause**: Added "Reserved words cannot be names: left, right, top, bottom, x, y, width, height" to skill

### Fixed: Vertex Coordinates
- Test 09 used `vertex a [x: 0, y: 0]` which now parses correctly
- The renderer handles explicit origin coordinates

## Remaining Issues

### 1. Markdown Code Block Wrapping (Medium Priority)
**Evidence**: Tests 05 and 07 (both haiku model) wrapped output in ``` blocks
**Current skill says**: "Output raw AIL code only (no markdown)"
**Observation**: sonnet models followed this instruction better than haiku

**Potential fixes**:
- A) Make instruction more emphatic: "Output ONLY raw AIL code. NO markdown code blocks."
- B) Add explicit negative example showing what NOT to do
- C) Accept this as model-dependent behavior (haiku is less instruction-following)

### 2. Shapes Without Layout Container (Low Priority)
**Evidence**: Tests 05, 07 declared shapes without putting them in a row/col
**Result**: Shapes rendered but may overlap
**Example**:
```
rect browser [label: "Browser"]
rect api [label: "API Server"]
rect database [label: "Database"]
```

**Not a parsing failure**, but could be improved with guidance:
- "Always use row { } or col { } to arrange multiple shapes"

### 3. Path Vertex at Origin (Informational)
**Evidence**: Test 09 used `vertex a [x: 0, y: 0]`
**Result**: Worked! The parser accepts explicit origin coordinates
**Skill example uses**: `vertex a` (no coordinates)

Both are valid - no change needed.

## Updated Priority List

### P0: Critical (Already Fixed in This Iteration)
1. ✅ Document reserved keywords in skill
2. ⏳ Improve parse error messages (still TODO - not tested)

### P1: High Priority
3. **Strengthen "no markdown" instruction**
   - Evidence: 2/12 tests wrapped in markdown
   - Effort: Trivial (documentation only)

### P2: Medium Priority
4. **Add layout guidance**
   - "Use row { } or col { } to arrange shapes side-by-side or stacked"
   - Effort: Small (documentation only)

5. **Unify path naming syntax** (from iteration 1)
   - `path "name" { }` requires quotes, `rect name` doesn't
   - Effort: Medium (parser change) or Small (documentation)

### P3: Low Priority
6. **Add constraint examples to skill**
   - Currently only in --grammar
   - Effort: Small

7. **Add more path examples**
   - Show different shapes (arrow, star, etc.)
   - Effort: Small

## Model Observations

| Model | Tests | Pass Rate | Markdown Issues | Notes |
|-------|-------|-----------|-----------------|-------|
| haiku | 6 | 100% | 2 | Less instruction-following |
| sonnet | 6 | 100% | 0 | Better at following format rules |

**Recommendation**: For production use, prefer sonnet for diagram generation. Haiku is acceptable but may need post-processing to strip markdown.

## Files Generated

- `tests2/in/` - 12 prompt files (copied from iteration 1)
- `tests2/out/` - 12 AIL files (subagent output, markdown stripped)
- `tests2/svg/` - 12 rendered SVG files

## Conclusion

The skill improvements achieved **100% parse success rate** (up from 92%). The main remaining issue is:

1. **Markdown wrapping by haiku** (2/12 tests) - cosmetic, easily stripped

The skill documentation is now highly effective for enabling zero-shot diagram generation by AI agents.

## Next Steps

1. Consider strengthening the "no markdown" instruction
2. Implement P0 item #2: Better parse error messages (if not done)
3. Optionally add layout guidance to reduce shape overlap
