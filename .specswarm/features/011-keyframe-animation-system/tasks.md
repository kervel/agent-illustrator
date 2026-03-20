# Tasks: Keyframe Animation System

## Phase 1: Named Connections

- [ ] 1.1 Add `Token::As` to lexer.rs
- [ ] 1.2 Add `name: Option<Identifier>` to `ConnectionDecl` in ast.rs
- [ ] 1.3 Extend connection parser in grammar.rs for `a -> b as name` syntax
- [ ] 1.4 Propagate connection name through layout engine to `ConnectionLayout`
- [ ] 1.5 Add tests for named connection parsing
- [ ] 1.6 Update grammar.ebnf

## Phase 2: Keyframe Parsing

- [ ] 2.1 Add tokens: `Keyframe`, `Show`, `Hide`, `Transform` to lexer.rs
- [ ] 2.2 Add AST types: `KeyframeDecl`, `KeyframeOp` enum (Show/Hide/Transform) to ast.rs
- [ ] 2.3 Add `Statement::Keyframe(KeyframeDecl)` variant
- [ ] 2.4 Implement keyframe block parser in grammar.rs
- [ ] 2.5 Validate references: hard error on nonexistent element/connection names
- [ ] 2.6 Add parser tests for keyframe syntax
- [ ] 2.7 Update grammar.ebnf

## Phase 3: Frame State Computation

- [ ] 3.1 Add `FrameState` struct (visibility map + transform overrides per element)
- [ ] 3.2 Implement cumulative state replay (fold keyframes into sequence of FrameStates)
- [ ] 3.3 Implement per-frame constraint solving with pinning (clone layout, apply transforms, re-solve)
- [ ] 3.4 Implement per-frame connection rerouting
- [ ] 3.5 Store per-frame LayoutResult snapshots
- [ ] 3.6 Add tests for cumulative state and per-frame solving

## Phase 4: Diff Engine & SVG Output

- [ ] 4.1 Implement diff engine: compare per-frame LayoutResult against frame 0
- [ ] 4.2 Generate `.frame-<name>` CSS classes with property diffs (x, y, width, height, rotation, opacity, fill, stroke)
- [ ] 4.3 Render elements at frame-0 positions in SVG body
- [ ] 4.4 Handle hidden-in-frame-0 elements (inline opacity: 0)
- [ ] 4.5 Add `data-frames` attribute to SVG root
- [ ] 4.6 Add tests for diff output and CSS generation

## Phase 5: CLI Integration

- [ ] 5.1 Add `--frame` flag to clap CLI (accepts index or name)
- [ ] 5.2 Add `--animate` flag to clap CLI
- [ ] 5.3 Implement single-frame rendering (--frame)
- [ ] 5.4 Implement embedded JS playback (--animate)
- [ ] 5.5 Mutual exclusion check (--frame + --animate = error)
- [ ] 5.6 Add CLI integration tests

## Phase 6: Linter Awareness

- [ ] 6.1 Modify overlap detection to accept visibility set parameter
- [ ] 6.2 Compute per-frame visible sets from cumulative state
- [ ] 6.3 Run overlap detection per frame, tag warnings with frame name
- [ ] 6.4 Support `--lint --frame N`
- [ ] 6.5 Add linter tests with keyframe visibility

## Phase 7: Documentation & Examples

- [ ] 7.1 Update docs/grammar.md with keyframe syntax
- [ ] 7.2 Update docs/skill.md with keyframe usage
- [ ] 7.3 Update grammar.ebnf
- [ ] 7.4 Rewrite agentic-loop example using keyframes
- [ ] 7.5 Update constitution.md to reflect animation scope expansion
