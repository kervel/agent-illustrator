//! SVG renderer for generating output from layout results
//!
//! This module takes a LayoutResult and produces an SVG string
//! with appropriate CSS classes for styling.

pub mod config;
pub mod path;
pub mod svg;

pub use config::SvgConfig;
pub use path::{resolve_path, ResolvedPath};
pub use svg::{render_svg, render_svg_with_stylesheet};
