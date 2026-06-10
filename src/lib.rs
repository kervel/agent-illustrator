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
pub use renderer::{render_svg, render_svg_with_keyframes, render_svg_with_stylesheet, SvgConfig};
pub use template::{resolve_templates, TemplateError, TemplateRegistry};

use thiserror::Error;

// Re-export Stylesheet for public API
pub use stylesheet::Stylesheet;

/// Controls how image href paths are emitted in SVG output
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ImageHrefMode {
    /// Use the path exactly as written in the AIL source
    #[default]
    Verbatim,
    /// Normalize the resolved path relative to CWD (removes `..` segments)
    Rewrite,
    /// Use the fully canonicalized absolute path
    Absolute,
    /// Inline the image as a base64 data URI
    Base64,
}

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
    /// Custom CSS to inject into the SVG `<style>` block
    pub custom_css: Option<String>,
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
    /// How image href paths are emitted in SVG output
    pub image_href_mode: ImageHrefMode,
    /// Render a single keyframe as static SVG (by index or name)
    pub frame: Option<String>,
    /// Embed minimal JS for animated playback
    pub animate: bool,
    /// Use pure CSS animation (no JS, works in GitLab/GitHub READMEs)
    pub animate_css: bool,
    /// Skip the auto-generated `.frame-X` CSS rules. Element/connection
    /// `kf-*` / `conn-*` classes and `data-frames` are still emitted; an
    /// external runtime drives transitions by toggling the SVG's class.
    pub no_frame_css: bool,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            layout: LayoutConfig::default(),
            svg: SvgConfig::default(),
            stylesheet: Stylesheet::default(),
            custom_css: None,
            debug: false,
            trace: false,
            lint: false,
            resolve_templates: true, // Templates are resolved by default
            template_base_path: None,
            image_href_mode: ImageHrefMode::default(),
            frame: None,
            animate: false,
            animate_css: false,
            no_frame_css: false,
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

    /// Set custom CSS to inject into the SVG `<style>` block
    pub fn with_custom_css(mut self, css: String) -> Self {
        self.custom_css = Some(css);
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

    /// Set the image href mode for SVG output
    pub fn with_image_href_mode(mut self, mode: ImageHrefMode) -> Self {
        self.image_href_mode = mode;
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
        registry.set_image_href_mode(config.image_href_mode);
        resolve_templates(doc, &mut registry)?
    } else {
        doc
    };

    // Desugar point-constraints (e.g. `a.tip = b.top - 4`) into scalar component
    // constraints before layout and the constraint solver see them.
    let doc = crate::parser::ast::expand_point_constraints(doc);

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

    // Build skip set for rotated template instances — their anchors were already
    // correctly transformed by the two-phase solver and must not be overwritten.
    let rotated_skip: std::collections::HashSet<String> = template_rotations
        .iter()
        .filter(|(_, angle)| angle.abs() > f64::EPSILON)
        .flat_map(|(name, _)| {
            // Include the template instance name and all its prefixed children
            let prefix = format!("{}_", name);
            let mut names = vec![name.clone()];
            for key in result.elements.keys() {
                if key.starts_with(&prefix) {
                    names.push(key.clone());
                }
            }
            names
        })
        .collect();
    let skip_ref = if rotated_skip.is_empty() {
        None
    } else {
        Some(&rotated_skip)
    };

    // Resolve constraints (relational positioning and offsets from `place` statements)
    layout::resolve_constraints(&mut result, &doc, skip_ref)?;

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

    // Keyframe processing (Feature 011)
    let keyframes = layout::keyframe::extract_keyframes(&doc);
    let frame_states = layout::keyframe::compute_frame_states(&keyframes);
    let frame_diffs = layout::keyframe::compute_frame_diffs(&result, &frame_states, &doc, &config.layout);

    // Lint pass
    let lint_warnings = if config.lint {
        layout::lint::check(&result, &doc)
    } else {
        Vec::new()
    };

    // Mutual exclusion check (Feature 011)
    if config.frame.is_some() && config.animate {
        return Err(RenderError::Layout(layout::LayoutError::validation_error(
            "--frame and --animate are mutually exclusive",
        )));
    }

    // Generate SVG with stylesheet
    let svg = if let Some(frame_selector) = &config.frame {
        // Single frame rendering: find the frame, render as static SVG
        if frame_states.is_empty() {
            return Err(RenderError::Layout(layout::LayoutError::validation_error(
                "--frame requires keyframes in the input",
            )));
        }
        let frame_idx = resolve_frame_index(frame_selector, &frame_states)?;
        let state = &frame_states[frame_idx];

        // Apply transforms if present, then remove hidden elements
        let mut frame_result = if !state.transforms.is_empty() {
            layout::keyframe::resolve_frame_for_static(
                &result, state, &doc, &config.layout,
            ).unwrap_or_else(|| result.clone())
        } else {
            result.clone()
        };
        frame_result.root_elements = filter_visible_elements(&frame_result.root_elements, &state.hidden_elements);
        frame_result.connections.retain(|c| {
            c.name.as_ref().is_none_or(|n| !state.hidden_connections.contains(&n.0))
        });

        render_svg_with_stylesheet(
            &frame_result,
            &config.svg,
            &config.stylesheet,
            config.custom_css.as_deref(),
            config.debug,
        )
    } else if !frame_diffs.is_empty() {
        let mut svg = render_svg_with_keyframes(
            &result,
            &config.svg,
            &config.stylesheet,
            config.custom_css.as_deref(),
            config.debug,
            &frame_states,
            &frame_diffs,
            config.no_frame_css,
        );

        // Inject animation: JS (--animate) or CSS-only (--animate-css)
        if config.animate {
            let js = generate_animate_js(&frame_states);
            if let Some(pos) = svg.rfind("</svg>") {
                svg.insert_str(pos, &js);
            }
        } else if config.animate_css {
            let conn_meta = renderer::svg::build_conn_meta(&result);
            let mut base_dims: std::collections::HashMap<String, (f64, f64)> = std::collections::HashMap::new();
            collect_base_dims(&result.root_elements, &mut base_dims);
            let css = generate_animate_css(&frame_states, &frame_diffs, &conn_meta, &base_dims);
            if let Some(pos) = svg.rfind("</style>") {
                svg.insert_str(pos, &css);
            }
        }

        svg
    } else {
        render_svg_with_stylesheet(
            &result,
            &config.svg,
            &config.stylesheet,
            config.custom_css.as_deref(),
            config.debug,
        )
    };

    Ok((svg, lint_warnings))
}

