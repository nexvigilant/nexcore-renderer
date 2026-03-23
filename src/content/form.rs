//! Form element support for NexBrowser.
//!
//! Provides types for `<input>`, `<button>`, `<textarea>`, and `<input type="checkbox">`
//! elements, along with a registry that tracks all form elements on the current page.
//!
//! ## Tier Classification
//!
//! - `FormElementKind`: T2-P (enum/recursion + state)
//! - `FormElement`: T2-C (state + boundary + location)
//! - `FormRegistry`: T2-C (sequence + state + mapping)
//!
//! ## Primitive Grounding
//!
//! | Concept       | T1 Primitive      | Symbol  |
//! |---------------|-------------------|---------|
//! | Element List  | Sequence          | sigma   |
//! | Value Editing | Mapping           | mu      |
//! | Focus/Cursor  | State             | varsigma|
//! | Bounds        | Boundary+Location | partial+lambda |

use crate::layout::Rect;
use crate::paint::{DisplayCommand, Point};
use crate::style::Color;

// ── Form element colors ─────────────────────────────────────────────

/// Border color for unfocused inputs.
const INPUT_BORDER: Color = Color {
    r: 140,
    g: 140,
    b: 160,
    a: 255,
};

/// Border color for focused inputs.
const INPUT_FOCUS_BORDER: Color = Color {
    r: 100,
    g: 160,
    b: 255,
    a: 255,
};

/// Background color for text inputs.
const INPUT_BG: Color = Color {
    r: 255,
    g: 255,
    b: 255,
    a: 255,
};

/// Text color for inputs.
const INPUT_TEXT: Color = Color {
    r: 30,
    g: 30,
    b: 30,
    a: 255,
};

/// Placeholder text color.
const INPUT_PLACEHOLDER: Color = Color {
    r: 160,
    g: 160,
    b: 170,
    a: 255,
};

/// Cursor color in focused fields.
const INPUT_CURSOR: Color = Color {
    r: 30,
    g: 30,
    b: 30,
    a: 255,
};

/// Button background color.
const BUTTON_BG: Color = Color {
    r: 230,
    g: 230,
    b: 235,
    a: 255,
};

/// Button hover background color.
const BUTTON_HOVER_BG: Color = Color {
    r: 210,
    g: 210,
    b: 220,
    a: 255,
};

/// Button text color.
const BUTTON_TEXT: Color = Color {
    r: 30,
    g: 30,
    b: 30,
    a: 255,
};

/// Button border color.
const BUTTON_BORDER: Color = Color {
    r: 180,
    g: 180,
    b: 190,
    a: 255,
};

/// Checkbox checkmark color.
const CHECKBOX_CHECK: Color = Color {
    r: 60,
    g: 120,
    b: 255,
    a: 255,
};

/// Default font size for form elements.
const FORM_FONT_SIZE: f32 = 14.0;

/// Default height for single-line inputs.
const INPUT_HEIGHT: f32 = 28.0;

/// Default width for text inputs.
const INPUT_WIDTH: f32 = 200.0;

/// Checkbox box size.
const CHECKBOX_SIZE: f32 = 16.0;

/// Inner padding for text inputs.
const INPUT_PAD: f32 = 6.0;

// ── Types ───────────────────────────────────────────────────────────

/// The kind of form element, carrying its specific state.
///
/// Tier: T2-P (enum/recursion + state)
#[derive(Debug, Clone)]
pub enum FormElementKind {
    /// Single-line text input (`<input type="text">`, `<input type="password">`, etc.).
    TextInput {
        /// Current text value.
        value: String,
        /// Placeholder text shown when value is empty.
        placeholder: String,
        /// Cursor position in characters (not bytes).
        cursor_pos: usize,
    },
    /// Clickable button (`<button>`, `<input type="submit">`).
    Button {
        /// Label text shown on the button.
        label: String,
    },
    /// Checkbox input (`<input type="checkbox">`).
    Checkbox {
        /// Whether the checkbox is checked.
        checked: bool,
        /// Label text next to the checkbox.
        label: String,
    },
    /// Multi-line text area (`<textarea>`).
    TextArea {
        /// Current text value.
        value: String,
        /// Cursor position in characters.
        cursor_pos: usize,
        /// Number of visible rows.
        rows: usize,
    },
}

/// A form element with its metadata and spatial bounds.
///
/// Tier: T2-C (state + boundary + location)
#[derive(Debug, Clone)]
pub struct FormElement {
    /// The kind and state of this element.
    pub kind: FormElementKind,
    /// HTML `id` attribute if present.
    pub id: Option<String>,
    /// HTML `name` attribute if present.
    pub name: Option<String>,
    /// Whether this element currently has keyboard focus.
    pub focused: bool,
    /// Whether the mouse is hovering over this element.
    pub hovered: bool,
    /// Bounding rectangle in page coordinates.
    pub bounds: Rect,
}

impl FormElement {
    /// Create a new text input element.
    #[must_use]
    pub fn text_input(
        id: Option<String>,
        name: Option<String>,
        value: String,
        placeholder: String,
    ) -> Self {
        Self {
            kind: FormElementKind::TextInput {
                value,
                placeholder,
                cursor_pos: 0,
            },
            id,
            name,
            focused: false,
            hovered: false,
            bounds: Rect {
                x: 0.0,
                y: 0.0,
                width: INPUT_WIDTH,
                height: INPUT_HEIGHT,
            },
        }
    }

    /// Create a new button element.
    #[must_use]
    pub fn button(id: Option<String>, name: Option<String>, label: String) -> Self {
        let width = (label.len() as f32 * FORM_FONT_SIZE * 0.6 + INPUT_PAD * 4.0).max(80.0);
        Self {
            kind: FormElementKind::Button { label },
            id,
            name,
            focused: false,
            hovered: false,
            bounds: Rect {
                x: 0.0,
                y: 0.0,
                width,
                height: INPUT_HEIGHT + 4.0,
            },
        }
    }

