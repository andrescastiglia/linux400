use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::Line,
    text::Text,
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::process::Command;

use crate::screens::{Screen, ScreenId, ScreenResult};
use crate::session::SessionContext;
use crate::style::*;

pub struct CommandLine {
    command: String,
    history: Vec<String>,
    history_index: usize,
    cursor_position: usize,
    output: Vec<String>,
    show_output: bool,
    session: SessionContext,
}

impl CommandLine {
    pub fn new() -> Self {
        Self::with_session(SessionContext::new(std::process::id() as u64))
    }

    pub fn with_session(session: SessionContext) -> Self {
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
            session,
        }
    }

    fn execute_command(&mut self) -> ScreenResult {
        let cmd = self.command.trim().to_string();
        if cmd.is_empty() {
            return ScreenResult::none();
        }

        if let Some(route) = self.route_interactive_command(&cmd) {
            if !self.history.iter().any(|h| h == &cmd) {
                self.history.insert(0, cmd.clone());
            }
            self.command.clear();
            self.cursor_position = 0;
            self.history_index = 0;
            return route;
        }

        self.output.clear();
        self.output.push(format!("CMD: {}", cmd));
        self.output.push("".to_string());

        if self.handle_session_command(&cmd) {
            if !self.history.iter().any(|h| h == &cmd) {
                self.history.insert(0, cmd.clone());
            }
            self.show_output = true;
            self.command.clear();
            self.cursor_position = 0;
            self.history_index = 0;
            return ScreenResult::none();
        }

        match cmd.as_str() {
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
                self.output
                    .push("  STRPDM    - Programming Development Manager".to_string());
                self.output
                    .push("  WRKMBRPDM - Work with source members".to_string());
                self.output
                    .push("  STRSEU    - Edit a source member".to_string());
                self.output
                    .push("  STRSQL    - Interactive SQL".to_string());
                self.output.push("  CALL PGM   - Call program".to_string());
                self.output
                    .push("  DSPSYSVAL - Display system value".to_string());
            }
            _ => {
                let state = self.session.snapshot();
                match Command::new("l400cmd")
                    .args(cmd.split_whitespace())
                    .env("L400_USER", &state.user_profile)
                    .env("L400_CURLIB", &state.current_library)
                    .env("L400_LIBLIST", state.library_list.join(":"))
                    .output()
                {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        for line in stdout.lines().chain(stderr.lines()) {
                            self.output.push(line.to_string());
                        }
                        if self.output.len() == 2 {
                            self.output.push(format!(
                                "Command '{}' completed with status {}",
                                cmd,
                                output.status.code().unwrap_or_default()
                            ));
                        }
                    }
                    Err(error) => {
                        self.output
                            .push(format!("Command '{}' could not run: {}", cmd, error));
                    }
                }
            }
        }

        if !self.history.iter().any(|h| h == &cmd) {
            self.history.insert(0, cmd.clone());
        }

        self.show_output = true;
        self.command.clear();
        self.cursor_position = 0;
        self.history_index = 0;
        ScreenResult::none()
    }

    fn route_interactive_command(&mut self, command: &str) -> Option<ScreenResult> {
        let tokens = command.split_whitespace().collect::<Vec<_>>();
        let action = tokens.first()?.to_uppercase();

        match action.as_str() {
            "GO" => {
                let target = tokens
                    .get(1)
                    .map(|value| value.trim().to_uppercase())
                    .unwrap_or_default();
                if target == "MAIN" {
                    Some(ScreenResult::goto(ScreenId::MainMenu))
                } else {
                    self.show_usage_error("Usage: GO MAIN");
                    Some(ScreenResult::none())
                }
            }
            "SIGNOFF" => Some(ScreenResult::goto(ScreenId::SignOn)),
            "STRPDM" => Some(ScreenResult::goto(ScreenId::PdmBrowser)),
            "STRSQL" => Some(ScreenResult::goto(ScreenId::StrSql)),
            "WRKMBRPDM" => {
                let file = extract_command_arg(&tokens[1..], "FILE").or_else(|| {
                    tokens
                        .get(1)
                        .filter(|token| !token.contains('('))
                        .map(|value| value.to_string())
                });
                match file.filter(|value| !value.trim().is_empty()) {
                    Some(file) => Some(ScreenResult::with_data(
                        ScreenId::WrkMbrPdm,
                        normalize_file_spec(&file, &self.session),
                    )),
                    None => {
                        self.show_usage_error("Usage: WRKMBRPDM FILE(QGPL/QCLSRC)");
                        Some(ScreenResult::none())
                    }
                }
            }
            "STRSEU" => {
                let file = extract_command_arg(&tokens[1..], "FILE").or_else(|| {
                    tokens
                        .get(1)
                        .filter(|token| !token.contains('('))
                        .map(|value| value.to_string())
                });
                let member = extract_command_arg(&tokens[1..], "MBR").or_else(|| {
                    tokens
                        .get(2)
                        .filter(|token| !token.contains('('))
                        .map(|value| value.to_string())
                });
                match (file, member) {
                    (Some(file), Some(member)) => Some(ScreenResult::with_data(
                        ScreenId::StrSeu,
                        format!(
                            "{}/{}",
                            normalize_file_spec(&file, &self.session),
                            member.trim().to_uppercase()
                        ),
                    )),
                    _ => {
                        self.show_usage_error("Usage: STRSEU FILE(QGPL/QCLSRC) MBR(HELLO.CLP)");
                        Some(ScreenResult::none())
                    }
                }
            }
            _ => None,
        }
    }

    fn show_usage_error(&mut self, message: &str) {
        self.output.clear();
        self.output.push(message.to_string());
        self.show_output = true;
    }

    fn handle_session_command(&mut self, command: &str) -> bool {
        let tokens = command.split_whitespace().collect::<Vec<_>>();
        let Some(action) = tokens.first().map(|value| value.to_uppercase()) else {
            return false;
        };
        match action.as_str() {
            "CHGCURLIB" => {
                if let Some(library) = extract_command_arg(&tokens[1..], "CURLIB")
                    .or_else(|| extract_command_arg(&tokens[1..], "LIB"))
                    .or_else(|| tokens.get(1).map(|value| value.to_string()))
                {
                    self.session.set_current_library(&library);
                    self.output.push(format!(
                        "Current library changed to {}",
                        library.to_uppercase()
                    ));
                } else {
                    self.output.push("Usage: CHGCURLIB LIB(QGPL)".to_string());
                }
                true
            }
            "ADDLIBLE" => {
                if let Some(library) = extract_command_arg(&tokens[1..], "LIB")
                    .or_else(|| tokens.get(1).map(|value| value.to_string()))
                {
                    self.session.add_library(&library);
                    self.output
                        .push(format!("{} added to library list", library.to_uppercase()));
                } else {
                    self.output.push("Usage: ADDLIBLE LIB(QGPL)".to_string());
                }
                true
            }
            "DSPLIBL" => {
                let state = self.session.snapshot();
                self.output
                    .push(format!("Current library: {}", state.current_library));
                self.output.push("Library list:".to_string());
                for library in state.library_list {
                    self.output.push(format!("  {library}"));
                }
                true
            }
            _ => false,
        }
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
            .split(frame.area());

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
            KeyCode::Enter => self.execute_command(),
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

fn extract_command_arg(tokens: &[&str], key: &str) -> Option<String> {
    tokens.iter().find_map(|token| {
        let token = token.trim();
        if !token.to_uppercase().starts_with(&format!("{key}(")) || !token.ends_with(')') {
            return None;
        }
        Some(token[key.len() + 1..token.len() - 1].trim().to_string())
    })
}

fn normalize_file_spec(spec: &str, session: &SessionContext) -> String {
    let spec = spec.trim();
    if let Some((library, file)) = spec.split_once('/') {
        format!(
            "{}/{}",
            library.trim().to_uppercase(),
            file.trim().to_uppercase()
        )
    } else {
        format!(
            "{}/{}",
            session.snapshot().current_library,
            spec.to_uppercase()
        )
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
            frame.set_cursor_position((area.x + cursor_x as u16, area.y));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signoff_returns_to_signon_screen() {
        let mut cmd = CommandLine::new();
        cmd.command = "SIGNOFF".to_string();

        let result = cmd.execute_command();

        assert_eq!(result.next, Some(ScreenId::SignOn));
    }
}
