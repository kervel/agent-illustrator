# Tasks: Layout and Render Pipeline

<!-- Tech Stack Validation: PASSED -->
<!-- Validated against: .specswarm/tech-stack.md -->
<!-- No prohibited technologies found -->
<!-- No new dependencies required -->

## Overview

| Attribute | Value |
|-----------|-------|
| Feature | 003 - Layout and Render Pipeline |
| Total Tasks | 28 |
| Parallel Opportunities | 12 |
| Estimated Phases | 8 |

---

## User Stories Mapping

| Story | Priority | Description | Tasks |
|-------|----------|-------------|-------|
| US1 | P1 | Render simple shapes with styles | T001-T011 |
| US2 | P2 | Layout containers arrange elements | T012-T015 |
| US3 | P3 | Position constraints modify layout | T016-T018 |
| US4 | P4 | Connections route between elements | T019-T022 |
| US5 | P5 | Full pipeline integration | T023-T028 |

---

## Phase 1: Setup & Foundation

**Goal**: Create module structure and core data types

### T001: Create layout module structure [Setup]
**File**: `src/layout/mod.rs`
**Action**: Create the layout module root with submodule declarations
```rust
pub mod config;
pub mod engine;
pub mod error;
pub mod routing;
pub mod types;

pub use config::LayoutConfig;
pub use error::LayoutError;
pub use types::*;
```
**Acceptance**: Module compiles, `cargo check` passes

---

### T002: Implement Point and BoundingBox types [P] [US1]
**File**: `src/layout/types.rs`
**Action**: Create core geometric types
```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl BoundingBox {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self { ... }
    pub fn right(&self) -> f64 { self.x + self.width }
    pub fn bottom(&self) -> f64 { self.y + self.height }
    pub fn center(&self) -> Point { ... }
    pub fn contains(&self, point: Point) -> bool { ... }
    pub fn intersects(&self, other: &BoundingBox) -> bool { ... }
    pub fn union(&self, other: &BoundingBox) -> BoundingBox { ... }
}
```
**Tests**: Unit tests for all BoundingBox methods
**Acceptance**: All geometric operations work correctly

---

### T003: Implement ResolvedStyles type [P] [US1]
**File**: `src/layout/types.rs`
**Action**: Add style resolution types
```rust
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ResolvedStyles {
    pub fill: Option<String>,
    pub stroke: Option<String>,
    pub stroke_width: Option<f64>,
    pub opacity: Option<f64>,
    pub font_size: Option<f64>,
    pub css_classes: Vec<String>,
}

impl ResolvedStyles {
    pub fn with_defaults() -> Self {
        Self {
            fill: Some("#f0f0f0".to_string()),
            stroke: Some("#333333".to_string()),
            stroke_width: Some(2.0),
            opacity: Some(1.0),
            font_size: Some(14.0),
            css_classes: vec![],
        }
    }

    pub fn from_modifiers(modifiers: &[Spanned<StyleModifier>]) -> Self { ... }
}
```
**Acceptance**: Styles resolve from AST modifiers correctly

---

### T004: Implement ElementLayout and ConnectionLayout [P] [US1]
**File**: `src/layout/types.rs`
**Action**: Add layout result types
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ElementType {
    Shape(ShapeType),
    Layout(LayoutType),
    Group,
}

#[derive(Debug, Clone)]
pub struct ElementLayout {
    pub id: Option<Identifier>,
    pub element_type: ElementType,
    pub bounds: BoundingBox,
    pub styles: ResolvedStyles,
    pub children: Vec<ElementLayout>,
    pub label: Option<LabelLayout>,
}

#[derive(Debug, Clone)]
pub struct LabelLayout {
    pub text: String,
    pub position: Point,
    pub anchor: TextAnchor,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextAnchor { Start, Middle, End }

#[derive(Debug, Clone)]
pub struct ConnectionLayout {
    pub from_id: Identifier,
    pub to_id: Identifier,
    pub direction: ConnectionDirection,
    pub path: Vec<Point>,
    pub styles: ResolvedStyles,
    pub label: Option<LabelLayout>,
}
```
**Acceptance**: All layout types compile and are clonable

---

### T005: Implement LayoutResult [US1]
**File**: `src/layout/types.rs`
**Action**: Add the main layout output type
```rust
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct LayoutResult {
    pub elements: HashMap<Identifier, ElementLayout>,
    pub root_elements: Vec<ElementLayout>,
    pub connections: Vec<ConnectionLayout>,
    pub bounds: BoundingBox,
}

impl LayoutResult {
    pub fn new() -> Self { ... }
    pub fn add_element(&mut self, element: ElementLayout) { ... }
    pub fn get_element(&self, id: &Identifier) -> Option<&ElementLayout> { ... }
    pub fn compute_bounds(&mut self) { ... }
}
```
**Acceptance**: LayoutResult can store and retrieve elements by ID

---

### T006: Implement LayoutConfig [P] [US1]
**File**: `src/layout/config.rs`
**Action**: Create configuration with defaults
```rust
#[derive(Debug, Clone)]
pub struct LayoutConfig {
    pub default_rect_size: (f64, f64),
    pub default_circle_radius: f64,
    pub element_spacing: f64,
    pub container_padding: f64,
    pub connection_spacing: f64,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            default_rect_size: (100.0, 50.0),
            default_circle_radius: 30.0,
            element_spacing: 20.0,
            container_padding: 10.0,
            connection_spacing: 10.0,
        }
    }
}
```
**Tests**: Verify default values match spec
**Acceptance**: Config provides sensible defaults

---

### T007: Implement LayoutError [P] [US1]
**File**: `src/layout/error.rs`
**Action**: Create error types with thiserror
```rust
use thiserror::Error;
use crate::parser::ast::Span;

