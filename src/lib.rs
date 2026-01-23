//! Agent Illustrator - A declarative illustration language for AI agents
//!
//! This library provides a parser, layout engine, and renderer for the Agent Illustrator DSL.
//!
//! # Example
//!
//! ```rust
//! use agent_illustrator::render;
//!
//! let svg = render("rect server").unwrap();
//! assert!(svg.contains("<svg"));
//! ```

pub mod error;
pub mod layout;
pub mod parser;
pub mod renderer;

pub use error::ParseError;
pub use layout::{LayoutConfig, LayoutError, LayoutResult};
pub use parser::{parse, Document};
pub use renderer::{render_svg, SvgConfig};

use thiserror::Error;

/// Errors that can occur during the render pipeline
#[derive(Debug, Error)]
pub enum RenderError {
    /// Error during parsing
    #[error("parse errors: {}", format_parse_errors(.0))]
    Parse(Vec<ParseError>),

    /// Error during layout
    #[error("layout error: {0}")]
    Layout(#[from] LayoutError),
}

impl From<Vec<ParseError>> for RenderError {
    fn from(errors: Vec<ParseError>) -> Self {
        RenderError::Parse(errors)
    }
}

fn format_parse_errors(errors: &[ParseError]) -> String {
    errors
        .iter()
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join("; ")
}

/// Configuration for the complete render pipeline
#[derive(Debug, Clone, Default)]
pub struct RenderConfig {
    /// Layout configuration
    pub layout: LayoutConfig,
    /// SVG output configuration
    pub svg: SvgConfig,
}

impl RenderConfig {
    /// Create a new configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the layout configuration
    pub fn with_layout(mut self, config: LayoutConfig) -> Self {
        self.layout = config;
        self
    }

    /// Set the SVG configuration
    pub fn with_svg(mut self, config: SvgConfig) -> Self {
        self.svg = config;
        self
    }
}

/// Render DSL source to SVG with default configuration
///
/// This is the main entry point for the library. It parses the source,
/// computes layout, and generates SVG output.
///
/// # Example
///
/// ```rust
/// use agent_illustrator::render;
///
/// let svg = render(r#"
///     row {
///         rect server
///         rect client
///     }
///     server -> client
/// "#).unwrap();
///
/// assert!(svg.contains("<svg"));
/// assert!(svg.contains("server"));
/// assert!(svg.contains("client"));
/// ```
pub fn render(source: &str) -> Result<String, RenderError> {
    render_with_config(source, RenderConfig::default())
}

/// Render DSL source to SVG with custom configuration
///
/// # Example
///
/// ```rust
/// use agent_illustrator::{render_with_config, RenderConfig, LayoutConfig, SvgConfig};
///
/// let config = RenderConfig::new()
///     .with_layout(LayoutConfig::default().with_element_spacing(30.0))
///     .with_svg(SvgConfig::default().with_viewbox_padding(50.0));
///
/// let svg = render_with_config("rect a rect b", config).unwrap();
/// assert!(svg.contains("<svg"));
/// ```
pub fn render_with_config(source: &str, config: RenderConfig) -> Result<String, RenderError> {
    // Parse the source
    let doc = parse(source)?;

    // Compute layout
    let mut result = layout::compute(&doc, &config.layout)?;

    // Resolve constraints
    layout::resolve_constraints(&mut result, &doc)?;

    // Route connections
    layout::route_connections(&mut result, &doc)?;

    // Generate SVG
    let svg = render_svg(&result, &config.svg);

    Ok(svg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_simple_shape() {
        let svg = render("rect server").unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("server"));
    }

    #[test]
    fn test_render_multiple_shapes() {
        let svg = render("rect a rect b").unwrap();
        assert!(svg.contains(r#"id="a""#));
        assert!(svg.contains(r#"id="b""#));
    }

    #[test]
    fn test_render_connection() {
        let svg = render(
            r#"
            rect a
            rect b
            a -> b
        "#,
        )
        .unwrap();
        assert!(svg.contains("ai-connection"));
    }

    #[test]
    fn test_render_row_layout() {
        let svg = render("row { rect a rect b }").unwrap();
        assert!(svg.contains("<g"));
        assert!(svg.contains("</g>"));
    }

    #[test]
    fn test_render_with_label() {
        let svg = render(r#"rect server [label: "Server"]"#).unwrap();
        assert!(svg.contains("<text"));
        assert!(svg.contains("Server"));
    }

    #[test]
    fn test_render_with_styles() {
        let svg = render(r#"rect server [fill: #ff0000]"#).unwrap();
        assert!(svg.contains(r##"fill="#ff0000""##));
    }

    #[test]
    fn test_render_undefined_reference_error() {
        let result = render("a -> b");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, RenderError::Layout(_)));
    }

    #[test]
    fn test_render_connection_direct_routing() {
        // Direct routing should produce a simple 2-point path (M x1,y1 L x2,y2)
        let svg = render(
            r#"
            row {
                rect a
                rect b
            }
            a -> b [routing: direct]
        "#,
        )
        .unwrap();
        assert!(svg.contains("ai-connection"));
        // The SVG path should be rendered - check it contains path element with d attribute
        assert!(svg.contains("<path"));
        // Direct routing between horizontally aligned elements creates a simple line
        // The path should NOT have multiple L commands for intermediate points
    }

    #[test]
    fn test_render_connection_orthogonal_routing_explicit() {
        // Explicit orthogonal routing should work the same as default
        let svg = render(
            r#"
            row {
                rect a
                rect b
            }
            a -> b [routing: orthogonal]
        "#,
        )
        .unwrap();
        assert!(svg.contains("ai-connection"));
        assert!(svg.contains("<path"));
    }
}
