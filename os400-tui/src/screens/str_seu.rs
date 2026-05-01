use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use l400::resolve_l400_root;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::path::PathBuf;
use std::process::Command;

use crate::screens::{Screen, ScreenId, ScreenResult};
use crate::style::*;
use crate::widgets::help_bar::{CpfMessage, HelpAction, HelpBar};

pub struct StrSeu {
    path: PathBuf,
    title: String,
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
    scroll_offset: usize,
    modified: bool,
    status_msg: Option<String>,
    undo_snapshot: Option<Vec<String>>,
    find_mode: bool,
    goto_mode: bool,
    prompt_buffer: String,
    find_term: String,
    return_to: ScreenId,
    return_data: Option<String>,
}

impl StrSeu {
    pub fn new(
        path: PathBuf,
        title: String,
        return_to: ScreenId,
        return_data: Option<String>,
    ) -> Self {
        let mut lines = if path.exists() {
            std::fs::read_to_string(&path)
                .unwrap_or_default()
                .lines()
                .map(|line| line.to_string())
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        if lines.is_empty() {
            lines.push(String::new());
        }

        Self {
            path,
            title,
            lines,
            cursor_row: 0,
            cursor_col: 0,
            scroll_offset: 0,
            modified: false,
            status_msg: None,
            undo_snapshot: None,
            find_mode: false,
            goto_mode: false,
            prompt_buffer: String::new(),
            find_term: String::new(),
            return_to,
            return_data,
        }
    }

    pub fn from_member_spec(
        library: &str,
        file: &str,
        member: &str,
        return_to: ScreenId,
        return_data: Option<String>,
    ) -> Self {
        let path = resolve_l400_root().join(library).join(file).join(member);
        let title = format!("{}/{}/{}", library, file, member);
        Self::new(path, title, return_to, return_data)
    }

    fn back_result(&self) -> ScreenResult {
        ScreenResult {
            next: Some(self.return_to),
            data: self.return_data.clone(),
        }
    }

    fn save(&mut self) {
        let content = self.lines.join("\n");
        match std::fs::write(&self.path, content) {
            Ok(_) => {
                self.modified = false;
                self.status_msg = Some("Member saved.".to_string());
            }
            Err(error) => {
                self.status_msg = Some(format!("Error saving member: {}", error));
            }
        }
    }

    fn reload(&mut self) {
        match std::fs::read_to_string(&self.path) {
            Ok(content) => {
                self.lines = content.lines().map(|line| line.to_string()).collect();
                if self.lines.is_empty() {
                    self.lines.push(String::new());
                }
                self.cursor_row = 0;
                self.cursor_col = 0;
                self.scroll_offset = 0;
                self.modified = false;
                self.status_msg = Some("Member reloaded.".to_string());
            }
            Err(error) => {
                self.status_msg = Some(format!("Error reloading member: {}", error));
            }
        }
    }

    fn member_parts(&self) -> Option<(&str, &str, &str)> {
        let mut parts = self.title.split('/');
        let library = parts.next()?;
        let file = parts.next()?;
        let member = parts.next()?;
        Some((library, file, member))
    }

    fn program_spec(&self) -> Option<String> {
        self.member_parts().map(|(library, _file, member)| {
            let stem = member.split('.').next().unwrap_or(member).to_uppercase();
            format!("{library}/{stem}")
        })
    }

    fn member_type(&self) -> String {
        self.member_parts()
            .and_then(|(_, _, member)| member.rsplit_once('.').map(|(_, ext)| ext.to_uppercase()))
            .unwrap_or_else(|| "CLP".to_string())
    }

    fn compile_command(&self) -> Option<String> {
        let (library, file, member) = self.member_parts()?;
        let pgm = self.program_spec()?;
        if self.member_type() == "C" {
            Some(format!(
                "CRTPGM PGM({pgm}) SRCFILE({library}/{file}) SRCMBR({member})"
            ))
        } else {
            Some(format!(
                "CRTCLPGM PGM({pgm}) SRCFILE({library}/{file}) SRCMBR({member})"
            ))
        }
    }

    fn run_toolchain_command(&mut self, command: String) {
        if self.modified {
            self.save();
        }
        match Command::new("l400cmd")
            .args(crate::cl_parser::tokenize_cl_command(&command))
            .output()
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let summary = stdout
                    .lines()
                    .chain(stderr.lines())
                    .last()
                    .unwrap_or("No compiler output.")
                    .to_string();
                self.status_msg = Some(format!(
                    "{} status={} {}",
                    command,
                    output.status.code().unwrap_or_default(),
                    summary
                ));
            }
            Err(error) => {
                self.status_msg = Some(format!("Error running {}: {}", command, error));
            }
        }
    }

    fn begin_edit(&mut self) {
        if self.undo_snapshot.is_none() {
            self.undo_snapshot = Some(self.lines.clone());
        }
        self.modified = true;
    }

    fn undo(&mut self) {
        if let Some(lines) = self.undo_snapshot.take() {
            self.lines = lines;
            self.ensure_cursor_in_bounds();
            self.modified = true;
            self.status_msg = Some("Undo complete.".to_string());
        } else {
            self.status_msg = Some("Nothing to undo.".to_string());
        }
    }

    fn find_next(&mut self) {
        let needle = self.prompt_buffer.trim().to_string();
        if needle.is_empty() {
            self.status_msg = Some("Find text is required.".to_string());
            return;
        }
        self.find_term = needle.clone();
        let start_row = self.cursor_row;
        for offset in 0..self.lines.len() {
            let row = (start_row + offset) % self.lines.len();
            let start_col = if row == start_row {
                self.cursor_col.saturating_add(1)
            } else {
                0
            };
            if let Some(col) = self.lines[row]
                .get(start_col..)
                .and_then(|line| line.find(&needle))
                .map(|col| col + start_col)
            {
                self.cursor_row = row;
                self.cursor_col = col;
                self.status_msg = Some(format!("Found at line {} column {}.", row + 1, col + 1));
                return;
            }
        }
        self.status_msg = Some(format!("'{needle}' not found."));
    }

    fn go_to_line(&mut self) {
        match self.prompt_buffer.trim().parse::<usize>() {
            Ok(line) if line > 0 && line <= self.lines.len() => {
                self.cursor_row = line - 1;
                self.cursor_col = 0;
                self.status_msg = Some(format!("Line {line}."));
            }
            _ => {
                self.status_msg = Some("Line number out of range.".to_string());
            }
        }
    }

    fn ensure_cursor_in_bounds(&mut self) {
        self.cursor_row = self.cursor_row.min(self.lines.len().saturating_sub(1));
        let line_len = self
            .lines
            .get(self.cursor_row)
            .map(|line| line.len())
            .unwrap_or_default();
        self.cursor_col = self.cursor_col.min(line_len);
    }

    fn adjust_scroll(&mut self, visible_rows: usize) {
        if visible_rows == 0 {
            return;
        }

        if self.cursor_row < self.scroll_offset {
            self.scroll_offset = self.cursor_row;
        } else if self.cursor_row >= self.scroll_offset + visible_rows {
            self.scroll_offset = self.cursor_row.saturating_sub(visible_rows - 1);
        }
    }
}

