use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders, Paragraph, Widget},
};
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use crate::style::*;
use crate::widgets::{ellipsize, sanitize_runtime_message};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HelpAction {
    pub key: &'static str,
    pub label: &'static str,
}

impl HelpAction {
    pub const fn new(key: &'static str, label: &'static str) -> Self {
        Self { key, label }
    }
}

pub struct HelpBar {
    command: Option<String>,
    actions: Vec<HelpAction>,
}

impl HelpBar {
    pub fn new() -> Self {
        Self {
            command: None,
            actions: Vec::new(),
        }
    }

    pub fn command(mut self, command: impl Into<String>) -> Self {
        self.command = Some(command.into());
        self
    }

    pub fn actions(mut self, actions: impl Into<Vec<HelpAction>>) -> Self {
        self.actions = actions.into();
        self
    }

    pub fn render(self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .style(STYLE_HELP)
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(
            Paragraph::new(ellipsize(self.line_string(), inner.width as usize)).style(STYLE_HELP),
            inner,
        );
    }

    fn line_string(&self) -> String {
        let mut text = String::new();
        for action in &self.actions {
            text.push_str(&format!("{}={}   ", action.key, action.label));
        }

        if let Some(command) = self.command.as_deref()
            && let Some(metadata) = l400::command_metadata(command)
        {
            text.push_str(&format!("{}: {}", metadata.name, metadata.text));
        }

        text
    }
}

impl Default for HelpBar {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for HelpBar {
    fn render(self, _area: Rect, _buf: &mut ratatui::buffer::Buffer) {
        // Screen-level rendering uses HelpBar::render so it can draw a bordered area.
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CpfMessage {
    pub id: &'static str,
    pub text: String,
    pub error: bool,
}

impl CpfMessage {
    pub fn info(id: &'static str, text: impl Into<String>) -> Self {
        let text = text.into();
        log_cpf_once(id, &text);
        Self {
            id,
            text,
            error: false,
        }
    }

    pub fn error(id: &'static str, text: impl Into<String>) -> Self {
        let text = text.into();
        log_cpf_once(id, &text);
        Self {
            id,
            text,
            error: true,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let style = if self.error {
            STYLE_ERROR
        } else {
            STYLE_NORMAL
        };
        frame.render_widget(
            Paragraph::new(ellipsize(
                format!("{} {}", self.id, sanitize_runtime_message(&self.text)),
                inner.width as usize,
            ))
            .style(style)
            .wrap(ratatui::widgets::Wrap { trim: true }),
            inner,
        );
    }
}

fn log_cpf_once(id: &str, text: &str) {
    static SEEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let key = format!("{id}:{}", sanitize_runtime_message(text));
    let seen = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
    if let Ok(mut seen) = seen.lock()
        && seen.insert(key)
    {
        let _ = l400::audit_event("TUI_CPF", "TUI", "os400-tui", &format!("{id} {text}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_bar_line_has_predictable_order() {
        let line = HelpBar::new()
            .command("WRKOBJ")
            .actions(vec![
                HelpAction::new("F3", "Back"),
                HelpAction::new("F5", "Refresh"),
            ])
            .line_string();
        assert!(line.starts_with("F3=Back   F5=Refresh"));
    }
}
