//! Render backend abstraction dispatching to Vello or custom GPU renderer.
//!
//! Provides a unified `RenderBackend` enum so `window.rs` can swap between
//! rendering implementations without changing call sites.
//!
//! ## Tier Classification
//!
//! - **T2-C**: `RenderBackend` (composed enum over renderers)

use super::GpuRenderer;
use super::vello_renderer::VelloRenderer;
use crate::paint::DisplayCommand;
use crate::style::Color;
use crate::text::TextRenderer;
use winit::dpi::PhysicalSize;

/// Render backend dispatching to either Vello or the custom GPU renderer.
///
/// Tier: T2-C (composed enum over rendering implementations)
pub enum RenderBackend {
    /// Custom wgpu renderer from Phase 3a (fallback).
    Custom(GpuRenderer),
    /// Vello compute-based 2D renderer (primary).
    Vello(VelloRenderer),
}

impl RenderBackend {
    /// Resize the active renderer.
    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        match self {
            Self::Custom(r) => r.resize(size),
            Self::Vello(r) => r.resize(size),
        }
    }

    /// Render display commands to the screen.
    ///
    /// # Errors
    /// Returns error if the active renderer fails.
    pub fn render(
        &mut self,
        commands: &[DisplayCommand],
        bg_color: Color,
        text_renderer: &mut TextRenderer,
    ) -> nexcore_error::Result<()> {
        match self {
            Self::Custom(r) => r.render(commands, bg_color, text_renderer),
            Self::Vello(r) => r.render(commands, bg_color, text_renderer),
        }
    }

    /// Get viewport size from the active renderer.
    #[must_use]
    pub fn size(&self) -> (u32, u32) {
        match self {
            Self::Custom(r) => r.size(),
            Self::Vello(r) => r.size(),
        }
    }

    /// Capture the current frame as a PNG byte vector.
    ///
    /// # Errors
    /// Returns error if GPU readback or PNG encoding fails.
    /// Only supported on the Vello backend.
    pub fn capture_to_png(&self) -> nexcore_error::Result<Vec<u8>> {
        match self {
            Self::Vello(r) => r.capture_to_png(),
            Self::Custom(_) => Err(nexcore_error::nexerror!(
                "Screenshot not supported on custom backend"
            )),
        }
    }

    /// Preload images into the renderer's cache.
    ///
    /// Resolves URLs against `base_url`, fetches bytes, decodes, and caches.
    /// Only implemented for the Vello backend; Custom backend is a no-op.
    pub fn preload_images(&mut self, urls: &[String], base_url: &str) {
        match self {
            Self::Vello(r) => r.preload_images(urls, base_url),
            Self::Custom(_) => {} // Custom renderer has no image cache
        }
    }
}
