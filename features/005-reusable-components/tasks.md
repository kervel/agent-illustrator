# Tasks: Reusable Components (Templates)

**Feature**: 005-reusable-components
**Generated**: 2026-01-24
**Status**: Ready for Implementation

<!-- Tech Stack Validation: PASSED -->
<!-- Validated against: .specswarm/tech-stack.md -->
<!-- No prohibited technologies found -->

---

## Summary

| Metric | Value |
|--------|-------|
| Total Tasks | 35 |
| Parallelizable | 18 |
| User Scenarios | 6 |
| Phases | 7 |

### User Scenario Coverage

| Scenario | Tasks | Priority |
|----------|-------|----------|
| US1: Inline Templates | T001-T025 | P1 (MVP) |
| US2: SVG Import | T026-T030 | P1 |
| US3: AIL Import | T031 | P2 |
| US4: Parameters | T020-T022 | P2 |
| US5: Exports & Connections | T032-T034 | P2 |
| US6: Error Handling | T035 | P3 |

---

## Phase 1: Foundation (Lexer & AST)

**Goal**: Add new tokens and AST types required by all scenarios.

**Checkpoint**: Parser compiles with new types, existing tests pass.

### T001: Add Template-Related Tokens to Lexer [P]

**File**: `src/parser/lexer.rs`

**Task**: Add three new tokens for template syntax.

```rust
#[token("template")]
Template,
#[token("from")]
From,
#[token("export")]
Export,
```

**Acceptance**:
- Lexer tokenizes `template`, `from`, `export` keywords
- Existing token tests still pass

---

### T002: Add TemplateSourceType Enum [P]

**File**: `src/parser/ast.rs`

**Task**: Add enum for template source types.

```rust
/// Source type for templates
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateSourceType {
    Inline,  // template "name" { ... }
    Svg,     // template "name" from "file.svg"
    Ail,     // template "name" from "file.ail"
}
```

**Acceptance**: Type compiles and is exported from ast module.

---

### T003: Add ParameterDef Struct [P]

**File**: `src/parser/ast.rs`

**Task**: Add struct for template parameter definitions.

```rust
/// Parameter definition with default value
#[derive(Debug, Clone, PartialEq)]
pub struct ParameterDef {
    pub name: Spanned<Identifier>,
    pub default_value: Spanned<StyleValue>,
}
```

**Acceptance**: Type compiles and is exported.

---

### T004: Add TemplateDecl Struct [P]

**File**: `src/parser/ast.rs`

**Task**: Add struct for template declarations.

```rust
/// Template declaration
#[derive(Debug, Clone, PartialEq)]
pub struct TemplateDecl {
    pub name: Spanned<Identifier>,
    pub source_type: TemplateSourceType,
    pub source_path: Option<Spanned<String>>,
    pub parameters: Vec<ParameterDef>,
    pub body: Option<Vec<Spanned<Statement>>>,
}
```

**Acceptance**: Type compiles.

---

### T005: Add TemplateInstance Struct [P]

**File**: `src/parser/ast.rs`

**Task**: Add struct for template instantiation.

```rust
/// Template instance: template_name "instance_name" [params]
#[derive(Debug, Clone, PartialEq)]
pub struct TemplateInstance {
    pub template_name: Spanned<Identifier>,
    pub instance_name: Spanned<Identifier>,
    pub arguments: Vec<(Spanned<Identifier>, Spanned<StyleValue>)>,
}
```

**Acceptance**: Type compiles.

---

### T006: Add ExportDecl Struct [P]

**File**: `src/parser/ast.rs`

**Task**: Add struct for export declarations.

```rust
/// Export declaration: export port1, port2
#[derive(Debug, Clone, PartialEq)]
pub struct ExportDecl {
    pub exports: Vec<Spanned<Identifier>>,
}
```

**Acceptance**: Type compiles.

---

### T007: Add Statement Variants

**File**: `src/parser/ast.rs`

**Task**: Extend Statement enum with template-related variants.

Add to `Statement` enum:
```rust
TemplateDecl(TemplateDecl),
TemplateInstance(TemplateInstance),
Export(ExportDecl),
```

**Dependencies**: T004, T005, T006

**Acceptance**: Statement enum includes new variants, compiles.

---

### T008: Add SvgEmbed ShapeType Variant

**File**: `src/parser/ast.rs`

**Task**: Add SvgEmbed variant to ShapeType for embedded SVG content.

```rust
// Add to ShapeType enum
SvgEmbed {
    content: String,
    intrinsic_width: Option<f64>,
    intrinsic_height: Option<f64>,
},
```

**Acceptance**: ShapeType compiles with new variant.

---

**CHECKPOINT**: Run `cargo build` and `cargo test`. All existing tests must pass.

---

## Phase 2: Grammar Extensions (Parsing)

