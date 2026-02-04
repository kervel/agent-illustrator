//! SVG generation from layout results

use crate::layout::{
    BoundingBox, ConnectionLayout, ElementLayout, ElementType, LayoutResult, Point, ResolvedStyles,
    RoutingMode, TextAnchor,
};
use crate::parser::ast::{ConnectionDirection, ShapeType};
use crate::stylesheet::Stylesheet;

use super::SvgConfig;

/// Build SVG elements incrementally
pub struct SvgBuilder {
    config: SvgConfig,
    defs: Vec<String>,
    styles: Vec<String>,
    elements: Vec<String>,
    connections: Vec<String>,
    indent: usize,
}

impl SvgBuilder {
    /// Create a new SVG builder
    pub fn new(config: SvgConfig) -> Self {
        Self {
            config,
            defs: vec![],
            styles: vec![],
            elements: vec![],
            connections: vec![],
            indent: 1,
        }
    }

    /// Add CSS custom properties from a stylesheet
    pub fn add_stylesheet(&mut self, stylesheet: &Stylesheet) {
        let mut css = String::from(":root {\n");
        for (token, value) in &stylesheet.colors {
            css.push_str(&format!("    --{}: {};\n", token, value));
        }
        css.push_str("  }\n");
        // Apply font-family to text elements if defined
        if stylesheet.colors.contains_key("font-family") {
            let prefix = self.prefix();
            css.push_str(&format!(
                "  .{}label, .{}text {{ font-family: var(--font-family); }}",
                prefix, prefix
            ));
        }
        self.styles.push(css);
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
        if self.config.pretty_print {
            "\n"
        } else {
            ""
        }
    }

    /// Add the arrow marker definition for directed connections
    pub fn add_arrow_marker(&mut self) {
        let prefix = self.prefix();
        // Use orient="auto" to automatically rotate the marker to match path direction
        // at the marker position. The arrow shape points right (+X), so it will
        // rotate to match the final segment direction (e.g., down for vertical paths).
        // Use fill="context-stroke" so the arrow inherits the line's stroke color.
        // Use markerUnits="strokeWidth" so arrow size scales with line thickness.
        self.defs.push(format!(
            r#"<marker id="{prefix}arrow" viewBox="0 0 10 10" refX="1" refY="5" markerWidth="4" markerHeight="4" markerUnits="strokeWidth" orient="auto">
      <path d="M0,0 L10,5 L0,10 Z" fill="context-stroke"/>
    </marker>"#
        ));
    }

