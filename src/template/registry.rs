//! Template registry for storing and retrieving template definitions

use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

use crate::parser::ast::{
    AnchorDecl, ExportDecl, ParameterDef, Spanned, Statement, StyleValue, TemplateDecl,
    TemplateSourceType,
};
use crate::ImageHrefMode;

/// Errors that can occur during template operations
#[derive(Debug, Error)]
pub enum TemplateError {
    /// Template not found in registry
    #[error("template not found: {name}")]
    NotFound { name: String },

    /// Duplicate template definition
    #[error("duplicate template definition: {name}")]
    Duplicate { name: String },

    /// Missing required parameter
    #[error("missing required parameter: {param} for template {template}")]
    MissingParameter { template: String, param: String },

    /// Invalid parameter type
    #[error("invalid parameter type for {param}: expected {expected}")]
    InvalidParameterType { param: String, expected: String },

    /// File not found for file-based template
    #[error("template file not found: {path}")]
    FileNotFound { path: PathBuf },

    /// Error reading template file
    #[error("error reading template file {path}: {message}")]
    FileReadError { path: PathBuf, message: String },

    /// Invalid SVG content
    #[error("invalid SVG content: {message}")]
    InvalidSvg { message: String },

    /// Circular template reference
    #[error("circular template reference detected: {chain}")]
    CircularReference { chain: String },

    /// Export not found in template
    #[error("exported identifier not found in template {template}: {export}")]
    ExportNotFound { template: String, export: String },
}

/// A stored template definition
#[derive(Debug, Clone)]
pub struct TemplateDefinition {
    /// Template name
    pub name: String,
    /// Source type (inline, SVG, AIL)
    pub source_type: TemplateSourceType,
    /// Path to source file (for file-based templates)
    pub source_path: Option<PathBuf>,
    /// Parameter definitions with defaults
    pub parameters: Vec<ParameterDef>,
    /// Template body (for inline templates)
    pub body: Option<Vec<Spanned<Statement>>>,
    /// Raw SVG content (for SVG templates, loaded lazily)
    pub svg_content: Option<String>,
    /// SVG viewBox dimensions (width, height)
    pub svg_dimensions: Option<(f64, f64)>,
    /// Exported identifiers for connection points
    pub exports: Vec<String>,
    /// Anchor declarations for custom connection points (Feature 009)
    pub anchors: Vec<AnchorDecl>,
}

impl TemplateDefinition {
    /// Create a new template definition from a TemplateDecl
    pub fn from_decl(decl: &TemplateDecl) -> Self {
        let mut exports = Vec::new();
        let mut anchors = Vec::new();

        // Extract exports and anchors from body if present
        if let Some(body) = &decl.body {
            for stmt in body {
                match &stmt.node {
                    Statement::Export(ExportDecl { exports: exp }) => {
                        for id in exp {
                            exports.push(id.node.0.clone());
                        }
                    }
                    Statement::AnchorDecl(anchor) => {
                        anchors.push(anchor.clone());
                    }
                    _ => {}
                }
            }
        }

        Self {
            name: decl.name.node.0.clone(),
            source_type: decl.source_type.clone(),
            source_path: decl.source_path.as_ref().map(|p| PathBuf::from(&p.node)),
            parameters: decl.parameters.clone(),
            body: decl.body.clone(),
            svg_content: None,
            svg_dimensions: None,
            exports,
            anchors,
        }
    }

    /// Get the default value for a parameter
    pub fn get_default(&self, param_name: &str) -> Option<&StyleValue> {
        self.parameters
            .iter()
            .find(|p| p.name.node.as_str() == param_name)
            .map(|p| &p.default_value.node)
    }

    /// Check if this template has a parameter
    pub fn has_parameter(&self, name: &str) -> bool {
        self.parameters.iter().any(|p| p.name.node.as_str() == name)
    }

    /// Get all parameter names
    pub fn parameter_names(&self) -> Vec<&str> {
        self.parameters
            .iter()
            .map(|p| p.name.node.as_str())
            .collect()
    }

    /// Check if this is a file-based template
    pub fn is_file_based(&self) -> bool {
        matches!(
            self.source_type,
            TemplateSourceType::Svg | TemplateSourceType::Ail | TemplateSourceType::Raster
        )
    }
}

