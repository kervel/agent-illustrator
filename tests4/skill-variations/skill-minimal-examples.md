# Agent Illustrator Skill (Minimal + Examples)

Output raw AIL code only.

## Syntax

```
SHAPES:   rect name [modifiers]  |  circle name  |  text "content"
LAYOUTS:  row { ... }  |  col { ... }  |  group name { ... }
CONNECT:  a -> b  |  a <- b  |  a <-> b  |  a -- b
MODIFY:   [fill: color, label: "text", size: N, gap: N, routing: direct]
```

FORBIDDEN as identifiers: left, right, top, bottom, x, y, width, height

## Pattern Library

### Linear Flow
```
row { rect a [label: "Start"]  rect b [label: "Process"]  rect c [label: "End"] }
a -> b
b -> c
```

### Cycle (2x2 grid)
```
col {
  row { rect a  rect b }
  row { rect d  rect c }
}
a -> b
b -> c
c -> d
d -> a
```

### Infinity/Figure-8
```
row {
  col {
    row { rect a [fill: blue]  rect b [fill: blue] }
    row { rect d [fill: blue]  rect c [fill: blue] }
  }
  col {
    row { rect e [fill: green]  rect f [fill: green] }
    row { rect h [fill: green]  rect g [fill: green] }
  }
}
a -> b -> c -> d -> a
e -> f -> g -> h -> e
d -> e [routing: direct]
h -> a [routing: direct]
```

### Hub-Spoke
```
col {
  row { rect spoke1  rect spoke2 }
  rect hub [fill: orange, label: "Central"]
  row { rect spoke3  rect spoke4 }
}
hub <-> spoke1
hub <-> spoke2
hub <-> spoke3
hub <-> spoke4
```

### Branching (Fork/Join)
```
col {
  rect start
  row { rect branch1  rect branch2  rect branch3 }
  rect join
}
start -> branch1
start -> branch2
start -> branch3
branch1 -> join
branch2 -> join
branch3 -> join
```

Think: "What pattern matches my diagram?" then adapt.