    /// Add a rectangle element
    pub fn add_rect(
        &mut self,
        id: Option<&str>,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        classes: &[String],
        styles: &str,
    ) {
        let prefix = self.prefix();
        let id_attr = id.map(|i| format!(r#" id="{}""#, i)).unwrap_or_default();
        let class_list = std::iter::once(format!("{}shape", prefix))
            .chain(std::iter::once(format!("{}rect", prefix)))
            .chain(classes.iter().cloned())
            .collect::<Vec<_>>()
            .join(" ");

        self.elements.push(format!(
            r#"{}<rect{} class="{}" x="{}" y="{}" width="{}" height="{}"{}/>"#,
            self.indent_str(),
            id_attr,
            class_list,
            x,
            y,
            w,
            h,
            styles
        ));
    }

    /// Add a debug rectangle with dashed border and tiny label
    pub fn add_debug_rect(&mut self, x: f64, y: f64, w: f64, h: f64, label: &str) {
        // Dashed magenta rectangle
        self.elements.push(format!(
            r##"{}<rect x="{}" y="{}" width="{}" height="{}" fill="none" stroke="#ff00ff" stroke-width="0.5" stroke-dasharray="2,2" opacity="0.7"/>"##,
            self.indent_str(),
            x, y, w, h
        ));
        // Tiny label at top-left
        if !label.is_empty() {
            self.elements.push(format!(
                r##"{}<text x="{}" y="{}" font-size="6" fill="#ff00ff" opacity="0.8">{}</text>"##,
                self.indent_str(),
                x + 1.0,
                y + 6.0,
                label
            ));
        }
    }

    /// Add a circle element
    pub fn add_circle(
        &mut self,
        id: Option<&str>,
        cx: f64,
        cy: f64,
        r: f64,
        classes: &[String],
        styles: &str,
    ) {
        let prefix = self.prefix();
        let id_attr = id.map(|i| format!(r#" id="{}""#, i)).unwrap_or_default();
        let class_list = std::iter::once(format!("{}shape", prefix))
            .chain(std::iter::once(format!("{}circle", prefix)))
            .chain(classes.iter().cloned())
            .collect::<Vec<_>>()
            .join(" ");

        self.elements.push(format!(
            r#"{}<circle{} class="{}" cx="{}" cy="{}" r="{}"{}/>"#,
            self.indent_str(),
            id_attr,
            class_list,
            cx,
            cy,
            r,
            styles
        ));
    }

    /// Add an ellipse element
    pub fn add_ellipse(
        &mut self,
        id: Option<&str>,
        cx: f64,
        cy: f64,
        rx: f64,
        ry: f64,
        classes: &[String],
        styles: &str,
    ) {
        let prefix = self.prefix();
        let id_attr = id.map(|i| format!(r#" id="{}""#, i)).unwrap_or_default();
        let class_list = std::iter::once(format!("{}shape", prefix))
            .chain(std::iter::once(format!("{}ellipse", prefix)))
            .chain(classes.iter().cloned())
            .collect::<Vec<_>>()
            .join(" ");

        self.elements.push(format!(
            r#"{}<ellipse{} class="{}" cx="{}" cy="{}" rx="{}" ry="{}"{}/>"#,
            self.indent_str(),
            id_attr,
            class_list,
            cx,
            cy,
            rx,
            ry,
            styles
        ));
    }

    /// Add a polygon element
    pub fn add_polygon(
        &mut self,
        id: Option<&str>,
        points: &[Point],
        classes: &[String],
        styles: &str,
    ) {
        let prefix = self.prefix();
        let id_attr = id.map(|i| format!(r#" id="{}""#, i)).unwrap_or_default();
        let class_list = std::iter::once(format!("{}shape", prefix))
            .chain(std::iter::once(format!("{}polygon", prefix)))
            .chain(classes.iter().cloned())
            .collect::<Vec<_>>()
            .join(" ");

        let points_str: String = points
            .iter()
            .map(|p| format!("{},{}", p.x, p.y))
            .collect::<Vec<_>>()
            .join(" ");

        self.elements.push(format!(
            r#"{}<polygon{} class="{}" points="{}"{}/>"#,
            self.indent_str(),
            id_attr,
            class_list,
            points_str,
            styles
        ));
    }

    /// Add a path element with custom d attribute (Feature 007)
    pub fn add_path(&mut self, id: Option<&str>, d: &str, classes: &[String], styles: &str) {
        let prefix = self.prefix();
        let id_attr = id.map(|i| format!(r#" id="{}""#, i)).unwrap_or_default();
        let class_list = std::iter::once(format!("{}shape", prefix))
            .chain(std::iter::once(format!("{}path", prefix)))
            .chain(classes.iter().cloned())
            .collect::<Vec<_>>()
            .join(" ");

        self.elements.push(format!(
            r#"{}<path{} class="{}" d="{}"{}/>"#,
            self.indent_str(),
            id_attr,
            class_list,
            d,
            if styles.is_empty() {
                String::new()
            } else {
                format!(" {}", styles)
            }
        ));
    }

    /// Add a line element
    pub fn add_line(
        &mut self,
        id: Option<&str>,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        classes: &[String],
        styles: &str,
    ) {
        let prefix = self.prefix();
        let id_attr = id.map(|i| format!(r#" id="{}""#, i)).unwrap_or_default();
        let class_list = std::iter::once(format!("{}shape", prefix))
            .chain(std::iter::once(format!("{}line", prefix)))
            .chain(classes.iter().cloned())
            .collect::<Vec<_>>()
            .join(" ");

        self.elements.push(format!(
            r#"{}<line{} class="{}" x1="{}" y1="{}" x2="{}" y2="{}"{}/>"#,
            self.indent_str(),
            id_attr,
            class_list,
            x1,
            y1,
            x2,
            y2,
            styles
        ));
    }

    /// Add a text element
    pub fn add_text(&mut self, text: &str, x: f64, y: f64, anchor: &TextAnchor, styles: &str) {
        let prefix = self.prefix();
        let anchor_str = match anchor {
            TextAnchor::Start => "start",
            TextAnchor::Middle => "middle",
            TextAnchor::End => "end",
        };

        self.elements.push(format!(
            r#"{}<text class="{}label" x="{}" y="{}" text-anchor="{}" dominant-baseline="middle"{}>{}</text>"#,
            self.indent_str(),
            prefix,
            x,
            y,
            anchor_str,
            styles,
            escape_xml(text)
        ));
    }

    /// Add a text shape element (with id, classes, and dominant-baseline for vertical centering)
    pub fn add_text_element(
        &mut self,
        id: Option<&str>,
        text: &str,
        x: f64,
        y: f64,
        anchor: &TextAnchor,
        classes: &[String],
        styles: &str,
    ) {
        let prefix = self.prefix();
        let id_attr = id.map(|i| format!(r#" id="{}""#, i)).unwrap_or_default();
        let anchor_str = match anchor {
            TextAnchor::Start => "start",
            TextAnchor::Middle => "middle",
            TextAnchor::End => "end",
        };
        let class_list = std::iter::once(format!("{}shape", prefix))
            .chain(std::iter::once(format!("{}text", prefix)))
            .chain(classes.iter().cloned())
            .collect::<Vec<_>>()
            .join(" ");

        self.elements.push(format!(
            r#"{}<text{} class="{}" x="{}" y="{}" text-anchor="{}" dominant-baseline="middle"{}>{}</text>"#,
            self.indent_str(),
            id_attr,
            class_list,
            x,
            y,
            anchor_str,
            styles,
            escape_xml(text)
        ));
    }

    /// Add a path for a connection
    pub fn add_connection_path(
        &mut self,
        path: &[Point],
        routing_mode: RoutingMode,
        classes: &[String],
        styles: &str,
        marker_end: bool,
        stroke_width: f64,
    ) {
        let prefix = self.prefix();
        let class_list = std::iter::once(format!("{}connection", prefix))
            .chain(classes.iter().cloned())
            .collect::<Vec<_>>()
            .join(" ");

        // Shorten endpoint when marker is present to place arrow tip at anchor position
        // The arrow marker has refX=1, so the arrow extends ~9 marker units past the endpoint.
        // With markerWidth=4 and markerUnits="strokeWidth", each marker unit = (4 * strokeWidth) / 10.
        // So pullback = 9 * (4/10) * strokeWidth = 3.6 * strokeWidth.
        let path = if marker_end && path.len() >= 2 {
            let mut shortened = path.to_vec();
            let last_idx = shortened.len() - 1;
            let prev_idx = last_idx - 1;

            // Calculate tangent direction at endpoint
            let dx = shortened[last_idx].x - shortened[prev_idx].x;
            let dy = shortened[last_idx].y - shortened[prev_idx].y;
            let len = (dx * dx + dy * dy).sqrt();

            if len > 0.001 {
                // Pull back to compensate for arrow length (scales with stroke width)
                let pullback = 3.6 * stroke_width;
                shortened[last_idx].x -= dx / len * pullback;
                shortened[last_idx].y -= dy / len * pullback;
            }
            shortened
        } else {
            path.to_vec()
        };

        // Generate path data based on routing mode
        let d = match routing_mode {
            RoutingMode::Curved if path.len() >= 4 => {
                // Cubic Bezier: M start C control1 control2 end [S control2 end]...
                let mut d = format!(
                    "M{} {} C{} {} {} {} {} {}",
                    path[0].x,
                    path[0].y,
                    path[1].x,
                    path[1].y,
                    path[2].x,
                    path[2].y,
                    path[3].x,
                    path[3].y
                );
                // Additional segments use C (cubic Bezier with explicit control points)
                // Each additional segment needs 3 points: ctrl1, ctrl2, endpoint
                let remaining = &path[4..];
                for chunk in remaining.chunks(3) {
                    if chunk.len() == 3 {
                        d.push_str(&format!(
                            " C{} {} {} {} {} {}",
                            chunk[0].x, chunk[0].y, chunk[1].x, chunk[1].y,
                            chunk[2].x, chunk[2].y
                        ));
                    } else if chunk.len() == 2 {
                        // Fallback: 2 points as quadratic Bezier
                        d.push_str(&format!(
                            " Q{} {} {} {}",
                            chunk[0].x, chunk[0].y, chunk[1].x, chunk[1].y
                        ));
                    } else if chunk.len() == 1 {
                        // Odd point at end - just draw line to it
                        d.push_str(&format!(" L{} {}", chunk[0].x, chunk[0].y));
                    }
                }
                d
            }
            _ => path_to_d(&path), // Default polyline for orthogonal/direct
        };

        let marker = if marker_end {
            format!(r#" marker-end="url(#{prefix}arrow)""#)
        } else {
            String::new()
        };

        self.connections.push(format!(
            r#"{}<path class="{}" d="{}" fill="none"{}{}/>"#,
            self.indent_str(),
            class_list,
            d,
            styles,
            marker
        ));
    }

    /// Add a group element with optional ID and classes
    pub fn start_group(&mut self, id: Option<&str>, classes: &[String]) {
        let id_attr = id.map(|i| format!(r#" id="{}""#, i)).unwrap_or_default();
        let class_attr = if classes.is_empty() {
            String::new()
        } else {
            format!(r#" class="{}""#, classes.join(" "))
        };

        self.elements
            .push(format!("{}<g{}{}>", self.indent_str(), id_attr, class_attr));
        self.indent += 1;
    }

    /// Close a group element
    pub fn end_group(&mut self) {
        self.indent = self.indent.saturating_sub(1);
        self.elements.push(format!("{}</g>", self.indent_str()));
    }

    /// Add a group element with optional ID, classes, and transform
    pub fn start_group_with_transform(
        &mut self,
        id: Option<&str>,
        classes: &[String],
        transform: &str,
    ) {
        let id_attr = id.map(|i| format!(r#" id="{}""#, i)).unwrap_or_default();
        let class_attr = if classes.is_empty() {
            String::new()
        } else {
            format!(r#" class="{}""#, classes.join(" "))
        };
        let transform_attr = if transform.is_empty() {
            String::new()
        } else {
            format!(r#" transform="{}""#, transform)
        };

        self.elements.push(format!(
            "{}<g{}{}{}>",
            self.indent_str(),
            id_attr,
            class_attr,
            transform_attr
        ));
        self.indent += 1;
    }

    /// Add raw SVG content (for embedded SVG templates)
    pub fn add_raw(&mut self, content: &str) {
        // Split content into lines and add with proper indentation
        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                self.elements
                    .push(format!("{}{}", self.indent_str(), trimmed));
            }
        }
    }

    /// Build the final SVG string
    pub fn build(self, viewbox: BoundingBox) -> String {
        let padding = self.config.viewbox_padding;
        let vb_x = viewbox.x - padding;
        let vb_y = viewbox.y - padding;
        let vb_w = viewbox.width + 2.0 * padding;
        let vb_h = viewbox.height + 2.0 * padding;

        let nl = self.newline();

        let mut svg = String::new();

        // XML declaration for standalone
        if self.config.standalone {
            svg.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
            svg.push_str(nl);
        }

        // SVG root element
        svg.push_str(&format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{} {} {} {}">"#,
            vb_x, vb_y, vb_w, vb_h
        ));
        svg.push_str(nl);

        // Style section for CSS custom properties
        if !self.styles.is_empty() {
            svg.push_str("  <style>");
            svg.push_str(nl);
            for style in &self.styles {
                svg.push_str("    ");
                svg.push_str(style);
                svg.push_str(nl);
            }
            svg.push_str("  </style>");
            svg.push_str(nl);
        }

        // Defs section if needed
        if !self.defs.is_empty() {
            svg.push_str("  <defs>");
            svg.push_str(nl);
            for def in &self.defs {
                svg.push_str("    ");
                svg.push_str(def);
                svg.push_str(nl);
            }
            svg.push_str("  </defs>");
            svg.push_str(nl);
        }

        // Elements
        for elem in &self.elements {
            svg.push_str(elem);
            svg.push_str(nl);
        }

        // Connections (rendered on top)
        for conn in &self.connections {
            svg.push_str(conn);
            svg.push_str(nl);
        }

        svg.push_str("</svg>");

        svg
    }
}

/// Render a LayoutResult to an SVG string (with default stylesheet)
pub fn render_svg(result: &LayoutResult, config: &SvgConfig) -> String {
    render_svg_with_stylesheet(result, config, &Stylesheet::default(), false)
}

/// Render a LayoutResult to an SVG string with a custom stylesheet
pub fn render_svg_with_stylesheet(
    result: &LayoutResult,
    config: &SvgConfig,
    stylesheet: &Stylesheet,
    debug: bool,
) -> String {
    let mut builder = SvgBuilder::new(config.clone());

    // Add CSS custom properties from the stylesheet
    builder.add_stylesheet(stylesheet);

    // Add arrow marker if there are any directed connections
    let has_directed = result.connections.iter().any(|c| {
        matches!(
            c.direction,
            ConnectionDirection::Forward | ConnectionDirection::Backward
        )
    });
    if has_directed {
        builder.add_arrow_marker();
    }

    // Render all root elements
    for element in &result.root_elements {
        render_element(element, &mut builder);
    }

    // Render all connections
    for conn in &result.connections {
        render_connection(conn, &mut builder);
    }

    // Render debug overlays
    if debug {
        for element in &result.root_elements {
            render_debug_bounds(element, &mut builder);
        }
    }

    builder.build(result.bounds)
}

/// Render debug bounds for an element and its children
fn render_debug_bounds(element: &ElementLayout, builder: &mut SvgBuilder) {
    let b = &element.bounds;
    let id = element.id.as_ref().map(|i| i.0.as_str()).unwrap_or("");

    // Draw dashed rectangle for bounds
    builder.add_debug_rect(b.x, b.y, b.width, b.height, id);

    // Recurse into children
    for child in &element.children {
        render_debug_bounds(child, builder);
    }
}

/// Wrap shape rendering with rotation transform if needed
fn render_shape_with_rotation<F>(element: &ElementLayout, builder: &mut SvgBuilder, render_fn: F)
where
    F: FnOnce(&mut SvgBuilder),
{
    if let Some(rotation) = element.styles.rotation {
        if rotation.abs() > f64::EPSILON {
            let center = element.bounds.center();
            let transform = format!("rotate({} {} {})", rotation, center.x, center.y);
            builder.start_group_with_transform(None, &[], &transform);
            render_fn(builder);
            builder.end_group();
        } else {
            render_fn(builder);
        }
    } else {
        render_fn(builder);
    }
}

/// Render a single element to the builder
fn render_element(element: &ElementLayout, builder: &mut SvgBuilder) {
    let id = element.id.as_ref().map(|i| i.0.as_str());
    let styles = format_styles(&element.styles);
    let classes = element.styles.css_classes.clone();

    match &element.element_type {
        ElementType::Shape(ShapeType::Rectangle) => {
            render_shape_with_rotation(element, builder, |b| {
                b.add_rect(
                    id,
                    element.bounds.x,
                    element.bounds.y,
                    element.bounds.width,
                    element.bounds.height,
                    &classes,
                    &styles,
                );
            });
        }
        ElementType::Shape(ShapeType::Circle) => {
            let r = element.bounds.width.min(element.bounds.height) / 2.0;
            render_shape_with_rotation(element, builder, |b| {
                b.add_circle(
                    id,
                    element.bounds.x + r,
                    element.bounds.y + r,
                    r,
                    &classes,
                    &styles,
                );
            });
        }
        ElementType::Shape(ShapeType::Ellipse) => {
            render_shape_with_rotation(element, builder, |b| {
                b.add_ellipse(
                    id,
                    element.bounds.x + element.bounds.width / 2.0,
                    element.bounds.y + element.bounds.height / 2.0,
                    element.bounds.width / 2.0,
                    element.bounds.height / 2.0,
                    &classes,
                    &styles,
                );
            });
        }
        ElementType::Shape(ShapeType::Polygon) => {
            // Default to a diamond shape for polygon
            let b = &element.bounds;
            let points = vec![
                Point::new(b.x + b.width / 2.0, b.y),
                Point::new(b.right(), b.y + b.height / 2.0),
                Point::new(b.x + b.width / 2.0, b.bottom()),
                Point::new(b.x, b.y + b.height / 2.0),
            ];
            render_shape_with_rotation(element, builder, |b| {
                b.add_polygon(id, &points, &classes, &styles);
            });
        }
        ElementType::Shape(ShapeType::Line) => {
            render_shape_with_rotation(element, builder, |b| {
                b.add_line(
                    id,
                    element.bounds.x,
                    element.bounds.y + element.bounds.height / 2.0,
                    element.bounds.right(),
                    element.bounds.y + element.bounds.height / 2.0,
                    &classes,
                    &styles,
                );
            });
        }
        ElementType::Shape(ShapeType::Icon { icon_name }) => {
            // For icons, render a placeholder rect with the icon name as text
            render_shape_with_rotation(element, builder, |b| {
                b.add_rect(
                    id,
                    element.bounds.x,
                    element.bounds.y,
                    element.bounds.width,
                    element.bounds.height,
                    &classes,
                    &styles,
                );
                // Add icon name as a label
                b.add_text(
                    icon_name,
                    element.bounds.x + element.bounds.width / 2.0,
                    element.bounds.y + element.bounds.height / 2.0,
                    &TextAnchor::Middle,
                    "",
                );
            });
        }
        ElementType::Shape(ShapeType::Text { content }) => {
            // Render text element as SVG text
            // Position text at the center of bounds, vertically centered using dominant-baseline
            let font_styles = element
                .styles
                .font_size
                .map(|fs| format!(r#" font-size="{}""#, fs))
                .unwrap_or_default();
            let fill_style = element
                .styles
                .fill
                .as_ref()
                .map(|f| format!(r#" fill="{}""#, f))
                .unwrap_or_default();
            let combined_styles = format!("{}{}", font_styles, fill_style);
            render_shape_with_rotation(element, builder, |b| {
                b.add_text_element(
                    id,
                    content,
                    element.bounds.x,
                    element.bounds.y + element.bounds.height / 2.0,
                    &TextAnchor::Start,
                    &classes,
                    &combined_styles,
                );
            });
        }
        ElementType::Shape(ShapeType::SvgEmbed {
            content,
            intrinsic_width,
            intrinsic_height,
        }) => {
            // Render embedded SVG content from a template
            let prefix = builder.prefix();
            let embed_classes = std::iter::once(format!("{}svg-embed", prefix))
                .chain(classes.iter().cloned())
                .collect::<Vec<_>>();

            // Calculate scale factors
            let scale_x = intrinsic_width
                .map(|w| element.bounds.width / w)
                .unwrap_or(1.0);
            let scale_y = intrinsic_height
                .map(|h| element.bounds.height / h)
                .unwrap_or(1.0);

            // Create group with transform for positioning, scaling, and optional rotation
            // SVG transforms apply right-to-left, so: rotate around center, then scale, then translate
            let transform = if let Some(rotation) = element.styles.rotation {
                if rotation.abs() > f64::EPSILON {
                    let cx = intrinsic_width.unwrap_or(element.bounds.width) / 2.0;
                    let cy = intrinsic_height.unwrap_or(element.bounds.height) / 2.0;
                    format!(
                        "translate({}, {}) scale({}, {}) rotate({} {} {})",
                        element.bounds.x, element.bounds.y, scale_x, scale_y, rotation, cx, cy
                    )
                } else {
                    format!(
                        "translate({}, {}) scale({}, {})",
                        element.bounds.x, element.bounds.y, scale_x, scale_y
                    )
                }
            } else {
                format!(
                    "translate({}, {}) scale({}, {})",
                    element.bounds.x, element.bounds.y, scale_x, scale_y
                )
            };

            builder.start_group_with_transform(id, &embed_classes, &transform);

            // Strip SVG wrapper and embed inner content
            let inner = strip_svg_wrapper(content);
            builder.add_raw(&inner);

            builder.end_group();
        }
        ElementType::Shape(ShapeType::Path(path_decl)) => {
            // Path shape rendering (Feature 007)
            let origin = Point::new(element.bounds.x, element.bounds.y);
            let resolved =
                super::path::resolve_path_with_options(path_decl, origin, element.path_normalize);
            let d = resolved.to_svg_d();

            if d.is_empty() {
                // Empty path - render nothing
                return;
            }

            render_shape_with_rotation(element, builder, |b| {
                b.add_path(id, &d, &classes, &styles);
            });
        }
        ElementType::Layout(_) | ElementType::Group => {
            // Start a group for containers (with optional rotation)
            let prefix = builder.prefix();
            let container_classes = std::iter::once(format!("{}container", prefix))
                .chain(classes.iter().cloned())
                .collect::<Vec<_>>();
            if let Some(rotation) = element.styles.rotation {
                if rotation.abs() > f64::EPSILON {
                    let center = element.bounds.center();
                    let transform = format!("rotate({} {} {})", rotation, center.x, center.y);
                    builder.start_group_with_transform(id, &container_classes, &transform);
                } else {
                    builder.start_group(id, &container_classes);
                }
            } else {
                builder.start_group(id, &container_classes);
            }

            // Render children
            for child in &element.children {
                render_element(child, builder);
            }

            builder.end_group();
        }
    }

    // Render label if present
    if let Some(label) = &element.label {
        let font_styles = element
            .styles
            .font_size
            .map(|fs| format!(r#" font-size="{}""#, fs))
            .unwrap_or_default();
        builder.add_text(
            &label.text,
            label.position.x,
            label.position.y,
            &label.anchor,
            &font_styles,
        );
    }
}

/// Render a connection to the builder
fn render_connection(conn: &ConnectionLayout, builder: &mut SvgBuilder) {
    let classes = conn.styles.css_classes.clone();
    let styles = format_connection_styles(&conn.styles);

    // Get stroke width for arrow pullback calculation (default: 2.0)
    let stroke_width = conn.styles.stroke_width.unwrap_or(2.0);

    let marker_end = matches!(
        conn.direction,
        ConnectionDirection::Forward | ConnectionDirection::Bidirectional
    );

    builder.add_connection_path(
        &conn.path,
        conn.routing_mode,
        &classes,
        &styles,
        marker_end,
        stroke_width,
    );

    // Render connection label if present
    if let Some(label) = &conn.label {
        // Use label's own styles if available (from referenced element),
        // otherwise apply subtle defaults for connector labels
        let label_styles = label
            .styles
            .as_ref()
            .map(format_text_styles)
            .unwrap_or_else(|| r#" fill="var(--text-2)" font-size="12""#.to_string());
        builder.add_text(
            &label.text,
            label.position.x,
            label.position.y,
            &label.anchor,
            &label_styles,
        );
    }
}

/// Format connection styles (stroke-focused, no fill)
fn format_connection_styles(styles: &ResolvedStyles) -> String {
    let mut parts = vec![];
    if let Some(stroke) = &styles.stroke {
        parts.push(format!(r#" stroke="{}""#, stroke));
    } else {
        parts.push(r##" stroke="#333""##.to_string());
    }
    if let Some(sw) = styles.stroke_width {
        parts.push(format!(r#" stroke-width="{}""#, sw));
    } else {
        parts.push(r#" stroke-width="2""#.to_string());
    }
    if let Some(dash) = &styles.stroke_dasharray {
        parts.push(format!(r#" stroke-dasharray="{}""#, dash));
    }
    parts.join("")
}

/// Format text styles (fill and font_size for labels)
fn format_text_styles(styles: &ResolvedStyles) -> String {
    let mut parts = vec![];
    if let Some(fill) = &styles.fill {
        parts.push(format!(r#"fill="{}""#, fill));
    }
    if let Some(font_size) = styles.font_size {
        parts.push(format!(r#"font-size="{}""#, font_size));
    }
    if !parts.is_empty() {
        // Add leading space so it can be appended to existing attributes
        format!(" {}", parts.join(" "))
    } else {
        String::new()
    }
}

/// Format ResolvedStyles as SVG attribute string
/// Applies sensible defaults when styles are not specified
fn format_styles(styles: &ResolvedStyles) -> String {
    let mut parts = vec![];

    // Default fill: light gray for visibility
    let fill = styles.fill.as_deref().unwrap_or("#f0f0f0");
    parts.push(format!(r#" fill="{}""#, fill));

    // Default stroke: dark gray
    let stroke = styles.stroke.as_deref().unwrap_or("#333333");
    parts.push(format!(r#" stroke="{}""#, stroke));

    // Default stroke-width: 1.5
    let sw = styles.stroke_width.unwrap_or(1.5);
    parts.push(format!(r#" stroke-width="{}""#, sw));
    if let Some(dash) = &styles.stroke_dasharray {
        parts.push(format!(r#" stroke-dasharray="{}""#, dash));
    }
    if let Some(op) = styles.opacity {
        if op < 1.0 {
            parts.push(format!(r#" opacity="{}""#, op));
        }
    }
    parts.join("")
}

/// Convert a path of points to an SVG path d attribute
fn path_to_d(path: &[Point]) -> String {
    if path.is_empty() {
        return String::new();
    }

    let mut d = format!("M{} {}", path[0].x, path[0].y);
    for point in &path[1..] {
        d.push_str(&format!(" L{} {}", point.x, point.y));
    }
    d
}

/// Escape special XML characters
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Strip the outer SVG wrapper from embedded SVG content
///
/// Removes the XML declaration, DOCTYPE, and outer <svg> tags,
/// returning only the inner content (paths, shapes, etc.)
fn strip_svg_wrapper(svg: &str) -> String {
    let mut result = svg.trim().to_string();

    // Remove XML declaration: <?xml ... ?>
    if let Some(start) = result.find("<?xml") {
        if let Some(end) = result[start..].find("?>") {
            result = result[start + end + 2..].trim().to_string();
        }
    }

    // Remove DOCTYPE
    if let Some(start) = result.find("<!DOCTYPE") {
        if let Some(end) = result[start..].find('>') {
            result = result[start + end + 1..].trim().to_string();
        }
    }

    // Remove outer <svg ...> tag
    if let Some(start) = result.find("<svg") {
        if let Some(end) = result[start..].find('>') {
            result = result[start + end + 1..].to_string();
        }
    }

    // Remove closing </svg> tag
    if let Some(pos) = result.rfind("</svg>") {
        result = result[..pos].trim().to_string();
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{AnchorSet, ElementType, LayoutResult, ResolvedStyles};
    use crate::parser::ast::{Identifier, LayoutType};

    #[test]
    fn test_path_to_d() {
        let path = vec![
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            Point::new(100.0, 100.0),
        ];
        let d = path_to_d(&path);
        assert_eq!(d, "M0 0 L100 0 L100 100");
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("a < b"), "a &lt; b");
        assert_eq!(escape_xml("a & b"), "a &amp; b");
        assert_eq!(escape_xml("<tag>"), "&lt;tag&gt;");
    }

    #[test]
    fn test_format_styles() {
        let styles = ResolvedStyles {
            fill: Some("#ff0000".to_string()),
            stroke: Some("#000000".to_string()),
            stroke_width: Some(2.0),
            stroke_dasharray: Some("4,2".to_string()),
            opacity: Some(0.5),
            font_size: None,
            css_classes: vec![],
            rotation: None,
        };
        let result = format_styles(&styles);
        assert!(result.contains(r##"fill="#ff0000""##));
        assert!(result.contains(r##"stroke="#000000""##));
        assert!(result.contains(r#"stroke-width="2""#));
        assert!(result.contains(r#"stroke-dasharray="4,2""#));
        assert!(result.contains(r#"opacity="0.5""#));
    }

    #[test]
    fn test_render_single_rect() {
        let mut result = LayoutResult::new();
        result.add_element(ElementLayout {
            id: Some(Identifier::new("box")),
            element_type: ElementType::Shape(ShapeType::Rectangle),
            bounds: BoundingBox::new(0.0, 0.0, 100.0, 50.0),
            styles: ResolvedStyles::default(),
            children: vec![],
            label: None,
            anchors: AnchorSet::default(),
            path_normalize: true,
        });
        result.compute_bounds();

        let config = SvgConfig::default();
        let svg = render_svg(&result, &config);

        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains(r#"id="box""#));
        assert!(svg.contains("ai-rect"));
    }

    #[test]
    fn test_render_with_connection() {
        let mut result = LayoutResult::new();
        result.add_element(ElementLayout {
            id: Some(Identifier::new("a")),
            element_type: ElementType::Shape(ShapeType::Rectangle),
            bounds: BoundingBox::new(0.0, 0.0, 50.0, 50.0),
            styles: ResolvedStyles::default(),
            children: vec![],
            label: None,
            anchors: AnchorSet::default(),
            path_normalize: true,
        });
        result.add_element(ElementLayout {
            id: Some(Identifier::new("b")),
            element_type: ElementType::Shape(ShapeType::Rectangle),
            bounds: BoundingBox::new(100.0, 0.0, 50.0, 50.0),
            styles: ResolvedStyles::default(),
            children: vec![],
            label: None,
            anchors: AnchorSet::default(),
            path_normalize: true,
        });
        result.connections.push(ConnectionLayout {
            from_id: Identifier::new("a"),
            to_id: Identifier::new("b"),
            direction: ConnectionDirection::Forward,
            path: vec![Point::new(50.0, 25.0), Point::new(100.0, 25.0)],
            styles: ResolvedStyles::default(),
            label: None,
            routing_mode: RoutingMode::default(),
        });
        result.compute_bounds();

        let config = SvgConfig::default();
        let svg = render_svg(&result, &config);

        assert!(svg.contains("<defs>"));
        assert!(svg.contains("ai-arrow"));
        assert!(svg.contains("ai-connection"));
        assert!(svg.contains("marker-end"));
    }

    #[test]
    fn test_render_nested_layout() {
        let mut result = LayoutResult::new();
        result.add_element(ElementLayout {
            id: Some(Identifier::new("container")),
            element_type: ElementType::Layout(LayoutType::Row),
            bounds: BoundingBox::new(0.0, 0.0, 200.0, 70.0),
            styles: ResolvedStyles::default(),
            children: vec![
                ElementLayout {
                    id: Some(Identifier::new("a")),
                    element_type: ElementType::Shape(ShapeType::Rectangle),
                    bounds: BoundingBox::new(10.0, 10.0, 50.0, 50.0),
                    styles: ResolvedStyles::default(),
                    children: vec![],
                    label: None,
                    anchors: AnchorSet::default(),
                    path_normalize: true,
                },
                ElementLayout {
                    id: Some(Identifier::new("b")),
                    element_type: ElementType::Shape(ShapeType::Rectangle),
                    bounds: BoundingBox::new(80.0, 10.0, 50.0, 50.0),
                    styles: ResolvedStyles::default(),
                    children: vec![],
                    label: None,
                    anchors: AnchorSet::default(),
                    path_normalize: true,
                },
            ],
            label: None,
            anchors: AnchorSet::default(),
            path_normalize: true,
        });
        result.compute_bounds();

        let config = SvgConfig::default();
        let svg = render_svg(&result, &config);

        assert!(svg.contains("<g"));
        assert!(svg.contains("</g>"));
        assert!(svg.contains("ai-container"));
        assert!(svg.contains(r#"id="a""#));
        assert!(svg.contains(r#"id="b""#));
    }
}