/// Registry for storing template definitions
#[derive(Debug, Default)]
pub struct TemplateRegistry {
    templates: HashMap<String, TemplateDefinition>,
    /// Base path for resolving relative file paths
    base_path: Option<PathBuf>,
    /// How image href paths are emitted in SVG output
    image_href_mode: ImageHrefMode,
}

impl TemplateRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new registry with a base path for file resolution
    pub fn with_base_path(base_path: PathBuf) -> Self {
        Self {
            templates: HashMap::new(),
            base_path: Some(base_path),
            image_href_mode: ImageHrefMode::default(),
        }
    }

    /// Register a template from a declaration
    pub fn register(&mut self, decl: &TemplateDecl) -> Result<(), TemplateError> {
        let name = decl.name.node.0.clone();
        if self.templates.contains_key(&name) {
            return Err(TemplateError::Duplicate { name });
        }

        let def = TemplateDefinition::from_decl(decl);
        self.templates.insert(name, def);
        Ok(())
    }

    /// Register a template definition directly
    pub fn register_definition(&mut self, def: TemplateDefinition) -> Result<(), TemplateError> {
        if self.templates.contains_key(&def.name) {
            return Err(TemplateError::Duplicate {
                name: def.name.clone(),
            });
        }
        self.templates.insert(def.name.clone(), def);
        Ok(())
    }

    /// Get a template by name
    pub fn get(&self, name: &str) -> Option<&TemplateDefinition> {
        self.templates.get(name)
    }

    /// Get a mutable reference to a template
    pub fn get_mut(&mut self, name: &str) -> Option<&mut TemplateDefinition> {
        self.templates.get_mut(name)
    }

    /// Check if a template exists
    pub fn contains(&self, name: &str) -> bool {
        self.templates.contains_key(name)
    }

    /// Get all template names
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.templates.keys().map(|s| s.as_str())
    }

    /// Get the base path for file resolution
    pub fn base_path(&self) -> Option<&PathBuf> {
        self.base_path.as_ref()
    }

    /// Set the base path for file resolution
    pub fn set_base_path(&mut self, path: PathBuf) {
        self.base_path = Some(path);
    }

    /// Set the image href mode
    pub fn set_image_href_mode(&mut self, mode: ImageHrefMode) {
        self.image_href_mode = mode;
    }

    /// Get the image href mode
    pub fn image_href_mode(&self) -> ImageHrefMode {
        self.image_href_mode
    }

    /// Resolve a relative path to an absolute path
    pub fn resolve_path(&self, relative: &str) -> PathBuf {
        if let Some(base) = &self.base_path {
            base.join(relative)
        } else {
            PathBuf::from(relative)
        }
    }

    /// Resolve an image href path according to the configured ImageHrefMode
    pub fn resolve_image_href(&self, relative: &str) -> String {
        match self.image_href_mode {
            ImageHrefMode::Verbatim => relative.to_string(),
            ImageHrefMode::Rewrite => {
                let full = self.resolve_path(relative);
                normalize_path(&full).to_string_lossy().to_string()
            }
            ImageHrefMode::Absolute => {
                let full = self.resolve_path(relative);
                match full.canonicalize() {
                    Ok(abs) => abs.to_string_lossy().to_string(),
                    // Fall back to normalized path if file doesn't exist yet
                    Err(_) => normalize_path(&full).to_string_lossy().to_string(),
                }
            }
        }
    }

    /// Load SVG content for a file-based template
    pub fn load_svg_template(&mut self, name: &str) -> Result<(), TemplateError> {
        let def = self
            .templates
            .get(name)
            .ok_or_else(|| TemplateError::NotFound {
                name: name.to_string(),
            })?;

        if def.source_type != TemplateSourceType::Svg {
            return Ok(()); // Not an SVG template
        }

        if def.svg_content.is_some() {
            return Ok(()); // Already loaded
        }

        let path = def
            .source_path
            .as_ref()
            .ok_or_else(|| TemplateError::FileNotFound {
                path: PathBuf::from(name),
            })?;

        let full_path = self.resolve_path(path.to_str().unwrap_or(""));

        let content =
            std::fs::read_to_string(&full_path).map_err(|e| TemplateError::FileReadError {
                path: full_path.clone(),
                message: e.to_string(),
            })?;

        // Parse SVG dimensions from viewBox or width/height attributes
        let dimensions = parse_svg_dimensions(&content);

        // Update the template with loaded content
        let def = self.templates.get_mut(name).unwrap();
        def.svg_content = Some(content);
        def.svg_dimensions = dimensions;

        Ok(())
    }

    /// Collect all template declarations from a document
    pub fn collect_from_statements(
        &mut self,
        statements: &[Spanned<Statement>],
    ) -> Result<(), TemplateError> {
        for stmt in statements {
            if let Statement::TemplateDecl(decl) = &stmt.node {
                self.register(decl)?;
            }
        }
        Ok(())
    }
}

