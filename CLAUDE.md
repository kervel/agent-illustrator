# Agent Illustrator

## The Problem

AI agents can generate two kinds of visual output:

1. **Diagram DSLs** (Mermaid, D2, PlantUML) — Work reliably, but only for predefined diagram types. You cannot draw a custom illustration of a robot holding a wrench.

2. **Low-level graphics** (SVG, TikZ) — Can draw anything, but LLMs fail. They hallucinate coordinates, produce malformed paths, and lack spatial reasoning.

There is no middle ground: a language that is **general-purpose** AND **LLM-friendly**.

## The Goal

Create a declarative illustration language where:

- An LLM can describe *what* to draw, not *how* to render it
- The language handles layout, positioning, and spatial relationships
- Output is predictable and correct on the first attempt
- Very strong linting helps the agent avoiding mistakes

## Success Criteria

An AI agent, given a prompt like "draw a server sending data to three clients, with the middle client highlighted as failing", produces a correct, readable illustration **without iteration or correction**.

## Non-Goals

- Replacing Mermaid for structured diagrams (flowcharts, sequences, ERDs)
- Pixel-perfect artistic control
- Animation or interactivity
- Competing with design tools (Figma, Illustrator)

## Core Principle

**Semantic over geometric.** The language describes meaning and relationships. The renderer decides coordinates.

## Development Setup

After cloning, activate the pre-commit hook:

```
git config core.hooksPath githooks
```

This re-renders all example SVGs (with Kapernikov stylesheet) and runs tests before each commit, catching rendering regressions early.

To re-render examples manually: `bash examples/render-all.sh`

## Rules for language changes

- Every new feature is discoverable: docs (`docs/skill*.md`, `grammar.md`, `examples.md`, `--doc` aliases) and an example.
- No single-purpose hacks. Guard readability and ergonomics strictly; prefer general, composable constructs.
- **The engine is domain-agnostic.** Layout, constraints, connections and lint know geometry and relations, never diagram types. No `class`, `swimlane`, `sequence`, `gantt` element or modifier, and no lint or default keyed on a domain notion (e.g. "a triangle end means inheritance"). Every diagram type (class, sequence, cross-functional, ...) is an example with a template, built from general primitives (`each`, `contains`, anchors, `constrain`, lists).
- **A need from one diagram type becomes a general primitive.** If a template is awkward, fix the primitive so every diagram type benefits (`each` over records, a neutral modifier like `bundle: apart|together`). Never add a feature only one diagram type can use.
- Two animation renderers: the JS player and the pure-CSS SVG renderer (no JS). Both must stay fully functional; where CSS genuinely can't express something, degrade gracefully.