/// Resolve a frame selector (index or name) to an index
fn resolve_frame_index(
    selector: &str,
    frame_states: &[layout::keyframe::FrameState],
) -> Result<usize, RenderError> {
    // Try as index first
    if let Ok(idx) = selector.parse::<usize>() {
        if idx < frame_states.len() {
            return Ok(idx);
        }
        return Err(RenderError::Layout(layout::LayoutError::validation_error(
            format!("frame index {} out of range (0-{})", idx, frame_states.len() - 1),
        )));
    }
    // Try as name
    for (i, state) in frame_states.iter().enumerate() {
        if state.name == selector {
            return Ok(i);
        }
    }
    Err(RenderError::Layout(layout::LayoutError::validation_error(
        format!("unknown frame '{}'. Available: {}", selector,
            frame_states.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(", ")),
    )))
}

/// Remove hidden elements from layout for static frame rendering.
/// Removes matching elements entirely (including labels) for clean single-frame output.
fn filter_visible_elements(
    elements: &[layout::ElementLayout],
    hidden: &std::collections::HashSet<String>,
) -> Vec<layout::ElementLayout> {
    elements
        .iter()
        .filter(|e| {
            e.id.as_ref().is_none_or(|id| !hidden.contains(&id.0))
        })
        .cloned()
        .map(|mut e| {
            e.children = filter_visible_elements(&e.children, hidden);
            e
        })
        .collect()
}

