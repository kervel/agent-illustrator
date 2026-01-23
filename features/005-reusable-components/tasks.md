# Tasks: Reusable Components (SVG and AIL Imports)

**Feature**: 005-reusable-components
**Generated**: 2026-01-23
**Total Tasks**: 32
**Parallel Opportunities**: 14 task groups

---

## Phase 1: Setup & Foundation

*These tasks establish the infrastructure needed by all user scenarios.*

### T001: Add new lexer tokens for component syntax
**File**: `src/parser/lexer.rs`
**Story**: Foundation
**Parallel**: No (prerequisite for all parsing)

Add three new tokens to the `Token` enum:
```rust
#[token("component")]
Component,
#[token("from")]
From,
#[token("export")]
Export,
```

Add corresponding test cases in the `mod tests` section.

**Acceptance**: `cargo test lexer` passes with new token tests.

---

### T002: Add AST types for components [P]
**File**: `src/parser/ast.rs`
**Story**: Foundation
**Parallel**: Yes (with T003)

Add the following types after the existing `AlignmentDecl`:

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

Add `SvgEmbed` variant to `ShapeType`:
```rust
SvgEmbed {
    content: String,
    intrinsic_width: Option<f64>,
    intrinsic_height: Option<f64>,
},
```

Add variants to `Statement` enum:
```rust
ComponentDecl(ComponentDecl),
ComponentInstance(ComponentInstance),
Export(ExportDecl),
```

**Acceptance**: `cargo check` passes.

---

### T003: Create import module structure [P]
**File**: `src/import/mod.rs` (new)
**Story**: Foundation
**Parallel**: Yes (with T002)

Create new module at `src/import/mod.rs`:
```rust
//! Import resolution for component files

mod resolver;
mod svg;

pub use resolver::{ImportResolver, ResolvedComponent, ComponentContent, ImportError};
pub use svg::{SvgInfo, SvgParseError};
```

Update `src/lib.rs` to include the module:
```rust
pub mod import;
```

**Acceptance**: `cargo check` passes (module exists but implementations are stubs).

---

### T004: Create error types for imports [P]
**File**: `src/import/resolver.rs` (new), `src/error.rs`
**Story**: Foundation
**Parallel**: Yes (with T005)

Create `src/import/resolver.rs` with error types:
```rust
use std::path::PathBuf;
use crate::parser::ast::Span;

#[derive(Debug, Clone)]
pub enum ImportError {
    FileNotFound { path: PathBuf, declared_at: Span },
    CircularImport { path: PathBuf, cycle: Vec<PathBuf> },
    InvalidSvg { path: PathBuf, reason: String },
    ParseError { path: PathBuf, errors: Vec<crate::ParseError> },
    IoError { path: PathBuf, message: String },
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileNotFound { path, .. } => write!(f, "File not found: {}", path.display()),
            Self::CircularImport { path, .. } => write!(f, "Circular import detected: {}", path.display()),
            Self::InvalidSvg { path, reason } => write!(f, "Invalid SVG {}: {}", path.display(), reason),
            Self::ParseError { path, .. } => write!(f, "Parse error in {}", path.display()),
            Self::IoError { path, message } => write!(f, "IO error reading {}: {}", path.display(), message),
        }
    }
}

impl std::error::Error for ImportError {}
```

**Acceptance**: `cargo check` passes.

---

### T005: Create SvgInfo type and parser [P]
**File**: `src/import/svg.rs` (new)
**Story**: Foundation
**Parallel**: Yes (with T004)

Create `src/import/svg.rs`:
```rust
//! SVG metadata extraction

#[derive(Debug, Clone)]
pub struct SvgInfo {
    pub content: String,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub view_box: Option<(f64, f64, f64, f64)>,
}

#[derive(Debug, Clone)]
pub struct SvgParseError(pub String);

impl SvgInfo {
    /// Parse SVG content and extract dimensions
    pub fn parse(content: &str) -> Result<Self, SvgParseError> {
        let view_box = Self::extract_viewbox(content);
        let width = Self::extract_dimension(content, "width");
        let height = Self::extract_dimension(content, "height");

        Ok(SvgInfo {
            content: content.to_string(),
            width,
            height,
            view_box,
        })
    }

    pub fn aspect_ratio(&self) -> Option<f64> {
        if let Some((_, _, w, h)) = self.view_box {
            if h > 0.0 { return Some(w / h); }
        }
        self.width.zip(self.height).map(|(w, h)| if h > 0.0 { w / h } else { 1.0 })
    }

    pub fn intrinsic_size(&self) -> (f64, f64) {
        if let Some((_, _, w, h)) = self.view_box {
            return (w, h);
        }
        (self.width.unwrap_or(100.0), self.height.unwrap_or(100.0))
    }

    fn extract_viewbox(content: &str) -> Option<(f64, f64, f64, f64)> {
        // Regex: viewBox\s*=\s*["']([^"']+)["']
        let re = regex::Regex::new(r#"viewBox\s*=\s*["']([^"']+)["']"#).ok()?;
        let caps = re.captures(content)?;
        let parts: Vec<f64> = caps[1].split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        if parts.len() == 4 {
            Some((parts[0], parts[1], parts[2], parts[3]))
        } else {
            None
        }
    }

    fn extract_dimension(content: &str, attr: &str) -> Option<f64> {
        let pattern = format!(r#"<svg[^>]*\s{}="([^"]+)""#, attr);
        let re = regex::Regex::new(&pattern).ok()?;
        let caps = re.captures(content)?;
        // Strip units (px, em, etc.) and parse number
        let val = &caps[1];
        val.trim_end_matches(|c: char| c.is_alphabetic() || c == '%')
            .parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_viewbox() {
        let svg = r#"<svg viewBox="0 0 100 50"></svg>"#;
        let info = SvgInfo::parse(svg).unwrap();
        assert_eq!(info.view_box, Some((0.0, 0.0, 100.0, 50.0)));
        assert_eq!(info.aspect_ratio(), Some(2.0));
    }

    #[test]
    fn test_parse_dimensions() {
        let svg = r#"<svg width="200" height="100"></svg>"#;
        let info = SvgInfo::parse(svg).unwrap();
        assert_eq!(info.width, Some(200.0));
        assert_eq!(info.height, Some(100.0));
    }
}
```

