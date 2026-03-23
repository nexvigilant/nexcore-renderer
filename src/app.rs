//! Browser application - window and event loop.

use crate::Result;
use crate::dom::{Arena, NodeId as DomNodeId};
use crate::layout::{LayoutBox, LayoutEngine, Rect};
use crate::net;
use crate::paint::{DisplayCommand, build_display_list, build_hit_regions};
use crate::style::StyledNode;
use crate::style::cascade::{CssRule, StyleOrigin};
use crate::style::parse::{extract_link_hrefs_arena, parse_stylesheet};

/// Browser state (T3 domain-specific).
pub struct Browser {
    tabs: Vec<Tab>,
    active_tab: usize,
    layout_engine: LayoutEngine,
    viewport: Rect,
    scroll_y: f32,
    zoom_level: f32,
    /// Hit-test regions built from the current display list.
    hit_regions: Vec<(usize, Rect)>,
    /// Maps paint-order ID → arena NodeId (replaces old NodeInfo/flatten).
    paint_to_arena: Vec<DomNodeId>,
}

/// A browser tab.
pub struct Tab {
    /// Current URL.
    pub url: String,
    /// Page title.
    pub title: String,
    /// Arena-based DOM tree.
    pub dom: Option<Arena>,
    /// Computed layout.
    pub layout: Option<LayoutBox>,
    /// Display commands for rendering.
    pub display_list: Vec<DisplayCommand>,
}

impl Default for Browser {
    fn default() -> Self {
        Self::new()
    }
}

impl Browser {
    /// Create a new browser instance.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tabs: vec![Tab::new("about:blank")],
            active_tab: 0,
            layout_engine: LayoutEngine::new(),
            viewport: Rect {
                x: 0.0,
                y: 0.0,
                width: 1280.0,
                height: 720.0,
            },
            scroll_y: 0.0,
            zoom_level: 1.0,
            hit_regions: Vec::new(),
            paint_to_arena: Vec::new(),
        }
    }

    /// Scroll the page by delta.
    pub fn scroll(&mut self, dy: f32) {
        self.scroll_y = (self.scroll_y + dy).max(0.0);
        tracing::debug!("Scroll Y: {}", self.scroll_y);
    }

    /// Zoom the page by factor.
    pub fn zoom(&mut self, factor: f32) {
        self.zoom_level = (self.zoom_level * factor).clamp(0.25, 4.0);
        tracing::debug!("Zoom: {}%", self.zoom_level * 100.0);
    }

    /// Navigate the active tab to a URL.
    ///
    /// # Errors
    /// Returns error if URL fetch or parsing fails.
    pub fn navigate(&mut self, url: &str) -> Result<()> {
        let html = net::fetch(url)?;
        let arena = Arena::parse(&html);

        // Fetch external <link rel="stylesheet"> CSS
        let external_rules = fetch_link_stylesheets(&arena, url);

        let styled = StyledNode::from_arena(&arena, &external_rules);
        let layout = self.layout_engine.layout(&styled, self.viewport, &arena);
        let display_list = build_display_list(&layout);
        self.hit_regions = build_hit_regions(&display_list);

        // Build paint-order → arena NodeId mapping (mirrors paint_box DFS order)
        self.paint_to_arena.clear();
        build_paint_map(&styled, &mut self.paint_to_arena);

        let title = extract_title(&arena).unwrap_or_else(|| url.to_string());

        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            tab.url = url.to_string();
            tab.title = title;
            tab.dom = Some(arena);
            tab.layout = Some(layout);
            tab.display_list = display_list;
        }
        self.scroll_y = 0.0;
        Ok(())
    }

    /// Get the active tab.
    #[must_use]
    pub fn active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active_tab)
    }

    /// Get the current URL of the active tab.
    #[must_use]
    pub fn current_url(&self) -> &str {
        self.tabs
            .get(self.active_tab)
            .map_or("about:blank", |t| &t.url)
    }

    /// Create a new tab.
    pub fn new_tab(&mut self, url: &str) {
        self.tabs.push(Tab::new(url));
        self.active_tab = self.tabs.len() - 1;
    }

    /// Close the active tab.
    pub fn close_tab(&mut self) {
        if self.tabs.len() > 1 {
            self.tabs.remove(self.active_tab);
            if self.active_tab >= self.tabs.len() {
                self.active_tab = self.tabs.len() - 1;
            }
        }
    }

    /// Set viewport size.
    pub fn resize(&mut self, width: f32, height: f32) {
        self.viewport.width = width;
        self.viewport.height = height;
    }

    /// Get the current viewport rectangle.
    #[must_use]
    pub fn viewport(&self) -> Rect {
        self.viewport
    }

    /// Get display list for rendering.
    #[must_use]
    pub fn display_list(&self) -> &[DisplayCommand] {
        self.tabs
            .get(self.active_tab)
            .map_or(&[], |t| &t.display_list)
    }

    /// Get hit-test regions for the current page.
    #[must_use]
    pub fn hit_regions(&self) -> &[(usize, Rect)] {
        &self.hit_regions
    }

    /// Find the href of an `<a>` tag at the given screen coordinates.
    ///
    /// Walks the arena ancestors from the hit node to find the nearest
    /// `<a>` ancestor with an `href` attribute, then resolves it against
    /// the current page URL.
    #[must_use]
    pub fn find_link_at(&self, x: f32, y: f32) -> Option<String> {
        use crate::input::HitTester;

        let paint_id = HitTester::hit_test(&self.hit_regions, x, y)?;
        self.resolve_link(paint_id)
    }

    /// Walk up from paint_id using arena ancestors to find nearest `<a href="...">`.
    fn resolve_link(&self, paint_id: usize) -> Option<String> {
        let arena_id = *self.paint_to_arena.get(paint_id)?;
        let arena = self.tabs.get(self.active_tab)?.dom.as_ref()?;

        // Check the node itself, then walk ancestors
        let mut current = Some(arena_id);
        while let Some(id) = current {
            if arena.tag(id) == Some("a")
                && let Some(attrs) = arena.attrs(id)
                && let Some(href) = attrs.get("href")
            {
                let base = self.current_url();
                return net::resolve(base, href).or_else(|| Some(href.clone()));
            }
            current = arena.parent(id);
        }
        None
    }
}

