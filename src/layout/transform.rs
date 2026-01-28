//! Rotation transformation utilities for the two-phase constraint solver.
//!
//! This module handles transforming element bounds and anchors after local
//! constraint solving, applying rotation around the template center.
//!
//! ## Two-Phase Solver Architecture
//!
//! The constraint solver operates in phases:
//! 1. **Local solving**: Constraints within each template instance are solved in isolation
//! 2. **Rotation transformation**: Template instances with rotation have their bounds and
//!    anchors transformed around the template center
//! 3. **Global solving**: Constraints between templates operate on post-rotation bounds
//!
//! ## Loose Bounds Algorithm
//!
//! Rather than computing mathematically tight bounds for rotated shapes (complex for
//! curves and arcs), we use the "loose bounds" algorithm:
//! 1. Get the 4 corners of the original axis-aligned bounding box (AABB)
//! 2. Rotate each corner around the rotation center
//! 3. Compute the AABB of the 4 rotated corner points
//!
//! This approach is simpler, faster, and matches CSS/SVG transform behavior.
//! The over-estimation is acceptable for layout positioning.
//!
//! ## Rotation Convention
//!
//! Rotation uses the SVG convention: clockwise positive angles, in degrees.
//! - 0° = no rotation
//! - 90° = rotated clockwise (right becomes down)
//! - 180° = upside down
//! - 270° = rotated counter-clockwise (right becomes up)

use crate::layout::types::{Anchor, AnchorDirection, BoundingBox, Point};

/// Represents a 2D rotation transformation around a center point.
///
/// Used during the rotation phase of two-phase constraint solving to transform
/// element bounds and anchors from local coordinates to global coordinates.
#[derive(Debug, Clone, Copy)]
pub struct RotationTransform {
    /// Rotation angle in degrees (clockwise positive, per SVG convention)
    pub angle_degrees: f64,
    /// Center point of rotation (typically the geometric center of the template)
    pub center: Point,
}

impl RotationTransform {
    /// Create a new rotation transform.
    ///
    /// # Arguments
    /// * `angle_degrees` - Rotation angle in degrees (clockwise positive)
    /// * `center` - Center point of rotation
    pub fn new(angle_degrees: f64, center: Point) -> Self {
        Self {
            angle_degrees,
            center,
        }
    }

    /// Check if this is effectively a no-op (0° rotation).
    ///
    /// Returns true if the rotation angle is close enough to zero that it
    /// would not produce any visible change.
    pub fn is_identity(&self) -> bool {
        self.angle_degrees.abs() < f64::EPSILON
    }

    /// Rotate a point around the center using standard 2D rotation matrix.
    ///
    /// Uses SVG convention: clockwise positive angles, Y-axis pointing down.
    ///
    /// In SVG's coordinate system (Y pointing down), clockwise rotation
    /// uses the standard rotation matrix:
    /// ```text
    /// x' = cx + (x - cx) * cos(θ) - (y - cy) * sin(θ)
    /// y' = cy + (x - cx) * sin(θ) + (y - cy) * cos(θ)
    /// ```
    ///
    /// # Arguments
    /// * `point` - The point to rotate
    ///
    /// # Returns
    /// The rotated point
    pub fn transform_point(&self, point: Point) -> Point {
        if self.is_identity() {
            return point;
        }

        let radians = self.angle_degrees.to_radians();
        let cos_a = radians.cos();
        let sin_a = radians.sin();

        let dx = point.x - self.center.x;
        let dy = point.y - self.center.y;

        // In SVG's coordinate system (Y-down), clockwise rotation uses:
        // [cos  -sin] [dx]
        // [sin   cos] [dy]
        Point {
            x: self.center.x + dx * cos_a - dy * sin_a,
            y: self.center.y + dx * sin_a + dy * cos_a,
        }
    }

