# Skill Variation Experiment Results

## Goal
Find a skill variation that teaches creative problem-solving for complex layouts (like the DevOps infinity loop) while keeping the skill concise.

## Variations Tested

### Variation A: Visual Shape First
- **Approach**: Ask "what shape should the whole diagram be?" first
- **Result**: FAILED - Haiku invented wrong syntax (`[color: ...]`, `path { 1 -> 2 }`)
- **Analysis**: Too abstract, didn't provide enough syntax guidance

### Variation B: Shape Vocabulary (Table)
- **Approach**: Table mapping intent → approach
- **Result**: FAILED (syntax) but EXCELLENT conceptual thinking
- **Analysis**: Sonnet tried to draw two circles with positioned items - correct visual approach but invented `move_to`, `x:`, `y:` syntax

### Variation C: Question-Driven
- **Approach**: Three questions to answer before coding
- **Result**: FAILED - Tried to nest content inside circles (not supported)
- **Analysis**: Too philosophical, didn't teach practical patterns

### Variation D: Table + Syntax Example
- **Approach**: Variation B table + arc_to example
- **Result**: PARTIAL - Paths worked, but positioning failed
- **Analysis**: Arcs work, but AIL doesn't support arbitrary x/y positioning

### Variation Final v2: Practical Layout Patterns
- **Approach**: Table + working example with full syntax
- **Result**: SUCCESS - Both Sonnet and Haiku produced valid code
- **Analysis**: Key improvements:
  1. Maps visual intent to practical layouts (not abstract shapes)
  2. Explains "use arrows to suggest curves, not actual curved shapes"
  3. Complete working example with shape types and colors

## Key Insights

1. **AIL is layout-driven, not coordinate-driven**
   - You can't say "put text at x: 50, y: 100"
   - Must use row/col/group for positioning
   - Arrows and routing create visual connections

2. **2x2 grids + arrows effectively suggest cycles**
   - Two 2x2 grids side-by-side suggests infinity shape
   - Circular arrows within each grid suggest rotation
   - Diagonal cross-connections link the two loops

3. **Concrete examples beat abstract guidance**
   - Variation B had the right thinking but wrong syntax
   - Variation Final v2 succeeded by showing complete, working code

4. **Include shape types in examples**
   - Bare identifiers are treated as template instantiations
   - Must show `rect name [modifiers]` format

## Updated Skill (in main.rs)

The Layout Planning section now uses the Final v2 approach:
- Visual intent → layout pattern table
- Key insight about using arrows for curves
- Complete infinity loop example with working syntax

## Files
- `output-final-sonnet.svg` - Sonnet output (SUCCESS)
- `output-final-v2-haiku.svg` - Haiku output (SUCCESS)
- `skill-example-test.svg` - Skill example verification (SUCCESS)
