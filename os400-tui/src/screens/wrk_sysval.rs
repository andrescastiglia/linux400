use crossterm::event::{KeyCode, KeyEvent};
use l400::{l400_run_dir, resolve_l400_root};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::screens::{Screen, ScreenResult};
use crate::style::*;
use crate::widgets::help_bar::{CpfMessage, HelpAction, HelpBar};
use crate::widgets::subfile_table::SubfileTable;

#[derive(Clone, Debug, PartialEq, Eq)]
struct SysVal {
    name: String,
    value: String,
    editable: bool,
    description: String,
}

pub struct WrkSysVal {
    values: Vec<SysVal>,
    table: SubfileTable,
    store_path: PathBuf,
    editing: bool,
    edit_buffer: String,
    detail: Option<String>,
    status: String,
}

impl WrkSysVal {
    pub fn new() -> Self {
        Self::with_store(l400_run_dir().join("system_values.env"))
    }

    fn with_store(store_path: PathBuf) -> Self {
        let mut screen = Self {
            values: Vec::new(),
            table: SubfileTable::new(
                vec!["Opt", "Name", "Value", "Editable", "Description"],
                vec![4, 14, 30, 10, 42],
            )
            .with_title("System values"),
            store_path,
            editing: false,
            edit_buffer: String::new(),
            detail: None,
            status: String::new(),
        };
        screen.refresh();
        screen
    }

    fn refresh(&mut self) {
        self.values = load_system_values(&self.store_path);
        self.sync_table();
        self.status = format!("{} system values loaded.", self.values.len());
    }

    fn sync_table(&mut self) {
        self.table.set_rows(
            self.values
                .iter()
                .map(|value| {
                    vec![
                        " ".to_string(),
                        value.name.clone(),
                        value.value.clone(),
                        if value.editable { "*YES" } else { "*NO" }.to_string(),
                        value.description.clone(),
                    ]
                })
                .collect(),
        );
    }

    fn selected(&self) -> Option<&SysVal> {
        self.table
            .selected()
            .and_then(|index| self.values.get(index))
            .or_else(|| self.values.first())
    }

    fn begin_change(&mut self) {
        let Some(value) = self.selected().cloned() else {
            self.status = "No system value selected.".to_string();
            return;
        };
        if !value.editable {
            self.status = format!("{} is display-only.", value.name);
            return;
        }
        self.editing = true;
        self.edit_buffer = value.value.clone();
    }

    fn finish_change(&mut self) {
        let Some(selected) = self.selected().cloned() else {
            self.status = "No system value selected.".to_string();
            return;
        };
        self.editing = false;
        let new_value = self.edit_buffer.trim().to_string();
        self.edit_buffer.clear();
        let mut overrides = read_overrides(&self.store_path);
        overrides.insert(selected.name.clone(), new_value.clone());
        match write_overrides(&self.store_path, &overrides) {
            Ok(_) => {
                self.status = format!("{} changed to {}.", selected.name, new_value);
                self.refresh();
            }
            Err(error) => self.status = format!("Error changing {}: {error}", selected.name),
        }
    }

    fn show_detail(&mut self) {
        self.detail = self.selected().map(|value| {
            format!(
                "{}\nValue: {}\nEditable: {}\n{}",
                value.name,
                value.value,
                if value.editable { "*YES" } else { "*NO" },
                value.description
            )
        });
    }
}

impl Screen for WrkSysVal {
    fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),
                Constraint::Min(0),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(frame.area());
        self.render_header(frame, chunks[0]);
        self.table.render(frame, chunks[1]);
        self.render_status(frame, chunks[2]);
        self.render_help(frame, chunks[3]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> ScreenResult {
        if self.editing {
            return match key.code {
                KeyCode::Enter => {
                    self.finish_change();
                    ScreenResult::none()
                }
                KeyCode::F(12) | KeyCode::Esc => {
                    self.editing = false;
                    self.edit_buffer.clear();
                    ScreenResult::none()
                }
                KeyCode::Backspace => {
                    self.edit_buffer.pop();
                    ScreenResult::none()
                }
                KeyCode::Char(c) => {
                    self.edit_buffer.push(c);
                    ScreenResult::none()
                }
                _ => ScreenResult::none(),
            };
        }
        match key.code {
            KeyCode::F(3) | KeyCode::F(12) | KeyCode::Esc => ScreenResult::back(),
            KeyCode::F(5) => {
                self.refresh();
                ScreenResult::none()
            }
            KeyCode::Up => {
                self.table.select_prev();
                ScreenResult::none()
            }
            KeyCode::Down => {
                self.table.select_next();
                ScreenResult::none()
            }
            KeyCode::Char('2') => {
                self.begin_change();
                ScreenResult::none()
            }
            KeyCode::Char('5') => {
                self.show_detail();
                ScreenResult::none()
            }
            _ => ScreenResult::none(),
        }
    }
}

