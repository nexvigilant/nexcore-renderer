//! Browser chrome — UI widgets surrounding the content area.
//!
//! Provides the `Widget` trait, theme constants, and `ChromeLayout`
//! that arranges tab bar, toolbar, sidebar, and status bar around
//! the content area.
//!
//! ## Tier Classification
//!
//! - `Widget`: T2-C (trait composing layout + paint + event handling)
//! - `Theme`: T1 (constants)
//! - `ChromeLayout`: T2-C (spatial composition)

pub mod sidebar;
pub mod status_bar;
pub mod tab_bar;
pub mod toolbar;

use crate::layout::Rect;
use crate::paint::DisplayCommand;
use crate::state::{Message, WidgetId};
use crate::style::Color;

/// Tier: T2-C — Trait for all chrome UI widgets.
///
/// Widgets know how to lay themselves out, paint, and handle events.
pub trait Widget {
    /// Unique identifier for this widget.
    fn id(&self) -> WidgetId;

    /// Compute layout within available space, returning the claimed rect.
    fn layout(&mut self, available: Rect) -> Rect;

    /// Generate display commands for rendering.
    fn paint(&self) -> Vec<DisplayCommand>;

    /// Handle a widget event, optionally producing a message.
    fn handle_click(&mut self, x: f32, y: f32) -> Option<Message>;

    /// Hit-test: is the point inside this widget?
    fn hit_test(&self, x: f32, y: f32) -> bool;
}

/// NexBrowser dark theme constants — grounded to Lex Primitiva visual language.
///
/// Tier: T1 — Pure constant values.
///
/// Each of the 16 Lex Primitiva symbols has a signature color (`PRIMITIVE_*`).
/// Chrome surfaces reference these primitive colors for semantic meaning.
pub struct Theme;

impl Theme {
    // ── Chrome dimensions ──────────────────────────────────────
    /// Tab bar height in pixels.
    pub const TAB_BAR_HEIGHT: f32 = 36.0;
    /// Toolbar (nav + URL bar) height in pixels.
    pub const TOOLBAR_HEIGHT: f32 = 40.0;
    /// Sidebar width when visible.
    pub const SIDEBAR_WIDTH: f32 = 280.0;
    /// Status bar height in pixels.
    pub const STATUS_BAR_HEIGHT: f32 = 24.0;

    // ── Lex Primitiva Color Language (16 symbols) ──────────────
    /// σ Sequence — Electric Blue: flow, ordered progression.
    pub const PRIMITIVE_SEQUENCE: Color = Color {
        r: 79,
        g: 195,
        b: 247,
        a: 255,
    };
    /// μ Mapping — Amber: transformation, warm connection.
    pub const PRIMITIVE_MAPPING: Color = Color {
        r: 255,
        g: 183,
        b: 77,
        a: 255,
    };
    /// ς State — Violet: mutable, modal, in-between.
    pub const PRIMITIVE_STATE: Color = Color {
        r: 206,
        g: 147,
        b: 216,
        a: 255,
    };
    /// ρ Recursion — Teal: depth, self-reference.
    pub const PRIMITIVE_RECURSION: Color = Color {
        r: 77,
        g: 182,
        b: 172,
        a: 255,
    };
    /// ∅ Void — Slate Gray: absence, negative space.
    pub const PRIMITIVE_VOID: Color = Color {
        r: 120,
        g: 144,
        b: 156,
        a: 255,
    };
    /// ∂ Boundary — Crimson: hard edge, validation wall.
    pub const PRIMITIVE_BOUNDARY: Color = Color {
        r: 239,
        g: 83,
        b: 80,
        a: 255,
    };
    /// ν Frequency — Lime: pulse, rhythm, periodic.
    pub const PRIMITIVE_FREQUENCY: Color = Color {
        r: 156,
        g: 204,
        b: 101,
        a: 255,
    };
    /// ∃ Existence — White: presence, "is".
    pub const PRIMITIVE_EXISTENCE: Color = Color {
        r: 236,
        g: 239,
        b: 241,
        a: 255,
    };
    /// π Persistence — Gold: durable, stored.
    pub const PRIMITIVE_PERSISTENCE: Color = Color {
        r: 255,
        g: 213,
        b: 79,
        a: 255,
    };
    /// → Causality — Orange: cause-effect, directed.
    pub const PRIMITIVE_CAUSALITY: Color = Color {
        r: 255,
        g: 138,
        b: 101,
        a: 255,
    };
    /// κ Comparison — Cyan: measurement, scale.
    pub const PRIMITIVE_COMPARISON: Color = Color {
        r: 77,
        g: 208,
        b: 225,
        a: 255,
    };
    /// N Quantity — Green: numeric, countable.
    pub const PRIMITIVE_QUANTITY: Color = Color {
        r: 102,
        g: 187,
        b: 106,
        a: 255,
    };
    /// λ Location — Indigo: place, spatial.
    pub const PRIMITIVE_LOCATION: Color = Color {
        r: 121,
        g: 134,
        b: 203,
        a: 255,
    };
    /// ∝ Irreversibility — Deep Red: cannot undo.
    pub const PRIMITIVE_IRREVERSIBILITY: Color = Color {
        r: 198,
        g: 40,
        b: 40,
        a: 255,
    };
    /// Σ Sum — Coral: aggregation, totality.
    pub const PRIMITIVE_SUM: Color = Color {
        r: 240,
        g: 98,
        b: 146,
        a: 255,
    };
    /// × Product — Peach: combination, conjunction.
    pub const PRIMITIVE_PRODUCT: Color = Color {
        r: 255,
        g: 171,
        b: 145,
        a: 255,
    };

