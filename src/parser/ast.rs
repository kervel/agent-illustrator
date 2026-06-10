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
    Secondary,
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
                    ColorCategory::Secondary => "secondary",
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
    /// Anchor declaration: `anchor name [position: element.property]` (Feature 009)
    AnchorDecl(AnchorDecl),
    /// Keyframe declaration: `keyframe "name" { show/hide/transform ... }` (Feature 011)
    Keyframe(KeyframeDecl),
}

/// Shape declaration
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeDecl {
    pub shape_type: Spanned<ShapeType>,
    pub name: Option<Spanned<Identifier>>,
    pub modifiers: Vec<Spanned<StyleModifier>>,
}

/// Internal element id for a grid cell, addressable via `grid.cell(row, col)`.
/// Shared between the parser (which desugars `cell(r,c)`) and the layout engine
/// (which emits the reference-only cell elements) so the names always agree.
pub fn grid_cell_id(grid: &str, row: usize, col: usize) -> String {
    format!("{}__cell_{}_{}", grid, row, col)
}

/// Direction a callout's pointer (tail) faces; also where its `tip` anchor sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PointerDir {
    Up,
    #[default]
    Down,
    Left,
    Right,
}

/// Built-in shape types
#[derive(Debug, Clone, PartialEq)]
pub enum ShapeType {
    Rectangle,
    Circle,
    Ellipse,
    Line,
    Polygon,
    /// Annotation pill with a triangular pointer; exposes a `tip` anchor at the
    /// pointer apex (the pointer-side edge center).
    Callout {
        pointer: PointerDir,
    },
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
        /// Content bbox origin to subtract so the artwork fills the element rect
        /// (0,0 when not trimmed).
        offset_x: f64,
        offset_y: f64,
    },
    /// Raster image reference (png, jpg, gif, webp, bmp)
    RasterImage {
        /// Path to the image file (relative to template base path)
        path: String,
    },
    /// Custom path shape (Feature 007)
    Path(PathDecl),
}

/// Connection between shapes
/// Updated in Feature 009 to support anchor references
/// Updated in Feature 011 to support named connections via `as` syntax
#[derive(Debug, Clone, PartialEq)]
pub struct ConnectionDecl {
    /// Source element with optional anchor (e.g., `box_a.right`)
    pub from: AnchorReference,
    /// Target element with optional anchor (e.g., `box_b.left`)
    pub to: AnchorReference,
    pub direction: ConnectionDirection,
    pub modifiers: Vec<Spanned<StyleModifier>>,
    /// Optional name for referencing in keyframes (e.g., `a -> b as req_arrow`)
    pub name: Option<Spanned<Identifier>>,
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
    /// Custom anchor declarations (Feature 009 - from template expansion)
    pub anchors: Vec<AnchorDecl>,
    /// Whether this group was created by template expansion (vs user-authored)
    pub is_template_instance: bool,
}

/// Keyframe declaration (Feature 011)
/// `keyframe "name" { show a, b; hide c; transform d [rotation: 45] }`
/// `keyframe "name" [no_resolve] { ... }` skips constraint re-solving
#[derive(Debug, Clone, PartialEq)]
pub struct KeyframeDecl {
    pub name: Spanned<String>,
    pub operations: Vec<Spanned<KeyframeOp>>,
    /// If true, skip constraint re-solving for this keyframe
    pub no_resolve: bool,
}

