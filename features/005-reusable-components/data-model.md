# Data Model: Reusable Components

## AST Types

### ComponentDecl

Declares an external file as a named component.

```rust
pub struct ComponentDecl {
    /// User-defined component name (e.g., "person", "server")
    pub name: Spanned<Identifier>,
    /// Path to source file, relative to importing file
    pub source_path: Spanned<String>,
    /// Detected source type based on file extension
    pub source_type: ComponentSourceType,
}

pub enum ComponentSourceType {
    /// .svg file - will be embedded as SVG content
    Svg,
    /// .ail file - will be parsed and expanded
    Ail,
}
```

**Relationships**:
- Referenced by `ComponentInstance.component_name`
- Source file parsed into `ResolvedComponent`

### ComponentInstance

An instantiation of a declared component.

```rust
pub struct ComponentInstance {
    /// Name of the component being instantiated
    pub component_name: Spanned<Identifier>,
    /// Unique name for this instance
    pub instance_name: Spanned<Identifier>,
    /// Style parameters and overrides
    pub parameters: Vec<Spanned<StyleModifier>>,
}
```

**Relationships**:
- References `ComponentDecl` by `component_name`
- Expands into concrete `Statement` elements

### ExportDecl

Declares which internal elements are accessible from outside.

```rust
pub struct ExportDecl {
    /// List of element names to export
    pub exports: Vec<Spanned<Identifier>>,
}
```

**Relationships**:
- Element names must exist in same document
- Referenced by connection resolution for dot-notation targets

## Import Resolution Types

### ResolvedComponent

A fully loaded and parsed component.

```rust
pub struct ResolvedComponent {
    /// Original source path
    pub source_path: PathBuf,
    /// Type of component
    pub source_type: ComponentSourceType,
    /// Parsed content
    pub content: ComponentContent,
    /// Exported element names (for AIL components)
    pub exports: HashSet<String>,
}

pub enum ComponentContent {
    /// SVG file info
    Svg(SvgInfo),
    /// Parsed AIL document
    Ail(Document),
}
```

### SvgInfo

Metadata and content from an SVG file.

```rust
pub struct SvgInfo {
    /// Raw SVG content (for embedding)
    pub content: String,
    /// Width from width attribute or viewBox
    pub width: Option<f64>,
    /// Height from height attribute or viewBox
    pub height: Option<f64>,
    /// Full viewBox if present: (min-x, min-y, width, height)
    pub view_box: Option<(f64, f64, f64, f64)>,
}
```

**Methods**:
- `aspect_ratio() -> Option<f64>`: Calculate w/h ratio
- `intrinsic_size() -> (f64, f64)`: Get natural size or default

### ImportResolver

Manages component loading with caching and cycle detection.

```rust
pub struct ImportResolver {
    /// Base path for relative imports
    base_path: PathBuf,
    /// Cache of resolved components
    resolved: HashMap<PathBuf, ResolvedComponent>,
    /// Files currently being resolved (cycle detection)
    in_progress: HashSet<PathBuf>,
}
```

**State Transitions**:
1. `resolve(path)` called
2. Check `resolved` cache → return if found
3. Check `in_progress` → error if found (circular)
4. Add to `in_progress`
5. Load file, parse content
6. For AIL: recursively resolve nested imports
7. Remove from `in_progress`
8. Add to `resolved`
9. Return

## Expansion Types

### ExpandedDocument

Document with all component instances replaced by concrete elements.

```rust
pub struct ExpandedDocument {
    /// Statements with components expanded
    pub statements: Vec<Spanned<Statement>>,
    /// Map from original instance paths to expanded element ids
    pub instance_map: HashMap<String, Vec<String>>,
}
```

### SvgEmbed

New shape type for embedded SVG content.

```rust
// Added to ShapeType enum
pub enum ShapeType {
    // ... existing variants
    SvgEmbed {
        /// SVG content to embed (inner content, no <svg> wrapper)
        content: String,
        /// Original width for aspect ratio calculation
        intrinsic_width: Option<f64>,
        /// Original height for aspect ratio calculation
        intrinsic_height: Option<f64>,
    },
}
```

## Layout Types

### ComponentInstanceLayout

Layout information for a component instance (extends ElementLayout).

Uses existing `ElementLayout` with:
- `element_type: ElementType::Shape(ShapeType::SvgEmbed { .. })` for SVG
- `element_type: ElementType::Group` for AIL (children are expanded elements)
- `id: Some(instance_name)` for connection targeting

## Entity Relationships

```
┌─────────────────┐     references     ┌──────────────────┐
│ ComponentInstance│──────────────────►│  ComponentDecl   │
└─────────────────┘                    └──────────────────┘
        │                                      │
        │ expands to                           │ loads
        ▼                                      ▼
┌─────────────────┐                   ┌──────────────────┐
│ Statement[]     │◄──────────────────│ ResolvedComponent│
│ (prefixed ids)  │    content from   └──────────────────┘
└─────────────────┘                            │
        │                                      │
        │ contains                    ┌────────┴────────┐
        ▼                             ▼                 ▼
┌─────────────────┐          ┌─────────────┐    ┌─────────────┐
│  SvgEmbed       │          │   SvgInfo   │    │  Document   │
│  (ShapeType)    │          │ (SVG meta)  │    │ (AIL AST)   │
└─────────────────┘          └─────────────┘    └─────────────┘
```

## Validation Rules

### ComponentDecl Validation
- `name` must be unique within document
- `name` must not conflict with built-in shape types
- `source_path` must exist and be readable
- `source_type` must match file extension

### ComponentInstance Validation
- `component_name` must reference a declared component
- `instance_name` must be unique within document scope
- `parameters` must be valid style modifiers

### ExportDecl Validation
- Each exported name must reference an existing element
- No duplicate exports
- Export declarations must appear at document top-level

### Connection Validation (extended)
- For dot-notation targets (e.g., `rack1.port1`):
  - First segment must be a component instance
  - Subsequent segments must be in component's export list
  - Non-exported internal elements produce clear errors
