//! GPU-native text rendering using Vello's `draw_glyphs()`.
//!
//! ## Architecture
//!
//! - **Shaping**: cosmic-text (FontSystem, Buffer, layout_runs)
//! - **Rendering**: Vello's `scene.draw_glyphs()` for GPU-native paths
//!
//! ## Tier Classification
//!
//! - **T1**: Glyph (id: N, x: N, y: N)
//! - **T2-P**: FontData (mapping μ), LayoutGlyph (position λ)
//! - **T2-C**: GpuTextShaper (composes T1/T2-P)
//!
//! ## Primitive Grounding
//!
//! | Concept | T1 Primitive | Symbol |
//! |---------|-------------|--------|
//! | Glyph Shape | Sequence + Boundary | σ + ∂ |
//! | Font Mapping | Mapping | μ |
//! | Text Position | Location | λ |
//! | Render Pass | Causality | → |

use crate::style::Color;
use cosmic_text::fontdb;
use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping};
use std::collections::BTreeMap;
use std::sync::Arc;
use vello::Glyph;
use vello::peniko::{self, Blob, Fill};

/// Cached font data for Vello rendering.
///
/// Tier: T2-P (cross-domain: cosmic-text → Vello)
#[derive(Clone)]
pub struct CachedFont {
    /// Raw font data blob for Vello.
    pub data: Arc<Vec<u8>>,
    /// Font face index within the file.
    pub index: u32,
}

/// GPU text shaper using cosmic-text for layout and Vello for rendering.
///
/// Tier: T2-C (composes FontSystem + font cache + layout)
pub struct GpuTextShaper {
    /// cosmic-text font system for shaping.
    font_system: FontSystem,
    /// Cache of font data by fontdb ID.
    ///
    /// `fontdb::ID` derives `Ord` (via slotmap), so it can be used directly
    /// as a `BTreeMap` key for deterministic iteration.
    font_cache: BTreeMap<fontdb::ID, CachedFont>,
}

impl Default for GpuTextShaper {
    fn default() -> Self {
        Self::new()
    }
}

impl GpuTextShaper {
    /// Create a new GPU text shaper.
    #[must_use]
    pub fn new() -> Self {
        Self {
            font_system: FontSystem::new(),
            font_cache: BTreeMap::new(),
        }
    }

    /// Shape text and return positioned glyphs ready for Vello rendering.
    ///
    /// Returns: Vec of (font_data, font_size, glyphs, color)
    #[must_use]
    pub fn shape_text(
        &mut self,
        text: &str,
        font_size: f32,
        color: Color,
        max_width: f32,
        base_x: f32,
        base_y: f32,
    ) -> Vec<ShapedRun> {
        let metrics = Metrics::new(font_size, font_size * 1.2);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);

        buffer.set_size(&mut self.font_system, Some(max_width), None);

        let attrs = Attrs::new().family(Family::SansSerif);
        buffer.set_text(&mut self.font_system, text, attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut self.font_system, false);

        let mut runs = Vec::new();

        for layout_run in buffer.layout_runs() {
            let mut glyphs = Vec::new();
            let mut current_font_id: Option<fontdb::ID> = None;

            for glyph in layout_run.glyphs.iter() {
                // Get the font ID for this glyph
                let font_id = glyph.font_id;

                // If font changed, flush current run
                if let Some(prev_id) = current_font_id {
                    if prev_id != font_id && !glyphs.is_empty() {
                        if let Some(cached) = self.get_or_cache_font(prev_id) {
                            runs.push(ShapedRun {
                                font_data: cached.data.clone(),
                                font_index: cached.index,
                                font_size,
                                color,
                                glyphs: std::mem::take(&mut glyphs),
                            });
                        }
                    }
                }
                current_font_id = Some(font_id);

                // Convert cosmic-text glyph to Vello glyph
                let vello_glyph = Glyph {
                    id: u32::from(glyph.glyph_id),
                    x: base_x + glyph.x + glyph.x_offset,
                    y: base_y + layout_run.line_y + glyph.y_offset,
                };
                glyphs.push(vello_glyph);
            }

            // Flush remaining glyphs
            if let Some(font_id) = current_font_id {
                if !glyphs.is_empty() {
                    if let Some(cached) = self.get_or_cache_font(font_id) {
                        runs.push(ShapedRun {
                            font_data: cached.data.clone(),
                            font_index: cached.index,
                            font_size,
                            color,
                            glyphs,
                        });
                    }
                }
            }
        }

        runs
    }

    /// Get or cache font data for a fontdb ID.
    fn get_or_cache_font(&mut self, font_id: fontdb::ID) -> Option<CachedFont> {
        if let Some(cached) = self.font_cache.get(&font_id) {
            return Some(cached.clone());
        }

        // Extract font data from fontdb
        let db = self.font_system.db();
        let mut result: Option<CachedFont> = None;

        db.with_face_data(font_id, |data, face_index| {
            let cached = CachedFont {
                data: Arc::new(data.to_vec()),
                index: face_index,
            };
            result = Some(cached);
        });

        if let Some(cached) = result {
            self.font_cache.insert(font_id, cached.clone());
            Some(cached)
        } else {
            None
        }
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

        let mut max_x: f32 = 0.0;
        let mut max_y: f32 = 0.0;

        for run in buffer.layout_runs() {
            for glyph in run.glyphs.iter() {
                let x = glyph.x + glyph.w;
                let y = run.line_y + glyph.y_offset + run.line_height;
                if x > max_x {
                    max_x = x;
                }
                if y > max_y {
                    max_y = y;
                }
            }
        }

        (max_x.ceil(), max_y.ceil())
    }
}

