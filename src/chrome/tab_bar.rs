//! Tab bar widget — tab strip with GROUNDED cycle counter.
//!
//! ## Tier Classification
//!
//! - `TabBar`: T3 (domain widget)
//! - `TabInfo`: T2-C (display data)

use super::{Theme, Widget};
use crate::layout::Rect;
use crate::paint::DisplayCommand;
use crate::state::{Message, TabId, WidgetId};

/// Tab width in pixels.
const TAB_WIDTH: f32 = 160.0;
/// Padding between tabs.
const TAB_PAD: f32 = 2.0;

/// Tier: T2-C — Display data for a single tab.
#[derive(Debug, Clone)]
pub struct TabInfo {
    /// Tab identifier.
    pub id: TabId,
    /// Tab title.
    pub title: String,
    /// Whether this tab is active.
    pub active: bool,
}

/// Tier: T3 — Tab bar widget.
pub struct TabBar {
    /// Bounding rect.
    rect: Rect,
    /// Tab display data.
    tabs: Vec<TabInfo>,
    /// GROUNDED cycle counter.
    grounded_cycle: u64,
}

impl TabBar {
    /// Create a new tab bar.
    #[must_use]
    pub fn new() -> Self {
        Self {
            rect: Rect::default(),
            tabs: Vec::new(),
            grounded_cycle: 0,
        }
    }

    /// Update tab data for rendering.
    pub fn set_tabs(&mut self, tabs: Vec<TabInfo>) {
        self.tabs = tabs;
    }

    /// Update the GROUNDED cycle counter.
    pub fn set_grounded_cycle(&mut self, cycle: u64) {
        self.grounded_cycle = cycle;
    }
}

impl Default for TabBar {
    fn default() -> Self {
        Self::new()
    }
}

/// Paint a single tab and its title, with primitive accent on active tab.
fn paint_tab(tab: &TabInfo, index: usize, bar_rect: &Rect) -> Vec<DisplayCommand> {
    let x = bar_rect.x + (index as f32) * (TAB_WIDTH + TAB_PAD) + TAB_PAD;
    let color = if tab.active {
        Theme::TAB_ACTIVE
    } else {
        Theme::TAB_INACTIVE
    };
    let tab_rect = Rect {
        x,
        y: bar_rect.y + 4.0,
        width: TAB_WIDTH,
        height: bar_rect.height - 4.0,
    };
    let title = truncate_title(&tab.title, 18);

    let mut cmds = vec![
        DisplayCommand::FillRect {
            rect: tab_rect,
            color,
            node_id: None,
        },
        DisplayCommand::DrawText {
            text: title,
            x: x + 8.0,
            y: bar_rect.y + 22.0,
            size: Theme::FONT_SIZE_SMALL,
            color: Theme::TAB_TEXT,
            node_id: None,
        },
    ];

    // Active tab gets a σ Sequence accent line at the bottom
    if tab.active {
        cmds.push(DisplayCommand::FillRect {
            rect: Rect {
                x,
                y: bar_rect.y + bar_rect.height - 2.0,
                width: TAB_WIDTH,
                height: 2.0,
            },
            color: Theme::PRIMITIVE_SEQUENCE,
            node_id: None,
        });
    }

    cmds
}

/// Paint the GROUNDED cycle counter (right-aligned).
fn paint_cycle_counter(cycle: u64, bar_rect: &Rect) -> DisplayCommand {
    DisplayCommand::DrawText {
        text: format!("GROUNDED: Cycle {cycle}"),
        x: bar_rect.x + bar_rect.width - 200.0,
        y: bar_rect.y + 22.0,
        size: Theme::FONT_SIZE_SMALL,
        color: Theme::GROUNDED_INDICATOR,
        node_id: None,
    }
}

impl Widget for TabBar {
    fn id(&self) -> WidgetId {
        WidgetId(100)
    }

    fn layout(&mut self, available: Rect) -> Rect {
        self.rect = Rect {
            x: available.x,
            y: available.y,
            width: available.width,
            height: Theme::TAB_BAR_HEIGHT,
        };
        self.rect
    }

    fn paint(&self) -> Vec<DisplayCommand> {
        let mut cmds = Vec::with_capacity(self.tabs.len() * 2 + 2);

        // Background
        cmds.push(DisplayCommand::FillRect {
            rect: self.rect,
            color: Theme::TAB_BAR_BG,
            node_id: None,
        });

        // Tabs
        for (i, tab) in self.tabs.iter().enumerate() {
            cmds.extend(paint_tab(tab, i, &self.rect));
        }

        // Cycle counter
        cmds.push(paint_cycle_counter(self.grounded_cycle, &self.rect));
        cmds
    }

    fn handle_click(&mut self, x: f32, _y: f32) -> Option<Message> {
        for (i, tab) in self.tabs.iter().enumerate() {
            let tab_x = self.rect.x + (i as f32) * (TAB_WIDTH + TAB_PAD) + TAB_PAD;
            if x >= tab_x && x < tab_x + TAB_WIDTH {
                return Some(Message::SelectTab(tab.id));
            }
        }
        None
    }

    fn hit_test(&self, x: f32, y: f32) -> bool {
        self.rect.contains(x, y)
    }
}

/// Truncate a string to max_chars, appending "..." if truncated.
fn truncate_title(s: &str, max_chars: usize) -> String {
    if s.len() <= max_chars {
        return s.to_string();
    }
    let end = s
        .char_indices()
        .nth(max_chars - 1)
        .map_or(s.len(), |(i, _)| i);
    format!("{}...", &s[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tab_bar_layout() {
        let mut bar = TabBar::new();
        let available = Rect {
            x: 0.0,
            y: 0.0,
            width: 1280.0,
            height: 36.0,
        };
        let rect = bar.layout(available);
        assert!((rect.height - 36.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_tab_bar_paint_empty() {
        let bar = TabBar::new();
        let cmds = bar.paint();
        // Background + cycle counter
        assert!(cmds.len() >= 2);
    }

    #[test]
    fn test_truncate_title() {
        assert_eq!(truncate_title("short", 18), "short");
        let long = "a very long title that exceeds";
        let truncated = truncate_title(long, 10);
        assert!(truncated.ends_with("..."));
    }
}
