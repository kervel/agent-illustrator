# Agent Illustrator Skill v3 - Minimal Rules

Output raw AIL code only.

## Core Syntax

```
rect name [fill: color, label: "text"]
circle name [size: N]
text "content" [font_size: N]

row [gap: N] { ... }
col [gap: N] { ... }
group name { ... }

a -> b [routing: direct]
a <-> b
```

## Rules

1. Modifiers go in `[brackets]` AFTER keyword, BEFORE `{`
2. Group names are identifiers: `group pipeline` NOT `group "pipeline"`
3. Forbidden names: left, right, top, bottom, x, y, width, height
4. Use `[routing: direct]` for diagonal connections

## Pattern Cheatsheet

**Cycle** (clockwise):
```
col {
  row { rect a  rect b }
  row { rect d  rect c }
}
a -> b -> c -> d -> a
```

**Infinity**:
```
row {
  col { row { rect a rect b } row { rect d rect c } }
  col { row { rect e rect f } row { rect h rect g } }
}
a -> b -> c -> d -> a
e -> f -> g -> h -> e
d -> e [routing: direct]
h -> a [routing: direct]
```

**Hub-spoke**:
```
col {
  row { rect s1  rect s2  rect s3 }
  rect hub
}
hub <-> s1
hub <-> s2
hub <-> s3
```

First write: `// PLAN: [pattern] for [intent]`
Then code.