/// Keyframe operation (Feature 011)
#[derive(Debug, Clone, PartialEq)]
pub enum KeyframeOp {
    /// Make elements/connections visible
    Show(Vec<Spanned<Identifier>>),
    /// Make elements/connections invisible
    Hide(Vec<Spanned<Identifier>>),
    /// Apply per-frame style/position overrides
    Transform {
        target: Spanned<Identifier>,
        modifiers: Vec<Spanned<StyleModifier>>,
    },
    /// Activate a constraint for this frame forward (cumulative).
    Constrain(ConstrainDecl),
    /// Deactivate named constraints from this frame forward.
    Disable(Vec<Spanned<Identifier>>),
    /// Reactivate previously disabled named constraints.
    Enable(Vec<Spanned<Identifier>>),
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
    /// Alpha for the fill only (emits SVG fill-opacity)
    FillOpacity,
    /// Alpha for the stroke only (emits SVG stroke-opacity)
    StrokeOpacity,
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
    /// Label position along connection path (0.0=start, 1.0=end, default 0.5)
    LabelAt,
    /// Perpendicular offset distance for connection labels (default 10.0)
    LabelOffset,
    /// Z-order for controlling render order (higher = on top, groups only)
    ZOrder,
    /// Pointer (tail) direction for callout shapes
    Pointer,
    /// Delta-X: position offset relative to the laid-out position (keyframe transforms)
    Dx,
    /// Delta-Y: position offset relative to the laid-out position (keyframe transforms)
    Dy,
    /// Uniform scale about the element's center (keyframe transforms)
    Scale,
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
    /// Bracketed list of values (e.g. `at: [1, 0]`, `col_labels: ["a", "b"]`)
    List(Vec<Spanned<StyleValue>>),
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
    /// Raster image import: `template "name" from "file.png"` (png, jpg, gif, webp, bmp)
    Raster,
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
    // Anchor coordinates (Feature 011)
    /// X-coordinate of a named anchor (e.g., "drain" from "drain_x")
    AnchorX(String),
    /// Y-coordinate of a named anchor (e.g., "gate" from "gate_y")
    AnchorY(String),
    /// A whole named anchor used as a point (e.g. `tag.tip`). Only meaningful in
    /// point-constraints, which desugar it into AnchorX/AnchorY before solving.
    Anchor(String),
}

impl ConstraintProperty {
    /// Parse from string (for parser integration)
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            // Built-in properties FIRST — order matters for precedence
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
            // Anchor fallback — ONLY reached if not a built-in property
            _ if s.ends_with("_x") => {
                let anchor_name = &s[..s.len() - 2];
                Some(Self::AnchorX(anchor_name.to_string()))
            }
            _ if s.ends_with("_y") => {
                let anchor_name = &s[..s.len() - 2];
                Some(Self::AnchorY(anchor_name.to_string()))
            }
            // Any other bare name is a whole-anchor reference (e.g. `tip`),
            // valid only in point-constraints; resolution errors if it doesn't exist.
            _ => Some(Self::Anchor(s.to_string())),
        }
    }

    /// Whether this property denotes a point (both axes) rather than a scalar.
    pub fn is_point(&self) -> bool {
        matches!(self, Self::Anchor(_))
    }

    /// The horizontal-coordinate property of the point this reference denotes.
    pub fn x_component(&self) -> ConstraintProperty {
        match self {
            Self::Left | Self::X => Self::Left,
            Self::Right => Self::Right,
            Self::Anchor(n) | Self::AnchorX(n) | Self::AnchorY(n) => Self::AnchorX(n.clone()),
            _ => Self::CenterX,
        }
    }

    /// The vertical-coordinate property of the point this reference denotes.
    pub fn y_component(&self) -> ConstraintProperty {
        match self {
            Self::Top | Self::Y => Self::Top,
            Self::Bottom => Self::Bottom,
            Self::Anchor(n) | Self::AnchorX(n) | Self::AnchorY(n) => Self::AnchorY(n.clone()),
            _ => Self::CenterY,
        }
    }

    /// Which axis an offset on this reference applies to in a point-constraint.
    /// Horizontal edges → Y; vertical edges → X; otherwise Y by default.
    pub fn offset_is_horizontal(&self) -> bool {
        matches!(self, Self::Left | Self::Right | Self::X | Self::CenterX)
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
    /// Optional name (`constrain <expr> as <name>`) — handle for keyframe disable/enable.
    pub name: Option<Spanned<Identifier>>,
}

/// Build a copy of a property reference with a different property.
fn property_ref_with(pr: &PropertyRef, prop: ConstraintProperty) -> PropertyRef {
    PropertyRef {
        element: pr.element.clone(),
        property: Spanned::new(prop, pr.property.span.clone()),
    }
}