**Goal**: Parse all three template declaration forms and instantiation.

**Checkpoint**: Parser accepts template syntax, produces correct AST nodes.

### T009: Parse Export Declaration

**File**: `src/parser/grammar.rs`

**Task**: Add parser rule for export statements.

```rust
let export_decl = just(Token::Export)
    .ignore_then(
        identifier.clone()
            .separated_by(just(Token::Comma))
            .at_least(1)
    )
    .map(|exports| Statement::Export(ExportDecl { exports }));
```

**Test Cases**:
- `export foo` → `ExportDecl { exports: [foo] }`
- `export foo, bar, baz` → `ExportDecl { exports: [foo, bar, baz] }`

**Acceptance**: Export statements parse correctly.

---

### T010: Parse File Template Declaration

**File**: `src/parser/grammar.rs`

**Task**: Add parser rule for file-based templates (SVG and AIL).

```rust
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
        Statement::TemplateDecl(TemplateDecl {
            name: name.map(|s| Identifier::new(s)),
            source_type,
            source_path: Some(path),
            parameters: vec![],
            body: None,
        })
    });
```

**Test Cases**:
- `template "icon" from "icons/person.svg"` → TemplateDecl with source_type=Svg
- `template "rack" from "components/rack.ail"` → TemplateDecl with source_type=Ail

**Acceptance**: File templates parse correctly.

---

### T011: Parse Parameter Definition List

**File**: `src/parser/grammar.rs`

**Task**: Add parser rule for parameter definitions.

```rust
let param_def = identifier.clone()
    .then_ignore(just(Token::Colon))
    .then(style_value.clone())
    .map(|(name, default)| ParameterDef {
        name,
        default_value: default
    });

let param_list = param_def
    .separated_by(just(Token::Comma))
    .allow_trailing()
    .delimited_by(just(Token::LParen), just(Token::RParen))
    .or_not()
    .map(|opt| opt.unwrap_or_default());
```

**Test Cases**:
- `(label: "Default")` → `[ParameterDef { name: label, default_value: "Default" }]`
- `(a: 1, b: blue)` → two ParameterDefs

**Acceptance**: Parameter lists parse correctly.

---

### T012: Parse Inline Template Declaration

**File**: `src/parser/grammar.rs`

**Task**: Add parser rule for inline template blocks.

```rust
let inline_template = just(Token::Template)
    .ignore_then(string_literal.clone())
    .then(param_list.clone())
    .then(
        statement.clone()
            .repeated()
            .delimited_by(just(Token::LBrace), just(Token::RBrace))
    )
    .map(|((name, params), body)| Statement::TemplateDecl(TemplateDecl {
        name: name.map(|s| Identifier::new(s)),
        source_type: TemplateSourceType::Inline,
        source_path: None,
        parameters: params,
        body: Some(body),
    }));
```

**Dependencies**: T011

**Test Case**:
```
template "box" {
    rect r
}
```
→ TemplateDecl with source_type=Inline, body containing rect

**Acceptance**: Inline templates with and without parameters parse correctly.

---

### T013: Combine Template Declaration Parsers

**File**: `src/parser/grammar.rs`

**Task**: Combine inline and file template parsers, add to statement parser.

```rust
let template_decl = inline_template.or(file_template);

// Add to statement parser alternatives
let statement = choice((
    // ... existing parsers
    template_decl,
    export_decl,
    // ...
));
```

**Dependencies**: T010, T012

**Acceptance**: Both template forms parse as statements.

---

### T014: Handle Template Instance Parsing

**File**: `src/parser/grammar.rs`

**Task**: Parse potential template instances (identifier followed by string literal).

**Note**: At parse time, we can't distinguish `rect "foo"` from `mytemplate "foo"`.
Strategy: Parse uniformly, resolve during template resolution phase based on known template names.

The existing shape parsing should handle this - template instances look like shapes.
During resolution, reclassify shapes whose type_name matches a template as TemplateInstance.

**Acceptance**: Syntax `identifier "name" [mods]` parses correctly.

---

**CHECKPOINT**: Parser tests for template syntax pass.

---

## Phase 3: Template Module Infrastructure

**Goal**: Create template resolution module with types and file loading.

**Checkpoint**: Template module compiles, can load files.

### T015: Create Template Module Structure [P]

**File**: `src/template/mod.rs` (new)

**Task**: Create the template module with submodule declarations.

```rust
//! Template resolution and expansion for reusable components

mod resolver;
mod svg;
mod expansion;
mod error;

pub use resolver::{TemplateResolver, TemplateRegistry, ResolvedTemplate, TemplateContent};
pub use svg::SvgInfo;
pub use expansion::expand_document;
pub use error::{TemplateError, ExpansionError};
```

Also add `mod template;` to `src/lib.rs`.

**Acceptance**: Module structure created, compiles.

---

### T016: Create Template Error Types [P]

