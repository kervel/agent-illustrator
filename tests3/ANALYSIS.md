# Design-First Approach Analysis

**Date**: 2026-01-25
**Test Case**: Complex login flowchart with branching, retry loop, and two end states

## Test Variants

| # | Approach | Model | Dimensions | Structure Quality |
|---|----------|-------|------------|-------------------|
| 01 | Baseline (direct) | Claude Sonnet | 520×750 | Good branching, but Lock Account misplaced |
| 02 | Design-First | Claude Sonnet | 480×700 | Good branching, endpoints in correct lanes |
| 03 | Baseline (direct) | Codex (gpt-5.2) | 246×432 | **Poor** - all shapes in single column |
| 04 | Design-First | Codex (gpt-5.2) | 452×404 | **Excellent** - proper lanes, compact |

## Key Findings

### 1. Design-First Dramatically Improves Codex Output

**Without design guidance (03-baseline-codex):**
```
col {
    rect start
    rect enter_credentials
    rect validate
    rect load_profile      <- Wrong! Should be in success lane
    rect dashboard
    rect show_error        <- Wrong! Should be in failure lane
    rect increment_attempts
    rect check_attempts
    rect lock_account
}
```
All shapes in a single vertical column - no visual distinction between paths.

**With design guidance (04-design-first-codex):**
```
col {
    rect start
    rect enter_credentials
    rect validate
    row {
        col { load_profile, dashboard }        <- Success lane
        col { show_error, increment, check,    <- Failure lane
              row { retry, lock_account } }    <- Branch within failure
    }
}
```
Proper lane structure with visual separation of paths.

### 2. Claude Benefits Less (Already Good at Layout)

Claude Sonnet already produced reasonable branching structure without design guidance.
Design-First mainly helped with endpoint placement (Lock Account in correct lane).

### 3. The "LAYOUT:" Prefix Forces Structured Thinking

When prompted to write a layout plan first:
- Codex: "col for shared start/enter/validate, then row with two cols: success path and failure path"
- This planning step translated directly into better code structure

## Design-First Instruction (Recommended)

```
## IMPORTANT: Design Before Coding

For complex diagrams with multiple paths:

1. **Identify the visual lanes** - what are the distinct flows?
   - Example: "success path (left), failure path (right), retry loop"

2. **Plan the structure** - how should lanes be arranged?
   - Use `row { }` to place parallel paths side-by-side
   - Use `col { }` for sequential steps within a path
   - Nest layouts: `col { shared_top, row { lane1, lane2 } }`

3. **Write the code** - implement the planned structure

FIRST write "LAYOUT:" with a one-line structure description.
THEN write the AIL code.
```

## Impact Assessment

| Model | Without Design-First | With Design-First | Improvement |
|-------|---------------------|-------------------|-------------|
| Claude Sonnet | Good | Good+ | Minor (endpoint placement) |
| Codex | Poor | Excellent | **Major** (usable vs not) |

## Conclusion

The design-first approach is **critical for non-Claude models** and **helpful for Claude**.
Adding the layout planning step to the skill documentation would significantly improve
cross-model compatibility and output quality for complex diagrams.

## Recommended Skill Update

Add to the `--skill` output:

```
## Layout Planning (for complex diagrams)

For diagrams with branching or multiple paths:

FIRST describe the structure: "LAYOUT: [your plan]"
- Identify distinct paths/lanes
- Plan nesting: row for parallel, col for sequential

Example:
LAYOUT: shared top (start/validate), then row with success lane (left) and error lane (right)

THEN write the AIL code following your plan.
```

## Files Generated

- `tests3/out/01-baseline-direct.ail` - Claude without design guidance
- `tests3/out/02-design-first-sonnet.ail` - Claude with design guidance
- `tests3/out/03-baseline-codex.ail` - Codex without design guidance
- `tests3/out/04-design-first-codex.ail` - Codex with design guidance
- `tests3/svg/*.svg` - Rendered outputs
