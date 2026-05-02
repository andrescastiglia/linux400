use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::style::*;

/// A text input field with visible length, cursor position, and optional masking.
///
/// Models a 5250-style input field: fixed visible width, active/inactive styles,
/// uppercase auto-conversion, and password masking.
#[derive(Clone, Debug)]
pub struct InputField {
    /// Current text value.
    pub value: String,
    /// Cursor position within the value (byte offset, always kept on a UTF-8 boundary).
    pub cursor: usize,
    /// Label displayed to the left of the field.
    pub label: String,
    /// Maximum visible width of the input area.
    pub width: u16,
    /// Whether this field is currently focused.
    pub active: bool,
    /// Whether to mask input (for passwords).
    pub masked: bool,
    /// Whether to auto-uppercase typed characters.
    pub uppercase: bool,
    /// Whether the field is required.
    pub required: bool,
}

impl InputField {
    pub fn new(label: impl Into<String>, width: u16) -> Self {
        Self {
            value: String::new(),
            cursor: 0,
            label: label.into(),
            width,
            active: false,
            masked: false,
            uppercase: false,
            required: false,
        }
    }

    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self.cursor = self.value.len();
        self
    }

    pub fn masked(mut self) -> Self {
        self.masked = true;
        self
    }

    pub fn uppercase(mut self) -> Self {
        self.uppercase = true;
        self
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Insert a character at the cursor position.
    pub fn insert_char(&mut self, c: char) {
        let c = if self.uppercase {
            c.to_ascii_uppercase()
        } else {
            c
        };
        if self.cursor <= self.value.len() {
            self.value.insert(self.cursor, c);
            self.cursor += c.len_utf8();
        }
    }

    /// Delete the character before the cursor (backspace).
    pub fn delete_back(&mut self) {
        if self.cursor > 0 {
            self.cursor = prev_boundary(&self.value, self.cursor);
            self.value.remove(self.cursor);
        }
    }

    /// Delete the character at the cursor (delete key).
    pub fn delete_forward(&mut self) {
        if self.cursor < self.value.len() {
            self.value.remove(self.cursor);
        }
    }

    /// Move cursor left by one character.
    pub fn move_left(&mut self) {
        self.cursor = prev_boundary(&self.value, self.cursor);
    }

    /// Move cursor right by one character.
    pub fn move_right(&mut self) {
        self.cursor = next_boundary(&self.value, self.cursor);
    }

    /// Move cursor to the beginning.
    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    /// Move cursor to the end.
    pub fn move_end(&mut self) {
        self.cursor = self.value.len();
    }

    /// Clear the field.
    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
    }

    /// The display text, masked or raw.
    fn display_text(&self) -> String {
        if self.masked {
            "*".repeat(self.value.chars().count())
        } else {
            self.value.clone()
        }
    }

    /// Pad display text to the visible width.
    fn padded_display(&self) -> String {
        let text = self.display_text();
        let char_count = text.chars().count();
        let width = self.width as usize;
        if char_count >= width {
            text.chars().take(width).collect()
        } else {
            format!("{}{}", text, " ".repeat(width - char_count))
        }
    }

    /// Render the field at the given area.
    ///
    /// The area should be a single row. The field renders as:
    /// `Label . . . :   [value          ]`
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let field_style = if self.active {
            STYLE_INPUT_ACTIVE
        } else {
            STYLE_INPUT_PROTECTED
        };

        let label_width = self.label.chars().count() + 3; // label + " : "
        let spans = vec![
            Span::styled(format!("{} : ", self.label), STYLE_NORMAL),
            Span::styled(self.padded_display(), field_style),
            if self.required {
                Span::styled("  *REQ", STYLE_WARNING)
            } else {
                Span::raw("")
            },
        ];

        frame.render_widget(Paragraph::new(Line::from(spans)), area);

        // Set cursor position when active.
        if self.active {
            let cursor_x =
                area.x + label_width as u16 + self.value[..self.cursor].chars().count() as u16;
            if cursor_x < area.x + area.width {
                frame.set_cursor_position((cursor_x, area.y));
            }
        }
    }
}

fn prev_boundary(value: &str, cursor: usize) -> usize {
    value[..cursor]
        .char_indices()
        .last()
        .map(|(index, _)| index)
        .unwrap_or(0)
}