Note: Add `regex = "1"` to Cargo.toml dependencies.

**Acceptance**: `cargo test svg` passes.

---

### T006: Add regex dependency
**File**: `Cargo.toml`
**Story**: Foundation
**Parallel**: No (needed for T005)

Add regex crate for SVG parsing:
```toml
[dependencies]
regex = "1"
```

**Acceptance**: `cargo build` succeeds.

---

**CHECKPOINT: Foundation Complete**
- All new types defined
- Module structure in place
- Error types ready
- SVG parsing implemented

---

## Phase 2: Scenario 1 - SVG Component Import

*Goal: User can import SVG files as components and instantiate them multiple times.*

### T007: Add component declaration parser rule
**File**: `src/parser/grammar.rs`
**Story**: S1 (SVG Import)
**Parallel**: No (requires T001, T002)

Add parser for component declarations after the existing `alignment_decl`:

```rust
// Component declaration: component "name" from "path"
let component_decl = just(Token::Component)
    .ignore_then(string_literal.clone())
    .then_ignore(just(Token::From))
    .then(string_literal.clone())
    .map_with(|(name, path), e| {
        let source_type = if path.node.ends_with(".svg") {
            ComponentSourceType::Svg
        } else {
            ComponentSourceType::Ail
        };
        Spanned::new(
            Statement::ComponentDecl(ComponentDecl {
                name: Spanned::new(Identifier::new(name.node), name.span.clone()),
                source_path: path,
                source_type,
            }),
            span_range(&e.span())
        )
    });
```

Add `component_decl` to the `choice()` in `statement` parser.

Add test:
```rust
#[test]
fn test_parse_component_decl() {
    let doc = parse(r#"component "person" from "icons/person.svg""#).expect("Should parse");
    match &doc.statements[0].node {
        Statement::ComponentDecl(c) => {
            assert_eq!(c.name.node.as_str(), "person");
            assert_eq!(c.source_path.node, "icons/person.svg");
            assert!(matches!(c.source_type, ComponentSourceType::Svg));
        }
        _ => panic!("Expected component decl"),
    }
}
```

**Acceptance**: Test passes, component declarations parse correctly.

---

### T008: Implement ImportResolver core [P]
**File**: `src/import/resolver.rs`
**Story**: S1 (SVG Import)
**Parallel**: Yes (with T009)

Complete the ImportResolver implementation:

```rust
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use crate::parser::ast::{Document, ComponentSourceType};
use super::svg::SvgInfo;

pub struct ResolvedComponent {
    pub source_path: PathBuf,
    pub source_type: ComponentSourceType,
    pub content: ComponentContent,
    pub exports: HashSet<String>,
}

pub enum ComponentContent {
    Svg(SvgInfo),
    Ail(Document),
}

pub struct ImportResolver {
    base_path: PathBuf,
    resolved: HashMap<PathBuf, ResolvedComponent>,
    in_progress: HashSet<PathBuf>,
}

impl ImportResolver {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            base_path,
            resolved: HashMap::new(),
            in_progress: HashSet::new(),
        }
    }

    pub fn resolve(&mut self, path: &str, declared_at: Span) -> Result<&ResolvedComponent, ImportError> {
        let full_path = self.base_path.join(path);
        let canonical = full_path.canonicalize()
            .map_err(|_| ImportError::FileNotFound {
                path: full_path.clone(),
                declared_at: declared_at.clone()
            })?;

        if self.in_progress.contains(&canonical) {
            return Err(ImportError::CircularImport {
                path: canonical,
                cycle: self.in_progress.iter().cloned().collect(),
            });
        }

        if self.resolved.contains_key(&canonical) {
            return Ok(self.resolved.get(&canonical).unwrap());
        }

        self.in_progress.insert(canonical.clone());

        let content = std::fs::read_to_string(&canonical)
            .map_err(|e| ImportError::IoError {
                path: canonical.clone(),
                message: e.to_string()
            })?;

        let resolved = self.parse_content(&canonical, &content, path)?;

        self.in_progress.remove(&canonical);
        self.resolved.insert(canonical.clone(), resolved);

        Ok(self.resolved.get(&canonical).unwrap())
    }

    fn parse_content(&mut self, path: &Path, content: &str, rel_path: &str) -> Result<ResolvedComponent, ImportError> {
        let source_type = if rel_path.ends_with(".svg") {
            ComponentSourceType::Svg
        } else {
            ComponentSourceType::Ail
        };

        match source_type {
            ComponentSourceType::Svg => {
                let svg_info = SvgInfo::parse(content)
                    .map_err(|e| ImportError::InvalidSvg {
                        path: path.to_path_buf(),
                        reason: e.0
                    })?;
                Ok(ResolvedComponent {
                    source_path: path.to_path_buf(),
                    source_type,
                    content: ComponentContent::Svg(svg_info),
                    exports: HashSet::new(),
                })
            }
            ComponentSourceType::Ail => {
                // AIL parsing handled in Phase 3
                todo!("AIL import not yet implemented")
            }
        }
    }
}
```