    /// Transform a bounding box using the "loose bounds" algorithm.
    ///
    /// Rather than computing mathematically tight bounds (complex for curves),
    /// we rotate the 4 corners of the original AABB and take the AABB of
    /// those rotated corners. This matches CSS/SVG transform behavior.
    ///
    /// # Arguments
    /// * `bounds` - The original axis-aligned bounding box
    ///
    /// # Returns
    /// The post-rotation axis-aligned bounding box (loose bounds)
    pub fn transform_bounds(&self, bounds: &BoundingBox) -> BoundingBox {
        if self.is_identity() {
            return *bounds;
        }

        // Get four corners of the original AABB
        let corners = [
            Point {
                x: bounds.x,
                y: bounds.y,
            },
            Point {
                x: bounds.x + bounds.width,
                y: bounds.y,
            },
            Point {
                x: bounds.x,
                y: bounds.y + bounds.height,
            },
            Point {
                x: bounds.x + bounds.width,
                y: bounds.y + bounds.height,
            },
        ];

        // Rotate all corners
        let rotated: Vec<Point> = corners.iter().map(|p| self.transform_point(*p)).collect();

        // Find AABB of rotated corners
        let min_x = rotated
            .iter()
            .map(|p| p.x)
            .fold(f64::INFINITY, f64::min);
        let max_x = rotated
            .iter()
            .map(|p| p.x)
            .fold(f64::NEG_INFINITY, f64::max);
        let min_y = rotated
            .iter()
            .map(|p| p.y)
            .fold(f64::INFINITY, f64::min);
        let max_y = rotated
            .iter()
            .map(|p| p.y)
            .fold(f64::NEG_INFINITY, f64::max);

        BoundingBox {
            x: min_x,
            y: min_y,
            width: max_x - min_x,
            height: max_y - min_y,
        }
    }

    /// Transform an anchor's position and direction.
    ///
    /// The position is rotated around the center, and the direction angle
    /// is adjusted to maintain the correct outward normal after rotation.
    ///
    /// # Arguments
    /// * `anchor` - The anchor to transform
    ///
    /// # Returns
    /// The transformed anchor with updated position and direction
    pub fn transform_anchor(&self, anchor: &Anchor) -> Anchor {
        if self.is_identity() {
            return anchor.clone();
        }

        Anchor {
            name: anchor.name.clone(),
            position: self.transform_point(anchor.position),
            direction: self.transform_direction(anchor.direction),
        }
    }

