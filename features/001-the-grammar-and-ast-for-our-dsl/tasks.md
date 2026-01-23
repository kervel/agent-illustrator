# Tasks: Grammar and AST for the Agent Illustrator DSL

<!-- Tech Stack Validation: PASSED -->
<!-- Validated against: .specswarm/tech-stack.md -->
<!-- No prohibited technologies found -->

## Overview

| Attribute | Value |
|-----------|-------|
| Feature | Grammar and AST for the Agent Illustrator DSL |
| Total Tasks | 24 |
| Phases | 6 |
| Parallel Opportunities | 8 task groups |

---

## Phase 1: Project Setup

**Goal:** Configure Cargo project with dependencies and module structure.

### T001: Update Cargo.toml with dependencies
**File:** `Cargo.toml`
**Story:** Setup

Update `Cargo.toml` to add all required dependencies:

```toml
[package]
name = "agent-illustrator"
version = "0.1.0"
edition = "2021"

[dependencies]
logos = "0.14"
chumsky = "1.0.0-alpha.7"
ariadne = "0.4"
thiserror = "1.0"

[dev-dependencies]
insta = "1.39"
pretty_assertions = "1.4"
```

**Acceptance:** `cargo check` passes with all dependencies resolved.

---

### T002: Create library module structure [P]
**File:** `src/lib.rs`
**Story:** Setup
**Parallel with:** T003, T004

Create the library root with module declarations:

```rust
//! Agent Illustrator - A declarative illustration language for AI agents
//!
//! This library provides a parser and AST for the Agent Illustrator DSL.

pub mod error;
pub mod parser;

pub use error::ParseError;
pub use parser::{parse, Document};
```

**Acceptance:** Module structure compiles.

---

### T003: Create parser module structure [P]
**File:** `src/parser/mod.rs`
**Story:** Setup
**Parallel with:** T002, T004

Create the parser module with submodule declarations:

```rust
//! Parser for the Agent Illustrator DSL

pub mod ast;
pub mod lexer;
mod parser;

pub use ast::*;
pub use parser::parse;
```

**Acceptance:** Parser module compiles (with placeholder submodules).

---

### T004: Create error module stub [P]
**File:** `src/error.rs`
**Story:** Setup
**Parallel with:** T002, T003

Create error type placeholder:

```rust
//! Error types for parsing and validation

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Parse error at {span:?}: {message}")]
    Syntax {
        span: std::ops::Range<usize>,
        message: String,
    },
}
```

**Acceptance:** Error module compiles.

---

**CHECKPOINT: Phase 1 Complete**
- [ ] `cargo check` passes
- [ ] All module files exist
- [ ] No compiler errors

---

## Phase 2: Lexer Implementation (Scenario 1 Foundation)

**Goal:** Implement token definitions using logos for lexical analysis.

### T005: Define Token enum with logos derive
**File:** `src/parser/lexer.rs`
**Story:** US1 (Simple Illustration)
**Depends on:** T001-T004

Implement the complete Token enum:

```rust
use logos::Logos;

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\n\r]+")]
pub enum Token {
    // Shape keywords
    #[token("rect")]
    Rect,
    #[token("circle")]
    Circle,
    #[token("ellipse")]
    Ellipse,
    #[token("line")]
    Line,
    #[token("polygon")]
    Polygon,
    #[token("icon")]
    Icon,

    // Layout keywords
    #[token("row")]
    Row,
    #[token("col")]
    Col,
    #[token("grid")]
    Grid,
    #[token("stack")]
    Stack,
    #[token("group")]
    Group,

    // Constraint keywords
    #[token("place")]
    Place,
    #[token("right-of")]
    RightOf,
    #[token("left-of")]
    LeftOf,
    #[token("above")]
    Above,
    #[token("below")]
    Below,
    #[token("inside")]
    Inside,

    // Connection operators
    #[token("->")]
    Arrow,
    #[token("<-")]
    ArrowBack,
    #[token("<->")]
    ArrowBoth,
    #[token("--")]
    Line,

    // Delimiters
    #[token("{")]
    BraceOpen,
    #[token("}")]
    BraceClose,
    #[token("[")]
    BracketOpen,
    #[token("]")]
    BracketClose,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,

    // Literals
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Ident(String),

    #[regex(r#""([^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        s[1..s.len()-1].to_string()
    })]
    String(String),

    #[regex(r"-?[0-9]+(\.[0-9]+)?", |lex| lex.slice().parse().ok())]
    Number(f64),

    #[regex(r"#[0-9a-fA-F]{3,6}", |lex| lex.slice().to_string())]
    HexColor(String),

    // Comments (skip)
    #[regex(r"//[^\n]*", logos::skip)]
    LineComment,

    #[regex(r"/\*[^*]*\*+(?:[^/*][^*]*\*+)*/", logos::skip)]
    BlockComment,
}

pub type Span = std::ops::Range<usize>;

pub fn lex(input: &str) -> impl Iterator<Item = (Token, Span)> + '_ {
    Token::lexer(input)
        .spanned()
        .filter_map(|(tok, span)| tok.ok().map(|t| (t, span)))
}
```

