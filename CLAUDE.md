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

## Success Criteria

An AI agent, given a prompt like "draw a server sending data to three clients, with the middle client highlighted as failing", produces a correct, readable illustration **without iteration or correction**.

## Non-Goals

- Replacing Mermaid for structured diagrams (flowcharts, sequences, ERDs)
- Pixel-perfect artistic control
- Animation or interactivity
- Competing with design tools (Figma, Illustrator)

## Core Principle

**Semantic over geometric.** The language describes meaning and relationships. The renderer decides coordinates.
