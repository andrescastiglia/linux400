use crossterm::event::{KeyCode, KeyEvent};
use l400::{
    DEFAULT_PF_MEMBER, LogicalFile, PfSchema, PhysicalFile, describe_object, read_pf_schema,
    read_string_attr, resolve_l400_root,
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph, Row, Table, TableState},
};
use std::path::PathBuf;

use crate::cl_parser::{extract_command_arg, tokenize_cl_command};
use crate::screens::{Screen, ScreenResult};
use crate::session::SessionContext;
use crate::style::*;
use crate::widgets::help_bar::{CpfMessage, HelpAction, HelpBar};

#[derive(Clone, Debug, PartialEq, Eq)]
struct DataRow {
    rrn: usize,
    key: String,
    cells: Vec<String>,
}

pub struct DspPfm {
    spec: String,
    path: PathBuf,
    member: String,
    headers: Vec<String>,
    rows: Vec<DataRow>,
    visible: Vec<DataRow>,
    state: TableState,
    filter: Option<(String, String)>,
    horizontal: usize,
    editing: bool,
    filtering: bool,
    filter_buffer: String,
    edit_buffer: String,
    status: String,
    lf_detail: Option<String>,
}

impl DspPfm {
    pub fn new(data: Option<&str>, session: SessionContext) -> Self {
        let spec = data
            .and_then(extract_file_spec)
            .unwrap_or_else(|| format!("{}/{}", session.snapshot().current_library, "CUSTOMERS"));
        let member = data
            .map(tokenize_cl_command)
            .and_then(|tokens| extract_command_arg(&tokens[1..], "MBR"))
            .unwrap_or_else(|| DEFAULT_PF_MEMBER.to_string());
        let path = object_path(&spec, &session);
        let mut screen = Self {
            spec,
            path,
            member,
            headers: Vec::new(),
            rows: Vec::new(),
            visible: Vec::new(),
            state: TableState::default(),
            filter: None,
            horizontal: 0,
            editing: false,
            filtering: false,
            filter_buffer: String::new(),
            edit_buffer: String::new(),
            status: String::new(),
            lf_detail: None,
        };
        screen.refresh();
        screen
    }

    fn refresh(&mut self) {
        self.headers.clear();
        self.rows.clear();
        self.lf_detail = None;
        match describe_object(&self.path) {
            Ok(object) if object.attribute.as_deref() == Some("LF") => self.load_lf(),
            Ok(_) => self.load_pf(),
            Err(error) => self.status = format!("Error opening {}: {error}", self.spec),
        }
        self.apply_filter();
    }

    fn load_pf(&mut self) {
        let schema = read_pf_schema(&self.path).unwrap_or_else(|_| PfSchema::minimal(0));
        self.headers = pf_headers(&schema);
        match PhysicalFile::open_member(&self.path, &self.member).and_then(|pf| pf.read_all()) {
            Ok(records) => {
                self.rows = records
                    .into_iter()
                    .enumerate()
                    .map(|(idx, (key, data))| DataRow {
                        rrn: idx + 1,
                        key: String::from_utf8_lossy(&key).to_string(),
                        cells: pf_cells(idx + 1, &schema, &key, &data),
                    })
                    .collect();
                self.status = format!("{} records in MBR({}).", self.rows.len(), self.member);
            }
            Err(error) => self.status = format!("Error reading PF: {error}"),
        }
    }

