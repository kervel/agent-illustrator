//! Agent Illustrator CLI
//!
//! Usage:
//!   agent-illustrator [OPTIONS] [FILE]
//!
//! Options:
//!   -s, --stylesheet <FILE>  Stylesheet file for color palette (TOML format)
//!   -g, --grammar            Show language grammar reference
//!   -e, --examples           Show annotated examples
//!   --skill                  Output LLM-optimized skill document
//!   -h, --help               Print help

use std::fs;
use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;

use clap::Parser;

use agent_illustrator::{render_with_config, RenderConfig, Stylesheet};

#[derive(Parser)]
#[command(name = "agent-illustrator")]
#[command(about = "Declarative illustration language for AI agents")]
struct Cli {
    /// Input file (reads from stdin if not provided)
    input: Option<PathBuf>,

    /// Stylesheet file for color palette (TOML format)
    #[arg(short, long)]
    stylesheet: Option<PathBuf>,

    /// Debug mode: show container bounds and element IDs
    #[arg(short, long)]
    debug: bool,

    /// Show language grammar reference
    #[arg(short, long)]
    grammar: bool,

    /// Show annotated examples
    #[arg(short, long)]
    examples: bool,

    /// Output LLM-optimized skill document for agent integration
    #[arg(long)]
    skill: bool,
}

