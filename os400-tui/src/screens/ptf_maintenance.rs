use std::process::Command;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::screens::{Screen, ScreenId, ScreenResult};
use crate::style::*;

pub struct PtfMaintenanceScreen {
    ptf_list: Vec<PtfInfo>,
    selected_index: usize,
    scroll_offset: usize,
    status_message: String,
    show_confirm_dialog: bool,
    pending_action: Option<PtfAction>,
}

struct PtfInfo {
    id: String,
    name: String,
    version: String,
    status: String,
}

enum PtfAction {
    Apply(String),
    Rollback(String),
}

impl Default for PtfMaintenanceScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl PtfMaintenanceScreen {
    pub fn new() -> Self {
        let mut screen = Self {
            ptf_list: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            status_message: String::new(),
            show_confirm_dialog: false,
            pending_action: None,
        };
        screen.load_ptf_list();
        screen
    }

    fn load_ptf_list(&mut self) {
        self.ptf_list.clear();

        let cache_dir = "/var/cache/l400/ptf";
        if let Ok(entries) = std::fs::read_dir(cache_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let manifest_path = path.join("manifest.toml");
                    if manifest_path.exists()
                        && let Ok(content) = std::fs::read_to_string(&manifest_path)
                    {
                        let id = extract_toml_value(&content, "package.id")
                            .unwrap_or_else(|| "Unknown".to_string());
                        let name = extract_toml_value(&content, "package.name")
                            .unwrap_or_else(|| "Unknown".to_string());
                        let version = extract_toml_value(&content, "package.version")
                            .unwrap_or_else(|| "Unknown".to_string());

                        let status = check_ptf_status(&id);

                        self.ptf_list.push(PtfInfo {
                            id,
                            name,
                            version,
                            status: status.clone(),
                        });
                    }
                }
            }
        }

        if self.ptf_list.is_empty() {
            self.status_message = "No PTFs found in cache.".to_string();
        }
    }
}