    // ── Chrome Base Colors (refined dark, warmer) ──────────────
    /// Tab bar background — Charcoal.
    pub const TAB_BAR_BG: Color = Color {
        r: 21,
        g: 21,
        b: 31,
        a: 255,
    };
    /// Active tab — Warm slate.
    pub const TAB_ACTIVE: Color = Color {
        r: 31,
        g: 31,
        b: 48,
        a: 255,
    };
    /// Inactive tab — Muted slate.
    pub const TAB_INACTIVE: Color = Color {
        r: 26,
        g: 26,
        b: 40,
        a: 255,
    };
    /// Tab text — Soft warm.
    pub const TAB_TEXT: Color = Color {
        r: 212,
        g: 204,
        b: 192,
        a: 255,
    };

    /// Toolbar background — Warm slate.
    pub const TOOLBAR_BG: Color = Color {
        r: 31,
        g: 31,
        b: 48,
        a: 255,
    };
    /// URL field background — Lifted surface.
    pub const URL_FIELD_BG: Color = Color {
        r: 40,
        g: 40,
        b: 56,
        a: 255,
    };
    /// URL text — Warm white.
    pub const URL_TEXT: Color = Color {
        r: 232,
        g: 224,
        b: 212,
        a: 255,
    };
    /// Navigation button — Stone.
    pub const NAV_BUTTON: Color = Color {
        r: 156,
        g: 148,
        b: 136,
        a: 255,
    };
    /// Cursor — σ Sequence (Electric Blue).
    pub const CURSOR: Color = Color {
        r: 79,
        g: 195,
        b: 247,
        a: 255,
    };

    /// Sidebar background — Dark carbon.
    pub const SIDEBAR_BG: Color = Color {
        r: 17,
        g: 17,
        b: 24,
        a: 255,
    };
    /// Sidebar item text — Muted warm.
    pub const SIDEBAR_TEXT: Color = Color {
        r: 176,
        g: 168,
        b: 156,
        a: 255,
    };
    /// Sidebar active item — σ Sequence (Electric Blue).
    pub const SIDEBAR_ACTIVE: Color = Color {
        r: 79,
        g: 195,
        b: 247,
        a: 255,
    };
    /// Sidebar separator — Subtle divider.
    pub const SIDEBAR_SEP: Color = Color {
        r: 42,
        g: 42,
        b: 58,
        a: 255,
    };

