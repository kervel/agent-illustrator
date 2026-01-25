//! Core types for the layout engine

use std::collections::HashMap;

use crate::parser::ast::{
    ColorValue, ConnectionDirection, Identifier, LayoutType, ShapeType, Span, Spanned, StyleKey,
    StyleModifier, StyleValue,
};

/// A 2D point in the coordinate system
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// A bounding box representing the spatial extent of an element
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl BoundingBox {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Create a zero-sized bounding box at the origin
    pub fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }

    /// Right edge x-coordinate
    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    /// Bottom edge y-coordinate
    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }

    /// Center point of the bounding box
    pub fn center(&self) -> Point {
        Point {
            x: self.x + self.width / 2.0,
            y: self.y + self.height / 2.0,
        }
    }

    /// Check if this bounding box contains a point
    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.x
            && point.x <= self.right()
            && point.y >= self.y
            && point.y <= self.bottom()
    }

    /// Check if this bounding box intersects another
    pub fn intersects(&self, other: &BoundingBox) -> bool {
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }

    /// Compute the union of two bounding boxes (smallest box containing both)
    pub fn union(&self, other: &BoundingBox) -> BoundingBox {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let right = self.right().max(other.right());
        let bottom = self.bottom().max(other.bottom());
        BoundingBox::new(x, y, right - x, bottom - y)
    }

    /// Expand this bounding box to include a point
    pub fn expand_to_include(&self, point: Point) -> BoundingBox {
        let x = self.x.min(point.x);
        let y = self.y.min(point.y);
        let right = self.right().max(point.x);
        let bottom = self.bottom().max(point.y);
        BoundingBox::new(x, y, right - x, bottom - y)
    }
}

impl Default for BoundingBox {
    fn default() -> Self {
        Self::zero()
    }
}

/// Resolved style properties ready for rendering
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ResolvedStyles {
    pub fill: Option<String>,
    pub stroke: Option<String>,
    pub stroke_width: Option<f64>,
    pub stroke_dasharray: Option<String>,
    pub opacity: Option<f64>,
    pub font_size: Option<f64>,
    pub css_classes: Vec<String>,
    /// Rotation angle in degrees (clockwise positive, 0 = no rotation)
    pub rotation: Option<f64>,
}

impl ResolvedStyles {
    /// Create styles with sensible defaults
    pub fn with_defaults() -> Self {
        Self {
            fill: Some("#f0f0f0".to_string()),
            stroke: Some("#333333".to_string()),
            stroke_width: Some(2.0),
            stroke_dasharray: None,
            opacity: Some(1.0),
            font_size: Some(14.0),
            css_classes: vec![],
            rotation: None,
        }
    }

    /// Create styles from AST style modifiers
    ///
    /// Symbolic colors are converted to CSS variable references (e.g., `var(--foreground-1)`).
    /// The actual color values are provided via a `<style>` block in the SVG output.
    pub fn from_modifiers(modifiers: &[Spanned<StyleModifier>]) -> Self {
        let mut styles = Self::default();

        for modifier in modifiers {
            match &modifier.node.key.node {
                StyleKey::Fill => {
                    styles.fill = Self::color_to_css(&modifier.node.value.node);
                }
                StyleKey::Stroke => {
                    styles.stroke = Self::color_to_css(&modifier.node.value.node);
                }
                StyleKey::StrokeWidth => {
                    if let StyleValue::Number { value, .. } = &modifier.node.value.node {
                        styles.stroke_width = Some(*value);
                    }
                }
                StyleKey::StrokeDasharray => {
                    // Accept string like "4,2" or keyword like "dashed"
                    if let StyleValue::String(s) = &modifier.node.value.node {
                        styles.stroke_dasharray = Some(s.clone());
                    } else if let StyleValue::Keyword(k) = &modifier.node.value.node {
                        // Convert keywords to dash patterns
                        let pattern = match k.as_str() {
                            "dashed" => "8,4",
                            "dotted" => "2,2",
                            _ => k.as_str(),
                        };
                        styles.stroke_dasharray = Some(pattern.to_string());
                    }
                }
                StyleKey::Opacity => {
                    if let StyleValue::Number { value, .. } = &modifier.node.value.node {
                        styles.opacity = Some(*value);
                    }
                }
                StyleKey::FontSize => {
                    if let StyleValue::Number { value, .. } = &modifier.node.value.node {
                        styles.font_size = Some(*value);
                    }
                }
                StyleKey::Class => {
                    if let StyleValue::String(c) = &modifier.node.value.node {
                        styles.css_classes.push(c.clone());
                    } else if let StyleValue::Keyword(k) = &modifier.node.value.node {
                        styles.css_classes.push(k.clone());
                    } else if let StyleValue::Identifier(id) = &modifier.node.value.node {
                        styles.css_classes.push(id.0.clone());
                    }
                }
                StyleKey::Rotation => {
                    if let StyleValue::Number { value, .. } = &modifier.node.value.node {
                        styles.rotation = Some(*value);
                    }
                }
                StyleKey::Label
                | StyleKey::LabelPosition
                | StyleKey::Gap
                | StyleKey::Size
                | StyleKey::Width
                | StyleKey::Height
                | StyleKey::Routing
                | StyleKey::Role
                | StyleKey::X
                | StyleKey::Y
                | StyleKey::Custom(_) => {
                    // Labels, label position, gap, size, routing, role, and position modifiers
                    // handled separately in layout engine; custom keys ignored for now
                }
            }
        }

        styles
    }

