# Implementation Plan: Reusable Components (Templates)

**Feature**: 005-reusable-components
**Created**: 2026-01-23
**Updated**: 2026-01-24
**Status**: Planning Complete

## Technical Context

### Language & Framework
- **Language**: Rust (2021 edition)
- **Build System**: Cargo
- **Parser**: chumsky 1.0.0-alpha.7 (parser combinators)
- **Lexer**: logos 0.14 (procedural macro-based lexer)
- **Error Reporting**: ariadne 0.4
- **Constraint Solver**: kasuari (Cassowary implementation)

### Existing Architecture

The codebase follows a clear pipeline:

1. **Lexer** (`src/parser/lexer.rs`): logos-based tokenizer producing `Token` enum
2. **Parser** (`src/parser/grammar.rs`): chumsky combinators producing AST
3. **AST** (`src/parser/ast.rs`): Typed tree representation of documents
4. **Layout** (`src/layout/`): Computes positions and bounds for elements
5. **Constraint Solver** (`src/layout/solver.rs`): Cassowary-based constraint resolution
6. **Renderer** (`src/renderer/svg.rs`): Generates SVG output from layout

Key existing types:
- `Statement` enum: top-level constructs (Shape, Connection, Layout, Group, Constrain, etc.)
- `ShapeType` enum: primitive shapes (Rectangle, Circle, Text, Icon, etc.)
- `ElementPath`: dot-notation path to nested elements (already exists for alignment)
- `Identifier`: validated name type
- `ConstraintExpr`: constraint expressions for the solver

### Key Files to Modify

| File | Changes |
|------|---------|
| `src/parser/lexer.rs` | Add `Template`, `From`, `Export` tokens |
| `src/parser/ast.rs` | Add `TemplateDecl`, `TemplateInstance`, `ExportDecl` types |
| `src/parser/grammar.rs` | Add template/instance/export parsing rules |
| `src/layout/types.rs` | Add template-aware layout types |
| `src/layout/engine.rs` | Handle template instantiation in layout |
| `src/layout/solver.rs` | Namespace constraints for template instances |
| `src/renderer/svg.rs` | Embed SVG content for template instances |
| `src/lib.rs` | Add template resolution module |

### New Files to Create

| File | Purpose |
|------|---------|
| `src/template/mod.rs` | Template resolution and expansion |
| `src/template/resolver.rs` | Path resolution and circular dependency detection |
| `src/template/svg.rs` | SVG parsing and dimension extraction |
| `src/template/expansion.rs` | Template instantiation and namespace prefixing |

## Architecture Decisions

### AD-1: Template Storage Strategy

**Decision**: Templates are resolved in a two-phase process: collect all declarations, then expand instances.

**Rationale**:
- Supports order-independent declarations (declarative, not procedural)
- All templates are known before any instance expansion
- Forward references work naturally
- Circular dependency detection before expansion

**Process**:
1. Parse document into AST (template declarations and instances both captured)
2. Collect all template declarations (inline blocks + file imports) into a TemplateRegistry
3. Validate: no undefined template references, no circular dependencies
4. Expand all instances, prefixing internal identifiers with instance names

### AD-2: Namespace Implementation

**Decision**: Scoped namespaces using prefixed identifiers.

When a template is instantiated, internal element names are prefixed:
```
template "rack" from "rack.ail"  // rack.ail contains: rect server
rack "r1"                         // Creates element: r1.server
rack "r2"                         // Creates element: r2.server
```

**Rationale**:
- Existing `ElementPath` type already supports dot notation
- No changes needed to the core identifier resolution
- Clear debugging (full paths visible in output)

### AD-3: Three Template Sources

**Decision**: Unified syntax for three template sources:

```ail
# Inline template (AIL statements)
template "labeled_box" (label: "Default") {
    rect box
    text lbl [content: label]
    row { box lbl }
    export box
}

# External AIL file
template "server" from "components/server.ail"

# External SVG file
template "icon" from "icons/person.svg"
```