**File**: `src/template/error.rs` (new)

**Task**: Define error types for template operations.

```rust
use std::path::PathBuf;
use crate::parser::Span;

#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    #[error("file not found: {path:?}")]
    FileNotFound { path: PathBuf, declared_at: Span },

    #[error("circular dependency detected: {path:?}")]
    CircularDependency { path: PathBuf, cycle: Vec<PathBuf> },

    #[error("invalid SVG: {reason}")]
    InvalidSvg { path: PathBuf, reason: String },

    #[error("parse error in {path:?}")]
    ParseError { path: PathBuf, message: String },

    #[error("unknown template: {name}")]
    UnknownTemplate { name: String, used_at: Span },

    #[error("duplicate template: {name}")]
    DuplicateTemplate { name: String, first: Span, second: Span },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum ExpansionError {
    #[error("cannot access non-exported element '{element}' in '{instance}'")]
    NonExportedElement {
        instance: String,
        element: String,
        available_exports: Vec<String>,
        used_at: Span,
    },

    #[error("unknown parameter '{parameter}' for template '{template}'")]
    UnknownParameter {
        template: String,
        parameter: String,
        available: Vec<String>,
        used_at: Span,
    },

    #[error("SVG templates do not support parameters")]
    SvgNoParameters { template: String, used_at: Span },
}
```

**Acceptance**: Error types compile with thiserror.

---

### T017: Create SvgInfo Parser [P]

**File**: `src/template/svg.rs` (new)

**Task**: Parse SVG files to extract dimensions.

```rust
#[derive(Debug, Clone)]
pub struct SvgInfo {
    pub content: String,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub view_box: Option<(f64, f64, f64, f64)>,
}

impl SvgInfo {
    pub fn parse(content: &str) -> Result<Self, String> {
        let width = extract_attr(content, "width");
        let height = extract_attr(content, "height");
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
            if h > 0.0 { w / h } else { 1.0 }
        } else if let (Some(w), Some(h)) = (self.width, self.height) {
            if h > 0.0 { w / h } else { 1.0 }
        } else {
            1.0
        }
    }
}

fn extract_attr(svg: &str, attr: &str) -> Option<f64> {
    // Simple regex to extract numeric attribute value
    let pattern = format!(r#"{}="([0-9.]+)"#, attr);
    regex::Regex::new(&pattern).ok()
        .and_then(|re| re.captures(svg))
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().parse().ok())
}

fn extract_viewbox(svg: &str) -> Option<(f64, f64, f64, f64)> {
    let re = regex::Regex::new(r#"viewBox="([0-9.-]+)\s+([0-9.-]+)\s+([0-9.-]+)\s+([0-9.-]+)""#).ok()?;
    let caps = re.captures(svg)?;
    Some((
        caps.get(1)?.as_str().parse().ok()?,
        caps.get(2)?.as_str().parse().ok()?,
        caps.get(3)?.as_str().parse().ok()?,
        caps.get(4)?.as_str().parse().ok()?,
    ))
}
```

**Note**: Uses regex crate (lightweight, already available in Rust ecosystem).

**Acceptance**: Can parse SVG strings and extract dimensions.

---

### T018: Create TemplateRegistry and ResolvedTemplate

**File**: `src/template/resolver.rs` (new)

**Task**: Define template registry types.

```rust
use std::collections::{HashMap, HashSet};
use crate::parser::ast::{ParameterDef, Statement, Spanned, TemplateSourceType};
use super::svg::SvgInfo;
use super::error::TemplateError;

#[derive(Debug, Clone)]
pub struct ResolvedTemplate {
    pub name: String,
    pub source_type: TemplateSourceType,
    pub parameters: Vec<ParameterDef>,
    pub exports: HashSet<String>,
    pub content: TemplateContent,
}

#[derive(Debug, Clone)]
pub enum TemplateContent {
    Inline(Vec<Spanned<Statement>>),
    Svg(SvgInfo),
    Ail(Vec<Spanned<Statement>>),
}

#[derive(Debug, Default)]
pub struct TemplateRegistry {
    templates: HashMap<String, ResolvedTemplate>,
}

impl TemplateRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, template: ResolvedTemplate) -> Result<(), TemplateError> {
        if self.templates.contains_key(&template.name) {
            return Err(TemplateError::DuplicateTemplate {
                name: template.name.clone(),
                first: Default::default(),  // Would need span tracking
                second: Default::default(),
            });
        }
        self.templates.insert(template.name.clone(), template);
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&ResolvedTemplate> {
        self.templates.get(name)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.templates.contains_key(name)
    }
}
```

**Acceptance**: Types compile and basic operations work.

---

### T019: Create TemplateResolver

**File**: `src/template/resolver.rs`

**Task**: Implement template resolution with file loading and cycle detection.