    /// Create a new checkbox element.
    #[must_use]
    pub fn checkbox(
        id: Option<String>,
        name: Option<String>,
        checked: bool,
        label: String,
    ) -> Self {
        let label_width = label.len() as f32 * FORM_FONT_SIZE * 0.6;
        Self {
            kind: FormElementKind::Checkbox { checked, label },
            id,
            name,
            focused: false,
            hovered: false,
            bounds: Rect {
                x: 0.0,
                y: 0.0,
                width: CHECKBOX_SIZE + 6.0 + label_width,
                height: CHECKBOX_SIZE.max(INPUT_HEIGHT),
            },
        }
    }

    /// Create a new textarea element.
    #[must_use]
    pub fn textarea(id: Option<String>, name: Option<String>, value: String, rows: usize) -> Self {
        let rows = if rows == 0 { 3 } else { rows };
        let height = rows as f32 * FORM_FONT_SIZE * 1.4 + INPUT_PAD * 2.0;
        Self {
            kind: FormElementKind::TextArea {
                value,
                cursor_pos: 0,
                rows,
            },
            id,
            name,
            focused: false,
            hovered: false,
            bounds: Rect {
                x: 0.0,
                y: 0.0,
                width: INPUT_WIDTH * 1.5,
                height,
            },
        }
    }

    /// Whether this element accepts keyboard text input.
    #[must_use]
    pub fn accepts_text_input(&self) -> bool {
        matches!(
            self.kind,
            FormElementKind::TextInput { .. } | FormElementKind::TextArea { .. }
        )
    }

    /// Insert a character at the cursor position for text-accepting elements.
    pub fn insert_char(&mut self, ch: char) {
        match &mut self.kind {
            FormElementKind::TextInput {
                value, cursor_pos, ..
            }
            | FormElementKind::TextArea {
                value, cursor_pos, ..
            } => {
                let byte_pos = char_to_byte_offset(value, *cursor_pos);
                if byte_pos <= value.len() {
                    value.insert(byte_pos, ch);
                    *cursor_pos += 1;
                }
            }
            _ => {}
        }
    }

    /// Delete the character before the cursor (backspace).
    pub fn backspace(&mut self) {
        match &mut self.kind {
            FormElementKind::TextInput {
                value, cursor_pos, ..
            }
            | FormElementKind::TextArea {
                value, cursor_pos, ..
            } => {
                if *cursor_pos > 0 {
                    let byte_pos = char_to_byte_offset(value, *cursor_pos);
                    // Find the previous char boundary
                    let prev_byte = value[..byte_pos]
                        .char_indices()
                        .next_back()
                        .map_or(0, |(i, _)| i);
                    value.remove(prev_byte);
                    *cursor_pos -= 1;
                }
            }
            _ => {}
        }
    }

    /// Delete the character after the cursor (delete key).
    pub fn delete_forward(&mut self) {
        match &mut self.kind {
            FormElementKind::TextInput {
                value, cursor_pos, ..
            }
            | FormElementKind::TextArea {
                value, cursor_pos, ..
            } => {
                let char_len = value.chars().count();
                if *cursor_pos < char_len {
                    let byte_pos = char_to_byte_offset(value, *cursor_pos);
                    if byte_pos < value.len() {
                        value.remove(byte_pos);
                    }
                }
            }
            _ => {}
        }
    }

    /// Move cursor left by one character.
    pub fn cursor_left(&mut self) {
        match &mut self.kind {
            FormElementKind::TextInput { cursor_pos, .. }
            | FormElementKind::TextArea { cursor_pos, .. } => {
                if *cursor_pos > 0 {
                    *cursor_pos -= 1;
                }
            }
            _ => {}
        }
    }

    /// Move cursor right by one character.
    pub fn cursor_right(&mut self) {
        match &mut self.kind {
            FormElementKind::TextInput {
                value, cursor_pos, ..
            }
            | FormElementKind::TextArea {
                value, cursor_pos, ..
            } => {
                let char_len = value.chars().count();
                if *cursor_pos < char_len {
                    *cursor_pos += 1;
                }
            }
            _ => {}
        }
    }

    /// Move cursor to the beginning of the text.
    pub fn cursor_home(&mut self) {
        match &mut self.kind {
            FormElementKind::TextInput { cursor_pos, .. }
            | FormElementKind::TextArea { cursor_pos, .. } => {
                *cursor_pos = 0;
            }
            _ => {}
        }
    }

    /// Move cursor to the end of the text.
    pub fn cursor_end(&mut self) {
        match &mut self.kind {
            FormElementKind::TextInput {
                value, cursor_pos, ..
            }
            | FormElementKind::TextArea {
                value, cursor_pos, ..
            } => {
                *cursor_pos = value.chars().count();
            }
            _ => {}
        }
    }

    /// Toggle checkbox state. Returns the new checked state, or None for non-checkboxes.
    pub fn toggle_checkbox(&mut self) -> Option<bool> {
        if let FormElementKind::Checkbox { checked, .. } = &mut self.kind {
            *checked = !*checked;
            Some(*checked)
        } else {
            None
        }
    }

    /// Get the current text value for text-accepting elements.
    #[must_use]
    pub fn text_value(&self) -> Option<&str> {
        match &self.kind {
            FormElementKind::TextInput { value, .. } | FormElementKind::TextArea { value, .. } => {
                Some(value)
            }
            _ => None,
        }
    }

