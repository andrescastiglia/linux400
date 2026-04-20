use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::Line,
    text::Text,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::screens::{Screen, ScreenId, ScreenResult};
use crate::style::*;

pub struct CommandLine {
    command: String,
    history: Vec<String>,
    history_index: usize,
    cursor_position: usize,
    output: Vec<String>,
    show_output: bool,
}

impl CommandLine {
    pub fn new() -> Self {
        Self {
            command: String::new(),
            history: vec![
                "WRKACTJOB".to_string(),
                "WRKOBJ".to_string(),
                "DSPDTAQ QUSRSYS QEZJOBLOG".to_string(),
            ],
            history_index: 0,
            cursor_position: 0,
            output: Vec::new(),
            show_output: false,
        }
    }

    fn execute_command(&mut self) {
        let cmd = self.command.trim();
        if cmd.is_empty() {
            return;
        }

        self.output.clear();
        self.output.push(format!("CMD: {}", cmd));
        self.output.push("".to_string());

        match cmd {
            "WRKACTJOB" => {
                self.output.push("Display Job Activity".to_string());
                self.output.push("Use option 4 to select a job".to_string());
            }
            "WRKOBJ" => {
                self.output.push("Work with Objects".to_string());
                self.output.push("Use F18 to change library".to_string());
            }
            cmd if cmd.starts_with("DSPDTAQ") => {
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                if parts.len() >= 3 {
                    self.output
                        .push(format!("Display Data Queue: {}/{}", parts[1], parts[2]));
                } else {
                    self.output
                        .push("Usage: DSPDTAQ LIBRARY DTAQNAME".to_string());
                }
            }
            "HELP" => {
                self.output.push("Available commands:".to_string());
                self.output
                    .push("  WRKACTJOB - Work with active jobs".to_string());
                self.output
                    .push("  WRKOBJ    - Work with objects".to_string());
                self.output
                    .push("  DSPDTAQ   - Display data queue".to_string());
                self.output.push("  CALL PGM   - Call program".to_string());
                self.output
                    .push("  DSPSYSVAL - Display system value".to_string());
            }
            _ => {
                self.output
                    .push(format!("Command '{}' executed successfully", cmd));
            }
        }

        if !self.history.iter().any(|h| h == cmd) {
            self.history.insert(0, cmd.to_string());
        }

        self.show_output = true;
        self.command.clear();
        self.cursor_position = 0;
    }
}

impl Screen for CommandLine {
    fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(frame.size());

        self.render_command_line(frame, chunks[0]);
        if self.show_output {
            self.render_output(frame, chunks[1]);
        }
        self.render_help(frame, chunks[2]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> ScreenResult {
        if self.show_output {
            match key.code {
                KeyCode::F(3) => return ScreenResult::goto(ScreenId::MainMenu),
                KeyCode::Enter | KeyCode::Esc => {
                    self.show_output = false;
                    return ScreenResult::none();
                }
                _ => return ScreenResult::none(),
            }
        }

        match key.code {
            KeyCode::F(3) => ScreenResult::goto(ScreenId::MainMenu),
            KeyCode::F(12) => ScreenResult::goto(ScreenId::MainMenu),
            KeyCode::Enter => {
                self.execute_command();
                ScreenResult::none()
            }
            KeyCode::Backspace => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                    self.command.remove(self.cursor_position);
                }
                ScreenResult::none()
            }
            KeyCode::Delete => {
                if self.cursor_position < self.command.len() {
                    self.command.remove(self.cursor_position);
                }
                ScreenResult::none()
            }
            KeyCode::Left => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
                ScreenResult::none()
            }
            KeyCode::Right => {
                if self.cursor_position < self.command.len() {
                    self.cursor_position += 1;
                }
                ScreenResult::none()
            }
            KeyCode::Home => {
                self.cursor_position = 0;
                ScreenResult::none()
            }
            KeyCode::End => {
                self.cursor_position = self.command.len();
                ScreenResult::none()
            }
            KeyCode::Up => {
                if self.history_index < self.history.len().saturating_sub(1) {
                    self.history_index += 1;
                    self.command = self.history[self.history_index].clone();
                    self.cursor_position = self.command.len();
                }
                ScreenResult::none()
            }
            KeyCode::Down => {
                if self.history_index > 0 {
                    self.history_index -= 1;
                    self.command = self.history[self.history_index].clone();
                    self.cursor_position = self.command.len();
                } else {
                    self.history_index = 0;
                    self.command.clear();
                    self.cursor_position = 0;
                }
                ScreenResult::none()
            }
            KeyCode::Char(c) => {
                self.command.insert(self.cursor_position, c);
                self.cursor_position += 1;
                ScreenResult::none()
            }
            _ => ScreenResult::none(),
        }
    }
}

impl CommandLine {
    fn render_command_line(&self, frame: &mut Frame, area: Rect) {
        let display = format!("> {}", self.command);

        let block = Block::default()
            .style(STYLE_HEADER)
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);

        frame.render_widget(block, area);

        let text = Paragraph::new(display.as_str()).style(STYLE_NORMAL);
        let inner = Rect::new(area.x + 1, area.y, area.width - 2, 1);
        frame.render_widget(text, inner);

        let cursor_x = self.cursor_position + 2;
        if cursor_x < area.width as usize - 1 {
            frame.set_cursor(area.x + cursor_x as u16, area.y);
        }
    }

    fn render_output(&self, frame: &mut Frame, area: Rect) {
        let text: Text = self
            .output
            .iter()
            .map(|line| Line::from(line.clone()))
            .collect();

        let block = Block::default()
            .title(" Command Output ")
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);

        frame.render_widget(block, area);

        let inner = Rect::new(area.x + 1, area.y + 1, area.width - 2, area.height - 2);
        frame.render_widget(Paragraph::new(text).style(STYLE_NORMAL), inner);
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let help_text = Line::from(vec![
            "F3=Exit   ".into(),
            "F12=Cancel   ".into(),
            "Enter=Execute   ".into(),
            "Up/Down=History".into(),
        ]);

        let block = Block::default()
            .style(STYLE_HELP)
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);

        frame.render_widget(block, area);

        let inner = Rect::new(area.x + 1, area.y + 1, area.width - 2, 1);
        frame.render_widget(Paragraph::new(help_text).style(STYLE_HELP), inner);
    }
}

impl Default for CommandLine {
    fn default() -> Self {
        Self::new()
    }
}