```rust
use std::path::PathBuf;

pub struct TemplateResolver {
    base_path: PathBuf,
    in_progress: HashSet<PathBuf>,
}

impl TemplateResolver {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            base_path,
            in_progress: HashSet::new(),
        }
    }

    pub fn resolve_all(&mut self, doc: &Document) -> Result<TemplateRegistry, TemplateError> {
        let mut registry = TemplateRegistry::new();

        // First pass: collect all template declarations
        for stmt in &doc.statements {
            if let Statement::TemplateDecl(decl) = &stmt.node {
                let resolved = self.resolve_template(decl)?;
                registry.insert(resolved)?;
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

    fn resolve_inline(&self, decl: &TemplateDecl) -> Result<ResolvedTemplate, TemplateError> {
        let exports = extract_exports_from_body(decl.body.as_ref().unwrap_or(&vec![]));
        Ok(ResolvedTemplate {
            name: decl.name.node.0.clone(),
            source_type: TemplateSourceType::Inline,
            parameters: decl.parameters.clone(),
            exports,
            content: TemplateContent::Inline(decl.body.clone().unwrap_or_default()),
        })
    }

    fn resolve_svg(&self, decl: &TemplateDecl) -> Result<ResolvedTemplate, TemplateError> {
        let path = decl.source_path.as_ref().unwrap();
        let full_path = self.base_path.join(&path.node);
        let content = std::fs::read_to_string(&full_path)?;
        let svg_info = SvgInfo::parse(&content)
            .map_err(|reason| TemplateError::InvalidSvg {
                path: full_path,
                reason
            })?;

        Ok(ResolvedTemplate {
            name: decl.name.node.0.clone(),
            source_type: TemplateSourceType::Svg,
            parameters: vec![],
            exports: HashSet::new(),
            content: TemplateContent::Svg(svg_info),
        })
    }

    fn resolve_ail(&mut self, decl: &TemplateDecl) -> Result<ResolvedTemplate, TemplateError> {
        let path = decl.source_path.as_ref().unwrap();
        let full_path = self.base_path.join(&path.node).canonicalize()?;

        // Circular dependency check
        if self.in_progress.contains(&full_path) {
            return Err(TemplateError::CircularDependency {
                path: full_path.clone(),
                cycle: self.in_progress.iter().cloned().collect(),
            });
        }
        self.in_progress.insert(full_path.clone());

        // Load and parse
        let content = std::fs::read_to_string(&full_path)?;
        let parsed = crate::parser::parse(&content)
            .map_err(|e| TemplateError::ParseError {
                path: full_path.clone(),
                message: format!("{:?}", e)
            })?;

        let exports = extract_exports_from_body(&parsed.statements);

        self.in_progress.remove(&full_path);

        Ok(ResolvedTemplate {
            name: decl.name.node.0.clone(),
            source_type: TemplateSourceType::Ail,
            parameters: vec![],
            exports,
            content: TemplateContent::Ail(parsed.statements),
        })
    }
}

fn extract_exports_from_body(stmts: &[Spanned<Statement>]) -> HashSet<String> {
    let mut exports = HashSet::new();
    for stmt in stmts {
        if let Statement::Export(export_decl) = &stmt.node {
            for id in &export_decl.exports {
                exports.insert(id.node.0.clone());
            }
        }
    }
    exports
}
```

**Dependencies**: T017, T018

**Acceptance**: Can resolve inline, SVG, and AIL templates. Detects circular dependencies.

---

**CHECKPOINT**: Template module compiles. Unit tests for SVG parsing and registry.

---

## Phase 4: Template Expansion (US1: Inline Templates - MVP)

**Goal**: Expand inline template instances into concrete AST elements.

**User Scenario**: US1 - Define and Use an Inline Template

**Checkpoint**: Inline templates work end-to-end.

### T020: Create Expansion Context with Parameter Binding

**File**: `src/template/expansion.rs` (new)

**Task**: Create expansion context with parameter binding.

```rust
use std::collections::HashMap;
use crate::parser::ast::*;
use super::{TemplateRegistry, ResolvedTemplate, TemplateContent};
use super::error::ExpansionError;

pub struct ExpansionContext<'a> {
    registry: &'a TemplateRegistry,
}

impl<'a> ExpansionContext<'a> {
    pub fn new(registry: &'a TemplateRegistry) -> Self {
        Self { registry }
    }

    fn bind_parameters(
        &self,
        template: &ResolvedTemplate,
        arguments: &[(Spanned<Identifier>, Spanned<StyleValue>)],
    ) -> Result<HashMap<String, StyleValue>, ExpansionError> {
        let mut bindings = HashMap::new();

        // Start with defaults
        for param in &template.parameters {
            bindings.insert(
                param.name.node.0.clone(),
                param.default_value.node.clone(),
            );
        }

        // Override with provided arguments
        for (name, value) in arguments {
            if !bindings.contains_key(&name.node.0) {
                return Err(ExpansionError::UnknownParameter {
                    template: template.name.clone(),
                    parameter: name.node.0.clone(),
                    available: template.parameters.iter()
                        .map(|p| p.name.node.0.clone())
                        .collect(),
                    used_at: name.span.clone(),
                });
            }
            bindings.insert(name.node.0.clone(), value.node.clone());
        }

        Ok(bindings)
    }
}
```