#[derive(Debug, Error)]
pub enum LayoutError {
    #[error("undefined identifier '{name}' at position {span:?}")]
    UndefinedIdentifier {
        name: String,
        span: Span,
        suggestions: Vec<String>,
    },

    #[error("conflicting constraints: {reason}")]
    ConflictingConstraints {
        constraints: Vec<String>,
        reason: String,
    },

    #[error("circular constraint dependency: {}", cycle.join(" -> "))]
    CircularConstraint {
        cycle: Vec<String>,
    },

    #[error("invalid layout for element '{element}': {reason}")]
    InvalidLayout {
        element: String,
        reason: String,
    },
}
```
**Acceptance**: Errors display meaningful messages

---

**Checkpoint**: Phase 1 complete when `cargo check` passes with all types defined.

---

## Phase 2: Reference Validation [US1]

**Goal**: Validate identifier references before layout

### T008: Implement identifier collection [US1]
**File**: `src/layout/mod.rs`
**Action**: Walk AST to collect all defined identifiers
```rust
use std::collections::HashSet;
use crate::parser::ast::*;

fn collect_defined_identifiers(doc: &Document) -> HashSet<String> {
    let mut ids = HashSet::new();
    for stmt in &doc.statements {
        collect_ids_from_statement(&stmt.node, &mut ids);
    }
    ids
}

fn collect_ids_from_statement(stmt: &Statement, ids: &mut HashSet<String>) {
    match stmt {
        Statement::Shape(s) => {
            if let Some(name) = &s.name {
                ids.insert(name.node.0.clone());
            }
        }
        Statement::Layout(l) => {
            if let Some(name) = &l.name {
                ids.insert(name.node.0.clone());
            }
            for child in &l.children {
                collect_ids_from_statement(&child.node, ids);
            }
        }
        Statement::Group(g) => {
            if let Some(name) = &g.name {
                ids.insert(name.node.0.clone());
            }
            for child in &g.children {
                collect_ids_from_statement(&child.node, ids);
            }
        }
        _ => {}
    }
}
```
**Acceptance**: All named elements are collected

---

### T009: Implement Levenshtein distance for suggestions [P] [US1]
**File**: `src/layout/mod.rs`
**Action**: Add typo suggestion helper
```rust
fn levenshtein_distance(a: &str, b: &str) -> usize {
    // Standard DP implementation
    ...
}

fn find_similar(defined: &HashSet<String>, target: &str, max_distance: usize) -> Vec<String> {
    defined.iter()
        .filter_map(|name| {
            let dist = levenshtein_distance(name, target);
            if dist <= max_distance { Some((name.clone(), dist)) } else { None }
        })
        .sorted_by_key(|(_, d)| *d)
        .map(|(name, _)| name)
        .take(3)
        .collect()
}
```
**Acceptance**: Similar names suggested for typos

---

### T010: Implement validate_references [US1]
**File**: `src/layout/mod.rs`
**Action**: Full reference validation function
```rust
pub fn validate_references(doc: &Document) -> Result<(), LayoutError> {
    let defined = collect_defined_identifiers(doc);

    for stmt in &doc.statements {
        validate_refs_in_statement(&stmt.node, &defined, stmt.span.clone())?;
    }
    Ok(())
}

fn validate_refs_in_statement(stmt: &Statement, defined: &HashSet<String>, span: Span) -> Result<(), LayoutError> {
    match stmt {
        Statement::Connection(c) => {
            if !defined.contains(&c.from.node.0) {
                return Err(LayoutError::UndefinedIdentifier {
                    name: c.from.node.0.clone(),
                    span: c.from.span.clone(),
                    suggestions: find_similar(defined, &c.from.node.0, 2),
                });
            }
            if !defined.contains(&c.to.node.0) {
                return Err(LayoutError::UndefinedIdentifier {
                    name: c.to.node.0.clone(),
                    span: c.to.span.clone(),
                    suggestions: find_similar(defined, &c.to.node.0, 2),
                });
            }
        }
        Statement::Constraint(c) => {
            // Validate subject and anchor
            ...
        }
        Statement::Layout(l) => {
            for child in &l.children {
                validate_refs_in_statement(&child.node, defined, child.span.clone())?;
            }
        }
        Statement::Group(g) => {
            for child in &g.children {
                validate_refs_in_statement(&child.node, defined, child.span.clone())?;
            }
        }
        _ => {}
    }
    Ok(())
}
```
**Tests**:
- Valid doc passes validation
- `a -> undefined` returns error with span
- Typo `servr` suggests `server`
**Acceptance**: Invalid references caught with helpful suggestions

---

### T011: Add validation tests [US1]
**File**: `tests/layout_tests.rs`
**Action**: Create layout test file with validation tests
```rust
use agent_illustrator::{parse, layout};

#[test]
fn test_valid_references_pass() {
    let doc = parse("rect a rect b a -> b").unwrap();
    assert!(layout::validate_references(&doc).is_ok());
}

#[test]
fn test_undefined_reference_error() {
    let doc = parse("rect a a -> undefined").unwrap();
    let err = layout::validate_references(&doc).unwrap_err();
    match err {
        layout::LayoutError::UndefinedIdentifier { name, .. } => {
            assert_eq!(name, "undefined");
        }
        _ => panic!("Expected UndefinedIdentifier"),
    }
}

#[test]
fn test_typo_suggestions() {
    let doc = parse("rect server server -> servr").unwrap();
    let err = layout::validate_references(&doc).unwrap_err();
    match err {
        layout::LayoutError::UndefinedIdentifier { suggestions, .. } => {
            assert!(suggestions.contains(&"server".to_string()));
        }
        _ => panic!("Expected UndefinedIdentifier"),
    }
}
```
**Acceptance**: All validation tests pass

---

**Checkpoint**: US1 foundation complete - validation working

---

## Phase 3: Basic Layout Engine [US1 continued]

**Goal**: Compute positions for shapes

### T012: Implement shape sizing [US1]
**File**: `src/layout/engine.rs`
**Action**: Compute default sizes for shapes
```rust
use crate::parser::ast::*;
use super::{LayoutConfig, ElementLayout, BoundingBox, ResolvedStyles, ElementType, LabelLayout, Point, TextAnchor};