**Acceptance**: SVG imports resolve correctly.

---

### T009: Add SvgEmbed layout handling [P]
**File**: `src/layout/engine.rs`
**Story**: S1 (SVG Import)
**Parallel**: Yes (with T008)

Add layout handling for `SvgEmbed` shape type. In the match on `element_type`:

```rust
ElementType::Shape(ShapeType::SvgEmbed { intrinsic_width, intrinsic_height, .. }) => {
    // Get explicit size from modifiers if provided
    let explicit_width = styles.get_number("width");
    let explicit_height = styles.get_number("height");

    let (width, height) = match (explicit_width, explicit_height) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => {
            // Width specified, calculate height from aspect ratio
            let aspect = intrinsic_width.zip(*intrinsic_height)
                .map(|(iw, ih)| iw / ih)
                .unwrap_or(1.0);
            (w, w / aspect)
        }
        (None, Some(h)) => {
            // Height specified, calculate width from aspect ratio
            let aspect = intrinsic_width.zip(*intrinsic_height)
                .map(|(iw, ih)| iw / ih)
                .unwrap_or(1.0);
            (h * aspect, h)
        }
        (None, None) => {
            // Use intrinsic size or default
            (intrinsic_width.unwrap_or(100.0), intrinsic_height.unwrap_or(100.0))
        }
    };

    BoundingBox::new(x, y, width, height)
}
```

**Acceptance**: SVG embeds have correct bounds.

---

### T010: Add SvgEmbed rendering
**File**: `src/renderer/svg.rs`
**Story**: S1 (SVG Import)
**Parallel**: No (requires T009)

Add rendering for `SvgEmbed` in `render_element` function:

```rust
ElementType::Shape(ShapeType::SvgEmbed { content, intrinsic_width, intrinsic_height }) => {
    // Calculate scale factors
    let iw = intrinsic_width.unwrap_or(100.0);
    let ih = intrinsic_height.unwrap_or(100.0);
    let scale_x = element.bounds.width / iw;
    let scale_y = element.bounds.height / ih;

    // Create group with transform
    let transform = format!(
        "translate({}, {}) scale({}, {})",
        element.bounds.x, element.bounds.y,
        scale_x, scale_y
    );

    builder.start_group_with_transform(id, &classes, &transform);
    builder.add_raw_svg(&strip_svg_wrapper(content));
    builder.end_group();
}
```

Add helper functions:
```rust
impl SvgBuilder {
    pub fn start_group_with_transform(&mut self, id: Option<&str>, classes: &[String], transform: &str) {
        let id_attr = id.map(|i| format!(r#" id="{}""#, i)).unwrap_or_default();
        let class_attr = if classes.is_empty() {
            String::new()
        } else {
            format!(r#" class="{}""#, classes.join(" "))
        };

        self.elements.push(format!(
            r#"{}<g{}{} transform="{}">"#,
            self.indent_str(), id_attr, class_attr, transform
        ));
        self.indent += 1;
    }

    pub fn add_raw_svg(&mut self, content: &str) {
        for line in content.lines() {
            self.elements.push(format!("{}{}", self.indent_str(), line));
        }
    }
}

fn strip_svg_wrapper(svg: &str) -> String {
    let mut result = svg.to_string();

    // Remove XML declaration
    if let Some(idx) = result.find("?>") {
        result = result[idx + 2..].to_string();
    }

    // Remove opening <svg...> tag
    if let Some(start) = result.find("<svg") {
        if let Some(end) = result[start..].find('>') {
            result = result[start + end + 1..].to_string();
        }
    }

    // Remove closing </svg> tag
    if let Some(idx) = result.rfind("</svg>") {
        result = result[..idx].to_string();
    }

    result.trim().to_string()
}
```

**Acceptance**: SVG content renders at correct position and scale.

---

### T011: Create example SVG component file
**File**: `examples/components/person.svg` (new)
**Story**: S1 (SVG Import)
**Parallel**: Yes

Create a simple test SVG:
```xml
<svg viewBox="0 0 40 60" xmlns="http://www.w3.org/2000/svg">
  <!-- Head -->
  <circle cx="20" cy="12" r="10" fill="currentColor"/>
  <!-- Body -->
  <rect x="10" y="24" width="20" height="30" rx="3" fill="currentColor"/>
</svg>
```

**Acceptance**: File exists and is valid SVG.

---

### T012: Create integration test for SVG import
**File**: `tests/integration_tests.rs`
**Story**: S1 (SVG Import)
**Parallel**: No (requires T007-T010)

Add integration test:
```rust
#[test]
fn test_svg_component_import() {
    let input = r#"
        component "person" from "components/person.svg"
        row {
            person "alice"
            person "bob"
            person "charlie"
        }
    "#;

    // Parse, expand, layout, render
    let doc = parse(input).expect("Should parse");
    // ... expand components
    // ... run layout
    // ... render to SVG

    // Verify three person instances rendered
    assert!(svg_output.contains("alice"));
    assert!(svg_output.contains("bob"));
    assert!(svg_output.contains("charlie"));
}
```

**Acceptance**: Integration test passes.

---

**CHECKPOINT: Scenario 1 Complete**
- SVG files can be imported as components
- Multiple instances render correctly
- Layout respects aspect ratios

---

## Phase 3: Scenario 2 - AIL Component Import

*Goal: User can import AIL files as components and instantiate them with preserved structure.*