/// Normalize a path by resolving `.` and `..` components without touching the filesystem.
fn normalize_path(path: &std::path::Path) -> PathBuf {
    use std::path::Component;
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                // Only pop if there's a normal component to pop
                if matches!(components.last(), Some(Component::Normal(_))) {
                    components.pop();
                } else {
                    components.push(component);
                }
            }
            Component::CurDir => {} // skip
            _ => components.push(component),
        }
    }
    components.iter().collect()
}

/// Parse SVG dimensions from content
fn parse_svg_dimensions(svg: &str) -> Option<(f64, f64)> {
    // Try to parse viewBox first
    if let Some(vb_start) = svg.find("viewBox=\"") {
        let vb_start = vb_start + 9;
        if let Some(vb_end) = svg[vb_start..].find('"') {
            let vb_str = &svg[vb_start..vb_start + vb_end];
            let parts: Vec<f64> = vb_str
                .split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();
            if parts.len() >= 4 {
                return Some((parts[2], parts[3]));
            }
        }
    }

    // Fall back to width/height attributes
    let width = parse_svg_attribute(svg, "width");
    let height = parse_svg_attribute(svg, "height");

    match (width, height) {
        (Some(w), Some(h)) => Some((w, h)),
        _ => None,
    }
}

/// Parse a numeric attribute from SVG
fn parse_svg_attribute(svg: &str, attr: &str) -> Option<f64> {
    let pattern = format!("{}=\"", attr);
    if let Some(start) = svg.find(&pattern) {
        let start = start + pattern.len();
        if let Some(end) = svg[start..].find('"') {
            let value_str = &svg[start..start + end];
            // Strip unit suffixes like px, pt, etc.
            let numeric: String = value_str
                .chars()
                .take_while(|c| c.is_numeric() || *c == '.')
                .collect();
            return numeric.parse().ok();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::{Identifier, Span, TemplateSourceType};

    fn make_span() -> Span {
        0..1
    }

    fn make_spanned<T>(node: T) -> Spanned<T> {
        Spanned::new(node, make_span())
    }

    #[test]
    fn test_registry_register_and_get() {
        let mut registry = TemplateRegistry::new();

        let decl = TemplateDecl {
            name: make_spanned(Identifier::new("box")),
            source_type: TemplateSourceType::Inline,
            source_path: None,
            parameters: vec![],
            body: Some(vec![]),
        };

        registry.register(&decl).expect("Should register");
        assert!(registry.contains("box"));
        assert!(registry.get("box").is_some());
    }

    #[test]
    fn test_registry_duplicate_error() {
        let mut registry = TemplateRegistry::new();

        let decl = TemplateDecl {
            name: make_spanned(Identifier::new("box")),
            source_type: TemplateSourceType::Inline,
            source_path: None,
            parameters: vec![],
            body: Some(vec![]),
        };

        registry
            .register(&decl)
            .expect("First register should succeed");
        let result = registry.register(&decl);
        assert!(matches!(result, Err(TemplateError::Duplicate { .. })));
    }

    #[test]
    fn test_parse_svg_dimensions_viewbox() {
        let svg = r#"<svg viewBox="0 0 100 50" xmlns="http://www.w3.org/2000/svg"></svg>"#;
        let dims = parse_svg_dimensions(svg);
        assert_eq!(dims, Some((100.0, 50.0)));
    }

    #[test]
    fn test_parse_svg_dimensions_width_height() {
        let svg = r#"<svg width="200" height="100" xmlns="http://www.w3.org/2000/svg"></svg>"#;
        let dims = parse_svg_dimensions(svg);
        assert_eq!(dims, Some((200.0, 100.0)));
    }

    #[test]
    fn test_parse_svg_dimensions_with_units() {
        let svg = r#"<svg width="200px" height="100px" xmlns="http://www.w3.org/2000/svg"></svg>"#;
        let dims = parse_svg_dimensions(svg);
        assert_eq!(dims, Some((200.0, 100.0)));
    }
}