/// Read a callout's `pointer:` direction from its modifiers (default down).
fn pointer_from_modifiers(modifiers: &[Spanned<StyleModifier>]) -> PointerDir {
    modifiers
        .iter()
        .find_map(|m| {
            if matches!(m.node.key.node, StyleKey::Pointer) {
                if let StyleValue::Keyword(k) = &m.node.value.node {
                    return match k.as_str() {
                        "up" => Some(PointerDir::Up),
                        "down" => Some(PointerDir::Down),
                        "left" => Some(PointerDir::Left),
                        "right" => Some(PointerDir::Right),
                        _ => None,
                    };
                }
            }
            None
        })
        .unwrap_or_default()
}

/// For a callout's `tip` point, the (x, y) box-edge properties that coincide
/// with the pointer apex. Using the callout's own box edges (rather than the
/// `tip` anchor) keeps the callout the target that moves.
fn callout_tip_components(pointer: PointerDir) -> (ConstraintProperty, ConstraintProperty) {
    match pointer {
        PointerDir::Down => (ConstraintProperty::CenterX, ConstraintProperty::Bottom),
        PointerDir::Up => (ConstraintProperty::CenterX, ConstraintProperty::Top),
        PointerDir::Left => (ConstraintProperty::Left, ConstraintProperty::CenterY),
        PointerDir::Right => (ConstraintProperty::Right, ConstraintProperty::CenterY),
    }
}

/// Collect `name -> pointer` for every callout shape in the document.
fn collect_callout_pointers(stmts: &[Spanned<Statement>], map: &mut std::collections::HashMap<String, PointerDir>) {
    for stmt in stmts {
        match &stmt.node {
            Statement::Shape(s) => {
                if matches!(s.shape_type.node, ShapeType::Callout { .. }) {
                    if let Some(name) = &s.name {
                        map.insert(name.node.0.clone(), pointer_from_modifiers(&s.modifiers));
                    }
                }
            }
            Statement::Layout(l) => collect_callout_pointers(&l.children, map),
            Statement::Group(g) => collect_callout_pointers(&g.children, map),
            _ => {}
        }
    }
}

/// The (x, y) component properties of the left side of a point-constraint.
/// A callout `tip` resolves to the callout's box edges; any other anchor falls
/// back to its AnchorX/AnchorY scalars.
fn left_components(
    left: &PropertyRef,
    callouts: &std::collections::HashMap<String, PointerDir>,
) -> (ConstraintProperty, ConstraintProperty) {
    if let ConstraintProperty::Anchor(name) = &left.property.node {
        if name == "tip" {
            let elem = left.element.node.leaf().0.as_str();
            if let Some(pointer) = callouts.get(elem) {
                return callout_tip_components(*pointer);
            }
        }
    }
    (
        left.property.node.x_component(),
        left.property.node.y_component(),
    )
}

/// Expand a point-constraint (`A.tip = B.top [± off]`, where the left side is a
/// whole-anchor point) into its two scalar component constraints. Returns the
/// original expression unchanged when it is not a point-constraint.
fn expand_point_expr(
    expr: &ConstraintExpr,
    callouts: &std::collections::HashMap<String, PointerDir>,
) -> Vec<ConstraintExpr> {
    match expr {
        ConstraintExpr::Equal { left, right } if left.property.node.is_point() => {
            let (lxp, lyp) = left_components(left, callouts);
            let lx = property_ref_with(left, lxp);
            let rx = property_ref_with(right, right.property.node.x_component());
            let ly = property_ref_with(left, lyp);
            let ry = property_ref_with(right, right.property.node.y_component());
            vec![
                ConstraintExpr::Equal {
                    left: lx,
                    right: rx,
                },
                ConstraintExpr::Equal {
                    left: ly,
                    right: ry,
                },
            ]
        }
        ConstraintExpr::EqualWithOffset {
            left,
            right,
            offset,
        } if left.property.node.is_point() => {
            let horizontal = right.property.node.offset_is_horizontal();
            let (lxp, lyp) = left_components(left, callouts);
            let lx = property_ref_with(left, lxp);
            let rx = property_ref_with(right, right.property.node.x_component());
            let ly = property_ref_with(left, lyp);
            let ry = property_ref_with(right, right.property.node.y_component());
            let x_expr = if horizontal {
                ConstraintExpr::EqualWithOffset {
                    left: lx,
                    right: rx,
                    offset: *offset,
                }
            } else {
                ConstraintExpr::Equal {
                    left: lx,
                    right: rx,
                }
            };
            let y_expr = if horizontal {
                ConstraintExpr::Equal {
                    left: ly,
                    right: ry,
                }
            } else {
                ConstraintExpr::EqualWithOffset {
                    left: ly,
                    right: ry,
                    offset: *offset,
                }
            };
            vec![x_expr, y_expr]
        }
        other => vec![other.clone()],
    }
}