pub fn compute_shape_size(shape: &ShapeDecl, config: &LayoutConfig) -> (f64, f64) {
    match &shape.shape_type.node {
        ShapeType::Rectangle => config.default_rect_size,
        ShapeType::Circle => {
            let d = config.default_circle_radius * 2.0;
            (d, d)
        }
        ShapeType::Ellipse => config.default_rect_size,
        ShapeType::Polygon => config.default_rect_size,
        ShapeType::Icon { .. } => config.default_rect_size,
        ShapeType::Line => (config.default_rect_size.0, 2.0),
    }
}

pub fn layout_shape(shape: &ShapeDecl, position: Point, config: &LayoutConfig) -> ElementLayout {
    let (width, height) = compute_shape_size(shape, config);
    let styles = ResolvedStyles::from_modifiers(&shape.modifiers);

    let label = extract_label(&shape.modifiers).map(|text| LabelLayout {
        text,
        position: Point { x: position.x + width / 2.0, y: position.y + height / 2.0 },
        anchor: TextAnchor::Middle,
    });

    ElementLayout {
        id: shape.name.as_ref().map(|n| n.node.clone()),
        element_type: ElementType::Shape(shape.shape_type.node.clone()),
        bounds: BoundingBox::new(position.x, position.y, width, height),
        styles,
        children: vec![],
        label,
    }
}

fn extract_label(modifiers: &[Spanned<StyleModifier>]) -> Option<String> {
    modifiers.iter().find_map(|m| {
        if matches!(m.node.key.node, StyleKey::Label) {
            match &m.node.value.node {
                StyleValue::String(s) => Some(s.clone()),
                _ => None,
            }
        } else {
            None
        }
    })
}
```
**Acceptance**: Shapes get appropriate default sizes

---

### T013: Implement row/column layout [US2]
**File**: `src/layout/engine.rs`
**Action**: Layout children horizontally or vertically
```rust
pub fn layout_row(children: &[Spanned<Statement>], position: Point, config: &LayoutConfig) -> (Vec<ElementLayout>, BoundingBox) {
    let mut layouts = vec![];
    let mut x = position.x + config.container_padding;
    let mut max_height = 0.0f64;

    for child in children {
        let child_layout = layout_statement(&child.node, Point { x, y: position.y + config.container_padding }, config);
        x += child_layout.bounds.width + config.element_spacing;
        max_height = max_height.max(child_layout.bounds.height);
        layouts.push(child_layout);
    }

    let total_width = x - position.x - config.element_spacing + config.container_padding;
    let total_height = max_height + 2.0 * config.container_padding;

    (layouts, BoundingBox::new(position.x, position.y, total_width, total_height))
}

pub fn layout_column(children: &[Spanned<Statement>], position: Point, config: &LayoutConfig) -> (Vec<ElementLayout>, BoundingBox) {
    let mut layouts = vec![];
    let mut y = position.y + config.container_padding;
    let mut max_width = 0.0f64;

    for child in children {
        let child_layout = layout_statement(&child.node, Point { x: position.x + config.container_padding, y }, config);
        y += child_layout.bounds.height + config.element_spacing;
        max_width = max_width.max(child_layout.bounds.width);
        layouts.push(child_layout);
    }

    let total_width = max_width + 2.0 * config.container_padding;
    let total_height = y - position.y - config.element_spacing + config.container_padding;

    (layouts, BoundingBox::new(position.x, position.y, total_width, total_height))
}
```
**Tests**: Row places children horizontally, column vertically
**Acceptance**: Layout containers arrange children correctly

---

### T014: Implement grid and stack layout [P] [US2]
**File**: `src/layout/engine.rs`
**Action**: Add grid and stack layout algorithms
```rust
pub fn layout_grid(children: &[Spanned<Statement>], position: Point, config: &LayoutConfig) -> (Vec<ElementLayout>, BoundingBox) {
    let n = children.len();
    let cols = (n as f64).sqrt().ceil() as usize;
    let rows = (n + cols - 1) / cols;

    let mut layouts = vec![];
    let mut max_cell_width = 0.0f64;
    let mut max_cell_height = 0.0f64;

    // First pass: compute max cell size
    for child in children {
        let temp = layout_statement(&child.node, Point { x: 0.0, y: 0.0 }, config);
        max_cell_width = max_cell_width.max(temp.bounds.width);
        max_cell_height = max_cell_height.max(temp.bounds.height);
    }

    // Second pass: place in grid
    for (i, child) in children.iter().enumerate() {
        let row = i / cols;
        let col = i % cols;
        let x = position.x + config.container_padding + col as f64 * (max_cell_width + config.element_spacing);
        let y = position.y + config.container_padding + row as f64 * (max_cell_height + config.element_spacing);
        layouts.push(layout_statement(&child.node, Point { x, y }, config));
    }

    let total_width = cols as f64 * (max_cell_width + config.element_spacing) - config.element_spacing + 2.0 * config.container_padding;
    let total_height = rows as f64 * (max_cell_height + config.element_spacing) - config.element_spacing + 2.0 * config.container_padding;

    (layouts, BoundingBox::new(position.x, position.y, total_width, total_height))
}

