//! Visual Primitives - Shape and Color pattern matching for Prima integration.
//!
//! Maps visual concepts to T1 Lex Primitiva for cross-domain transfer.
//!
//! ## Tier Classification
//!
//! | Visual Concept | T1 Primitives | Grounding |
//! |----------------|---------------|-----------|
//! | Point | λ + λ | (x_location, y_location) |
//! | Color | N + N + N + N | (r, g, b, a) quantities |
//! | Circle | λ + N | center_location + radius_quantity |
//! | Triangle | σ[λ,λ,λ] | sequence_of_3_locations |
//! | Line | λ → λ | start → end causality |
//! | Rect | σ + N + N | 4_vertices + width + height |

use crate::paint::Point;
use crate::style::Color;

/// A visual shape classification.
///
/// Tier: T2-P (cross-domain primitive)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeKind {
    /// Point: λ + λ
    Point,
    /// Line: λ → λ
    Line,
    /// Triangle: σ[λ,λ,λ]
    Triangle,
    /// Rectangle: σ + N + N
    Rectangle,
    /// Circle: λ + N
    Circle,
    /// Polygon: σ[λ, ...]
    Polygon,
}

impl ShapeKind {
    /// Get the T1 primitive composition for this shape.
    #[must_use]
    pub const fn primitives(&self) -> &'static str {
        match self {
            Self::Point => "λ + λ",
            Self::Line => "λ → λ",
            Self::Triangle => "σ[λ,λ,λ]",
            Self::Rectangle => "σ + N + N",
            Self::Circle => "λ + N",
            Self::Polygon => "σ[λ...]",
        }
    }

    /// Get the primitive count for this shape.
    #[must_use]
    pub const fn primitive_count(&self) -> usize {
        match self {
            Self::Point => 2,
            Self::Line => 2,
            Self::Triangle => 3,
            Self::Rectangle => 3,
            Self::Circle => 2,
            Self::Polygon => 2, // minimum for sequence + location
        }
    }

    /// Get the transfer confidence for this shape.
    ///
    /// Based on primitive tier: T1=1.0, T2-P=0.9, T2-C=0.7
    #[must_use]
    pub const fn transfer_confidence(&self) -> f32 {
        match self {
            Self::Point | Self::Circle | Self::Line => 0.95, // T2-P
            Self::Triangle | Self::Rectangle => 0.90,        // T2-C simple
            Self::Polygon => 0.80,                           // T2-C composite
        }
    }
}

/// A color decomposed to T1 primitive quantities.
///
/// Grounding: N(r) + N(g) + N(b) + N(a)
#[derive(Debug, Clone, Copy)]
pub struct ColorPrimitive {
    /// Red quantity [0.0, 1.0]
    pub r: f32,
    /// Green quantity [0.0, 1.0]
    pub g: f32,
    /// Blue quantity [0.0, 1.0]
    pub b: f32,
    /// Alpha quantity [0.0, 1.0]
    pub a: f32,
}

impl ColorPrimitive {
    /// Create from a Color.
    #[must_use]
    pub fn from_color(c: Color) -> Self {
        Self {
            r: f32::from(c.r) / 255.0,
            g: f32::from(c.g) / 255.0,
            b: f32::from(c.b) / 255.0,
            a: f32::from(c.a) / 255.0,
        }
    }