/// Rewrite a document so every point-constraint becomes two scalar constraints.
/// Run after template resolution and before layout/constraint solving.
pub fn expand_point_constraints(mut doc: Document) -> Document {
    let mut callouts = std::collections::HashMap::new();
    collect_callout_pointers(&doc.statements, &mut callouts);
    doc.statements = expand_point_constraints_in(doc.statements, &callouts);
    doc
}

fn expand_point_constraints_in(
    stmts: Vec<Spanned<Statement>>,
    callouts: &std::collections::HashMap<String, PointerDir>,
) -> Vec<Spanned<Statement>> {
    let mut out = Vec::with_capacity(stmts.len());
    for stmt in stmts {
        let span = stmt.span.clone();
        match stmt.node {
            Statement::Constrain(decl) => {
                let exprs = expand_point_expr(&decl.expr, callouts);
                for expr in exprs {
                    out.push(Spanned::new(
                        Statement::Constrain(ConstrainDecl { expr, name: decl.name.clone() }),
                        span.clone(),
                    ));
                }
            }
            Statement::Layout(mut l) => {
                l.children = expand_point_constraints_in(l.children, callouts);
                out.push(Spanned::new(Statement::Layout(l), span));
            }
            Statement::Group(mut g) => {
                g.children = expand_point_constraints_in(g.children, callouts);
                out.push(Spanned::new(Statement::Group(g), span));
            }
            other => out.push(Spanned::new(other, span)),
        }
    }
    out
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
    /// Radius-based arc: `[radius: 20, sweep: clockwise, large_arc: true]`
    Radius {
        radius: f64,
        sweep: SweepDirection,
        large_arc: bool,
    },
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

// ============================================
// Anchor Types (Feature 009)
// ============================================

/// Reference to an element with optional anchor name (T003)
/// Used in connections: `element.anchor` or just `element`
#[derive(Debug, Clone, PartialEq)]
pub struct AnchorReference {
    /// The element being referenced
    pub element: Spanned<Identifier>,
    /// Optional anchor name (e.g., "top", "left", "input")
    pub anchor: Option<Spanned<String>>,
}

impl AnchorReference {
    /// Create a reference to just an element (anchor auto-detect)
    pub fn element_only(element: Spanned<Identifier>) -> Self {
        Self {
            element,
            anchor: None,
        }
    }

    /// Create a reference to an element with a specific anchor
    pub fn with_anchor(element: Spanned<Identifier>, anchor: Spanned<String>) -> Self {
        Self {
            element,
            anchor: Some(anchor),
        }
    }
}

/// Cardinal direction for anchor direction specification (T004)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardinalDirection {
    Up,
    Down,
    Left,
    Right,
}

/// Anchor direction specification in template declarations (T004)
#[derive(Debug, Clone, PartialEq)]
pub enum AnchorDirectionSpec {
    /// Cardinal direction: up, down, left, right
    Cardinal(CardinalDirection),
    /// Angle in degrees (0=right, 90=down, 180=left, 270=up)
    Angle(f64),
}

/// Position specification for template anchor declarations (T004)
#[derive(Debug, Clone, PartialEq)]
pub enum AnchorPosition {
    /// Reference to an element's property: `body.left`, `header.bottom`
    PropertyRef(PropertyRef),
    /// Property reference with offset: `body.left + 10`
    PropertyRefWithOffset { prop_ref: PropertyRef, offset: f64 },
}