pub fn layout_stack(children: &[Spanned<Statement>], position: Point, config: &LayoutConfig) -> (Vec<ElementLayout>, BoundingBox) {
    let mut layouts = vec![];
    let mut max_width = 0.0f64;
    let mut max_height = 0.0f64;

    for child in children {
        let child_layout = layout_statement(&child.node, Point { x: position.x + config.container_padding, y: position.y + config.container_padding }, config);
        max_width = max_width.max(child_layout.bounds.width);
        max_height = max_height.max(child_layout.bounds.height);
        layouts.push(child_layout);
    }

    (layouts, BoundingBox::new(position.x, position.y, max_width + 2.0 * config.container_padding, max_height + 2.0 * config.container_padding))
}
```
**Acceptance**: Grid distributes in square-ish pattern, stack overlays

---

### T015: Implement compute_layout entry point [US2]
**File**: `src/layout/engine.rs`
**Action**: Main layout computation function
```rust
use super::{LayoutResult, LayoutError};

pub fn compute(doc: &Document, config: &LayoutConfig) -> Result<LayoutResult, LayoutError> {
    // First validate references
    super::validate_references(doc)?;

    let mut result = LayoutResult::new();
    let mut position = Point { x: 0.0, y: 0.0 };

    for stmt in &doc.statements {
        match &stmt.node {
            Statement::Connection(_) | Statement::Constraint(_) => continue, // Handle later
            _ => {
                let element = layout_statement(&stmt.node, position, config);
                position.y += element.bounds.height + config.element_spacing;
                result.add_element(element);
            }
        }
    }

    result.compute_bounds();
    Ok(result)
}

fn layout_statement(stmt: &Statement, position: Point, config: &LayoutConfig) -> ElementLayout {
    match stmt {
        Statement::Shape(s) => layout_shape(s, position, config),
        Statement::Layout(l) => {
            let (children, bounds) = match l.layout_type.node {
                LayoutType::Row => layout_row(&l.children, position, config),
                LayoutType::Column => layout_column(&l.children, position, config),
                LayoutType::Grid => layout_grid(&l.children, position, config),
                LayoutType::Stack => layout_stack(&l.children, position, config),
            };
            ElementLayout {
                id: l.name.as_ref().map(|n| n.node.clone()),
                element_type: ElementType::Layout(l.layout_type.node.clone()),
                bounds,
                styles: ResolvedStyles::from_modifiers(&l.modifiers),
                children,
                label: None,
            }
        }
        Statement::Group(g) => {
            let (children, bounds) = layout_column(&g.children, position, config);
            ElementLayout {
                id: g.name.as_ref().map(|n| n.node.clone()),
                element_type: ElementType::Group,
                bounds,
                styles: ResolvedStyles::from_modifiers(&g.modifiers),
                children,
                label: None,
            }
        }
        _ => unreachable!("Connections and constraints handled separately"),
    }
}
```
**Tests**: End-to-end layout computation
**Acceptance**: `row { rect a rect b }` produces valid LayoutResult

---

**Checkpoint**: US2 complete - layout containers working

---

## Phase 4: Constraint Resolution [US3]

**Goal**: Apply position constraints

### T016: Build constraint graph [US3]
**File**: `src/layout/engine.rs`
**Action**: Extract and organize constraints
```rust
use std::collections::{HashMap, HashSet};

struct ConstraintGraph {
    // subject -> Vec<(relation, anchor)>
    constraints: HashMap<String, Vec<(PositionRelation, String)>>,
}

impl ConstraintGraph {
    fn from_document(doc: &Document) -> Self {
        let mut constraints = HashMap::new();
        for stmt in &doc.statements {
            if let Statement::Constraint(c) = &stmt.node {
                constraints
                    .entry(c.subject.node.0.clone())
                    .or_insert_with(Vec::new)
                    .push((c.relation.node.clone(), c.anchor.node.0.clone()));
            }
        }
        Self { constraints }
    }

    fn detect_cycle(&self) -> Option<Vec<String>> {
        // DFS cycle detection
        ...
    }

    fn topological_order(&self) -> Result<Vec<String>, LayoutError> {
        if let Some(cycle) = self.detect_cycle() {
            return Err(LayoutError::CircularConstraint { cycle });
        }
        // Kahn's algorithm
        ...
    }
}
```
**Acceptance**: Constraints extracted and cycles detected

---

### T017: Implement constraint application [US3]
**File**: `src/layout/engine.rs`
**Action**: Apply constraints to modify positions
```rust
pub fn resolve_constraints(result: &mut LayoutResult, doc: &Document) -> Result<(), LayoutError> {
    let graph = ConstraintGraph::from_document(doc);
    let order = graph.topological_order()?;

    for subject_id in order {
        if let Some(constraints) = graph.constraints.get(&subject_id) {
            for (relation, anchor_id) in constraints {
                apply_constraint(result, &subject_id, relation, &anchor_id)?;
            }
        }
    }

    Ok(())
}

