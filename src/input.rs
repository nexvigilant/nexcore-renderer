//! Input handling for keyboard and mouse events.
//!
//! Manages keyboard/mouse state, the address bar overlay, and
//! translates raw window events into high-level `BrowserAction` values.

use crate::layout::Rect;
use crate::paint::DisplayCommand;
use crate::style::Color;
use winit::event::{ElementState, KeyEvent, MouseButton};
use winit::keyboard::{KeyCode, PhysicalKey};

// ── Address bar constants ───────────────────────────────────────────
/// Height of the address bar in pixels.
pub const ADDRESS_BAR_HEIGHT: f32 = 40.0;
/// Horizontal padding inside the text field.
const BAR_PAD_X: f32 = 12.0;
/// Vertical padding inside the text field.
const BAR_PAD_Y: f32 = 6.0;
/// Address bar background color (dark chrome).
const BAR_BG: Color = Color {
    r: 36,
    g: 36,
    b: 54,
    a: 255,
};
/// Address bar text field background.
const BAR_FIELD_BG: Color = Color {
    r: 50,
    g: 50,
    b: 72,
    a: 255,
};
/// Address bar text color.
const BAR_TEXT_COLOR: Color = Color {
    r: 220,
    g: 220,
    b: 240,
    a: 255,
};
/// Address bar cursor color.
const BAR_CURSOR_COLOR: Color = Color {
    r: 120,
    g: 180,
    b: 255,
    a: 255,
};

// ── Address bar state ───────────────────────────────────────────────

/// State of the browser address bar overlay.
#[derive(Debug, Clone, Default)]
pub struct AddressBarState {
    /// Whether the address bar text field is focused for editing.
    pub focused: bool,
    /// The text currently displayed / being edited.
    pub text: String,
    /// Cursor position (byte offset into `text`).
    pub cursor: usize,
}

impl AddressBarState {
    /// Set the displayed URL (e.g. after navigation).
    pub fn set_url(&mut self, url: &str) {
        self.text = url.to_string();
        self.cursor = self.text.len();
    }

    /// Focus the address bar and select all text for replacement.
    pub fn focus(&mut self) {
        self.focused = true;
        self.cursor = self.text.len();
    }

    /// Unfocus the address bar.
    pub fn blur(&mut self) {
        self.focused = false;
    }

    /// Insert a character at the cursor.
    pub fn insert_char(&mut self, ch: char) {
        if self.cursor <= self.text.len() {
            self.text.insert(self.cursor, ch);
            self.cursor += ch.len_utf8();
        }
    }

    /// Delete the character before the cursor (backspace).
    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            // Find the previous char boundary
            let prev = self.text[..self.cursor]
                .char_indices()
                .next_back()
                .map_or(0, |(i, _)| i);
            self.text.remove(prev);
            self.cursor = prev;
        }
    }

    /// Consume the current text as a URL to navigate to.
    /// Returns the URL and blurs the bar.
    pub fn submit(&mut self) -> Option<String> {
        if self.text.is_empty() {
            return None;
        }
        self.focused = false;
        let url = self.text.clone();
        Some(url)
    }

    /// Build display commands for the address bar overlay.
    ///
    /// `viewport_width` is the current window width in pixels.
    #[must_use]
    pub fn build_display_commands(&self, viewport_width: f32) -> Vec<DisplayCommand> {
        let mut cmds = Vec::with_capacity(4);

        // Full-width bar background
        cmds.push(DisplayCommand::FillRect {
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: viewport_width,
                height: ADDRESS_BAR_HEIGHT,
            },
            color: BAR_BG,
            node_id: None,
        });

        // Text field background (inset)
        let field_x = BAR_PAD_X;
        let field_y = BAR_PAD_Y;
        let field_w = viewport_width - BAR_PAD_X * 2.0;
        let field_h = ADDRESS_BAR_HEIGHT - BAR_PAD_Y * 2.0;
        cmds.push(DisplayCommand::FillRect {
            rect: Rect {
                x: field_x,
                y: field_y,
                width: field_w,
                height: field_h,
            },
            color: BAR_FIELD_BG,
            node_id: None,
        });

        // URL text
        let display_text = if self.text.is_empty() && !self.focused {
            "Type a URL and press Enter..."
        } else {
            &self.text
        };

        let text_x = field_x + 8.0;
        let text_y = field_y + field_h * 0.5 + 5.0; // approximate vertical centering
        let font_size = 14.0;

        let text_color = if self.text.is_empty() && !self.focused {
            Color {
                r: 140,
                g: 140,
                b: 160,
                a: 255,
            }
        } else {
            BAR_TEXT_COLOR
        };

        cmds.push(DisplayCommand::DrawText {
            text: display_text.to_string(),
            x: text_x,
            y: text_y,
            size: font_size,
            color: text_color,
            node_id: None,
        });

        // Blinking cursor (shown when focused)
        if self.focused {
            let cursor_x = text_x + self.cursor as f32 * font_size * 0.6;
            cmds.push(DisplayCommand::FillRect {
                rect: Rect {
                    x: cursor_x,
                    y: field_y + 4.0,
                    width: 2.0,
                    height: field_h - 8.0,
                },
                color: BAR_CURSOR_COLOR,
                node_id: None,
            });
        }

        cmds
    }

    /// Check if a click position is inside the address bar area.
    #[must_use]
    pub fn contains_click(&self, _x: f32, y: f32) -> bool {
        y < ADDRESS_BAR_HEIGHT
    }
}

