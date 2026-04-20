use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use l400::resolve_l400_root;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::path::PathBuf;

use crate::screens::{Screen, ScreenId, ScreenResult};
use crate::style::*;

pub struct StrSeu {
    path: PathBuf,
    title: String,
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
    scroll_offset: usize,
    modified: bool,
    status_msg: Option<String>,
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
                self.status_msg = Some("Miembro guardado.".to_string());
            }
            Err(error) => {
                self.status_msg = Some(format!("Error al guardar: {}", error));
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
                self.status_msg = Some("Miembro recargado.".to_string());
            }
            Err(error) => {
                self.status_msg = Some(format!("Error al recargar: {}", error));
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
            .split(frame.size());

        self.render_header(frame, chunks[0]);
        self.adjust_scroll(chunks[1].height.saturating_sub(2) as usize);
        self.render_editor(frame, chunks[1]);
        self.render_status(frame, chunks[2]);
        self.render_help(frame, chunks[3]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> ScreenResult {
        self.status_msg = None;

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
            KeyCode::F(12) | KeyCode::Esc => self.back_result(),
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
                self.modified = true;
                let remainder = self.lines[self.cursor_row].split_off(self.cursor_col);
                self.cursor_row += 1;
                self.cursor_col = 0;
                self.lines.insert(self.cursor_row, remainder);
                ScreenResult::none()
            }
            KeyCode::Backspace => {
                self.modified = true;
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
                self.modified = true;
                if self.cursor_col < self.lines[self.cursor_row].len() {
                    self.lines[self.cursor_row].remove(self.cursor_col);
                } else if self.cursor_row + 1 < self.lines.len() {
                    let next = self.lines.remove(self.cursor_row + 1);
                    self.lines[self.cursor_row].push_str(&next);
                }
                ScreenResult::none()
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.modified = true;
                self.lines[self.cursor_row].insert(self.cursor_col, c);
                self.cursor_col += 1;
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

        let text = Line::from(format!(
            "  Line {:04}.00   Column {:02}   F3=Save   F5=Reload   F12=Cancel",
            self.cursor_row + 1,
            self.cursor_col + 1
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
                Line::from(vec![number, Span::raw(content)])
            })
            .collect::<Vec<_>>();

        frame.render_widget(Paragraph::new(lines).style(STYLE_NORMAL), inner);

        let cursor_x = inner.x + 8 + self.cursor_col as u16;
        let cursor_y = inner.y + self.cursor_row.saturating_sub(self.scroll_offset) as u16;
        if cursor_x < inner.x + inner.width && cursor_y < inner.y + inner.height {
            frame.set_cursor(cursor_x, cursor_y);
        }
    }

    fn render_status(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(
            Paragraph::new(self.status_msg.clone().unwrap_or_default()).style(STYLE_NORMAL),
            inner,
        );
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let help_text = Line::from(vec![
            "F3=Save   ".into(),
            "F5=Reload   ".into(),
            "F12=Cancel".into(),
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
