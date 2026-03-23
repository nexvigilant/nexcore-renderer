//! Status bar widget — GROUNDED status, confidence, learning count.
//!
//! ## Tier Classification
//!
//! - `StatusBar`: T3 (domain widget)

use super::{Theme, Widget};
use crate::layout::Rect;
use crate::paint::DisplayCommand;
use crate::state::{Message, WidgetId};

/// Tier: T3 — Status bar widget showing GROUNDED loop state.
pub struct StatusBar {
    /// Bounding rect.
    rect: Rect,
    /// Current GROUNDED cycle count.
    cycle: u64,
    /// Current confidence (0.0 to 1.0).
    confidence: f64,
    /// Whether confidence is trending up.
    confidence_up: bool,
    /// Total learning count.
    learning_count: usize,
}

impl StatusBar {
    /// Create a new status bar.
    #[must_use]
    pub fn new() -> Self {
        Self {
            rect: Rect::default(),
            cycle: 0,
            confidence: 0.5,
            confidence_up: true,
            learning_count: 0,
        }
    }

    /// Update GROUNDED status.
    pub fn set_grounded_status(
        &mut self,
        cycle: u64,
        confidence: f64,
        confidence_up: bool,
        learning_count: usize,
    ) {
        self.cycle = cycle;
        self.confidence = confidence;
        self.confidence_up = confidence_up;
        self.learning_count = learning_count;
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

/// Format the GROUNDED status text with ρ primitive symbol.
fn format_status(cycle: u64) -> String {
    format!("ρ GROUNDED: Cycle {cycle}")
}

/// Format the confidence text with primitive-semantic arrow.
fn format_confidence(confidence: f64, up: bool) -> String {
    let (symbol, arrow) = if up { ("N", "↑") } else { ("∂", "↓") };
    format!("{symbol} Confidence: {:.2}{arrow}", confidence)
}

/// Format the learning count with π persistence symbol.
fn format_learnings(count: usize) -> String {
    format!("π Learnings: {count}")
}

impl Widget for StatusBar {
    fn id(&self) -> WidgetId {
        WidgetId(103)
    }

    fn layout(&mut self, available: Rect) -> Rect {
        self.rect = Rect {
            x: available.x,
            y: available.y,
            width: available.width,
            height: Theme::STATUS_BAR_HEIGHT,
        };
        self.rect
    }

    fn paint(&self) -> Vec<DisplayCommand> {
        let mut cmds = Vec::with_capacity(4);

        // Background
        cmds.push(DisplayCommand::FillRect {
            rect: self.rect,
            color: Theme::STATUS_BG,
            node_id: None,
        });

        let text_y = self.rect.y + 16.0;

        // GROUNDED indicator
        cmds.push(DisplayCommand::DrawText {
            text: format_status(self.cycle),
            x: self.rect.x + 12.0,
            y: text_y,
            size: Theme::FONT_SIZE_SMALL,
            color: Theme::GROUNDED_INDICATOR,
            node_id: None,
        });

        // Confidence
        let conf_color = if self.confidence_up {
            Theme::CONFIDENCE_UP
        } else {
            Theme::CONFIDENCE_DOWN
        };
        cmds.push(DisplayCommand::DrawText {
            text: format_confidence(self.confidence, self.confidence_up),
            x: self.rect.x + 240.0,
            y: text_y,
            size: Theme::FONT_SIZE_SMALL,
            color: conf_color,
            node_id: None,
        });

        // Learning count — π Persistence (Gold)
        cmds.push(DisplayCommand::DrawText {
            text: format_learnings(self.learning_count),
            x: self.rect.x + 440.0,
            y: text_y,
            size: Theme::FONT_SIZE_SMALL,
            color: Theme::LEARNINGS_COLOR,
            node_id: None,
        });

        cmds
    }

    fn handle_click(&mut self, _x: f32, _y: f32) -> Option<Message> {
        None
    }

    fn hit_test(&self, x: f32, y: f32) -> bool {
        self.rect.contains(x, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_bar_paint() {
        let bar = StatusBar::new();
        let cmds = bar.paint();
        // bg + 3 text items
        assert_eq!(cmds.len(), 4);
    }

    #[test]
    fn test_format_status() {
        assert_eq!(format_status(42), "ρ GROUNDED: Cycle 42");
    }

    #[test]
    fn test_format_confidence() {
        let up = format_confidence(0.73, true);
        assert!(up.contains("↑"));
        let down = format_confidence(0.45, false);
        assert!(down.contains("↓"));
    }
}