    /// Build display commands for this form element.
    #[must_use]
    pub fn build_display_commands(&self, node_id: Option<usize>) -> Vec<DisplayCommand> {
        match &self.kind {
            FormElementKind::TextInput {
                value,
                placeholder,
                cursor_pos,
            } => self.build_text_input_commands(value, placeholder, *cursor_pos, node_id),
            FormElementKind::Button { label } => self.build_button_commands(label, node_id),
            FormElementKind::Checkbox { checked, label } => {
                self.build_checkbox_commands(*checked, label, node_id)
            }
            FormElementKind::TextArea {
                value, cursor_pos, ..
            } => self.build_textarea_commands(value, *cursor_pos, node_id),
        }
    }

    /// Build display commands for a text input.
    fn build_text_input_commands(
        &self,
        value: &str,
        placeholder: &str,
        cursor_pos: usize,
        node_id: Option<usize>,
    ) -> Vec<DisplayCommand> {
        let mut cmds = Vec::with_capacity(4);

        // Border rectangle (slightly larger than the input)
        let border_color = if self.focused {
            INPUT_FOCUS_BORDER
        } else {
            INPUT_BORDER
        };
        cmds.push(DisplayCommand::FillRect {
            rect: Rect {
                x: self.bounds.x - 1.0,
                y: self.bounds.y - 1.0,
                width: self.bounds.width + 2.0,
                height: self.bounds.height + 2.0,
            },
            color: border_color,
            node_id,
        });

        // Background fill
        cmds.push(DisplayCommand::FillRect {
            rect: self.bounds,
            color: INPUT_BG,
            node_id,
        });

        // Text or placeholder
        let (display_text, text_color) = if value.is_empty() && !self.focused {
            (placeholder, INPUT_PLACEHOLDER)
        } else {
            (value, INPUT_TEXT)
        };

        if !display_text.is_empty() {
            cmds.push(DisplayCommand::DrawText {
                text: display_text.to_string(),
                x: self.bounds.x + INPUT_PAD,
                y: self.bounds.y + self.bounds.height * 0.5 + FORM_FONT_SIZE * 0.35,
                size: FORM_FONT_SIZE,
                color: text_color,
                node_id,
            });
        }

        // Cursor when focused
        if self.focused {
            let cursor_x = self.bounds.x + INPUT_PAD + cursor_pos as f32 * FORM_FONT_SIZE * 0.6;
            cmds.push(DisplayCommand::FillRect {
                rect: Rect {
                    x: cursor_x,
                    y: self.bounds.y + 4.0,
                    width: 1.5,
                    height: self.bounds.height - 8.0,
                },
                color: INPUT_CURSOR,
                node_id,
            });
        }

        cmds
    }

    /// Build display commands for a button.
    fn build_button_commands(&self, label: &str, node_id: Option<usize>) -> Vec<DisplayCommand> {
        let mut cmds = Vec::with_capacity(3);

        // Border
        cmds.push(DisplayCommand::FillRect {
            rect: Rect {
                x: self.bounds.x - 1.0,
                y: self.bounds.y - 1.0,
                width: self.bounds.width + 2.0,
                height: self.bounds.height + 2.0,
            },
            color: BUTTON_BORDER,
            node_id,
        });

        // Background (hover-sensitive)
        let bg = if self.hovered {
            BUTTON_HOVER_BG
        } else {
            BUTTON_BG
        };
        cmds.push(DisplayCommand::FillRect {
            rect: self.bounds,
            color: bg,
            node_id,
        });

        // Centered label text
        if !label.is_empty() {
            let text_width = label.len() as f32 * FORM_FONT_SIZE * 0.6;
            let text_x = self.bounds.x + (self.bounds.width - text_width) * 0.5;
            let text_y = self.bounds.y + self.bounds.height * 0.5 + FORM_FONT_SIZE * 0.35;
            cmds.push(DisplayCommand::DrawText {
                text: label.to_string(),
                x: text_x,
                y: text_y,
                size: FORM_FONT_SIZE,
                color: BUTTON_TEXT,
                node_id,
            });
        }

        cmds
    }

    /// Build display commands for a checkbox.
    fn build_checkbox_commands(
        &self,
        checked: bool,
        label: &str,
        node_id: Option<usize>,
    ) -> Vec<DisplayCommand> {
        let mut cmds = Vec::with_capacity(5);

        let box_y = self.bounds.y + (self.bounds.height - CHECKBOX_SIZE) * 0.5;

        // Checkbox border
        cmds.push(DisplayCommand::FillRect {
            rect: Rect {
                x: self.bounds.x - 1.0,
                y: box_y - 1.0,
                width: CHECKBOX_SIZE + 2.0,
                height: CHECKBOX_SIZE + 2.0,
            },
            color: if self.focused {
                INPUT_FOCUS_BORDER
            } else {
                INPUT_BORDER
            },
            node_id,
        });

        // Checkbox background
        cmds.push(DisplayCommand::FillRect {
            rect: Rect {
                x: self.bounds.x,
                y: box_y,
                width: CHECKBOX_SIZE,
                height: CHECKBOX_SIZE,
            },
            color: INPUT_BG,
            node_id,
        });

        // Checkmark when checked (two lines forming a check)
        if checked {
            // Left leg of check
            cmds.push(DisplayCommand::StrokeLine {
                start: Point::new(self.bounds.x + 3.0, box_y + CHECKBOX_SIZE * 0.5),
                end: Point::new(
                    self.bounds.x + CHECKBOX_SIZE * 0.4,
                    box_y + CHECKBOX_SIZE - 3.0,
                ),
                width: 2.0,
                color: CHECKBOX_CHECK,
                node_id,
            });
            // Right leg of check
            cmds.push(DisplayCommand::StrokeLine {
                start: Point::new(
                    self.bounds.x + CHECKBOX_SIZE * 0.4,
                    box_y + CHECKBOX_SIZE - 3.0,
                ),
                end: Point::new(self.bounds.x + CHECKBOX_SIZE - 3.0, box_y + 3.0),
                width: 2.0,
                color: CHECKBOX_CHECK,
                node_id,
            });
        }

        // Label text
        if !label.is_empty() {
            cmds.push(DisplayCommand::DrawText {
                text: label.to_string(),
                x: self.bounds.x + CHECKBOX_SIZE + 6.0,
                y: self.bounds.y + self.bounds.height * 0.5 + FORM_FONT_SIZE * 0.35,
                size: FORM_FONT_SIZE,
                color: INPUT_TEXT,
                node_id,
            });
        }

        cmds
    }