// ── Input state ─────────────────────────────────────────────────────

/// Input state tracking.
#[derive(Debug, Default)]
pub struct InputState {
    /// Current mouse position.
    pub mouse_pos: (f32, f32),
    /// Mouse button states.
    pub mouse_buttons: MouseButtons,
    /// Keyboard modifiers.
    pub modifiers: Modifiers,
    /// Current scroll offset.
    pub scroll_offset: (f32, f32),
    /// Text input buffer (legacy, kept for compatibility).
    pub text_buffer: String,
    /// Currently focused element ID.
    pub focused_element: Option<usize>,
    /// Address bar state.
    pub address_bar: AddressBarState,
}

/// Mouse button state.
#[derive(Debug, Default, Clone, Copy)]
pub struct MouseButtons {
    /// Left mouse button pressed.
    pub left: bool,
    /// Right mouse button pressed.
    pub right: bool,
    /// Middle mouse button pressed.
    pub middle: bool,
}

/// Keyboard modifiers.
#[derive(Debug, Default, Clone, Copy)]
pub struct Modifiers {
    /// Shift key pressed.
    pub shift: bool,
    /// Control key pressed.
    pub ctrl: bool,
    /// Alt key pressed.
    pub alt: bool,
    /// Super/Meta key pressed.
    pub meta: bool,
}

/// Browser action triggered by input.
#[derive(Debug, Clone)]
pub enum BrowserAction {
    /// Navigate to URL.
    Navigate(String),
    /// Go back in history.
    Back,
    /// Go forward in history.
    Forward,
    /// Reload current page.
    Reload,
    /// Scroll by delta.
    Scroll {
        /// Horizontal delta.
        dx: f32,
        /// Vertical delta.
        dy: f32,
    },
    /// Scroll by one page (Page Up/Down or Space/Shift+Space).
    ScrollPage {
        /// True for page down, false for page up.
        down: bool,
    },
    /// Scroll to edge (Home/End).
    ScrollToEdge {
        /// True for top (Home), false for bottom (End).
        top: bool,
    },
    /// Scroll by one line (Arrow Up/Down).
    ScrollLine {
        /// True for line down, false for line up.
        down: bool,
    },
    /// Click at position (page content area).
    Click {
        /// X coordinate.
        x: f32,
        /// Y coordinate.
        y: f32,
    },
    /// Focus the address bar.
    FocusAddressBar,
    /// Focus element.
    Focus(usize),
    /// Text input.
    TextInput(String),
    /// Open DevTools.
    DevTools,
    /// Zoom in/out.
    Zoom(f32),
    /// Find in page.
    Find(String),
    /// Copy selection.
    Copy,
    /// Paste clipboard.
    Paste,
    /// Focus a form element by its registry index.
    FocusFormElement(usize),
    /// Insert a character into the focused form element.
    FormCharInput(char),
    /// Backspace in the focused form element.
    FormBackspace,
    /// Delete forward in the focused form element.
    FormDeleteForward,
    /// Move cursor left in the focused form element.
    FormCursorLeft,
    /// Move cursor right in the focused form element.
    FormCursorRight,
    /// Move cursor to start in the focused form element.
    FormCursorHome,
    /// Move cursor to end in the focused form element.
    FormCursorEnd,
    /// Toggle a checkbox form element.
    FormToggleCheckbox(usize),
    /// Tab to next form element.
    FormTabNext,
    /// Shift+Tab to previous form element.
    FormTabPrev,
    /// No action.
    None,
}