impl Screen for PtfMaintenanceScreen {
    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),
                Constraint::Min(0),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(area);

        self.render_header(frame, chunks[0]);
        self.render_list(frame, chunks[1]);
        self.render_status(frame, chunks[2]);
        self.render_help(frame, chunks[3]);

        if self.show_confirm_dialog {
            self.render_confirm_dialog(frame);
        }
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> ScreenResult {
        use crossterm::event::KeyCode;

        if self.show_confirm_dialog {
            match key.code {
                KeyCode::Enter => {
                    if let Some(action) = &self.pending_action {
                        match action {
                            PtfAction::Apply(ptf_id) => {
                                self.status_message = format!("Applying PTF {}...", ptf_id);
                                let output = Command::new("l400")
                                    .args(["APYPTF", ptf_id, "*APPLY", "*YES"])
                                    .output();
                                match output {
                                    Ok(out) => {
                                        if out.status.success() {
                                            self.status_message =
                                                format!("PTF {} applied successfully", ptf_id);
                                        } else {
                                            self.status_message = format!(
                                                "Failed: {}",
                                                String::from_utf8_lossy(&out.stderr)
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        self.status_message = format!("Error: {}", e);
                                    }
                                }
                            }
                            PtfAction::Rollback(ptf_id) => {
                                self.status_message = format!("Rolling back PTF {}...", ptf_id);
                                let output = Command::new("l400")
                                    .args(["APYPTF", ptf_id, "*ROLLBACK", "*YES"])
                                    .output();
                                match output {
                                    Ok(out) => {
                                        if out.status.success() {
                                            self.status_message =
                                                format!("PTF {} rolled back", ptf_id);
                                        } else {
                                            self.status_message = format!(
                                                "Failed: {}",
                                                String::from_utf8_lossy(&out.stderr)
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        self.status_message = format!("Error: {}", e);
                                    }
                                }
                            }
                        }
                        self.show_confirm_dialog = false;
                        self.pending_action = None;
                        self.load_ptf_list();
                    }
                    ScreenResult::none()
                }
                KeyCode::F(12) | KeyCode::Esc => {
                    self.show_confirm_dialog = false;
                    self.pending_action = None;
                    ScreenResult::none()
                }
                _ => ScreenResult::none(),
            }
        } else {
            match key.code {
                KeyCode::Up => {
                    if self.selected_index > 0 {
                        self.selected_index -= 1;
                        if self.selected_index < self.scroll_offset {
                            self.scroll_offset = self.selected_index;
                        }
                    }
                    ScreenResult::none()
                }
                KeyCode::Down => {
                    if self.selected_index < self.ptf_list.len().saturating_sub(1) {
                        self.selected_index += 1;
                        if self.selected_index >= self.scroll_offset + 15 {
                            self.scroll_offset = self.selected_index.saturating_sub(14);
                        }
                    }
                    ScreenResult::none()
                }
                KeyCode::F(5) => {
                    self.load_ptf_list();
                    self.status_message = "PTF list refreshed.".to_string();
                    ScreenResult::none()
                }
                KeyCode::F(6) => {
                    if !self.ptf_list.is_empty() {
                        let ptf_id = self.ptf_list[self.selected_index].id.clone();
                        self.pending_action = Some(PtfAction::Apply(ptf_id));
                        self.show_confirm_dialog = true;
                    }
                    ScreenResult::none()
                }
                KeyCode::F(7) => {
                    if !self.ptf_list.is_empty() {
                        let ptf_id = self.ptf_list[self.selected_index].id.clone();
                        self.pending_action = Some(PtfAction::Rollback(ptf_id));
                        self.show_confirm_dialog = true;
                    }
                    ScreenResult::none()
                }
                KeyCode::F(3) | KeyCode::F(12) => ScreenResult::goto(ScreenId::MainMenu),
                _ => ScreenResult::none(),
            }
        }
    }
}

impl PtfMaintenanceScreen {
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" PTF Maintenance ")
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER)
            .style(STYLE_HEADER);
        frame.render_widget(block, area);

        let text = vec![
            Line::from(vec![
                Span::styled("System: ", STYLE_NORMAL),
                Span::styled("L400   ", STYLE_NORMAL),
                Span::styled("User: ", STYLE_NORMAL),
                Span::styled("QSECOFR   ", STYLE_NORMAL),
            ]),
            Line::from(vec![Span::styled(
                "PTF ID     NAME                      VERSION   STATUS",
                STYLE_NORMAL,
            )]),
        ];

        let inner = Rect::new(area.x + 1, area.y + 1, area.width - 2, 2);
        frame.render_widget(Paragraph::new(text).style(STYLE_NORMAL), inner);
    }

    fn render_list(&self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .ptf_list
            .iter()
            .skip(self.scroll_offset)
            .take(15)
            .enumerate()
            .map(|(i, ptf)| {
                let idx = self.scroll_offset + i;
                let marker = if idx == self.selected_index { ">" } else { " " };
                let line = format!(
                    "{} {:<10} {:<25} {:<10} {:<10}",
                    marker, ptf.id, ptf.name, ptf.version, ptf.status
                );
                ListItem::new(line).style(if idx == self.selected_index {
                    STYLE_OPTION_SELECTED
                } else {
                    STYLE_OPTION
                })
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(STYLE_BORDER),
            )
            .style(STYLE_NORMAL);

        frame.render_widget(list, area);
    }

    fn render_status(&self, frame: &mut Frame, area: Rect) {
        let text = if self.status_message.is_empty() {
            " ".repeat(area.width as usize)
        } else {
            format!(" {}", self.status_message)
        };
        frame.render_widget(Paragraph::new(text).style(STYLE_NORMAL), area);
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let text = " F3=Exit   F5=Refresh   F6=Apply   F7=Rollback   F12=Cancel ";
        frame.render_widget(Paragraph::new(text).style(STYLE_NORMAL), area);
    }

    fn render_confirm_dialog(&self, frame: &mut Frame) {
        let area = frame.area();
        let dialog_area = Rect::new(area.width / 2 - 20, area.height / 2 - 4, 40, 8);

        frame.render_widget(ratatui::widgets::Clear, dialog_area);

        let block = Block::default()
            .title(" Confirm Action ")
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);
        frame.render_widget(block, dialog_area);

        if let Some(action) = &self.pending_action {
            let text = match action {
                PtfAction::Apply(ptf_id) => format!(" Apply PTF: {}? ", ptf_id),
                PtfAction::Rollback(ptf_id) => format!(" Rollback PTF: {}? ", ptf_id),
            };
            let inner = Rect::new(
                dialog_area.x + 1,
                dialog_area.y + 2,
                dialog_area.width - 2,
                1,
            );
            frame.render_widget(Paragraph::new(text).style(STYLE_NORMAL), inner);

            let hint = Rect::new(
                dialog_area.x + 1,
                dialog_area.y + 4,
                dialog_area.width - 2,
                1,
            );
            frame.render_widget(
                Paragraph::new("ENTER=Confirm  F12=Cancel").style(STYLE_NORMAL),
                hint,
            );
        }
    }
}

fn extract_toml_value(content: &str, key: &str) -> Option<String> {
    // Handle dotted key format: package.id = "..."
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with(&format!("{} = ", key))
            && let Some(start) = line.find('"')
            && let Some(end) = line.rfind('"')
            && start != end
        {
            return Some(line[start + 1..end].to_string());
        }
    }

    // Handle section-based format: [package] \n id = "..."
    if let Some((section, subkey)) = key.split_once('.') {
        let mut in_section = false;
        for line in content.lines() {
            let line = line.trim();
            if line == format!("[{}]", section) {
                in_section = true;
                continue;
            }
            if in_section {
                if line.starts_with('[') {
                    // Entered a new section
                    break;
                }
                if line.starts_with(&format!("{} = ", subkey))
                    && let Some(start) = line.find('"')
                    && let Some(end) = line.rfind('"')
                    && start != end
                {
                    return Some(line[start + 1..end].to_string());
                }
            }
        }
    }
    None
}

fn check_ptf_status(ptf_id: &str) -> String {
    let audit_path = "/var/log/l400/ptf-audit.log";
    if let Ok(content) = std::fs::read_to_string(audit_path) {
        for line in content.lines() {
            if line.contains(ptf_id)
                && (line.contains("APPLY") || line.contains("apply"))
                && (line.contains("success") || line.contains("Ok"))
            {
                return "APPLIED".to_string();
            }
            if line.contains(ptf_id)
                && (line.contains("ROLLBACK") || line.contains("rollback"))
                && (line.contains("success") || line.contains("Ok"))
            {
                return "ROLLED BACK".to_string();
            }
        }
    }
    "CACHED".to_string()
}
