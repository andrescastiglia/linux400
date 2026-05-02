use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::style::*;

/// A modal confirmation dialog for destructive actions.
///
/// Replaces the ad-hoc `pending_delete`/`pending_action` patterns found
/// in ObjectBrowser, WorkManagement, and AdminCommandView with a single
/// reusable widget.
#[derive(Clone, Debug)]
pub struct ConfirmDialog {
    title: String,
    message: String,
    confirmed: Option<bool>,
}

impl ConfirmDialog {
    /// Create a new confirmation dialog.
    pub fn new(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            confirmed: None,
        }
    }

    /// Handle a key event. Returns `true` if the dialog consumed the event.
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Enter => {
                self.confirmed = Some(true);
                true
            }
            KeyCode::F(12) | KeyCode::Esc => {
                self.confirmed = Some(false);
                true
            }
            _ => true, // absorb all keys while dialog is open
        }
    }

    /// Returns `Some(true)` if confirmed, `Some(false)` if cancelled,
    /// `None` if still waiting for input.
    pub fn result(&self) -> Option<bool> {
        self.confirmed
    }

    /// Whether the dialog was confirmed.
    pub fn is_confirmed(&self) -> bool {
        self.confirmed == Some(true)
    }

    /// Whether the dialog was cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.confirmed == Some(false)
    }

    /// Whether the dialog is still waiting for input.
    pub fn is_pending(&self) -> bool {
        self.confirmed.is_none()
    }

    /// Render the dialog as a centered popup over the given area.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let width = 50.min(area.width.saturating_sub(4));
        let height = 7.min(area.height.saturating_sub(2));

        let popup = centered_rect(width, height, area);
        frame.render_widget(Clear, popup);

        let block = Block::default()
            .title(format!(" {} ", self.title))
            .style(STYLE_HEADER)
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);

        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(2), Constraint::Length(1)])
            .split(inner);

        frame.render_widget(
            Paragraph::new(self.message.clone())
                .style(STYLE_WARNING)
                .alignment(Alignment::Center),
            chunks[0],
        );

        frame.render_widget(
            Paragraph::new("Enter=Confirm   F12=Cancel")
                .style(STYLE_HELP)
                .alignment(Alignment::Center),
            chunks[1],
        );
    }
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(width),
            Constraint::Fill(1),
        ])
        .split(area);

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(height),
            Constraint::Fill(1),
        ])
        .split(horizontal[1]);

    vertical[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::from(code)
    }

    #[test]
    fn enter_confirms() {
        let mut dialog = ConfirmDialog::new("Delete", "Delete QGPL/TEST?");
        assert!(dialog.is_pending());
        assert!(dialog.handle_key(key(KeyCode::Enter)));
        assert!(dialog.is_confirmed());
    }

    #[test]
    fn f12_cancels() {
        let mut dialog = ConfirmDialog::new("Delete", "Delete?");
        assert!(dialog.handle_key(key(KeyCode::F(12))));
        assert!(dialog.is_cancelled());
    }

    #[test]
    fn esc_cancels() {
        let mut dialog = ConfirmDialog::new("Delete", "Delete?");
        assert!(dialog.handle_key(key(KeyCode::Esc)));
        assert!(dialog.is_cancelled());
    }

    #[test]
    fn other_keys_are_absorbed() {
        let mut dialog = ConfirmDialog::new("Test", "Sure?");
        assert!(dialog.handle_key(key(KeyCode::Char('a'))));
        assert!(dialog.is_pending()); // no decision yet
    }
}
