use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::process::Command;

use crate::screens::{Screen, ScreenId, ScreenResult};
use crate::session::SessionContext;
use crate::style::*;

pub struct SystemPanel {
    title: String,
    command: String,
    lines: Vec<String>,
    scroll: usize,
    status: String,
    session: SessionContext,
}

impl SystemPanel {
    pub fn new(command: impl Into<String>, session: SessionContext) -> Self {
        let command = command.into();
        let mut screen = Self {
            title: command_title(&command),
            command,
            lines: Vec::new(),
            scroll: 0,
            status: String::new(),
            session,
        };
        screen.refresh();
        screen
    }

    fn refresh(&mut self) {
        let state = self.session.snapshot();
        match Command::new("l400cmd")
            .args(self.command.split_whitespace())
            .env("L400_USER", &state.user_profile)
            .env("L400_CURLIB", &state.current_library)
            .env("L400_LIBLIST", state.library_list.join(":"))
            .output()
        {
            Ok(output) => {
                self.lines = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .chain(String::from_utf8_lossy(&output.stderr).lines())
                    .map(str::to_string)
                    .collect();
                if self.lines.is_empty() {
                    self.lines
                        .push("Sin datos de runtime para mostrar.".to_string());
                }
                self.status = format!(
                    "{} finalizo con estado {}.",
                    self.command,
                    output.status.code().unwrap_or_default()
                );
            }
            Err(error) => {
                self.lines = vec![format!("No se pudo ejecutar '{}': {}", self.command, error)];
                self.status = "Runtime no disponible.".to_string();
            }
        }
        self.scroll = self.scroll.min(self.lines.len().saturating_sub(1));
    }
}

impl Screen for SystemPanel {
    fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(frame.area());

        self.render_header(frame, chunks[0]);
        self.render_body(frame, chunks[1]);
        self.render_status(frame, chunks[2]);
        self.render_help(frame, chunks[3]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> ScreenResult {
        match key.code {
            KeyCode::F(3) | KeyCode::F(12) => ScreenResult::goto(ScreenId::MainMenu),
            KeyCode::F(4) => ScreenResult::goto(ScreenId::CommandLine),
            KeyCode::F(5) => {
                self.refresh();
                ScreenResult::none()
            }
            KeyCode::Up => {
                self.scroll = self.scroll.saturating_sub(1);
                ScreenResult::none()
            }
            KeyCode::Down => {
                self.scroll = self
                    .scroll
                    .saturating_add(1)
                    .min(self.lines.len().saturating_sub(1));
                ScreenResult::none()
            }
            KeyCode::PageUp => {
                self.scroll = self.scroll.saturating_sub(10);
                ScreenResult::none()
            }
            KeyCode::PageDown => {
                self.scroll = self
                    .scroll
                    .saturating_add(10)
                    .min(self.lines.len().saturating_sub(1));
                ScreenResult::none()
            }
            _ => ScreenResult::none(),
        }
    }
}

impl SystemPanel {
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(format!(" {} ", self.title))
            .style(STYLE_HEADER)
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(
            Paragraph::new(format!("Command: {}", self.command)).style(STYLE_NORMAL),
            inner,
        );
    }

    fn render_body(&self, frame: &mut Frame, area: Rect) {
        let height = area.height.saturating_sub(2) as usize;
        let text = self
            .lines
            .iter()
            .skip(self.scroll)
            .take(height)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(Paragraph::new(text).style(STYLE_NORMAL), inner);
    }

    fn render_status(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(
            Paragraph::new(self.status.clone()).style(STYLE_NORMAL),
            inner,
        );
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let help_text = Line::from(vec![
            "F3=Exit   ".into(),
            "F4=Prompt   ".into(),
            "F5=Refresh   ".into(),
            "PgUp/PgDn=Roll   ".into(),
            "F12=Cancel".into(),
        ]);
        let block = Block::default()
            .style(STYLE_HELP)
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(Paragraph::new(help_text).style(STYLE_HELP), inner);
    }
}

fn command_title(command: &str) -> String {
    command
        .split_whitespace()
        .next()
        .unwrap_or("DISPLAY")
        .to_string()
}
