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
    /// Connection(s): `a -> b` or chained `a -> b -> c [styles]`
    Connection(Vec<ConnectionDecl>),
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
    /// Constrain statement: `constrain a.left = b.left`
    Constrain(ConstrainDecl),
    /// Template declaration: `template "name" { ... }` or `template "name" from "path"`
    TemplateDecl(TemplateDecl),
    /// Template instance: `template_name "instance_name" [params]`
    TemplateInstance(TemplateInstance),
    /// Export declaration: `export port1, port2`
    Export(ExportDecl),
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
    Icon {
        icon_name: String,
    },
    Text {
        content: String,
    },
    /// Embedded SVG content from template instantiation
    SvgEmbed {
        content: String,
        intrinsic_width: Option<f64>,
        intrinsic_height: Option<f64>,
    },
    /// Custom path shape (Feature 007)
    Path(PathDecl),
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

/// Position constraint
/// Supports both relational positioning and direct offsets:
/// - `place a right-of b` - relative positioning
/// - `place a [x: 10, y: 20]` - absolute or offset positioning
/// - `place a right-of b [x: 10]` - relative with additional offset
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintDecl {
    pub subject: Spanned<Identifier>,
    /// Optional relation (right-of, left-of, etc.)
    pub relation: Option<Spanned<PositionRelation>>,
    /// Optional anchor element (required if relation is specified)
    pub anchor: Option<Spanned<Identifier>>,
    /// Optional position modifiers (x, y offsets)
    pub modifiers: Vec<Spanned<StyleModifier>>,
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
    /// X position offset (used with place constraints)
    X,
    /// Y position offset (used with place constraints)
    Y,
    /// Stroke dash pattern (e.g., "4,2" for dashed lines)
    StrokeDasharray,
    /// Rotation angle in degrees (clockwise positive)
    Rotation,
    Custom(String),
}

/// Style values
#[derive(Debug, Clone, PartialEq)]
pub enum StyleValue {
    Color(ColorValue),
    Number {
        value: f64,
        unit: Option<String>,
    },
    String(String),
    Keyword(String),
    /// Identifier reference (for `[label: my_shape]` syntax)
    Identifier(Identifier),
    /// List of identifiers (for `[via: c1, c2, c3]` syntax - Feature 008)
    IdentifierList(Vec<Identifier>),
}

// ============================================
// Template Types (Feature 005)
// ============================================

/// Source type for templates
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateSourceType {
    /// Inline template: `template "name" { ... }`
    Inline,
    /// SVG file import: `template "name" from "file.svg"`
    Svg,
    /// AIL file import: `template "name" from "file.ail"`
    Ail,
}

/// Parameter definition with default value
#[derive(Debug, Clone, PartialEq)]
pub struct ParameterDef {
    pub name: Spanned<Identifier>,
    pub default_value: Spanned<StyleValue>,
}

/// Template declaration
#[derive(Debug, Clone, PartialEq)]
pub struct TemplateDecl {
    pub name: Spanned<Identifier>,
    pub source_type: TemplateSourceType,
    pub source_path: Option<Spanned<String>>,
    pub parameters: Vec<ParameterDef>,
    pub body: Option<Vec<Spanned<Statement>>>,
}

/// Template instance: template_name "instance_name" [params]
#[derive(Debug, Clone, PartialEq)]
pub struct TemplateInstance {
    pub template_name: Spanned<Identifier>,
    pub instance_name: Spanned<Identifier>,
    pub arguments: Vec<(Spanned<Identifier>, Spanned<StyleValue>)>,
}

/// Export declaration: export port1, port2
#[derive(Debug, Clone, PartialEq)]
pub struct ExportDecl {
    pub exports: Vec<Spanned<Identifier>>,
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
        &self
            .segments
            .last()
            .expect("ElementPath must have at least one segment")
            .node
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

// ============================================
// Constraint Types (Feature 005)
// ============================================

/// Properties that can be referenced in constraints
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintProperty {
    // Position
    X,
    Y,
    // Size
    Width,
    Height,
    // Edges
    Left,
    Right,
    Top,
    Bottom,
    // Centers
    CenterX,
    CenterY,
    Center, // Both center_x and center_y
}

impl ConstraintProperty {
    /// Parse from string (for parser integration)
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "x" => Some(Self::X),
            "y" => Some(Self::Y),
            "width" => Some(Self::Width),
            "height" => Some(Self::Height),
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            "top" => Some(Self::Top),
            "bottom" => Some(Self::Bottom),
            "center_x" | "horizontal_center" => Some(Self::CenterX),
            "center_y" | "vertical_center" => Some(Self::CenterY),
            "center" => Some(Self::Center),
            _ => None,
        }
    }
}

/// Reference to an element's property
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyRef {
    pub element: Spanned<ElementPath>,
    pub property: Spanned<ConstraintProperty>,
}

