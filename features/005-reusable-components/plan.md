# Implementation Plan: Reusable Components (SVG and AIL Imports)

**Feature**: 005-reusable-components
**Created**: 2026-01-23
**Status**: Planning Complete

## Technical Context

### Language & Framework
- **Language**: Rust (2021 edition)
- **Build System**: Cargo
- **Parser**: chumsky 1.0.0-alpha.7 (parser combinators)
- **Lexer**: logos 0.14 (procedural macro-based lexer)
- **Error Reporting**: ariadne 0.4
- **Serialization**: serde 1.x with TOML support

### Existing Architecture

The codebase follows a clear pipeline:

1. **Lexer** (`src/parser/lexer.rs`): logos-based tokenizer producing `Token` enum
2. **Parser** (`src/parser/grammar.rs`): chumsky combinators producing AST
3. **AST** (`src/parser/ast.rs`): Typed tree representation of documents
4. **Layout** (`src/layout/`): Computes positions and bounds for elements
5. **Renderer** (`src/renderer/svg.rs`): Generates SVG output from layout

Key existing types:
- `Statement` enum: top-level constructs (Shape, Connection, Layout, Group, etc.)
- `ShapeType` enum: primitive shapes (Rectangle, Circle, Text, Icon, etc.)
- `ElementPath`: dot-notation path to nested elements (already exists for alignment)
- `Identifier`: validated name type

### Key Files to Modify

| File | Changes |
|------|---------|
| `src/parser/lexer.rs` | Add `Component`, `From`, `Export` tokens |
| `src/parser/ast.rs` | Add `ComponentDecl`, `ComponentInstance`, `ExportDecl` types |
| `src/parser/grammar.rs` | Add component/instance/export parsing rules |
| `src/layout/types.rs` | Add component-aware layout types |
| `src/layout/engine.rs` | Handle component instantiation in layout |
| `src/renderer/svg.rs` | Embed SVG content for component instances |
| `src/lib.rs` | Add import resolution module |

### New Files to Create

| File | Purpose |
|------|---------|
| `src/import/mod.rs` | Import resolution and file loading |
| `src/import/resolver.rs` | Path resolution and circular dependency detection |
| `src/import/svg.rs` | SVG parsing and dimension extraction |

## Architecture Decisions

### AD-1: Component Storage Strategy

**Decision**: Components are resolved at parse time into an expanded AST.