**Differences by source type**:
| Feature | Inline AIL | External AIL | External SVG |
|---------|------------|--------------|--------------|
| Parameters | Yes (explicit) | Yes (explicit) | No |
| Exports | Yes | Yes | No (bounding box only) |
| Constraints | Yes | Yes | No |
| Connection points | Exported elements | Exported elements | Bounding box edges |

### AD-4: Export Mechanism

**Decision**: Explicit export declarations in AIL templates.

```
template "router" {
    rect input_port
    rect output_port
    rect body
    export input_port, output_port
}
```

**Rationale**:
- Templates control their public interface
- Prevents accidental coupling to internal structure
- SVG templates implicitly "export" their bounding box only

### AD-5: SVG Sizing Strategy

**Decision**: Extract viewBox from SVG, scale to fit layout allocation while preserving aspect ratio.

Process:
1. Parse SVG to extract width, height, or viewBox
2. Calculate aspect ratio
3. During layout, scale to fit allocated space
4. Allow `[width: X, height: Y]` overrides on instantiation

### AD-6: Parameter Binding

**Decision**: Parameters are declared with defaults in template definition, bound at instantiation.

```ail
template "button" (label: "Click", color: blue) {
    rect bg [fill: color]
    text txt [content: label]
}

button "submit" [label: "Submit", color: green]
button "cancel"  // Uses defaults: "Click", blue
```

**Rationale**:
- Explicit over implicit (constitution principle)
- Clear what can be customized
- Type safety possible in future

## Implementation Phases

### Phase 1: Grammar & AST Extensions

**Goal**: Parse template declarations, instances, parameters, and exports.

#### 1.1 Lexer Tokens

Add to `Token` enum in `lexer.rs`:
```rust
#[token("template")]
Template,
#[token("from")]
From,
#[token("export")]
Export,
```

#### 1.2 AST Types

Add to `ast.rs`:
```rust
/// Source type for templates
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateSourceType {
    Inline,  // template "name" { ... }
    Svg,     // template "name" from "file.svg"
    Ail,     // template "name" from "file.ail"
}

/// Parameter definition with default value
#[derive(Debug, Clone, PartialEq)]
pub struct ParameterDef {
    pub name: Spanned<Identifier>,
    pub default_value: Spanned<StyleValue>,
}

/// Template declaration
#[derive(Debug, Clone, PartialEq)]
pub struct TemplateDecl {
    pub name: Spanned<Identifier>,
    pub source_type: TemplateSourceType,
    pub source_path: Option<Spanned<String>>,      // For file imports
    pub parameters: Vec<ParameterDef>,              // For AIL templates
    pub body: Option<Vec<Spanned<Statement>>>,      // For inline templates
}

/// Template instance: template_name "instance_name" [params]
#[derive(Debug, Clone, PartialEq)]
pub struct TemplateInstance {
    pub template_name: Spanned<Identifier>,
    pub instance_name: Spanned<Identifier>,
    pub arguments: Vec<(Spanned<Identifier>, Spanned<StyleValue>)>,
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
    TemplateDecl(TemplateDecl),
    TemplateInstance(TemplateInstance),
    Export(ExportDecl),
}
```

#### 1.3 Parser Rules