impl Tab {
    fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            title: "New Tab".to_string(),
            dom: None,
            layout: None,
            display_list: Vec::new(),
        }
    }
}

/// Extract page title from arena-based DOM.
fn extract_title(arena: &Arena) -> Option<String> {
    let title_id = arena.find_tag(arena.root(), "title")?;
    for &child in arena.children(title_id) {
        if let Some(t) = arena.text(child) {
            let trimmed = t.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

/// Build paint-order → arena NodeId mapping.
/// Mirrors the pre-order DFS in `paint_box`.
fn build_paint_map(styled: &StyledNode, out: &mut Vec<DomNodeId>) {
    out.push(styled.node_id);
    for child in &styled.children {
        build_paint_map(child, out);
    }
}

/// Fetch and parse external CSS from `<link rel="stylesheet">` elements.
///
/// Resolves each `href` against the base page URL, fetches the CSS text,
/// and parses it into `CssRule` values.  Failed fetches are silently skipped
/// (matching browser behavior — a missing stylesheet doesn't break the page).
fn fetch_link_stylesheets(arena: &Arena, base_url: &str) -> Vec<CssRule> {
    let hrefs = extract_link_hrefs_arena(arena);
    let mut rules = Vec::new();
    for href in &hrefs {
        let resolved = net::resolve(base_url, href).unwrap_or_else(|| href.clone());
        match net::fetch(&resolved) {
            Ok(css_text) => {
                rules.extend(parse_stylesheet(&css_text, StyleOrigin::Author));
            }
            Err(e) => {
                tracing::warn!("Failed to fetch stylesheet {}: {}", resolved, e);
            }
        }
    }
    rules
}
