//! Scroll state and viewport management.
//!
//! Tracks scroll position, content dimensions, and viewport size.
//! Provides clamped scrolling, scroll fraction for scrollbar indicators,
//! and display list transformation for paint-time offset.
//!
//! ## Tier Classification
//!
//! - `ScrollState`: T2-P (State + Quantity)
//! - `scroll_fraction`: T2-P (Mapping)
//! - `apply_scroll_transform`: T2-C (Sequence + State + Mapping)

use crate::layout::Rect;
use crate::paint::DisplayCommand;
use crate::style::Color;

/// Default line height in pixels for line-based scrolling.
const LINE_HEIGHT: f32 = 20.0;

/// Scrollbar visual width in pixels.
const SCROLLBAR_WIDTH: f32 = 8.0;

/// Minimum scrollbar thumb height in pixels.
const SCROLLBAR_MIN_THUMB: f32 = 20.0;

/// Scrollbar track color (semi-transparent dark).
const SCROLLBAR_TRACK_COLOR: Color = Color {
    r: 40,
    g: 40,
    b: 60,
    a: 100,
};

/// Scrollbar thumb color (semi-transparent light).
const SCROLLBAR_THUMB_COLOR: Color = Color {
    r: 160,
    g: 160,
    b: 200,
    a: 160,
};

/// Scroll position and viewport state.
///
/// Tier: T2-P (ς State + N Quantity)
/// Grounding: encapsulated_context + measured_values
#[derive(Debug, Clone)]
pub struct ScrollState {
    /// Current horizontal scroll offset in pixels.
    pub scroll_x: f32,
    /// Current vertical scroll offset in pixels.
    pub scroll_y: f32,
    /// Total content width in pixels.
    pub content_width: f32,
    /// Total content height in pixels.
    pub content_height: f32,
    /// Viewport width in pixels.
    pub viewport_width: f32,
    /// Viewport height in pixels.
    pub viewport_height: f32,
}

impl Default for ScrollState {
    fn default() -> Self {
        Self::new()
    }
}

impl ScrollState {
    /// Create a new scroll state with default dimensions.
    #[must_use]
    pub fn new() -> Self {
        Self {
            scroll_x: 0.0,
            scroll_y: 0.0,
            content_width: 0.0,
            content_height: 0.0,
            viewport_width: 1280.0,
            viewport_height: 720.0,
        }
    }

    /// Scroll by a relative delta, clamping to valid bounds.
    pub fn scroll_by(&mut self, dx: f32, dy: f32) {
        self.scroll_x = (self.scroll_x + dx).clamp(0.0, self.max_scroll_x());
        self.scroll_y = (self.scroll_y + dy).clamp(0.0, self.max_scroll_y());
    }

    /// Scroll to an absolute position, clamping to valid bounds.
    pub fn scroll_to(&mut self, x: f32, y: f32) {
        self.scroll_x = x.clamp(0.0, self.max_scroll_x());
        self.scroll_y = y.clamp(0.0, self.max_scroll_y());
    }

    /// Maximum horizontal scroll offset.
    #[must_use]
    pub fn max_scroll_x(&self) -> f32 {
        (self.content_width - self.viewport_width).max(0.0)
    }

    /// Maximum vertical scroll offset.
    #[must_use]
    pub fn max_scroll_y(&self) -> f32 {
        (self.content_height - self.viewport_height).max(0.0)
    }

    /// Vertical scroll fraction (0.0 = top, 1.0 = bottom).
    ///
    /// Returns 0.0 when content fits within viewport (no scrolling needed).
    #[must_use]
    pub fn scroll_fraction_y(&self) -> f32 {
        let max = self.max_scroll_y();
        if max <= 0.0 { 0.0 } else { self.scroll_y / max }
    }

    /// Horizontal scroll fraction (0.0 = left, 1.0 = right).
    ///
    /// Returns 0.0 when content fits within viewport.
    #[must_use]
    pub fn scroll_fraction_x(&self) -> f32 {
        let max = self.max_scroll_x();
        if max <= 0.0 { 0.0 } else { self.scroll_x / max }
    }

    /// Whether the content overflows the viewport vertically.
    #[must_use]
    pub fn can_scroll_y(&self) -> bool {
        self.content_height > self.viewport_height
    }