Add template parsing to `grammar.rs`:
```rust
// Parameter definition: name: default_value
let param_def = identifier.clone()
    .then_ignore(just(Token::Colon))
    .then(style_value.clone())
    .map(|(name, default)| ParameterDef { name, default_value: default });

// Parameter list: (param1: val1, param2: val2)
let param_list = param_def
    .separated_by(just(Token::Comma))
    .delimited_by(just(Token::LParen), just(Token::RParen))
    .or_not()
    .map(|opt| opt.unwrap_or_default());

// Inline template: template "name" (params) { body }
let inline_template = just(Token::Template)
    .ignore_then(string_literal.clone())
    .then(param_list.clone())
    .then(statement.repeated().delimited_by(just(Token::LBrace), just(Token::RBrace)))
    .map(|((name, params), body)| TemplateDecl {
        name: Spanned::new(Identifier::new(name.node), name.span),
        source_type: TemplateSourceType::Inline,
        source_path: None,
        parameters: params,
        body: Some(body),
    });

// File template: template "name" from "path"
let file_template = just(Token::Template)
    .ignore_then(string_literal.clone())
    .then_ignore(just(Token::From))
    .then(string_literal.clone())
    .map(|(name, path)| {
        let source_type = if path.node.ends_with(".svg") {
            TemplateSourceType::Svg
        } else {
            TemplateSourceType::Ail
        };
        TemplateDecl {
            name: Spanned::new(Identifier::new(name.node), name.span),
            source_type,
            source_path: Some(path),
            parameters: vec![],  // Loaded from file for AIL
            body: None,
        }
    });

// Combined template declaration
let template_decl = inline_template.or(file_template);

// Export declaration: export name1, name2
let export_decl = just(Token::Export)
    .ignore_then(identifier.separated_by(just(Token::Comma)))
    .map(|exports| ExportDecl { exports });
```

**Complexity Note**: Template instantiation (`person "alice"`) looks like shape declarations (`rect "name"`). Strategy:
1. Parse all identifiers followed by string literals as potential instances
2. During resolution phase, classify as shape or template instance based on known template names

### Phase 2: Template Resolution

**Goal**: Resolve all template declarations and validate references.

#### 2.1 Template Module Structure

Create `src/template/mod.rs`:
```rust
mod resolver;
mod svg;
mod expansion;

pub use resolver::{TemplateResolver, TemplateRegistry, ResolvedTemplate};
pub use svg::SvgInfo;
pub use expansion::expand_document;
```

#### 2.2 Template Registry

Create `src/template/resolver.rs`:
```rust
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub struct TemplateRegistry {
    templates: HashMap<String, ResolvedTemplate>,
}

pub struct ResolvedTemplate {
    pub name: String,
    pub source_type: TemplateSourceType,
    pub parameters: Vec<ParameterDef>,
    pub exports: HashSet<String>,
    pub content: TemplateContent,
}

pub enum TemplateContent {
    Inline(Vec<Spanned<Statement>>),
    Svg(SvgInfo),
    Ail(Vec<Spanned<Statement>>),  // Parsed from file
}

pub struct TemplateResolver {
    base_path: PathBuf,
    in_progress: HashSet<PathBuf>,  // For cycle detection
}

impl TemplateResolver {
    /// Build registry from document, resolving all templates
    pub fn resolve_all(
        &mut self,
        doc: &Document,
    ) -> Result<TemplateRegistry, TemplateError> {
        let mut registry = TemplateRegistry::new();

        // First pass: collect all template declarations
        for stmt in &doc.statements {
            if let Statement::TemplateDecl(decl) = &stmt.node {
                let resolved = self.resolve_template(decl)?;
                registry.insert(resolved)?;
            }
        }

        // Validate: no undefined template references
        for stmt in &doc.statements {
            if let Statement::TemplateInstance(inst) = &stmt.node {
                if !registry.contains(&inst.template_name.node.0) {
                    return Err(TemplateError::UnknownTemplate {
                        name: inst.template_name.clone(),
                    });
                }
            }
        }

        Ok(registry)
    }

    fn resolve_template(&mut self, decl: &TemplateDecl) -> Result<ResolvedTemplate, TemplateError> {
        match decl.source_type {
            TemplateSourceType::Inline => self.resolve_inline(decl),
            TemplateSourceType::Svg => self.resolve_svg(decl),
            TemplateSourceType::Ail => self.resolve_ail(decl),
        }
    }

    fn resolve_ail(&mut self, decl: &TemplateDecl) -> Result<ResolvedTemplate, TemplateError> {
        let path = decl.source_path.as_ref().unwrap();
        let full_path = self.base_path.join(&path.node).canonicalize()?;

        // Circular dependency check
        if self.in_progress.contains(&full_path) {
            return Err(TemplateError::CircularDependency { path: full_path });
        }
        self.in_progress.insert(full_path.clone());

        // Load and parse AIL file
        let content = std::fs::read_to_string(&full_path)?;
        let parsed = parse_ail(&content)?;

        // Extract exports
        let exports = extract_exports(&parsed);

        // Extract parameters (from template declaration in file if present)
        let parameters = extract_parameters(&parsed);

        self.in_progress.remove(&full_path);

        Ok(ResolvedTemplate {
            name: decl.name.node.0.clone(),
            source_type: TemplateSourceType::Ail,
            parameters,
            exports,
            content: TemplateContent::Ail(parsed.statements),
        })
    }
}
```

