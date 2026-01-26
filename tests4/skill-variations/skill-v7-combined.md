# Agent Illustrator Skill v7 - Combined Approach

Create diagrams with Agent Illustrator DSL. Output raw AIL code only.

## Step 1: Identify Visual Metaphor

First, determine what SHAPE the diagram should form:

| Request mentions... | Visual metaphor | Layout approach |
|---------------------|-----------------|-----------------|
| "cycle", "loop", "continuous" | Circle/loop | 2x2 grid with cycling arrows |
| "infinity", "∞", "figure-8" | Infinity | Two 2x2 grids side-by-side |
| "pipeline", "flow", "stages" | Linear | Single row or col |
| "hub", "central", "gateway" | Hub-spoke | Central element surrounded |
| "tree", "hierarchy", "levels" | Tree | Nested cols for depth |
| "branches", "merge", "split" | Diamond/branches | Fork then join pattern |

## Step 2: Choose Shapes Semantically

| Element represents... | Use shape |
|----------------------|-----------|
| Process, action, step | rect |
| Data store, database | ellipse |
| State, node, endpoint | circle |

**Don't add decorative elements not in requirements.**

## Step 3: Plan Connections

| Connection type | Routing |
|-----------------|---------|
| Sequential flow | (default orthogonal) |
| Feedback/return | `[routing: curved]` |
| Crossing another line | `[routing: curved]` |
| Diagonal shortcut | `[routing: direct]` |

## Syntax Quick Reference

```
rect name [fill: color, label: "text"]
circle name [fill: color, size: N]
ellipse name [fill: color, label: "text"]

row [gap: N] { }
col [gap: N] { }
group name { }  // name is identifier, NOT string

a -> b [routing: curved]
```

## Layout Pattern Templates

### Infinity (∞) - for "two interconnected loops"
```
row {
  col {
    row { rect a  rect b }
    row { rect d  rect c }
  }
  col {
    row { rect e  rect f }
    row { rect h  rect g }
  }
}
// Left loop: a→b→c→d→a
// Right loop: e→f→g→h→e
// Cross: d→e, h→a (both curved)
```

### Single Loop - for "cycle", "continuous"
```
col {
  row { rect a  rect b }
  row { rect d  rect c }
}
a -> b -> c -> d
d -> a [routing: curved]
```

### Hub-spoke - for "central", "gateway"
```
col {
  row { rect s1  rect s2 }
  rect hub [fill: gold]
  row { rect s3  rect s4 }
}
hub <-> s1
hub <-> s2  // etc.
```

## Planning Comment

Before code, write:
```
/* PLAN
   Visual: [metaphor from Step 1]
   Layout: [pattern name]
   Elements: [list with shape types]
   Curves needed: [which connections]
*/
```

## Rules

1. Modifiers: `[brackets]` BEFORE `{`
2. Group names: identifiers only
3. Forbidden names: left, right, top, bottom, x, y, width, height
