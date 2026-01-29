//! Core types for the layout engine

use std::collections::HashMap;

use crate::parser::ast::{
    ColorValue, ConnectionDirection, ConstraintProperty, Identifier, LayoutType, ShapeType, Span,
    Spanned, StyleKey, StyleModifier, StyleValue,
};

use super::routing::RoutingMode;

// ============================================
// Anchor Types (Feature 009)
// ============================================

/// Direction a connector should approach/leave an anchor.
/// Represents the outward normal at the anchor point.
/// Connectors should arrive/depart perpendicular to the shape.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnchorDirection {
    /// 270° - connector comes from above
    Up,
    /// 90° - connector comes from below
    Down,
    /// 180° - connector comes from the left
    Left,
    /// 0° - connector comes from the right
    Right,
    /// Custom angle in degrees (0=right, 90=down, 180=left, 270=up)
    Angle(f64),
}

impl AnchorDirection {
    /// Convert direction to a unit vector
    pub fn to_vector(&self) -> Point {
        let angle_rad = match self {
            AnchorDirection::Up => 270.0_f64.to_radians(),
            AnchorDirection::Down => 90.0_f64.to_radians(),
            AnchorDirection::Left => 180.0_f64.to_radians(),
            AnchorDirection::Right => 0.0_f64.to_radians(),
            AnchorDirection::Angle(deg) => deg.to_radians(),
        };
        Point::new(angle_rad.cos(), angle_rad.sin())
    }

    /// Infer direction from a constraint property
    pub fn from_property(prop: &ConstraintProperty) -> Self {
        match prop {
            ConstraintProperty::Left => AnchorDirection::Left,
            ConstraintProperty::Right => AnchorDirection::Right,
            ConstraintProperty::Top => AnchorDirection::Up,
            ConstraintProperty::Bottom => AnchorDirection::Down,
            // Default to Down for center and other properties
            _ => AnchorDirection::Down,
        }
    }

    /// Convert to angle in degrees
    pub fn to_degrees(&self) -> f64 {
        match self {
            AnchorDirection::Up => 270.0,
            AnchorDirection::Down => 90.0,
            AnchorDirection::Left => 180.0,
            AnchorDirection::Right => 0.0,
            AnchorDirection::Angle(deg) => *deg,
        }
    }

    /// Create an AnchorDirection from an angle in degrees.
    ///
    /// Snaps to cardinal directions if within 1 degree tolerance.
    /// Normalizes angles to 0-360 range.
    ///
    /// # Arguments
    /// * `degrees` - Angle in degrees (0=right, 90=down, 180=left, 270=up)
    ///
    /// # Returns
    /// The appropriate AnchorDirection variant
    pub fn from_degrees(degrees: f64) -> Self {
        // Normalize to 0-360
        let normalized = ((degrees % 360.0) + 360.0) % 360.0;

        // Snap to cardinal if close (within 1 degree)
        if (normalized - 0.0).abs() < 1.0 || (normalized - 360.0).abs() < 1.0 {
            AnchorDirection::Right
        } else if (normalized - 90.0).abs() < 1.0 {
            AnchorDirection::Down
        } else if (normalized - 180.0).abs() < 1.0 {
            AnchorDirection::Left
        } else if (normalized - 270.0).abs() < 1.0 {
            AnchorDirection::Up
        } else {
            AnchorDirection::Angle(normalized)
        }
    }
}

/// A named attachment point on a shape (T002)
#[derive(Debug, Clone, PartialEq)]
pub struct Anchor {
    /// Name of the anchor (e.g., "top", "left", "input")
    pub name: String,
    /// Position of the anchor in absolute coordinates
    pub position: Point,
    /// Direction connectors should approach/leave (outward normal)
    pub direction: AnchorDirection,
}

impl Anchor {
    /// Create a new anchor
    pub fn new(name: impl Into<String>, position: Point, direction: AnchorDirection) -> Self {
        Self {
            name: name.into(),
            position,
            direction,
        }
    }
}

/// Collection of anchors for an element (T002)
#[derive(Debug, Clone, Default)]
pub struct AnchorSet {
    anchors: HashMap<String, Anchor>,
}