**Acceptance**: Parameter binding works with defaults and overrides.

---

### T021: Implement Identifier Prefixing

**File**: `src/template/expansion.rs`

**Task**: Implement recursive identifier prefixing for namespace scoping.

```rust
impl<'a> ExpansionContext<'a> {
    fn prefix_identifier(&self, id: &Identifier, prefix: &str) -> Identifier {
        Identifier::new(format!("{}.{}", prefix, id.0))
    }

    fn prefix_statement(
        &self,
        stmt: &Spanned<Statement>,
        prefix: &str,
        bindings: &HashMap<String, StyleValue>,
    ) -> Result<Spanned<Statement>, ExpansionError> {
        match &stmt.node {
            Statement::Shape(shape) => {
                let mut new_shape = shape.clone();
                if let Some(ref name) = new_shape.name {
                    new_shape.name = Some(name.clone().map(|id| self.prefix_identifier(&id, prefix)));
                }
                // Substitute parameter values in modifiers
                new_shape.modifiers = self.substitute_modifiers(&new_shape.modifiers, bindings);
                Ok(Spanned::new(Statement::Shape(new_shape), stmt.span.clone()))
            }
            Statement::Connection(conn) => {
                let mut new_conn = conn.clone();
                // Prefix from and to paths
                // ... implementation
                Ok(Spanned::new(Statement::Connection(new_conn), stmt.span.clone()))
            }
            Statement::Export(_) => {
                // Exports are not emitted in expanded output - return empty/skip
                Ok(stmt.clone())  // Will be filtered out
            }
            _ => Ok(stmt.clone()),
        }
    }

    fn substitute_modifiers(
        &self,
        modifiers: &[Spanned<StyleModifier>],
        bindings: &HashMap<String, StyleValue>,
    ) -> Vec<Spanned<StyleModifier>> {
        modifiers.iter().map(|m| {
            let mut new_mod = m.clone();
            if let StyleValue::Identifier(ref id) = new_mod.node.value.node {
                if let Some(bound_value) = bindings.get(&id.0) {
                    new_mod.node.value = Spanned::new(bound_value.clone(), new_mod.node.value.span.clone());
                }
            }
            new_mod
        }).collect()
    }
}
```

**Acceptance**: Identifiers in expanded templates have instance prefix.

---

### T022: Implement expand_instance for AIL Templates

**File**: `src/template/expansion.rs`

**Task**: Expand AIL template instances (inline and file).

```rust
impl<'a> ExpansionContext<'a> {
    fn expand_instance(
        &self,
        inst: &TemplateInstance,
    ) -> Result<Vec<Spanned<Statement>>, ExpansionError> {
        let template = self.registry.get(&inst.template_name.node.0)
            .ok_or_else(|| ExpansionError::UnknownTemplate {
                name: inst.template_name.node.0.clone(),
            })?;

        let prefix = &inst.instance_name.node.0;

        match &template.content {
            TemplateContent::Svg(svg_info) => {
                self.create_svg_shape(inst, svg_info)
            }
            TemplateContent::Inline(stmts) | TemplateContent::Ail(stmts) => {
                let bindings = self.bind_parameters(template, &inst.arguments)?;

                let mut expanded = Vec::new();
                for stmt in stmts {
                    if matches!(&stmt.node, Statement::Export(_)) {
                        continue;
                    }
                    let prefixed = self.prefix_statement(stmt, prefix, &bindings)?;
                    expanded.push(prefixed);
                }
                Ok(expanded)
            }
        }
    }
}
```

**Dependencies**: T020, T021

**Acceptance**: AIL template instances expand with prefixed identifiers and bound parameters.

---

### T023: Implement expand_document

**File**: `src/template/expansion.rs`

**Task**: Implement main document expansion function.

```rust
pub fn expand_document(
    doc: Document,
    registry: &TemplateRegistry,
) -> Result<Document, ExpansionError> {
    let ctx = ExpansionContext::new(registry);

    let mut expanded_statements = Vec::new();

    for stmt in doc.statements {
        match &stmt.node {
            Statement::TemplateDecl(_) => {
                // Template declarations are consumed; don't emit
                continue;
            }
            Statement::TemplateInstance(inst) => {
                let expanded = ctx.expand_instance(inst)?;
                expanded_statements.extend(expanded);
            }
            Statement::Export(_) => {
                // Top-level exports are metadata; don't emit
                continue;
            }
            _ => {
                expanded_statements.push(stmt);
            }
        }
    }

    Ok(Document { statements: expanded_statements })
}
```

