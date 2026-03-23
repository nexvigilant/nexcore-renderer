//! Sidebar widget — collapsible panel container.
//!
//! ## Tier Classification
//!
//! - `Sidebar`: T3 (domain widget)
//! - `SidebarItem`: T2-C (display data)

use super::{Theme, Widget};
use crate::layout::Rect;
use crate::paint::DisplayCommand;
use crate::state::{Message, PanelId, WidgetId};
use crate::style::Color;

/// Height of each sidebar item.
const ITEM_HEIGHT: f32 = 32.0;
/// Left padding for item text.
const ITEM_PAD: f32 = 16.0;

/// Tier: T2-C — A sidebar menu item grounded to a Lex Primitiva color.
#[derive(Debug, Clone)]
pub struct SidebarItem {
    /// Panel ID this item opens.
    pub panel: PanelId,
    /// Display label.
    pub label: String,
    /// Icon character (single Unicode char).
    pub icon: char,
    /// Primitive color for the icon (from Lex Primitiva).
    pub color: Color,
}

/// Tier: T3 — Sidebar widget.
pub struct Sidebar {
    /// Bounding rect.
    rect: Rect,
    /// Menu items.
    items: Vec<SidebarItem>,
    /// Currently active panel.
    active: PanelId,
    /// Whether sidebar is visible.
    visible: bool,
}

impl Sidebar {
    /// Create a new sidebar with default items.
    #[must_use]
    pub fn new() -> Self {
        Self {
            rect: Rect::default(),
            items: default_items(),
            active: PanelId::GROUNDED,
            visible: true,
        }
    }

    /// Set the active panel.
    pub fn set_active(&mut self, panel: PanelId) {
        self.active = panel;
    }

    /// Set visibility.
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }
}

impl Default for Sidebar {
    fn default() -> Self {
        Self::new()
    }
}

/// Default sidebar items — each grounded to a dominant Lex Primitiva color.
fn default_items() -> Vec<SidebarItem> {
    vec![
        SidebarItem {
            panel: PanelId::GROUNDED,
            label: "GROUNDED Loop".into(),
            icon: '◉',
            color: Theme::PRIMITIVE_RECURSION, // ρ — the loop itself
        },
        SidebarItem {
            panel: PanelId::HYPOTHESIS,
            label: "Hypothesis Queue".into(),
            icon: '◬',
            color: Theme::PRIMITIVE_EXISTENCE, // ∃ — does it exist?
        },
        SidebarItem {
            panel: PanelId::EXPERIENCE,
            label: "Experience Store".into(),
            icon: '◈',
            color: Theme::PRIMITIVE_PERSISTENCE, // π — durable knowledge
        },
        SidebarItem {
            panel: PanelId::SIGNAL,
            label: "Signal Dash".into(),
            icon: '◇',
            color: Theme::PRIMITIVE_BOUNDARY, // ∂ — threshold detection
        },
        SidebarItem {
            panel: PanelId::BRAIN,
            label: "Brain View".into(),
            icon: '◎',
            color: Theme::PRIMITIVE_MAPPING, // μ — neural mapping
        },
        SidebarItem {
            panel: PanelId::GUARDIAN,
            label: "Guardian".into(),
            icon: '◆',
            color: Theme::PRIMITIVE_COMPARISON, // κ — homeostasis comparison
        },
        SidebarItem {
            panel: PanelId::MCP,
            label: "MCP Tools".into(),
            icon: '◐',
            color: Theme::PRIMITIVE_SEQUENCE, // σ — tool pipeline
        },
        SidebarItem {
            panel: PanelId::CLOUD,
            label: "Cloud Status".into(),
            icon: '\u{2601}',
            color: Theme::PRIMITIVE_LOCATION, // λ — spatial placement
        },
    ]
}