    /// Status bar background — Deep obsidian.
    pub const STATUS_BG: Color = Color {
        r: 13,
        g: 13,
        b: 20,
        a: 255,
    };
    /// Status bar text — Dim warm.
    pub const STATUS_TEXT: Color = Color {
        r: 140,
        g: 132,
        b: 120,
        a: 255,
    };
    /// GROUNDED indicator — ρ Recursion (Teal: the loop itself).
    pub const GROUNDED_INDICATOR: Color = Color {
        r: 77,
        g: 182,
        b: 172,
        a: 255,
    };
    /// Confidence trend up — N Quantity (Green: measurable).
    pub const CONFIDENCE_UP: Color = Color {
        r: 102,
        g: 187,
        b: 106,
        a: 255,
    };
    /// Confidence trend down — ∂ Boundary (Crimson: threshold breach).
    pub const CONFIDENCE_DOWN: Color = Color {
        r: 239,
        g: 83,
        b: 80,
        a: 255,
    };
    /// Learnings count — π Persistence (Gold: stored knowledge).
    pub const LEARNINGS_COLOR: Color = Color {
        r: 255,
        g: 213,
        b: 79,
        a: 255,
    };

    /// Default font size for chrome UI.
    pub const FONT_SIZE: f32 = 14.0;
    /// Small font size.
    pub const FONT_SIZE_SMALL: f32 = 11.0;
}

/// Tier: T2-C — Chrome layout calculator.
///
/// Computes the bounding rectangles for each chrome area
/// based on window dimensions and sidebar visibility.
#[derive(Debug, Clone, Copy)]
pub struct ChromeLayout {
    /// Tab bar area.
    pub tab_bar: Rect,
    /// Toolbar (nav buttons + URL bar) area.
    pub toolbar: Rect,
    /// Sidebar area (zero-width if hidden).
    pub sidebar: Rect,
    /// Content area.
    pub content: Rect,
    /// Status bar area.
    pub status_bar: Rect,
}

impl ChromeLayout {
    /// Compute layout from window dimensions and sidebar state.
    #[must_use]
    pub fn compute(width: f32, height: f32, sidebar_visible: bool) -> Self {
        let sidebar_w = if sidebar_visible {
            Theme::SIDEBAR_WIDTH
        } else {
            0.0
        };
        let tab_bar_h = Theme::TAB_BAR_HEIGHT;
        let toolbar_h = Theme::TOOLBAR_HEIGHT;
        let status_h = Theme::STATUS_BAR_HEIGHT;
        let content_y = tab_bar_h + toolbar_h;
        let content_h = (height - content_y - status_h).max(0.0);

        Self {
            tab_bar: Rect {
                x: 0.0,
                y: 0.0,
                width,
                height: tab_bar_h,
            },
            toolbar: Rect {
                x: 0.0,
                y: tab_bar_h,
                width,
                height: toolbar_h,
            },
            sidebar: Rect {
                x: 0.0,
                y: content_y,
                width: sidebar_w,
                height: content_h,
            },
            content: Rect {
                x: sidebar_w,
                y: content_y,
                width: (width - sidebar_w).max(0.0),
                height: content_h,
            },
            status_bar: Rect {
                x: 0.0,
                y: height - status_h,
                width,
                height: status_h,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_with_sidebar() {
        let layout = ChromeLayout::compute(1280.0, 720.0, true);
        assert!((layout.tab_bar.height - 36.0).abs() < f32::EPSILON);
        assert!((layout.toolbar.height - 40.0).abs() < f32::EPSILON);
        assert!((layout.sidebar.width - 280.0).abs() < f32::EPSILON);
        assert!((layout.content.x - 280.0).abs() < f32::EPSILON);
        assert!((layout.status_bar.height - 24.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_layout_without_sidebar() {
        let layout = ChromeLayout::compute(1280.0, 720.0, false);
        assert!(layout.sidebar.width.abs() < f32::EPSILON);
        assert!(layout.content.x.abs() < f32::EPSILON);
        assert!((layout.content.width - 1280.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_layout_content_fills_remaining() {
        let layout = ChromeLayout::compute(1280.0, 720.0, true);
        let total = layout.tab_bar.height
            + layout.toolbar.height
            + layout.content.height
            + layout.status_bar.height;
        assert!((total - 720.0).abs() < f32::EPSILON);
    }
}