**Dependencies**: T022

**Acceptance**: Full documents expand correctly.

---

### T024: Integrate Template Expansion into Pipeline

**File**: `src/lib.rs` (or main processing function)

**Task**: Add template resolution and expansion to the processing pipeline.

```rust
pub fn process_document(source: &str, base_path: PathBuf) -> Result<Document, Error> {
    // 1. Parse
    let doc = parser::parse(source)?;

    // 2. Resolve templates
    let mut resolver = template::TemplateResolver::new(base_path);
    let registry = resolver.resolve_all(&doc)?;

    // 3. Expand templates
    let expanded = template::expand_document(doc, &registry)?;

    // 4. Continue with layout, etc.
    Ok(expanded)
}
```

**Dependencies**: T019, T023

**Acceptance**: End-to-end pipeline works with templates.

---

### T025: Integration Test - Inline Template End-to-End

**File**: `tests/integration/templates.rs` (new)

**Task**: Create integration test for inline templates.

```rust
#[test]
fn test_inline_template_basic() {
    let source = r#"
        template "box" {
            rect r
        }

        box "b1"
        box "b2"
    "#;

    let result = process_and_render(source);

    // Should have two rects with prefixed IDs
    assert!(result.contains("b1"));
    assert!(result.contains("b2"));
}

#[test]
fn test_inline_template_with_params() {
    let source = r#"
        template "labeled" (text: "Default") {
            rect r
            text t [content: text]
        }

        labeled "l1" [text: "Hello"]
        labeled "l2"
    "#;

    let result = process_and_render(source);

    assert!(result.contains("Hello"));
    assert!(result.contains("Default"));
}

#[test]
fn test_forward_reference() {
    let source = r#"
        // Instance before declaration (forward reference)
        box "b1"

        template "box" {
            rect r
        }
    "#;

    let result = process(source);
    assert!(result.is_ok());
}
```

**Dependencies**: T024

**Acceptance**: Integration tests pass.

---

**CHECKPOINT - US1 COMPLETE**: Inline templates work end-to-end.

---

## Phase 5: SVG Import (US2)

**Goal**: Import and render SVG files as templates.

**User Scenario**: US2 - Import and Use an SVG Icon Multiple Times

### T026: Implement create_svg_shape

**File**: `src/template/expansion.rs`

**Task**: Create SvgEmbed shape from SVG template instance.

```rust
impl<'a> ExpansionContext<'a> {
    fn create_svg_shape(
        &self,
        inst: &TemplateInstance,
        svg_info: &SvgInfo,
    ) -> Result<Vec<Spanned<Statement>>, ExpansionError> {
        // Check SVG templates don't have parameters
        if !inst.arguments.is_empty() {
            return Err(ExpansionError::SvgNoParameters {
                template: inst.template_name.node.0.clone(),
                used_at: inst.template_name.span.clone(),
            });
        }

        let shape = Shape {
            shape_type: ShapeType::SvgEmbed {
                content: svg_info.content.clone(),
                intrinsic_width: svg_info.width,
                intrinsic_height: svg_info.height,
            },
            name: Some(inst.instance_name.clone()),
            modifiers: vec![],
        };

        Ok(vec![Spanned::new(
            Statement::Shape(shape),
            inst.template_name.span.clone(),
        )])
    }
}
```

**Acceptance**: SVG templates create SvgEmbed shapes. Parameters rejected.

---

### T027: Add SvgEmbed Layout Handling

**File**: `src/layout/engine.rs`

**Task**: Handle SvgEmbed in layout calculations.

```rust
ShapeType::SvgEmbed { intrinsic_width, intrinsic_height, .. } => {
    let aspect = match (intrinsic_width, intrinsic_height) {
        (Some(w), Some(h)) if *h > 0.0 => *w / *h,
        _ => 1.0,
    };

    // Check for explicit size modifiers
    let explicit_width = get_modifier_value(element, "width");
    let explicit_height = get_modifier_value(element, "height");

    let (width, height) = match (explicit_width, explicit_height) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => (w, w / aspect),
        (None, Some(h)) => (h * aspect, h),
        (None, None) => {
            let default_size = 100.0;
            (default_size, default_size / aspect)
        }
    };

    BoundingBox::new(x, y, width, height)
}
```

**Acceptance**: SvgEmbed shapes have correct dimensions with aspect ratio preservation.

---

### T028: Add SvgEmbed Rendering

**File**: `src/renderer/svg.rs`

**Task**: Render SvgEmbed shapes in SVG output.