**Acceptance:** Lexer tokenizes sample input correctly.

---

### T006: Add lexer unit tests
**File:** `src/parser/lexer.rs` (tests module)
**Story:** US1
**Depends on:** T005

Add tests at the bottom of lexer.rs:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shape_keywords() {
        let tokens: Vec<_> = lex("rect circle ellipse").map(|(t,_)| t).collect();
        assert_eq!(tokens, vec![Token::Rect, Token::Circle, Token::Ellipse]);
    }

    #[test]
    fn test_connection_operators() {
        let tokens: Vec<_> = lex("-> <- <-> --").map(|(t,_)| t).collect();
        assert_eq!(tokens, vec![Token::Arrow, Token::ArrowBack, Token::ArrowBoth, Token::Line]);
    }

    #[test]
    fn test_identifiers_and_strings() {
        let tokens: Vec<_> = lex(r#"server "my name""#).map(|(t,_)| t).collect();
        assert_eq!(tokens, vec![
            Token::Ident("server".to_string()),
            Token::String("my name".to_string())
        ]);
    }

    #[test]
    fn test_comments_skipped() {
        let tokens: Vec<_> = lex("rect // comment\ncircle").map(|(t,_)| t).collect();
        assert_eq!(tokens, vec![Token::Rect, Token::Circle]);
    }
}
```

**Acceptance:** `cargo test` passes for lexer tests.

---

**CHECKPOINT: Phase 2 Complete (Lexer)**
- [ ] All tokens defined
- [ ] Lexer tests pass
- [ ] Comments are correctly skipped

---

## Phase 3: AST Type Definitions (All Scenarios)

**Goal:** Define all AST types per data-model.md.

### T007: Define Span and Spanned wrapper [P]
**File:** `src/parser/ast.rs`
**Story:** All
**Parallel with:** T008

```rust
//! Abstract Syntax Tree types for the Agent Illustrator DSL

/// Byte range in source text
pub type Span = std::ops::Range<usize>;

/// AST node with source location
#[derive(Debug, Clone, PartialEq)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(node: T, span: Span) -> Self {
        Self { node, span }
    }
}
```

**Acceptance:** Types compile.

---

### T008: Define Identifier type [P]
**File:** `src/parser/ast.rs`
**Story:** All
**Parallel with:** T007

```rust
/// Valid identifier (alphanumeric + underscore, starts with letter/_)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Identifier(pub String);

impl Identifier {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
```

**Acceptance:** Identifier type compiles.

---

### T009: Define Document and Statement types
**File:** `src/parser/ast.rs`
**Story:** All
**Depends on:** T007, T008

```rust
/// Root AST node - a complete illustration document
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub statements: Vec<Spanned<Statement>>,
}

/// Top-level statement in a document
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// Shape declaration: `rect "name" [styles]`
    Shape(ShapeDecl),
    /// Connection: `a -> b [styles]`
    Connection(ConnectionDecl),
    /// Layout container: `row { ... }`
    Layout(LayoutDecl),
    /// Semantic group: `group "name" { ... }`
    Group(GroupDecl),
    /// Position constraint: `place a right-of b`
    Constraint(ConstraintDecl),
}
```

**Acceptance:** Statement enum compiles.

---

### T010: Define ShapeDecl and ShapeType [P]
**File:** `src/parser/ast.rs`
**Story:** US1
**Parallel with:** T011, T012

```rust
/// Shape declaration
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeDecl {
    pub shape_type: Spanned<ShapeType>,
    pub name: Option<Spanned<Identifier>>,
    pub modifiers: Vec<Spanned<StyleModifier>>,
}

