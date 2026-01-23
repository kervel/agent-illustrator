//! Abstract Syntax Tree types for the Agent Illustrator DSL

/// Byte range in source text
pub type Span = std::ops::Range<usize>;

/// Semantic color categories for brand-agnostic illustrations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColorCategory {
    Foreground,
    Background,
    Text,
    Accent,
}

/// Light/dark modifier for colors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lightness {
    Light,
    Dark,
}

/// A color value - either concrete (hex/named) or symbolic (resolved at render time)
#[derive(Debug, Clone, PartialEq)]
pub enum ColorValue {
    /// Hex color like #ff0000 or #f00
    Hex(String),
    /// Named SVG color like red, blue (passed to SVG as-is)
    Named(String),
    /// Symbolic token like foreground-1, text-dark (resolved via stylesheet)
    Symbolic {
        category: ColorCategory,
        variant: Option<u8>,
        lightness: Option<Lightness>,
    },
}

impl ColorValue {
    /// Convert to string representation for stylesheet lookup
    ///
    /// Returns Some for Symbolic colors, None for concrete colors.
    pub fn token_string(&self) -> Option<String> {
        match self {
            ColorValue::Symbolic {
                category,
                variant,
                lightness,
            } => {
                let cat = match category {
                    ColorCategory::Foreground => "foreground",
                    ColorCategory::Background => "background",
                    ColorCategory::Text => "text",
                    ColorCategory::Accent => "accent",
                };
                let mut s = cat.to_string();
                if let Some(v) = variant {
                    s.push_str(&format!("-{}", v));
                }
                if let Some(l) = lightness {
                    s.push_str(match l {
                        Lightness::Light => "-light",
                        Lightness::Dark => "-dark",
                    });
                }
                Some(s)
            }
            _ => None,
        }
    }

    /// Get the concrete color string for hex or named colors
    pub fn concrete_string(&self) -> Option<&str> {
        match self {
            ColorValue::Hex(s) | ColorValue::Named(s) => Some(s.as_str()),
            ColorValue::Symbolic { .. } => None,
        }
    }
}

/// AST node with source location
#[derive(Debug, Clone, PartialEq)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(node: T, span: Span) -> Self {
        Self { node, span }
    }
}

/// Valid identifier (alphanumeric + underscore, starts with letter/_)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Identifier(pub String);

impl Identifier {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Root AST node - a complete illustration document
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub statements: Vec<Spanned<Statement>>,
}

/// Top-level statement in a document
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// Shape declaration: `rect "name" [styles]`
    Shape(ShapeDecl),
    /// Connection: `a -> b [styles]`
    Connection(ConnectionDecl),
    /// Layout container: `row { ... }`
    Layout(LayoutDecl),
    /// Semantic group: `group "name" { ... }`
    Group(GroupDecl),
    /// Position constraint: `place a right-of b`
    Constraint(ConstraintDecl),
    /// Label element: `label { text "Foo" }` or `label: text "Foo"`
    /// Contains any statement that acts as a label for its parent container
    /// DEPRECATED: Use `[role: label]` modifier instead
    Label(Box<Statement>),
    /// Alignment constraint: `align a.left = b.left`
    Alignment(AlignmentDecl),
}

/// Shape declaration
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeDecl {
    pub shape_type: Spanned<ShapeType>,
    pub name: Option<Spanned<Identifier>>,
    pub modifiers: Vec<Spanned<StyleModifier>>,
}

/// Built-in shape types
#[derive(Debug, Clone, PartialEq)]
pub enum ShapeType {
    Rectangle,
    Circle,
    Ellipse,
    Line,
    Polygon,
    Icon { icon_name: String },
    Text { content: String },
}

/// Connection between shapes
#[derive(Debug, Clone, PartialEq)]
pub struct ConnectionDecl {
    pub from: Spanned<Identifier>,
    pub to: Spanned<Identifier>,
    pub direction: ConnectionDirection,
    pub modifiers: Vec<Spanned<StyleModifier>>,
}

/// Connection directionality
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionDirection {
    /// `->` directed from source to target
    Forward,
    /// `<-` directed from target to source
    Backward,
    /// `<->` bidirectional
    Bidirectional,
    /// `--` undirected
    Undirected,
}