    /// Get the T1 primitive composition.
    #[must_use]
    pub const fn primitives() -> &'static str {
        "N(r) + N(g) + N(b) + N(a)"
    }

    /// Compute the luminance (weighted average for human perception).
    #[must_use]
    pub fn luminance(&self) -> f32 {
        0.299 * self.r + 0.587 * self.g + 0.114 * self.b
    }

    /// Check if color is "dark" (luminance < 0.5).
    #[must_use]
    pub fn is_dark(&self) -> bool {
        self.luminance() < 0.5
    }

    /// Compute distance to another color (Euclidean in RGB space).
    #[must_use]
    pub fn distance(&self, other: &Self) -> f32 {
        let dr = self.r - other.r;
        let dg = self.g - other.g;
        let db = self.b - other.b;
        (dr * dr + dg * dg + db * db).sqrt()
    }

    /// Match to named color primitive.
    #[must_use]
    pub fn named_match(&self) -> &'static str {
        let colors = [
            (
                "red",
                Self {
                    r: 1.0,
                    g: 0.0,
                    b: 0.0,
                    a: 1.0,
                },
            ),
            (
                "green",
                Self {
                    r: 0.0,
                    g: 0.5,
                    b: 0.0,
                    a: 1.0,
                },
            ),
            (
                "blue",
                Self {
                    r: 0.0,
                    g: 0.0,
                    b: 1.0,
                    a: 1.0,
                },
            ),
            (
                "yellow",
                Self {
                    r: 1.0,
                    g: 1.0,
                    b: 0.0,
                    a: 1.0,
                },
            ),
            (
                "cyan",
                Self {
                    r: 0.0,
                    g: 1.0,
                    b: 1.0,
                    a: 1.0,
                },
            ),
            (
                "magenta",
                Self {
                    r: 1.0,
                    g: 0.0,
                    b: 1.0,
                    a: 1.0,
                },
            ),
            (
                "white",
                Self {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: 1.0,
                },
            ),
            (
                "black",
                Self {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 1.0,
                },
            ),
        ];

        let mut best_match = "unknown";
        let mut best_dist = f32::MAX;

        for (name, reference) in &colors {
            let dist = self.distance(reference);
            if dist < best_dist {
                best_dist = dist;
                best_match = name;
            }
        }

        best_match
    }
}

/// A shape decomposed to T1 primitives.
///
/// Generic container for any visual shape.
#[derive(Debug, Clone)]
pub struct ShapePrimitive {
    /// The kind of shape.
    pub kind: ShapeKind,
    /// Center or primary location (λ).
    pub center: Point,
    /// Vertices for polygonal shapes (σ[λ...]).
    pub vertices: Vec<Point>,
    /// Radius for circles (N).
    pub radius: f32,
    /// Fill color primitive.
    pub fill: ColorPrimitive,
    /// Stroke color primitive (if any).
    pub stroke: Option<ColorPrimitive>,
    /// Stroke width (N).
    pub stroke_width: f32,
}

impl ShapePrimitive {
    /// Create a circle primitive.
    #[must_use]
    pub fn circle(center: Point, radius: f32, fill: Color) -> Self {
        Self {
            kind: ShapeKind::Circle,
            center,
            vertices: vec![],
            radius,
            fill: ColorPrimitive::from_color(fill),
            stroke: None,
            stroke_width: 0.0,
        }
    }

    /// Create a triangle primitive.
    #[must_use]
    pub fn triangle(p1: Point, p2: Point, p3: Point, fill: Color) -> Self {
        let center = Point::new((p1.x + p2.x + p3.x) / 3.0, (p1.y + p2.y + p3.y) / 3.0);
        Self {
            kind: ShapeKind::Triangle,
            center,
            vertices: vec![p1, p2, p3],
            radius: 0.0,
            fill: ColorPrimitive::from_color(fill),
            stroke: None,
            stroke_width: 0.0,
        }
    }

    /// Create a line primitive.
    #[must_use]
    pub fn line(start: Point, end: Point, width: f32, color: Color) -> Self {
        let center = Point::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);
        Self {
            kind: ShapeKind::Line,
            center,
            vertices: vec![start, end],
            radius: 0.0,
            fill: ColorPrimitive::from_color(color),
            stroke: None,
            stroke_width: width,
        }
    }

    /// Get the T1 primitive representation.
    #[must_use]
    pub fn to_primitive_string(&self) -> String {
        format!(
            "{}[center=({:.1},{:.1}), fill={}, confidence={:.2}]",
            self.kind.primitives(),
            self.center.x,
            self.center.y,
            self.fill.named_match(),
            self.kind.transfer_confidence(),
        )
    }

    /// Get transfer confidence for this shape.
    #[must_use]
    pub fn transfer_confidence(&self) -> f32 {
        self.kind.transfer_confidence()
    }
}