    /// Convert a StyleValue to a CSS color string
    ///
    /// - Hex colors: pass through (e.g., `#ff0000`)
    /// - Named colors: pass through (e.g., `red`)
    /// - Symbolic colors: convert to CSS variable reference (e.g., `var(--foreground-1)`)
    fn color_to_css(value: &StyleValue) -> Option<String> {
        match value {
            StyleValue::Color(color_value) => match color_value {
                // Hex and named colors pass through unchanged
                ColorValue::Hex(s) | ColorValue::Named(s) => Some(s.clone()),
                // Symbolic colors become CSS variable references
                ColorValue::Symbolic { .. } => {
                    let token = color_value.token_string()?;
                    Some(format!("var(--{})", token))
                }
            },
            // Keywords that aren't symbolic colors are treated as named colors
            StyleValue::Keyword(k) => Some(k.clone()),
            // Identifiers can be color keywords like "red", "blue", etc.
            StyleValue::Identifier(id) => Some(id.0.clone()),
            _ => None,
        }
    }

    /// Merge another style set, with other taking precedence
    pub fn merge(&self, other: &ResolvedStyles) -> ResolvedStyles {
        ResolvedStyles {
            fill: other.fill.clone().or_else(|| self.fill.clone()),
            stroke: other.stroke.clone().or_else(|| self.stroke.clone()),
            stroke_width: other.stroke_width.or(self.stroke_width),
            stroke_dasharray: other
                .stroke_dasharray
                .clone()
                .or_else(|| self.stroke_dasharray.clone()),
            opacity: other.opacity.or(self.opacity),
            font_size: other.font_size.or(self.font_size),
            css_classes: {
                let mut classes = self.css_classes.clone();
                classes.extend(other.css_classes.clone());
                classes
            },
            rotation: other.rotation.or(self.rotation),
        }
    }
}

/// Type of element in the layout
#[derive(Debug, Clone, PartialEq)]
pub enum ElementType {
    Shape(ShapeType),
    Layout(LayoutType),
    Group,
}

/// Text anchor position for labels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAnchor {
    Start,
    Middle,
    End,
}

/// Layout information for a label
#[derive(Debug, Clone)]
pub struct LabelLayout {
    pub text: String,
    pub position: Point,
    pub anchor: TextAnchor,
    /// Optional styles for the label (used when referencing a styled element)
    pub styles: Option<ResolvedStyles>,
}

/// Layout information for a single element
#[derive(Debug, Clone)]
pub struct ElementLayout {
    pub id: Option<Identifier>,
    pub element_type: ElementType,
    pub bounds: BoundingBox,
    pub styles: ResolvedStyles,
    pub children: Vec<ElementLayout>,
    pub label: Option<LabelLayout>,
}

impl ElementLayout {
    /// Get the identifier as a string, if present
    pub fn id_str(&self) -> Option<&str> {
        self.id.as_ref().map(|id| id.0.as_str())
    }
}

/// Layout information for a connection between elements
#[derive(Debug, Clone)]
pub struct ConnectionLayout {
    pub from_id: Identifier,
    pub to_id: Identifier,
    pub direction: ConnectionDirection,
    pub path: Vec<Point>,
    pub styles: ResolvedStyles,
    pub label: Option<LabelLayout>,
}

/// The complete result of layout computation
#[derive(Debug, Clone)]
pub struct LayoutResult {
    /// All elements indexed by identifier
    pub elements: HashMap<String, ElementLayout>,
    /// Root-level elements in document order
    pub root_elements: Vec<ElementLayout>,
    /// All connections
    pub connections: Vec<ConnectionLayout>,
    /// Bounding box containing all elements
    pub bounds: BoundingBox,
}