### T013: Add export declaration parser
**File**: `src/parser/grammar.rs`
**Story**: S2 (AIL Import)
**Parallel**: No

Add parser for export declarations:
```rust
// Export declaration: export id1, id2, ...
let export_decl = just(Token::Export)
    .ignore_then(
        identifier.clone()
            .separated_by(just(Token::Comma))
            .at_least(1)
            .collect::<Vec<_>>()
    )
    .map_with(|exports, e| {
        Spanned::new(
            Statement::Export(ExportDecl { exports }),
            span_range(&e.span())
        )
    });
```

Add `export_decl` to statement `choice()`.

Add test:
```rust
#[test]
fn test_parse_export_decl() {
    let doc = parse("export port1, port2, port3").expect("Should parse");
    match &doc.statements[0].node {
        Statement::Export(e) => {
            assert_eq!(e.exports.len(), 3);
            assert_eq!(e.exports[0].node.as_str(), "port1");
        }
        _ => panic!("Expected export"),
    }
}
```

**Acceptance**: Export declarations parse correctly.

---

### T014: Extend ImportResolver for AIL files [P]
**File**: `src/import/resolver.rs`
**Story**: S2 (AIL Import)
**Parallel**: Yes (with T015)

Complete the AIL branch in `parse_content`:
```rust
ComponentSourceType::Ail => {
    // Parse the AIL document
    let doc = crate::parser::parse(content)
        .map_err(|errs| ImportError::ParseError {
            path: path.to_path_buf(),
            errors: errs
        })?;

    // Extract exports
    let mut exports = HashSet::new();
    for stmt in &doc.statements {
        if let Statement::Export(e) = &stmt.node {
            for export in &e.exports {
                exports.insert(export.node.as_str().to_string());
            }
        }
    }

    // Recursively resolve any nested imports
    let old_base = std::mem::replace(&mut self.base_path, path.parent().unwrap().to_path_buf());
    for stmt in &doc.statements {
        if let Statement::ComponentDecl(c) = &stmt.node {
            self.resolve(&c.source_path.node, c.source_path.span.clone())?;
        }
    }
    self.base_path = old_base;

    Ok(ResolvedComponent {
        source_path: path.to_path_buf(),
        source_type,
        content: ComponentContent::Ail(doc),
        exports,
    })
}
```

**Acceptance**: AIL files parse and exports are extracted.

---

### T015: Create component expansion module [P]
**File**: `src/import/expansion.rs` (new)
**Story**: S2 (AIL Import)
**Parallel**: Yes (with T014)

Create expansion logic:
```rust
//! Component instance expansion

use std::collections::{HashMap, HashSet};
use crate::parser::ast::*;
use super::resolver::{ResolvedComponent, ComponentContent};
use super::svg::SvgInfo;

pub struct ExpansionContext {
    components: HashMap<String, ResolvedComponent>,
}

#[derive(Debug)]
pub enum ExpansionError {
    UnknownComponent { name: String, span: Span },
    NonExportedElement { component: String, element: String, span: Span },
}

impl ExpansionContext {
    pub fn new(components: HashMap<String, ResolvedComponent>) -> Self {
        Self { components }
    }

    pub fn expand(&self, doc: Document) -> Result<Document, ExpansionError> {
        let mut expanded = vec![];

        for stmt in doc.statements {
            match &stmt.node {
                Statement::ComponentDecl(_) | Statement::Export(_) => {
                    // Skip declarations and exports in expanded output
                }
                Statement::ComponentInstance(inst) => {
                    let expanded_stmts = self.expand_instance(inst, "")?;
                    expanded.extend(expanded_stmts);
                }
                _ => {
                    expanded.push(stmt);
                }
            }
        }

        Ok(Document { statements: expanded })
    }

    fn expand_instance(&self, inst: &ComponentInstance, prefix: &str) -> Result<Vec<Spanned<Statement>>, ExpansionError> {
        let component = self.components.get(inst.component_name.node.as_str())
            .ok_or_else(|| ExpansionError::UnknownComponent {
                name: inst.component_name.node.as_str().to_string(),
                span: inst.component_name.span.clone(),
            })?;

        let instance_prefix = if prefix.is_empty() {
            inst.instance_name.node.as_str().to_string()
        } else {
            format!("{}.{}", prefix, inst.instance_name.node.as_str())
        };

        match &component.content {
            ComponentContent::Svg(svg_info) => {
                self.expand_svg_instance(inst, svg_info, &instance_prefix)
            }
            ComponentContent::Ail(doc) => {
                self.expand_ail_instance(inst, doc, &instance_prefix)
            }
        }
    }

    fn expand_svg_instance(&self, inst: &ComponentInstance, svg: &SvgInfo, prefix: &str) -> Result<Vec<Spanned<Statement>>, ExpansionError> {
        let (iw, ih) = svg.intrinsic_size();
        let shape = ShapeDecl {
            shape_type: Spanned::new(
                ShapeType::SvgEmbed {
                    content: svg.content.clone(),
                    intrinsic_width: Some(iw),
                    intrinsic_height: Some(ih),
                },
                inst.component_name.span.clone(),
            ),
            name: Some(Spanned::new(
                Identifier::new(prefix),
                inst.instance_name.span.clone(),
            )),
            modifiers: inst.parameters.clone(),
        };

        Ok(vec![Spanned::new(Statement::Shape(shape), inst.instance_name.span.clone())])
    }

    fn expand_ail_instance(&self, inst: &ComponentInstance, doc: &Document, prefix: &str) -> Result<Vec<Spanned<Statement>>, ExpansionError> {
        let mut expanded = vec![];

        for stmt in &doc.statements {
            match &stmt.node {
                Statement::ComponentDecl(_) | Statement::Export(_) => continue,
                Statement::ComponentInstance(nested) => {
                    expanded.extend(self.expand_instance(nested, prefix)?);
                }
                _ => {
                    let prefixed = self.prefix_statement(stmt, prefix);
                    expanded.push(prefixed);
                }
            }
        }

        // Wrap in a group with the instance name
        let group = GroupDecl {
            name: Some(Spanned::new(
                Identifier::new(prefix),
                inst.instance_name.span.clone(),
            )),
            children: expanded,
            modifiers: inst.parameters.clone(),
        };

        Ok(vec![Spanned::new(Statement::Group(group), inst.instance_name.span.clone())])
    }

    fn prefix_statement(&self, stmt: &Spanned<Statement>, prefix: &str) -> Spanned<Statement> {
        // Prefix all identifiers in the statement
        // This is a deep clone with name transformation
        // Implementation details: recursively update all Identifier nodes
        todo!("Implement identifier prefixing")
    }
}
```