fn apply_constraint(
    result: &mut LayoutResult,
    subject_id: &str,
    relation: &PositionRelation,
    anchor_id: &str,
) -> Result<(), LayoutError> {
    let anchor_bounds = result.get_element_by_name(anchor_id)
        .ok_or_else(|| LayoutError::UndefinedIdentifier {
            name: anchor_id.to_string(),
            span: 0..0,
            suggestions: vec![],
        })?
        .bounds;

    let subject = result.get_element_mut_by_name(subject_id)
        .ok_or_else(|| LayoutError::UndefinedIdentifier {
            name: subject_id.to_string(),
            span: 0..0,
            suggestions: vec![],
        })?;

    let spacing = 20.0; // Could come from config

    match relation {
        PositionRelation::RightOf => {
            subject.bounds.x = anchor_bounds.right() + spacing;
            subject.bounds.y = anchor_bounds.y;
        }
        PositionRelation::LeftOf => {
            subject.bounds.x = anchor_bounds.x - subject.bounds.width - spacing;
            subject.bounds.y = anchor_bounds.y;
        }
        PositionRelation::Below => {
            subject.bounds.x = anchor_bounds.x;
            subject.bounds.y = anchor_bounds.bottom() + spacing;
        }
        PositionRelation::Above => {
            subject.bounds.x = anchor_bounds.x;
            subject.bounds.y = anchor_bounds.y - subject.bounds.height - spacing;
        }
        PositionRelation::Inside => {
            subject.bounds.x = anchor_bounds.x + (anchor_bounds.width - subject.bounds.width) / 2.0;
            subject.bounds.y = anchor_bounds.y + (anchor_bounds.height - subject.bounds.height) / 2.0;
        }
    }

    Ok(())
}
```
**Acceptance**: Constraints modify element positions correctly

---

### T018: Implement conflict detection [US3]
**File**: `src/layout/engine.rs`
**Action**: Detect unsatisfiable constraint combinations
```rust
fn detect_conflicts(result: &LayoutResult, doc: &Document) -> Result<(), LayoutError> {
    // Check for spatial conflicts after constraint resolution
    let mut conflicts = vec![];

    for stmt in &doc.statements {
        if let Statement::Constraint(c1) = &stmt.node {
            for stmt2 in &doc.statements {
                if let Statement::Constraint(c2) = &stmt2.node {
                    if c1.subject.node.0 == c2.subject.node.0 &&
                       are_conflicting(&c1.relation.node, &c2.relation.node) {
                        conflicts.push(format!(
                            "{} {} {} conflicts with {} {} {}",
                            c1.subject.node.0, relation_name(&c1.relation.node), c1.anchor.node.0,
                            c2.subject.node.0, relation_name(&c2.relation.node), c2.anchor.node.0
                        ));
                    }
                }
            }
        }
    }

    if !conflicts.is_empty() {
        return Err(LayoutError::ConflictingConstraints {
            constraints: conflicts,
            reason: "Multiple position constraints cannot be satisfied simultaneously".to_string(),
        });
    }

    Ok(())
}

fn are_conflicting(a: &PositionRelation, b: &PositionRelation) -> bool {
    matches!(
        (a, b),
        (PositionRelation::RightOf, PositionRelation::LeftOf) |
        (PositionRelation::LeftOf, PositionRelation::RightOf) |
        (PositionRelation::Above, PositionRelation::Below) |
        (PositionRelation::Below, PositionRelation::Above)
    )
}
```
**Tests**: Conflicting constraints produce clear error
**Acceptance**: `place a right-of b` + `place a left-of b` fails with error

---

**Checkpoint**: US3 complete - constraints working

---

## Phase 5: Connection Routing [US4]

**Goal**: Route connections between elements

### T019: Implement edge attachment points [US4]
**File**: `src/layout/routing.rs`
**Action**: Compute where connections attach to shapes
```rust
use super::{Point, BoundingBox};

#[derive(Debug, Clone, Copy)]
pub enum Edge { Top, Bottom, Left, Right }

pub fn attachment_point(bounds: &BoundingBox, edge: Edge) -> Point {
    match edge {
        Edge::Top => Point { x: bounds.x + bounds.width / 2.0, y: bounds.y },
        Edge::Bottom => Point { x: bounds.x + bounds.width / 2.0, y: bounds.bottom() },
        Edge::Left => Point { x: bounds.x, y: bounds.y + bounds.height / 2.0 },
        Edge::Right => Point { x: bounds.right(), y: bounds.y + bounds.height / 2.0 },
    }
}

pub fn best_edges(from: &BoundingBox, to: &BoundingBox) -> (Edge, Edge) {
    let dx = to.center().x - from.center().x;
    let dy = to.center().y - from.center().y;

    if dx.abs() > dy.abs() {
        // Primarily horizontal
        if dx > 0.0 {
            (Edge::Right, Edge::Left)
        } else {
            (Edge::Left, Edge::Right)
        }
    } else {
        // Primarily vertical
        if dy > 0.0 {
            (Edge::Bottom, Edge::Top)
        } else {
            (Edge::Top, Edge::Bottom)
        }
    }
}
```
**Acceptance**: Edges computed based on relative positions

---

### T020: Implement orthogonal path routing [US4]
**File**: `src/layout/routing.rs`
**Action**: Create L-shaped or direct paths
```rust
pub fn route_orthogonal(from: Point, to: Point) -> Vec<Point> {
    if (from.x - to.x).abs() < 1.0 || (from.y - to.y).abs() < 1.0 {
        // Direct line (horizontal or vertical)
        vec![from, to]
    } else {
        // L-shaped route: horizontal first, then vertical
        let mid = Point { x: to.x, y: from.y };
        vec![from, mid, to]
    }
}

pub fn route_connection(from_bounds: &BoundingBox, to_bounds: &BoundingBox) -> Vec<Point> {
    let (from_edge, to_edge) = best_edges(from_bounds, to_bounds);
    let start = attachment_point(from_bounds, from_edge);
    let end = attachment_point(to_bounds, to_edge);
    route_orthogonal(start, end)
}
```
**Acceptance**: Connections have clean orthogonal paths

---

### T021: Implement route_connections for LayoutResult [US4]
**File**: `src/layout/routing.rs`
**Action**: Route all connections in document
```rust
use crate::parser::ast::*;
use super::{LayoutResult, ConnectionLayout, ResolvedStyles, LayoutError, LabelLayout, TextAnchor};

pub fn route_connections(result: &mut LayoutResult, doc: &Document) -> Result<(), LayoutError> {
    for stmt in &doc.statements {
        if let Statement::Connection(conn) = &stmt.node {
            let from_element = result.get_element_by_name(&conn.from.node.0)
                .ok_or_else(|| LayoutError::UndefinedIdentifier {
                    name: conn.from.node.0.clone(),
                    span: conn.from.span.clone(),
                    suggestions: vec![],
                })?;
            let to_element = result.get_element_by_name(&conn.to.node.0)
                .ok_or_else(|| LayoutError::UndefinedIdentifier {
                    name: conn.to.node.0.clone(),
                    span: conn.to.span.clone(),
                    suggestions: vec![],
                })?;

            let path = route_connection(&from_element.bounds, &to_element.bounds);
            let styles = ResolvedStyles::from_modifiers(&conn.modifiers);

            let label = extract_connection_label(&conn.modifiers, &path);

            result.connections.push(ConnectionLayout {
                from_id: conn.from.node.clone(),
                to_id: conn.to.node.clone(),
                direction: conn.direction,
                path,
                styles,
                label,
            });
        }
    }

    Ok(())
}

