//! SVG renderer for generating output from layout results
//!
//! This module takes a LayoutResult and produces an SVG string
//! with appropriate CSS classes for styling.

pub mod config;
pub mod svg;

pub use config::SvgConfig;
pub use svg::render_svg;
