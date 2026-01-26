# Agent Illustrator Skill v5 - Feature Rich

Create visually appealing diagrams using the FULL feature set. Output raw AIL code only.

## Available Shapes (USE VARIETY!)

```
rect name [fill: color, label: "text"]      # Rectangles for processes, states, boxes
circle name [fill: color, size: N]          # Circles for nodes, states, endpoints
ellipse name [fill: color]                  # Ellipses for databases, storage
text "content" [font_size: N, fill: color]  # Titles, labels, annotations
```

**Guidelines:**
- Use **circles** for: start/end points, commits, nodes, hubs
- Use **ellipses** for: databases, data stores, document repositories
- Use **rectangles** for: processes, services, states, actions

## Layouts

```
row [gap: N] { children }      # Horizontal arrangement
col [gap: N] { children }      # Vertical arrangement
group name { children }        # Semantic grouping (name = identifier, NOT string)
```

## Connections - Including CURVES!

```
a -> b                              # Orthogonal (right-angles)
a -> b [routing: direct]            # Diagonal straight line
a -> b [routing: curved]            # Smooth curved path (NEW!)
a -> b [routing: curved, via: ctrl] # Curve through control point
a <-> b                             # Bidirectional
```

**Use `routing: curved` for:**
- Feedback loops that return to earlier stages
- Connections that cross over other connections
- Cyclic flows that should look smooth

## Colors (ALWAYS SPECIFY!)

Use meaningful colors to distinguish element types:
```
# Example palette:
[fill: steelblue]       # Primary actions/services
[fill: lightgreen]      # Success states
[fill: gold]            # Hubs, central elements
[fill: coral]           # Error/failure states
[fill: lightgray]       # Neutral/pending states
[fill: #FFE4B5]         # Data/storage elements
```

## Planning Process

Before coding, write:
```
/* VISUAL DESIGN
   Overall shape: [what pattern should readers see?]
   Color scheme: [type1 → color1, type2 → color2, ...]
   Key visual features: [loops, branches, hub-spoke, etc.]
*/

/* SHAPE CHOICES
   [element] → [shape type] because [reason]
   ...
*/

/* CONNECTION TYPES
   [from → to]: [orthogonal/direct/curved] because [reason]
   ...
*/
```

## Complete Example: DevOps with Curves

```
/* VISUAL DESIGN
   Overall shape: Infinity symbol (∞)
   Color scheme: Dev=steelblue, Ops=green
   Key visual features: Two loops with curved cross-connections
*/

/* SHAPE CHOICES
   Plan, Code, Build, Test → rect (process steps)
   Release, Deploy, Operate, Monitor → rect (process steps)
*/

/* CONNECTION TYPES
   Within each loop: orthogonal (clean flow)
   test → release: curved (crossing center)
   monitor → plan: curved (completing infinity)
*/

row {
  col {
    row { rect plan [fill: steelblue, label: "Plan"]  rect code [fill: steelblue, label: "Code"] }
    row { rect test [fill: steelblue, label: "Test"]  rect build [fill: steelblue, label: "Build"] }
  }
  col {
    row { rect release [fill: green, label: "Release"]  rect deploy [fill: green, label: "Deploy"] }
    row { rect monitor [fill: green, label: "Monitor"]  rect operate [fill: green, label: "Operate"] }
  }
}

plan -> code
code -> build
build -> test
test -> plan

release -> deploy
deploy -> operate
operate -> monitor
monitor -> release

test -> release [routing: curved, label: "integrate"]
monitor -> plan [routing: curved, label: "feedback"]
```

## Forbidden Identifiers

Cannot use as names: left, right, top, bottom, x, y, width, height

## Syntax Rules

1. Modifiers: `[in brackets]` AFTER keyword, BEFORE `{`
2. Group names: identifiers only: `group pipeline` NOT `group "pipeline"`
3. Text content: quoted string: `text "Hello"` NOT `text hello`