    fn load_lf(&mut self) {
        self.headers = vec![
            "RRN".to_string(),
            "LF_KEY".to_string(),
            "PF_KEY".to_string(),
        ];
        match LogicalFile::open(&self.path).and_then(|lf| lf.read_all_idx()) {
            Ok(records) => {
                let base = read_string_attr(&self.path, l400::L400_BASE_PF_ATTR)
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| "-".to_string());
                let keys = read_pf_schema(PathBuf::from(&base).as_path())
                    .map(|schema| schema.key_fields.join(","))
                    .unwrap_or_else(|_| "KEY".to_string());
                self.lf_detail = Some(format!("Base PF: {base} Key fields: {keys}"));
                self.rows = records
                    .into_iter()
                    .enumerate()
                    .map(|(idx, (secondary, primary))| DataRow {
                        rrn: idx + 1,
                        key: String::from_utf8_lossy(&secondary).to_string(),
                        cells: vec![
                            (idx + 1).to_string(),
                            String::from_utf8_lossy(&secondary).to_string(),
                            String::from_utf8_lossy(&primary).to_string(),
                        ],
                    })
                    .collect();
                self.status = format!(
                    "{} LF index rows. {}",
                    self.rows.len(),
                    self.lf_detail.clone().unwrap_or_default()
                );
            }
            Err(error) => self.status = format!("Error reading LF: {error}"),
        }
    }

    fn apply_filter(&mut self) {
        self.visible = self
            .rows
            .iter()
            .filter(|row| {
                self.filter.as_ref().is_none_or(|(field, value)| {
                    self.headers
                        .iter()
                        .position(|header| header == field)
                        .and_then(|index| row.cells.get(index))
                        .is_some_and(|cell| cell.contains(value))
                })
            })
            .cloned()
            .collect();
        if self.visible.is_empty() {
            self.state.select(None);
        } else if self.state.selected().is_none() {
            self.state.select(Some(0));
        }
    }

    fn begin_edit(&mut self) {
        if let Some(row) = self
            .state
            .selected()
            .and_then(|index| self.visible.get(index))
        {
            self.editing = true;
            self.edit_buffer = format!(
                "{} {}",
                row.key,
                row.cells.last().cloned().unwrap_or_default()
            );
        }
    }

    fn finish_edit(&mut self) {
        let mut parts = self.edit_buffer.splitn(2, ' ');
        let key = parts.next().unwrap_or_default().trim().to_string();
        let data = parts.next().unwrap_or_default().trim().to_string();
        self.editing = false;
        self.edit_buffer.clear();
        match PhysicalFile::open_member(&self.path, &self.member)
            .and_then(|pf| pf.write_rcd(key.as_bytes(), data.as_bytes()))
        {
            Ok(_) => {
                self.status = format!("Record KEY({key}) changed.");
                self.refresh();
            }
            Err(error) => self.status = format!("Error editing record: {error}"),
        }
    }

    fn finish_filter(&mut self) {
        let value = self.filter_buffer.trim().to_uppercase();
        self.filtering = false;
        self.filter_buffer.clear();
        if value.is_empty() {
            self.filter = None;
        } else if let Some((field, wanted)) = value.split_once('=') {
            self.filter = Some((field.trim().to_string(), wanted.trim().to_string()));
        } else {
            self.status = "Filter format is FIELD=VALUE.".to_string();
        }
        self.apply_filter();
    }
}