Update `src/import/mod.rs`:
```rust
mod expansion;
pub use expansion::{ExpansionContext, ExpansionError};
```

**Acceptance**: Expansion context compiles.

---

### T016: Implement identifier prefixing
**File**: `src/import/expansion.rs`
**Story**: S2 (AIL Import)
**Parallel**: No (requires T015)

Complete the `prefix_statement` function and add helpers:
```rust
fn prefix_statement(&self, stmt: &Spanned<Statement>, prefix: &str) -> Spanned<Statement> {
    let new_node = match &stmt.node {
        Statement::Shape(s) => Statement::Shape(self.prefix_shape(s, prefix)),
        Statement::Connection(c) => Statement::Connection(self.prefix_connection(c, prefix)),
        Statement::Layout(l) => Statement::Layout(self.prefix_layout(l, prefix)),
        Statement::Group(g) => Statement::Group(self.prefix_group(g, prefix)),
        Statement::Constraint(c) => Statement::Constraint(self.prefix_constraint(c, prefix)),
        Statement::Alignment(a) => Statement::Alignment(self.prefix_alignment(a, prefix)),
        Statement::Label(inner) => {
            let prefixed_inner = self.prefix_statement(
                &Spanned::new(*inner.clone(), stmt.span.clone()),
                prefix,
            );
            Statement::Label(Box::new(prefixed_inner.node))
        }
        // Skip these in prefixing
        Statement::ComponentDecl(_) | Statement::ComponentInstance(_) | Statement::Export(_) => {
            stmt.node.clone()
        }
    };

    Spanned::new(new_node, stmt.span.clone())
}

fn prefix_identifier(&self, id: &Spanned<Identifier>, prefix: &str) -> Spanned<Identifier> {
    Spanned::new(
        Identifier::new(format!("{}.{}", prefix, id.node.as_str())),
        id.span.clone(),
    )
}

fn prefix_shape(&self, shape: &ShapeDecl, prefix: &str) -> ShapeDecl {
    ShapeDecl {
        shape_type: shape.shape_type.clone(),
        name: shape.name.as_ref().map(|n| self.prefix_identifier(n, prefix)),
        modifiers: shape.modifiers.clone(),
    }
}

// Add similar methods for Connection, Layout, Group, Constraint, Alignment...
```

**Acceptance**: All identifiers in expanded content are prefixed.

---

### T017: Create example AIL component file
**File**: `examples/components/server-rack.ail` (new)
**Story**: S2 (AIL Import)
**Parallel**: Yes

```
// A server rack component with exported connection points
col {
    rect server1 [size: 40, fill: foreground-1]
    rect server2 [size: 40, fill: foreground-1]
    rect server3 [size: 40, fill: foreground-1]
}

// Connection points
circle input_port [size: 8, fill: accent-1]
circle output_port [size: 8, fill: accent-2]

// Expose connection points for external wiring
export input_port, output_port
```

**Acceptance**: File parses without errors.

---

### T018: Integration test for AIL import
**File**: `tests/integration_tests.rs`
**Story**: S2 (AIL Import)
**Parallel**: No (requires T013-T016)

```rust
#[test]
fn test_ail_component_import() {
    let input = r#"
        component "rack" from "components/server-rack.ail"
        row {
            rack "rack1"
            rack "rack2"
            rack "rack3"
        }
    "#;

    let doc = parse(input).expect("Should parse");
    // Expand, layout, render...

    // Verify three rack instances with prefixed elements
    assert!(svg_output.contains("rack1.server1"));
    assert!(svg_output.contains("rack2.server1"));
}
```

**Acceptance**: AIL imports render with correct namespacing.

---

**CHECKPOINT: Scenario 2 Complete**
- AIL files can be imported
- Export declarations work
- Elements are properly namespaced

---

## Phase 4: Scenario 4 & 6 - Nested Imports & Error Handling

*Goal: Handle transitive imports and provide clear error messages.*

### T019: Add circular import detection test
**File**: `tests/integration_tests.rs`
**Story**: S4 (Nested) / S6 (Errors)
**Parallel**: No