/// Layout container
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutDecl {
    pub layout_type: Spanned<LayoutType>,
    pub name: Option<Spanned<Identifier>>,
    pub children: Vec<Spanned<Statement>>,
    pub modifiers: Vec<Spanned<StyleModifier>>,
}

/// Layout arrangement strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutType {
    Row,
    Column,
    Grid,
    Stack,
}

/// Semantic group (no layout implication)
#[derive(Debug, Clone, PartialEq)]
pub struct GroupDecl {
    pub name: Option<Spanned<Identifier>>,
    pub children: Vec<Spanned<Statement>>,
    pub modifiers: Vec<Spanned<StyleModifier>>,
}

/// Position constraint (experimental)
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintDecl {
    pub subject: Spanned<Identifier>,
    pub relation: Spanned<PositionRelation>,
    pub anchor: Spanned<Identifier>,
}

/// Relative position relations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionRelation {
    RightOf,
    LeftOf,
    Above,
    Below,
    Inside,
}

/// Key-value style modifier
#[derive(Debug, Clone, PartialEq)]
pub struct StyleModifier {
    pub key: Spanned<StyleKey>,
    pub value: Spanned<StyleValue>,
}

/// Known style keys (extensible)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StyleKey {
    Fill,
    Stroke,
    StrokeWidth,
    Opacity,
    Label,
    /// Position of a connection label (left, right, or center)
    LabelPosition,
    FontSize,
    Class,
    /// Gap between elements in a layout (can be negative for overlap)
    Gap,
    /// Size for shapes (creates square/circle with this dimension)
    Size,
    /// Explicit width for shapes
    Width,
    /// Explicit height for shapes
    Height,
    /// Routing mode for connections (direct or orthogonal)
    Routing,
    /// Role modifier for shape positioning (e.g., `role: label`)
    Role,
    Custom(String),
}

/// Style values
#[derive(Debug, Clone, PartialEq)]
pub enum StyleValue {
    Color(ColorValue),
    Number { value: f64, unit: Option<String> },
    String(String),
    Keyword(String),
    /// Identifier reference (for `[label: my_shape]` syntax)
    Identifier(Identifier),
}

// ============================================
// Alignment Types (Feature 004)
// ============================================

/// Axis type for alignment compatibility
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

/// Alignment edge on an element's bounding box
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    // Horizontal axis (affects x-coordinate)
    Left,
    HorizontalCenter,
    Right,
    // Vertical axis (affects y-coordinate)
    Top,
    VerticalCenter,
    Bottom,
}

impl Edge {
    /// Returns true if this edge is horizontal (affects x-position)
    pub fn is_horizontal(&self) -> bool {
        matches!(self, Edge::Left | Edge::HorizontalCenter | Edge::Right)
    }

    /// Returns true if this edge is vertical (affects y-position)
    pub fn is_vertical(&self) -> bool {
        matches!(self, Edge::Top | Edge::VerticalCenter | Edge::Bottom)
    }

    /// Get axis type for compatibility checking
    pub fn axis(&self) -> Axis {
        if self.is_horizontal() {
            Axis::Horizontal
        } else {
            Axis::Vertical
        }
    }
}

/// Path to an element through the group hierarchy
/// Examples: "my_element", "group1.item", "outer.inner.shape"
#[derive(Debug, Clone, PartialEq)]
pub struct ElementPath {
    /// Path segments (identifiers separated by dots)
    pub segments: Vec<Spanned<Identifier>>,
}

impl ElementPath {
    /// Create a simple path (single segment)
    pub fn simple(id: Identifier, span: Span) -> Self {
        Self {
            segments: vec![Spanned::new(id, span)],
        }
    }

    /// Get the final segment (leaf element name)
    pub fn leaf(&self) -> &Identifier {
        &self.segments.last().expect("ElementPath must have at least one segment").node
    }

    /// Check if this is a simple (single-segment) path
    pub fn is_simple(&self) -> bool {
        self.segments.len() == 1
    }
}

impl std::fmt::Display for ElementPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let path_str: Vec<&str> = self.segments.iter().map(|s| s.node.as_str()).collect();
        write!(f, "{}", path_str.join("."))
    }
}