    /// Build display commands for a textarea.
    fn build_textarea_commands(
        &self,
        value: &str,
        cursor_pos: usize,
        node_id: Option<usize>,
    ) -> Vec<DisplayCommand> {
        let mut cmds = Vec::with_capacity(4);

        // Border
        let border_color = if self.focused {
            INPUT_FOCUS_BORDER
        } else {
            INPUT_BORDER
        };
        cmds.push(DisplayCommand::FillRect {
            rect: Rect {
                x: self.bounds.x - 1.0,
                y: self.bounds.y - 1.0,
                width: self.bounds.width + 2.0,
                height: self.bounds.height + 2.0,
            },
            color: border_color,
            node_id,
        });

        // Background
        cmds.push(DisplayCommand::FillRect {
            rect: self.bounds,
            color: INPUT_BG,
            node_id,
        });

        // Text content (multi-line: split on newlines)
        let line_height = FORM_FONT_SIZE * 1.4;
        let mut char_count = 0;
        let mut cursor_line = 0;
        let mut cursor_col = 0;

        for (line_idx, line) in value.split('\n').enumerate() {
            let line_chars = line.chars().count();
            // Track which line/column the cursor is on
            if char_count + line_chars >= cursor_pos && cursor_col == 0 && line_idx >= cursor_line {
                cursor_line = line_idx;
                cursor_col = cursor_pos.saturating_sub(char_count);
            }
            char_count += line_chars + 1; // +1 for the newline character

            let y = self.bounds.y + INPUT_PAD + (line_idx as f32 + 1.0) * line_height;
            if y > self.bounds.y + self.bounds.height {
                break;
            }
            if !line.is_empty() {
                cmds.push(DisplayCommand::DrawText {
                    text: line.to_string(),
                    x: self.bounds.x + INPUT_PAD,
                    y,
                    size: FORM_FONT_SIZE,
                    color: INPUT_TEXT,
                    node_id,
                });
            }
        }

        // Cursor when focused
        if self.focused {
            let cursor_x = self.bounds.x + INPUT_PAD + cursor_col as f32 * FORM_FONT_SIZE * 0.6;
            let cursor_y = self.bounds.y + INPUT_PAD + cursor_line as f32 * line_height;
            cmds.push(DisplayCommand::FillRect {
                rect: Rect {
                    x: cursor_x,
                    y: cursor_y + 2.0,
                    width: 1.5,
                    height: line_height - 4.0,
                },
                color: INPUT_CURSOR,
                node_id,
            });
        }

        cmds
    }
}

// ── FormRegistry ────────────────────────────────────────────────────

/// Registry tracking all form elements on the current page.
///
/// Tier: T2-C (sequence + state + mapping)
///
/// Elements are stored in document order. The focused element index
/// is tracked separately for efficient access.
#[derive(Debug, Clone, Default)]
pub struct FormRegistry {
    /// All form elements in document order.
    elements: Vec<FormElement>,
    /// Index of the currently focused element, if any.
    focused_index: Option<usize>,
}

impl FormRegistry {
    /// Create a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a form element. Returns its index.
    pub fn register(&mut self, element: FormElement) -> usize {
        let idx = self.elements.len();
        self.elements.push(element);
        idx
    }

    /// Get the number of registered elements.
    #[must_use]
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Whether the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Get a reference to a form element by index.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&FormElement> {
        self.elements.get(index)
    }

    /// Get a mutable reference to a form element by index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut FormElement> {
        self.elements.get_mut(index)
    }

    /// Get the currently focused element, if any.
    #[must_use]
    pub fn focused(&self) -> Option<&FormElement> {
        self.focused_index.and_then(|idx| self.elements.get(idx))
    }

    /// Get a mutable reference to the currently focused element.
    pub fn focused_mut(&mut self) -> Option<&mut FormElement> {
        self.focused_index
            .and_then(|idx| self.elements.get_mut(idx))
    }

    /// Get the index of the currently focused element.
    #[must_use]
    pub fn focused_index(&self) -> Option<usize> {
        self.focused_index
    }

    /// Set focus to the element at the given index.
    /// Blurs the previously focused element.
    pub fn set_focus(&mut self, index: usize) {
        // Blur previous
        if let Some(prev) = self.focused_index {
            if let Some(elem) = self.elements.get_mut(prev) {
                elem.focused = false;
            }
        }
        // Focus new
        if let Some(elem) = self.elements.get_mut(index) {
            elem.focused = true;
            self.focused_index = Some(index);
        }
    }

    /// Remove focus from all elements.
    pub fn blur_all(&mut self) {
        if let Some(prev) = self.focused_index.take() {
            if let Some(elem) = self.elements.get_mut(prev) {
                elem.focused = false;
            }
        }
    }

    /// Move focus to the next focusable element (Tab key).
    /// Returns the index of the newly focused element, if any.
    pub fn focus_next(&mut self) -> Option<usize> {
        if self.elements.is_empty() {
            return None;
        }

        let start = self.focused_index.map_or(0, |i| i + 1);
        // Search from start to end, then wrap around
        let total = self.elements.len();
        for offset in 0..total {
            let idx = (start + offset) % total;
            // All form elements are focusable (buttons, checkboxes, inputs, textareas)
            self.set_focus(idx);
            return Some(idx);
        }
        None
    }

    /// Move focus to the previous focusable element (Shift+Tab).
    /// Returns the index of the newly focused element, if any.
    pub fn focus_prev(&mut self) -> Option<usize> {
        if self.elements.is_empty() {
            return None;
        }

        let total = self.elements.len();
        let start = self
            .focused_index
            .map_or(total - 1, |i| if i == 0 { total - 1 } else { i - 1 });

        for offset in 0..total {
            let idx = (start + total - offset) % total;
            self.set_focus(idx);
            return Some(idx);
        }
        None
    }

    /// Hit-test: find which form element, if any, contains the given point.
    /// Returns the element index.
    #[must_use]
    pub fn hit_test(&self, x: f32, y: f32) -> Option<usize> {
        // Reverse iterate so elements painted later (on top) are found first
        self.elements
            .iter()
            .enumerate()
            .rev()
            .find(|(_, elem)| elem.bounds.contains(x, y))
            .map(|(idx, _)| idx)
    }

    /// Update hover state for all elements based on mouse position.
    pub fn update_hover(&mut self, x: f32, y: f32) {
        for elem in &mut self.elements {
            elem.hovered = elem.bounds.contains(x, y);
        }
    }

    /// Clear all elements (e.g., on page navigation).
    pub fn clear(&mut self) {
        self.elements.clear();
        self.focused_index = None;
    }

    /// Build display commands for all registered form elements.
    #[must_use]
    pub fn build_display_commands(&self) -> Vec<DisplayCommand> {
        let mut cmds = Vec::new();
        for (idx, elem) in self.elements.iter().enumerate() {
            // Use the element index as a pseudo node_id for hit-testing
            cmds.extend(elem.build_display_commands(Some(idx)));
        }
        cmds
    }

    /// Iterator over all elements.
    pub fn iter(&self) -> impl Iterator<Item = &FormElement> {
        self.elements.iter()
    }

    /// Mutable iterator over all elements.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut FormElement> {
        self.elements.iter_mut()
    }
}

