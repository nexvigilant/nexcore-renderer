//! Paint module - renders layout boxes to display commands.

use crate::layout::{BoxContent, LayoutBox, Rect};
use crate::style::{Color, ListStyleType, TextAlign, TextDecoration};

pub mod image;

/// A point in 2D space.
///
/// Tier: T2-P (primitive composition: λ + λ)
/// Grounding: location_x + location_y
#[derive(Debug, Clone, Copy, Default)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    /// Create a new point.
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// A display list command.
///
/// Each command carries an optional `node_id` that maps back to the
/// originating DOM element. This allows hit-testing against rendered
/// content to resolve which DOM node was clicked.
///
/// ## Shape Primitives (Phase 4)
///
/// | Command | T1 Primitives | Grounding |
/// |---------|---------------|-----------|
/// | FillRect | σ + N + N | sequence of 4 vertices, width/height |
/// | FillCircle | λ + N | center location + radius |
/// | FillTriangle | σ[λ,λ,λ] | sequence of 3 locations |
/// | StrokeLine | λ → λ + N | start → end causality + width |
#[derive(Debug, Clone)]
pub enum DisplayCommand {
    /// Fill a rectangle with color.
    FillRect {
        rect: Rect,
        color: Color,
        /// Source node identifier for hit-testing.
        node_id: Option<usize>,
    },
    /// Fill a circle with color.
    ///
    /// Tier: T2-P (λ + N + μ)
    /// Grounding: center_location + radius_quantity + color_mapping
    FillCircle {
        center: Point,
        radius: f32,
        color: Color,
        node_id: Option<usize>,
    },
    /// Fill a triangle with color.
    ///
    /// Tier: T2-C (σ[λ,λ,λ] + μ)
    /// Grounding: sequence_of_3_locations + color_mapping
    FillTriangle {
        p1: Point,
        p2: Point,
        p3: Point,
        color: Color,
        node_id: Option<usize>,
    },
    /// Stroke a line between two points.
    ///
    /// Tier: T2-P (λ → λ + N + μ)
    /// Grounding: start_location → end_location + width_quantity + color_mapping
    StrokeLine {
        start: Point,
        end: Point,
        width: f32,
        color: Color,
        node_id: Option<usize>,
    },
    /// Draw text at position.
    DrawText {
        text: String,
        x: f32,
        y: f32,
        size: f32,
        color: Color,
        /// Source node identifier for hit-testing.
        node_id: Option<usize>,
    },
    /// Draw an image.
    DrawImage {
        src: String,
        rect: Rect,
        /// Source node identifier for hit-testing.
        node_id: Option<usize>,
    },
    /// Blit raw RGBA framebuffer data at a position.
    ///
    /// Used by the NVOS compositor bridge to upload CPU-composited
    /// frames into the GPU pipeline.
    ///
    /// Tier: T2-C (μ + ∂ — framebuffer_mapping at boundary)
    BlitRgba {
        /// Target rectangle on screen.
        rect: Rect,
        /// Source framebuffer width in pixels.
        width: u32,
        /// Source framebuffer height in pixels.
        height: u32,
        /// Raw RGBA pixel data (4 bytes per pixel).
        data: Vec<u8>,
    },
}

impl DisplayCommand {
    /// Get the node ID associated with this display command.
    #[must_use]
    pub fn node_id(&self) -> Option<usize> {
        match self {
            Self::FillRect { node_id, .. }
            | Self::FillCircle { node_id, .. }
            | Self::FillTriangle { node_id, .. }
            | Self::StrokeLine { node_id, .. }
            | Self::DrawText { node_id, .. }
            | Self::DrawImage { node_id, .. } => *node_id,
            Self::BlitRgba { .. } => None,
        }
    }

    /// Get the bounding rect for hit-testing.
    #[must_use]
    pub fn hit_rect(&self) -> Option<Rect> {
        match self {
            Self::FillRect { rect, .. } | Self::DrawImage { rect, .. } => Some(*rect),
            Self::FillCircle { center, radius, .. } => Some(Rect {
                x: center.x - radius,
                y: center.y - radius,
                width: radius * 2.0,
                height: radius * 2.0,
            }),
            Self::FillTriangle { p1, p2, p3, .. } => {
                let min_x = p1.x.min(p2.x).min(p3.x);
                let max_x = p1.x.max(p2.x).max(p3.x);
                let min_y = p1.y.min(p2.y).min(p3.y);
                let max_y = p1.y.max(p2.y).max(p3.y);
                Some(Rect {
                    x: min_x,
                    y: min_y,
                    width: max_x - min_x,
                    height: max_y - min_y,
                })
            }
            Self::StrokeLine {
                start, end, width, ..
            } => {
                let min_x = start.x.min(end.x) - width / 2.0;
                let max_x = start.x.max(end.x) + width / 2.0;
                let min_y = start.y.min(end.y) - width / 2.0;
                let max_y = start.y.max(end.y) + width / 2.0;
                Some(Rect {
                    x: min_x,
                    y: min_y,
                    width: max_x - min_x,
                    height: max_y - min_y,
                })
            }
            Self::DrawText {
                x, y, size, text, ..
            } => {
                // Approximate text bounds: width ~ char_count * size * 0.6, height ~ size * 1.2
                let approx_width = text.len() as f32 * size * 0.6;
                let approx_height = size * 1.2;
                Some(Rect {
                    x: *x,
                    y: *y - *size,
                    width: approx_width,
                    height: approx_height,
                })
            }
            Self::BlitRgba { rect, .. } => Some(*rect),
        }
    }
}