fn main() {
    let cli = Cli::parse();

    // Handle documentation flags first
    if cli.grammar {
        print_grammar();
        return;
    }

    if cli.examples {
        print_examples();
        return;
    }

    if cli.skill {
        print_skill();
        return;
    }

    // If no input file and stdin is a terminal (interactive), show intro help
    if cli.input.is_none() && io::stdin().is_terminal() {
        print_intro();
        return;
    }

    // Load stylesheet
    let stylesheet = match &cli.stylesheet {
        Some(path) => match Stylesheet::from_file(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error loading stylesheet '{}': {}", path.display(), e);
                std::process::exit(1);
            }
        },
        None => Stylesheet::default(),
    };

    // Read input
    let source = match &cli.input {
        Some(path) => match fs::read_to_string(path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Error reading file '{}': {}", path.display(), e);
                std::process::exit(1);
            }
        },
        None => {
            let mut buffer = String::new();
            match io::stdin().read_to_string(&mut buffer) {
                Ok(_) => buffer,
                Err(e) => {
                    eprintln!("Error reading from stdin: {}", e);
                    std::process::exit(1);
                }
            }
        }
    };

    // Render with stylesheet and debug mode
    let config = RenderConfig::new()
        .with_stylesheet(stylesheet)
        .with_debug(cli.debug);
    match render_with_config(&source, config) {
        Ok(svg) => {
            println!("{}", svg);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn print_intro() {
    println!(
        r#"Agent Illustrator - Declarative illustration language for AI agents

USAGE:
    agent-illustrator [OPTIONS] [FILE]
    echo '<code>' | agent-illustrator

OPTIONS:
    -g, --grammar      Show language grammar reference
    -e, --examples     Show annotated examples
    --skill            Output LLM skill document (for embedding in agent context)
    -s, --stylesheet   Custom color palette (TOML file)
    -d, --debug        Show element bounds and IDs
    -h, --help         Print help

QUICK START:
    echo 'row {{ rect a  rect b }}  a -> b' | agent-illustrator > output.svg

This creates two rectangles in a row with a connecting arrow.
Run --grammar for syntax reference or --examples for more patterns."#
    );
}

fn print_grammar() {
    println!(
        r#"AGENT ILLUSTRATOR GRAMMAR
=========================

SHAPES
------
rect [name] [modifiers]      Rectangle (default 60x40)
circle [name] [modifiers]    Circle
ellipse [name] [modifiers]   Ellipse
text "content" [name] [mod]  Text element
path [name] [mod] {{ ... }}   Custom shape with vertices/arcs

LAYOUTS
-------
row [name] [mod] {{ ... }}    Horizontal arrangement
col [name] [mod] {{ ... }}    Vertical arrangement
group [name] [mod] {{ ... }}  Semantic grouping (no layout)
stack [name] [mod] {{ ... }}  Overlapping elements

CONNECTIONS
-----------
a -> b [mod]    Directed arrow from a to b
a <- b [mod]    Directed arrow from b to a
a <-> b [mod]   Bidirectional arrow
a -- b [mod]    Undirected line

STYLE MODIFIERS
---------------
Modifiers go in brackets after the element name:
    rect mybox [fill: blue, stroke: #333, stroke_width: 2]

Common modifiers:
    fill: <color>           Fill color
    stroke: <color>         Border color
    stroke_width: <number>  Border thickness
    size: <number>          Width and height (square/circle)
    width: <number>         Explicit width
    height: <number>        Explicit height
    gap: <number>           Space between children (layouts)
    label: "text"           Add label to shape
    rotation: <degrees>     Rotate element (clockwise)
    routing: direct         Diagonal line (vs default orthogonal)

COLORS
------
Hex:      #ff0000, #f00
Named:    red, blue, green, steelblue
Symbolic: foreground, background, accent, text
          foreground-1, accent-dark, text-light

CONSTRAINTS
-----------
constrain a.left = b.left              Align left edges
constrain a.center_x = b.center_x      Center horizontally
constrain a.top = b.bottom + 20        Position with offset
constrain a.width = 100                Fixed dimension
constrain container contains a, b [padding: 10]

Properties: left, right, top, bottom, center_x, center_y, width, height

TEMPLATES
---------
template mytemplate {{ ... }}          Define reusable group
mytemplate instance_name [params]      Instantiate template

PATHS (Custom Shapes)
---------------------
path "name" [modifiers] {{
    vertex a                           Start point (origin)
    line_to b [x: 50, y: 0]            Straight line to point
    arc_to c [x: 50, y: 30, radius: 10] Curved segment
    close                              Close path to start
}}

Position syntax: [x: N, y: N]
Arc options: radius: N, bulge: N (-1 to 1), sweep: cw/ccw"#
    );
}

fn print_examples() {
    println!(
        r#"AGENT ILLUSTRATOR EXAMPLES
==========================

EXAMPLE 1: Simple shapes in a row
---------------------------------
row {{
    rect client [fill: steelblue]
    rect server [fill: green]
}}
client -> server [label: "request"]

Creates two rectangles side-by-side with a labeled arrow.

EXAMPLE 2: Nested layout
------------------------
col main {{
    text "System Architecture" title [font_size: 20]
    row components {{
        col frontend {{
            rect ui [label: "UI"]
            rect api [label: "API"]
        }}
        col backend {{
            rect service [label: "Service"]
            rect db [label: "Database", fill: orange]
        }}
    }}
}}
api -> service
service -> db

Vertical layout containing a title and a 2x2 grid of components.

EXAMPLE 3: Styling connections
------------------------------
rect a [size: 40]
rect b [size: 40]
rect c [size: 40]

row {{ a  b  c }}

a -> b [stroke: green, stroke_width: 3]
b -> c [routing: direct, stroke: red]
a <-> c [stroke_dasharray: "4,2"]

Three shapes with different connection styles: thick green arrow,
diagonal red arrow, and dashed bidirectional arrow.

EXAMPLE 4: Constraints for alignment
------------------------------------
rect header [width: 200, height: 30]
rect body [width: 200, height: 100]
rect footer [width: 200, height: 30]

constrain header.bottom = body.top
constrain body.bottom = footer.top
constrain header.center_x = body.center_x
constrain body.center_x = footer.center_x

Three rectangles stacked vertically and centered.

EXAMPLE 5: Groups with labels
-----------------------------
group server {{
    text "Web Server" [role: label, font_size: 14]
    col {{
        rect nginx [size: 30, label: "nginx"]
        rect app [size: 30, label: "app"]
    }}
}}

A labeled group containing two stacked components.

EXAMPLE 6: Custom shapes with paths
-----------------------------------
path "arrow" [fill: steelblue] {{
    vertex a
    line_to b [x: 60, y: 15]
    line_to c [x: 30, y: 0]
    line_to d [x: 30, y: 10]
    line_to e [x: 0, y: 10]
    line_to f [x: 0, y: 20]
    line_to g [x: 30, y: 20]
    line_to h [x: 30, y: 30]
    close
}}

A custom arrow shape. Paths let you define any polygon with
straight lines (line_to) or curves (arc_to with radius/bulge)."#
    );
}

fn print_skill() {
    println!(
        r#"# Agent Illustrator Skill

Create diagrams with Agent Illustrator DSL. Output raw AIL code only (no markdown).

## Quick Reference

SHAPES: rect, circle, ellipse, text "content", path "name" {{ }}
LAYOUTS: row {{ }}, col {{ }}, group {{ }}
CONNECTIONS: a -> b, a <- b, a <-> b, a -- b
MODIFIERS: [key: value, ...] after element name

## Core Patterns

```
row {{ circle start  rect process  circle end }}
start -> process
process -> end
```

```
rect box [fill: steelblue, size: 50]
circle node [fill: red, size: 30]
ellipse db [fill: orange, width: 60, height: 40]
```

```
col {{
    text "Title" [font_size: 18]
    row {{
        rect svc [label: "Service"]
        ellipse db [label: "DB"]
    }}
}}
```

```
a -> b [label: "flow", stroke: green]
a -> b [routing: direct]  // diagonal line
```

```
path "rounded" [fill: blue] {{
    vertex a
    line_to b [x: 40, y: 0]
    arc_to c [x: 50, y: 10, radius: 10]
    line_to d [x: 50, y: 30]
    arc_to e [x: 40, y: 40, radius: 10]
    close
}}
```

## Layout Planning

For complex diagrams, plan the structure before coding:

1. **Identify logical groups** - what belongs together?
   - Architecture: tiers (frontend, backend, database)
   - Network: zones (internal, DMZ, external)
   - Flowchart: paths (success, failure, retry)

2. **Choose arrangement** - side-by-side or stacked?
   - `row {{ }}` places groups horizontally
   - `col {{ }}` places groups vertically
   - Nest them for complex layouts

3. **Write LAYOUT plan first, then code**

Examples:
- "LAYOUT: 3 tiers stacked (clients, services, databases)"
- "LAYOUT: gateway left, 3 service columns right with own DBs"
- "LAYOUT: shared top, then row with success/failure lanes"

## Modifiers

| Modifier | Example | Purpose |
|----------|---------|---------|
| fill | fill: blue | Fill color |
| stroke | stroke: #333 | Border color |
| size | size: 40 | Width=height |
| width/height | width: 100 | Explicit size |
| gap | gap: 20 | Layout spacing |
| label | label: "x" | Shape label |
| routing | routing: direct | Diagonal lines |

## Rules

1. Names are identifiers: letters, numbers, underscore (e.g., `myShape`, `item_1`)
2. Reserved words cannot be names: left, right, top, bottom, x, y, width, height
3. Shapes in layouts auto-position; connections reference shapes by name
4. Path names must be quoted: `path "name" {{ }}`

## Usage

```bash
echo 'row {{ rect a  rect b }}  a -> b' | agent-illustrator > out.svg
```

## More Help

Run `agent-illustrator --examples` for annotated examples.
Run `agent-illustrator --grammar` for full syntax reference."#
    );
}
