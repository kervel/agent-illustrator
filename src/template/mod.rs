//! Template system for reusable components
//!
//! This module provides the infrastructure for defining, storing, and resolving templates
//! in Agent Illustrator. Templates allow users to define reusable shapes and compositions
//! that can be instantiated with custom parameters.
//!
//! # Example
//!
//! ```text
//! // Define a template
//! template "server" (fill: blue, size: 50) {
//!     rect box [fill: fill, size: size]
//!     text "Server" label [role: label]
//! }
//!
//! // Instantiate the template
//! server myserver [fill: red, size: 100]
//! ```

mod registry;
mod resolver;

pub use registry::{TemplateDefinition, TemplateError, TemplateRegistry};
pub use resolver::{resolve_templates, ResolutionContext};