/// Anchor declaration in a template (T004)
/// Syntax: `anchor name [position: element.property, direction: up]`
#[derive(Debug, Clone, PartialEq)]
pub struct AnchorDecl {
    /// Name of the anchor
    pub name: Spanned<Identifier>,
    /// Position specification
    pub position: AnchorPosition,
    /// Direction specification (optional, inferred from position property if not specified)
    pub direction: Option<AnchorDirectionSpec>,
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
    fn test_constraint_property_from_str_builtins() {
        assert_eq!(
            ConstraintProperty::from_str("x"),
            Some(ConstraintProperty::X)
        );
        assert_eq!(
            ConstraintProperty::from_str("y"),
            Some(ConstraintProperty::Y)
        );
        assert_eq!(
            ConstraintProperty::from_str("left"),
            Some(ConstraintProperty::Left)
        );
        assert_eq!(
            ConstraintProperty::from_str("right"),
            Some(ConstraintProperty::Right)
        );
        assert_eq!(
            ConstraintProperty::from_str("top"),
            Some(ConstraintProperty::Top)
        );
        assert_eq!(
            ConstraintProperty::from_str("bottom"),
            Some(ConstraintProperty::Bottom)
        );
        assert_eq!(
            ConstraintProperty::from_str("center_x"),
            Some(ConstraintProperty::CenterX)
        );
        assert_eq!(
            ConstraintProperty::from_str("center_y"),
            Some(ConstraintProperty::CenterY)
        );
        assert_eq!(
            ConstraintProperty::from_str("horizontal_center"),
            Some(ConstraintProperty::CenterX)
        );
        assert_eq!(
            ConstraintProperty::from_str("vertical_center"),
            Some(ConstraintProperty::CenterY)
        );
        assert_eq!(
            ConstraintProperty::from_str("center"),
            Some(ConstraintProperty::Center)
        );
        assert_eq!(
            ConstraintProperty::from_str("width"),
            Some(ConstraintProperty::Width)
        );
        assert_eq!(
            ConstraintProperty::from_str("height"),
            Some(ConstraintProperty::Height)
        );
        // Bare non-builtin names are whole-anchor references (point-constraints).
        assert_eq!(
            ConstraintProperty::from_str("unknown"),
            Some(ConstraintProperty::Anchor("unknown".to_string()))
        );
    }

    #[test]
    fn test_constraint_property_from_str_anchors() {
        // Anchor references use _x/_y suffix
        assert_eq!(
            ConstraintProperty::from_str("drain_x"),
            Some(ConstraintProperty::AnchorX("drain".to_string()))
        );
        assert_eq!(
            ConstraintProperty::from_str("drain_y"),
            Some(ConstraintProperty::AnchorY("drain".to_string()))
        );
        assert_eq!(
            ConstraintProperty::from_str("gate_x"),
            Some(ConstraintProperty::AnchorX("gate".to_string()))
        );
        assert_eq!(
            ConstraintProperty::from_str("gate_y"),
            Some(ConstraintProperty::AnchorY("gate".to_string()))
        );
        assert_eq!(
            ConstraintProperty::from_str("left_conn_x"),
            Some(ConstraintProperty::AnchorX("left_conn".to_string()))
        );
        assert_eq!(
            ConstraintProperty::from_str("left_conn_y"),
            Some(ConstraintProperty::AnchorY("left_conn".to_string()))
        );
    }

    #[test]
    fn test_constraint_property_builtins_not_treated_as_anchors() {
        // center_x and center_y must resolve to builtins, NOT AnchorX("center")/AnchorY("center")
        assert_eq!(
            ConstraintProperty::from_str("center_x"),
            Some(ConstraintProperty::CenterX)
        );
        assert_eq!(
            ConstraintProperty::from_str("center_y"),
            Some(ConstraintProperty::CenterY)
        );
        // "x" and "y" are builtins too — they're not "_x"/"_y" suffixed
        assert_eq!(
            ConstraintProperty::from_str("x"),
            Some(ConstraintProperty::X)
        );
        assert_eq!(
            ConstraintProperty::from_str("y"),
            Some(ConstraintProperty::Y)
        );
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