/// Built-in shape types
#[derive(Debug, Clone, PartialEq)]
pub enum ShapeType {
    Rectangle,
    Circle,
    Ellipse,
    Line,
    Polygon,
    Icon { icon_name: String },
}
```

**Acceptance:** Shape types compile.

---

### T011: Define ConnectionDecl and ConnectionDirection [P]
**File:** `src/parser/ast.rs`
**Story:** US1
**Parallel with:** T010, T012

```rust
/// Connection between shapes
#[derive(Debug, Clone, PartialEq)]
pub struct ConnectionDecl {
    pub from: Spanned<Identifier>,
    pub to: Spanned<Identifier>,
    pub direction: ConnectionDirection,
    pub modifiers: Vec<Spanned<StyleModifier>>,
}

/// Connection directionality
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionDirection {
    /// `->` directed from source to target
    Forward,
    /// `<-` directed from target to source
    Backward,
    /// `<->` bidirectional
    Bidirectional,
    /// `--` undirected
    Undirected,
}
```

**Acceptance:** Connection types compile.

---

### T012: Define LayoutDecl, LayoutType, GroupDecl [P]
**File:** `src/parser/ast.rs`
**Story:** US2, US4
**Parallel with:** T010, T011

```rust
/// Layout container
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutDecl {
    pub layout_type: Spanned<LayoutType>,
    pub name: Option<Spanned<Identifier>>,
    pub children: Vec<Spanned<Statement>>,
    pub modifiers: Vec<Spanned<StyleModifier>>,
}

/// Layout arrangement strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutType {
    Row,
    Column,
    Grid,
    Stack,
}

/// Semantic group (no layout implication)
#[derive(Debug, Clone, PartialEq)]
pub struct GroupDecl {
    pub name: Option<Spanned<Identifier>>,
    pub children: Vec<Spanned<Statement>>,
    pub modifiers: Vec<Spanned<StyleModifier>>,
}
```

**Acceptance:** Layout and Group types compile.

---

### T013: Define ConstraintDecl and PositionRelation
**File:** `src/parser/ast.rs`
**Story:** US2
**Depends on:** T008

```rust
/// Position constraint (experimental)
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintDecl {
    pub subject: Spanned<Identifier>,
    pub relation: Spanned<PositionRelation>,
    pub anchor: Spanned<Identifier>,
}

/// Relative position relations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionRelation {
    RightOf,
    LeftOf,
    Above,
    Below,
    Inside,
}
```

**Acceptance:** Constraint types compile.

---

### T014: Define StyleModifier, StyleKey, StyleValue
**File:** `src/parser/ast.rs`
**Story:** US2
**Depends on:** T007

```rust
/// Key-value style modifier
#[derive(Debug, Clone, PartialEq)]
pub struct StyleModifier {
    pub key: Spanned<StyleKey>,
    pub value: Spanned<StyleValue>,
}

/// Known style keys (extensible)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StyleKey {
    Fill,
    Stroke,
    StrokeWidth,
    Opacity,
    Label,
    FontSize,
    Class,
    Custom(String),
}

/// Style values
#[derive(Debug, Clone, PartialEq)]
pub enum StyleValue {
    Color(String),
    Number { value: f64, unit: Option<String> },
    String(String),
    Keyword(String),
}
```

**Acceptance:** Style types compile.

---

**CHECKPOINT: Phase 3 Complete (AST)**
- [ ] All AST types defined
- [ ] `cargo check` passes
- [ ] Types match data-model.md

---

## Phase 4: Parser Implementation (All Scenarios)

**Goal:** Implement chumsky parser to produce AST from tokens.

### T015: Create parser module with chumsky setup
**File:** `src/parser/parser.rs`
**Story:** All
**Depends on:** T005-T014

Set up the parser module with chumsky imports and helper types:

```rust
//! Parser implementation using chumsky

