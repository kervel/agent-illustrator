//! SVG generation from layout results

use crate::layout::{
    BoundingBox, ConnectionLayout, ElementLayout, ElementType, LayoutResult, Point, ResolvedStyles,
    RoutingMode, TextAnchor,
};
use crate::parser::ast::{ConnectionDirection, PointerDir, ShapeType};
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
    /// Frame names for data-frames attribute (Feature 011)
    data_frames: Option<String>,
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
            data_frames: None,
        }
    }

    /// Add raw CSS to the SVG `<style>` block
    pub fn add_custom_css(&mut self, css: &str) {
        self.styles.push(css.to_string());
    }

    /// Add CSS custom properties from a stylesheet
    pub fn add_stylesheet(&mut self, stylesheet: &Stylesheet) {
        if stylesheet.colors.is_empty() {
            return;
        }
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
    #[allow(clippy::too_many_arguments)]
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
    #[allow(clippy::too_many_arguments)]
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
    #[allow(clippy::too_many_arguments)]
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

    /// Add an image element for raster images
    #[allow(clippy::too_many_arguments)]
    pub fn add_image(
        &mut self,
        id: Option<&str>,
        href: &str,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        classes: &[String],
        transform: Option<&str>,
    ) {
        let id_attr = id.map(|i| format!(r#" id="{}""#, i)).unwrap_or_default();
        let class_list = classes.join(" ");
        let transform_attr = transform
            .map(|t| format!(r#" transform="{}""#, t))
            .unwrap_or_default();

        self.elements.push(format!(
            r#"{}<image{} class="{}" href="{}" x="{}" y="{}" width="{}" height="{}"{}/>"#,
            self.indent_str(),
            id_attr,
            class_list,
            href,
            x,
            y,
            width,
            height,
            transform_attr
        ));
    }

    /// Add a text element
    pub fn add_text(&mut self, text: &str, x: f64, y: f64, anchor: &TextAnchor, styles: &str) {
        self.add_text_with_classes(text, x, y, anchor, styles, "");
    }

    /// Add a text label with extra CSS classes appended (e.g. `conn-<name>` so a
    /// connection's label is toggled together with its path by keyframe CSS).
    pub fn add_text_with_classes(
        &mut self,
        text: &str,
        x: f64,
        y: f64,
        anchor: &TextAnchor,
        styles: &str,
        extra_classes: &str,
    ) {
        let prefix = self.prefix();
        let anchor_str = match anchor {
            TextAnchor::Start => "start",
            TextAnchor::Middle => "middle",
            TextAnchor::End => "end",
        };
        let extra = if extra_classes.is_empty() {
            String::new()
        } else {
            format!(" {}", extra_classes)
        };

        self.elements.push(format!(
            r#"{}<text class="{}label{}" x="{}" y="{}" text-anchor="{}" dominant-baseline="middle"{}>{}</text>"#,
            self.indent_str(),
            prefix,
            extra,
            x,
            y,
            anchor_str,
            styles,
            escape_xml(text)
        ));
    }

    /// Add a text shape element (with id, classes, and dominant-baseline for vertical centering)
    #[allow(clippy::too_many_arguments)]
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

        let d = connection_path_d(path, routing_mode, marker_end, stroke_width);

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

    /// Add a group element with opacity (for hiding keyframe elements)
    pub fn start_opacity_group(&mut self, opacity: f64) {
        self.elements.push(format!(
            r#"{}<g opacity="{}">"#,
            self.indent_str(),
            opacity
        ));
        self.indent += 1;
    }

    /// Add a visibility group for keyframe-hidden elements.
    /// Uses a CSS class so frame CSS rules can override visibility.
    pub fn start_visibility_group(&mut self, element_id: &str) {
        self.elements.push(format!(
            r#"{}<g class="kf-hidden kf-{} kf-anim">"#,
            self.indent_str(),
            element_id
        ));
        self.indent += 1;
    }

    /// Add a keyframe class group for elements that start visible but are
    /// toggled by a later keyframe. Carries `kf-{id}` (without `kf-hidden`)
    /// so a later frame's `.kf-{id} { opacity: 0 }` rule has a node to bind to.
    pub fn start_kf_class_group(&mut self, element_id: &str) {
        self.elements.push(format!(
            r#"{}<g class="kf-{} kf-anim">"#,
            self.indent_str(),
            element_id
        ));
        self.indent += 1;
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
        let data_frames_attr = self
            .data_frames
            .as_ref()
            .map(|f| format!(r#" data-frames="{}""#, f))
            .unwrap_or_default();
        svg.push_str(&format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{} {} {} {}"{}>"#,
            vb_x, vb_y, vb_w, vb_h, data_frames_attr
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
    render_svg_with_stylesheet(result, config, &Stylesheet::default(), None, false)
}

/// Render a LayoutResult with keyframe animation CSS (Feature 011)
#[allow(clippy::too_many_arguments)]
pub fn render_svg_with_keyframes(
    result: &LayoutResult,
    config: &SvgConfig,
    stylesheet: &Stylesheet,
    custom_css: Option<&str>,
    debug: bool,
    frame_states: &[crate::layout::keyframe::FrameState],
    frame_diffs: &[crate::layout::keyframe::FrameLayout],
    no_frame_css: bool,
) -> String {
    let mut builder = SvgBuilder::new(config.clone());

    // Add CSS custom properties from the stylesheet
    builder.add_stylesheet(stylesheet);

    // Set data-frames attribute
    let frame_names: Vec<&str> = frame_diffs.iter().map(|f| f.name.as_str()).collect();
    builder.data_frames = Some(frame_names.join(","));

    // Generate keyframe CSS. With `no_frame_css`, only emit the base hidden
    // rule — element/connection class hooks remain so an external runtime
    // can supply per-frame visibility rules.
    let keyframe_css = if no_frame_css {
        String::from("/* Keyframe CSS suppressed (--no-frame-css) */\n.kf-hidden { opacity: 0; }\n.kf-anim { transition: transform 0.5s ease, opacity 0.5s ease; }\n.ai-shape { transition: width 0.5s ease, height 0.5s ease, fill 0.5s ease, stroke 0.5s ease; }\n")
    } else {
        generate_keyframe_css(frame_states, frame_diffs)
    };
    builder.add_custom_css(&keyframe_css);

    // Add custom CSS after keyframe CSS
    if let Some(css) = custom_css {
        builder.add_custom_css(css);
    }

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

    // Render elements at frame-0 positions, with hidden elements getting opacity: 0
    let empty_set = std::collections::HashSet::new();
    let frame0_hidden = if !frame_states.is_empty() {
        &frame_states[0].hidden_elements
    } else {
        &empty_set
    };

    // Elements that start visible but are toggled (opacity diff) by a later
    // keyframe need a `kf-{id}` class hook so their hide/show rules bind.
    // Elements hidden in frame 0 already get the class via start_visibility_group.
    let kf_referenced: std::collections::HashSet<String> = frame_diffs
        .iter()
        .flat_map(|f| f.element_diffs.iter())
        .filter(|(_, diff)| !diff.is_empty())
        .map(|(id, _)| id.clone())
        .filter(|id| !frame0_hidden.contains(id))
        .collect();

    let mut sorted_elements: Vec<&ElementLayout> = result.root_elements.iter().collect();
    sorted_elements.sort_by_key(|e| e.z_order);
    for element in &sorted_elements {
        render_element_with_visibility(element, &mut builder, frame0_hidden, &kf_referenced);
    }

    // Render connections, with hidden connections getting opacity: 0
    let frame0_hidden_conns = if !frame_states.is_empty() {
        &frame_states[0].hidden_connections
    } else {
        &empty_set
    };

    for (i, conn) in result.connections.iter().enumerate() {
        let id = conn
            .name
            .as_ref()
            .map(|n| n.0.clone())
            .unwrap_or_else(|| format!("idx{}", i));
        let hidden0 = conn
            .name
            .as_ref()
            .map_or(false, |n| frame0_hidden_conns.contains(&n.0));
        if hidden0 {
            // Render with opacity 0 for hidden connections
            let mut hidden_conn = conn.clone();
            hidden_conn.styles.opacity = Some(0.0);
            render_connection(&hidden_conn, &mut builder, Some(&id));
            continue;
        }
        render_connection(conn, &mut builder, Some(&id));
    }

    // Render debug overlays
    if debug {
        for element in &result.root_elements {
            render_debug_bounds(element, &mut builder);
        }
    }

    builder.build(result.bounds)
}

/// Render an element, marking hidden elements with opacity: 0.
/// Wraps hidden elements in `<g opacity="0">` so shape + label are hidden together.
fn render_element_with_visibility(
    element: &ElementLayout,
    builder: &mut SvgBuilder,
    hidden: &std::collections::HashSet<String>,
    kf_referenced: &std::collections::HashSet<String>,
) {
    if let Some(id) = &element.id {
        if hidden.contains(&id.0) {
            // Use CSS class for hiding so frame CSS can override it
            builder.start_visibility_group(&id.0);
            render_element_inner(element, builder, hidden, kf_referenced);
            builder.end_group();
            return;
        }
        if kf_referenced.contains(&id.0) {
            // Visible at frame 0 but toggled by a later keyframe: wrap in a
            // kf-{id} class group so the later `.kf-{id} { opacity: 0 }` binds.
            builder.start_kf_class_group(&id.0);
            render_element_inner(element, builder, hidden, kf_referenced);
            builder.end_group();
            return;
        }
    }
    render_element_inner(element, builder, hidden, kf_referenced);
}

/// Generate CSS for keyframe frame switching
fn generate_keyframe_css(
    _frame_states: &[crate::layout::keyframe::FrameState],
    frame_diffs: &[crate::layout::keyframe::FrameLayout],
) -> String {
    let mut css = String::new();
    css.push_str("/* Keyframe animation CSS (auto-generated) */\n");
    css.push_str(".kf-hidden { opacity: 0; }\n");
    css.push_str(".kf-anim { transition: transform 0.5s ease, opacity 0.5s ease; }\n");
    css.push_str(".ai-shape { transition: width 0.5s ease, height 0.5s ease, fill 0.5s ease, stroke 0.5s ease; }\n");

    for frame in frame_diffs {
        let class_name = format!("frame-{}", frame.name);
        css.push_str(&format!(".{} {{\n", class_name));

        // Element diffs. Position + rotation + opacity animate on the wrapper group
        // (.kf-{id}, which contains shape + label, so the label rides along); size +
        // color animate on the inner shape (#{id}).
        for (elem_id, diff) in &frame.element_diffs {
            if let Some(opacity) = diff.opacity {
                css.push_str(&format!(
                    "  .kf-{} {{ opacity: {}; }}\n",
                    elem_id, opacity
                ));
            }
            // Position + rotation → transform on the wrapper group (label rides along).
            let mut xf = Vec::new();
            if diff.tx.is_some() || diff.ty.is_some() {
                xf.push(format!(
                    "translate({}px, {}px)",
                    diff.tx.unwrap_or(0.0),
                    diff.ty.unwrap_or(0.0)
                ));
            }
            if let Some(rot) = diff.rotation {
                xf.push(format!("rotate({}deg)", rot));
            }
            if !xf.is_empty() {
                css.push_str(&format!("  .kf-{} {{ transform: {}; }}\n", elem_id, xf.join(" ")));
            }
            // Size + color → inner shape.
            let mut props = Vec::new();
            if let Some(w) = diff.width {
                props.push(format!("width: {}px", w));
            }
            if let Some(h) = diff.height {
                props.push(format!("height: {}px", h));
            }
            if let Some(ref fill) = diff.fill {
                props.push(format!("fill: {}", fill));
            }
            if let Some(ref stroke) = diff.stroke {
                props.push(format!("stroke: {}", stroke));
            }
            if !props.is_empty() {
                css.push_str(&format!(
                    "  #{} {{ {}; }}\n",
                    elem_id,
                    props.join("; ")
                ));
            }
        }

        // Connection visibility diffs. Target `.conn-<name>` (not
        // `.ai-connection.conn-<name>`) so the rule matches both the path and its
        // label (the label carries `conn-<name>` but not the `ai-connection` class).
        for (conn_name, diff) in &frame.connection_diffs {
            if let Some(opacity) = diff.opacity {
                css.push_str(&format!(
                    "  .conn-{} {{ opacity: {}; }}\n",
                    conn_name, opacity
                ));
            }
        }

        css.push_str("}\n");
    }

    css
}

/// Render a LayoutResult to an SVG string with a custom stylesheet
pub fn render_svg_with_stylesheet(
    result: &LayoutResult,
    config: &SvgConfig,
    stylesheet: &Stylesheet,
    custom_css: Option<&str>,
    debug: bool,
) -> String {
    let mut builder = SvgBuilder::new(config.clone());

    // Add CSS custom properties from the stylesheet
    builder.add_stylesheet(stylesheet);

    // Add custom CSS after stylesheet variables (so it can reference/override them)
    if let Some(css) = custom_css {
        builder.add_custom_css(css);
    }

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

    // Render all root elements, sorted by z_order (stable sort preserves document order)
    let mut sorted_elements: Vec<&ElementLayout> = result.root_elements.iter().collect();
    sorted_elements.sort_by_key(|e| e.z_order);
    for element in &sorted_elements {
        render_element(element, &mut builder);
    }

    // Render all connections
    for conn in &result.connections {
        render_connection(conn, &mut builder, None);
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

/// Render a single element to the builder (no-keyframe path, no visibility checks)
fn render_element(element: &ElementLayout, builder: &mut SvgBuilder) {
    render_element_inner(
        element,
        builder,
        &std::collections::HashSet::new(),
        &std::collections::HashSet::new(),
    );
}

/// Render a single element to the builder with visibility checks for children
fn render_element_inner(
    element: &ElementLayout,
    builder: &mut SvgBuilder,
    hidden: &std::collections::HashSet<String>,
    kf_referenced: &std::collections::HashSet<String>,
) {
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
        ElementType::Shape(ShapeType::RasterImage { path }) => {
            // Render raster image as SVG <image> element
            let prefix = builder.prefix();
            let image_classes = std::iter::once(format!("{}raster-image", prefix))
                .chain(classes.iter().cloned())
                .collect::<Vec<_>>();

            // Apply rotation transform if specified
            let transform = if let Some(rotation) = element.styles.rotation {
                if rotation.abs() > f64::EPSILON {
                    let center = element.bounds.center();
                    Some(format!("rotate({} {} {})", rotation, center.x, center.y))
                } else {
                    None
                }
            } else {
                None
            };

            builder.add_image(
                id,
                path,
                element.bounds.x,
                element.bounds.y,
                element.bounds.width,
                element.bounds.height,
                &image_classes,
                transform.as_deref(),
            );
        }
        ElementType::Shape(ShapeType::Callout { pointer }) => {
            // Rounded pill + triangular pointer as a single closed path
            let d = callout_path_d(&element.bounds, *pointer);
            render_shape_with_rotation(element, builder, |b| {
                b.add_path(id, &d, &classes, &styles);
            });
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
        ElementType::GridCell => {
            // Reference-only cell: addressable via g.cell(r,c) but never drawn.
            return;
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

            // Render children (with visibility checks for keyframe animations)
            for child in &element.children {
                render_element_with_visibility(child, builder, hidden, kf_referenced);
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

/// Render a connection to the builder. `id` is the stable keyframe identity
/// (name, or `idx<N>` for unnamed) used for the `conn-<id>` CSS class; `None` on the
/// non-keyframe path falls back to the connection's name.
fn render_connection(conn: &ConnectionLayout, builder: &mut SvgBuilder, id: Option<&str>) {
    let mut classes = conn.styles.css_classes.clone();
    // Stable connection class for keyframe targeting (path morph / crossfade / opacity).
    let conn_class = id
        .map(|s| s.to_string())
        .or_else(|| conn.name.as_ref().map(|n| n.0.clone()));
    if let Some(c) = &conn_class {
        classes.push(format!("conn-{}", c));
    }
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
        let mut label_styles = label
            .styles
            .as_ref()
            .map(format_text_styles)
            .unwrap_or_else(|| r#" fill="var(--text-2)" font-size="12""#.to_string());
        // Propagate the connection's opacity to the label so a hidden connection
        // (e.g. frame-0 hidden) hides its label too.
        if let Some(opacity) = conn.styles.opacity {
            if (opacity - 1.0).abs() > f64::EPSILON {
                label_styles.push_str(&format!(r#" opacity="{}""#, opacity));
            }
        }
        // Carry the `conn-<id>` class so keyframe frame CSS toggles the label
        // together with the path.
        let extra_classes = conn_class
            .as_ref()
            .map(|c| format!("conn-{}", c))
            .unwrap_or_default();
        builder.add_text_with_classes(
            &label.text,
            label.position.x,
            label.position.y,
            &label.anchor,
            &label_styles,
            &extra_classes,
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
    if let Some(so) = styles.stroke_opacity {
        parts.push(format!(r#" stroke-opacity="{}""#, so));
    }
    if let Some(opacity) = styles.opacity {
        if (opacity - 1.0).abs() > f64::EPSILON {
            parts.push(format!(r#" opacity="{}""#, opacity));
        }
    }
    parts.join("")
}

/// Format text styles (fill and font_size for labels)
fn format_text_styles(styles: &ResolvedStyles) -> String {
    let mut parts = vec![];
    if let Some(fill) = &styles.fill {
        parts.push(format!(r#"fill="{}""#, fill));
    }
    if let Some(fo) = styles.fill_opacity {
        parts.push(format!(r#"fill-opacity="{}""#, fo));
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
    if let Some(fo) = styles.fill_opacity {
        parts.push(format!(r#" fill-opacity="{}""#, fo));
    }
    if let Some(so) = styles.stroke_opacity {
        parts.push(format!(r#" stroke-opacity="{}""#, so));
    }
    if let Some(op) = styles.opacity {
        if op < 1.0 {
            parts.push(format!(r#" opacity="{}""#, op));
        }
    }
    parts.join("")
}

/// Build the SVG path `d` for a callout: a rounded-rect pill (inset on the
/// pointer side) with a triangular pointer whose apex sits at the bounds edge.
/// The `tip` anchor coincides with that apex.
fn callout_path_d(b: &BoundingBox, pointer: PointerDir) -> String {
    let ps = crate::layout::engine::CALLOUT_POINTER_SIZE;
    // Pill rectangle, inset by the pointer depth on the pointer side.
    let (px, py, pw, ph) = match pointer {
        PointerDir::Up => (b.x, b.y + ps, b.width, b.height - ps),
        PointerDir::Down => (b.x, b.y, b.width, b.height - ps),
        PointerDir::Left => (b.x + ps, b.y, b.width - ps, b.height),
        PointerDir::Right => (b.x, b.y, b.width - ps, b.height),
    };
    let rr = 6.0_f64.min(pw / 2.0).min(ph / 2.0).max(0.0);
    let cx = px + pw / 2.0;
    let cy = py + ph / 2.0;
    // Half-width of the triangle base along the pointer-side edge.
    let tb = (ps).min((pw / 2.0 - rr).max(2.0)).min((ph / 2.0 - rr).max(2.0));
    let (l, r, t, bot) = (px, px + pw, py, py + ph);
    // Clockwise arc helper.
    let arc = |ex: f64, ey: f64| format!("A{:.2} {:.2} 0 0 1 {:.2} {:.2}", rr, rr, ex, ey);
    let m = |x: f64, y: f64| format!("M{:.2} {:.2}", x, y);
    let line = |x: f64, y: f64| format!("L{:.2} {:.2}", x, y);
    let mut seg: Vec<String> = Vec::new();
    match pointer {
        PointerDir::Down => {
            seg.push(m(l + rr, t));
            seg.push(line(r - rr, t));
            seg.push(arc(r, t + rr));
            seg.push(line(r, bot - rr));
            seg.push(arc(r - rr, bot));
            seg.push(line(cx + tb, bot));
            seg.push(line(cx, b.bottom())); // apex (tip)
            seg.push(line(cx - tb, bot));
            seg.push(line(l + rr, bot));
            seg.push(arc(l, bot - rr));
            seg.push(line(l, t + rr));
            seg.push(arc(l + rr, t));
        }
        PointerDir::Up => {
            seg.push(m(l + rr, t));
            seg.push(line(cx - tb, t));
            seg.push(line(cx, b.y)); // apex (tip)
            seg.push(line(cx + tb, t));
            seg.push(line(r - rr, t));
            seg.push(arc(r, t + rr));
            seg.push(line(r, bot - rr));
            seg.push(arc(r - rr, bot));
            seg.push(line(l + rr, bot));
            seg.push(arc(l, bot - rr));
            seg.push(line(l, t + rr));
            seg.push(arc(l + rr, t));
        }
        PointerDir::Left => {
            seg.push(m(l + rr, t));
            seg.push(line(r - rr, t));
            seg.push(arc(r, t + rr));
            seg.push(line(r, bot - rr));
            seg.push(arc(r - rr, bot));
            seg.push(line(l + rr, bot));
            seg.push(arc(l, bot - rr));
            seg.push(line(l, cy + tb));
            seg.push(line(b.x, cy)); // apex (tip)
            seg.push(line(l, cy - tb));
            seg.push(line(l, t + rr));
            seg.push(arc(l + rr, t));
        }
        PointerDir::Right => {
            seg.push(m(l + rr, t));
            seg.push(line(r - rr, t));
            seg.push(arc(r, t + rr));
            seg.push(line(r, cy - tb));
            seg.push(line(b.right(), cy)); // apex (tip)
            seg.push(line(r, cy + tb));
            seg.push(line(r, bot - rr));
            seg.push(arc(r - rr, bot));
            seg.push(line(l + rr, bot));
            seg.push(arc(l, bot - rr));
            seg.push(line(l, t + rr));
            seg.push(arc(l + rr, t));
        }
    }
    format!("{} Z", seg.join(" "))
}

/// Convert a path of points to an SVG path d attribute
/// Build the SVG path `d` string for a connection, including the arrow-marker pullback,
/// matching exactly what `add_connection_path` renders. Shared so per-frame keyframe CSS
/// (`d: path(...)`) targets the same geometry the base path uses.
fn connection_path_d(path: &[Point], routing_mode: RoutingMode, marker_end: bool, stroke_width: f64) -> String {
    // Shorten the endpoint when a marker is present so the arrow tip lands on the anchor.
    // pullback = 9 * (markerWidth=4 / 10) * strokeWidth = 3.6 * strokeWidth.
    let path = if marker_end && path.len() >= 2 {
        let mut shortened = path.to_vec();
        let last_idx = shortened.len() - 1;
        let prev_idx = last_idx - 1;
        let dx = shortened[last_idx].x - shortened[prev_idx].x;
        let dy = shortened[last_idx].y - shortened[prev_idx].y;
        let len = (dx * dx + dy * dy).sqrt();
        if len > 0.001 {
            let pullback = 3.6 * stroke_width;
            shortened[last_idx].x -= dx / len * pullback;
            shortened[last_idx].y -= dy / len * pullback;
        }
        shortened
    } else {
        path.to_vec()
    };

    match routing_mode {
        RoutingMode::Curved if path.len() >= 4 => {
            let mut d = format!(
                "M{} {} C{} {} {} {} {} {}",
                path[0].x, path[0].y, path[1].x, path[1].y, path[2].x, path[2].y, path[3].x, path[3].y
            );
            for chunk in path[4..].chunks(3) {
                if chunk.len() == 3 {
                    d.push_str(&format!(
                        " C{} {} {} {} {} {}",
                        chunk[0].x, chunk[0].y, chunk[1].x, chunk[1].y, chunk[2].x, chunk[2].y
                    ));
                } else if chunk.len() == 2 {
                    d.push_str(&format!(" Q{} {} {} {}", chunk[0].x, chunk[0].y, chunk[1].x, chunk[1].y));
                } else if chunk.len() == 1 {
                    d.push_str(&format!(" L{} {}", chunk[0].x, chunk[0].y));
                }
            }
            d
        }
        _ => path_to_d(&path),
    }
}

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
            fill_opacity: None,
            stroke_opacity: None,
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
    fn test_format_styles_with_opacities() {
        let styles = ResolvedStyles {
            fill: Some("var(--secondary-1)".to_string()),
            stroke: Some("#000000".to_string()),
            stroke_width: Some(2.0),
            stroke_dasharray: None,
            opacity: None,
            fill_opacity: Some(0.7),
            stroke_opacity: Some(0.4),
            font_size: None,
            css_classes: vec![],
            rotation: None,
        };
        let result = format_styles(&styles);
        // Symbolic color is preserved, not flattened
        assert!(result.contains(r#"fill="var(--secondary-1)""#));
        assert!(result.contains(r#"fill-opacity="0.7""#));
        assert!(result.contains(r#"stroke-opacity="0.4""#));
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
            z_order: 0,
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
            z_order: 0,
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
            z_order: 0,
        });
        result.connections.push(ConnectionLayout {
            from_id: Identifier::new("a"),
            to_id: Identifier::new("b"),
            direction: ConnectionDirection::Forward,
            path: vec![Point::new(50.0, 25.0), Point::new(100.0, 25.0)],
            styles: ResolvedStyles::default(),
            label: None,
            routing_mode: RoutingMode::default(),
            name: None,
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
                    z_order: 0,
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
                    z_order: 0,
                },
            ],
            label: None,
            anchors: AnchorSet::default(),
            path_normalize: true,
            z_order: 0,
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