/// Generate pure CSS animation (no JS required).
/// Each element/connection gets its own @keyframes animation that toggles
/// opacity at the right frame percentages.
/// Collect each element's base (frame-0) width/height by id, recursively.
fn collect_base_dims(
    elements: &[layout::ElementLayout],
    out: &mut std::collections::HashMap<String, (f64, f64)>,
) {
    for e in elements {
        if let Some(id) = &e.id {
            out.insert(id.0.clone(), (e.bounds.width, e.bounds.height));
        }
        collect_base_dims(&e.children, out);
    }
}

/// Emit a `@keyframes` body for a per-frame value timeline, holding each frame's value
/// then easing to the next (mimics the JS frame-class + transition feel). `vals[i]` is
/// the full CSS value for frame i (e.g. "translate(40px, 0px)"); None = `identity`.
/// Returns None when every frame equals `identity` (nothing to animate).
fn smooth_keyframes_body(prop: &str, vals: &[Option<String>], identity: &str, pct_per_frame: f64) -> Option<String> {
    if vals.iter().all(|v| v.as_deref().unwrap_or(identity) == identity) {
        return None;
    }
    let val = |i: usize| vals[i].clone().unwrap_or_else(|| identity.to_string());
    let trans = pct_per_frame * 0.3; // ~30% of each frame window eases; the rest holds
    let mut body = String::new();
    body.push_str(&format!("  0% {{ {}: {}; }}\n", prop, val(0)));
    for i in 1..vals.len() {
        let start = i as f64 * pct_per_frame;
        body.push_str(&format!("  {:.2}% {{ {}: {}; }}\n", start, prop, val(i - 1)));
        body.push_str(&format!("  {:.2}% {{ {}: {}; }}\n", (start + trans).min(100.0), prop, val(i)));
    }
    body.push_str(&format!("  100% {{ {}: {}; }}\n", prop, val(vals.len() - 1)));
    Some(body)
}