impl Default for StrSeu {
    fn default() -> Self {
        Self::from_member_spec(
            "QSYS",
            "QCLSRC",
            "NEWMBR.CLP",
            ScreenId::WrkMbrPdm,
            Some("QSYS/QCLSRC".to_string()),
        )
    }
}

impl Screen for StrSeu {
    fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(2),
                Constraint::Length(3),
            ])
            .split(frame.area());

        self.render_header(frame, chunks[0]);
        self.adjust_scroll(chunks[1].height.saturating_sub(2) as usize);
        self.render_editor(frame, chunks[1]);
        self.render_status(frame, chunks[2]);
        self.render_help(frame, chunks[3]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> ScreenResult {
        self.status_msg = None;

        if self.find_mode || self.goto_mode {
            return match key.code {
                KeyCode::Enter => {
                    if self.find_mode {
                        self.find_next();
                    } else {
                        self.go_to_line();
                    }
                    self.find_mode = false;
                    self.goto_mode = false;
                    self.prompt_buffer.clear();
                    ScreenResult::none()
                }
                KeyCode::F(12) | KeyCode::Esc => {
                    self.find_mode = false;
                    self.goto_mode = false;
                    self.prompt_buffer.clear();
                    ScreenResult::none()
                }
                KeyCode::Backspace => {
                    self.prompt_buffer.pop();
                    ScreenResult::none()
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.prompt_buffer.push(c);
                    ScreenResult::none()
                }
                _ => ScreenResult::none(),
            };
        }

        match key.code {
            KeyCode::F(3) => {
                if self.modified {
                    self.save();
                }
                self.back_result()
            }
            KeyCode::F(5) => {
                self.reload();
                ScreenResult::none()
            }
            KeyCode::F(14) => {
                if let Some(command) = self.compile_command() {
                    self.run_toolchain_command(command);
                } else {
                    self.status_msg = Some("Cannot derive member spec for compile.".to_string());
                }
                ScreenResult::none()
            }
            KeyCode::F(17) => {
                if let Some(pgm) = self.program_spec() {
                    self.run_toolchain_command(format!("CRTPGM PGM({pgm})"));
                } else {
                    self.status_msg = Some("Cannot derive program spec for CRTPGM.".to_string());
                }
                ScreenResult::none()
            }
            KeyCode::F(12) | KeyCode::Esc => self.back_result(),
            KeyCode::F(13) => {
                self.goto_mode = true;
                self.prompt_buffer.clear();
                self.status_msg = Some("Go to line:".to_string());
                ScreenResult::none()
            }
            KeyCode::F(16) => {
                self.find_mode = true;
                self.prompt_buffer = self.find_term.clone();
                self.status_msg = Some("Find:".to_string());
                ScreenResult::none()
            }
            KeyCode::Up => {
                if self.cursor_row > 0 {
                    self.cursor_row -= 1;
                }
                self.ensure_cursor_in_bounds();
                ScreenResult::none()
            }
            KeyCode::Down => {
                if self.cursor_row + 1 < self.lines.len() {
                    self.cursor_row += 1;
                }
                self.ensure_cursor_in_bounds();
                ScreenResult::none()
            }
            KeyCode::Left => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                } else if self.cursor_row > 0 {
                    self.cursor_row -= 1;
                    self.cursor_col = self.lines[self.cursor_row].len();
                }
                ScreenResult::none()
            }
            KeyCode::Right => {
                let line_len = self.lines[self.cursor_row].len();
                if self.cursor_col < line_len {
                    self.cursor_col += 1;
                } else if self.cursor_row + 1 < self.lines.len() {
                    self.cursor_row += 1;
                    self.cursor_col = 0;
                }
                ScreenResult::none()
            }
            KeyCode::Home => {
                self.cursor_col = 0;
                ScreenResult::none()
            }
            KeyCode::End => {
                self.cursor_col = self.lines[self.cursor_row].len();
                ScreenResult::none()
            }
            KeyCode::Enter => {
                self.begin_edit();
                let remainder = self.lines[self.cursor_row].split_off(self.cursor_col);
                self.cursor_row += 1;
                self.cursor_col = 0;
                self.lines.insert(self.cursor_row, remainder);
                ScreenResult::none()
            }
            KeyCode::Backspace => {
                self.begin_edit();
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                    self.lines[self.cursor_row].remove(self.cursor_col);
                } else if self.cursor_row > 0 {
                    let current = self.lines.remove(self.cursor_row);
                    self.cursor_row -= 1;
                    self.cursor_col = self.lines[self.cursor_row].len();
                    self.lines[self.cursor_row].push_str(&current);
                }
                ScreenResult::none()
            }
            KeyCode::Delete => {
                self.begin_edit();
                if self.cursor_col < self.lines[self.cursor_row].len() {
                    self.lines[self.cursor_row].remove(self.cursor_col);
                } else if self.cursor_row + 1 < self.lines.len() {
                    let next = self.lines.remove(self.cursor_row + 1);
                    self.lines[self.cursor_row].push_str(&next);
                }
                ScreenResult::none()
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.begin_edit();
                self.lines[self.cursor_row].insert(self.cursor_col, c);
                self.cursor_col += 1;
                ScreenResult::none()
            }
            KeyCode::Char('z') | KeyCode::Char('Z')
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.undo();
                ScreenResult::none()
            }
            _ => ScreenResult::none(),
        }
    }
}

