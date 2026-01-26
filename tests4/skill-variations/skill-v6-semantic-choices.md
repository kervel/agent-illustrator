# Agent Illustrator Skill v6 - Semantic Choices

Create diagrams with Agent Illustrator DSL. Output raw AIL code only.

## Shape Selection Guide

Choose shapes based on what elements REPRESENT, not for variety:

| Element Type | Shape | Reason |
|--------------|-------|--------|
| Process/Action/Step | rect | Standard flowchart convention |
| Data store/Database | ellipse | Traditional database symbol |
| Start/End/State | circle | State machine convention |
| Decision point | rect (smaller) | With branching arrows |

**Don't add shapes that aren't in the requirements.** If asked for a pipeline, don't add decorative hubs.

## Connection Routing

| Situation | Routing | Why |
|-----------|---------|-----|
| Sequential steps | (default) | Clean orthogonal paths |
| Feedback loop to earlier stage | `[routing: curved]` | Shows "going back" |
| Connection crossing others | `[routing: curved]` | Avoids visual collision |
| Diagonal shortcut | `[routing: direct]` | Straight line |

## Colors

Use colors to encode MEANING:
- Group related items (same color = same category)
- Highlight important paths (distinct color for errors, success)
- Don't colorize arbitrarily

## Syntax Reference

```
rect name [fill: color, label: "text"]
circle name [fill: color, size: N]
ellipse name [fill: color, label: "text"]
text "content" [font_size: N]

row [gap: N] { }
col [gap: N] { }
group name { }  // identifier, NOT quoted string

a -> b [routing: curved]
a <-> b
```

## Planning Template

```
/* ANALYSIS
   Required elements: [list from requirements]
   Element types: [process/data/state for each]
   Connections: [which need curves for feedback/crossing]
*/
```

## Examples

### Pipeline with Feedback
```
/* ANALYSIS
   Required: Data, Process, Train, Evaluate
   Types: Data=ellipse, others=rect
   Curves: Evaluate→Process is feedback
*/

row {
  ellipse data [fill: #FFE4B5, label: "Data"]
  rect process [fill: steelblue, label: "Process"]
  rect train [fill: steelblue, label: "Train"]
  rect evaluate [fill: steelblue, label: "Evaluate"]
}
data -> process -> train -> evaluate
evaluate -> process [routing: curved, label: "iterate"]
```

### Cycle (4 steps, no decorative elements)
```
col {
  row { rect a [fill: blue, label: "A"]  rect b [fill: blue, label: "B"] }
  row { rect d [fill: blue, label: "D"]  rect c [fill: blue, label: "C"] }
}
a -> b -> c -> d -> a [routing: curved]
```

## Rules

1. Modifiers: `[brackets]` AFTER keyword, BEFORE `{`
2. Group names: `group pipeline` NOT `group "pipeline"`
3. Forbidden names: left, right, top, bottom, x, y, width, height
