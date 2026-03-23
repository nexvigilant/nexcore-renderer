//! Text rendering using cosmic-text.

use crate::style::Color;
use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping, SwashCache};

/// Text renderer using cosmic-text for shaping and rasterization.
pub struct TextRenderer {
    font_system: FontSystem,
    swash_cache: SwashCache,
}

impl Default for TextRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl TextRenderer {
    /// Create a new text renderer with system fonts.
    #[must_use]
    pub fn new() -> Self {
        Self {
            font_system: FontSystem::new(),
            swash_cache: SwashCache::new(),
        }
    }

    /// Render text to RGBA pixels.
    ///
    /// Returns (width, height, rgba_pixels).
    #[must_use]
    pub fn render_text(
        &mut self,
        text: &str,
        font_size: f32,
        color: Color,
        max_width: f32,
    ) -> (u32, u32, Vec<u8>) {
        let metrics = Metrics::new(font_size, font_size * 1.2);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);

        buffer.set_size(&mut self.font_system, Some(max_width), None);

        let attrs = Attrs::new()
            .family(Family::SansSerif)
            .color(cosmic_text::Color::rgba(color.r, color.g, color.b, color.a));

        buffer.set_text(&mut self.font_system, text, attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut self.font_system, false);

        // Calculate bounds
        let (width, height) = self.measure_buffer(&buffer);
        if width == 0 || height == 0 {
            return (1, 1, vec![0, 0, 0, 0]);
        }

        // Rasterize glyphs
        let mut pixels = vec![0u8; (width * height * 4) as usize];

        buffer.draw(
            &mut self.font_system,
            &mut self.swash_cache,
            cosmic_text::Color::rgba(color.r, color.g, color.b, color.a),
            |x, y, w, h, color| {
                let x = x as u32;
                let y = y as u32;
                for dy in 0..h {
                    for dx in 0..w {
                        let px = x + dx;
                        let py = y + dy;
                        if px < width && py < height {
                            let idx = ((py * width + px) * 4) as usize;
                            if idx + 3 < pixels.len() {
                                // Alpha blend
                                let src_a = color.a() as f32 / 255.0;
                                let dst_a = pixels[idx + 3] as f32 / 255.0;
                                let out_a = src_a + dst_a * (1.0 - src_a);

                                if out_a > 0.0 {
                                    pixels[idx] = ((color.r() as f32 * src_a
                                        + pixels[idx] as f32 * dst_a * (1.0 - src_a))
                                        / out_a)
                                        as u8;
                                    pixels[idx + 1] = ((color.g() as f32 * src_a
                                        + pixels[idx + 1] as f32 * dst_a * (1.0 - src_a))
                                        / out_a)
                                        as u8;
                                    pixels[idx + 2] = ((color.b() as f32 * src_a
                                        + pixels[idx + 2] as f32 * dst_a * (1.0 - src_a))
                                        / out_a)
                                        as u8;
                                    pixels[idx + 3] = (out_a * 255.0) as u8;
                                }
                            }
                        }
                    }
                }
            },
        );

        (width, height, pixels)
    }

    fn measure_buffer(&self, buffer: &Buffer) -> (u32, u32) {
        let mut max_x: f32 = 0.0;
        let mut max_y: f32 = 0.0;

        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                let x = glyph.x + glyph.w;
                let y = run.line_y + glyph.y + run.line_height;
                if x > max_x {
                    max_x = x;
                }
                if y > max_y {
                    max_y = y;
                }
            }
        }

        ((max_x.ceil() as u32).max(1), (max_y.ceil() as u32).max(1))
    }

    /// Measure text dimensions without rendering.
    #[must_use]
    pub fn measure_text(&mut self, text: &str, font_size: f32, max_width: f32) -> (f32, f32) {
        let metrics = Metrics::new(font_size, font_size * 1.2);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        buffer.set_size(&mut self.font_system, Some(max_width), None);

        let attrs = Attrs::new().family(Family::SansSerif);
        buffer.set_text(&mut self.font_system, text, attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut self.font_system, false);

        let (w, h) = self.measure_buffer(&buffer);
        (w as f32, h as f32)
    }
}

/// Cached text texture for GPU rendering.
pub struct TextTexture {
    /// Texture width.
    pub width: u32,
    /// Texture height.
    pub height: u32,
    /// RGBA pixel data.
    pub pixels: Vec<u8>,
}

impl TextTexture {
    /// Create from rendered text.
    #[must_use]
    pub fn from_text(renderer: &mut TextRenderer, text: &str, size: f32, color: Color) -> Self {
        let (width, height, pixels) = renderer.render_text(text, size, color, 2000.0);
        Self {
            width,
            height,
            pixels,
        }
    }
}
