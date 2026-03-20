# Implementation Plan: Keyframe Animation System

## Constitution Check

**Note**: Constitution lists "animation or interactivity" as out of scope. This feature deliberately expands scope — animation support is the explicit goal of this feature. The constitution should be updated after implementation.

## Technical Context

- **Language**: Rust (edition 2021)
- **Parser**: logos lexer + chumsky parser combinators
- **Constraint solver**: kasuari (Cassowary)
- **Output**: SVG with CSS

## Architecture Overview

The feature adds a new pipeline stage between constraint solving and SVG rendering. Keyframes are parsed as new AST nodes, then processed after the global layout to produce per-frame layout states. The SVG renderer diffs these states against frame 0 to emit CSS.

```
Parse → Templates → Layout → Constraints → [KEYFRAME PROCESSING] → Route → Render
                                                    ↓
                                            For each frame:
                                              1. Apply transforms
                                              2. Re-solve constraints (pinned)
                                              3. Re-route connections
                                              4. Compute diff vs frame 0
```

## Implementation Phases

### Phase 1: Named Connections (FR1)
- Add `Token::As` to lexer
- Extend connection parser for `as name` syntax
- Add `name: Option<Identifier>` to `ConnectionDecl` in AST
- Propagate name through layout to `ConnectionLayout`
- Update connection routing to preserve names

### Phase 2: Keyframe Parsing (FR2, FR3)
- Add `Token::Keyframe`, `Token::Show`, `Token::Hide`, `Token::Transform` to lexer
- Add AST types: `KeyframeDecl`, `KeyframeOp` (Show/Hide/Transform)
- Add `Statement::Keyframe(KeyframeDecl)` variant
- Parse keyframe blocks in grammar.rs
- Validate element/connection references exist (hard error)

### Phase 3: Frame State Computation (FR4, FR5)
- Compute cumulative visibility per frame (replay show/hide ops)
- For frames with transforms: clone layout, apply overrides, re-solve constraints with pinning
- Re-route all connections per frame using solved positions
- Store per-frame `LayoutResult` snapshots

### Phase 4: Diff Engine & SVG Output (FR6)
- Diff each frame's layout against frame 0
- Emit elements at frame-0 positions in SVG body
- Generate `.frame-<name>` CSS classes with property diffs
- Add `data-frames` attribute to SVG root
- Handle hidden-in-frame-0 elements (inline `opacity: 0`)

### Phase 5: CLI Integration (FR7, FR9)
- Add `--frame` and `--animate` flags to clap CLI
- `--frame N`/`--frame "name"`: render single frame as static SVG
- `--animate`: embed inline JS for playback
- Mutual exclusion check

### Phase 6: Linter Awareness (FR8)
- Modify overlap detection to run per-frame
- Compute visible set per frame from cumulative state
- Tag lint warnings with frame name(s)
- Support `--lint --frame N`

### Phase 7: Documentation & Examples
- Update grammar.md and skill.md
- Update grammar.ebnf (constitution requirement)
- Create agentic-loop example using keyframes
- Update CLAUDE.md if needed