// ── DOM extraction ──────────────────────────────────────────────────

/// Extract form elements from an arena-based DOM tree.
///
/// Walks the DOM and creates `FormElement` entries for recognized
/// form tags: `input`, `button`, `textarea`, `select`.
///
/// Bounds are set to default sizes; the caller should update them
/// after layout computation.
#[must_use]
pub fn extract_form_elements(arena: &crate::dom::Arena) -> FormRegistry {
    let mut registry = FormRegistry::new();
    extract_recursive(arena, arena.root(), &mut registry);
    registry
}

/// Recursive DFS extraction of form elements from the arena.
fn extract_recursive(
    arena: &crate::dom::Arena,
    node_id: crate::dom::NodeId,
    registry: &mut FormRegistry,
) {
    if let Some(crate::dom::NodeKind::Element { tag, attrs }) = arena.kind(node_id) {
        match tag.as_str() {
            "input" => {
                let input_type = attrs.get("type").map(String::as_str).unwrap_or("text");
                let id = attrs.get("id").cloned();
                let name = attrs.get("name").cloned();

                match input_type {
                    "checkbox" => {
                        let checked = attrs.contains_key("checked");
                        let label = attrs.get("value").cloned().unwrap_or_default();
                        registry.register(FormElement::checkbox(id, name, checked, label));
                    }
                    "submit" | "button" => {
                        let label = attrs
                            .get("value")
                            .cloned()
                            .unwrap_or_else(|| "Submit".to_string());
                        registry.register(FormElement::button(id, name, label));
                    }
                    // "text", "password", "email", "search", "tel", "url", etc.
                    _ => {
                        let value = attrs.get("value").cloned().unwrap_or_default();
                        let placeholder = attrs.get("placeholder").cloned().unwrap_or_default();
                        registry.register(FormElement::text_input(id, name, value, placeholder));
                    }
                }
            }
            "button" => {
                let id = attrs.get("id").cloned();
                let name = attrs.get("name").cloned();
                // Button label from text children
                let label = collect_text_children(arena, node_id);
                let label = if label.is_empty() {
                    "Button".to_string()
                } else {
                    label
                };
                registry.register(FormElement::button(id, name, label));
            }
            "textarea" => {
                let id = attrs.get("id").cloned();
                let name = attrs.get("name").cloned();
                let rows: usize = attrs.get("rows").and_then(|r| r.parse().ok()).unwrap_or(3);
                let value = collect_text_children(arena, node_id);
                registry.register(FormElement::textarea(id, name, value, rows));
            }
            _ => {}
        }
    }

    // Recurse into children
    for &child in arena.children(node_id) {
        extract_recursive(arena, child, registry);
    }
}

/// Collect text from direct text-node children.
fn collect_text_children(arena: &crate::dom::Arena, node_id: crate::dom::NodeId) -> String {
    let mut text = String::new();
    for &child in arena.children(node_id) {
        if let Some(t) = arena.text(child) {
            let trimmed = t.trim();
            if !trimmed.is_empty() {
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(trimmed);
            }
        }
    }
    text
}

// ── Helpers ─────────────────────────────────────────────────────────