use chumsky::prelude::*;
use crate::parser::ast::*;
use crate::parser::lexer::Token;

type ParserInput<'a> = chumsky::input::SpannedInput<Token, Span, &'a [(Token, Span)]>;

/// Parse DSL source code into an AST
pub fn parse(input: &str) -> Result<Document, Vec<crate::ParseError>> {
    let tokens: Vec<_> = crate::parser::lexer::lex(input).collect();
    let len = input.len();
    let eoi = len..len;

    document_parser()
        .parse(tokens.as_slice().spanned(eoi))
        .into_result()
        .map_err(|errs| errs.into_iter().map(|e| e.into()).collect())
}
```

**Acceptance:** Parser module structure compiles.

---

### T016: Implement primitive parsers (identifier, string, number)
**File:** `src/parser/parser.rs`
**Story:** US1
**Depends on:** T015

```rust
fn identifier<'a>() -> impl Parser<'a, ParserInput<'a>, Spanned<Identifier>, extra::Err<Rich<'a, Token>>> {
    select! {
        Token::Ident(s) => Identifier::new(s),
    }
    .map_with(|id, e| Spanned::new(id, e.span()))
}

fn string_literal<'a>() -> impl Parser<'a, ParserInput<'a>, Spanned<String>, extra::Err<Rich<'a, Token>>> {
    select! {
        Token::String(s) => s,
    }
    .map_with(|s, e| Spanned::new(s, e.span()))
}

fn number<'a>() -> impl Parser<'a, ParserInput<'a>, Spanned<f64>, extra::Err<Rich<'a, Token>>> {
    select! {
        Token::Number(n) => n,
    }
    .map_with(|n, e| Spanned::new(n, e.span()))
}
```

**Acceptance:** Primitive parsers work in isolation.

---

### T017: Implement style modifier parser
**File:** `src/parser/parser.rs`
**Story:** US2
**Depends on:** T016

```rust
fn style_key<'a>() -> impl Parser<'a, ParserInput<'a>, Spanned<StyleKey>, extra::Err<Rich<'a, Token>>> {
    identifier().map(|id| {
        let key = match id.node.as_str() {
            "fill" => StyleKey::Fill,
            "stroke" => StyleKey::Stroke,
            "stroke_width" => StyleKey::StrokeWidth,
            "opacity" => StyleKey::Opacity,
            "label" => StyleKey::Label,
            "font_size" => StyleKey::FontSize,
            "class" => StyleKey::Class,
            other => StyleKey::Custom(other.to_string()),
        };
        Spanned::new(key, id.span)
    })
}

fn style_value<'a>() -> impl Parser<'a, ParserInput<'a>, Spanned<StyleValue>, extra::Err<Rich<'a, Token>>> {
    choice((
        select! { Token::HexColor(c) => StyleValue::Color(c) }.map_with(|v, e| Spanned::new(v, e.span())),
        number().map(|n| Spanned::new(StyleValue::Number { value: n.node, unit: None }, n.span)),
        string_literal().map(|s| Spanned::new(StyleValue::String(s.node), s.span)),
        identifier().map(|id| Spanned::new(StyleValue::Keyword(id.node.0), id.span)),
    ))
}

fn modifier<'a>() -> impl Parser<'a, ParserInput<'a>, Spanned<StyleModifier>, extra::Err<Rich<'a, Token>>> {
    style_key()
        .then_ignore(just(Token::Colon))
        .then(style_value())
        .map_with(|(key, value), e| {
            Spanned::new(StyleModifier { key, value }, e.span())
        })
}

fn modifier_block<'a>() -> impl Parser<'a, ParserInput<'a>, Vec<Spanned<StyleModifier>>, extra::Err<Rich<'a, Token>>> {
    modifier()
        .separated_by(just(Token::Comma))
        .allow_trailing()
        .collect()
        .delimited_by(just(Token::BracketOpen), just(Token::BracketClose))
}
```

**Acceptance:** Style modifier parsing works.

---

### T018: Implement shape declaration parser
**File:** `src/parser/parser.rs`
**Story:** US1
**Depends on:** T016, T017

```rust
fn shape_type<'a>() -> impl Parser<'a, ParserInput<'a>, Spanned<ShapeType>, extra::Err<Rich<'a, Token>>> {
    choice((
        just(Token::Rect).to(ShapeType::Rectangle),
        just(Token::Circle).to(ShapeType::Circle),
        just(Token::Ellipse).to(ShapeType::Ellipse),
        just(Token::Line).to(ShapeType::Line),
        just(Token::Polygon).to(ShapeType::Polygon),
        just(Token::Icon)
            .ignore_then(string_literal())
            .map(|s| ShapeType::Icon { icon_name: s.node }),
    ))
    .map_with(|st, e| Spanned::new(st, e.span()))
}