impl Screen for DspPfm {
    fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),
                Constraint::Min(0),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(crate::screens::screen_area(frame));
        self.render_header(frame, chunks[0]);
        self.render_table(frame, chunks[1]);
        CpfMessage::info("CPF0000", self.status.clone()).render(frame, chunks[2]);
        self.render_help(frame, chunks[3]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> ScreenResult {
        if self.editing {
            return match key.code {
                KeyCode::Enter => {
                    self.finish_edit();
                    ScreenResult::none()
                }
                KeyCode::Esc | KeyCode::F(12) => {
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
        if self.filtering {
            return match key.code {
                KeyCode::Enter => {
                    self.finish_filter();
                    ScreenResult::none()
                }
                KeyCode::Esc | KeyCode::F(12) => {
                    self.filtering = false;
                    self.filter_buffer.clear();
                    ScreenResult::none()
                }
                KeyCode::Backspace => {
                    self.filter_buffer.pop();
                    ScreenResult::none()
                }
                KeyCode::Char(c) => {
                    self.filter_buffer.push(c.to_ascii_uppercase());
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
            KeyCode::F(10) => {
                self.horizontal = self.horizontal.saturating_sub(1);
                ScreenResult::none()
            }
            KeyCode::F(11) => {
                self.horizontal = self
                    .horizontal
                    .saturating_add(1)
                    .min(self.headers.len().saturating_sub(1));
                ScreenResult::none()
            }
            KeyCode::F(17) => {
                self.filtering = true;
                self.filter_buffer = self
                    .filter
                    .as_ref()
                    .map(|(field, value)| format!("{field}={value}"))
                    .unwrap_or_default();
                ScreenResult::none()
            }
            KeyCode::Char('2') => {
                self.begin_edit();
                ScreenResult::none()
            }
            KeyCode::Up => {
                if let Some(i) = self.state.selected() {
                    self.state.select(Some(i.saturating_sub(1)));
                }
                ScreenResult::none()
            }
            KeyCode::Down => {
                if let Some(i) = self.state.selected() {
                    self.state.select(Some(
                        i.saturating_add(1)
                            .min(self.visible.len().saturating_sub(1)),
                    ));
                }
                ScreenResult::none()
            }
            _ => ScreenResult::none(),
        }
    }
}

impl DspPfm {
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let text = if self.editing {
            format!("Edit KEY DATA: {}", self.edit_buffer)
        } else if self.filtering {
            format!("Filter FIELD=VALUE: {}", self.filter_buffer)
        } else {
            format!(
                "File: {} MBR({}) Count({}) F10/F11=Scroll 2=Edit",
                self.spec,
                self.member,
                self.visible.len()
            )
        };
        frame.render_widget(
            Paragraph::new(text).style(STYLE_NORMAL).block(
                Block::default()
                    .title(" DSPPFM ")
                    .style(STYLE_HEADER)
                    .borders(Borders::ALL)
                    .border_style(STYLE_BORDER),
            ),
            area,
        );
    }

    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        let headers = self
            .headers
            .iter()
            .skip(self.horizontal)
            .take(6)
            .cloned()
            .collect::<Vec<_>>();
        let rows = self.visible.iter().map(|row| {
            Row::new(
                row.cells
                    .iter()
                    .skip(self.horizontal)
                    .take(6)
                    .cloned()
                    .collect::<Vec<_>>(),
            )
        });
        let widths = headers
            .iter()
            .map(|_| Constraint::Length(18))
            .collect::<Vec<_>>();
        let table = Table::new(rows, widths)
            .header(Row::new(headers).style(STYLE_TABLE_HEADER))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(STYLE_BORDER),
            )
            .style(STYLE_NORMAL)
            .row_highlight_style(STYLE_SELECTION);
        frame.render_stateful_widget(table, area, &mut self.state);
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        HelpBar::new()
            .command("DSPPFM")
            .actions(vec![
                HelpAction::new("F3", "Back"),
                HelpAction::new("F5", "Refresh"),
                HelpAction::new("F10/F11", "Columns"),
                HelpAction::new("2", "Edit"),
            ])
            .render(frame, area);
    }
}

fn extract_file_spec(command: &str) -> Option<String> {
    let tokens = tokenize_cl_command(command);
    extract_command_arg(&tokens[1..], "FILE").or_else(|| tokens.get(1).cloned())
}

fn object_path(spec: &str, session: &SessionContext) -> PathBuf {
    let root = resolve_l400_root();
    if let Some((library, file)) = spec.split_once('/') {
        root.join(library).join(file)
    } else {
        root.join(session.snapshot().current_library).join(spec)
    }
}

fn pf_headers(schema: &PfSchema) -> Vec<String> {
    if schema.fields.is_empty() {
        vec!["RRN".to_string(), "KEY".to_string(), "DATA".to_string()]
    } else {
        let mut headers = vec!["RRN".to_string()];
        headers.extend(schema.fields.iter().map(|field| field.name.clone()));
        headers
    }
}

fn pf_cells(rrn: usize, schema: &PfSchema, key: &[u8], data: &[u8]) -> Vec<String> {
    let key = String::from_utf8_lossy(key).to_string();
    let data = String::from_utf8_lossy(data).to_string();
    if schema.fields.is_empty() {
        vec![rrn.to_string(), key, data]
    } else {
        let parts = data.split('|').map(str::to_string).collect::<Vec<_>>();
        let mut cells = vec![rrn.to_string()];
        for field in &schema.fields {
            if field.name == "KEY" || field.name == "RRN" {
                cells.push(key.clone());
            } else if field.name == "DATA" {
                cells.push(data.clone());
            } else {
                cells.push(
                    parts
                        .get(cells.len().saturating_sub(2))
                        .cloned()
                        .unwrap_or_default(),
                );
            }
        }
        cells
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_headers_include_pf_fields() {
        let schema = PfSchema {
            record_len: 80,
            fields: vec![
                l400::PfField {
                    name: "KEY".into(),
                    type_: "CHAR".into(),
                    length: 10,
                    text: None,
                },
                l400::PfField {
                    name: "DATA".into(),
                    type_: "CHAR".into(),
                    length: 40,
                    text: None,
                },
            ],
            key_fields: vec!["KEY".into()],
        };
        assert_eq!(pf_headers(&schema), vec!["RRN", "KEY", "DATA"]);
    }
}