/// A specific alignment anchor point
#[derive(Debug, Clone, PartialEq)]
pub struct AlignmentAnchor {
    /// Path to the element
    pub element: Spanned<ElementPath>,
    /// Edge of the element to align
    pub edge: Spanned<Edge>,
}

/// Alignment constraint: aligns edges of multiple elements
/// Example: `align a.left = b.left = c.left`
#[derive(Debug, Clone, PartialEq)]
pub struct AlignmentDecl {
    /// Anchors to align (at least 2)
    pub anchors: Vec<AlignmentAnchor>,
}

impl AlignmentDecl {
    /// Check that all anchors are on the same axis
    pub fn is_valid(&self) -> bool {
        if self.anchors.len() < 2 {
            return false;
        }
        let first_axis = self.anchors[0].edge.node.axis();
        self.anchors.iter().all(|a| a.edge.node.axis() == first_axis)
    }

    /// Get the axis of this alignment
    pub fn axis(&self) -> Option<Axis> {
        self.anchors.first().map(|a| a.edge.node.axis())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_axis_classification() {
        assert!(Edge::Left.is_horizontal());
        assert!(Edge::HorizontalCenter.is_horizontal());
        assert!(Edge::Right.is_horizontal());
        assert!(!Edge::Left.is_vertical());

        assert!(Edge::Top.is_vertical());
        assert!(Edge::VerticalCenter.is_vertical());
        assert!(Edge::Bottom.is_vertical());
        assert!(!Edge::Top.is_horizontal());

        assert_eq!(Edge::Left.axis(), Axis::Horizontal);
        assert_eq!(Edge::Top.axis(), Axis::Vertical);
    }

    #[test]
    fn test_element_path_simple() {
        let path = ElementPath::simple(Identifier::new("foo"), 0..3);
        assert!(path.is_simple());
        assert_eq!(path.leaf().as_str(), "foo");
        assert_eq!(path.to_string(), "foo");
    }

    #[test]
    fn test_element_path_nested() {
        let path = ElementPath {
            segments: vec![
                Spanned::new(Identifier::new("group1"), 0..6),
                Spanned::new(Identifier::new("item"), 7..11),
                Spanned::new(Identifier::new("child"), 12..17),
            ],
        };
        assert!(!path.is_simple());
        assert_eq!(path.leaf().as_str(), "child");
        assert_eq!(path.to_string(), "group1.item.child");
    }

    #[test]
    fn test_alignment_decl_validation() {
        // Valid: two anchors on same axis
        let valid = AlignmentDecl {
            anchors: vec![
                AlignmentAnchor {
                    element: Spanned::new(ElementPath::simple(Identifier::new("a"), 0..1), 0..1),
                    edge: Spanned::new(Edge::Left, 2..6),
                },
                AlignmentAnchor {
                    element: Spanned::new(ElementPath::simple(Identifier::new("b"), 9..10), 9..10),
                    edge: Spanned::new(Edge::Left, 11..15),
                },
            ],
        };
        assert!(valid.is_valid());
        assert_eq!(valid.axis(), Some(Axis::Horizontal));

        // Invalid: only one anchor
        let invalid_single = AlignmentDecl {
            anchors: vec![AlignmentAnchor {
                element: Spanned::new(ElementPath::simple(Identifier::new("a"), 0..1), 0..1),
                edge: Spanned::new(Edge::Left, 2..6),
            }],
        };
        assert!(!invalid_single.is_valid());

        // Mixed axes is still structurally valid (semantic validation happens later)
        let mixed = AlignmentDecl {
            anchors: vec![
                AlignmentAnchor {
                    element: Spanned::new(ElementPath::simple(Identifier::new("a"), 0..1), 0..1),
                    edge: Spanned::new(Edge::Left, 2..6),
                },
                AlignmentAnchor {
                    element: Spanned::new(ElementPath::simple(Identifier::new("b"), 9..10), 9..10),
                    edge: Spanned::new(Edge::Top, 11..14),
                },
            ],
        };
        // is_valid checks axis consistency
        assert!(!mixed.is_valid());
    }
}