**Rationale**:
- Simpler layout engine (doesn't need to understand components)
- Allows validation of component references during parsing
- Component content is inlined with unique instance prefixes

**Alternative Rejected**: Runtime component instantiation
- Would require the layout engine to resolve components
- More complex error handling at layout time

### AD-2: Namespace Implementation

**Decision**: Scoped namespaces using prefixed identifiers.

When a component is instantiated, internal element names are prefixed:
```
component "rack" from "rack.ail"  // rack.ail contains: rect server
rack "r1"                          // Creates element: r1.server
rack "r2"                          // Creates element: r2.server
```

**Rationale**:
- Existing `ElementPath` type already supports dot notation
- No changes needed to the core identifier resolution
- Clear debugging (full paths visible in output)

### AD-3: Export Mechanism

**Decision**: Explicit export declarations in AIL source files.

```
rect input_port
rect output_port
export input_port, output_port
```

**Rationale**:
- Components control their public interface
- Prevents accidental coupling to internal structure
- Similar to module systems in programming languages

### AD-4: SVG Sizing Strategy

**Decision**: Extract viewBox from SVG, scale to fit layout allocation while preserving aspect ratio.

Process:
1. Parse SVG to extract width, height, or viewBox
2. Calculate aspect ratio
3. During layout, scale to fit allocated space
4. Allow `[width: X, height: Y]` overrides

**Rationale**:
- Respects SVG author's intended proportions
- Integrates naturally with existing layout system
- Explicit overrides for special cases

## Implementation Phases

### Phase 1: Grammar & AST Extensions

**Goal**: Parse component declarations, instances, and exports.

#### 1.1 Lexer Tokens

Add to `Token` enum in `lexer.rs`:
```rust
#[token("component")]
Component,
#[token("from")]
From,
#[token("export")]
Export,
```

#### 1.2 AST Types

Add to `ast.rs`:
```rust
/// Source type for component imports
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComponentSourceType {
    Svg,
    Ail,
}

/// Component declaration: component "name" from "path.svg"
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentDecl {
    pub name: Spanned<Identifier>,
    pub source_path: Spanned<String>,
    pub source_type: ComponentSourceType,
}

/// Component instance: person "alice" [styles]
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentInstance {
    pub component_name: Spanned<Identifier>,
    pub instance_name: Spanned<Identifier>,
    pub parameters: Vec<Spanned<StyleModifier>>,
}

/// Export declaration: export port1, port2
#[derive(Debug, Clone, PartialEq)]
pub struct ExportDecl {
    pub exports: Vec<Spanned<Identifier>>,
}
```

Add variants to `Statement`:
```rust
pub enum Statement {
    // ... existing variants
    ComponentDecl(ComponentDecl),
    ComponentInstance(ComponentInstance),
    Export(ExportDecl),
}
```

#### 1.3 Parser Rules

Add component parsing to `grammar.rs`:
```rust
// Component declaration
let component_decl = just(Token::Component)
    .ignore_then(string_literal.clone())
    .then_ignore(just(Token::From))
    .then(string_literal.clone())
    .map(|(name, path)| {
        let source_type = if path.node.ends_with(".svg") {
            ComponentSourceType::Svg
        } else {
            ComponentSourceType::Ail
        };
        ComponentDecl {
            name: Spanned::new(Identifier::new(name.node), name.span),
            source_path: path,
            source_type,
        }
    });

// Component instance: component_name "instance_name" [modifiers]
// This requires context - after parsing all component decls,
// we know which identifiers are component names
```

**Complexity Note**: Component instantiation looks like shape declarations (`rect "name"`). We need a two-pass approach or parser context to distinguish.

**Strategy**: First-pass collects component declarations. Second-pass resolves instances.

### Phase 2: Import Resolution

**Goal**: Load and parse imported files with cycle detection.

#### 2.1 Import Module Structure

Create `src/import/mod.rs`:
```rust
mod resolver;
mod svg;

pub use resolver::{ImportResolver, ResolvedComponent};
pub use svg::SvgInfo;
```

#### 2.2 Path Resolution

Create `src/import/resolver.rs`:
```rust
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub struct ImportResolver {
    base_path: PathBuf,
    resolved: HashMap<PathBuf, ResolvedComponent>,
    in_progress: HashSet<PathBuf>,  // For cycle detection
}

pub struct ResolvedComponent {
    pub source_type: ComponentSourceType,
    pub content: ComponentContent,
}

pub enum ComponentContent {
    Svg(SvgInfo),
    Ail(Document),  // Parsed AIL AST
}

impl ImportResolver {
    pub fn resolve(&mut self, path: &str) -> Result<ResolvedComponent, ImportError> {
        let full_path = self.base_path.join(path).canonicalize()?;

        // Check for circular import
        if self.in_progress.contains(&full_path) {
            return Err(ImportError::CircularImport(full_path));
        }

        // Check cache
        if let Some(resolved) = self.resolved.get(&full_path) {
            return Ok(resolved.clone());
        }

        // Mark as in-progress
        self.in_progress.insert(full_path.clone());

        // Load and parse
        let content = std::fs::read_to_string(&full_path)?;
        let resolved = self.parse_content(&full_path, &content)?;

        // Remove from in-progress, add to cache
        self.in_progress.remove(&full_path);
        self.resolved.insert(full_path, resolved.clone());

        Ok(resolved)
    }
}
```

#### 2.3 SVG Parsing

Create `src/import/svg.rs`:
```rust
pub struct SvgInfo {
    pub content: String,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub view_box: Option<(f64, f64, f64, f64)>,  // x, y, width, height
}

impl SvgInfo {
    pub fn parse(content: &str) -> Result<Self, SvgParseError> {
        // Extract root <svg> attributes
        // Parse width/height attributes or viewBox
        // Store raw content for embedding
    }

    pub fn aspect_ratio(&self) -> Option<f64> {
        // Calculate from viewBox or explicit dimensions
    }
}
```

### Phase 3: Component Expansion

**Goal**: Transform component instances into concrete AST elements.

#### 3.1 Expansion Context

```rust
pub struct ExpansionContext {
    components: HashMap<String, ResolvedComponent>,
    exports: HashMap<String, HashSet<String>>,  // component -> exported names
}

impl ExpansionContext {
    /// Expand a document, replacing component instances with concrete elements
    pub fn expand(&self, doc: Document) -> Result<ExpandedDocument, ExpansionError> {
        let mut expanded_statements = vec![];

        for stmt in doc.statements {
            match stmt.node {
                Statement::ComponentDecl(_) => {
                    // Already processed, skip
                }
                Statement::ComponentInstance(inst) => {
                    let expanded = self.expand_instance(&inst)?;
                    expanded_statements.extend(expanded);
                }
                Statement::Export(_) => {
                    // Record exports but don't emit
                }
                other => {
                    expanded_statements.push(Spanned::new(other, stmt.span));
                }
            }
        }

        Ok(ExpandedDocument { statements: expanded_statements })
    }

    fn expand_instance(&self, inst: &ComponentInstance) -> Result<Vec<Spanned<Statement>>, ExpansionError> {
        let component = self.components.get(inst.component_name.node.as_str())
            .ok_or(ExpansionError::UnknownComponent(inst.component_name.clone()))?;

        match &component.content {
            ComponentContent::Svg(svg_info) => {
                // Create a special SvgEmbed shape
                Ok(vec![/* SvgEmbed statement */])
            }
            ComponentContent::Ail(doc) => {
                // Prefix all identifiers with instance name
                // Recursively expand nested components
                self.prefix_and_expand(doc, &inst.instance_name.node.0)
            }
        }
    }
}
```

#### 3.2 New Shape Type for SVG Embeds

Add to `ShapeType`:
```rust
pub enum ShapeType {
    // ... existing variants
    SvgEmbed {
        content: String,
        intrinsic_width: Option<f64>,
        intrinsic_height: Option<f64>,
    },
}
```

### Phase 4: Layout Integration

**Goal**: Handle component instances in the layout engine.

#### 4.1 SVG Embed Layout

In `src/layout/engine.rs`, add handling for `SvgEmbed`:
```rust
ElementType::Shape(ShapeType::SvgEmbed { intrinsic_width, intrinsic_height, .. }) => {
    // Check for explicit size modifiers
    let (width, height) = if let (Some(w), Some(h)) = (explicit_width, explicit_height) {
        (w, h)
    } else {
        // Scale to fit allocated space while preserving aspect ratio
        let aspect = intrinsic_width.zip(intrinsic_height)
            .map(|(w, h)| w / h)
            .unwrap_or(1.0);

        let available_width = allocated_bounds.width;
        let available_height = allocated_bounds.height;

        if available_width / available_height > aspect {
            // Height-constrained
            (available_height * aspect, available_height)
        } else {
            // Width-constrained
            (available_width, available_width / aspect)
        }
    };

    BoundingBox::new(x, y, width, height)
}
```

#### 4.2 Dot-Notation Connection Resolution

Extend connection resolution to handle exported elements:
```rust
// When resolving "rack1.port1":
// 1. Find element "rack1" (the component instance)
// 2. Check if "port1" is in the component's exports
// 3. If exported, resolve to "rack1.port1" internal element
// 4. If not exported, emit error
```

### Phase 5: SVG Rendering

**Goal**: Embed SVG content in output.

#### 5.1 SVG Embed Rendering

In `src/renderer/svg.rs`:
```rust
ElementType::Shape(ShapeType::SvgEmbed { content, .. }) => {
    // Create a <g> with transform for positioning
    builder.start_group(id, &classes);

    // Add transform to position the embedded SVG
    let transform = format!(
        "translate({}, {}) scale({}, {})",
        element.bounds.x,
        element.bounds.y,
        scale_x,
        scale_y
    );

    // Embed the SVG content (stripping outer <svg> tags)
    builder.add_raw_svg(&strip_svg_wrapper(content));

    builder.end_group();
}
```

#### 5.2 SVG Content Processing

```rust
fn strip_svg_wrapper(svg: &str) -> &str {
    // Remove <?xml?> declaration
    // Remove <svg> opening tag
    // Remove </svg> closing tag
    // Return inner content
}
```

## Testing Strategy

### Unit Tests

1. **Lexer**: New tokens parse correctly
2. **Parser**: Component declarations, instances, exports parse to correct AST
3. **Import Resolution**: Path resolution, cycle detection
4. **SVG Parsing**: Dimension extraction from various SVG formats
5. **Expansion**: Component instances expand correctly with prefixes

### Integration Tests

1. **Simple SVG import**: Single SVG component, multiple instances
2. **AIL import**: AIL file with shapes, multiple instances
3. **Nested imports**: Component importing another component
4. **Exports**: Connection to exported element succeeds
5. **Non-export connection**: Connection to non-exported element fails
6. **Circular import**: Proper error for A imports B imports A
7. **Missing file**: Clear error message with path

### Snapshot Tests

Use `insta` for SVG output snapshots:
1. Document with multiple SVG component instances
2. Document with AIL component and connections to exports
3. Document with nested component hierarchy

## Error Handling

### New Error Types

```rust
pub enum ImportError {
    FileNotFound { path: PathBuf, declared_at: Span },
    CircularImport { path: PathBuf, cycle: Vec<PathBuf> },
    InvalidSvg { path: PathBuf, reason: String },
    ParseError { path: PathBuf, errors: Vec<ParseError> },
}

pub enum ExpansionError {
    UnknownComponent { name: Identifier, used_at: Span },
    NonExportedElement { component: String, element: String, used_at: Span },
    DuplicateExport { name: Identifier, first: Span, second: Span },
}
```

### Error Messages (using ariadne)

```
Error: Unknown component 'server'
   ╭─[main.ail:5:1]
   │
 5 │ server "s1"
   │ ^^^^^^ component 'server' was not declared
   │
   ╰─ help: declare component with: component "server" from "path/to/server.ail"
```

## Dependencies

No new external dependencies required. The existing stack handles all needs:
- File I/O: Rust std library
- SVG parsing: Basic regex/string parsing (no full XML parser needed)
- Path resolution: std::path

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Parser complexity with context-dependent parsing | Use two-pass approach: collect decls, then resolve instances |
| SVG compatibility issues | Support only basic SVG attributes (width, height, viewBox) |
| Performance with many imports | Cache resolved components; lazy loading |
| Nested namespace complexity | Clear prefixing rules; comprehensive tests |

## Definition of Done

- [ ] All parser tests pass
- [ ] All integration tests pass
- [ ] Snapshot tests for SVG output
- [ ] Documentation updated in grammar.ebnf
- [ ] Example files demonstrating feature
- [ ] Error messages include helpful context
- [ ] No regression in existing tests