fn generate_animate_css(
    frame_states: &[layout::keyframe::FrameState],
    frame_diffs: &[layout::keyframe::FrameLayout],
    conn_meta: &std::collections::HashMap<String, (layout::RoutingMode, bool, f64)>,
    base_dims: &std::collections::HashMap<String, (f64, f64)>,
) -> String {
    let n = frame_diffs.len();
    if n == 0 {
        return String::new();
    }

    let frame_duration = 2.0; // seconds per frame
    let total_duration = n as f64 * frame_duration;
    let pct_per_frame = 100.0 / n as f64;

    let mut css = String::new();
    css.push_str("\n/* CSS-only animation (auto-generated) */\n");

    // Collect all elements that need animation: track their opacity per frame
    let mut elem_timelines: std::collections::BTreeMap<String, Vec<f64>> = std::collections::BTreeMap::new();
    let mut conn_timelines: std::collections::BTreeMap<String, Vec<f64>> = std::collections::BTreeMap::new();

    // Build per-frame visibility state for each element
    for (i, state) in frame_states.iter().enumerate() {
        // Elements: hidden = 0, visible = 1 (check if element is in hidden set)
        // We need to know ALL element IDs that are ever hidden
        for elem_id in &state.hidden_elements {
            elem_timelines.entry(elem_id.clone()).or_insert_with(|| vec![1.0; n]);
            elem_timelines.get_mut(elem_id).unwrap()[i] = 0.0;
        }
        for conn_id in &state.hidden_connections {
            conn_timelines.entry(conn_id.clone()).or_insert_with(|| vec![1.0; n]);
            conn_timelines.get_mut(conn_id).unwrap()[i] = 0.0;
        }
    }

    // Also check frame diffs for opacity overrides (transforms with explicit opacity)
    for (i, diff) in frame_diffs.iter().enumerate() {
        for (elem_id, d) in &diff.element_diffs {
            if let Some(opacity) = d.opacity {
                let timeline = elem_timelines.entry(elem_id.clone()).or_insert_with(|| vec![0.0; n]);
                timeline[i] = opacity;
            }
        }
        for (conn_id, d) in &diff.connection_diffs {
            if let Some(opacity) = d.opacity {
                let timeline = conn_timelines.entry(conn_id.clone()).or_insert_with(|| vec![0.0; n]);
                timeline[i] = opacity;
            }
        }
    }

    // Generate @keyframes for each element
    for (elem_id, timeline) in &elem_timelines {
        // Skip elements that never change (always hidden or always visible)
        if timeline.windows(2).all(|w| (w[0] - w[1]).abs() < f64::EPSILON) {
            continue;
        }

        let anim_name = format!("kf-anim-{}", elem_id);
        css.push_str(&format!("@keyframes {} {{\n", anim_name));

        for (i, &opacity) in timeline.iter().enumerate() {
            let start_pct = i as f64 * pct_per_frame;
            let end_pct = (i + 1) as f64 * pct_per_frame;
            // Use step timing: element should have this opacity for the entire frame
            if i == n - 1 {
                css.push_str(&format!("  {:.1}% {{ opacity: {}; }}\n", start_pct, opacity));
            } else {
                css.push_str(&format!(
                    "  {:.1}%, {:.1}% {{ opacity: {}; }}\n",
                    start_pct,
                    end_pct - 0.01,
                    opacity
                ));
            }
        }
        css.push_str("}\n");

        css.push_str(&format!(
            ".kf-{} {{ animation: {} {:.1}s step-end infinite; }}\n",
            elem_id, anim_name, total_duration
        ));
    }

    // Element geometry (smooth): transform on the wrapper, width/height on the shape.
    let mut xf_tl: std::collections::BTreeMap<String, Vec<Option<String>>> = std::collections::BTreeMap::new();
    let mut w_tl: std::collections::BTreeMap<String, Vec<Option<String>>> = std::collections::BTreeMap::new();
    let mut h_tl: std::collections::BTreeMap<String, Vec<Option<String>>> = std::collections::BTreeMap::new();
    for (i, diff) in frame_diffs.iter().enumerate() {
        for (id, d) in &diff.element_diffs {
            if let Some(t) = renderer::svg::frame_transform_css(d.tx, d.ty, d.rotation) {
                xf_tl.entry(id.clone()).or_insert_with(|| vec![None; n])[i] = Some(t);
            }
            if let Some(wv) = d.width {
                w_tl.entry(id.clone()).or_insert_with(|| vec![None; n])[i] = Some(format!("{}px", wv));
            }
            if let Some(hv) = d.height {
                h_tl.entry(id.clone()).or_insert_with(|| vec![None; n])[i] = Some(format!("{}px", hv));
            }
        }
    }
    for (id, vals) in &xf_tl {
        if let Some(body) = smooth_keyframes_body("transform", vals, "translate(0px, 0px)", pct_per_frame) {
            let anim = format!("kf-geo-{}", id);
            css.push_str(&format!("@keyframes {} {{\n{}}}\n", anim, body));
            css.push_str(&format!(".kf-{} {{ animation: {} {:.1}s ease infinite; }}\n", id, anim, total_duration));
        }
    }
    for (id, vals) in &w_tl {
        let identity = base_dims.get(id).map(|(w, _)| format!("{}px", w)).unwrap_or_default();
        if let Some(body) = smooth_keyframes_body("width", vals, &identity, pct_per_frame) {
            let anim = format!("kf-width-{}", id);
            css.push_str(&format!("@keyframes {} {{\n{}}}\n", anim, body));
            css.push_str(&format!("#{} {{ animation: {} {:.1}s ease infinite; }}\n", id, anim, total_duration));
        }
    }
    for (id, vals) in &h_tl {
        let identity = base_dims.get(id).map(|(_, h)| format!("{}px", h)).unwrap_or_default();
        if let Some(body) = smooth_keyframes_body("height", vals, &identity, pct_per_frame) {
            let anim = format!("kf-height-{}", id);
            css.push_str(&format!("@keyframes {} {{\n{}}}\n", anim, body));
            css.push_str(&format!("#{} {{ animation: {} {:.1}s ease infinite; }}\n", id, anim, total_duration));
        }
    }

    // Generate @keyframes for each connection
    for (conn_id, timeline) in &conn_timelines {
        if timeline.windows(2).all(|w| (w[0] - w[1]).abs() < f64::EPSILON) {
            continue;
        }

        let anim_name = format!("kf-anim-conn-{}", conn_id);
        css.push_str(&format!("@keyframes {} {{\n", anim_name));

        for (i, &opacity) in timeline.iter().enumerate() {
            let start_pct = i as f64 * pct_per_frame;
            let end_pct = (i + 1) as f64 * pct_per_frame;
            if i == n - 1 {
                css.push_str(&format!("  {:.1}% {{ opacity: {}; }}\n", start_pct, opacity));
            } else {
                css.push_str(&format!(
                    "  {:.1}%, {:.1}% {{ opacity: {}; }}\n",
                    start_pct,
                    end_pct - 0.01,
                    opacity
                ));
            }
        }
        css.push_str("}\n");

        css.push_str(&format!(
            ".ai-connection.conn-{} {{ animation: {} {:.1}s step-end infinite; }}\n",
            conn_id, anim_name, total_duration
        ));
    }

    css
}

