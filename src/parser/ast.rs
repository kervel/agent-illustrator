//! Abstract Syntax Tree types for the Agent Illustrator DSL

/// Byte range in source text
pub type Span = std::ops::Range<usize>;

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
    FontSize,
    Class,
    Custom(String),
}

/// Style values
#[derive(Debug, Clone, PartialEq)]
pub enum StyleValue {
    Color(String),
    Number { value: f64, unit: Option<String> },
    String(String),
    Keyword(String),
}