    /// Transform an anchor direction by adding the rotation angle.
    ///
    /// # Arguments
    /// * `dir` - The original anchor direction
    ///
    /// # Returns
    /// The rotated direction
    pub fn transform_direction(&self, dir: AnchorDirection) -> AnchorDirection {
        let original_angle = dir.to_degrees();
        // Add angle for clockwise rotation in SVG coordinates
        // When shape rotates 90° CW, direction vectors rotate 90° CW too:
        // Right (0°) -> Down (90°), Down (90°) -> Left (180°), etc.
        let new_angle = original_angle + self.angle_degrees;
        AnchorDirection::from_degrees(new_angle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::types::AnchorSet;

    const EPSILON: f64 = 0.001;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < EPSILON
    }

    #[test]
    fn test_identity_rotation() {
        let t = RotationTransform::new(0.0, Point { x: 50.0, y: 50.0 });
        assert!(t.is_identity());

        let p = Point { x: 100.0, y: 0.0 };
        let result = t.transform_point(p);
        assert!(approx_eq(result.x, p.x));
        assert!(approx_eq(result.y, p.y));
    }

    #[test]
    fn test_90_degree_rotation_around_origin() {
        let t = RotationTransform::new(90.0, Point { x: 0.0, y: 0.0 });

        // Point (1, 0) rotated 90° clockwise around origin should be (0, 1)
        let p = Point { x: 1.0, y: 0.0 };
        let result = t.transform_point(p);
        assert!(approx_eq(result.x, 0.0), "x: expected 0.0, got {}", result.x);
        assert!(approx_eq(result.y, 1.0), "y: expected 1.0, got {}", result.y);
    }

    #[test]
    fn test_180_degree_rotation() {
        let t = RotationTransform::new(180.0, Point { x: 0.0, y: 0.0 });

        // Point (1, 0) rotated 180° should be (-1, 0)
        let p = Point { x: 1.0, y: 0.0 };
        let result = t.transform_point(p);
        assert!(
            approx_eq(result.x, -1.0),
            "x: expected -1.0, got {}",
            result.x
        );
        assert!(approx_eq(result.y, 0.0), "y: expected 0.0, got {}", result.y);
    }

    #[test]
    fn test_270_degree_rotation() {
        let t = RotationTransform::new(270.0, Point { x: 0.0, y: 0.0 });

        // Point (1, 0) rotated 270° clockwise (= 90° counter-clockwise) should be (0, -1)
        let p = Point { x: 1.0, y: 0.0 };
        let result = t.transform_point(p);
        assert!(approx_eq(result.x, 0.0), "x: expected 0.0, got {}", result.x);
        assert!(
            approx_eq(result.y, -1.0),
            "y: expected -1.0, got {}",
            result.y
        );
    }

    #[test]
    fn test_45_degree_rotation() {
        let t = RotationTransform::new(45.0, Point { x: 0.0, y: 0.0 });

        // Point (1, 0) rotated 45° should be (√2/2, √2/2)
        let p = Point { x: 1.0, y: 0.0 };
        let result = t.transform_point(p);
        let expected = std::f64::consts::FRAC_1_SQRT_2;
        assert!(
            approx_eq(result.x, expected),
            "x: expected {}, got {}",
            expected,
            result.x
        );
        assert!(
            approx_eq(result.y, expected),
            "y: expected {}, got {}",
            expected,
            result.y
        );
    }

    #[test]
    fn test_rotation_around_non_origin_center() {
        // Rotate around center (50, 50)
        let t = RotationTransform::new(90.0, Point { x: 50.0, y: 50.0 });

        // Point (100, 50) is 50 units to the right of center
        // After 90° clockwise rotation, it should be 50 units below center: (50, 100)
        let p = Point { x: 100.0, y: 50.0 };
        let result = t.transform_point(p);
        assert!(
            approx_eq(result.x, 50.0),
            "x: expected 50.0, got {}",
            result.x
        );
        assert!(
            approx_eq(result.y, 100.0),
            "y: expected 100.0, got {}",
            result.y
        );
    }

    #[test]
    fn test_loose_bounds_identity() {
        let t = RotationTransform::new(0.0, Point { x: 50.0, y: 25.0 });
        let bounds = BoundingBox {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 50.0,
        };
        let result = t.transform_bounds(&bounds);
        assert!(approx_eq(result.x, bounds.x));
        assert!(approx_eq(result.y, bounds.y));
        assert!(approx_eq(result.width, bounds.width));
        assert!(approx_eq(result.height, bounds.height));
    }

    #[test]
    fn test_loose_bounds_90_degrees() {
        // 100x50 box centered at (50, 25), rotated 90° should become 50x100
        let t = RotationTransform::new(90.0, Point { x: 50.0, y: 25.0 });
        let bounds = BoundingBox {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 50.0,
        };
        let result = t.transform_bounds(&bounds);

        // After 90° rotation, width and height should swap
        assert!(
            approx_eq(result.width, 50.0),
            "width: expected 50.0, got {}",
            result.width
        );
        assert!(
            approx_eq(result.height, 100.0),
            "height: expected 100.0, got {}",
            result.height
        );
    }

    #[test]
    fn test_loose_bounds_45_degrees() {
        // A 100x100 square rotated 45° should expand to ~141x141 (diagonal becomes side)
        let t = RotationTransform::new(45.0, Point { x: 50.0, y: 50.0 });
        let bounds = BoundingBox {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        let result = t.transform_bounds(&bounds);

        // Diagonal of 100x100 square is 100*√2 ≈ 141.42
        let expected_size = 100.0 * std::f64::consts::SQRT_2;
        assert!(
            (result.width - expected_size).abs() < 0.1,
            "width: expected ~{}, got {}",
            expected_size,
            result.width
        );
        assert!(
            (result.height - expected_size).abs() < 0.1,
            "height: expected ~{}, got {}",
            expected_size,
            result.height
        );
    }

    #[test]
    fn test_direction_from_degrees_cardinals() {
        assert!(matches!(
            AnchorDirection::from_degrees(0.0),
            AnchorDirection::Right
        ));
        assert!(matches!(
            AnchorDirection::from_degrees(90.0),
            AnchorDirection::Down
        ));
        assert!(matches!(
            AnchorDirection::from_degrees(180.0),
            AnchorDirection::Left
        ));
        assert!(matches!(
            AnchorDirection::from_degrees(270.0),
            AnchorDirection::Up
        ));
        assert!(matches!(
            AnchorDirection::from_degrees(360.0),
            AnchorDirection::Right
        ));
    }

    #[test]
    fn test_direction_from_degrees_non_cardinal() {
        match AnchorDirection::from_degrees(45.0) {
            AnchorDirection::Angle(deg) => assert!(approx_eq(deg, 45.0)),
            _ => panic!("Expected Angle variant"),
        }

        match AnchorDirection::from_degrees(135.0) {
            AnchorDirection::Angle(deg) => assert!(approx_eq(deg, 135.0)),
            _ => panic!("Expected Angle variant"),
        }
    }

    #[test]
    fn test_direction_from_degrees_normalization() {
        // Negative angles should normalize to 0-360
        match AnchorDirection::from_degrees(-90.0) {
            AnchorDirection::Up => {} // 270°
            other => panic!("Expected Up, got {:?}", other),
        }

        // Angles > 360 should normalize
        assert!(matches!(
            AnchorDirection::from_degrees(450.0),
            AnchorDirection::Down
        )); // 90°
    }

    #[test]
    fn test_anchor_transformation_90_degrees() {
        let t = RotationTransform::new(90.0, Point { x: 50.0, y: 25.0 });

        // Anchor at right edge pointing right
        let anchor = Anchor::new("right", Point { x: 100.0, y: 25.0 }, AnchorDirection::Right);
        let result = t.transform_anchor(&anchor);

        // After 90° clockwise:
        // - Position (100, 25) -> rotated around (50, 25) -> (50, 75) [50 right becomes 50 down]
        // - Direction Right (0°) -> Down (90°)
        assert!(
            approx_eq(result.position.x, 50.0),
            "x: expected 50.0, got {}",
            result.position.x
        );
        assert!(
            approx_eq(result.position.y, 75.0),
            "y: expected 75.0, got {}",
            result.position.y
        );
        assert!(
            matches!(result.direction, AnchorDirection::Down),
            "direction: expected Down, got {:?}",
            result.direction
        );
    }

    #[test]
    fn test_anchor_transformation_left_becomes_up() {
        let t = RotationTransform::new(90.0, Point { x: 0.0, y: 0.0 });

        // Anchor pointing left (180°)
        let anchor = Anchor::new("left", Point { x: 0.0, y: 0.0 }, AnchorDirection::Left);
        let result = t.transform_anchor(&anchor);

        // After 90° clockwise: Left (180°) + 90° = 270° = Up
        assert!(
            matches!(result.direction, AnchorDirection::Up),
            "direction: expected Up, got {:?}",
            result.direction
        );
    }

    #[test]
    fn test_anchor_set_transform() {
        let t = RotationTransform::new(90.0, Point { x: 50.0, y: 50.0 });

        let bounds = BoundingBox::new(0.0, 0.0, 100.0, 100.0);
        let anchors = AnchorSet::simple_shape(&bounds);

        let transformed = anchors.transform(&t);

        // After 90° rotation around center (50, 50):
        // - "right" anchor at (100, 50) -> (50, 100), direction Down
        // - "top" anchor at (50, 0) -> (100, 50), direction Right
        // - "left" anchor at (0, 50) -> (50, 0), direction Up
        // - "bottom" anchor at (50, 100) -> (0, 50), direction Left

        let right = transformed.get("right").unwrap();
        assert!(
            approx_eq(right.position.x, 50.0),
            "right x: expected 50.0, got {}",
            right.position.x
        );
        assert!(
            approx_eq(right.position.y, 100.0),
            "right y: expected 100.0, got {}",
            right.position.y
        );
        assert!(
            matches!(right.direction, AnchorDirection::Down),
            "right direction: expected Down, got {:?}",
            right.direction
        );

        let top = transformed.get("top").unwrap();
        assert!(
            approx_eq(top.position.x, 100.0),
            "top x: expected 100.0, got {}",
            top.position.x
        );
        assert!(
            approx_eq(top.position.y, 50.0),
            "top y: expected 50.0, got {}",
            top.position.y
        );
        assert!(
            matches!(top.direction, AnchorDirection::Right),
            "top direction: expected Right, got {:?}",
            top.direction
        );
    }
}