fn shape_decl<'a>() -> impl Parser<'a, ParserInput<'a>, ShapeDecl, extra::Err<Rich<'a, Token>>> {
    shape_type()
        .then(identifier().or_not())
        .then(modifier_block().or_not())
        .map(|((shape_type, name), modifiers)| ShapeDecl {
            shape_type,
            name,
            modifiers: modifiers.unwrap_or_default(),
        })
}
```

**Acceptance:** Shape declarations parse correctly.

---

### T019: Implement connection declaration parser
**File:** `src/parser/parser.rs`
**Story:** US1
**Depends on:** T016, T017

```rust
fn connection_op<'a>() -> impl Parser<'a, ParserInput<'a>, ConnectionDirection, extra::Err<Rich<'a, Token>>> {
    choice((
        just(Token::Arrow).to(ConnectionDirection::Forward),
        just(Token::ArrowBack).to(ConnectionDirection::Backward),
        just(Token::ArrowBoth).to(ConnectionDirection::Bidirectional),
        just(Token::Line).to(ConnectionDirection::Undirected),
    ))
}

fn connection_decl<'a>() -> impl Parser<'a, ParserInput<'a>, ConnectionDecl, extra::Err<Rich<'a, Token>>> {
    identifier()
        .then(connection_op())
        .then(identifier())
        .then(modifier_block().or_not())
        .map(|(((from, direction), to), modifiers)| ConnectionDecl {
            from,
            to,
            direction,
            modifiers: modifiers.unwrap_or_default(),
        })
}
```

**Acceptance:** Connection declarations parse correctly.

---

### T020: Implement layout and group parsers
**File:** `src/parser/parser.rs`
**Story:** US2, US4
**Depends on:** T016, T017

```rust
fn layout_type<'a>() -> impl Parser<'a, ParserInput<'a>, Spanned<LayoutType>, extra::Err<Rich<'a, Token>>> {
    choice((
        just(Token::Row).to(LayoutType::Row),
        just(Token::Col).to(LayoutType::Column),
        just(Token::Grid).to(LayoutType::Grid),
        just(Token::Stack).to(LayoutType::Stack),
    ))
    .map_with(|lt, e| Spanned::new(lt, e.span()))
}

fn layout_decl<'a>(
    stmt: impl Parser<'a, ParserInput<'a>, Spanned<Statement>, extra::Err<Rich<'a, Token>>> + Clone,
) -> impl Parser<'a, ParserInput<'a>, LayoutDecl, extra::Err<Rich<'a, Token>>> {
    layout_type()
        .then(identifier().or_not())
        .then(modifier_block().or_not())
        .then(
            stmt.clone()
                .repeated()
                .collect()
                .delimited_by(just(Token::BraceOpen), just(Token::BraceClose)),
        )
        .map(|(((layout_type, name), modifiers), children)| LayoutDecl {
            layout_type,
            name,
            children,
            modifiers: modifiers.unwrap_or_default(),
        })
}