fn next_boundary(value: &str, cursor: usize) -> usize {
    value[cursor..]
        .char_indices()
        .nth(1)
        .map(|(index, _)| cursor + index)
        .unwrap_or(value.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_cursor_movement() {
        let mut field = InputField::new("User", 16);
        field.insert_char('a');
        field.insert_char('b');
        assert_eq!(field.value, "ab");
        assert_eq!(field.cursor, 2);

        field.move_left();
        assert_eq!(field.cursor, 1);

        field.insert_char('X');
        assert_eq!(field.value, "aXb");
        assert_eq!(field.cursor, 2);
    }

    #[test]
    fn uppercase_auto_converts() {
        let mut field = InputField::new("Name", 10).uppercase();
        field.insert_char('q');
        field.insert_char('p');
        assert_eq!(field.value, "QP");
    }

    #[test]
    fn masked_display() {
        let mut field = InputField::new("Password", 10).masked();
        field.value = "secret".to_string();
        assert_eq!(field.display_text(), "******");
    }

    #[test]
    fn delete_back_and_forward() {
        let mut field = InputField::new("F", 10).with_value("hello");
        field.delete_back(); // removes 'o'
        assert_eq!(field.value, "hell");

        field.cursor = 0;
        field.delete_forward(); // removes 'h'
        assert_eq!(field.value, "ell");
    }

    #[test]
    fn unicode_backspace_uses_utf8_boundaries() {
        let mut field = InputField::new("Name", 10);
        field.insert_char('ñ');
        field.insert_char('A');
        field.move_left();
        field.delete_back();
        assert_eq!(field.value, "A");
        assert!(field.value.is_char_boundary(field.cursor));
    }

    #[test]
    fn clear_resets_field() {
        let mut field = InputField::new("F", 10).with_value("data");
        field.clear();
        assert_eq!(field.value, "");
        assert_eq!(field.cursor, 0);
    }

    #[test]
    fn home_moves_cursor_to_beginning() {
        let mut field = InputField::new("F", 10).with_value("hello");
        field.cursor = 3;
        field.move_home();
        assert_eq!(field.cursor, 0);
    }

    #[test]
    fn end_moves_cursor_to_end() {
        let mut field = InputField::new("F", 10).with_value("hello");
        field.cursor = 2;
        field.move_end();
        assert_eq!(field.cursor, 5);
    }

    #[test]
    fn left_arrow_moves_cursor_left() {
        let mut field = InputField::new("F", 10).with_value("hello");
        field.cursor = 3;
        field.move_left();
        assert_eq!(field.cursor, 2);
    }

    #[test]
    fn right_arrow_moves_cursor_right() {
        let mut field = InputField::new("F", 10).with_value("hello");
        field.cursor = 2;
        field.move_right();
        assert_eq!(field.cursor, 3);
    }

    #[test]
    fn left_arrow_at_beginning_stays() {
        let mut field = InputField::new("F", 10).with_value("hello");
        field.cursor = 0;
        field.move_left();
        assert_eq!(field.cursor, 0);
    }

    #[test]
    fn right_arrow_at_end_stays() {
        let mut field = InputField::new("F", 10).with_value("hello");
        field.cursor = 5;
        field.move_right();
        assert_eq!(field.cursor, 5);
    }

    #[test]
    fn backspace_at_beginning_does_nothing() {
        let mut field = InputField::new("F", 10).with_value("hello");
        field.cursor = 0;
        field.delete_back();
        assert_eq!(field.value, "hello");
        assert_eq!(field.cursor, 0);
    }

    #[test]
    fn delete_at_end_does_nothing() {
        let mut field = InputField::new("F", 10).with_value("hello");
        field.cursor = 5;
        field.delete_forward();
        assert_eq!(field.value, "hello");
    }

    #[test]
    fn insert_char_in_middle() {
        let mut field = InputField::new("F", 10).with_value("hello");
        field.cursor = 2;
        field.insert_char('X');
        assert_eq!(field.value, "heXllo");
        assert_eq!(field.cursor, 3);
    }

    #[test]
    fn uppercase_field_converts_to_uppercase() {
        let mut field = InputField::new("F", 10).uppercase().with_value("hello");
        field.cursor = 5;
        field.insert_char('w');
        assert_eq!(field.value, "helloW");
        assert_eq!(field.cursor, 6);
    }
}