    /// Whether the content overflows the viewport horizontally.
    #[must_use]
    pub fn can_scroll_x(&self) -> bool {
        self.content_width > self.viewport_width
    }

    /// Default line height for line-based scrolling.
    #[must_use]
    pub fn line_height() -> f32 {
        LINE_HEIGHT
    }

    /// Update viewport dimensions.
    pub fn set_viewport(&mut self, width: f32, height: f32) {
        self.viewport_width = width;
        self.viewport_height = height;
        // Re-clamp scroll position after viewport change
        self.scroll_x = self.scroll_x.clamp(0.0, self.max_scroll_x());
        self.scroll_y = self.scroll_y.clamp(0.0, self.max_scroll_y());
    }

    /// Update content dimensions (typically after layout).
    pub fn set_content_size(&mut self, width: f32, height: f32) {
        self.content_width = width;
        self.content_height = height;
        // Re-clamp scroll position after content size change
        self.scroll_x = self.scroll_x.clamp(0.0, self.max_scroll_x());
        self.scroll_y = self.scroll_y.clamp(0.0, self.max_scroll_y());
    }

    /// Reset scroll position to origin.
    pub fn reset(&mut self) {
        self.scroll_x = 0.0;
        self.scroll_y = 0.0;
    }

    /// Scroll down by one line.
    pub fn scroll_line_down(&mut self) {
        self.scroll_by(0.0, LINE_HEIGHT);
    }

    /// Scroll up by one line.
    pub fn scroll_line_up(&mut self) {
        self.scroll_by(0.0, -LINE_HEIGHT);
    }

    /// Scroll down by one page (viewport height).
    pub fn page_down(&mut self) {
        self.scroll_by(0.0, self.viewport_height);
    }

    /// Scroll up by one page (viewport height).
    pub fn page_up(&mut self) {
        self.scroll_by(0.0, -self.viewport_height);
    }

    /// Scroll to the top of the content.
    pub fn scroll_to_top(&mut self) {
        self.scroll_to(self.scroll_x, 0.0);
    }

    /// Scroll to the bottom of the content.
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_to(self.scroll_x, self.max_scroll_y());
    }
}

// ── Paint-time scroll transform ─────────────────────────────────

/// Apply scroll offset to a display list, producing a new list with
/// transformed coordinates and viewport clipping.
///
/// Commands whose bounding rects fall entirely outside the viewport
/// after scrolling are excluded. Remaining commands have their
/// y-coordinates offset by `-scroll_y` and x-coordinates by `-scroll_x`.
///
/// Tier: T2-C (σ Sequence + ς State + μ Mapping)
#[must_use]
pub fn apply_scroll_transform(
    commands: &[DisplayCommand],
    scroll: &ScrollState,
) -> Vec<DisplayCommand> {
    let sx = scroll.scroll_x;
    let sy = scroll.scroll_y;
    let vw = scroll.viewport_width;
    let vh = scroll.viewport_height;

    commands
        .iter()
        .filter_map(|cmd| {
            // Check visibility via hit_rect
            if let Some(rect) = cmd.hit_rect() {
                let shifted_y = rect.y - sy;
                let shifted_x = rect.x - sx;
                // Cull if entirely outside viewport
                if shifted_y + rect.height < 0.0
                    || shifted_y > vh
                    || shifted_x + rect.width < 0.0
                    || shifted_x > vw
                {
                    return None;
                }
            }
            Some(offset_command(cmd, sx, sy))
        })
        .collect()
}