/// Counter for assigning node IDs during display list construction.
struct PaintContext {
    next_id: usize,
}

impl PaintContext {
    fn new() -> Self {
        Self { next_id: 0 }
    }

    fn next(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

/// Build display list from layout tree.
#[must_use]
pub fn build_display_list(root: &LayoutBox) -> Vec<DisplayCommand> {
    let mut commands = Vec::new();
    let mut ctx = PaintContext::new();
    paint_box(root, &mut commands, &mut ctx);
    commands
}

fn paint_box(layout_box: &LayoutBox, commands: &mut Vec<DisplayCommand>, ctx: &mut PaintContext) {
    let node_id = ctx.next();
    let style = &layout_box.style;
    let rect = layout_box.rect;

    // Background
    if style.background_color.a > 0 {
        commands.push(DisplayCommand::FillRect {
            rect,
            color: apply_opacity(style.background_color, style.opacity),
            node_id: Some(node_id),
        });
    }

    // Borders (rendered as thin filled rects on each edge)
    let bc = apply_opacity(style.border_color, style.opacity);
    if style.border.top > 0.0 && bc.a > 0 {
        commands.push(DisplayCommand::FillRect {
            rect: Rect {
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: style.border.top,
            },
            color: bc,
            node_id: Some(node_id),
        });
    }
    if style.border.bottom > 0.0 && bc.a > 0 {
        commands.push(DisplayCommand::FillRect {
            rect: Rect {
                x: rect.x,
                y: rect.y + rect.height - style.border.bottom,
                width: rect.width,
                height: style.border.bottom,
            },
            color: bc,
            node_id: Some(node_id),
        });
    }
    if style.border.left > 0.0 && bc.a > 0 {
        commands.push(DisplayCommand::FillRect {
            rect: Rect {
                x: rect.x,
                y: rect.y,
                width: style.border.left,
                height: rect.height,
            },
            color: bc,
            node_id: Some(node_id),
        });
    }
    if style.border.right > 0.0 && bc.a > 0 {
        commands.push(DisplayCommand::FillRect {
            rect: Rect {
                x: rect.x + rect.width - style.border.right,
                y: rect.y,
                width: style.border.right,
                height: rect.height,
            },
            color: bc,
            node_id: Some(node_id),
        });
    }

    // List marker (bullet/number)
    if style.list_style_type != ListStyleType::None {
        let marker = match style.list_style_type {
            ListStyleType::Disc => "\u{2022}",   // •
            ListStyleType::Circle => "\u{25CB}", // ○
            ListStyleType::Square => "\u{25A0}", // ■
            ListStyleType::Decimal => "#",       // simplified — ideally counter
            ListStyleType::None => "",
        };
        if !marker.is_empty() {
            commands.push(DisplayCommand::DrawText {
                text: marker.to_string(),
                x: rect.x - 16.0,
                y: rect.y + style.font_size,
                size: style.font_size,
                color: apply_opacity(style.color, style.opacity),
                node_id: Some(node_id),
            });
        }
    }

    // Content
    match &layout_box.content {
        BoxContent::Text(text) => {
            let text_color = apply_opacity(style.color, style.opacity);

            // Text-align offset (approximate: assume char width ≈ size * 0.6)
            let approx_text_width = text.len() as f32 * style.font_size * 0.6;
            let x_offset = match style.text_align {
                TextAlign::Left => 0.0,
                TextAlign::Center => (rect.width - approx_text_width).max(0.0) / 2.0,
                TextAlign::Right => (rect.width - approx_text_width).max(0.0),
            };

            let text_x = rect.x + x_offset;
            let text_y = rect.y + style.font_size;

            commands.push(DisplayCommand::DrawText {
                text: text.clone(),
                x: text_x,
                y: text_y,
                size: style.font_size,
                color: text_color,
                node_id: Some(node_id),
            });

            // Text decoration: underline
            if style.text_decoration == TextDecoration::Underline {
                let underline_y = text_y + 2.0;
                commands.push(DisplayCommand::StrokeLine {
                    start: Point::new(text_x, underline_y),
                    end: Point::new(text_x + approx_text_width.min(rect.width), underline_y),
                    width: 1.0,
                    color: text_color,
                    node_id: Some(node_id),
                });
            }

            // Text decoration: line-through
            if style.text_decoration == TextDecoration::LineThrough {
                let strike_y = rect.y + style.font_size * 0.6;
                commands.push(DisplayCommand::StrokeLine {
                    start: Point::new(text_x, strike_y),
                    end: Point::new(text_x + approx_text_width.min(rect.width), strike_y),
                    width: 1.0,
                    color: text_color,
                    node_id: Some(node_id),
                });
            }
        }
        BoxContent::Image { src } => {
            commands.push(DisplayCommand::DrawImage {
                src: src.clone(),
                rect,
                node_id: Some(node_id),
            });
        }
        _ => {}
    }

    // Children
    for child in &layout_box.children {
        paint_box(child, commands, ctx);
    }
}

/// Apply opacity to a color's alpha channel.
fn apply_opacity(color: Color, opacity: f32) -> Color {
    if (opacity - 1.0).abs() < f32::EPSILON {
        return color;
    }
    Color {
        r: color.r,
        g: color.g,
        b: color.b,
        a: ((color.a as f32) * opacity) as u8,
    }
}

/// Append form element display commands to an existing display list.
///
/// Takes a `FormRegistry` and produces display commands for all registered
/// form elements, appending them after the main page content so they render
/// on top.
///
/// Tier: T2-C (σ + μ — sequence of form elements mapped to display commands)
pub fn append_form_commands(
    commands: &mut Vec<DisplayCommand>,
    registry: &crate::content::form::FormRegistry,
) {
    commands.extend(registry.build_display_commands());
}

/// Collect unique image URLs from a display list.
///
/// Scans for `DrawImage` commands and returns deduplicated, non-empty
/// source URLs in encounter order. Used to preload images after navigation.
///
/// Tier: T2-P (σ + κ — sequence scan with comparison dedup)
#[must_use]
pub fn collect_image_urls(commands: &[DisplayCommand]) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    let mut urls = Vec::new();
    for cmd in commands {
        if let DisplayCommand::DrawImage { src, .. } = cmd {
            if !src.is_empty() && seen.insert(src.clone()) {
                urls.push(src.clone());
            }
        }
    }
    urls
}

/// Build hit-test regions from a display list.
///
/// Returns pairs of `(node_id, rect)` for every command that has
/// both a node ID and a computable bounding rect. Ordered in paint
/// order (back to front).
#[must_use]
pub fn build_hit_regions(commands: &[DisplayCommand]) -> Vec<(usize, Rect)> {
    commands
        .iter()
        .filter_map(|cmd| {
            let id = cmd.node_id()?;
            let rect = cmd.hit_rect()?;
            Some((id, rect))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rect() -> Rect {
        Rect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        }
    }

    fn make_draw_image(src: &str) -> DisplayCommand {
        DisplayCommand::DrawImage {
            src: src.to_string(),
            rect: make_rect(),
            node_id: Some(0),
        }
    }

    #[test]
    fn test_collect_image_urls_basic() {
        let commands = vec![
            make_draw_image("https://example.com/logo.png"),
            make_draw_image("https://example.com/hero.jpg"),
        ];
        let urls = collect_image_urls(&commands);
        assert_eq!(urls.len(), 2);
        assert_eq!(urls[0], "https://example.com/logo.png");
        assert_eq!(urls[1], "https://example.com/hero.jpg");
    }

    #[test]
    fn test_collect_image_urls_dedup() {
        let commands = vec![
            make_draw_image("logo.png"),
            make_draw_image("hero.jpg"),
            make_draw_image("logo.png"), // duplicate
        ];
        let urls = collect_image_urls(&commands);
        assert_eq!(urls.len(), 2);
    }

    #[test]
    fn test_collect_image_urls_skip_empty() {
        let commands = vec![make_draw_image(""), make_draw_image("logo.png")];
        let urls = collect_image_urls(&commands);
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0], "logo.png");
    }

    #[test]
    fn test_collect_image_urls_no_images() {
        let commands = vec![DisplayCommand::FillRect {
            rect: make_rect(),
            color: Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            node_id: None,
        }];
        let urls = collect_image_urls(&commands);
        assert!(urls.is_empty());
    }

    #[test]
    fn test_collect_image_urls_empty_list() {
        let urls = collect_image_urls(&[]);
        assert!(urls.is_empty());
    }

    #[test]
    fn test_collect_image_urls_mixed_commands() {
        let commands = vec![
            DisplayCommand::FillRect {
                rect: make_rect(),
                color: Color {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                },
                node_id: Some(0),
            },
            make_draw_image("img/photo.webp"),
            DisplayCommand::DrawText {
                text: "Hello".to_string(),
                x: 10.0,
                y: 20.0,
                size: 16.0,
                color: Color {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 255,
                },
                node_id: Some(1),
            },
            make_draw_image("img/icon.gif"),
        ];
        let urls = collect_image_urls(&commands);
        assert_eq!(urls.len(), 2);
        assert_eq!(urls[0], "img/photo.webp");
        assert_eq!(urls[1], "img/icon.gif");
    }
}