impl InputState {
    /// Create new input state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Update mouse position.
    pub fn set_mouse_pos(&mut self, x: f32, y: f32) {
        self.mouse_pos = (x, y);
    }

    /// Handle mouse button event.
    pub fn handle_mouse_button(
        &mut self,
        button: MouseButton,
        state: ElementState,
    ) -> BrowserAction {
        let pressed = state == ElementState::Pressed;

        match button {
            MouseButton::Left => {
                self.mouse_buttons.left = pressed;
                if pressed {
                    let (x, y) = self.mouse_pos;
                    // Click in address bar area?
                    if self.address_bar.contains_click(x, y) {
                        self.address_bar.focus();
                        return BrowserAction::FocusAddressBar;
                    }
                    // Clicked outside address bar -- blur it
                    self.address_bar.blur();
                    BrowserAction::Click { x, y }
                } else {
                    BrowserAction::None
                }
            }
            MouseButton::Right => {
                self.mouse_buttons.right = pressed;
                BrowserAction::None // Context menu would go here
            }
            MouseButton::Middle => {
                self.mouse_buttons.middle = pressed;
                BrowserAction::None
            }
            _ => BrowserAction::None,
        }
    }

    /// Handle scroll event.
    pub fn handle_scroll(&mut self, dx: f32, dy: f32) -> BrowserAction {
        self.scroll_offset.0 += dx;
        self.scroll_offset.1 += dy;
        BrowserAction::Scroll { dx, dy }
    }

    /// Handle keyboard event.
    pub fn handle_key(&mut self, event: &KeyEvent) -> BrowserAction {
        if event.state != ElementState::Pressed {
            return BrowserAction::None;
        }

        // Handle keyboard shortcuts first (Ctrl+*)
        if self.modifiers.ctrl {
            return self.handle_ctrl_shortcut(event);
        }

        // If address bar is focused, route keys there
        if self.address_bar.focused {
            return self.handle_address_bar_key(event);
        }

        // Handle navigation and scroll keys
        match &event.physical_key {
            PhysicalKey::Code(KeyCode::Backspace) => {
                self.text_buffer.pop();
                BrowserAction::None
            }
            PhysicalKey::Code(KeyCode::Enter) => {
                let url = std::mem::take(&mut self.text_buffer);
                if url.is_empty() {
                    BrowserAction::None
                } else {
                    BrowserAction::Navigate(url)
                }
            }
            PhysicalKey::Code(KeyCode::Escape) => {
                self.text_buffer.clear();
                self.focused_element = None;
                BrowserAction::None
            }
            PhysicalKey::Code(KeyCode::F5) => BrowserAction::Reload,
            PhysicalKey::Code(KeyCode::F12) => BrowserAction::DevTools,
            // ── Scroll keys (when address bar is not focused) ────
            PhysicalKey::Code(KeyCode::PageDown) => BrowserAction::ScrollPage { down: true },
            PhysicalKey::Code(KeyCode::PageUp) => BrowserAction::ScrollPage { down: false },
            PhysicalKey::Code(KeyCode::Home) => BrowserAction::ScrollToEdge { top: true },
            PhysicalKey::Code(KeyCode::End) => BrowserAction::ScrollToEdge { top: false },
            PhysicalKey::Code(KeyCode::ArrowDown) => BrowserAction::ScrollLine { down: true },
            PhysicalKey::Code(KeyCode::ArrowUp) => BrowserAction::ScrollLine { down: false },
            PhysicalKey::Code(KeyCode::Space) => {
                if self.modifiers.shift {
                    BrowserAction::ScrollPage { down: false }
                } else {
                    BrowserAction::ScrollPage { down: true }
                }
            }
            _ => BrowserAction::None,
        }
    }

