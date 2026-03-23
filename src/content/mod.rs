//! Content rendering — trait + internal renderer wrapper.
//!
//! The `ContentRenderer` trait abstracts over different content types
//! (web pages, internal `nex://` pages, etc.). `InternalRenderer` wraps
//! the existing `Browser` struct.
//!
//! ## Tier Classification
//!
//! - `ContentRenderer`: T2-C (trait)
//! - `InternalRenderer`: T3 (domain wrapper)

pub mod form;

use crate::layout::Rect;
use crate::paint::DisplayCommand;

/// Tier: T2-C — Trait for content area rendering.
///
/// Implementations handle navigation, display list generation,
/// scrolling, and link resolution.
pub trait ContentRenderer {
    /// Navigate to a URL.
    ///
    /// # Errors
    /// Returns error if navigation fails.
    fn navigate(&mut self, url: &str) -> crate::Result<()>;

    /// Get the current display list.
    fn display_list(&self) -> &[DisplayCommand];

    /// Get the page title.
    fn title(&self) -> &str;

    /// Get the current URL.
    fn current_url(&self) -> &str;

    /// Scroll by vertical delta.
    fn scroll(&mut self, dy: f32);

    /// Find a link at screen coordinates.
    fn find_link_at(&self, x: f32, y: f32) -> Option<String>;

    /// Resize the content viewport.
    fn resize(&mut self, width: f32, height: f32);

    /// Get the content viewport.
    fn viewport(&self) -> Rect;
}

/// Tier: T3 — Wraps the existing `Browser` as a `ContentRenderer`.
pub struct InternalRenderer {
    browser: crate::Browser,
}

impl InternalRenderer {
    /// Create a new internal renderer.
    #[must_use]
    pub fn new() -> Self {
        Self {
            browser: crate::Browser::new(),
        }
    }

    /// Get a reference to the underlying browser.
    #[must_use]
    pub fn browser(&self) -> &crate::Browser {
        &self.browser
    }

    /// Get a mutable reference to the underlying browser.
    pub fn browser_mut(&mut self) -> &mut crate::Browser {
        &mut self.browser
    }
}

impl Default for InternalRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentRenderer for InternalRenderer {
    fn navigate(&mut self, url: &str) -> crate::Result<()> {
        self.browser.navigate(url)
    }

    fn display_list(&self) -> &[DisplayCommand] {
        self.browser.display_list()
    }

    fn title(&self) -> &str {
        self.browser.active_tab().map_or("New Tab", |t| &t.title)
    }

    fn current_url(&self) -> &str {
        self.browser.current_url()
    }

    fn scroll(&mut self, dy: f32) {
        self.browser.scroll(dy);
    }

    fn find_link_at(&self, x: f32, y: f32) -> Option<String> {
        self.browser.find_link_at(x, y)
    }

    fn resize(&mut self, width: f32, height: f32) {
        self.browser.resize(width, height);
    }

    fn viewport(&self) -> Rect {
        self.browser.viewport()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_internal_renderer_creation() {
        let renderer = InternalRenderer::new();
        assert_eq!(renderer.title(), "New Tab");
        assert_eq!(renderer.current_url(), "about:blank");
    }

    #[test]
    fn test_display_list_empty_initially() {
        let renderer = InternalRenderer::new();
        assert!(renderer.display_list().is_empty());
    }
}