impl WrkSysVal {
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let line = if self.editing {
            format!("Change value: {}", self.edit_buffer)
        } else {
            "Options: 2=Change 5=Display".to_string()
        };
        frame.render_widget(
            Paragraph::new(line).style(STYLE_NORMAL).block(
                Block::default()
                    .title(" WRKSYSVAL ")
                    .style(STYLE_HEADER)
                    .borders(Borders::ALL)
                    .border_style(STYLE_BORDER),
            ),
            area,
        );
    }

    fn render_status(&self, frame: &mut Frame, area: Rect) {
        let message = self.detail.clone().unwrap_or_else(|| self.status.clone());
        CpfMessage::info("CPF0000", message).render(frame, area);
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        HelpBar::new()
            .command("WRKSYSVAL")
            .actions(vec![
                HelpAction::new("F3", "Back"),
                HelpAction::new("F5", "Refresh"),
                HelpAction::new("2", "Change"),
                HelpAction::new("5", "Display"),
            ])
            .render(frame, area);
    }
}

fn load_system_values(path: &Path) -> Vec<SysVal> {
    let overrides = read_overrides(path);
    default_values()
        .into_iter()
        .map(|mut value| {
            if let Some(saved) = overrides.get(&value.name) {
                value.value.clone_from(saved);
            }
            value
        })
        .collect()
}

fn default_values() -> Vec<SysVal> {
    vec![
        SysVal {
            name: "QROOT".to_string(),
            value: resolve_l400_root().display().to_string(),
            editable: false,
            description: "Linux/400 object root.".to_string(),
        },
        SysVal {
            name: "QMODE".to_string(),
            value: "DEV".to_string(),
            editable: true,
            description: "Runtime profile: DEV, DEGRADED or FULL.".to_string(),
        },
        SysVal {
            name: "QAUTOCFG".to_string(),
            value: "*YES".to_string(),
            editable: true,
            description: "Allow TUI/runtime to create missing support paths.".to_string(),
        },
        SysVal {
            name: "QDATEFMT".to_string(),
            value: "ISO".to_string(),
            editable: true,
            description: "Date display format used by operator panels.".to_string(),
        },
    ]
}

fn read_overrides(path: &Path) -> HashMap<String, String> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.trim().to_uppercase(), value.trim().to_string()))
        .collect()
}

fn write_overrides(path: &Path, values: &HashMap<String, String>) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut rows = values.iter().collect::<Vec<_>>();
    rows.sort_by(|left, right| left.0.cmp(right.0));
    std::fs::write(
        path,
        rows.into_iter()
            .map(|(key, value)| format!("{key}={value}\n"))
            .collect::<String>(),
    )
}

impl Default for WrkSysVal {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changes_editable_system_value() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = temp.path().join("sysvals.env");
        let mut screen = WrkSysVal::with_store(store.clone());
        let idx = screen
            .values
            .iter()
            .position(|value| value.name == "QMODE")
            .expect("QMODE");
        for _ in 0..idx {
            screen.table.select_next();
        }
        screen.begin_change();
        screen.edit_buffer = "FULL".to_string();
        screen.finish_change();

        assert!(
            std::fs::read_to_string(store)
                .expect("store")
                .contains("QMODE=FULL")
        );
        assert!(
            screen
                .values
                .iter()
                .any(|value| value.name == "QMODE" && value.value == "FULL")
        );
    }
}
