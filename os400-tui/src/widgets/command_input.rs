use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::style::*;

/// An embeddable single-line command input with Tab-autocompletion from `*CMD` metadata.
///
/// Can be placed at the bottom of any screen to allow the operator to type
/// commands without leaving the current context.
pub struct CommandInput {
    /// Current input text.
    pub value: String,
    /// Cursor position (byte offset).
    pub cursor: usize,
    /// Whether the widget is currently focused / active.
    pub active: bool,
    /// Autocompletion candidates (populated on Tab).
    candidates: Vec<String>,
    /// Index into candidates cycle.
    candidate_index: usize,
}

impl CommandInput {
    pub fn new() -> Self {
        Self {
            value: String::new(),
            cursor: 0,
            active: false,
            candidates: Vec::new(),
            candidate_index: 0,
        }
    }

    /// Handle a key event. Returns `Some(command)` when Enter is pressed
    /// with a non-empty command. Returns `None` otherwise.
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<String> {
        match key.code {
            KeyCode::Enter => {
                let cmd = self.value.trim().to_string();
                if cmd.is_empty() {
                    return None;
                }
                self.value.clear();
                self.cursor = 0;
                self.candidates.clear();
                Some(cmd)
            }
            KeyCode::Char(c) => {
                self.candidates.clear();
                let c = c.to_ascii_uppercase();
                self.value.insert(self.cursor, c);
                self.cursor += 1;
                None
            }
            KeyCode::Backspace => {
                self.candidates.clear();
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.value.remove(self.cursor);
                }
                None
            }
            KeyCode::Delete => {
                self.candidates.clear();
                if self.cursor < self.value.len() {
                    self.value.remove(self.cursor);
                }
                None
            }
            KeyCode::Left => {
                self.cursor = self.cursor.saturating_sub(1);
                None
            }
            KeyCode::Right => {
                self.cursor = self.cursor.saturating_add(1).min(self.value.len());
                None
            }
            KeyCode::Home => {
                self.cursor = 0;
                None
            }
            KeyCode::End => {
                self.cursor = self.value.len();
                None
            }
            KeyCode::Tab => {
                self.autocomplete();
                None
            }
            _ => None,
        }
    }

    /// Cycle through autocompletion candidates from `COMMAND_METADATA`.
    fn autocomplete(&mut self) {
        if self.candidates.is_empty() {
            let prefix = self.value.trim().to_uppercase();
            if prefix.is_empty() {
                return;
            }
            self.candidates = l400::COMMAND_METADATA
                .iter()
                .filter(|meta| meta.name.starts_with(&prefix))
                .map(|meta| meta.name.to_string())
                .collect();
            self.candidate_index = 0;
        } else {
            self.candidate_index = (self.candidate_index + 1) % self.candidates.len();
        }

        if let Some(candidate) = self.candidates.get(self.candidate_index) {
            self.value = candidate.clone();
            self.cursor = self.value.len();
        }
    }

    /// Clear the input.
    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
        self.candidates.clear();
    }

    /// Render the command input as a single-line prompt.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let prompt_style = if self.active {
            STYLE_INPUT_ACTIVE
        } else {
            STYLE_INPUT_PROTECTED
        };

        let width = area.width as usize;
        let prefix = "===> ";
        let prefix_len = prefix.len();
        let available = width.saturating_sub(prefix_len);
        let display = if self.value.len() > available {
            &self.value[self.value.len() - available..]
        } else {
            &self.value
        };
        let padding = available.saturating_sub(display.len());

        let spans = vec![
            Span::styled(prefix, STYLE_NORMAL),
            Span::styled(format!("{}{}", display, " ".repeat(padding)), prompt_style),
        ];

        frame.render_widget(Paragraph::new(Line::from(spans)), area);

        if self.active {
            let cursor_x = area.x + prefix_len as u16 + self.cursor.min(available) as u16;
            if cursor_x < area.x + area.width {
                frame.set_cursor_position((cursor_x, area.y));
            }
        }
    }
}

impl Default for CommandInput {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn enter_returns_command_and_clears() {
        let mut input = CommandInput::new();
        input.value = "WRKOBJ".to_string();
        input.cursor = 6;
        let result = input.handle_key(key(KeyCode::Enter));
        assert_eq!(result.as_deref(), Some("WRKOBJ"));
        assert!(input.value.is_empty());
        assert_eq!(input.cursor, 0);
    }

    #[test]
    fn enter_on_empty_returns_none() {
        let mut input = CommandInput::new();
        assert_eq!(input.handle_key(key(KeyCode::Enter)), None);
    }

    #[test]
    fn typing_uppercases_automatically() {
        let mut input = CommandInput::new();
        input.handle_key(key(KeyCode::Char('w')));
        input.handle_key(key(KeyCode::Char('r')));
        input.handle_key(key(KeyCode::Char('k')));
        assert_eq!(input.value, "WRK");
    }

    #[test]
    fn backspace_deletes_before_cursor() {
        let mut input = CommandInput::new();
        input.value = "ABC".to_string();
        input.cursor = 3;
        input.handle_key(key(KeyCode::Backspace));
        assert_eq!(input.value, "AB");
    }

    #[test]
    fn tab_autocompletes_from_metadata() {
        let mut input = CommandInput::new();
        input.value = "WRKOBJ".to_string();
        input.cursor = 6;
        input.handle_key(key(KeyCode::Tab));
        // Should match at least WRKOBJ from COMMAND_METADATA
        assert!(input.value.starts_with("WRKOBJ"));
    }
}
