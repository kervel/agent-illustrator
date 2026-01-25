---
parent_branch: main
feature_number: 008
status: In Progress
created_at: 2026-01-24T18:05:00+01:00
---

# Feature: Agent Discoverability

## Overview

Enable AI agents to discover and learn the Agent Illustrator DSL without prior knowledge. When an agent calls the CLI without arguments, it should receive self-explaining help. Additional flags provide access to grammar documentation, usage examples, and a concise agent-oriented skill description that can be embedded directly into an agent's context.

The goal is zero-shot usability: an AI agent with no training on this DSL should be able to produce correct illustrations by reading the CLI's built-in documentation.

## User Scenarios

### Scenario 1: Agent Discovers the CLI
An AI agent is instructed to "draw a diagram". It has access to the `agent-illustrator` binary but no prior knowledge of the language. The agent runs `agent-illustrator` without arguments and receives a helpful summary showing:
- What the tool does
- Available flags for learning more
- A minimal working example

### Scenario 2: Agent Learns the Grammar
The agent runs `agent-illustrator --grammar` to receive a formal description of the language syntax, including all shape types, layout containers, connection syntax, and style modifiers.

### Scenario 3: Agent Explores Examples
The agent runs `agent-illustrator --examples` to see annotated examples demonstrating common patterns like layouts, connections, styling, and constraints.

### Scenario 4: Agent Skill Integration
A developer wants to embed Agent Illustrator capability into an AI agent's system prompt. They run `agent-illustrator --skill` to get a concise, self-contained skill document suitable for LLM context. This skill should be comprehensive enough for the agent to produce correct output but concise enough to fit in limited context windows.

## Functional Requirements

### FR-1: Default Help (No Arguments)
When the CLI is invoked with no arguments and no stdin input:
- Display a brief description of the tool
- List available flags with short descriptions
- Show a minimal working example that produces valid output
- Exit with code 0

### FR-2: Grammar Flag
The `--grammar` or `-g` flag displays:
- Shape types: rect, circle, ellipse, text, icon
- Layout containers: row, col, group, stack, grid
- Connection operators: `->`, `<-`, `<->`, `--`
- Style modifiers: fill, stroke, stroke_width, size, width, height, gap, label, rotation, routing, etc.
- Constraint syntax: constrain statements with properties (left, right, top, bottom, center_x, center_y, width, height)
- Color values: hex (#ff0000), named (red), symbolic (foreground-1, accent-dark)
- Template syntax: template declaration and instantiation

### FR-3: Examples Flag
The `--examples` or `-e` flag displays:
- Simple shape example with annotation
- Layout example (row/col with children)
- Connection example with styling
- Constraint example
- Each example shows both the input and describes the expected output

### FR-4: Skill Flag
The `--skill` flag outputs a concise skill document containing:
- Tool purpose (1-2 sentences)
- Grammar summary (compact reference)
- Common patterns (most-used constructs)
- Anti-patterns to avoid
- Example workflow

The skill document should be:
- Under 2000 tokens when tokenized
- Self-contained (no external references)
- Optimized for LLM consumption (clear structure, explicit rules)

### FR-5: Backward Compatibility
- Existing CLI behavior with input files or stdin remains unchanged
- `--help` flag behavior remains standard (clap-generated)
- New flags do not conflict with existing `-s` (stylesheet) or `-d` (debug) flags

## Success Criteria

- An AI agent with access only to CLI documentation can produce valid AIL code on the first attempt
- Running `agent-illustrator --skill` produces output that fits in a 4K token context window
- All documentation is machine-readable (clean formatting, no interactive elements)
- Test: A subagent spawned without prior context can read the skill and produce a working diagram

## Key Entities

| Entity | Description |
|--------|-------------|
| Grammar | Formal syntax description of the AIL language |
| Examples | Annotated code samples demonstrating patterns |
| Skill | Concise LLM-optimized documentation for agent integration |

## Assumptions

- Agents have terminal access and can execute the CLI
- Skill content should be static (embedded in binary, not fetched)
- The grammar description does not need to be a formal BNF; a readable specification is sufficient
- Examples should demonstrate core features, not exhaustive edge cases