    /// Handle a key event when the address bar is focused.
    fn handle_address_bar_key(&mut self, event: &KeyEvent) -> BrowserAction {
        match &event.physical_key {
            PhysicalKey::Code(KeyCode::Enter) => {
                if let Some(url) = self.address_bar.submit() {
                    // Auto-add scheme if missing
                    let resolved = if url.contains("://") || url.starts_with("data:") {
                        url
                    } else if url.contains('.') {
                        format!("https://{url}")
                    } else {
                        url
                    };
                    BrowserAction::Navigate(resolved)
                } else {
                    BrowserAction::None
                }
            }
            PhysicalKey::Code(KeyCode::Escape) => {
                self.address_bar.blur();
                BrowserAction::None
            }
            PhysicalKey::Code(KeyCode::Backspace) => {
                self.address_bar.backspace();
                BrowserAction::None
            }
            _ => {
                // Insert printable characters from the logical key text
                if let Some(text) = &event.text {
                    for ch in text.as_str().chars() {
                        if !ch.is_control() {
                            self.address_bar.insert_char(ch);
                        }
                    }
                }
                BrowserAction::None
            }
        }
    }

    fn handle_ctrl_shortcut(&mut self, event: &KeyEvent) -> BrowserAction {
        match &event.physical_key {
            PhysicalKey::Code(KeyCode::KeyR) => BrowserAction::Reload,
            PhysicalKey::Code(KeyCode::KeyL) => {
                // Focus address bar
                self.address_bar.focus();
                BrowserAction::FocusAddressBar
            }
            PhysicalKey::Code(KeyCode::KeyF) => BrowserAction::Find(String::new()),
            PhysicalKey::Code(KeyCode::KeyC) => BrowserAction::Copy,
            PhysicalKey::Code(KeyCode::KeyV) => BrowserAction::Paste,
            PhysicalKey::Code(KeyCode::Equal) | PhysicalKey::Code(KeyCode::NumpadAdd) => {
                BrowserAction::Zoom(1.1)
            }
            PhysicalKey::Code(KeyCode::Minus) | PhysicalKey::Code(KeyCode::NumpadSubtract) => {
                BrowserAction::Zoom(0.9)
            }
            PhysicalKey::Code(KeyCode::Digit0) => BrowserAction::Zoom(1.0),
            _ => BrowserAction::None,
        }
    }

    /// Handle text input.
    pub fn handle_text_input(&mut self, text: &str) {
        self.text_buffer.push_str(text);
    }

    /// Update modifier state.
    pub fn set_modifiers(&mut self, modifiers: winit::keyboard::ModifiersState) {
        self.modifiers.shift = modifiers.shift_key();
        self.modifiers.ctrl = modifiers.control_key();
        self.modifiers.alt = modifiers.alt_key();
        self.modifiers.meta = modifiers.super_key();
    }
}

/// Hit testing for click events.
pub struct HitTester;

impl HitTester {
    /// Find element at position.
    #[must_use]
    pub fn hit_test(elements: &[(usize, Rect)], x: f32, y: f32) -> Option<usize> {
        // Reverse iterate (top elements first in paint order)
        for (id, rect) in elements.iter().rev() {
            if rect.contains(x, y) {
                return Some(*id);
            }
        }
        None
    }
}

/// Translates keyboard events into `BrowserAction` values when a form
/// element has focus.
///
/// The caller checks whether a form element is focused and routes the
/// event here instead of the default key handler.
///
/// Tier: T2-C (state + mapping + causality)
pub struct FormInputHandler;