/// Expression in a constrain statement
#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintExpr {
    /// a.prop = b.prop
    Equal {
        left: PropertyRef,
        right: PropertyRef,
    },
    /// a.prop = b.prop + offset
    EqualWithOffset {
        left: PropertyRef,
        right: PropertyRef,
        offset: f64,
    },
    /// a.prop = constant
    Constant { left: PropertyRef, value: f64 },
    /// a.prop >= value
    GreaterOrEqual { left: PropertyRef, value: f64 },
    /// a.prop <= value
    LessOrEqual { left: PropertyRef, value: f64 },
    /// a.center = midpoint(b, c) or a.center = midpoint(b, c) + offset
    Midpoint {
        target: PropertyRef,
        a: Spanned<Identifier>,
        b: Spanned<Identifier>,
        /// Offset to add to midpoint (0.0 for no offset)
        offset: f64,
    },
    /// container contains a, b, c [padding: 20]
    Contains {
        container: Spanned<Identifier>,
        elements: Vec<Spanned<Identifier>>,
        padding: Option<f64>,
    },
}

/// Constrain statement declaration
#[derive(Debug, Clone, PartialEq)]
pub struct ConstrainDecl {
    pub expr: ConstraintExpr,
}

// ============================================
// Path Shape Types (Feature 007)
// ============================================

/// Arc sweep direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SweepDirection {
    #[default]
    Clockwise,
    Counterclockwise,
}

/// Arc curve parameters
#[derive(Debug, Clone, PartialEq)]
pub enum ArcParams {
    /// Radius-based arc: `[radius: 20, sweep: clockwise]`
    Radius { radius: f64, sweep: SweepDirection },
    /// Bulge-based arc: `[bulge: 0.3]`
    Bulge(f64),
}

impl Default for ArcParams {
    fn default() -> Self {
        ArcParams::Bulge(0.414) // tan(π/8) - gentle quarter-circle
    }
}

/// Vertex position specification
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VertexPosition {
    /// X offset from origin
    pub x: Option<f64>,
    /// Y offset from origin
    pub y: Option<f64>,
}

/// Vertex declaration
#[derive(Debug, Clone, PartialEq)]
pub struct VertexDecl {
    /// Vertex name (required for referencing)
    pub name: Spanned<Identifier>,
    /// Optional position (relative to shape origin)
    pub position: Option<VertexPosition>,
}

/// Line segment declaration
#[derive(Debug, Clone, PartialEq)]
pub struct LineToDecl {
    /// Target vertex (existing or implicit)
    pub target: Spanned<Identifier>,
    /// Optional position for implicit vertex creation
    pub position: Option<VertexPosition>,
}

/// Arc segment declaration
#[derive(Debug, Clone, PartialEq)]
pub struct ArcToDecl {
    /// Target vertex (existing or implicit)
    pub target: Spanned<Identifier>,
    /// Optional position for implicit vertex creation
    pub position: Option<VertexPosition>,
    /// Arc parameters (radius, bulge, sweep)
    pub params: ArcParams,
}

/// Quadratic Bezier curve segment declaration (Feature 008)
#[derive(Debug, Clone, PartialEq)]
pub struct CurveToDecl {
    /// Target vertex (existing or implicit)
    pub target: Spanned<Identifier>,
    /// Optional steering vertex reference (control point)
    /// When None, system auto-generates control point
    pub via: Option<Spanned<Identifier>>,
    /// Optional position for implicit vertex creation
    pub position: Option<VertexPosition>,
}

/// Commands that can appear inside a path block
#[derive(Debug, Clone, PartialEq)]
pub enum PathCommand {
    /// Explicit vertex declaration: `vertex name [position]`
    Vertex(VertexDecl),
    /// Straight line segment: `line_to target [position]`
    LineTo(LineToDecl),
    /// Arc segment: `arc_to target [arc_params]`
    ArcTo(ArcToDecl),
    /// Quadratic Bezier curve segment: `curve_to target [via: control, position]` (Feature 008)
    CurveTo(CurveToDecl),
    /// Close path with straight line: `close`
    Close,
    /// Close path with arc: `close_arc [arc_params]`
    CloseArc(ArcParams),
}

/// The body of a path shape
#[derive(Debug, Clone, PartialEq)]
pub struct PathBody {
    /// Sequence of path commands (vertices, segments, close)
    pub commands: Vec<Spanned<PathCommand>>,
}

/// Path shape declaration
#[derive(Debug, Clone, PartialEq)]
pub struct PathDecl {
    /// Shape name (optional)
    pub name: Option<Spanned<Identifier>>,
    /// Path body: vertices and segments
    pub body: PathBody,
    /// Style modifiers (fill, stroke, etc.)
    pub modifiers: Vec<Spanned<StyleModifier>>,
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
}