impl LayoutResult {
    /// Create an empty layout result
    pub fn new() -> Self {
        Self {
            elements: HashMap::new(),
            root_elements: vec![],
            connections: vec![],
            bounds: BoundingBox::zero(),
        }
    }

    /// Add an element to the layout
    pub fn add_element(&mut self, element: ElementLayout) {
        // Index by ID if present
        if let Some(id) = &element.id {
            self.elements.insert(id.0.clone(), element.clone());
        }
        // Also index children recursively
        self.index_children(&element);
        self.root_elements.push(element);
    }

    fn index_children(&mut self, element: &ElementLayout) {
        for child in &element.children {
            if let Some(id) = &child.id {
                self.elements.insert(id.0.clone(), child.clone());
            }
            self.index_children(child);
        }
    }

    /// Get an element by identifier
    pub fn get_element(&self, id: &Identifier) -> Option<&ElementLayout> {
        self.elements.get(&id.0)
    }

    /// Get an element by name string
    pub fn get_element_by_name(&self, name: &str) -> Option<&ElementLayout> {
        self.elements.get(name)
    }

    /// Get mutable reference to element by name (for constraint resolution)
    pub fn get_element_mut_by_name(&mut self, name: &str) -> Option<&mut ElementLayout> {
        // First check root elements
        for elem in &mut self.root_elements {
            if elem.id.as_ref().map(|id| id.0.as_str()) == Some(name) {
                return Some(elem);
            }
            // Check children recursively
            if let Some(child) = find_element_mut(&mut elem.children, name) {
                return Some(child);
            }
        }
        None
    }

    /// Remove an element by name (used to hide elements that are used as connection labels)
    pub fn remove_element_by_name(&mut self, name: &str) {
        // Remove from the index
        self.elements.remove(name);

        // Remove from root_elements
        self.root_elements
            .retain(|elem| elem.id.as_ref().map(|id| id.0.as_str()) != Some(name));

        // Also remove from children recursively
        for elem in &mut self.root_elements {
            remove_from_children(&mut elem.children, name);
        }
    }

    /// Compute the bounding box that contains all elements
    pub fn compute_bounds(&mut self) {
        if self.root_elements.is_empty() {
            self.bounds = BoundingBox::zero();
            return;
        }

        let mut bounds = self.root_elements[0].bounds;
        for element in &self.root_elements[1..] {
            bounds = bounds.union(&element.bounds);
        }

        // Also include connection paths
        for conn in &self.connections {
            for point in &conn.path {
                bounds = bounds.expand_to_include(*point);
            }
            // Include connection labels
            if let Some(label) = &conn.label {
                bounds = expand_bounds_for_label(bounds, label);
            }
        }

        // Include element labels recursively
        for element in &self.root_elements {
            bounds = expand_bounds_for_element_labels(bounds, element);
        }

        self.bounds = bounds;
    }
}

/// Estimate the width of a text label (approximate: ~7px per character for default font)
fn estimate_label_width(text: &str) -> f64 {
    text.len() as f64 * 7.0
}

/// Expand bounds to include a label, accounting for text anchor
fn expand_bounds_for_label(bounds: BoundingBox, label: &LabelLayout) -> BoundingBox {
    let estimated_width = estimate_label_width(&label.text);
    let estimated_height = 14.0; // approximate line height

    // Calculate label bounds based on anchor
    let (label_left, label_right) = match label.anchor {
        TextAnchor::Start => (label.position.x, label.position.x + estimated_width),
        TextAnchor::Middle => (
            label.position.x - estimated_width / 2.0,
            label.position.x + estimated_width / 2.0,
        ),
        TextAnchor::End => (label.position.x - estimated_width, label.position.x),
    };

    // Labels extend above their position point (text baseline)
    let label_top = label.position.y - estimated_height;
    let label_bottom = label.position.y;

    let label_bounds = BoundingBox::new(
        label_left,
        label_top,
        label_right - label_left,
        label_bottom - label_top,
    );

    bounds.union(&label_bounds)
}

/// Recursively expand bounds to include all element labels
fn expand_bounds_for_element_labels(bounds: BoundingBox, element: &ElementLayout) -> BoundingBox {
    let mut bounds = bounds;

    // Include this element's label
    if let Some(label) = &element.label {
        bounds = expand_bounds_for_label(bounds, label);
    }

    // Recursively include children's labels
    for child in &element.children {
        bounds = expand_bounds_for_element_labels(bounds, child);
    }

    bounds
}