```rust
#[test]
fn test_circular_import_detected() {
    // Create temp files: a.ail imports b.ail, b.ail imports a.ail
    let temp_dir = tempdir().unwrap();

    let a_path = temp_dir.path().join("a.ail");
    let b_path = temp_dir.path().join("b.ail");

    std::fs::write(&a_path, r#"component "b" from "b.ail""#).unwrap();
    std::fs::write(&b_path, r#"component "a" from "a.ail""#).unwrap();

    let mut resolver = ImportResolver::new(temp_dir.path().to_path_buf());
    let result = resolver.resolve("a.ail", 0..0);

    assert!(matches!(result, Err(ImportError::CircularImport { .. })));
}

#[test]
fn test_missing_import_error() {
    let input = r#"component "missing" from "nonexistent.svg""#;
    let doc = parse(input).expect("Should parse");

    let mut resolver = ImportResolver::new(PathBuf::from("."));
    let result = resolver.resolve("nonexistent.svg", 0..0);

    assert!(matches!(result, Err(ImportError::FileNotFound { .. })));
}
```

**Acceptance**: Circular imports and missing files produce clear errors.

---

### T020: Add ariadne error formatting for imports
**File**: `src/error.rs`
**Story**: S6 (Errors)
**Parallel**: Yes

Extend error reporting to handle import errors with source locations:
```rust
use ariadne::{Report, ReportKind, Source, Label};
use crate::import::ImportError;

pub fn report_import_error(error: &ImportError, source: &str, filename: &str) -> String {
    let mut output = Vec::new();

    match error {
        ImportError::FileNotFound { path, declared_at } => {
            Report::build(ReportKind::Error, filename, declared_at.start)
                .with_message(format!("Cannot find file '{}'", path.display()))
                .with_label(
                    Label::new((filename, declared_at.clone()))
                        .with_message("import declared here")
                )
                .with_help(format!("Check that '{}' exists relative to '{}'", path.display(), filename))
                .finish()
                .write((filename, Source::from(source)), &mut output)
                .unwrap();
        }
        ImportError::CircularImport { path, cycle } => {
            // Format cycle path for clarity
            let cycle_str = cycle.iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(" -> ");

            Report::build(ReportKind::Error, filename, 0)
                .with_message("Circular import detected")
                .with_note(format!("Import cycle: {} -> {}", cycle_str, path.display()))
                .finish()
                .write((filename, Source::from(source)), &mut output)
                .unwrap();
        }
        // Add other cases...
    }

    String::from_utf8(output).unwrap()
}
```

**Acceptance**: Import errors show helpful context.

---

### T021: Test nested imports (3 levels deep)
**File**: `tests/integration_tests.rs`
**Story**: S4 (Nested)
**Parallel**: No

```rust
#[test]
fn test_nested_component_imports() {
    // Create: workstation.ail -> monitor.svg, keyboard.svg
    let temp_dir = tempdir().unwrap();

    // Level 1: SVG components
    std::fs::write(
        temp_dir.path().join("monitor.svg"),
        r#"<svg viewBox="0 0 80 60"><rect width="80" height="60"/></svg>"#
    ).unwrap();

    std::fs::write(
        temp_dir.path().join("keyboard.svg"),
        r#"<svg viewBox="0 0 100 30"><rect width="100" height="30"/></svg>"#
    ).unwrap();

    // Level 2: AIL component using SVGs
    std::fs::write(
        temp_dir.path().join("workstation.ail"),
        r#"
            component "monitor" from "monitor.svg"
            component "keyboard" from "keyboard.svg"
            col {
                monitor "screen"
                keyboard "keys"
            }
        "#
    ).unwrap();

    // Level 3: Main file using AIL component
    let main_input = r#"
        component "ws" from "workstation.ail"
        row {
            ws "desk1"
            ws "desk2"
        }
    "#;

    // Parse and expand
    // Verify desk1.screen and desk2.keys exist in output
}
```

**Acceptance**: Three levels of nesting work correctly.

---

**CHECKPOINT: Scenarios 4 & 6 Complete**
- Nested imports resolve correctly
- Circular imports detected
- Missing files produce clear errors

---

## Phase 5: Scenario 5 - Export & Connection Targeting

*Goal: Connections can target exported elements within component instances.*

### T022: Extend connection resolution for dot notation
**File**: `src/layout/engine.rs` (or appropriate file)
**Story**: S5 (Exports)
**Parallel**: No

When resolving connection targets, handle dot notation:
```rust
fn resolve_connection_target(&self, target: &ElementPath, exports: &HashMap<String, HashSet<String>>) -> Result<String, ConnectionError> {
    if target.is_simple() {
        // Simple reference: just the element name
        return Ok(target.leaf().as_str().to_string());
    }

    // Dot notation: instance.export
    let segments: Vec<&str> = target.segments.iter().map(|s| s.node.as_str()).collect();

    if segments.len() == 2 {
        let instance = segments[0];
        let export = segments[1];

        // Check if this is a component instance
        if let Some(component_exports) = exports.get(instance) {
            // Verify the export is declared
            if !component_exports.contains(export) {
                return Err(ConnectionError::NonExportedElement {
                    component: instance.to_string(),
                    element: export.to_string(),
                });
            }
            // Return the prefixed path
            return Ok(format!("{}.{}", instance, export));
        }
    }

    // Fallback to regular path resolution
    Ok(target.to_string())
}
```

**Acceptance**: Dot notation connections resolve to correct elements.

---

### T023: Add validation for non-exported access [P]
**File**: `src/import/expansion.rs`
**Story**: S5 (Exports)
**Parallel**: Yes (with T024)