```rust
ShapeType::SvgEmbed { content, intrinsic_width, intrinsic_height } => {
    let bounds = &element.bounds;

    // Calculate scale
    let scale_x = intrinsic_width.map(|w| bounds.width / w).unwrap_or(1.0);
    let scale_y = intrinsic_height.map(|h| bounds.height / h).unwrap_or(1.0);

    builder.open_element("g")
        .attr("id", element.id.as_deref().unwrap_or("svg-embed"))
        .attr("class", "svg-embed")
        .attr("transform", &format!(
            "translate({}, {}) scale({}, {})",
            bounds.x, bounds.y, scale_x, scale_y
        ));

    let inner_content = strip_svg_wrapper(content);
    builder.raw(&inner_content);

    builder.close_element("g");
}

fn strip_svg_wrapper(svg: &str) -> String {
    let s = svg.trim();
    // Remove <?xml ... ?>
    let s = if let Some(end) = s.find("?>") {
        &s[end + 2..]
    } else { s };
    let s = s.trim();
    // Remove <svg ...>
    let s = if s.starts_with("<svg") {
        if let Some(end) = s.find('>') {
            &s[end + 1..]
        } else { s }
    } else { s };
    // Remove </svg>
    let s = s.trim();
    if s.ends_with("</svg>") {
        &s[..s.len() - 6]
    } else { s }.trim().to_string()
}
```

**Acceptance**: SVG content renders correctly with positioning and scaling.

---

### T029: Integration Test - SVG Import

**File**: `tests/integration/templates.rs`

**Task**: Test SVG template import.

```rust
#[test]
fn test_svg_template_import() {
    let svg_content = r#"<svg viewBox="0 0 100 100"><circle cx="50" cy="50" r="40"/></svg>"#;
    let test_dir = tempfile::tempdir().unwrap();
    std::fs::write(test_dir.path().join("icon.svg"), svg_content).unwrap();

    let source = r#"
        template "icon" from "icon.svg"

        icon "i1"
        icon "i2" [width: 50]
    "#;

    let result = process_and_render_with_base(source, test_dir.path());

    assert!(result.matches("<circle").count() >= 2);
    assert!(result.contains("svg-embed"));
}

#[test]
fn test_svg_template_rejects_params() {
    let svg_content = r#"<svg viewBox="0 0 100 100"></svg>"#;
    let test_dir = tempfile::tempdir().unwrap();
    std::fs::write(test_dir.path().join("icon.svg"), svg_content).unwrap();

    let source = r#"
        template "icon" from "icon.svg"
        icon "i1" [color: red]
    "#;

    let result = process_with_base(source, test_dir.path());
    assert!(result.is_err());
}
```

**Dependencies**: T026, T027, T028

**Acceptance**: SVG import tests pass.

---

**CHECKPOINT - US2 COMPLETE**: SVG imports work with proper scaling.

---

## Phase 6: AIL Import & Exports (US3, US5)

**Goal**: Import external AIL files and support exports for connections.

### T031: Integration Test - AIL Import

**File**: `tests/integration/templates.rs`

**Task**: Test AIL file import.

```rust
#[test]
fn test_ail_template_import() {
    let ail_content = r#"
        rect box
        text label [content: "Server"]
        export box
    "#;
    let test_dir = tempfile::tempdir().unwrap();
    std::fs::write(test_dir.path().join("server.ail"), ail_content).unwrap();

    let source = r#"
        template "server" from "server.ail"

        server "s1"
        server "s2"
    "#;

    let result = process_and_render_with_base(source, test_dir.path());

    assert!(result.contains("s1"));
    assert!(result.contains("s2"));
}
```

**Dependencies**: T019 (AIL resolver)

**Acceptance**: AIL import works.

---

### T032: Validate Export References in Connections

**File**: `src/template/expansion.rs` or `src/layout/engine.rs`

**Task**: Validate that dot-notation connections only target exported elements.

```rust
pub fn validate_connection_target(
    path: &ElementPath,
    instance_exports: &HashMap<String, HashSet<String>>,
) -> Result<(), ExpansionError> {
    if path.components.len() > 1 {
        let instance_name = &path.components[0].node.0;
        let target_name = &path.components[1].node.0;

        if let Some(exports) = instance_exports.get(instance_name) {
            if !exports.contains(target_name) {
                return Err(ExpansionError::NonExportedElement {
                    instance: instance_name.clone(),
                    element: target_name.clone(),
                    available_exports: exports.iter().cloned().collect(),
                    used_at: path.components[1].span.clone(),
                });
            }
        }
    }
    Ok(())
}
```

**Acceptance**: Clear error when connecting to non-exported element.

---

### T033: Integration Test - Export Connections

**File**: `tests/integration/templates.rs`

**Task**: Test connecting to exported elements.