fn find_element_mut<'a>(
    children: &'a mut [ElementLayout],
    name: &str,
) -> Option<&'a mut ElementLayout> {
    for child in children {
        if child.id.as_ref().map(|id| id.0.as_str()) == Some(name) {
            return Some(child);
        }
        if let Some(found) = find_element_mut(&mut child.children, name) {
            return Some(found);
        }
    }
    None
}

fn remove_from_children(children: &mut Vec<ElementLayout>, name: &str) {
    children.retain(|elem| elem.id.as_ref().map(|id| id.0.as_str()) != Some(name));
    for child in children {
        remove_from_children(&mut child.children, name);
    }
}

impl Default for LayoutResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about a constraint for error reporting
#[derive(Debug, Clone)]
pub struct ConstraintInfo {
    pub subject: String,
    pub relation: String,
    pub anchor: String,
    pub span: Span,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_creation() {
        let p = Point::new(10.0, 20.0);
        assert_eq!(p.x, 10.0);
        assert_eq!(p.y, 20.0);
    }

    #[test]
    fn test_bounding_box_edges() {
        let bb = BoundingBox::new(10.0, 20.0, 100.0, 50.0);
        assert_eq!(bb.right(), 110.0);
        assert_eq!(bb.bottom(), 70.0);
    }

    #[test]
    fn test_bounding_box_center() {
        let bb = BoundingBox::new(0.0, 0.0, 100.0, 50.0);
        let center = bb.center();
        assert_eq!(center.x, 50.0);
        assert_eq!(center.y, 25.0);
    }

    #[test]
    fn test_bounding_box_contains() {
        let bb = BoundingBox::new(0.0, 0.0, 100.0, 100.0);
        assert!(bb.contains(Point::new(50.0, 50.0)));
        assert!(bb.contains(Point::new(0.0, 0.0)));
        assert!(bb.contains(Point::new(100.0, 100.0)));
        assert!(!bb.contains(Point::new(-1.0, 50.0)));
        assert!(!bb.contains(Point::new(101.0, 50.0)));
    }

    #[test]
    fn test_bounding_box_intersects() {
        let a = BoundingBox::new(0.0, 0.0, 100.0, 100.0);
        let b = BoundingBox::new(50.0, 50.0, 100.0, 100.0);
        let c = BoundingBox::new(200.0, 200.0, 50.0, 50.0);

        assert!(a.intersects(&b));
        assert!(b.intersects(&a));
        assert!(!a.intersects(&c));
        assert!(!c.intersects(&a));
    }

    #[test]
    fn test_bounding_box_union() {
        let a = BoundingBox::new(0.0, 0.0, 50.0, 50.0);
        let b = BoundingBox::new(100.0, 100.0, 50.0, 50.0);
        let union = a.union(&b);

        assert_eq!(union.x, 0.0);
        assert_eq!(union.y, 0.0);
        assert_eq!(union.width, 150.0);
        assert_eq!(union.height, 150.0);
    }

    #[test]
    fn test_resolved_styles_defaults() {
        let styles = ResolvedStyles::with_defaults();
        assert_eq!(styles.fill, Some("#f0f0f0".to_string()));
        assert_eq!(styles.stroke, Some("#333333".to_string()));
        assert_eq!(styles.stroke_width, Some(2.0));
    }

    #[test]
    fn test_layout_result_add_element() {
        let mut result = LayoutResult::new();
        let element = ElementLayout {
            id: Some(Identifier::new("test")),
            element_type: ElementType::Shape(ShapeType::Rectangle),
            bounds: BoundingBox::new(0.0, 0.0, 100.0, 50.0),
            styles: ResolvedStyles::default(),
            children: vec![],
            label: None,
        };

        result.add_element(element);

        assert_eq!(result.root_elements.len(), 1);
        assert!(result.get_element_by_name("test").is_some());
    }

    #[test]
    fn test_resolved_styles_rotation() {
        use crate::parser::ast::{Spanned, StyleKey, StyleModifier, StyleValue};

        let modifiers = vec![Spanned::new(
            StyleModifier {
                key: Spanned::new(StyleKey::Rotation, 0..8),
                value: Spanned::new(
                    StyleValue::Number {
                        value: 45.0,
                        unit: None,
                    },
                    10..12,
                ),
            },
            0..12,
        )];

        let styles = ResolvedStyles::from_modifiers(&modifiers);
        assert_eq!(styles.rotation, Some(45.0));
    }
}