#### 2.3 SVG Parsing

Create `src/template/svg.rs`:
```rust
pub struct SvgInfo {
    pub content: String,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub view_box: Option<(f64, f64, f64, f64)>,
}

impl SvgInfo {
    pub fn parse(content: &str) -> Result<Self, SvgParseError> {
        // Extract <svg> attributes using regex (no full XML parser needed)
        let width = extract_svg_attr(content, "width");
        let height = extract_svg_attr(content, "height");
        let view_box = extract_viewbox(content);

        Ok(Self {
            content: content.to_string(),
            width,
            height,
            view_box,
        })
    }

    pub fn aspect_ratio(&self) -> f64 {
        if let Some((_, _, w, h)) = self.view_box {
            w / h
        } else if let (Some(w), Some(h)) = (self.width, self.height) {
            w / h
        } else {
            1.0  // Default square
        }
    }
}
```

### Phase 3: Template Expansion

**Goal**: Transform template instances into concrete AST elements.

#### 3.1 Expansion Context

Create `src/template/expansion.rs`:
```rust
pub struct ExpansionContext<'a> {
    registry: &'a TemplateRegistry,
}

impl<'a> ExpansionContext<'a> {
    pub fn expand_document(&self, doc: Document) -> Result<Document, ExpansionError> {
        let mut expanded_statements = vec![];

        for stmt in doc.statements {
            match &stmt.node {
                Statement::TemplateDecl(_) => {
                    // Template declarations are consumed; don't emit
                }
                Statement::TemplateInstance(inst) => {
                    let expanded = self.expand_instance(inst)?;
                    expanded_statements.extend(expanded);
                }
                Statement::Export(_) => {
                    // Exports are metadata; don't emit to expanded doc
                }
                _ => {
                    expanded_statements.push(stmt);
                }
            }
        }

        Ok(Document { statements: expanded_statements })
    }

    fn expand_instance(&self, inst: &TemplateInstance) -> Result<Vec<Spanned<Statement>>, ExpansionError> {
        let template = self.registry.get(&inst.template_name.node.0)?;
        let prefix = &inst.instance_name.node.0;

        match &template.content {
            TemplateContent::Svg(svg_info) => {
                // Create SvgEmbed shape
                Ok(vec![self.create_svg_shape(prefix, svg_info, &inst.arguments)?])
            }
            TemplateContent::Inline(stmts) | TemplateContent::Ail(stmts) => {
                // Bind parameters
                let bindings = self.bind_parameters(&template.parameters, &inst.arguments)?;

                // Prefix identifiers and substitute parameters
                let prefixed = self.prefix_statements(stmts, prefix, &bindings)?;

                Ok(prefixed)
            }
        }
    }

    fn prefix_statements(
        &self,
        stmts: &[Spanned<Statement>],
        prefix: &str,
        bindings: &HashMap<String, StyleValue>,
    ) -> Result<Vec<Spanned<Statement>>, ExpansionError> {
        // Recursively prefix all identifiers
        // Substitute parameter references with bound values
        // Handle nested template instances
    }
}
```

#### 3.2 New Shape Type for SVG Embeds

Add to `ShapeType` in `ast.rs`:
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