impl StrSeu {
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let marker = if self.modified { "*" } else { "" };
        let block = Block::default()
            .title(format!(" STRSEU - {} {} ", self.title, marker))
            .style(STYLE_HEADER)
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);
        frame.render_widget(block, area);

        let prompt = if self.find_mode {
            format!("   Find: {}", self.prompt_buffer)
        } else if self.goto_mode {
            format!("   Go to line: {}", self.prompt_buffer)
        } else {
            String::new()
        };
        let text = Line::from(format!(
            "  Line {:04}.00   Column {:02}   F3=Save   F5=Reload   F12=Cancel{}",
            self.cursor_row + 1,
            self.cursor_col + 1,
            prompt
        ));
        let inner = Rect::new(area.x + 1, area.y + 1, area.width - 2, 1);
        frame.render_widget(Paragraph::new(text).style(STYLE_NORMAL), inner);
    }

    fn render_editor(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let visible_rows = inner.height as usize;
        let lines = (self.scroll_offset..self.scroll_offset + visible_rows)
            .map(|row| {
                let number = Span::styled(
                    format!("{:04}.00 ", row + 1),
                    ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray),
                );
                let content = self.lines.get(row).cloned().unwrap_or_default();
                let mut spans = vec![number];
                if !self.find_term.is_empty() {
                    if let Some(pos) = content.find(&self.find_term) {
                        spans.push(Span::raw(content[..pos].to_string()));
                        spans.push(Span::styled(self.find_term.clone(), STYLE_SELECTION));
                        spans.push(Span::raw(content[pos + self.find_term.len()..].to_string()));
                    } else {
                        spans.push(Span::raw(content));
                    }
                } else {
                    spans.push(Span::raw(content));
                }
                Line::from(spans)
            })
            .collect::<Vec<_>>();

        frame.render_widget(Paragraph::new(lines).style(STYLE_NORMAL), inner);

        let cursor_x = inner.x + 8 + self.cursor_col as u16;
        let cursor_y = inner.y + self.cursor_row.saturating_sub(self.scroll_offset) as u16;
        if cursor_x < inner.x + inner.width && cursor_y < inner.y + inner.height {
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }

    fn render_status(&self, frame: &mut Frame, area: Rect) {
        let message = self.status_msg.clone().unwrap_or_default();
        let cpf = if message.to_ascii_lowercase().contains("error") {
            CpfMessage::error("CPF9898", message)
        } else {
            CpfMessage::info("CPF0000", message)
        };
        cpf.render(frame, area);
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        HelpBar::new()
            .command("STRSEU")
            .actions(vec![
                HelpAction::new("F3", "Save"),
                HelpAction::new("F5", "Reload"),
                HelpAction::new("F13", "Line"),
                HelpAction::new("F14", "CRTCLPGM"),
                HelpAction::new("F16", "Find"),
                HelpAction::new("Ctrl-Z", "Undo"),
                HelpAction::new("F12", "Cancel"),
            ])
            .render(frame, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn goto_find_and_undo_are_local_editor_actions() {
        let mut seu = StrSeu::from_member_spec(
            "QGPL",
            "QCLSRC",
            "HELLO.CLP",
            ScreenId::WrkMbrPdm,
            Some("QGPL/QCLSRC".to_string()),
        );
        seu.lines = vec![
            "PGM".to_string(),
            "SNDPGMMSG".to_string(),
            "ENDPGM".to_string(),
        ];

        seu.goto_mode = true;
        seu.prompt_buffer = "3".to_string();
        assert_eq!(seu.handle_key(KeyEvent::from(KeyCode::Enter)).next, None);
        assert_eq!(seu.cursor_row, 2);

        seu.find_mode = true;
        seu.prompt_buffer = "SNDPGMMSG".to_string();
        assert_eq!(seu.handle_key(KeyEvent::from(KeyCode::Enter)).next, None);
        assert_eq!(seu.cursor_row, 1);

        assert_eq!(
            seu.handle_key(KeyEvent::from(KeyCode::Char('X'))).next,
            None
        );
        assert!(seu.lines[1].contains('X'));
        assert_eq!(
            seu.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL))
                .next,
            None
        );
        assert_eq!(seu.lines[1], "SNDPGMMSG");
    }

    #[test]
    fn compile_command_uses_member_type() {
        let seu = StrSeu::from_member_spec(
            "QGPL",
            "QCSRC",
            "HELLO.C",
            ScreenId::WrkMbrPdm,
            Some("QGPL/QCSRC".to_string()),
        );
        assert!(
            seu.compile_command()
                .expect("compile command")
                .starts_with("CRTPGM")
        );
    }
}
