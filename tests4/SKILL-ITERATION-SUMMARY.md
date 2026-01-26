# Skill Iteration Summary

## Skill Variants Tested

| Skill | Focus | Key Finding |
|-------|-------|-------------|
| baseline | Minimal reference | LAYOUT: instruction output literally |
| structured-reasoning | Mandatory block comments | Improved planning but Claude syntax errors |
| chain-of-thought | Step-by-step line comments | Similar to structured, less verbose |
| v2-explicit-syntax | Common mistakes to avoid | Helps with syntax but not layout |
| v3-minimal-rules | Just the essentials | Concise but agents need more guidance |
| v4-deep-reasoning | 5-step detailed process | Thorough but verbose |
| v5-feature-rich | Encourages shapes/curves | Agents used features BUT added noise |
| v6-semantic-choices | Match shapes to meaning | Good choices but lost layout patterns |
| **v7-combined** | Metaphor + Semantic + Patterns | Best balance |

## Key Insights

### 1. Layout Pattern Recognition is Critical
Agents need to be told explicitly that "infinity" = "two 2x2 grids side-by-side". Without this mapping, they invent their own layouts.

### 2. Semantic Shape Guidance Works
When told "ellipse = data store, circle = state", agents follow this. But they may over-apply (adding decorative circles).

### 3. "Don't add decorative elements" is Important
v5 caused agents to add hubs/circles that weren't needed. v6/v7 explicitly discourage this.

### 4. Curves Need Explicit Trigger Words
Agents only use `[routing: curved]` when:
- Told to use it for "feedback loops"
- Given examples showing it

### 5. Claude vs Codex Differences
| Aspect | Claude (Haiku) | Codex (GPT-5.2) |
|--------|----------------|-----------------|
| Syntax accuracy | Frequent errors (`group "x"`, `gap: 20`) | Usually correct |
| Layout quality | Good ideas, poor execution | Follows templates well |
| Feature usage | Conservative | Uses what's demonstrated |

## Recommended Skill Structure (v7 approach)

```
1. Visual metaphor identification (table of keywords → patterns)
2. Semantic shape selection (table of element types → shapes)
3. Connection routing guidance (table of situations → routing)
4. Concrete layout templates with code
5. Syntax rules and forbidden identifiers
```

## Renderer Improvements (DONE)

1. ✅ **Default colors** - `#f0f0f0` fill, `#333333` stroke when not specified
2. ✅ **Label auto-sizing** - Shapes grow to fit labels (~8px/char + 20px padding)
3. ⏳ **Better error messages** - TODO: Help agents understand syntax mistakes

## Parser Limitation Discovered

**Chained arrows not supported**: `a -> b -> c` fails with parse error.
Must write as separate connections: `a -> b` and `b -> c`

The v7 skill inadvertently shows chained syntax in examples - needs update.

## SVGs for Visual Review

Best outputs in `tests4/svg/`:
- `01-devops-v7-codex.svg` - Clean infinity with curves
- `01-devops-manual-test.svg` - Reference implementation
- `02-ml-pipeline-v5-codex.svg` - Shows ellipse, circle, curves (but label overflow)