**Goal**: Handle template instances in the layout engine.

#### 4.1 SVG Embed Layout

In `src/layout/engine.rs`, add handling for `SvgEmbed`:
```rust
ShapeType::SvgEmbed { intrinsic_width, intrinsic_height, .. } => {
    let aspect = intrinsic_width.zip(*intrinsic_height)
        .map(|(w, h)| w / h)
        .unwrap_or(1.0);

    // Check for explicit size in modifiers
    let (width, height) = self.resolve_size_with_aspect(element, aspect);

    BoundingBox::new(x, y, width, height)
}
```

#### 4.2 Constraint Scoping

Constraints from template instances are prefixed to maintain encapsulation:
```rust
// Original constraint in template:
// constrain a.left = b.left

// After expansion with prefix "router1":
// constrain router1.a.left = router1.b.left
```

Parent constraints can reference exported elements:
```rust
// Parent document:
constrain cable.right = router1.wan.left  // OK: wan is exported

// This would error:
constrain cable.right = router1.internal.left  // ERROR: internal not exported
```

#### 4.3 Export Validation

During constraint collection, validate that dot-notation references only target exported elements:
```rust
fn validate_element_reference(&self, path: &ElementPath) -> Result<(), LayoutError> {
    if path.components.len() > 1 {
        let instance_name = &path.components[0];
        let target_name = &path.components[1];

        if let Some(template_name) = self.instance_templates.get(instance_name) {
            let template = self.registry.get(template_name)?;
            if !template.exports.contains(target_name) {
                return Err(LayoutError::NonExportedElement {
                    instance: instance_name.clone(),
                    element: target_name.clone(),
                    available_exports: template.exports.iter().cloned().collect(),
                });
            }
        }
    }
    Ok(())
}
```

### Phase 5: SVG Rendering

**Goal**: Embed SVG content in output.

#### 5.1 SVG Embed Rendering

In `src/renderer/svg.rs`:
```rust
ShapeType::SvgEmbed { content, .. } => {
    // Create group with transform
    let transform = format!(
        "translate({}, {})",
        element.bounds.x,
        element.bounds.y
    );

    builder.open_element("g")
        .attr("id", &element.id)
        .attr("transform", &transform)
        .attr("class", "svg-embed");

    // Calculate scale to fit bounds
    let scale_x = element.bounds.width / intrinsic_width.unwrap_or(element.bounds.width);
    let scale_y = element.bounds.height / intrinsic_height.unwrap_or(element.bounds.height);

    // Inner group for scaling
    builder.open_element("g")
        .attr("transform", &format!("scale({}, {})", scale_x, scale_y));

    // Embed SVG content (stripped of outer wrapper)
    builder.raw(&strip_svg_wrapper(content));

    builder.close_element("g");
    builder.close_element("g");
}
```

#### 5.2 SVG Content Processing

```rust
fn strip_svg_wrapper(svg: &str) -> String {
    // Remove XML declaration: <?xml ... ?>
    let without_xml = REGEX_XML_DECL.replace(svg, "");

    // Remove outer <svg ...> and </svg> tags, keep inner content
    let without_svg_open = REGEX_SVG_OPEN.replace(&without_xml, "");
    let without_svg_close = REGEX_SVG_CLOSE.replace(&without_svg_open, "");

    without_svg_close.to_string()
}
```

## Testing Strategy

### Unit Tests

1. **Lexer**: `template`, `from`, `export` tokens parse correctly
2. **Parser**:
   - Inline templates with parameters
   - File templates (SVG and AIL)
   - Export declarations
   - Template instances with arguments
3. **Resolution**:
   - Template registry building
   - Circular dependency detection
   - Missing file errors
4. **SVG Parsing**: Dimension extraction from various SVG formats
5. **Expansion**:
   - Identifier prefixing
   - Parameter substitution
   - Nested template expansion

### Integration Tests