/// Convert a character offset to a byte offset in a string.
///
/// Returns `s.len()` if `char_offset` is past the end.
fn char_to_byte_offset(s: &str, char_offset: usize) -> usize {
    s.char_indices()
        .nth(char_offset)
        .map_or(s.len(), |(byte_idx, _)| byte_idx)
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // --- FormElement construction tests ---

    #[test]
    fn text_input_creation() {
        let elem = FormElement::text_input(
            Some("user".to_string()),
            Some("username".to_string()),
            String::new(),
            "Enter name".to_string(),
        );
        assert!(matches!(elem.kind, FormElementKind::TextInput { .. }));
        assert_eq!(elem.id.as_deref(), Some("user"));
        assert_eq!(elem.name.as_deref(), Some("username"));
        assert!(!elem.focused);
        assert!(elem.accepts_text_input());
    }

    #[test]
    fn button_creation() {
        let elem = FormElement::button(None, None, "Click Me".to_string());
        assert!(matches!(elem.kind, FormElementKind::Button { .. }));
        assert!(!elem.accepts_text_input());
        assert!(elem.bounds.width >= 80.0);
    }

    #[test]
    fn checkbox_creation() {
        let elem = FormElement::checkbox(None, None, false, "Accept".to_string());
        assert!(matches!(
            elem.kind,
            FormElementKind::Checkbox { checked: false, .. }
        ));
        assert!(!elem.accepts_text_input());
    }

    #[test]
    fn textarea_creation() {
        let elem = FormElement::textarea(None, None, "hello\nworld".to_string(), 5);
        if let FormElementKind::TextArea { rows, .. } = &elem.kind {
            assert_eq!(*rows, 5);
        } else {
            panic!("expected TextArea");
        }
        assert!(elem.accepts_text_input());
    }

    #[test]
    fn textarea_zero_rows_defaults_to_three() {
        let elem = FormElement::textarea(None, None, String::new(), 0);
        if let FormElementKind::TextArea { rows, .. } = &elem.kind {
            assert_eq!(*rows, 3);
        } else {
            panic!("expected TextArea");
        }
    }

    // --- Cursor movement tests ---

    #[test]
    fn insert_char_at_beginning() {
        let mut elem = FormElement::text_input(None, None, String::new(), String::new());
        elem.insert_char('a');
        elem.insert_char('b');
        assert_eq!(elem.text_value(), Some("ab"));
    }

    #[test]
    fn backspace_removes_last_char() {
        let mut elem = FormElement::text_input(None, None, "abc".to_string(), String::new());
        // Move cursor to end
        elem.cursor_end();
        elem.backspace();
        assert_eq!(elem.text_value(), Some("ab"));
    }

    #[test]
    fn backspace_at_beginning_does_nothing() {
        let mut elem = FormElement::text_input(None, None, "abc".to_string(), String::new());
        elem.cursor_home();
        elem.backspace();
        assert_eq!(elem.text_value(), Some("abc"));
    }

    #[test]
    fn cursor_movement_left_right() {
        let mut elem = FormElement::text_input(None, None, "hello".to_string(), String::new());
        elem.cursor_end(); // at position 5
        elem.cursor_left(); // at position 4
        elem.cursor_left(); // at position 3
        elem.insert_char('X');
        assert_eq!(elem.text_value(), Some("helXlo"));
    }

    #[test]
    fn cursor_left_at_zero_stays() {
        let mut elem = FormElement::text_input(None, None, "x".to_string(), String::new());
        elem.cursor_home();
        elem.cursor_left();
        // Cursor should still be at 0
        elem.insert_char('A');
        assert_eq!(elem.text_value(), Some("Ax"));
    }

    #[test]
    fn cursor_right_at_end_stays() {
        let mut elem = FormElement::text_input(None, None, "x".to_string(), String::new());
        elem.cursor_end();
        elem.cursor_right(); // should not advance past end
        elem.insert_char('B');
        assert_eq!(elem.text_value(), Some("xB"));
    }

    #[test]
    fn delete_forward_removes_char_after_cursor() {
        let mut elem = FormElement::text_input(None, None, "abc".to_string(), String::new());
        elem.cursor_home();
        elem.delete_forward();
        assert_eq!(elem.text_value(), Some("bc"));
    }

    #[test]
    fn delete_forward_at_end_does_nothing() {
        let mut elem = FormElement::text_input(None, None, "abc".to_string(), String::new());
        elem.cursor_end();
        elem.delete_forward();
        assert_eq!(elem.text_value(), Some("abc"));
    }

    #[test]
    fn cursor_home_and_end() {
        let mut elem = FormElement::text_input(None, None, "hello".to_string(), String::new());
        elem.cursor_end();
        elem.insert_char('!');
        assert_eq!(elem.text_value(), Some("hello!"));

        elem.cursor_home();
        elem.insert_char('>');
        assert_eq!(elem.text_value(), Some(">hello!"));
    }

    // --- Checkbox toggle tests ---

    #[test]
    fn toggle_checkbox() {
        let mut elem = FormElement::checkbox(None, None, false, "opt".to_string());
        let result = elem.toggle_checkbox();
        assert_eq!(result, Some(true));
        if let FormElementKind::Checkbox { checked, .. } = &elem.kind {
            assert!(*checked);
        }
        let result = elem.toggle_checkbox();
        assert_eq!(result, Some(false));
    }

    #[test]
    fn toggle_on_non_checkbox_returns_none() {
        let mut elem = FormElement::text_input(None, None, String::new(), String::new());
        assert_eq!(elem.toggle_checkbox(), None);
    }

    // --- Unicode cursor tests ---

    #[test]
    fn unicode_insert_and_cursor() {
        let mut elem = FormElement::text_input(None, None, String::new(), String::new());
        // Insert multi-byte characters
        elem.insert_char('a');
        elem.insert_char('\u{00e9}'); // e-acute (2 bytes in UTF-8)
        elem.insert_char('b');
        assert_eq!(elem.text_value(), Some("a\u{00e9}b"));

        // Cursor should be at char position 3
        elem.cursor_left();
        elem.backspace();
        assert_eq!(elem.text_value(), Some("ab"));
    }

    // --- FormRegistry tests ---

    #[test]
    fn registry_register_and_get() {
        let mut reg = FormRegistry::new();
        assert!(reg.is_empty());
        let idx = reg.register(FormElement::text_input(
            None,
            None,
            String::new(),
            String::new(),
        ));
        assert_eq!(idx, 0);
        assert_eq!(reg.len(), 1);
        assert!(!reg.is_empty());
        assert!(reg.get(0).is_some());
        assert!(reg.get(1).is_none());
    }

    #[test]
    fn registry_focus_and_blur() {
        let mut reg = FormRegistry::new();
        reg.register(FormElement::text_input(
            None,
            None,
            String::new(),
            String::new(),
        ));
        reg.register(FormElement::button(None, None, "OK".to_string()));

        assert!(reg.focused().is_none());

        reg.set_focus(0);
        assert_eq!(reg.focused_index(), Some(0));
        assert!(reg.focused().is_some());
        assert!(reg.get(0).map_or(false, |e| e.focused));
        assert!(reg.get(1).map_or(true, |e| !e.focused));

        reg.set_focus(1);
        assert_eq!(reg.focused_index(), Some(1));
        // Previous element should be blurred
        assert!(reg.get(0).map_or(true, |e| !e.focused));
        assert!(reg.get(1).map_or(false, |e| e.focused));

        reg.blur_all();
        assert!(reg.focused().is_none());
        assert!(reg.get(0).map_or(true, |e| !e.focused));
        assert!(reg.get(1).map_or(true, |e| !e.focused));
    }

    #[test]
    fn registry_focus_next_cycles() {
        let mut reg = FormRegistry::new();
        reg.register(FormElement::text_input(
            None,
            None,
            String::new(),
            String::new(),
        ));
        reg.register(FormElement::button(None, None, "A".to_string()));
        reg.register(FormElement::checkbox(None, None, false, "B".to_string()));

        // First Tab from no focus -> element 0
        assert_eq!(reg.focus_next(), Some(0));
        // Second Tab -> element 1
        assert_eq!(reg.focus_next(), Some(1));
        // Third Tab -> element 2
        assert_eq!(reg.focus_next(), Some(2));
        // Fourth Tab -> wraps to element 0
        assert_eq!(reg.focus_next(), Some(0));
    }

    #[test]
    fn registry_focus_prev_cycles() {
        let mut reg = FormRegistry::new();
        reg.register(FormElement::text_input(
            None,
            None,
            String::new(),
            String::new(),
        ));
        reg.register(FormElement::button(None, None, "A".to_string()));

        // Shift+Tab from no focus -> last element
        assert_eq!(reg.focus_prev(), Some(1));
        // Shift+Tab -> wraps to element 0
        assert_eq!(reg.focus_prev(), Some(0));
        // Shift+Tab from 0 -> wraps to element 1
        assert_eq!(reg.focus_prev(), Some(1));
    }

    #[test]
    fn registry_focus_next_empty_returns_none() {
        let mut reg = FormRegistry::new();
        assert_eq!(reg.focus_next(), None);
    }

    #[test]
    fn registry_hit_test() {
        let mut reg = FormRegistry::new();
        let mut elem1 = FormElement::text_input(None, None, String::new(), String::new());
        elem1.bounds = Rect {
            x: 10.0,
            y: 10.0,
            width: 100.0,
            height: 30.0,
        };
        let mut elem2 = FormElement::button(None, None, "OK".to_string());
        elem2.bounds = Rect {
            x: 10.0,
            y: 50.0,
            width: 80.0,
            height: 30.0,
        };
        reg.register(elem1);
        reg.register(elem2);

        assert_eq!(reg.hit_test(50.0, 20.0), Some(0)); // inside elem1
        assert_eq!(reg.hit_test(50.0, 60.0), Some(1)); // inside elem2
        assert_eq!(reg.hit_test(200.0, 200.0), None); // outside both
    }

    #[test]
    fn registry_update_hover() {
        let mut reg = FormRegistry::new();
        let mut elem = FormElement::button(None, None, "Hover".to_string());
        elem.bounds = Rect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 30.0,
        };
        reg.register(elem);

        reg.update_hover(50.0, 15.0);
        assert!(reg.get(0).map_or(false, |e| e.hovered));

        reg.update_hover(200.0, 200.0);
        assert!(reg.get(0).map_or(true, |e| !e.hovered));
    }

    #[test]
    fn registry_clear() {
        let mut reg = FormRegistry::new();
        reg.register(FormElement::text_input(
            None,
            None,
            String::new(),
            String::new(),
        ));
        reg.set_focus(0);
        assert_eq!(reg.len(), 1);
        assert!(reg.focused().is_some());

        reg.clear();
        assert!(reg.is_empty());
        assert!(reg.focused().is_none());
    }

    #[test]
    fn registry_display_commands() {
        let mut reg = FormRegistry::new();
        let mut elem = FormElement::text_input(None, None, "hello".to_string(), String::new());
        elem.bounds = Rect {
            x: 10.0,
            y: 10.0,
            width: 200.0,
            height: 28.0,
        };
        reg.register(elem);

        let cmds = reg.build_display_commands();
        // Should have at least: border rect + bg rect + text
        assert!(cmds.len() >= 3);
    }

    // --- char_to_byte_offset tests ---

    #[test]
    fn char_to_byte_ascii() {
        let s = "hello";
        assert_eq!(char_to_byte_offset(s, 0), 0);
        assert_eq!(char_to_byte_offset(s, 3), 3);
        assert_eq!(char_to_byte_offset(s, 5), 5); // past end returns len
    }

    #[test]
    fn char_to_byte_multibyte() {
        let s = "a\u{00e9}b"; // a (1 byte) + e-acute (2 bytes) + b (1 byte)
        assert_eq!(char_to_byte_offset(s, 0), 0); // 'a'
        assert_eq!(char_to_byte_offset(s, 1), 1); // e-acute starts at byte 1
        assert_eq!(char_to_byte_offset(s, 2), 3); // 'b' starts at byte 3
        assert_eq!(char_to_byte_offset(s, 3), 4); // past end
    }

    #[test]
    fn char_to_byte_empty() {
        assert_eq!(char_to_byte_offset("", 0), 0);
        assert_eq!(char_to_byte_offset("", 5), 0);
    }

    // --- DOM extraction tests ---

    #[test]
    fn extract_text_input_from_dom() {
        let arena = crate::dom::Arena::parse(
            r#"<html><body><input type="text" name="user" placeholder="Name"></body></html>"#,
        );
        let registry = extract_form_elements(&arena);
        assert_eq!(registry.len(), 1);
        let elem = registry.get(0);
        assert!(elem.is_some());
        let elem = elem.unwrap_or_else(|| unreachable!());
        assert!(matches!(elem.kind, FormElementKind::TextInput { .. }));
        assert_eq!(elem.name.as_deref(), Some("user"));
    }

    #[test]
    fn extract_button_from_dom() {
        let arena = crate::dom::Arena::parse(r#"<html><body><button>Click</button></body></html>"#);
        let registry = extract_form_elements(&arena);
        assert_eq!(registry.len(), 1);
        if let Some(elem) = registry.get(0) {
            assert!(matches!(elem.kind, FormElementKind::Button { .. }));
        }
    }

    #[test]
    fn extract_checkbox_from_dom() {
        let arena = crate::dom::Arena::parse(
            r#"<html><body><input type="checkbox" checked></body></html>"#,
        );
        let registry = extract_form_elements(&arena);
        assert_eq!(registry.len(), 1);
        if let Some(elem) = registry.get(0) {
            if let FormElementKind::Checkbox { checked, .. } = &elem.kind {
                assert!(*checked);
            }
        }
    }

    #[test]
    fn extract_textarea_from_dom() {
        let arena = crate::dom::Arena::parse(
            r#"<html><body><textarea rows="5">Initial text</textarea></body></html>"#,
        );
        let registry = extract_form_elements(&arena);
        assert_eq!(registry.len(), 1);
        if let Some(elem) = registry.get(0) {
            if let FormElementKind::TextArea { rows, value, .. } = &elem.kind {
                assert_eq!(*rows, 5);
                assert!(value.contains("Initial text"));
            }
        }
    }

    #[test]
    fn extract_submit_input_as_button() {
        let arena = crate::dom::Arena::parse(
            r#"<html><body><input type="submit" value="Go"></body></html>"#,
        );
        let registry = extract_form_elements(&arena);
        assert_eq!(registry.len(), 1);
        if let Some(elem) = registry.get(0) {
            assert!(matches!(elem.kind, FormElementKind::Button { .. }));
        }
    }

    #[test]
    fn extract_multiple_form_elements() {
        let arena = crate::dom::Arena::parse(
            r#"<html><body>
            <input type="text" name="user">
            <input type="password" name="pass">
            <input type="checkbox" name="remember">
            <button>Login</button>
            </body></html>"#,
        );
        let registry = extract_form_elements(&arena);
        assert_eq!(registry.len(), 4);
    }

    #[test]
    fn extract_no_form_elements() {
        let arena = crate::dom::Arena::parse(r#"<html><body><p>No forms here</p></body></html>"#);
        let registry = extract_form_elements(&arena);
        assert!(registry.is_empty());
    }

    // --- Display command generation tests ---

    #[test]
    fn text_input_generates_border_bg_text_commands() {
        let mut elem = FormElement::text_input(None, None, "hello".to_string(), String::new());
        elem.bounds = Rect {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 28.0,
        };
        let cmds = elem.build_display_commands(Some(0));
        // border + bg + text = 3
        assert_eq!(cmds.len(), 3);
    }

    #[test]
    fn focused_text_input_includes_cursor() {
        let mut elem = FormElement::text_input(None, None, "hello".to_string(), String::new());
        elem.bounds = Rect {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 28.0,
        };
        elem.focused = true;
        let cmds = elem.build_display_commands(Some(0));
        // border + bg + text + cursor = 4
        assert_eq!(cmds.len(), 4);
    }

    #[test]
    fn empty_unfocused_input_shows_placeholder() {
        let mut elem = FormElement::text_input(None, None, String::new(), "Enter text".to_string());
        elem.bounds = Rect {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 28.0,
        };
        let cmds = elem.build_display_commands(Some(0));
        // border + bg + placeholder text = 3
        assert_eq!(cmds.len(), 3);
        // Check that the placeholder text is rendered
        let has_placeholder = cmds.iter().any(|cmd| {
            if let DisplayCommand::DrawText { text, .. } = cmd {
                text == "Enter text"
            } else {
                false
            }
        });
        assert!(has_placeholder);
    }

    #[test]
    fn button_generates_border_bg_label() {
        let mut elem = FormElement::button(None, None, "Go".to_string());
        elem.bounds = Rect {
            x: 0.0,
            y: 0.0,
            width: 80.0,
            height: 32.0,
        };
        let cmds = elem.build_display_commands(Some(0));
        assert_eq!(cmds.len(), 3); // border + bg + label
    }

    #[test]
    fn checkbox_checked_has_checkmark_lines() {
        let mut elem = FormElement::checkbox(None, None, true, "Yes".to_string());
        elem.bounds = Rect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 28.0,
        };
        let cmds = elem.build_display_commands(Some(0));
        // border + bg + 2 stroke lines (check) + label = 5
        assert_eq!(cmds.len(), 5);
        let stroke_count = cmds
            .iter()
            .filter(|c| matches!(c, DisplayCommand::StrokeLine { .. }))
            .count();
        assert_eq!(stroke_count, 2);
    }

    #[test]
    fn checkbox_unchecked_no_checkmark() {
        let mut elem = FormElement::checkbox(None, None, false, "No".to_string());
        elem.bounds = Rect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 28.0,
        };
        let cmds = elem.build_display_commands(Some(0));
        // border + bg + label = 3
        assert_eq!(cmds.len(), 3);
    }
}
