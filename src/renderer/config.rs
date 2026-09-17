//! Configuration for SVG rendering

/// Configuration options for SVG output
#[derive(Debug, Clone)]
pub struct SvgConfig {
    /// Padding around the viewBox
    pub viewbox_padding: f64,

    /// Whether to include XML declaration and standalone attributes
    pub standalone: bool,

    /// Whether to format output with indentation
    pub pretty_print: bool,

    /// Prefix for CSS class names (e.g., "ai-" for "ai-shape")
    pub class_prefix: Option<String>,

    /// Short token making this document's keyframe classes unique.
    ///
    /// Per-frame rules are built from names the author chose — `.frame-<kf>`
    /// and `.kfp-<element>` — so two diagrams inlined into one page that both
    /// name a keyframe "idle" and an element "v1" wrote rules matching each
    /// other's elements. That is invisible in isolation: each file is correct,
    /// `--frame` is correct, and a pixel diff of the two rendered separately
    /// says identical. It only appears once both are inlined, which is the
    /// documented way to use animated output.
    ///
    /// Empty leaves class names unscoped, which is what a single standalone
    /// diagram wants.
    pub scope: String,
}

impl Default for SvgConfig {
    fn default() -> Self {
        Self {
            viewbox_padding: 60.0,
            standalone: true,
            pretty_print: true,
            class_prefix: Some("ai-".to_string()),
            scope: String::new(),
        }
    }
}

impl SvgConfig {
    /// Create a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the viewBox padding
    pub fn with_viewbox_padding(mut self, padding: f64) -> Self {
        self.viewbox_padding = padding;
        self
    }

    /// Set whether output is standalone
    pub fn with_standalone(mut self, standalone: bool) -> Self {
        self.standalone = standalone;
        self
    }

    /// Set whether to pretty-print output
    pub fn with_pretty_print(mut self, pretty: bool) -> Self {
        self.pretty_print = pretty;
        self
    }

    /// Set the CSS class prefix
    pub fn with_class_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.class_prefix = Some(prefix.into());
        self
    }

    /// Remove the CSS class prefix
    pub fn without_class_prefix(mut self) -> Self {
        self.class_prefix = None;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SvgConfig::default();
        assert_eq!(config.viewbox_padding, 60.0);
        assert!(config.standalone);
        assert!(config.pretty_print);
        assert_eq!(config.class_prefix, Some("ai-".to_string()));
    }

    #[test]
    fn test_builder_pattern() {
        let config = SvgConfig::new()
            .with_viewbox_padding(10.0)
            .with_standalone(false)
            .with_pretty_print(false)
            .with_class_prefix("my-");

        assert_eq!(config.viewbox_padding, 10.0);
        assert!(!config.standalone);
        assert!(!config.pretty_print);
        assert_eq!(config.class_prefix, Some("my-".to_string()));
    }
}
