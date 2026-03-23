//! Toolbar widget — navigation buttons + URL bar.
//!
//! ## Tier Classification
//!
//! - `Toolbar`: T3 (domain widget)

use super::{Theme, Widget};
use crate::layout::Rect;
use crate::paint::DisplayCommand;
use crate::state::{Message, WidgetId};

/// Navigation button width.
const NAV_BTN_SIZE: f32 = 32.0;
/// Padding between nav buttons.
const NAV_PAD: f32 = 4.0;
/// URL field horizontal padding.
const URL_PAD: f32 = 8.0;

/// Tier: T3 — Toolbar widget with nav buttons and URL bar.
pub struct Toolbar {
    /// Bounding rect.
    rect: Rect,
    /// Current URL text.
    url_text: String,
    /// Whether the URL field is focused.
    focused: bool,
}

impl Toolbar {
    /// Create a new toolbar.
    #[must_use]
    pub fn new() -> Self {
        Self {
            rect: Rect::default(),
            url_text: String::new(),
            focused: false,
        }
    }

    /// Update the displayed URL.
    pub fn set_url(&mut self, url: &str) {
        self.url_text = url.to_string();
    }

    /// Set focus state.
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }
}

impl Default for Toolbar {
    fn default() -> Self {
        Self::new()
    }
}

/// Paint the navigation buttons (back, forward, reload, home).
fn paint_nav_buttons(rect: &Rect) -> Vec<DisplayCommand> {
    let buttons = ["◄", "►", "⟳", "⌂"];
    let y = rect.y + (rect.height - NAV_BTN_SIZE) / 2.0;
    let mut cmds = Vec::with_capacity(buttons.len() * 2);

    for (i, label) in buttons.iter().enumerate() {
        let x = rect.x + NAV_PAD + (i as f32) * (NAV_BTN_SIZE + NAV_PAD);
        let btn_rect = Rect {
            x,
            y,
            width: NAV_BTN_SIZE,
            height: NAV_BTN_SIZE,
        };
        cmds.push(DisplayCommand::FillRect {
            rect: btn_rect,
            color: Theme::TAB_INACTIVE,
            node_id: None,
        });
        cmds.push(DisplayCommand::DrawText {
            text: (*label).to_string(),
            x: x + 8.0,
            y: y + 20.0,
            size: Theme::FONT_SIZE,
            color: Theme::NAV_BUTTON,
            node_id: None,
        });
    }
    cmds
}

/// Paint the URL input field.
fn paint_url_field(rect: &Rect, url: &str, focused: bool) -> Vec<DisplayCommand> {
    let nav_width = NAV_PAD + 4.0 * (NAV_BTN_SIZE + NAV_PAD);
    let field_x = rect.x + nav_width + URL_PAD;
    let field_y = rect.y + 6.0;
    let field_w = rect.width - nav_width - URL_PAD * 2.0 - 40.0;
    let field_h = rect.height - 12.0;

    let field_rect = Rect {
        x: field_x,
        y: field_y,
        width: field_w,
        height: field_h,
    };
    let display_text = if url.is_empty() && !focused {
        "Type a URL and press Enter..."
    } else {
        url
    };

    let text_color = if url.is_empty() && !focused {
        Theme::STATUS_TEXT
    } else {
        Theme::URL_TEXT
    };

    vec![
        DisplayCommand::FillRect {
            rect: field_rect,
            color: Theme::URL_FIELD_BG,
            node_id: None,
        },
        DisplayCommand::DrawText {
            text: display_text.to_string(),
            x: field_x + 8.0,
            y: field_y + field_h * 0.5 + 5.0,
            size: 14.0,
            color: text_color,
            node_id: None,
        },
    ]
}

impl Widget for Toolbar {
    fn id(&self) -> WidgetId {
        WidgetId(101)
    }

    fn layout(&mut self, available: Rect) -> Rect {
        self.rect = Rect {
            x: available.x,
            y: available.y,
            width: available.width,
            height: Theme::TOOLBAR_HEIGHT,
        };
        self.rect
    }

    fn paint(&self) -> Vec<DisplayCommand> {
        let mut cmds = Vec::with_capacity(12);

        // Background
        cmds.push(DisplayCommand::FillRect {
            rect: self.rect,
            color: Theme::TOOLBAR_BG,
            node_id: None,
        });

        // Nav buttons
        cmds.extend(paint_nav_buttons(&self.rect));

        // URL field
        cmds.extend(paint_url_field(&self.rect, &self.url_text, self.focused));

        // Hamburger menu button (right side)
        let menu_x = self.rect.x + self.rect.width - 36.0;
        cmds.push(DisplayCommand::DrawText {
            text: "≡".to_string(),
            x: menu_x + 8.0,
            y: self.rect.y + 26.0,
            size: 18.0,
            color: Theme::NAV_BUTTON,
            node_id: None,
        });

        cmds
    }

    fn handle_click(&mut self, x: f32, _y: f32) -> Option<Message> {
        let nav_width = NAV_PAD + 4.0 * (NAV_BTN_SIZE + NAV_PAD);

        // Check nav buttons
        if x < self.rect.x + nav_width {
            let btn_index = ((x - self.rect.x - NAV_PAD) / (NAV_BTN_SIZE + NAV_PAD)) as usize;
            return match btn_index {
                0 => Some(Message::GoBack),
                1 => Some(Message::GoForward),
                2 => Some(Message::Reload),
                3 => Some(Message::Navigate("nex://welcome".into())),
                _ => None,
            };
        }

        // URL field click
        Some(Message::FocusAddressBar)
    }

    fn hit_test(&self, x: f32, y: f32) -> bool {
        self.rect.contains(x, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toolbar_layout() {
        let mut toolbar = Toolbar::new();
        let available = Rect {
            x: 0.0,
            y: 36.0,
            width: 1280.0,
            height: 40.0,
        };
        let rect = toolbar.layout(available);
        assert!((rect.height - 40.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_toolbar_paint() {
        let toolbar = Toolbar::new();
        let cmds = toolbar.paint();
        assert!(!cmds.is_empty());
    }

    #[test]
    fn test_nav_button_click() {
        let mut toolbar = Toolbar::new();
        toolbar.rect = Rect {
            x: 0.0,
            y: 0.0,
            width: 1280.0,
            height: 40.0,
        };
        // Click on first nav button (back)
        let msg = toolbar.handle_click(10.0, 20.0);
        assert!(matches!(msg, Some(Message::GoBack)));
    }
}