impl AnchorSet {
    /// Create an empty anchor set
    pub fn new() -> Self {
        Self {
            anchors: HashMap::new(),
        }
    }

    /// Get an anchor by name
    pub fn get(&self, name: &str) -> Option<&Anchor> {
        self.anchors.get(name)
    }

    /// Insert an anchor
    pub fn insert(&mut self, anchor: Anchor) {
        self.anchors.insert(anchor.name.clone(), anchor);
    }

    /// Get all anchor names
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.anchors.keys().map(|s| s.as_str())
    }

    /// Create anchors for a simple shape (rect, ellipse, circle)
    /// Returns 4 anchors: top, bottom, left, right
    pub fn simple_shape(bounds: &BoundingBox) -> Self {
        let mut set = Self::new();
        set.insert(Anchor::new("top", bounds.top_center(), AnchorDirection::Up));
        set.insert(Anchor::new(
            "bottom",
            bounds.bottom_center(),
            AnchorDirection::Down,
        ));
        set.insert(Anchor::new(
            "left",
            bounds.left_center(),
            AnchorDirection::Left,
        ));
        set.insert(Anchor::new(
            "right",
            bounds.right_center(),
            AnchorDirection::Right,
        ));
        set
    }

    /// Create anchors for a path shape
    /// Returns 8 anchors: top, bottom, left, right + 4 corners
    pub fn path_shape(bounds: &BoundingBox) -> Self {
        let mut set = Self::simple_shape(bounds);
        // Add corner anchors with diagonal directions
        set.insert(Anchor::new(
            "top_left",
            bounds.top_left(),
            AnchorDirection::Angle(225.0),
        ));
        set.insert(Anchor::new(
            "top_right",
            bounds.top_right(),
            AnchorDirection::Angle(315.0),
        ));
        set.insert(Anchor::new(
            "bottom_left",
            bounds.bottom_left(),
            AnchorDirection::Angle(135.0),
        ));
        set.insert(Anchor::new(
            "bottom_right",
            bounds.bottom_right(),
            AnchorDirection::Angle(45.0),
        ));
        set
    }

    /// Create anchors for an element type with the given bounds.
    /// This determines the appropriate anchor set based on element type:
    /// - Path shapes get 8 anchors (4 sides + 4 corners)
    /// - All other shapes, layouts, and groups get 4 anchors (top, bottom, left, right)
    pub fn for_element_type(element_type: &ElementType, bounds: &BoundingBox) -> Self {
        match element_type {
            ElementType::Shape(ShapeType::Path(_)) => Self::path_shape(bounds),
            _ => Self::simple_shape(bounds),
        }
    }

    /// Update the built-in anchors (top, bottom, left, right, and corners for paths)
    /// to reflect new bounds. Custom anchors are preserved but NOT updated.
    /// Use this after moving an element to keep anchors in sync with bounds.
    pub fn update_builtin_from_bounds(&mut self, element_type: &ElementType, bounds: &BoundingBox) {
        // Always update the 4 cardinal anchors
        self.insert(Anchor::new("top", bounds.top_center(), AnchorDirection::Up));
        self.insert(Anchor::new(
            "bottom",
            bounds.bottom_center(),
            AnchorDirection::Down,
        ));
        self.insert(Anchor::new(
            "left",
            bounds.left_center(),
            AnchorDirection::Left,
        ));
        self.insert(Anchor::new(
            "right",
            bounds.right_center(),
            AnchorDirection::Right,
        ));

        // For path shapes, also update corner anchors
        if matches!(element_type, ElementType::Shape(ShapeType::Path(_))) {
            self.insert(Anchor::new(
                "top_left",
                bounds.top_left(),
                AnchorDirection::Angle(225.0),
            ));
            self.insert(Anchor::new(
                "top_right",
                bounds.top_right(),
                AnchorDirection::Angle(315.0),
            ));
            self.insert(Anchor::new(
                "bottom_left",
                bounds.bottom_left(),
                AnchorDirection::Angle(135.0),
            ));
            self.insert(Anchor::new(
                "bottom_right",
                bounds.bottom_right(),
                AnchorDirection::Angle(45.0),
            ));
        }
    }

    /// Create anchors from a list of custom anchor definitions
    pub fn from_custom(anchors: impl IntoIterator<Item = Anchor>) -> Self {
        let mut set = Self::new();
        for anchor in anchors {
            set.insert(anchor);
        }
        set
    }

    /// Merge another anchor set into this one (other takes precedence)
    pub fn merge(&mut self, other: &AnchorSet) {
        for (name, anchor) in &other.anchors {
            self.anchors.insert(name.clone(), anchor.clone());
        }
    }

    /// Check if the set is empty
    pub fn is_empty(&self) -> bool {
        self.anchors.is_empty()
    }

    /// Get the number of anchors
    pub fn len(&self) -> usize {
        self.anchors.len()
    }

    /// Transform all anchors in this set using the given rotation.
    ///
    /// Creates a new AnchorSet with all anchor positions and directions
    /// transformed according to the rotation.
    ///
    /// # Arguments
    /// * `rotation` - The rotation transform to apply
    ///
    /// # Returns
    /// A new AnchorSet with transformed anchors
    pub fn transform(&self, rotation: &super::transform::RotationTransform) -> AnchorSet {
        AnchorSet {
            anchors: self
                .anchors
                .iter()
                .map(|(name, anchor)| (name.clone(), rotation.transform_anchor(anchor)))
                .collect(),
        }
    }

    /// Translate all anchor positions by the given delta.
    pub fn translate(&mut self, dx: f64, dy: f64) {
        for anchor in self.anchors.values_mut() {
            anchor.position.x += dx;
            anchor.position.y += dy;
        }
    }

    /// Rotate only the directions of all anchors in place (positions unchanged).
    ///
    /// Used to re-apply direction rotation after `recompute_custom_anchors` which
    /// overwrites directions from the original (non-rotated) AST.
    ///
    /// # Arguments
    /// * `rotation` - The rotation transform to apply to directions
    pub fn rotate_directions(&mut self, rotation: &super::transform::RotationTransform) {
        for anchor in self.anchors.values_mut() {
            anchor.direction = rotation.transform_direction(anchor.direction);
        }
    }
}