Add validation during expansion:
```rust
pub fn validate_connections(&self, doc: &Document) -> Vec<ExpansionError> {
    let mut errors = vec![];

    for stmt in &doc.statements {
        if let Statement::Connection(conn) = &stmt.node {
            // Check 'from' and 'to' targets
            if let Err(e) = self.validate_target(&conn.from) {
                errors.push(e);
            }
            if let Err(e) = self.validate_target(&conn.to) {
                errors.push(e);
            }
        }
    }

    errors
}

fn validate_target(&self, target: &Spanned<Identifier>) -> Result<(), ExpansionError> {
    let path = target.node.as_str();
    if path.contains('.') {
        let parts: Vec<&str> = path.split('.').collect();
        if parts.len() == 2 {
            let instance = parts[0];
            let element = parts[1];

            if let Some(component) = self.get_component_for_instance(instance) {
                if !component.exports.contains(element) {
                    return Err(ExpansionError::NonExportedElement {
                        component: instance.to_string(),
                        element: element.to_string(),
                        span: target.span.clone(),
                    });
                }
            }
        }
    }
    Ok(())
}
```

**Acceptance**: Non-exported access produces clear error.

---

### T024: Create component instance parser rule [P]
**File**: `src/parser/grammar.rs`
**Story**: S3 (Parameters) / S5 (Exports)
**Parallel**: Yes (with T023)

This is tricky because component instances look like shapes. Use a two-phase approach:

Phase 1: Parse as potential instance (when identifier is not a shape keyword)
```rust
// In the statement parser, add a catch-all for unknown identifiers that might be component instances
// This will be validated later during expansion
let potential_instance = identifier.clone()
    .then(string_literal.clone())
    .then(modifier_block.clone().or_not())
    .map_with(|((name, instance), mods), e| {
        Spanned::new(
            Statement::ComponentInstance(ComponentInstance {
                component_name: name,
                instance_name: Spanned::new(Identifier::new(instance.node), instance.span),
                parameters: mods.unwrap_or_default(),
            }),
            span_range(&e.span())
        )
    });
```

Note: The shape parser has priority, so known shapes (rect, circle, etc.) parse as shapes. Unknown identifiers fall through to potential_instance.

Add test:
```rust
#[test]
fn test_parse_component_instance() {
    let doc = parse(r#"person "alice" [fill: blue]"#).expect("Should parse");
    match &doc.statements[0].node {
        Statement::ComponentInstance(inst) => {
            assert_eq!(inst.component_name.node.as_str(), "person");
            assert_eq!(inst.instance_name.node.as_str(), "alice");
            assert_eq!(inst.parameters.len(), 1);
        }
        _ => panic!("Expected component instance"),
    }
}
```

**Acceptance**: Component instances parse with parameters.

---

### T025: Integration test for export connections
**File**: `tests/integration_tests.rs`
**Story**: S5 (Exports)
**Parallel**: No

```rust
#[test]
fn test_connection_to_export() {
    let input = r#"
        component "router" from "components/router.ail"

        rect server [fill: blue]
        router "r1"

        server -> r1.wan
    "#;

    // Router AIL has: export wan, lan
    // Connection should resolve to r1.wan element
}

#[test]
fn test_connection_to_non_export_fails() {
    let input = r#"
        component "router" from "components/router.ail"

        rect server
        router "r1"

        server -> r1.internal_element
    "#;

    // Should produce clear error about non-exported element
}
```

**Acceptance**: Export connections work, non-export access errors.

---

**CHECKPOINT: Scenario 5 Complete**
- Connections target exported elements
- Dot notation resolved correctly
- Non-exported access produces errors

---

## Phase 6: Scenario 3 - Component Parameters

*Goal: Components can accept style parameters that customize rendering.*

### T026: Style propagation for SVG components
**File**: `src/import/expansion.rs`
**Story**: S3 (Parameters)
**Parallel**: No

When expanding SVG instances, apply style modifiers:
```rust
fn expand_svg_instance(&self, inst: &ComponentInstance, svg: &SvgInfo, prefix: &str) -> Result<Vec<Spanned<Statement>>, ExpansionError> {
    let (iw, ih) = svg.intrinsic_size();

    // Merge instance parameters with defaults
    let mut modifiers = inst.parameters.clone();

    // Add CSS variable support for fill/stroke
    // SVGs using currentColor will inherit these

    let shape = ShapeDecl {
        shape_type: Spanned::new(
            ShapeType::SvgEmbed {
                content: svg.content.clone(),
                intrinsic_width: Some(iw),
                intrinsic_height: Some(ih),
            },
            inst.component_name.span.clone(),
        ),
        name: Some(Spanned::new(
            Identifier::new(prefix),
            inst.instance_name.span.clone(),
        )),
        modifiers,
    };

    Ok(vec![Spanned::new(Statement::Shape(shape), inst.instance_name.span.clone())])
}
```

**Acceptance**: Style parameters apply to SVG instances.

---

### T027: Style propagation for AIL components
**File**: `src/import/expansion.rs`
**Story**: S3 (Parameters)
**Parallel**: No (requires T026)

Apply instance styles to AIL component group:
```rust
fn expand_ail_instance(&self, inst: &ComponentInstance, doc: &Document, prefix: &str) -> Result<Vec<Spanned<Statement>>, ExpansionError> {
    let mut expanded = vec![];

    for stmt in &doc.statements {
        // ... expand statements
    }

    // Wrap in a group with instance name AND style modifiers
    let group = GroupDecl {
        name: Some(Spanned::new(
            Identifier::new(prefix),
            inst.instance_name.span.clone(),
        )),
        children: expanded,
        modifiers: inst.parameters.clone(),  // Apply instance styles to group
    };

    Ok(vec![Spanned::new(Statement::Group(group), inst.instance_name.span.clone())])
}
```

**Acceptance**: AIL instances inherit styles from instance modifiers.

---

### T028: Integration test for parameters
**File**: `tests/integration_tests.rs`
**Story**: S3 (Parameters)
**Parallel**: No

