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
pub mod template;

pub use error::ParseError;
pub use layout::{LayoutConfig, LayoutError, LayoutResult};
pub use parser::{parse, Document};
pub use renderer::{render_svg, render_svg_with_stylesheet, SvgConfig};
pub use template::{resolve_templates, TemplateError, TemplateRegistry};

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

    /// Error during template resolution
    #[error("template error: {0}")]
    Template(#[from] TemplateError),
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
    /// Trace mode: show internal constraint solver and routing debug output
    pub trace: bool,
    /// Lint mode: check for layout defects
    pub lint: bool,
    /// Whether to resolve templates (default: true)
    pub resolve_templates: bool,
    /// Base path for resolving template file references
    pub template_base_path: Option<std::path::PathBuf>,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            layout: LayoutConfig::default(),
            svg: SvgConfig::default(),
            stylesheet: Stylesheet::default(),
            debug: false,
            trace: false,
            lint: false,
            resolve_templates: true, // Templates are resolved by default
            template_base_path: None,
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

    /// Enable or disable trace mode (internal debug output)
    pub fn with_trace(mut self, trace: bool) -> Self {
        self.trace = trace;
        self
    }

    /// Enable or disable lint mode
    pub fn with_lint(mut self, lint: bool) -> Self {
        self.lint = lint;
        self
    }

    /// Enable or disable template resolution
    pub fn with_resolve_templates(mut self, resolve: bool) -> Self {
        self.resolve_templates = resolve;
        self
    }

    /// Set the base path for template file resolution
    pub fn with_template_base_path(mut self, path: std::path::PathBuf) -> Self {
        self.template_base_path = Some(path);
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

/// Validate all color references in a document against the stylesheet
///
/// Returns an error if any symbolic color (like `foreground`, `accent-1`) is not
/// defined in the stylesheet or default palette.
fn validate_colors(doc: &Document, stylesheet: &Stylesheet) -> Result<(), RenderError> {
    use parser::ast::{Statement, StyleValue};

    fn check_color(value: &StyleValue, stylesheet: &Stylesheet) -> Result<(), String> {
        if let StyleValue::Color(color_value) = value {
            if let Some(token) = color_value.token_string() {
                stylesheet::validate_color_token(&token, stylesheet)?;
            }
        }
        Ok(())
    }

    fn validate_modifiers(
        modifiers: &[parser::Spanned<parser::ast::StyleModifier>],
        stylesheet: &Stylesheet,
    ) -> Result<(), String> {
        for modifier in modifiers {
            check_color(&modifier.node.value.node, stylesheet)?;
        }
        Ok(())
    }

    fn validate_statement(stmt: &Statement, stylesheet: &Stylesheet) -> Result<(), String> {
        match stmt {
            Statement::Shape(s) => validate_modifiers(&s.modifiers, stylesheet)?,
            Statement::Layout(l) => {
                validate_modifiers(&l.modifiers, stylesheet)?;
                for child in &l.children {
                    validate_statement(&child.node, stylesheet)?;
                }
            }
            Statement::Group(g) => {
                validate_modifiers(&g.modifiers, stylesheet)?;
                for child in &g.children {
                    validate_statement(&child.node, stylesheet)?;
                }
            }
            Statement::Connection(connections) => {
                for conn in connections {
                    validate_modifiers(&conn.modifiers, stylesheet)?;
                }
            }
            Statement::TemplateDecl(t) => {
                if let Some(body) = &t.body {
                    for child in body {
                        validate_statement(&child.node, stylesheet)?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    for stmt in &doc.statements {
        validate_statement(&stmt.node, stylesheet)
            .map_err(|e| RenderError::Layout(layout::LayoutError::validation_error(e)))?;
    }

    Ok(())
}

/// Extract rotation modifiers from template instances in a document.
///
/// Scans all statements (including nested ones) for template instances with
/// a `rotation` modifier and builds a map from instance name to rotation angle.
fn extract_template_rotations(doc: &Document) -> std::collections::HashMap<String, f64> {
    use parser::ast::{Statement, StyleValue};
    let mut rotations = std::collections::HashMap::new();

    fn visit_statements(
        stmts: &[parser::ast::Spanned<Statement>],
        rotations: &mut std::collections::HashMap<String, f64>,
    ) {
        for stmt in stmts {
            match &stmt.node {
                Statement::TemplateInstance(inst) => {
                    // Check for rotation modifier
                    for (key, value) in &inst.arguments {
                        if key.node.0 == "rotation" {
                            if let StyleValue::Number { value: angle, .. } = &value.node {
                                rotations.insert(inst.instance_name.node.0.clone(), *angle);
                            }
                        }
                    }
                }
                Statement::Layout(l) => {
                    visit_statements(&l.children, rotations);
                }
                Statement::Group(g) => {
                    visit_statements(&g.children, rotations);
                }
                Statement::Label(inner) => {
                    // Labels contain a single inner statement
                    let inner_spanned = parser::ast::Spanned {
                        node: (**inner).clone(),
                        span: stmt.span.clone(),
                    };
                    visit_statements(&[inner_spanned], rotations);
                }
                _ => {}
            }
        }
    }

    visit_statements(&doc.statements, &mut rotations);
    rotations
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
    let (svg, _) = render_pipeline(source, config)?;
    Ok(svg)
}

/// Render DSL source to SVG with lint checking.
///
/// Returns the SVG string and any lint warnings found.
pub fn render_with_lint(
    source: &str,
    config: RenderConfig,
) -> Result<(String, Vec<layout::lint::LintWarning>), RenderError> {
    render_pipeline(source, config)
}

/// Internal shared render pipeline.
fn render_pipeline(
    source: &str,
    config: RenderConfig,
) -> Result<(String, Vec<layout::lint::LintWarning>), RenderError> {
    // Parse the source
    let doc = parse(source)?;

    // Extract rotation modifiers from template instances BEFORE resolution
    // (template instances are converted to groups during resolution, losing their modifiers)
    let template_rotations = extract_template_rotations(&doc);

    // Resolve templates if enabled
    let doc = if config.resolve_templates {
        let mut registry = if let Some(base) = &config.template_base_path {
            TemplateRegistry::with_base_path(base.clone())
        } else {
            TemplateRegistry::new()
        };
        resolve_templates(doc, &mut registry)?
    } else {
        doc
    };

    // Validate color references against stylesheet
    validate_colors(&doc, &config.stylesheet)?;

    // Create layout config with trace flag propagated
    let mut layout_config = config.layout.clone();
    layout_config.trace = config.trace;

    // Compute layout
    let mut result = layout::compute(&doc, &layout_config)?;

    // Resolve constrain statements first (constraint-solver based positioning)
    // This must run before place statements so that offsets are applied after alignment
    // Use two-phase solver when there are rotations, otherwise use single-phase
    if template_rotations.is_empty() {
        layout::resolve_constrain_statements(&mut result, &doc, &layout_config)?;
    } else {
        layout::engine::resolve_constrain_statements_two_phase(
            &mut result,
            &doc,
            &layout_config,
            &template_rotations,
        )?;
    }

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

    // Lint pass
    let lint_warnings = if config.lint {
        layout::lint::check(&result, &doc)
    } else {
        Vec::new()
    };

    // Generate SVG with stylesheet
    let svg = render_svg_with_stylesheet(&result, &config.svg, &config.stylesheet, config.debug);

    Ok((svg, lint_warnings))
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
    fn test_render_connection_curved_routing() {
        // Curved routing should produce a cubic Bezier (M ... C ...)
        let svg = render(
            r#"
            row {
                rect a
                rect b
            }
            a -> b [routing: curved]
        "#,
        )
        .unwrap();

        assert!(svg.contains("ai-connection"));
        assert!(svg.contains("<path"));
        // Curved routing uses SVG C command for cubic Bezier
        assert!(
            svg.contains(" C") || svg.contains("C "),
            "Curved routing should use cubic Bezier (C command)"
        );
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

    #[test]
    fn test_render_curved_connection_with_via() {
        // Curved connection with external via point (determines curve bulge)
        let svg = render(
            r#"
            rect a [x: 0, y: 0]
            rect b [x: 200, y: 0]
            circle ctrl [x: 100, y: 100, size: 6]
            a -> b [routing: curved, via: ctrl]
        "#,
        )
        .unwrap();
        assert!(svg.contains("ai-connection"));
        assert!(svg.contains("<path"));
        // Should use C command for cubic Bezier
        assert!(
            svg.contains(" C") || svg.contains("C "),
            "Via-routed curve should use cubic Bezier (C command)"
        );
    }

    #[test]
    fn test_render_curved_connection_multi_via() {
        // Multi-via with explicit C commands for each segment
        let svg = render(
            r#"
            rect a [x: 0, y: 0]
            rect b [x: 200, y: 0]
            circle c1 [x: 50, y: 50, size: 6]
            circle c2 [x: 150, y: -50, size: 6]
            a -> b [routing: curved, via: c1, via: c2]
        "#,
        )
        .unwrap();
        assert!(svg.contains("ai-connection"));
        assert!(svg.contains("<path"));
        // Multi-via should produce multiple C commands (explicit cubic Beziers)
        assert!(
            svg.contains(" C") || svg.contains("C "),
            "Multi-via should use C commands"
        );
    }
}