fn extract_connection_label(modifiers: &[Spanned<StyleModifier>], path: &[Point]) -> Option<LabelLayout> {
    // Find label modifier and position at midpoint of path
    ...
}
```
**Acceptance**: All connections routed with valid paths

---

### T022: Add routing tests [US4]
**File**: `tests/layout_tests.rs`
**Action**: Add routing test cases
```rust
#[test]
fn test_horizontal_connection() {
    let doc = parse("rect a rect b [right-of: a] a -> b").unwrap();
    // ... verify connection path is horizontal
}

#[test]
fn test_vertical_connection() {
    let doc = parse("rect a rect b [below: a] a -> b").unwrap();
    // ... verify connection path is vertical
}

#[test]
fn test_diagonal_connection_uses_l_route() {
    // Elements not aligned should get L-shaped path
}
```
**Acceptance**: Connection routing tests pass

---

**Checkpoint**: US4 complete - connections routing

---

## Phase 6: SVG Renderer [US1, US5]

**Goal**: Generate SVG from LayoutResult

### T023: Create renderer module structure [P] [US5]
**File**: `src/renderer/mod.rs`
**Action**: Create renderer module
```rust
pub mod config;
pub mod svg;

pub use config::SvgConfig;
pub use svg::render_svg;
```

**File**: `src/renderer/config.rs`
```rust
#[derive(Debug, Clone)]
pub struct SvgConfig {
    pub viewbox_padding: f64,
    pub standalone: bool,
    pub pretty_print: bool,
    pub class_prefix: Option<String>,
}

impl Default for SvgConfig {
    fn default() -> Self {
        Self {
            viewbox_padding: 20.0,
            standalone: true,
            pretty_print: true,
            class_prefix: Some("ai-".to_string()),
        }
    }
}
```
**Acceptance**: Renderer module compiles

---

### T024: Implement SvgBuilder [US5]
**File**: `src/renderer/svg.rs`
**Action**: Create SVG building utilities
```rust
use super::SvgConfig;

pub struct SvgBuilder {
    config: SvgConfig,
    defs: Vec<String>,
    elements: Vec<String>,
    connections: Vec<String>,
    indent: usize,
}

impl SvgBuilder {
    pub fn new(config: SvgConfig) -> Self {
        Self {
            config,
            defs: vec![],
            elements: vec![],
            connections: vec![],
            indent: 0,
        }
    }

    fn prefix(&self) -> String {
        self.config.class_prefix.clone().unwrap_or_default()
    }

    fn indent_str(&self) -> String {
        if self.config.pretty_print {
            "  ".repeat(self.indent)
        } else {
            String::new()
        }
    }

    fn newline(&self) -> &str {
        if self.config.pretty_print { "\n" } else { "" }
    }

    pub fn add_arrow_marker(&mut self) {
        let prefix = self.prefix();
        self.defs.push(format!(
            r#"<marker id="{prefix}arrow" viewBox="0 0 10 10" refX="10" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
      <path d="M0,0 L10,5 L0,10 Z" class="{prefix}arrow-head"/>
    </marker>"#
        ));
    }

