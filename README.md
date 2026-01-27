# Agent Illustrator

A declarative illustration language for AI agents. Describe *what* to draw, not *how* to render it.

## The Problem

AI agents can generate two kinds of visual output:

1. **Diagram DSLs** (Mermaid, D2, PlantUML) — Work reliably, but only for predefined diagram types
2. **Low-level graphics** (SVG, TikZ) — Can draw anything, but LLMs fail with coordinates and spatial reasoning

Agent Illustrator fills the gap: a **general-purpose** language that is **LLM-friendly**.

## Installation

### Nix (Linux & macOS)

```bash
# Run directly without installing
nix run github:kervel/agent-illustrator -- --help

# Install to your profile
nix profile install github:kervel/agent-illustrator
```

If flakes aren't enabled by default:
```bash
nix --extra-experimental-features 'nix-command flakes' run github:kervel/agent-illustrator
```

### Pre-built Binaries

Download from [GitHub Releases](https://github.com/kervel/agent-illustrator/releases):
- Linux (x86_64, aarch64)
- macOS (x86_64, aarch64)
- Windows (x86_64)

### From Source

```bash
cargo install --git https://github.com/kervel/agent-illustrator
```

## Quick Start

```bash
# Show examples
agent-illustrator --examples

# Show grammar reference
agent-illustrator --grammar

# Render a file
agent-illustrator diagram.ail > diagram.svg

# Use a custom stylesheet
agent-illustrator diagram.ail --stylesheet kapernikov.toml > diagram.svg
```

## Example

```
col main {
    text "System Architecture" title [font_size: 20]
    row components {
        col frontend {
            rect ui [label: "UI"]
            rect api [label: "API"]
        }
        col backend {
            rect service [label: "Service"]
            rect db [label: "Database", fill: accent-dark]
        }
    }
}
api -> service
service -> db
```

This produces an SVG with:
- Nested layouts (col contains row contains cols)
- Automatic positioning and spacing
- Connections that cross layout boundaries
- Styleable colors via CSS custom properties

## Features

- **Semantic layouts**: `row`, `col`, `stack`, `grid` — describe structure, not coordinates
- **Smart connections**: `a -> b` routes automatically, supports curved paths with `via:` points
- **Anchors**: `a.top -> b.bottom` for precise connection points
- **Styleable colors**: `accent-dark`, `secondary-light` — swap palettes with `--stylesheet`
- **Constraints**: `constrain a.left = b.right + 20` for advanced positioning
- **Templates**: Reusable components with parameters

## Documentation

```bash
agent-illustrator --grammar    # Language reference
agent-illustrator --examples   # Annotated examples
agent-illustrator --skill      # LLM integration prompt
```

## License

MIT
