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
pub mod stylesheet;

pub use error::ParseError;
pub use layout::{LayoutConfig, LayoutError, LayoutResult};
pub use parser::{parse, Document};
pub use renderer::{render_svg, render_svg_with_stylesheet, SvgConfig};

use thiserror::Error;

// Re-export Stylesheet for public API
pub use stylesheet::Stylesheet;

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
#[derive(Debug, Clone)]
pub struct RenderConfig {
    /// Layout configuration
    pub layout: LayoutConfig,
    /// SVG output configuration
    pub svg: SvgConfig,
    /// Stylesheet for color resolution
    pub stylesheet: Stylesheet,
    /// Debug mode: show container bounds and element IDs
    pub debug: bool,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            layout: LayoutConfig::default(),
            svg: SvgConfig::default(),
            stylesheet: Stylesheet::default(),
            debug: false,
        }
    }
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

    /// Set the stylesheet for color resolution
    pub fn with_stylesheet(mut self, stylesheet: Stylesheet) -> Self {
        self.stylesheet = stylesheet;
        self
    }

    /// Enable or disable debug mode
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
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

    // Resolve constrain statements first (constraint-solver based positioning)
    // This must run before place statements so that offsets are applied after alignment
    layout::resolve_constrain_statements(&mut result, &doc, &config.layout)?;

    // Resolve constraints (relational positioning and offsets from `place` statements)
    layout::resolve_constraints(&mut result, &doc)?;

    // Route connections
    layout::route_connections(&mut result, &doc)?;

    // Debug output
    if config.debug {
        fn print_tree(elem: &layout::ElementLayout, depth: usize) {
            let indent = "  ".repeat(depth);
            let id = elem.id.as_ref().map(|i| i.0.as_str()).unwrap_or("<anon>");
            eprintln!(
                "{}[{}] x={:.1} y={:.1} w={:.1} h={:.1}",
                indent, id, elem.bounds.x, elem.bounds.y, elem.bounds.width, elem.bounds.height
            );
            for child in &elem.children {
                print_tree(child, depth + 1);
            }
        }
        eprintln!("=== Layout Debug ===");
        for elem in &result.root_elements {
            print_tree(elem, 0);
        }
        eprintln!("====================");
    }

    // Generate SVG with stylesheet
    let svg = render_svg_with_stylesheet(&result, &config.svg, &config.stylesheet, config.debug);

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

    #[test]
    fn test_render_text_shape() {
        // Text shape should render as SVG text element
        let svg = render(r#"text "Hello World" greeting"#).unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("<text"));
        assert!(svg.contains("Hello World"));
        assert!(svg.contains(r#"id="greeting""#));
        assert!(svg.contains("ai-text")); // CSS class for text shapes
    }

    #[test]
    fn test_render_text_with_fill() {
        // Text shape with fill color
        let svg = render(r#"text "Red Text" red_text [fill: red]"#).unwrap();
        assert!(svg.contains("Red Text"));
        assert!(svg.contains(r#"fill="red""#));
    }

    #[test]
    fn test_render_text_with_font_size() {
        // Text shape with custom font size
        let svg = render(r#"text "Big Text" big [font_size: 24]"#).unwrap();
        assert!(svg.contains("Big Text"));
        assert!(svg.contains(r#"font-size="24""#));
    }

    #[test]
    fn test_render_text_with_connection() {
        // Two text elements connected by an arrow
        let svg = render(
            r#"
            row {
                text "Label A" a
                text "Label B" b
            }
            a -> b
        "#,
        )
        .unwrap();
        assert!(svg.contains("Label A"));
        assert!(svg.contains("Label B"));
        assert!(svg.contains("ai-connection"));
        assert!(svg.contains("<path"));
    }

    #[test]
    fn test_render_text_in_layout() {
        // Text elements in a row layout
        let svg = render(
            r#"
            row {
                text "First" first
                text "Second" second
                text "Third" third
            }
        "#,
        )
        .unwrap();
        assert!(svg.contains("First"));
        assert!(svg.contains("Second"));
        assert!(svg.contains("Third"));
    }
}