```rust
#[test]
fn test_export_connection() {
    let source = r#"
        template "router" {
            rect body
            rect wan
            rect lan
            export wan, lan
        }

        rect cable
        router "r1"

        connect cable -> r1.wan
    "#;

    let result = process_and_render(source);
    assert!(result.is_ok());
}

#[test]
fn test_non_export_connection_error() {
    let source = r#"
        template "router" {
            rect body
            rect internal
            export body
        }

        rect cable
        router "r1"

        connect cable -> r1.internal
    "#;

    let result = process(source);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not exported"));
}
```

**Dependencies**: T032

**Acceptance**: Export validation works correctly.

---

### T034: Test Circular Dependency Detection

**File**: `tests/integration/templates.rs`

**Task**: Test that circular dependencies are caught.

```rust
#[test]
fn test_circular_dependency_error() {
    let test_dir = tempfile::tempdir().unwrap();

    std::fs::write(
        test_dir.path().join("a.ail"),
        r#"template "b" from "b.ail""#
    ).unwrap();

    std::fs::write(
        test_dir.path().join("b.ail"),
        r#"template "a" from "a.ail""#
    ).unwrap();

    let source = r#"template "a" from "a.ail""#;

    let result = process_with_base(source, test_dir.path());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("circular"));
}
```

**Dependencies**: T019

**Acceptance**: Circular dependencies produce clear error.

---

**CHECKPOINT - US3, US5 COMPLETE**: AIL imports and exports work.

---

## Phase 7: Error Messages (US6)

**Goal**: Comprehensive, helpful error messages using ariadne.

### T035: Add ariadne Error Formatting

**File**: `src/template/error.rs`

**Task**: Implement ariadne-based error formatting for template errors.

```rust
use ariadne::{Report, ReportKind, Label, Source};

impl TemplateError {
    pub fn to_report(&self, filename: &str, source: &str) -> Report<(&str, std::ops::Range<usize>)> {
        match self {
            TemplateError::UnknownTemplate { name, used_at } => {
                Report::build(ReportKind::Error, filename, used_at.start)
                    .with_message(format!("Unknown template '{}'", name))
                    .with_label(
                        Label::new((filename, used_at.start..used_at.end))
                            .with_message("template not declared")
                    )
                    .with_help("Declare template with:\n  template \"name\" from \"path.ail\"\n  -- or --\n  template \"name\" { ... }")
                    .finish()
            }
            TemplateError::FileNotFound { path, declared_at } => {
                Report::build(ReportKind::Error, filename, declared_at.start)
                    .with_message(format!("File not found: {:?}", path))
                    .with_label(
                        Label::new((filename, declared_at.start..declared_at.end))
                            .with_message("file does not exist")
                    )
                    .finish()
            }
            // ... other variants
            _ => {
                Report::build(ReportKind::Error, filename, 0)
                    .with_message(self.to_string())
                    .finish()
            }
        }
    }
}
```

**Acceptance**: Errors render with helpful ariadne formatting.

---

**CHECKPOINT - FEATURE COMPLETE**: All user scenarios implemented with good error messages.

---

## Dependency Graph

```
T001 ─┬─► T007 ─► T013 ─► T014 ─► T024 ─► T025
T002 ─┤
T003 ─┤
T004 ─┤
T005 ─┤
T006 ─┘

T008 ─────────────────────────────► T026 ─► T027 ─► T028 ─► T029

T009 ─► T013
T010 ─► T013
T011 ─► T012 ─► T013

T015 ─┬─► T019 ─► T024
T016 ─┤
T017 ─┤
T018 ─┘

T020 ─► T021 ─► T022 ─► T023 ─► T024

T031 (depends on T019)
T032 ─► T033
T034 (depends on T019)
T035 (final polish)
```

## Parallel Execution Groups

**Group 1** (Foundation - all parallel):
- T001, T002, T003, T004, T005, T006, T008

**Group 2** (After Group 1):
- T007 (needs T004-T006)

**Group 3** (Parsing - after T007):
- T009, T010, T011 (parallel)
- T012 (needs T011)
- T013 (needs T010, T012)
- T014 (needs T013)

**Group 4** (Template Module - parallel with Group 3):
- T015, T016, T017, T018 (all parallel)
- T019 (needs T017, T018)

**Group 5** (Expansion - needs T019):
- T020, T021 (parallel)
- T022 (needs T020, T021)
- T023 (needs T022)
- T024 (needs T014, T019, T023)

**Group 6** (Integration tests - needs T024):
- T025, T026-T029 (parallel SVG path)
- T031-T035 (AIL and polish)

## Implementation Strategy

**MVP (Minimum Viable Product)**: Phases 1-4 (T001-T025)
- Inline templates only
- Demonstrates core value proposition
- Can be delivered independently

**Iteration 2**: Phase 5 (T026-T029)
- Add SVG import support

**Iteration 3**: Phases 6-7 (T031-T035)
- Add AIL import support
- Export validation
- Error message polish

---

*Generated by SpecSwarm tasks workflow*