/// A shaped text run ready for Vello rendering.
///
/// Tier: T2-C (composition of font + glyphs + style)
pub struct ShapedRun {
    /// Font data bytes.
    pub font_data: Arc<Vec<u8>>,
    /// Font face index.
    pub font_index: u32,
    /// Font size in pixels.
    pub font_size: f32,
    /// Text color.
    pub color: Color,
    /// Positioned glyphs.
    pub glyphs: Vec<Glyph>,
}

impl ShapedRun {
    /// Render this run into a Vello scene.
    ///
    /// Uses `scene.draw_glyphs()` for native GPU path rendering.
    pub fn render_to_scene(&self, scene: &mut vello::Scene) {
        if self.glyphs.is_empty() {
            return;
        }

        // Create FontData from our cached data
        // Blob::from takes Vec<u8>, we need to clone since we have Arc<Vec<u8>>
        let blob: Blob<u8> = Blob::from((*self.font_data).clone());
        let font_data = peniko::FontData::new(blob, self.font_index);

        let brush = peniko::Brush::Solid(peniko::Color::from_rgba8(
            self.color.r,
            self.color.g,
            self.color.b,
            self.color.a,
        ));

        scene
            .draw_glyphs(&font_data)
            .font_size(self.font_size)
            .brush(&brush)
            .draw(Fill::NonZero, self.glyphs.iter().copied());
    }
}

/// Render text directly to a Vello scene using GPU-native paths.
///
/// This is the Phase 3c entry point - no CPU rasterization, pure GPU.
///
/// ## Primitive Grounding
/// - Input: text (σ), position (λ), style (μ)
/// - Output: GPU draw commands (→)
pub fn render_text_gpu(
    scene: &mut vello::Scene,
    shaper: &mut GpuTextShaper,
    text: &str,
    x: f32,
    y: f32,
    font_size: f32,
    color: Color,
    max_width: f32,
) {
    let runs = shaper.shape_text(text, font_size, color, max_width, x, y);
    for run in runs {
        run.render_to_scene(scene);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_text_shaper_creation() {
        let shaper = GpuTextShaper::new();
        assert!(shaper.font_cache.is_empty());
    }

    #[test]
    fn test_shape_empty_text() {
        let mut shaper = GpuTextShaper::new();
        let runs = shaper.shape_text("", 16.0, Color::WHITE, 500.0, 0.0, 0.0);
        assert!(runs.is_empty());
    }

    #[test]
    fn test_shape_simple_text() {
        let mut shaper = GpuTextShaper::new();
        let runs = shaper.shape_text("Hello", 16.0, Color::WHITE, 500.0, 0.0, 0.0);
        // Should produce at least one run with glyphs
        assert!(!runs.is_empty());
        assert!(!runs[0].glyphs.is_empty());
    }

    #[test]
    fn test_measure_text() {
        let mut shaper = GpuTextShaper::new();
        let (w, h) = shaper.measure_text("Hello World", 16.0, 500.0);
        assert!(w > 0.0);
        assert!(h > 0.0);
    }

    #[test]
    fn test_font_caching() {
        let mut shaper = GpuTextShaper::new();
        // Shape twice to test caching
        let _ = shaper.shape_text("Test", 16.0, Color::WHITE, 500.0, 0.0, 0.0);
        let _ = shaper.shape_text("Test2", 16.0, Color::WHITE, 500.0, 0.0, 0.0);
        // Font cache should have entries now
        assert!(!shaper.font_cache.is_empty());
    }

    #[test]
    fn test_glyph_positions() {
        let mut shaper = GpuTextShaper::new();
        let runs = shaper.shape_text("AB", 16.0, Color::WHITE, 500.0, 10.0, 20.0);
        if !runs.is_empty() && runs[0].glyphs.len() >= 2 {
            // First glyph should be at base position
            assert!(runs[0].glyphs[0].x >= 10.0);
            // Second glyph should be offset from first
            assert!(runs[0].glyphs[1].x > runs[0].glyphs[0].x);
        }
    }

    #[test]
    fn test_shaped_run_fields() {
        let mut shaper = GpuTextShaper::new();
        let red = Color {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        };
        let runs = shaper.shape_text("X", 24.0, red, 500.0, 0.0, 0.0);
        if !runs.is_empty() {
            assert_eq!(runs[0].font_size, 24.0);
            assert_eq!(runs[0].color.r, 255);
            assert!(!runs[0].font_data.is_empty());
        }
    }

    #[test]
    fn test_render_to_scene() {
        let mut shaper = GpuTextShaper::new();
        let runs = shaper.shape_text("Test", 16.0, Color::WHITE, 500.0, 0.0, 0.0);
        let mut scene = vello::Scene::new();
        for run in runs {
            run.render_to_scene(&mut scene);
        }
        // Scene should have encoded data now (hard to verify internals)
    }
}