/// Offset a single display command by scroll deltas.
fn offset_command(cmd: &DisplayCommand, sx: f32, sy: f32) -> DisplayCommand {
    match cmd {
        DisplayCommand::FillRect {
            rect,
            color,
            node_id,
        } => DisplayCommand::FillRect {
            rect: offset_rect(rect, sx, sy),
            color: *color,
            node_id: *node_id,
        },
        DisplayCommand::FillCircle {
            center,
            radius,
            color,
            node_id,
        } => DisplayCommand::FillCircle {
            center: crate::paint::Point::new(center.x - sx, center.y - sy),
            radius: *radius,
            color: *color,
            node_id: *node_id,
        },
        DisplayCommand::FillTriangle {
            p1,
            p2,
            p3,
            color,
            node_id,
        } => DisplayCommand::FillTriangle {
            p1: crate::paint::Point::new(p1.x - sx, p1.y - sy),
            p2: crate::paint::Point::new(p2.x - sx, p2.y - sy),
            p3: crate::paint::Point::new(p3.x - sx, p3.y - sy),
            color: *color,
            node_id: *node_id,
        },
        DisplayCommand::StrokeLine {
            start,
            end,
            width,
            color,
            node_id,
        } => DisplayCommand::StrokeLine {
            start: crate::paint::Point::new(start.x - sx, start.y - sy),
            end: crate::paint::Point::new(end.x - sx, end.y - sy),
            width: *width,
            color: *color,
            node_id: *node_id,
        },
        DisplayCommand::DrawText {
            text,
            x,
            y,
            size,
            color,
            node_id,
        } => DisplayCommand::DrawText {
            text: text.clone(),
            x: x - sx,
            y: y - sy,
            size: *size,
            color: *color,
            node_id: *node_id,
        },
        DisplayCommand::DrawImage { src, rect, node_id } => DisplayCommand::DrawImage {
            src: src.clone(),
            rect: offset_rect(rect, sx, sy),
            node_id: *node_id,
        },
        DisplayCommand::BlitRgba {
            rect,
            width,
            height,
            data,
        } => DisplayCommand::BlitRgba {
            rect: offset_rect(rect, sx, sy),
            width: *width,
            height: *height,
            data: data.clone(),
        },
    }
}

/// Offset a rectangle by scroll amounts.
fn offset_rect(rect: &Rect, sx: f32, sy: f32) -> Rect {
    Rect {
        x: rect.x - sx,
        y: rect.y - sy,
        width: rect.width,
        height: rect.height,
    }
}

// ── Scrollbar indicator ─────────────────────────────────────────

/// Build display commands for a vertical scrollbar indicator.
///
/// Renders a thin bar on the right edge of the viewport showing
/// the current scroll position. Height is proportional to
/// viewport/content ratio; position reflects scroll fraction.
///
/// Returns empty vec if content fits within viewport.
///
/// Tier: T2-P (N Quantity + λ Location)
#[must_use]
pub fn build_scrollbar_commands(
    scroll: &ScrollState,
    viewport_x_offset: f32,
) -> Vec<DisplayCommand> {
    if !scroll.can_scroll_y() {
        return Vec::new();
    }

    let mut cmds = Vec::with_capacity(2);

    let track_x = viewport_x_offset + scroll.viewport_width - SCROLLBAR_WIDTH;
    let track_height = scroll.viewport_height;

    // Track background
    cmds.push(DisplayCommand::FillRect {
        rect: Rect {
            x: track_x,
            y: 0.0,
            width: SCROLLBAR_WIDTH,
            height: track_height,
        },
        color: SCROLLBAR_TRACK_COLOR,
        node_id: None,
    });

    // Thumb
    let ratio = scroll.viewport_height / scroll.content_height;
    let thumb_height = (track_height * ratio).max(SCROLLBAR_MIN_THUMB);
    let available = track_height - thumb_height;
    let thumb_y = available * scroll.scroll_fraction_y();

    cmds.push(DisplayCommand::FillRect {
        rect: Rect {
            x: track_x,
            y: thumb_y,
            width: SCROLLBAR_WIDTH,
            height: thumb_height,
        },
        color: SCROLLBAR_THUMB_COLOR,
        node_id: None,
    });

    cmds
}