fn group_decl<'a>(
    stmt: impl Parser<'a, ParserInput<'a>, Spanned<Statement>, extra::Err<Rich<'a, Token>>> + Clone,
) -> impl Parser<'a, ParserInput<'a>, GroupDecl, extra::Err<Rich<'a, Token>>> {
    just(Token::Group)
        .ignore_then(identifier().or_not())
        .then(modifier_block().or_not())
        .then(
            stmt.clone()
                .repeated()
                .collect()
                .delimited_by(just(Token::BraceOpen), just(Token::BraceClose)),
        )
        .map(|((name, modifiers), children)| GroupDecl {
            name,
            children,
            modifiers: modifiers.unwrap_or_default(),
        })
}
```

**Acceptance:** Layout and group declarations parse correctly.

---

### T021: Implement constraint parser
**File:** `src/parser/parser.rs`
**Story:** US2
**Depends on:** T016

```rust
fn position_relation<'a>() -> impl Parser<'a, ParserInput<'a>, Spanned<PositionRelation>, extra::Err<Rich<'a, Token>>> {
    choice((
        just(Token::RightOf).to(PositionRelation::RightOf),
        just(Token::LeftOf).to(PositionRelation::LeftOf),
        just(Token::Above).to(PositionRelation::Above),
        just(Token::Below).to(PositionRelation::Below),
        just(Token::Inside).to(PositionRelation::Inside),
    ))
    .map_with(|rel, e| Spanned::new(rel, e.span()))
}

fn constraint_decl<'a>() -> impl Parser<'a, ParserInput<'a>, ConstraintDecl, extra::Err<Rich<'a, Token>>> {
    just(Token::Place)
        .ignore_then(identifier())
        .then(position_relation())
        .then(identifier())
        .map(|((subject, relation), anchor)| ConstraintDecl {
            subject,
            relation,
            anchor,
        })
}
```

**Acceptance:** Constraint declarations parse correctly.

---

### T022: Implement statement and document parsers
**File:** `src/parser/parser.rs`
**Story:** All
**Depends on:** T018-T021

```rust
fn statement<'a>() -> impl Parser<'a, ParserInput<'a>, Spanned<Statement>, extra::Err<Rich<'a, Token>>> {
    recursive(|stmt| {
        choice((
            constraint_decl().map(Statement::Constraint),
            layout_decl(stmt.clone()).map(Statement::Layout),
            group_decl(stmt.clone()).map(Statement::Group),
            connection_decl().map(Statement::Connection),
            shape_decl().map(Statement::Shape),
        ))
        .map_with(|s, e| Spanned::new(s, e.span()))
    })
}

fn document_parser<'a>() -> impl Parser<'a, ParserInput<'a>, Document, extra::Err<Rich<'a, Token>>> {
    statement()
        .repeated()
        .collect()
        .map(|statements| Document { statements })
}
```

**Acceptance:** Complete documents parse successfully.

---

**CHECKPOINT: Phase 4 Complete (Parser)**
- [ ] All grammar constructs parse
- [ ] Recursive nesting works
- [ ] `cargo check` passes

---

## Phase 5: Error Formatting (Scenario 3)

**Goal:** Implement beautiful error messages using ariadne.

### T023: Implement ParseError conversion and display
**File:** `src/error.rs`
**Story:** US3
**Depends on:** T015-T022

Update error.rs with ariadne formatting:

```rust
use ariadne::{Color, Label, Report, ReportKind, Source};
use thiserror::Error;

pub type Span = std::ops::Range<usize>;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Parse error at {span:?}: {message}")]
    Syntax {
        span: Span,
        message: String,
        expected: Vec<String>,
    },
}

impl ParseError {
    /// Format the error with source context using ariadne
    pub fn format(&self, source: &str, filename: &str) -> String {
        let mut buf = Vec::new();
        match self {
            ParseError::Syntax { span, message, expected } => {
                let expected_str = if expected.is_empty() {
                    String::new()
                } else {
                    format!("\nExpected: {}", expected.join(", "))
                };

                Report::build(ReportKind::Error, filename, span.start)
                    .with_message(message)
                    .with_label(
                        Label::new((filename, span.clone()))
                            .with_message(format!("{}{}", message, expected_str))
                            .with_color(Color::Red),
                    )
                    .finish()
                    .write((filename, Source::from(source)), &mut buf)
                    .unwrap();
            }
        }
        String::from_utf8(buf).unwrap()
    }
}

impl<'a> From<chumsky::error::Rich<'a, crate::parser::lexer::Token>> for ParseError {
    fn from(err: chumsky::error::Rich<'a, crate::parser::lexer::Token>) -> Self {
        ParseError::Syntax {
            span: err.span().into_range(),
            message: err.to_string(),
            expected: err.expected().map(|e| format!("{:?}", e)).collect(),
        }
    }
}
```

**Acceptance:** Errors display with source context and line numbers.

---

**CHECKPOINT: Phase 5 Complete (Errors)**
- [ ] Errors include line/column info
- [ ] ariadne formatting works
- [ ] Error messages are clear

---

## Phase 6: Integration Testing & Validation

**Goal:** Verify parser works end-to-end with real examples.

### T024: Create integration tests with example documents
**File:** `tests/integration_tests.rs`
**Story:** All
**Depends on:** All previous tasks

```rust
use agent_illustrator::parse;

