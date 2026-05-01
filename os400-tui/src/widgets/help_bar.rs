use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::style::*;

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
        frame.render_widget(Paragraph::new(self.line()).style(STYLE_HELP), inner);
    }

    fn line(&self) -> Line<'static> {
        let mut spans = Vec::new();
        for action in &self.actions {
            spans.push(Span::raw(format!("{}={}   ", action.key, action.label)));
        }

        if let Some(command) = self.command.as_deref()
            && let Some(metadata) = l400::command_metadata(command)
        {
            spans.push(Span::raw(format!("{}: {}", metadata.name, metadata.text)));
        }

        Line::from(spans)
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
        Self {
            id,
            text: text.into(),
            error: false,
        }
    }

    pub fn error(id: &'static str, text: impl Into<String>) -> Self {
        Self {
            id,
            text: text.into(),
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
            Paragraph::new(format!("{} {}", self.id, self.text))
                .style(style)
                .wrap(ratatui::widgets::Wrap { trim: true }),
            inner,
        );
    }
}