/// Generate minimal JS for animated playback
fn generate_animate_js(frame_states: &[layout::keyframe::FrameState]) -> String {
    let frame_names: Vec<&str> = frame_states.iter().map(|s| s.name.as_str()).collect();
    format!(
        r#"<script>
(function() {{
  var frames = {:?};
  var current = 0;
  var svg = document.querySelector('svg[data-frames]');
  function showFrame(i) {{
    frames.forEach(function(f) {{ svg.classList.remove('frame-' + f); }});
    svg.classList.add('frame-' + frames[i]);
  }}
  showFrame(0);
  svg.addEventListener('click', function() {{
    current = (current + 1) % frames.length;
    showFrame(current);
  }});
  setInterval(function() {{
    current = (current + 1) % frames.length;
    showFrame(current);
  }}, 2000);
}})();
</script>"#,
        frame_names
    )
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
    fn test_render_fill_opacity() {
        let svg = render(r#"rect cell [fill: #ff0000, fill_opacity: 0.7]"#).unwrap();
        assert!(svg.contains(r##"fill="#ff0000""##));
        assert!(svg.contains(r#"fill-opacity="0.7""#));
    }

    #[test]
    fn test_render_stroke_opacity() {
        let svg = render(r#"rect cell [stroke: #333333, stroke_opacity: 0.5]"#).unwrap();
        assert!(svg.contains(r#"stroke-opacity="0.5""#));
    }

    #[test]
    fn test_render_opacity_whole_element() {
        let svg = render(r#"rect cell [opacity: 0.3]"#).unwrap();
        assert!(svg.contains(r#"opacity="0.3""#));
    }

    #[test]
    fn test_render_fill_opacity_composes_with_symbolic_color() {
        // fill_opacity must keep the symbolic/CSS-token color, not flatten it to a blended hex
        let svg = render(r#"rect cell [fill: secondary-1, fill_opacity: 0.5]"#).unwrap();
        assert!(svg.contains(r#"fill="var(--secondary-1)""#));
        assert!(svg.contains(r#"fill-opacity="0.5""#));
    }

    #[test]
    fn test_render_fill_opacity_clamped_low() {
        // Values below 0 clamp to 0
        let svg = render(r#"rect cell [fill: #ff0000, fill_opacity: -0.5]"#).unwrap();
        assert!(svg.contains(r#"fill-opacity="0""#));
    }

    #[test]
    fn test_render_callout_emits_pill_pointer_and_label() {
        let svg =
            render(r#"callout tag [label: "= subject", pointer: down, fill: #fff, stroke: #000]"#)
                .unwrap();
        // Single closed path (pill + pointer) and a centered label
        assert!(svg.contains("<path"));
        assert!(svg.contains(" Z\""));
        assert!(svg.contains("= subject"));
    }

    #[test]
    fn test_render_callout_tip_anchor_resolves_in_connection() {
        // tag.tip must resolve as a connection endpoint
        let svg = render(
            r#"
            rect box
            callout tag [label: "x", pointer: down]
            tag.tip -> box [routing: direct]
        "#,
        )
        .unwrap();
        // callout path + connection path
        assert!(svg.matches("<path").count() >= 2);
    }

    #[test]
    fn test_render_callout_pointer_direction_moves_apex() {
        // pointer:up puts the apex at the top edge (y=0); pointer:down at the bottom
        let up = render(r#"callout t [label: "x", pointer: up]"#).unwrap();
        let down = render(r#"callout t [label: "x", pointer: down]"#).unwrap();
        assert_ne!(up, down);
    }

    #[test]
    fn test_render_grid_lattice_coordinates() {
        // Cells sit on a regular lattice: cell_width/height + gap, row-major.
        let svg = render(
            r#"grid g [cols: 2, rows: 2, gap: 5, cell_width: 50, cell_height: 50] {
                rect a [fill: #111]
                rect b [fill: #222]
                rect c [fill: #333]
                rect d [fill: #444]
            }"#,
        )
        .unwrap();
        // Columns 50px wide + 5px gap → x offsets differ by 55; rows likewise.
        // Children inherit the 50x50 cell size.
        assert!(svg.contains(r#"width="50" height="50""#));
        // 4 rects placed
        assert_eq!(svg.matches(r#"class="ai-shape ai-rect""#).count(), 4);
    }

    #[test]
    fn test_render_grid_at_placement_is_sparse() {
        // Only declared cells render; unoccupied cells stay empty.
        let svg = render(
            r#"grid g [cols: 3, rows: 3, cell_width: 40, cell_height: 40] {
                rect [at: [0,0], fill: #111]
                rect [at: [2,2], fill: #222]
            }"#,
        )
        .unwrap();
        // Two declared cells → exactly two rects (cells themselves don't render)
        assert_eq!(svg.matches(r#"class="ai-shape ai-rect""#).count(), 2);
    }

    #[test]
    fn test_render_grid_labels_render() {
        let svg = render(
            r#"grid g [cols: 2, rows: 2, cell_width: 40, cell_height: 40,
                      col_labels: ["A","B"], row_labels: ["one","two"]] {
                rect [at: [0,0]]
            }"#,
        )
        .unwrap();
        for label in ["A", "B", "one", "two"] {
            assert!(svg.contains(&format!(">{}</text>", label)), "missing label {label}");
        }
    }

    #[test]
    fn test_render_grid_cell_addressing_in_constraint() {
        // g.cell(r,c) resolves as a constraint reference and moves the target.
        let svg = render(
            r#"
            grid g [cols: 3, rows: 3, cell_width: 40, cell_height: 40] {
                rect [at: [1,1], fill: #111]
            }
            rect marker [width: 6, height: 6, fill: #f00]
            constrain marker.center_x = g.cell(1,1).center_x
            constrain marker.center_y = g.cell(1,1).center_y
        "#,
        )
        .unwrap();
        // Renders without undefined-identifier errors
        assert!(svg.contains(r#"id="marker""#));
    }

    #[test]
    fn test_render_escaped_quotes_in_text_are_unescaped() {
        // `\"` in a label must render as a real quote (XML-escaped), not a literal backslash.
        let svg = render(r#"text "what does \"it\" refer to?" cap"#).unwrap();
        assert!(svg.contains("&quot;it&quot;"), "got: {svg}");
        assert!(!svg.contains(r#"\"#), "backslash leaked into output: {svg}");
    }

    #[test]
    fn test_render_contains_accepts_grid_cells() {
        // `contains` must accept grid.cell(r,c) refs (e.g. a highlight box around a row).
        let svg = render(
            r#"
            grid g [cols: 3, rows: 3, cell_width: 40, cell_height: 40] {
                rect [at: [1,0]]
                rect [at: [1,2]]
            }
            rect hl [fill: accent-1, fill_opacity: 0.2]
            constrain hl contains g.cell(1,0), g.cell(1,2) [padding: 4]
        "#,
        )
        .unwrap();
        assert!(svg.contains(r#"id="hl""#));
    }

    #[test]
    fn test_render_callout_point_constraint_aims_tip_at_cell() {
        // tag.tip = g.cell(1,1).top - 4 must move the callout (not the cell).
        let svg = render(
            r#"
            grid g [cols: 3, rows: 3, cell_width: 40, cell_height: 40] {
                rect [at: [1,1], fill: secondary-1]
            }
            callout tag [label: "x", pointer: down]
            constrain tag.tip = g.cell(1,1).top - 4
        "#,
        )
        .unwrap();
        // Callout path present; layout resolved (no panic / error)
        assert!(svg.contains(r#"id="tag""#));
    }

    #[test]
    fn test_render_attention_heatmap_fixture() {
        // Acceptance-style triangular heatmap with labels + an annotation callout.
        let svg = render(
            r#"
            grid heat [cols: 6, rows: 6, gap: 5, cell_width: 56, cell_height: 56,
                       col_labels: ["The","cat","sat","because","it","was"],
                       row_labels: ["The","cat","sat","because","it","was"]] {
                rect [at: [0,0], fill: secondary-1, fill_opacity: 1.00]
                rect [at: [1,0], fill: secondary-1, fill_opacity: 0.30]
                rect [at: [1,1], fill: secondary-1, fill_opacity: 0.70]
                rect [at: [2,0], fill: secondary-1, fill_opacity: 0.20]
                rect [at: [2,1], fill: secondary-1, fill_opacity: 0.40]
                rect [at: [2,2], fill: secondary-1, fill_opacity: 0.90]
            }
            callout tag [label: "= subject", pointer: down, stroke: accent-1, fill: background-1]
            constrain tag.tip = heat.cell(1,1).top - 4
        "#,
        )
        .unwrap();
        // Triangular fill gradient, labels, and the callout all present
        assert!(svg.contains(r#"fill-opacity="0.7""#));
        assert!(svg.contains(r#"fill-opacity="0.2""#));
        assert!(svg.contains(">because</text>"));
        assert!(svg.contains("= subject"));
        assert!(svg.contains(r#"id="tag""#));
    }

    #[test]
    fn test_render_fill_opacity_clamped_high() {
        // Values above 1 clamp to 1
        let svg = render(r#"rect cell [fill: #ff0000, fill_opacity: 1.5]"#).unwrap();
        assert!(svg.contains(r#"fill-opacity="1""#));
    }

    #[test]
    fn test_render_heatmap_fill_opacity_gradient() {
        // Triangular heatmap: same hue, descending fill_opacity, renders and round-trips
        let svg = render(
            r#"
            col {
                row { rect c1 [fill: secondary-1, fill_opacity: 0.9] }
                row {
                    rect c2 [fill: secondary-1, fill_opacity: 0.6]
                    rect c3 [fill: secondary-1, fill_opacity: 0.6]
                }
                row {
                    rect c4 [fill: secondary-1, fill_opacity: 0.3]
                    rect c5 [fill: secondary-1, fill_opacity: 0.3]
                    rect c6 [fill: secondary-1, fill_opacity: 0.3]
                }
            }
        "#,
        )
        .unwrap();
        assert!(svg.contains(r#"fill="var(--secondary-1)""#));
        assert!(svg.contains(r#"fill-opacity="0.9""#));
        assert!(svg.contains(r#"fill-opacity="0.6""#));
        assert!(svg.contains(r#"fill-opacity="0.3""#));
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