/// Resolved anchor with absolute position for connection routing (T013)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedAnchor {
    /// Absolute position of the anchor
    pub position: Point,
    /// Direction connectors should approach/leave
    pub direction: AnchorDirection,
}

impl ResolvedAnchor {
    /// Create a new resolved anchor
    pub fn new(position: Point, direction: AnchorDirection) -> Self {
        Self {
            position,
            direction,
        }
    }

    /// Create from an Anchor
    pub fn from_anchor(anchor: &Anchor) -> Self {
        Self {
            position: anchor.position,
            direction: anchor.direction,
        }
    }
}

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

    // ============================================
    // Anchor Position Helpers (T014)
    // ============================================

    /// Top edge center point
    pub fn top_center(&self) -> Point {
        Point::new(self.x + self.width / 2.0, self.y)
    }

    /// Bottom edge center point
    pub fn bottom_center(&self) -> Point {
        Point::new(self.x + self.width / 2.0, self.y + self.height)
    }

    /// Left edge center point
    pub fn left_center(&self) -> Point {
        Point::new(self.x, self.y + self.height / 2.0)
    }

    /// Right edge center point
    pub fn right_center(&self) -> Point {
        Point::new(self.x + self.width, self.y + self.height / 2.0)
    }

    /// Top-left corner point
    pub fn top_left(&self) -> Point {
        Point::new(self.x, self.y)
    }

    /// Top-right corner point
    pub fn top_right(&self) -> Point {
        Point::new(self.x + self.width, self.y)
    }

    /// Bottom-left corner point
    pub fn bottom_left(&self) -> Point {
        Point::new(self.x, self.y + self.height)
    }

    /// Bottom-right corner point
    pub fn bottom_right(&self) -> Point {
        Point::new(self.x + self.width, self.y + self.height)
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
    /// Anchor points on this element (Feature 009)
    pub anchors: AnchorSet,
    /// Whether to normalize path geometry to the element origin when rendering.
    /// Paths that have already been rotated in layout should skip normalization.
    pub path_normalize: bool,
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
    pub routing_mode: RoutingMode, // Feature 008: track routing mode for rendering
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

    /// Compute the bounding box that contains all elements.
    /// Walks leaf elements recursively so that container bounds (which may be
    /// stale after constraint solving) don't inflate the viewBox.
    pub fn compute_bounds(&mut self) {
        if self.root_elements.is_empty() {
            self.bounds = BoundingBox::zero();
            return;
        }

        let mut leaf_bounds: Option<BoundingBox> = None;
        for element in &self.root_elements {
            collect_leaf_bounds(element, &mut leaf_bounds);
        }

        let mut bounds = leaf_bounds.unwrap_or_else(BoundingBox::zero);

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

/// Recursively collect bounds from leaf elements (those without children).
/// This avoids using container bounds which may be stale after constraint solving.
fn collect_leaf_bounds(element: &ElementLayout, bounds: &mut Option<BoundingBox>) {
    if element.children.is_empty() {
        *bounds = Some(match bounds {
            Some(b) => b.union(&element.bounds),
            None => element.bounds,
        });
    } else {
        for child in &element.children {
            collect_leaf_bounds(child, bounds);
        }
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

// ============================================
// Local Solver Types (Feature 010)
// ============================================

/// Result of the local constraint solving phase for a single template instance.
///
/// During the two-phase constraint solver:
/// 1. Local phase: Solve constraints within each template instance independently
/// 2. This struct captures the solved positions before rotation transformation
/// 3. Global phase: Apply rotation, then solve cross-template constraints
#[derive(Debug, Clone)]
pub struct LocalSolverResult {
    /// The template instance identifier (e.g., "alice", "bob")
    pub template_instance: String,
    /// Bounds of all elements within this template instance, keyed by element name
    pub element_bounds: HashMap<String, BoundingBox>,
    /// Anchor sets for all elements within this template instance
    pub anchors: HashMap<String, AnchorSet>,
    /// Rotation angle in degrees (if template instance has rotation)
    pub rotation: Option<f64>,
    /// Pre-rotation bounds for elements within this template instance
    pub pre_rotation_bounds: HashMap<String, BoundingBox>,
    /// Pre-rotation anchors for elements within this template instance
    pub pre_rotation_anchors: HashMap<String, AnchorSet>,
    /// Rotation center for this template instance (if rotated)
    pub rotation_center: Option<Point>,
}

impl LocalSolverResult {
    /// Create a new local solver result for a template instance
    pub fn new(template_instance: impl Into<String>) -> Self {
        Self {
            template_instance: template_instance.into(),
            element_bounds: HashMap::new(),
            anchors: HashMap::new(),
            rotation: None,
            pre_rotation_bounds: HashMap::new(),
            pre_rotation_anchors: HashMap::new(),
            rotation_center: None,
        }
    }

    /// Set the rotation angle for this template instance
    pub fn with_rotation(mut self, angle_degrees: f64) -> Self {
        self.rotation = Some(angle_degrees);
        self
    }

    /// Add an element's bounds to this result
    pub fn add_element_bounds(&mut self, element_name: impl Into<String>, bounds: BoundingBox) {
        self.element_bounds.insert(element_name.into(), bounds);
    }

    /// Add an element's anchors to this result
    pub fn add_anchors(&mut self, element_name: impl Into<String>, anchors: AnchorSet) {
        self.anchors.insert(element_name.into(), anchors);
    }

    /// Get the bounding box that encompasses all elements in this template instance
    pub fn combined_bounds(&self) -> Option<BoundingBox> {
        let mut combined: Option<BoundingBox> = None;
        for bounds in self.element_bounds.values() {
            combined = Some(match combined {
                Some(existing) => existing.union(bounds),
                None => *bounds,
            });
        }
        combined
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
            anchors: AnchorSet::default(),
            path_normalize: true,
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

    // ============================================
    // Anchor Tests (Feature 009)
    // ============================================

    #[test]
    fn test_anchor_direction_to_vector() {
        let up = AnchorDirection::Up.to_vector();
        assert!((up.x - 0.0).abs() < 0.001);
        assert!((up.y - (-1.0)).abs() < 0.001);

        let down = AnchorDirection::Down.to_vector();
        assert!((down.x - 0.0).abs() < 0.001);
        assert!((down.y - 1.0).abs() < 0.001);

        let left = AnchorDirection::Left.to_vector();
        assert!((left.x - (-1.0)).abs() < 0.001);
        assert!((left.y - 0.0).abs() < 0.001);

        let right = AnchorDirection::Right.to_vector();
        assert!((right.x - 1.0).abs() < 0.001);
        assert!((right.y - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_anchor_direction_to_degrees() {
        assert_eq!(AnchorDirection::Up.to_degrees(), 270.0);
        assert_eq!(AnchorDirection::Down.to_degrees(), 90.0);
        assert_eq!(AnchorDirection::Left.to_degrees(), 180.0);
        assert_eq!(AnchorDirection::Right.to_degrees(), 0.0);
        assert_eq!(AnchorDirection::Angle(45.0).to_degrees(), 45.0);
    }

    #[test]
    fn test_anchor_direction_from_property() {
        assert_eq!(
            AnchorDirection::from_property(&ConstraintProperty::Left),
            AnchorDirection::Left
        );
        assert_eq!(
            AnchorDirection::from_property(&ConstraintProperty::Right),
            AnchorDirection::Right
        );
        assert_eq!(
            AnchorDirection::from_property(&ConstraintProperty::Top),
            AnchorDirection::Up
        );
        assert_eq!(
            AnchorDirection::from_property(&ConstraintProperty::Bottom),
            AnchorDirection::Down
        );
    }

    #[test]
    fn test_bounding_box_anchor_positions() {
        let bb = BoundingBox::new(0.0, 0.0, 100.0, 50.0);

        let top = bb.top_center();
        assert_eq!(top.x, 50.0);
        assert_eq!(top.y, 0.0);

        let bottom = bb.bottom_center();
        assert_eq!(bottom.x, 50.0);
        assert_eq!(bottom.y, 50.0);

        let left = bb.left_center();
        assert_eq!(left.x, 0.0);
        assert_eq!(left.y, 25.0);

        let right = bb.right_center();
        assert_eq!(right.x, 100.0);
        assert_eq!(right.y, 25.0);
    }

    #[test]
    fn test_bounding_box_corner_positions() {
        let bb = BoundingBox::new(10.0, 20.0, 100.0, 50.0);

        assert_eq!(bb.top_left(), Point::new(10.0, 20.0));
        assert_eq!(bb.top_right(), Point::new(110.0, 20.0));
        assert_eq!(bb.bottom_left(), Point::new(10.0, 70.0));
        assert_eq!(bb.bottom_right(), Point::new(110.0, 70.0));
    }

    #[test]
    fn test_simple_shape_anchors() {
        let bounds = BoundingBox::new(0.0, 0.0, 100.0, 50.0);
        let anchors = AnchorSet::simple_shape(&bounds);

        assert_eq!(anchors.len(), 4);

        let top = anchors.get("top").unwrap();
        assert_eq!(top.position, Point::new(50.0, 0.0));
        assert_eq!(top.direction, AnchorDirection::Up);

        let bottom = anchors.get("bottom").unwrap();
        assert_eq!(bottom.position, Point::new(50.0, 50.0));
        assert_eq!(bottom.direction, AnchorDirection::Down);

        let left = anchors.get("left").unwrap();
        assert_eq!(left.position, Point::new(0.0, 25.0));
        assert_eq!(left.direction, AnchorDirection::Left);

        let right = anchors.get("right").unwrap();
        assert_eq!(right.position, Point::new(100.0, 25.0));
        assert_eq!(right.direction, AnchorDirection::Right);
    }

    #[test]
    fn test_path_shape_anchors() {
        let bounds = BoundingBox::new(0.0, 0.0, 100.0, 50.0);
        let anchors = AnchorSet::path_shape(&bounds);

        // Should have 8 anchors: 4 edges + 4 corners
        assert_eq!(anchors.len(), 8);

        // Check corners exist with correct positions
        let tl = anchors.get("top_left").unwrap();
        assert_eq!(tl.position, Point::new(0.0, 0.0));
        assert_eq!(tl.direction, AnchorDirection::Angle(225.0));

        let tr = anchors.get("top_right").unwrap();
        assert_eq!(tr.position, Point::new(100.0, 0.0));
        assert_eq!(tr.direction, AnchorDirection::Angle(315.0));

        let bl = anchors.get("bottom_left").unwrap();
        assert_eq!(bl.position, Point::new(0.0, 50.0));
        assert_eq!(bl.direction, AnchorDirection::Angle(135.0));

        let br = anchors.get("bottom_right").unwrap();
        assert_eq!(br.position, Point::new(100.0, 50.0));
        assert_eq!(br.direction, AnchorDirection::Angle(45.0));
    }

    #[test]
    fn test_anchor_set_names() {
        let bounds = BoundingBox::new(0.0, 0.0, 100.0, 50.0);
        let anchors = AnchorSet::simple_shape(&bounds);

        let names: Vec<&str> = anchors.names().collect();
        assert!(names.contains(&"top"));
        assert!(names.contains(&"bottom"));
        assert!(names.contains(&"left"));
        assert!(names.contains(&"right"));
    }

    #[test]
    fn test_anchor_set_merge() {
        let bounds = BoundingBox::new(0.0, 0.0, 100.0, 50.0);
        let mut base = AnchorSet::simple_shape(&bounds);

        // Add custom anchor
        let mut custom = AnchorSet::new();
        custom.insert(Anchor::new(
            "input",
            Point::new(0.0, 10.0),
            AnchorDirection::Left,
        ));

        base.merge(&custom);

        // Should have 5 anchors now
        assert_eq!(base.len(), 5);
        assert!(base.get("input").is_some());
    }

    #[test]
    fn test_resolved_anchor_from_anchor() {
        let anchor = Anchor::new("test", Point::new(50.0, 25.0), AnchorDirection::Right);
        let resolved = ResolvedAnchor::from_anchor(&anchor);

        assert_eq!(resolved.position, anchor.position);
        assert_eq!(resolved.direction, anchor.direction);
    }

    #[test]
    fn test_for_element_type_rect() {
        let bounds = BoundingBox::new(10.0, 20.0, 100.0, 50.0);
        let element_type = ElementType::Shape(ShapeType::Rectangle);
        let anchors = AnchorSet::for_element_type(&element_type, &bounds);

        // Simple shapes get 4 anchors
        assert_eq!(anchors.len(), 4);
        assert!(anchors.get("top").is_some());
        assert!(anchors.get("bottom").is_some());
        assert!(anchors.get("left").is_some());
        assert!(anchors.get("right").is_some());
    }

    #[test]
    fn test_for_element_type_path() {
        use crate::parser::ast::{PathBody, PathDecl};
        let bounds = BoundingBox::new(10.0, 20.0, 100.0, 50.0);
        let element_type = ElementType::Shape(ShapeType::Path(PathDecl {
            name: None,
            body: PathBody { commands: vec![] },
            modifiers: vec![],
        }));
        let anchors = AnchorSet::for_element_type(&element_type, &bounds);

        // Path shapes get 8 anchors (4 edges + 4 corners)
        assert_eq!(anchors.len(), 8);
        assert!(anchors.get("top_left").is_some());
        assert!(anchors.get("top_right").is_some());
        assert!(anchors.get("bottom_left").is_some());
        assert!(anchors.get("bottom_right").is_some());
    }

    #[test]
    fn test_update_builtin_from_bounds() {
        // Create anchors at initial position
        let initial_bounds = BoundingBox::new(0.0, 0.0, 100.0, 50.0);
        let element_type = ElementType::Shape(ShapeType::Rectangle);
        let mut anchors = AnchorSet::for_element_type(&element_type, &initial_bounds);

        // Verify initial "top" anchor position
        let top = anchors.get("top").unwrap();
        assert_eq!(top.position, Point::new(50.0, 0.0));

        // Move element to new position
        let new_bounds = BoundingBox::new(200.0, 100.0, 100.0, 50.0);
        anchors.update_builtin_from_bounds(&element_type, &new_bounds);

        // Verify "top" anchor is updated
        let top = anchors.get("top").unwrap();
        assert_eq!(top.position, Point::new(250.0, 100.0));

        // Verify other anchors are also updated
        let bottom = anchors.get("bottom").unwrap();
        assert_eq!(bottom.position, Point::new(250.0, 150.0));

        let left = anchors.get("left").unwrap();
        assert_eq!(left.position, Point::new(200.0, 125.0));

        let right = anchors.get("right").unwrap();
        assert_eq!(right.position, Point::new(300.0, 125.0));
    }

    #[test]
    fn test_update_builtin_preserves_custom_anchors() {
        // Create anchors with a custom anchor
        let initial_bounds = BoundingBox::new(0.0, 0.0, 100.0, 50.0);
        let element_type = ElementType::Shape(ShapeType::Rectangle);
        let mut anchors = AnchorSet::for_element_type(&element_type, &initial_bounds);

        // Add a custom anchor
        anchors.insert(Anchor::new(
            "custom",
            Point::new(10.0, 10.0),
            AnchorDirection::Angle(45.0),
        ));
        assert_eq!(anchors.len(), 5);

        // Move element to new position
        let new_bounds = BoundingBox::new(200.0, 100.0, 100.0, 50.0);
        anchors.update_builtin_from_bounds(&element_type, &new_bounds);

        // Custom anchor should still exist with its ORIGINAL position
        // (custom anchors are NOT updated by update_builtin_from_bounds)
        let custom = anchors.get("custom").unwrap();
        assert_eq!(custom.position, Point::new(10.0, 10.0));
        assert_eq!(anchors.len(), 5);
    }

    // ============================================
    // Local Solver Result Tests (Feature 010)
    // ============================================

    #[test]
    fn test_local_solver_result_new() {
        let result = LocalSolverResult::new("alice");
        assert_eq!(result.template_instance, "alice");
        assert!(result.element_bounds.is_empty());
        assert!(result.anchors.is_empty());
        assert!(result.rotation.is_none());
    }

    #[test]
    fn test_local_solver_result_with_rotation() {
        let result = LocalSolverResult::new("bob").with_rotation(90.0);
        assert_eq!(result.template_instance, "bob");
        assert_eq!(result.rotation, Some(90.0));
    }

    #[test]
    fn test_local_solver_result_add_element_bounds() {
        let mut result = LocalSolverResult::new("alice");
        let bounds = BoundingBox::new(10.0, 20.0, 100.0, 50.0);
        result.add_element_bounds("head", bounds);

        assert_eq!(result.element_bounds.len(), 1);
        assert_eq!(result.element_bounds.get("head"), Some(&bounds));
    }

    #[test]
    fn test_local_solver_result_add_anchors() {
        let mut result = LocalSolverResult::new("alice");
        let bounds = BoundingBox::new(0.0, 0.0, 100.0, 50.0);
        let anchors = AnchorSet::simple_shape(&bounds);
        result.add_anchors("head", anchors.clone());

        assert_eq!(result.anchors.len(), 1);
        assert!(result.anchors.get("head").is_some());
    }

    #[test]
    fn test_local_solver_result_combined_bounds_empty() {
        let result = LocalSolverResult::new("alice");
        assert!(result.combined_bounds().is_none());
    }

    #[test]
    fn test_local_solver_result_combined_bounds_single() {
        let mut result = LocalSolverResult::new("alice");
        let bounds = BoundingBox::new(10.0, 20.0, 100.0, 50.0);
        result.add_element_bounds("head", bounds);

        let combined = result.combined_bounds().unwrap();
        assert_eq!(combined.x, 10.0);
        assert_eq!(combined.y, 20.0);
        assert_eq!(combined.width, 100.0);
        assert_eq!(combined.height, 50.0);
    }

    #[test]
    fn test_local_solver_result_combined_bounds_multiple() {
        let mut result = LocalSolverResult::new("alice");

        // Head at (0, 0) with size 30x30
        result.add_element_bounds("head", BoundingBox::new(0.0, 0.0, 30.0, 30.0));
        // Torso at (0, 40) with size 30x50
        result.add_element_bounds("torso", BoundingBox::new(0.0, 40.0, 30.0, 50.0));

        let combined = result.combined_bounds().unwrap();
        // Combined should be from (0,0) to (30, 90)
        assert_eq!(combined.x, 0.0);
        assert_eq!(combined.y, 0.0);
        assert_eq!(combined.width, 30.0);
        assert_eq!(combined.height, 90.0);
    }
}
