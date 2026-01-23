//! Agent Illustrator - A declarative illustration language for AI agents
//!
//! This library provides a parser and AST for the Agent Illustrator DSL.

pub mod error;
pub mod parser;

pub use error::ParseError;
pub use parser::{parse, Document};