impl FormInputHandler {
    /// Handle a key event when a form element is focused.
    ///
    /// Returns a form-specific `BrowserAction`, or `BrowserAction::None`
    /// if the key is not relevant to form editing.
    #[must_use]
    pub fn handle_key(event: &KeyEvent, modifiers: &Modifiers) -> BrowserAction {
        if event.state != ElementState::Pressed {
            return BrowserAction::None;
        }

        // Tab cycles focus between form elements
        if let PhysicalKey::Code(KeyCode::Tab) = &event.physical_key {
            return if modifiers.shift {
                BrowserAction::FormTabPrev
            } else {
                BrowserAction::FormTabNext
            };
        }

        // Escape blurs the form element (handled by returning None, caller should blur)
        // Other keys are handled by the form element type
        match &event.physical_key {
            PhysicalKey::Code(KeyCode::Backspace) => BrowserAction::FormBackspace,
            PhysicalKey::Code(KeyCode::Delete) => BrowserAction::FormDeleteForward,
            PhysicalKey::Code(KeyCode::ArrowLeft) => BrowserAction::FormCursorLeft,
            PhysicalKey::Code(KeyCode::ArrowRight) => BrowserAction::FormCursorRight,
            PhysicalKey::Code(KeyCode::Home) => BrowserAction::FormCursorHome,
            PhysicalKey::Code(KeyCode::End) => BrowserAction::FormCursorEnd,
            _ => {
                // Insert printable characters
                if let Some(text) = &event.text {
                    for ch in text.as_str().chars() {
                        if !ch.is_control() {
                            return BrowserAction::FormCharInput(ch);
                        }
                    }
                }
                BrowserAction::None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_contains() {
        let rect = Rect {
            x: 10.0,
            y: 10.0,
            width: 100.0,
            height: 50.0,
        };
        assert!(rect.contains(50.0, 30.0));
        assert!(!rect.contains(5.0, 5.0));
        assert!(!rect.contains(150.0, 30.0));
    }

    #[test]
    fn test_hit_test() {
        let elements = vec![
            (
                1,
                Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 100.0,
                    height: 100.0,
                },
            ),
            (
                2,
                Rect {
                    x: 50.0,
                    y: 50.0,
                    width: 100.0,
                    height: 100.0,
                },
            ),
        ];

        // Should hit top element (id=2) when overlapping
        assert_eq!(HitTester::hit_test(&elements, 75.0, 75.0), Some(2));
        // Should hit bottom element when not overlapping
        assert_eq!(HitTester::hit_test(&elements, 25.0, 25.0), Some(1));
        // Should miss when outside all
        assert_eq!(HitTester::hit_test(&elements, 200.0, 200.0), None);
    }

    #[test]
    fn test_scroll_accumulation() {
        let mut input = InputState::new();
        input.handle_scroll(10.0, 20.0);
        input.handle_scroll(-5.0, 10.0);
        assert_eq!(input.scroll_offset, (5.0, 30.0));
    }

    #[test]
    fn test_address_bar_insert_and_backspace() {
        let mut bar = AddressBarState::default();
        bar.focus();
        bar.insert_char('h');
        bar.insert_char('i');
        assert_eq!(bar.text, "hi");
        assert_eq!(bar.cursor, 2);
        bar.backspace();
        assert_eq!(bar.text, "h");
        assert_eq!(bar.cursor, 1);
    }

    #[test]
    fn test_address_bar_submit() {
        let mut bar = AddressBarState::default();
        bar.focus();
        bar.insert_char('x');
        let url = bar.submit();
        assert_eq!(url, Some("x".to_string()));
        assert!(!bar.focused);
    }

    #[test]
    fn test_address_bar_submit_empty() {
        let mut bar = AddressBarState::default();
        bar.focus();
        let url = bar.submit();
        assert_eq!(url, None);
    }

    #[test]
    fn test_address_bar_click_detection() {
        let bar = AddressBarState::default();
        assert!(bar.contains_click(100.0, 10.0));
        assert!(bar.contains_click(100.0, 39.0));
        assert!(!bar.contains_click(100.0, 41.0));
    }
}
