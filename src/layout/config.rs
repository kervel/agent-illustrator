//! Configuration for the layout engine

/// Configuration options for layout computation
#[derive(Debug, Clone)]
pub struct LayoutConfig {
    /// Default size for rectangles (width, height)
    pub default_rect_size: (f64, f64),

    /// Default radius for circles
    pub default_circle_radius: f64,

    /// Default width for line shapes
    pub default_line_width: f64,

    /// Default size for ellipses (width, height)
    pub default_ellipse_size: (f64, f64),

    /// Spacing between sibling elements
    pub element_spacing: f64,

    /// Padding inside layout containers
    pub container_padding: f64,

    /// Minimum spacing for connection routes around elements
    pub connection_spacing: f64,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            default_rect_size: (80.0, 30.0),
            default_circle_radius: 25.0,
            default_line_width: 80.0,
            default_ellipse_size: (80.0, 45.0),
            element_spacing: 4.0,
            container_padding: 5.0,
            connection_spacing: 10.0,
        }
    }
}

impl LayoutConfig {
    /// Create a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the default rectangle size
    pub fn with_rect_size(mut self, width: f64, height: f64) -> Self {
        self.default_rect_size = (width, height);
        self
    }

    /// Set the default circle radius
    pub fn with_circle_radius(mut self, radius: f64) -> Self {
        self.default_circle_radius = radius;
        self
    }

    /// Set the spacing between elements
    pub fn with_element_spacing(mut self, spacing: f64) -> Self {
        self.element_spacing = spacing;
        self
    }

    /// Set the container padding
    pub fn with_container_padding(mut self, padding: f64) -> Self {
        self.container_padding = padding;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LayoutConfig::default();
        assert_eq!(config.default_rect_size, (80.0, 30.0));
        assert_eq!(config.default_circle_radius, 25.0);
        assert_eq!(config.default_line_width, 80.0);
        assert_eq!(config.default_ellipse_size, (80.0, 45.0));
        assert_eq!(config.element_spacing, 4.0);
        assert_eq!(config.container_padding, 5.0);
        assert_eq!(config.connection_spacing, 10.0);
    }

    #[test]
    fn test_builder_pattern() {
        let config = LayoutConfig::new()
            .with_rect_size(150.0, 75.0)
            .with_element_spacing(30.0);

        assert_eq!(config.default_rect_size, (150.0, 75.0));
        assert_eq!(config.element_spacing, 30.0);
    }
}