    pub fn add_rect(&mut self, id: Option<&str>, x: f64, y: f64, w: f64, h: f64, classes: &[String], styles: &str) {
        let prefix = self.prefix();
        let id_attr = id.map(|i| format!(r#" id="{}""#, i)).unwrap_or_default();
        let class_list = std::iter::once(format!("{}shape", prefix))
            .chain(std::iter::once(format!("{}rect", prefix)))
            .chain(classes.iter().cloned())
            .collect::<Vec<_>>()
            .join(" ");

        self.elements.push(format!(
            r#"{}<rect{} class="{}" x="{}" y="{}" width="{}" height="{}"{}/>"#,
            self.indent_str(), id_attr, class_list, x, y, w, h, styles
        ));
    }

    // Similar methods for circle, ellipse, polygon, text, path...

    pub fn build(self, viewbox: (f64, f64, f64, f64)) -> String {
        // Assemble final SVG
        ...
    }
}
```
**Acceptance**: SvgBuilder can create SVG elements

---

### T025: Implement shape renderers [US5]
**File**: `src/renderer/svg.rs`
**Action**: Render each shape type
```rust
use crate::layout::{ElementLayout, ElementType, ResolvedStyles};
use crate::parser::ast::ShapeType;

pub fn render_element(element: &ElementLayout, builder: &mut SvgBuilder) {
    let id = element.id.as_ref().map(|i| i.0.as_str());
    let styles = format_styles(&element.styles);
    let classes = element.styles.css_classes.clone();

    match &element.element_type {
        ElementType::Shape(ShapeType::Rectangle) => {
            builder.add_rect(
                id,
                element.bounds.x,
                element.bounds.y,
                element.bounds.width,
                element.bounds.height,
                &classes,
                &styles,
            );
        }
        ElementType::Shape(ShapeType::Circle) => {
            let r = element.bounds.width.min(element.bounds.height) / 2.0;
            builder.add_circle(
                id,
                element.bounds.x + r,
                element.bounds.y + r,
                r,
                &classes,
                &styles,
            );
        }
        ElementType::Shape(ShapeType::Ellipse) => {
            builder.add_ellipse(
                id,
                element.bounds.x + element.bounds.width / 2.0,
                element.bounds.y + element.bounds.height / 2.0,
                element.bounds.width / 2.0,
                element.bounds.height / 2.0,
                &classes,
                &styles,
            );
        }
        // ... other shapes
        ElementType::Layout(_) | ElementType::Group => {
            // Render container (optional rect) then children
            for child in &element.children {
                render_element(child, builder);
            }
        }
    }

    // Render label if present
    if let Some(label) = &element.label {
        builder.add_text(&label.text, label.position.x, label.position.y, &label.anchor);
    }
}

fn format_styles(styles: &ResolvedStyles) -> String {
    let mut parts = vec![];
    if let Some(fill) = &styles.fill {
        parts.push(format!(r#" fill="{}""#, fill));
    }
    if let Some(stroke) = &styles.stroke {
        parts.push(format!(r#" stroke="{}""#, stroke));
    }
    if let Some(sw) = styles.stroke_width {
        parts.push(format!(r#" stroke-width="{}""#, sw));
    }
    if let Some(op) = styles.opacity {
        if op < 1.0 {
            parts.push(format!(r#" opacity="{}""#, op));
        }
    }
    parts.join("")
}
```
**Acceptance**: All shape types render to SVG

---

### T026: Implement connection renderer [US5]
**File**: `src/renderer/svg.rs`
**Action**: Render connections with arrows
```rust
use crate::layout::ConnectionLayout;
use crate::parser::ast::ConnectionDirection;

pub fn render_connection(conn: &ConnectionLayout, builder: &mut SvgBuilder) {
    let prefix = builder.prefix();

    // Build path data
    let path_data = conn.path.iter()
        .enumerate()
        .map(|(i, p)| {
            if i == 0 {
                format!("M{},{}", p.x, p.y)
            } else {
                format!(" L{},{}", p.x, p.y)
            }
        })
        .collect::<String>();

    // Determine markers
    let (marker_start, marker_end) = match conn.direction {
        ConnectionDirection::Forward => (None, Some(format!("url(#{}arrow)", prefix))),
        ConnectionDirection::Backward => (Some(format!("url(#{}arrow)", prefix)), None),
        ConnectionDirection::Bidirectional => (
            Some(format!("url(#{}arrow)", prefix)),
            Some(format!("url(#{}arrow)", prefix)),
        ),
        ConnectionDirection::Undirected => (None, None),
    };

    let direction_class = match conn.direction {
        ConnectionDirection::Forward => "forward",
        ConnectionDirection::Backward => "backward",
        ConnectionDirection::Bidirectional => "bidirectional",
        ConnectionDirection::Undirected => "undirected",
    };

    builder.add_path(
        &path_data,
        &[format!("{}connection", prefix), format!("{}connection-{}", prefix, direction_class)],
        &format_styles(&conn.styles),
        marker_start.as_deref(),
        marker_end.as_deref(),
    );

    // Render label if present
    if let Some(label) = &conn.label {
        builder.add_text(&label.text, label.position.x, label.position.y, &label.anchor);
    }
}
```
**Acceptance**: Connections render with appropriate arrows

---

### T027: Implement render_svg entry point [US5]
**File**: `src/renderer/svg.rs`
**Action**: Main rendering function
```rust
use crate::layout::LayoutResult;
use super::SvgConfig;

pub fn render_svg(result: &LayoutResult, config: &SvgConfig) -> String {
    let mut builder = SvgBuilder::new(config.clone());

    // Add defs (arrow markers)
    builder.add_arrow_marker();

    // Render all elements
    for element in &result.root_elements {
        render_element(element, &mut builder);
    }

    // Render all connections
    for conn in &result.connections {
        render_connection(conn, &mut builder);
    }

    // Compute viewbox with padding
    let p = config.viewbox_padding;
    let viewbox = (
        result.bounds.x - p,
        result.bounds.y - p,
        result.bounds.width + 2.0 * p,
        result.bounds.height + 2.0 * p,
    );

    builder.build(viewbox)
}
```
**Acceptance**: render_svg produces valid SVG string

---

**Checkpoint**: US5 renderer complete

---

## Phase 7: Pipeline Integration [US5]

**Goal**: Single entry point API

### T028: Implement render() and render_with_config() [US5]
**File**: `src/lib.rs`
**Action**: Add public render API
```rust
pub mod error;
pub mod layout;
pub mod parser;
pub mod renderer;

pub use error::{ParseError, RenderError};
pub use layout::{LayoutConfig, LayoutError, LayoutResult};
pub use parser::{parse, Document};
pub use renderer::{render_svg, SvgConfig};

/// Configuration for the complete render pipeline
#[derive(Debug, Clone, Default)]
pub struct RenderConfig {
    pub layout: LayoutConfig,
    pub svg: SvgConfig,
}

/// Render DSL source to SVG with default configuration
pub fn render(source: &str) -> Result<String, RenderError> {
    render_with_config(source, RenderConfig::default())
}

/// Render DSL source to SVG with custom configuration
pub fn render_with_config(source: &str, config: RenderConfig) -> Result<String, RenderError> {
    // Parse
    let doc = parse(source).map_err(|errs| RenderError::Parse(errs))?;

    // Layout
    let mut result = layout::compute(&doc, &config.layout)
        .map_err(RenderError::Layout)?;

    // Resolve constraints
    layout::resolve_constraints(&mut result, &doc)
        .map_err(RenderError::Layout)?;

    // Route connections
    layout::route_connections(&mut result, &doc)
        .map_err(RenderError::Layout)?;

    // Render
    let svg = render_svg(&result, &config.svg);

    Ok(svg)
}
```

**File**: `src/error.rs`
```rust
use thiserror::Error;
use crate::layout::LayoutError;
use crate::parser::ParseError;

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("parse error: {0:?}")]
    Parse(Vec<ParseError>),

    #[error("layout error: {0}")]
    Layout(#[from] LayoutError),

    #[error("render error: {0}")]
    Render(String),
}
```
**Acceptance**: Single function call produces SVG

---

**Checkpoint**: Full pipeline working

---

## Phase 8: Polish & Integration Tests [US5]

### T029: Add snapshot tests [P] [US5]
**File**: `tests/integration_tests.rs`
**Action**: Add render snapshot tests
```rust
use agent_illustrator::render;
use insta::assert_snapshot;

#[test]
fn test_render_simple_shapes() {
    let svg = render(r#"
        rect server [fill: #3B82F6, label: "Server"]
        rect db [fill: #10B981, label: "Database"]
    "#).unwrap();
    assert_snapshot!(svg);
}

#[test]
fn test_render_with_connection() {
    let svg = render(r#"
        rect a [label: "A"]
        rect b [label: "B"]
        a -> b
    "#).unwrap();
    assert_snapshot!(svg);
}

#[test]
fn test_render_row_layout() {
    let svg = render(r#"
        row {
            rect a [label: "A"]
            rect b [label: "B"]
            rect c [label: "C"]
        }
    "#).unwrap();
    assert_snapshot!(svg);
}

#[test]
fn test_render_nested_layout() {
    let svg = render(r#"
        column {
            row {
                rect a [label: "A"]
                rect b [label: "B"]
            }
            row {
                rect c [label: "C"]
                rect d [label: "D"]
            }
        }
        a -> c
        b -> d
    "#).unwrap();
    assert_snapshot!(svg);
}
```
**Acceptance**: Snapshots created and pass

---

### T030: Update main.rs with CLI usage [P] [US5]
**File**: `src/main.rs`
**Action**: Add basic CLI
```rust
use std::io::{self, Read};
use agent_illustrator::render;

fn main() {
    let mut input = String::new();

    if let Err(e) = io::stdin().read_to_string(&mut input) {
        eprintln!("Error reading input: {}", e);
        std::process::exit(1);
    }

    match render(&input) {
        Ok(svg) => println!("{}", svg),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
```
**Acceptance**: `echo "rect a" | cargo run` produces SVG

---

### T031: Add error rendering with ariadne [US5]
**File**: `src/error.rs`
**Action**: Pretty-print errors with source context
```rust
use ariadne::{Report, ReportKind, Source, Label, Color};

impl RenderError {
    pub fn print_report(&self, source: &str) {
        match self {
            RenderError::Parse(errors) => {
                for err in errors {
                    // Use ariadne to render parse errors
                    ...
                }
            }
            RenderError::Layout(LayoutError::UndefinedIdentifier { name, span, suggestions }) => {
                let mut report = Report::build(ReportKind::Error, (), span.start)
                    .with_message(format!("undefined identifier '{}'", name))
                    .with_label(
                        Label::new(span.clone())
                            .with_message("not defined")
                            .with_color(Color::Red)
                    );

                if !suggestions.is_empty() {
                    report = report.with_help(format!("did you mean: {}?", suggestions.join(", ")));
                }

                report.finish().print(Source::from(source)).unwrap();
            }
            // ... other error types
        }
    }
}
```
**Acceptance**: Errors display with source snippets and suggestions

---

### T032: Final validation and cleanup [US5]
**File**: Multiple
**Action**:
- Run `cargo fmt`
- Run `cargo clippy` and fix warnings
- Verify all tests pass
- Update lib.rs exports
- Add rustdoc comments to public APIs
**Acceptance**: `cargo clippy -- -D warnings` passes, tests green

---

**Checkpoint**: Feature complete

---

## Dependency Graph

```
T001 (module structure)
  ├── T002, T003, T004 [P] (types)
  │   └── T005 (LayoutResult)
  │       └── T006, T007 [P] (config, error)
  │           └── T008 (collect ids)
  │               └── T009 [P] (Levenshtein)
  │               └── T010 (validate_references)
  │                   └── T011 (validation tests)
  │                       └── T012 (shape sizing)
  │                           └── T013, T014 [P] (row/col, grid/stack)
  │                               └── T015 (compute_layout)
  │                                   └── T016 (constraint graph)
  │                                       └── T017 (apply constraints)
  │                                           └── T018 (conflict detection)
  │                                               └── T019 (edge attachment)
  │                                                   └── T020 (orthogonal routing)
  │                                                       └── T021 (route_connections)
  │                                                           └── T022 (routing tests)
  │                                                               └── T023 (renderer module) [P]
  │                                                                   └── T024 (SvgBuilder)
  │                                                                       └── T025 (shape renderers)
  │                                                                           └── T026 (connection renderer)
  │                                                                               └── T027 (render_svg)
  │                                                                                   └── T028 (pipeline API)
  │                                                                                       └── T029, T030, T031 [P] (polish)
  │                                                                                           └── T032 (final cleanup)
```

---

## Parallel Execution Guide

### Phase 1 (Setup)
```
T002, T003, T004 can run in parallel (different type definitions)
T006, T007 can run in parallel (config and error are independent)
```

### Phase 3 (Layout)
```
T013, T014 can run in parallel (row/col vs grid/stack)
```

### Phase 6 (Renderer)
```
T023 (module), T024 (builder), T025 (shapes) are sequential
T026 depends on T025
```

### Phase 8 (Polish)
```
T029, T030, T031 can run in parallel (tests, CLI, error rendering)
```

---

## Summary

| Metric | Value |
|--------|-------|
| Total Tasks | 32 |
| Parallel Opportunities | 12 tasks marked [P] |
| User Stories Covered | 5 |
| Phases | 8 |
| Estimated Checkpoints | 8 |

**MVP Scope**: Complete through T022 (US1-US4) for functional layout and routing.

**Next Command**: `/specswarm:implement` to begin execution.

---

*Generated: 2026-01-23*