/// Extract visual primitives from a display list.
///
/// This is the bridge between rendering and Prima pattern matching.
#[must_use]
pub fn extract_primitives(commands: &[crate::paint::DisplayCommand]) -> Vec<ShapePrimitive> {
    use crate::paint::DisplayCommand;

    commands
        .iter()
        .filter_map(|cmd| {
            match cmd {
                DisplayCommand::FillCircle {
                    center,
                    radius,
                    color,
                    ..
                } => Some(ShapePrimitive::circle(*center, *radius, *color)),
                DisplayCommand::FillTriangle {
                    p1, p2, p3, color, ..
                } => Some(ShapePrimitive::triangle(*p1, *p2, *p3, *color)),
                DisplayCommand::StrokeLine {
                    start,
                    end,
                    width,
                    color,
                    ..
                } => Some(ShapePrimitive::line(*start, *end, *width, *color)),
                // Rectangles are already T1, but we could extract them too
                _ => None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shape_kind_primitives() {
        assert_eq!(ShapeKind::Circle.primitives(), "λ + N");
        assert_eq!(ShapeKind::Triangle.primitives(), "σ[λ,λ,λ]");
        assert_eq!(ShapeKind::Line.primitives(), "λ → λ");
    }

    #[test]
    fn test_color_primitive_from_color() {
        let c = Color {
            r: 255,
            g: 128,
            b: 0,
            a: 255,
        };
        let cp = ColorPrimitive::from_color(c);
        assert!((cp.r - 1.0).abs() < f32::EPSILON);
        assert!((cp.g - 0.502).abs() < 0.01);
        assert!(cp.b.abs() < f32::EPSILON);
    }

    #[test]
    fn test_color_named_match() {
        let red = ColorPrimitive {
            r: 0.9,
            g: 0.1,
            b: 0.1,
            a: 1.0,
        };
        assert_eq!(red.named_match(), "red");

        let blue = ColorPrimitive {
            r: 0.0,
            g: 0.0,
            b: 0.95,
            a: 1.0,
        };
        assert_eq!(blue.named_match(), "blue");
    }

    #[test]
    fn test_color_luminance() {
        let white = ColorPrimitive {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        };
        assert!((white.luminance() - 1.0).abs() < 0.01);

        let black = ColorPrimitive {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        };
        assert!(black.luminance().abs() < f32::EPSILON);
    }

    #[test]
    fn test_shape_primitive_circle() {
        let circle = ShapePrimitive::circle(
            Point::new(100.0, 100.0),
            50.0,
            Color {
                r: 255,
                g: 0,
                b: 0,
                a: 255,
            },
        );
        assert_eq!(circle.kind, ShapeKind::Circle);
        assert!((circle.radius - 50.0).abs() < f32::EPSILON);
        assert_eq!(circle.fill.named_match(), "red");
    }

    #[test]
    fn test_shape_primitive_triangle() {
        let tri = ShapePrimitive::triangle(
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            Point::new(50.0, 100.0),
            Color {
                r: 0,
                g: 255,
                b: 0,
                a: 255,
            },
        );
        assert_eq!(tri.kind, ShapeKind::Triangle);
        assert_eq!(tri.vertices.len(), 3);
    }

    #[test]
    fn test_transfer_confidence() {
        assert!(ShapeKind::Circle.transfer_confidence() > 0.9);
        assert!(ShapeKind::Triangle.transfer_confidence() >= 0.9);
        assert!(ShapeKind::Polygon.transfer_confidence() >= 0.8);
    }

    #[test]
    fn test_primitive_string() {
        let circle = ShapePrimitive::circle(
            Point::new(50.0, 50.0),
            25.0,
            Color {
                r: 0,
                g: 0,
                b: 255,
                a: 255,
            },
        );
        let s = circle.to_primitive_string();
        assert!(s.contains("λ + N"));
        assert!(s.contains("blue"));
    }
}