#[test]
fn test_simple_shapes() {
    let input = r#"
        rect server
        circle db [fill: blue]
        server -> db
    "#;

    let doc = parse(input).expect("Should parse");
    assert_eq!(doc.statements.len(), 3);
}

#[test]
fn test_layout_container() {
    let input = r#"
        row {
            rect a
            rect b
            rect c [fill: red]
        }
    "#;

    let doc = parse(input).expect("Should parse");
    assert_eq!(doc.statements.len(), 1);
    // Verify it's a layout with 3 children
}

#[test]
fn test_nested_groups() {
    let input = r#"
        group datacenter {
            col {
                group rack1 {
                    rect server1
                    rect server2
                }
                group rack2 {
                    rect server3
                }
            }
        }
    "#;

    let doc = parse(input).expect("Should parse");
    assert_eq!(doc.statements.len(), 1);
}

#[test]
fn test_connections_with_labels() {
    let input = r#"
        rect client
        rect server
        client -> server [label: "HTTP", style: dashed]
    "#;

    let doc = parse(input).expect("Should parse");
    assert_eq!(doc.statements.len(), 3);
}

#[test]
fn test_constraints() {
    let input = r#"
        rect server
        rect client
        place client right-of server
    "#;

    let doc = parse(input).expect("Should parse");
    assert_eq!(doc.statements.len(), 3);
}

#[test]
fn test_icon_shapes() {
    let input = r#"
        icon "server" myserver [fill: gray]
        icon "database" db1
    "#;

    let doc = parse(input).expect("Should parse");
    assert_eq!(doc.statements.len(), 2);
}

#[test]
fn test_error_reporting() {
    let input = "rect [invalid";
    let result = parse(input);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(!errors.is_empty());
}
```

**Acceptance:** All integration tests pass.

---

**CHECKPOINT: Phase 6 Complete (Integration)**
- [ ] All integration tests pass
- [ ] `cargo test` succeeds
- [ ] `cargo clippy` clean
- [ ] `cargo fmt` applied

---

## Task Dependencies

```
Phase 1 (Setup)
  T001 ─────────────────┐
  T002 [P] ─────────────┼──► Phase 2
  T003 [P] ─────────────┤
  T004 [P] ─────────────┘

Phase 2 (Lexer)
  T005 ──► T006

Phase 3 (AST) - Can start after T005
  T007 [P] ─┬──► T009 ──► T013
  T008 [P] ─┘
  T010 [P] ─┬──► T014
  T011 [P] ─┤
  T012 [P] ─┘

Phase 4 (Parser) - Needs Phase 2 + Phase 3
  T015 ──► T016 ──► T017 ──┬──► T022
                          │
  T018 ◄──────────────────┤
  T019 ◄──────────────────┤
  T020 ◄──────────────────┤
  T021 ◄──────────────────┘

Phase 5 (Errors) - Needs T022
  T023

Phase 6 (Testing) - Needs all
  T024
```

## Parallel Execution Groups

**Group 1:** T002, T003, T004 (project setup)
**Group 2:** T007, T008 (core AST types)
**Group 3:** T010, T011, T012 (declaration types)
**Group 4:** T018, T019, T020, T021 (parser functions - after T017)

## Summary

| Phase | Tasks | Parallel | Story Coverage |
|-------|-------|----------|----------------|
| 1. Setup | 4 | 3 | Foundation |
| 2. Lexer | 2 | 0 | US1 |
| 3. AST | 8 | 6 | All |
| 4. Parser | 8 | 4 | All |
| 5. Errors | 1 | 0 | US3 |
| 6. Testing | 1 | 0 | All |
| **Total** | **24** | **13** | |

**MVP Scope:** Phases 1-4 (T001-T022) provide a working parser. Phase 5-6 add polish.