1. **Inline template**: Define and instantiate inline template
2. **SVG import**: Single SVG template, multiple instances
3. **AIL import**: AIL file with shapes, multiple instances
4. **Parameters**: Template with parameters, various bindings
5. **Nested templates**: Template using another template
6. **Exports**: Connection to exported element succeeds
7. **Non-export connection**: Connection to non-exported element fails with clear error
8. **Circular dependency**: Proper error for A uses B uses A
9. **Missing file**: Clear error message with path
10. **Forward reference**: Instance before template declaration

### Snapshot Tests

Use `insta` for SVG output snapshots:
1. Document with multiple inline template instances
2. Document with SVG template instances at different sizes
3. Document with AIL template and connections to exports
4. Document with nested template hierarchy
5. Document with parameterized templates

## Error Handling

### New Error Types

```rust
pub enum TemplateError {
    FileNotFound {
        path: PathBuf,
        declared_at: Span
    },
    CircularDependency {
        path: PathBuf,
        cycle: Vec<PathBuf>
    },
    InvalidSvg {
        path: PathBuf,
        reason: String
    },
    ParseError {
        path: PathBuf,
        errors: Vec<ParseError>
    },
    UnknownTemplate {
        name: Spanned<Identifier>
    },
    DuplicateTemplate {
        name: String,
        first: Span,
        second: Span
    },
}

pub enum ExpansionError {
    NonExportedElement {
        instance: String,
        element: String,
        available_exports: Vec<String>,
        used_at: Span
    },
    MissingParameter {
        template: String,
        parameter: String,
        used_at: Span
    },
    UnknownParameter {
        template: String,
        parameter: String,
        available: Vec<String>,
        used_at: Span
    },
}
```

### Error Messages (using ariadne)

```
Error: Unknown template 'server'
   ╭─[main.ail:12:1]
   │
12 │ server "s1"
   │ ^^^^^^ template 'server' was not declared
   │
   ╰─ help: declare template with:
      template "server" from "path/to/server.ail"
      -- or --
      template "server" { ... }
```

```
Error: Cannot access non-exported element 'internal_node'
   ╭─[main.ail:15:20]
   │
15 │ connect cable -> router1.internal_node
   │                  ^^^^^^^^^^^^^^^^^^^^^ 'internal_node' is not exported by 'router1'
   │
   ├─ note: template 'router' exports: wan, lan
   ╰─ help: add 'export internal_node' to the template definition
```

## Tech Stack Compliance Report

### Approved Technologies (already in stack)
- Rust (2021 edition)
- chumsky (parser)
- logos (lexer)
- ariadne (error reporting)
- kasuari (constraint solver)
- insta (snapshot testing)

### New Technologies
None required. All functionality implemented with existing approved stack.

## Dependencies

No new external dependencies required. The existing stack handles all needs:
- File I/O: Rust std library
- SVG parsing: Basic regex/string parsing (no full XML parser needed)
- Path resolution: std::path

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Parser complexity with context-dependent parsing | Two-phase approach: parse all, then resolve template vs shape |
| Order-independent declarations | Build full template registry before expansion |
| SVG compatibility issues | Support only basic SVG attributes (width, height, viewBox) |
| Performance with many templates | Cache resolved templates; lazy file loading |
| Nested namespace complexity | Clear prefixing rules; comprehensive tests |
| Constraint scoping errors | Validate export access at constraint collection time |

## Definition of Done

- [ ] All parser tests pass for new syntax
- [ ] Template resolution handles all three source types
- [ ] Circular dependency detection works
- [ ] Forward references work (order-independent)
- [ ] Parameter binding works for AIL templates
- [ ] SVG templates have no parameters (enforced)
- [ ] Export validation prevents access to non-exported elements
- [ ] Constraint scoping preserves encapsulation
- [ ] All integration tests pass
- [ ] Snapshot tests for SVG output
- [ ] Documentation updated in grammar.ebnf
- [ ] Example files demonstrating feature
- [ ] Error messages include helpful context
- [ ] No regression in existing tests
