//! Stylesheet system for color palette support
//!
//! This module provides symbolic color tokens that can be resolved to concrete
//! color values via stylesheets. This enables brand-agnostic illustrations that
//! can be rendered with different color schemes.

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;
use thiserror::Error;

/// Errors that can occur when loading or parsing stylesheets
#[derive(Error, Debug)]
pub enum StylesheetError {
    #[error("Failed to read stylesheet file: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to parse stylesheet TOML: {0}")]
    ParseError(#[from] toml::de::Error),
}

/// A stylesheet mapping symbolic colors to concrete values
#[derive(Debug, Clone)]
pub struct Stylesheet {
    /// Optional name for the stylesheet
    pub name: Option<String>,
    /// Optional description
    pub description: Option<String>,
    /// Color mappings: token name -> hex color
    pub colors: HashMap<String, String>,
}

/// TOML structure for deserializing stylesheets
#[derive(Deserialize)]
struct TomlStylesheet {
    metadata: Option<TomlMetadata>,
    colors: HashMap<String, String>,
}

#[derive(Deserialize)]
struct TomlMetadata {
    name: Option<String>,
    description: Option<String>,
}

/// Default color palette - neutral grays with blue accent and orange secondary
const DEFAULT_PALETTE: &str = r##"
[colors]
# Foreground colors (primary visual elements)
foreground-1 = "#333333"
foreground-2 = "#666666"
foreground-3 = "#999999"
foreground-light = "#e0e0e0"
foreground-dark = "#1a1a1a"

# Background colors
background-1 = "#ffffff"
background-2 = "#f5f5f5"
background-3 = "#eeeeee"
background-light = "#ffffff"
background-dark = "#333333"

# Text colors
text-1 = "#333333"
text-2 = "#666666"
text-3 = "#999999"
text-light = "#ffffff"
text-dark = "#1a1a1a"

# Accent colors (Material Blue - primary)
accent-1 = "#2196f3"
accent-2 = "#e3f2fd"
accent-3 = "#bbdefb"
accent-light = "#e3f2fd"
accent-dark = "#1565c0"

# Secondary colors (Material Orange - for contrast/agent)
secondary-1 = "#ff9800"
secondary-2 = "#fff3e0"
secondary-3 = "#ffe0b2"
secondary-light = "#fff3e0"
secondary-dark = "#e65100"

# Status colors
status-success = "#4caf50"
status-warning = "#ff9800"
status-error = "#f44336"
"##;

impl Stylesheet {
    /// Load stylesheet from TOML file
    pub fn from_file(path: &Path) -> Result<Self, StylesheetError> {
        let content = std::fs::read_to_string(path)?;
        Self::from_str(&content)
    }

    /// Load stylesheet from TOML string
    pub fn from_str(content: &str) -> Result<Self, StylesheetError> {
        let parsed: TomlStylesheet = toml::from_str(content)?;

        Ok(Stylesheet {
            name: parsed.metadata.as_ref().and_then(|m| m.name.clone()),
            description: parsed.metadata.as_ref().and_then(|m| m.description.clone()),
            colors: parsed.colors,
        })
    }

    /// Resolve a symbolic color token to a concrete value
    ///
    /// Returns None if the token is not defined in this stylesheet.
    pub fn resolve(&self, token: &str) -> Option<&str> {
        self.colors.get(token).map(|s| s.as_str())
    }

    /// Resolve a symbolic color token with fallback to default palette
    ///
    /// Fallback order:
    /// 1. Check this stylesheet for exact token
    /// 2. Check default palette for exact token
    /// 3. Use category default (foreground → #333333, etc.)
    pub fn resolve_or_default(&self, token: &str) -> String {
        // Try this stylesheet first
        if let Some(color) = self.resolve(token) {
            return color.to_string();
        }

        // Fallback to default palette
        let default = Self::default();
        if let Some(color) = default.resolve(token) {
            return color.to_string();
        }

        // Final fallback: category defaults
        if token.starts_with("foreground") {
            return "#333333".to_string();
        }
        if token.starts_with("background") {
            return "#ffffff".to_string();
        }
        if token.starts_with("text") {
            return "#333333".to_string();
        }
        if token.starts_with("accent") {
            return "#2196f3".to_string();
        }
        if token.starts_with("secondary") {
            return "#ff9800".to_string();
        }
        if token.starts_with("status") {
            return "#666666".to_string();
        }

        // Unknown category - return dark gray
        "#333333".to_string()
    }
}

impl Default for Stylesheet {
    fn default() -> Self {
        Self::from_str(DEFAULT_PALETTE).expect("Default palette should be valid TOML")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_stylesheet() {
        let stylesheet = Stylesheet::default();
        assert!(stylesheet.colors.contains_key("foreground-1"));
        assert!(stylesheet.colors.contains_key("background-1"));
        assert!(stylesheet.colors.contains_key("text-1"));
        assert!(stylesheet.colors.contains_key("accent-1"));
    }

    #[test]
    fn test_resolve_existing_token() {
        let stylesheet = Stylesheet::default();
        assert_eq!(stylesheet.resolve("foreground-1"), Some("#333333"));
        assert_eq!(stylesheet.resolve("accent-1"), Some("#2196f3"));
    }

    #[test]
    fn test_resolve_missing_token() {
        let stylesheet = Stylesheet::default();
        assert_eq!(stylesheet.resolve("nonexistent"), None);
    }

    #[test]
    fn test_resolve_or_default_fallback() {
        // Empty stylesheet should fall back to defaults
        let empty = Stylesheet {
            name: None,
            description: None,
            colors: HashMap::new(),
        };
        assert_eq!(empty.resolve_or_default("foreground-1"), "#333333");
    }

    #[test]
    fn test_resolve_or_default_category_fallback() {
        // Even for unknown tokens, category defaults apply
        let empty = Stylesheet {
            name: None,
            description: None,
            colors: HashMap::new(),
        };
        // Unknown specific token but known category
        assert_eq!(empty.resolve_or_default("foreground-99"), "#333333");
        assert_eq!(empty.resolve_or_default("background-custom"), "#ffffff");
    }

    #[test]
    fn test_parse_toml_with_metadata() {
        let toml_str = r##"
[metadata]
name = "Test Theme"
description = "A test theme"

[colors]
foreground-1 = "#000000"
"##;
        let stylesheet = Stylesheet::from_str(toml_str).expect("Should parse");
        assert_eq!(stylesheet.name, Some("Test Theme".to_string()));
        assert_eq!(stylesheet.description, Some("A test theme".to_string()));
        assert_eq!(stylesheet.resolve("foreground-1"), Some("#000000"));
    }

    #[test]
    fn test_parse_toml_without_metadata() {
        let toml_str = r##"
[colors]
foreground-1 = "#111111"
"##;
        let stylesheet = Stylesheet::from_str(toml_str).expect("Should parse");
        assert_eq!(stylesheet.name, None);
        assert_eq!(stylesheet.resolve("foreground-1"), Some("#111111"));
    }

    #[test]
    fn test_invalid_toml_error() {
        let invalid = "this is not valid toml {{{{";
        let result = Stylesheet::from_str(invalid);
        assert!(result.is_err());
    }
}