/// Paint a single sidebar item — icon in primitive color, label in text color.
fn paint_item(
    item: &SidebarItem,
    index: usize,
    rect: &Rect,
    active: PanelId,
) -> Vec<DisplayCommand> {
    let y = rect.y + (index as f32) * ITEM_HEIGHT + 8.0;
    let is_active = item.panel == active;
    let label_color = if is_active {
        Theme::SIDEBAR_ACTIVE
    } else {
        Theme::SIDEBAR_TEXT
    };

    let mut cmds = Vec::with_capacity(3);

    if is_active {
        cmds.push(DisplayCommand::FillRect {
            rect: Rect {
                x: rect.x,
                y,
                width: rect.width,
                height: ITEM_HEIGHT,
            },
            color: Theme::TAB_INACTIVE,
            node_id: None,
        });
    }

    // Icon in its primitive color
    cmds.push(DisplayCommand::DrawText {
        text: item.icon.to_string(),
        x: rect.x + ITEM_PAD,
        y: y + 20.0,
        size: Theme::FONT_SIZE,
        color: item.color,
        node_id: None,
    });

    // Label in text/active color
    cmds.push(DisplayCommand::DrawText {
        text: item.label.clone(),
        x: rect.x + ITEM_PAD + 22.0,
        y: y + 20.0,
        size: Theme::FONT_SIZE,
        color: label_color,
        node_id: None,
    });

    cmds
}

impl Widget for Sidebar {
    fn id(&self) -> WidgetId {
        WidgetId(102)
    }

    fn layout(&mut self, available: Rect) -> Rect {
        self.rect = if self.visible {
            Rect {
                x: available.x,
                y: available.y,
                width: Theme::SIDEBAR_WIDTH,
                height: available.height,
            }
        } else {
            Rect {
                x: available.x,
                y: available.y,
                width: 0.0,
                height: 0.0,
            }
        };
        self.rect
    }

    fn paint(&self) -> Vec<DisplayCommand> {
        if !self.visible {
            return Vec::new();
        }

        let mut cmds = Vec::with_capacity(self.items.len() * 3 + 1);

        // Background
        cmds.push(DisplayCommand::FillRect {
            rect: self.rect,
            color: Theme::SIDEBAR_BG,
            node_id: None,
        });

        // Items
        for (i, item) in self.items.iter().enumerate() {
            cmds.extend(paint_item(item, i, &self.rect, self.active));
        }

        // Separator line (right edge)
        cmds.push(DisplayCommand::FillRect {
            rect: Rect {
                x: self.rect.x + self.rect.width - 1.0,
                y: self.rect.y,
                width: 1.0,
                height: self.rect.height,
            },
            color: Theme::SIDEBAR_SEP,
            node_id: None,
        });

        cmds
    }

    fn handle_click(&mut self, _x: f32, y: f32) -> Option<Message> {
        if !self.visible {
            return None;
        }
        let relative_y = y - self.rect.y - 8.0;
        if relative_y < 0.0 {
            return None;
        }
        let index = (relative_y / ITEM_HEIGHT) as usize;
        self.items
            .get(index)
            .map(|item| Message::SelectPanel(item.panel))
    }

    fn hit_test(&self, x: f32, y: f32) -> bool {
        self.visible && self.rect.contains(x, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sidebar_layout_visible() {
        let mut sidebar = Sidebar::new();
        let available = Rect {
            x: 0.0,
            y: 76.0,
            width: 280.0,
            height: 620.0,
        };
        let rect = sidebar.layout(available);
        assert!((rect.width - 280.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_sidebar_layout_hidden() {
        let mut sidebar = Sidebar::new();
        sidebar.set_visible(false);
        let available = Rect {
            x: 0.0,
            y: 76.0,
            width: 280.0,
            height: 620.0,
        };
        let rect = sidebar.layout(available);
        assert!(rect.width.abs() < f32::EPSILON);
    }

    #[test]
    fn test_sidebar_paint_hidden() {
        let mut sidebar = Sidebar::new();
        sidebar.set_visible(false);
        assert!(sidebar.paint().is_empty());
    }

    #[test]
    fn test_default_items() {
        let items = default_items();
        assert_eq!(items.len(), 8);
        assert_eq!(items[0].panel, PanelId::GROUNDED);
        assert_eq!(items[7].panel, PanelId::CLOUD);
    }
}