```rust
#[test]
fn test_component_with_parameters() {
    let input = r#"
        component "person" from "components/person.svg"
        row {
            person "alice" [fill: blue]
            person "bob" [fill: red]
            person "charlie" [fill: green]
        }
    "#;

    // Render and verify each person has different fill color
}
```

**Acceptance**: Parameters customize each instance.

---

**CHECKPOINT: Scenario 3 Complete**
- Parameters passed to instances
- Styles applied correctly

---

## Phase 7: Polish & Documentation

### T029: Update grammar.ebnf documentation [P]
**File**: `features/001-the-grammar-and-ast-for-our-dsl/contracts/grammar.ebnf`
**Story**: Documentation
**Parallel**: Yes (with T030, T031)

Add new syntax rules:
```ebnf
(* Component declarations *)
component_decl = "component", string_literal, "from", string_literal ;

(* Component instances *)
component_instance = identifier, string_literal, [ modifier_block ] ;

(* Export declarations *)
export_decl = "export", identifier, { ",", identifier } ;

(* Updated statement rule *)
statement = shape_decl | connection | layout | group | constraint | alignment
          | component_decl | component_instance | export_decl ;
```

**Acceptance**: Grammar documentation is complete.

---

### T030: Create example: team-diagram.ail [P]
**File**: `examples/team-diagram.ail` (new)
**Story**: Documentation
**Parallel**: Yes (with T029, T031)

```
// Team diagram using person components
component "person" from "components/person.svg"

row [gap: 20] {
    person "alice" [fill: blue, label: "Alice (Lead)"]
    person "bob" [fill: green]
    person "charlie" [fill: green]
    person "diana" [fill: green]
}

// Org chart connections
alice -> bob
alice -> charlie
alice -> diana
```

**Acceptance**: Example renders correctly.

---

### T031: Create example: datacenter.ail [P]
**File**: `examples/datacenter.ail` (new)
**Story**: Documentation
**Parallel**: Yes (with T029, T030)

```
// Datacenter diagram with server rack components
component "rack" from "components/server-rack.ail"
component "router" from "components/router.ail"

row [gap: 40] {
    col {
        rack "rack1"
        rack "rack2"
    }

    router "main_router"

    col {
        rack "rack3"
        rack "rack4"
    }
}

// Network connections via exported ports
rack1.output -> main_router.lan
rack2.output -> main_router.lan
main_router.wan -> rack3.input
main_router.wan -> rack4.input
```

**Acceptance**: Complex example with exports works.

---

### T032: Snapshot tests for SVG output
**File**: `tests/integration_tests.rs`
**Story**: Quality
**Parallel**: No (requires all previous tasks)

Add insta snapshot tests:
```rust
#[test]
fn test_svg_output_simple_component() {
    let input = r#"
        component "person" from "components/person.svg"
        person "alice"
    "#;

    let svg = render_to_svg(input);
    insta::assert_snapshot!(svg);
}

#[test]
fn test_svg_output_ail_component() {
    let input = r#"
        component "rack" from "components/server-rack.ail"
        rack "r1"
    "#;

    let svg = render_to_svg(input);
    insta::assert_snapshot!(svg);
}

#[test]
fn test_svg_output_nested_components() {
    // Test with workstation containing monitor + keyboard
    let svg = render_to_svg(nested_input);
    insta::assert_snapshot!(svg);
}
```

**Acceptance**: All snapshot tests pass.

---

## Summary

### Task Count by Story

| Story | Tasks | Description |
|-------|-------|-------------|
| Foundation | 6 | Setup, types, module structure |
| S1: SVG Import | 6 | Import and render SVG components |
| S2: AIL Import | 6 | Import and render AIL components |
| S4+S6: Nested & Errors | 3 | Transitive imports, error handling |
| S5: Exports | 4 | Connection targeting via exports |
| S3: Parameters | 3 | Style customization |
| Documentation | 4 | Examples, grammar docs, snapshots |
| **Total** | **32** | |

### Parallel Execution Opportunities

**Foundation Phase** (parallel group):
- T002 (AST types) + T003 (module structure)
- T004 (error types) + T005 (SVG parser)

**Scenario 1** (parallel group):
- T008 (resolver) + T009 (layout)

**Scenario 2** (parallel group):
- T014 (AIL resolver) + T015 (expansion module)

**Scenario 5** (parallel group):
- T023 (validation) + T024 (parser rule)

**Documentation** (parallel group):
- T029 (grammar) + T030 (team example) + T031 (datacenter example)

### Dependency Graph

```
T001 (tokens)
  │
  ├─► T002 (AST) ─┬─► T007 (parser) ─► T012 (test S1)
  │               │
  └─► T003 (mod) ─┴─► T004 (errors) ─► T020 (error fmt)
                  │
                  └─► T005 (SVG) ─► T006 (regex dep)
                          │
                          └─► T008 (resolver) ─► T014 (AIL) ─► T015 (expand)
                                    │                              │
                                    └─► T009 (layout) ─────────────┴─► T016 (prefix)
                                            │                              │
                                            └─► T010 (render) ─────────────┴─► T018 (test S2)
                                                                               │
                                                    T013 (export parser) ◄─────┘
                                                            │
                                                            └─► T22-25 (S5 tasks)
                                                                    │
                                                                    └─► T26-28 (S3 tasks)
                                                                            │
                                                                            └─► T29-32 (polish)
```

### Suggested MVP Scope

**MVP (Scenario 1 only)**: Tasks T001-T012
- Import SVG files as components
- Multiple instances in layouts
- Basic styling

This provides immediate value and validates the core architecture before adding AIL imports and exports.