// ── Unit Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_scroll(content_h: f32, viewport_h: f32) -> ScrollState {
        ScrollState {
            scroll_x: 0.0,
            scroll_y: 0.0,
            content_width: 800.0,
            content_height: content_h,
            viewport_width: 800.0,
            viewport_height: viewport_h,
        }
    }

    // ── ScrollState basic operations ────────────────────────────

    #[test]
    fn test_new_defaults() {
        let s = ScrollState::new();
        assert!((s.scroll_x).abs() < f32::EPSILON);
        assert!((s.scroll_y).abs() < f32::EPSILON);
    }

    #[test]
    fn test_scroll_by_positive() {
        let mut s = make_scroll(2000.0, 600.0);
        s.scroll_by(0.0, 100.0);
        assert!((s.scroll_y - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_scroll_by_negative_clamps_to_zero() {
        let mut s = make_scroll(2000.0, 600.0);
        s.scroll_by(0.0, -100.0);
        assert!((s.scroll_y).abs() < f32::EPSILON);
    }

    #[test]
    fn test_scroll_by_clamps_to_max() {
        let mut s = make_scroll(2000.0, 600.0);
        s.scroll_by(0.0, 99999.0);
        assert!((s.scroll_y - 1400.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_scroll_to_absolute() {
        let mut s = make_scroll(2000.0, 600.0);
        s.scroll_to(0.0, 500.0);
        assert!((s.scroll_y - 500.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_scroll_to_clamps_negative() {
        let mut s = make_scroll(2000.0, 600.0);
        s.scroll_to(0.0, -50.0);
        assert!((s.scroll_y).abs() < f32::EPSILON);
    }

    #[test]
    fn test_scroll_to_clamps_max() {
        let mut s = make_scroll(2000.0, 600.0);
        s.scroll_to(0.0, 5000.0);
        assert!((s.scroll_y - 1400.0).abs() < f32::EPSILON);
    }

    // ── Max scroll ──────────────────────────────────────────────

    #[test]
    fn test_max_scroll_y_when_content_larger() {
        let s = make_scroll(2000.0, 600.0);
        assert!((s.max_scroll_y() - 1400.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_max_scroll_y_when_content_fits() {
        let s = make_scroll(400.0, 600.0);
        assert!((s.max_scroll_y()).abs() < f32::EPSILON);
    }

    #[test]
    fn test_max_scroll_y_exact_fit() {
        let s = make_scroll(600.0, 600.0);
        assert!((s.max_scroll_y()).abs() < f32::EPSILON);
    }

    #[test]
    fn test_max_scroll_x() {
        let s = ScrollState {
            content_width: 1600.0,
            viewport_width: 800.0,
            ..ScrollState::new()
        };
        assert!((s.max_scroll_x() - 800.0).abs() < f32::EPSILON);
    }

    // ── Scroll fraction ─────────────────────────────────────────

    #[test]
    fn test_scroll_fraction_at_top() {
        let s = make_scroll(2000.0, 600.0);
        assert!((s.scroll_fraction_y()).abs() < f32::EPSILON);
    }

    #[test]
    fn test_scroll_fraction_at_bottom() {
        let mut s = make_scroll(2000.0, 600.0);
        s.scroll_to(0.0, s.max_scroll_y());
        assert!((s.scroll_fraction_y() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_scroll_fraction_midway() {
        let mut s = make_scroll(2000.0, 600.0);
        s.scroll_to(0.0, 700.0);
        assert!((s.scroll_fraction_y() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_scroll_fraction_when_no_overflow() {
        let s = make_scroll(400.0, 600.0);
        assert!((s.scroll_fraction_y()).abs() < f32::EPSILON);
    }

    #[test]
    fn test_scroll_fraction_x_when_no_overflow() {
        let s = ScrollState::new();
        assert!((s.scroll_fraction_x()).abs() < f32::EPSILON);
    }

    // ── Can scroll ──────────────────────────────────────────────

    #[test]
    fn test_can_scroll_y_true() {
        let s = make_scroll(2000.0, 600.0);
        assert!(s.can_scroll_y());
    }

    #[test]
    fn test_can_scroll_y_false() {
        let s = make_scroll(400.0, 600.0);
        assert!(!s.can_scroll_y());
    }

    #[test]
    fn test_can_scroll_x() {
        let s = ScrollState {
            content_width: 1600.0,
            viewport_width: 800.0,
            ..ScrollState::new()
        };
        assert!(s.can_scroll_x());
    }

    // ── Navigation helpers ──────────────────────────────────────

    #[test]
    fn test_page_down() {
        let mut s = make_scroll(3000.0, 600.0);
        s.page_down();
        assert!((s.scroll_y - 600.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_page_up() {
        let mut s = make_scroll(3000.0, 600.0);
        s.scroll_to(0.0, 1000.0);
        s.page_up();
        assert!((s.scroll_y - 400.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_page_up_clamps_to_zero() {
        let mut s = make_scroll(3000.0, 600.0);
        s.scroll_to(0.0, 200.0);
        s.page_up();
        assert!((s.scroll_y).abs() < f32::EPSILON);
    }

    #[test]
    fn test_scroll_to_top() {
        let mut s = make_scroll(2000.0, 600.0);
        s.scroll_to(0.0, 500.0);
        s.scroll_to_top();
        assert!((s.scroll_y).abs() < f32::EPSILON);
    }

    #[test]
    fn test_scroll_to_bottom() {
        let mut s = make_scroll(2000.0, 600.0);
        s.scroll_to_bottom();
        assert!((s.scroll_y - 1400.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_scroll_line_down() {
        let mut s = make_scroll(2000.0, 600.0);
        s.scroll_line_down();
        assert!((s.scroll_y - LINE_HEIGHT).abs() < f32::EPSILON);
    }

    #[test]
    fn test_scroll_line_up() {
        let mut s = make_scroll(2000.0, 600.0);
        s.scroll_to(0.0, 100.0);
        s.scroll_line_up();
        assert!((s.scroll_y - (100.0 - LINE_HEIGHT)).abs() < f32::EPSILON);
    }

    #[test]
    fn test_reset() {
        let mut s = make_scroll(2000.0, 600.0);
        s.scroll_to(50.0, 500.0);
        s.reset();
        assert!((s.scroll_x).abs() < f32::EPSILON);
        assert!((s.scroll_y).abs() < f32::EPSILON);
    }

    // ── Viewport/content size updates ───────────────────────────

    #[test]
    fn test_set_viewport_reclamps() {
        let mut s = make_scroll(2000.0, 600.0);
        s.scroll_to(0.0, 1400.0); // At max
        s.set_viewport(800.0, 1000.0); // Bigger viewport
        // New max = 2000 - 1000 = 1000, scroll should be clamped to 1000
        assert!((s.scroll_y - 1000.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_set_content_size_reclamps() {
        let mut s = make_scroll(2000.0, 600.0);
        s.scroll_to(0.0, 1400.0);
        s.set_content_size(800.0, 800.0); // Smaller content
        // New max = 800 - 600 = 200
        assert!((s.scroll_y - 200.0).abs() < f32::EPSILON);
    }

    // ── Scroll transform ────────────────────────────────────────

    #[test]
    fn test_transform_offsets_y() {
        let mut s = make_scroll(2000.0, 600.0);
        s.scroll_to(0.0, 100.0);

        let commands = vec![DisplayCommand::FillRect {
            rect: Rect {
                x: 0.0,
                y: 200.0,
                width: 100.0,
                height: 50.0,
            },
            color: Color::WHITE,
            node_id: Some(0),
        }];

        let transformed = apply_scroll_transform(&commands, &s);
        assert_eq!(transformed.len(), 1);

        if let DisplayCommand::FillRect { rect, .. } = &transformed[0] {
            assert!((rect.y - 100.0).abs() < f32::EPSILON); // 200 - 100
        } else {
            panic!("Expected FillRect");
        }
    }

    #[test]
    fn test_transform_culls_above_viewport() {
        let mut s = make_scroll(2000.0, 600.0);
        s.scroll_to(0.0, 500.0);

        // This rect is at y=0..50, after scroll offset it becomes -500..-450
        let commands = vec![DisplayCommand::FillRect {
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 50.0,
            },
            color: Color::WHITE,
            node_id: Some(0),
        }];

        let transformed = apply_scroll_transform(&commands, &s);
        assert!(transformed.is_empty());
    }

    #[test]
    fn test_transform_culls_below_viewport() {
        let s = make_scroll(2000.0, 600.0);

        // This rect is at y=700, which is below viewport (0..600)
        let commands = vec![DisplayCommand::FillRect {
            rect: Rect {
                x: 0.0,
                y: 700.0,
                width: 100.0,
                height: 50.0,
            },
            color: Color::WHITE,
            node_id: Some(0),
        }];

        let transformed = apply_scroll_transform(&commands, &s);
        assert!(transformed.is_empty());
    }

    #[test]
    fn test_transform_keeps_partially_visible() {
        let mut s = make_scroll(2000.0, 600.0);
        s.scroll_to(0.0, 100.0);

        // Rect at y=80..130, after scroll becomes -20..30 -- partially visible
        let commands = vec![DisplayCommand::FillRect {
            rect: Rect {
                x: 0.0,
                y: 80.0,
                width: 100.0,
                height: 50.0,
            },
            color: Color::WHITE,
            node_id: Some(0),
        }];

        let transformed = apply_scroll_transform(&commands, &s);
        assert_eq!(transformed.len(), 1);
    }

    #[test]
    fn test_transform_text_command() {
        let mut s = make_scroll(2000.0, 600.0);
        s.scroll_to(0.0, 50.0);

        let commands = vec![DisplayCommand::DrawText {
            text: "hello".to_string(),
            x: 10.0,
            y: 100.0,
            size: 16.0,
            color: Color::BLACK,
            node_id: Some(1),
        }];

        let transformed = apply_scroll_transform(&commands, &s);
        assert_eq!(transformed.len(), 1);

        if let DisplayCommand::DrawText { x, y, .. } = &transformed[0] {
            assert!((x - 10.0).abs() < f32::EPSILON);
            assert!((y - 50.0).abs() < f32::EPSILON); // 100 - 50
        } else {
            panic!("Expected DrawText");
        }
    }

    #[test]
    fn test_transform_empty_list() {
        let s = make_scroll(2000.0, 600.0);
        let transformed = apply_scroll_transform(&[], &s);
        assert!(transformed.is_empty());
    }

    // ── Scrollbar indicator ─────────────────────────────────────

    #[test]
    fn test_scrollbar_hidden_when_no_overflow() {
        let s = make_scroll(400.0, 600.0);
        let cmds = build_scrollbar_commands(&s, 0.0);
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_scrollbar_shown_when_overflow() {
        let s = make_scroll(2000.0, 600.0);
        let cmds = build_scrollbar_commands(&s, 0.0);
        assert_eq!(cmds.len(), 2); // track + thumb
    }

    #[test]
    fn test_scrollbar_thumb_at_top() {
        let s = make_scroll(2000.0, 600.0);
        let cmds = build_scrollbar_commands(&s, 0.0);
        // Thumb should be at y=0 when scroll is at top
        if let DisplayCommand::FillRect { rect, .. } = &cmds[1] {
            assert!((rect.y).abs() < f32::EPSILON);
        } else {
            panic!("Expected FillRect for thumb");
        }
    }

    #[test]
    fn test_scrollbar_thumb_at_bottom() {
        let mut s = make_scroll(2000.0, 600.0);
        s.scroll_to_bottom();
        let cmds = build_scrollbar_commands(&s, 0.0);
        // Thumb should be at bottom
        if let DisplayCommand::FillRect { rect, .. } = &cmds[1] {
            let ratio: f32 = 600.0 / 2000.0;
            let thumb_h: f32 = (600.0_f32 * ratio).max(SCROLLBAR_MIN_THUMB);
            let expected_y: f32 = 600.0 - thumb_h;
            assert!((rect.y - expected_y).abs() < 1.0);
        } else {
            panic!("Expected FillRect for thumb");
        }
    }

    #[test]
    fn test_scrollbar_with_x_offset() {
        let s = make_scroll(2000.0, 600.0);
        let cmds = build_scrollbar_commands(&s, 280.0);
        // Track should be at x = 280 + 800 - 8 = 1072
        if let DisplayCommand::FillRect { rect, .. } = &cmds[0] {
            assert!((rect.x - 1072.0).abs() < f32::EPSILON);
        } else {
            panic!("Expected FillRect for track");
        }
    }

    #[test]
    fn test_scrollbar_thumb_min_height() {
        // Very tall content -> tiny ratio -> thumb clamped to min
        let s = make_scroll(100_000.0, 600.0);
        let cmds = build_scrollbar_commands(&s, 0.0);
        if let DisplayCommand::FillRect { rect, .. } = &cmds[1] {
            assert!(rect.height >= SCROLLBAR_MIN_THUMB);
        } else {
            panic!("Expected FillRect for thumb");
        }
    }

    // ── Horizontal scroll ───────────────────────────────────────

    #[test]
    fn test_horizontal_scroll_by() {
        let mut s = ScrollState {
            content_width: 1600.0,
            viewport_width: 800.0,
            ..make_scroll(2000.0, 600.0)
        };
        s.scroll_by(100.0, 0.0);
        assert!((s.scroll_x - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_horizontal_scroll_clamps() {
        let mut s = ScrollState {
            content_width: 1600.0,
            viewport_width: 800.0,
            ..make_scroll(2000.0, 600.0)
        };
        s.scroll_by(9999.0, 0.0);
        assert!((s.scroll_x - 800.0).abs() < f32::EPSILON);
    }
}
