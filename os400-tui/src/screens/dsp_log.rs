use crossterm::event::{KeyCode, KeyEvent};
use l400::{l400_run_dir, qhst_path};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
};
use std::path::{Path, PathBuf};

use crate::screens::{Screen, ScreenResult};
use crate::style::*;
use crate::widgets::help_bar::{CpfMessage, HelpAction, HelpBar};
use crate::widgets::subfile_table::SubfileTable;

#[derive(Clone, Debug, PartialEq, Eq)]
struct LogEntry {
    source: String,
    timestamp: String,
    severity: String,
    event: String,
    user: String,
    message: String,
}

pub struct DspLog {
    entries: Vec<LogEntry>,
    visible: Vec<LogEntry>,
    table: SubfileTable,
    status: String,
    severity_filter: Option<String>,
    event_filter: Option<String>,
    date_filter: Option<String>,
}

impl DspLog {
    pub fn new() -> Self {
        let mut screen = Self {
            entries: Vec::new(),
            visible: Vec::new(),
            table: SubfileTable::new(
                vec!["Src", "Timestamp", "Sev", "Event", "User", "Message"],
                vec![10, 14, 8, 18, 12, 42],
            )
            .with_title("System log"),
            status: String::new(),
            severity_filter: None,
            event_filter: None,
            date_filter: None,
        };
        screen.refresh();
        screen
    }

    fn refresh(&mut self) {
        self.entries = load_log_entries(&qhst_path(), &l400_run_dir().join("joblogs"));
        self.apply_filters();
    }

    fn apply_filters(&mut self) {
        self.visible = self
            .entries
            .iter()
            .filter(|entry| {
                self.severity_filter
                    .as_ref()
                    .is_none_or(|filter| &entry.severity == filter)
                    && self
                        .event_filter
                        .as_ref()
                        .is_none_or(|filter| entry.event.contains(filter))
                    && self
                        .date_filter
                        .as_ref()
                        .is_none_or(|filter| entry.timestamp.starts_with(filter))
            })
            .cloned()
            .collect();
        self.table.set_rows(
            self.visible
                .iter()
                .map(|entry| {
                    vec![
                        entry.source.clone(),
                        entry.timestamp.clone(),
                        entry.severity.clone(),
                        entry.event.clone(),
                        entry.user.clone(),
                        entry.message.clone(),
                    ]
                })
                .collect(),
        );
        self.status = format!(
            "{} entries. F6=severity {:?} F7=event {:?} F8=date {:?}.",
            self.visible.len(),
            self.severity_filter.as_deref().unwrap_or("*ALL"),
            self.event_filter.as_deref().unwrap_or("*ALL"),
            self.date_filter.as_deref().unwrap_or("*ALL")
        );
    }

    fn cycle_severity(&mut self) {
        self.severity_filter = match self.severity_filter.as_deref() {
            None => Some("ERROR".to_string()),
            Some("ERROR") => Some("WARN".to_string()),
            Some("WARN") => Some("INFO".to_string()),
            _ => None,
        };
        self.apply_filters();
    }

    fn cycle_event(&mut self) {
        let mut events = self
            .entries
            .iter()
            .map(|entry| entry.event.clone())
            .collect::<Vec<_>>();
        events.sort();
        events.dedup();
        if events.is_empty() {
            self.event_filter = None;
        } else {
            let next = self
                .event_filter
                .as_ref()
                .and_then(|current| events.iter().position(|event| event == current))
                .map_or(0, |index| index + 1);
            self.event_filter = events.get(next).cloned();
        }
        self.apply_filters();
    }

    fn cycle_date(&mut self) {
        let mut dates = self
            .entries
            .iter()
            .filter_map(|entry| entry.timestamp.get(0..10).map(str::to_string))
            .collect::<Vec<_>>();
        dates.sort();
        dates.dedup();
        if dates.is_empty() {
            self.date_filter = None;
        } else {
            let next = self
                .date_filter
                .as_ref()
                .and_then(|current| dates.iter().position(|date| date == current))
                .map_or(0, |index| index + 1);
            self.date_filter = dates.get(next).cloned();
        }
        self.apply_filters();
    }
}

impl Screen for DspLog {
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
        CpfMessage::info("CPF0000", self.status.clone()).render(frame, chunks[2]);
        self.render_help(frame, chunks[3]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> ScreenResult {
        match key.code {
            KeyCode::F(3) | KeyCode::F(12) | KeyCode::Esc => ScreenResult::back(),
            KeyCode::F(5) => {
                self.refresh();
                ScreenResult::none()
            }
            KeyCode::F(6) => {
                self.cycle_severity();
                ScreenResult::none()
            }
            KeyCode::F(7) => {
                self.cycle_event();
                ScreenResult::none()
            }
            KeyCode::F(8) => {
                self.cycle_date();
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
            KeyCode::PageUp => {
                self.table.page_up();
                ScreenResult::none()
            }
            KeyCode::PageDown => {
                self.table.page_down();
                ScreenResult::none()
            }
            _ => ScreenResult::none(),
        }
    }
}

impl DspLog {
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(
            Paragraph::new("F5=Refresh F6=Severity F7=Event F8=Date")
                .style(STYLE_NORMAL)
                .block(
                    Block::default()
                        .title(" DSPLOG - QHST / QEZJOBLOG ")
                        .style(STYLE_HEADER)
                        .borders(Borders::ALL)
                        .border_style(STYLE_BORDER),
                ),
            area,
        );
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        HelpBar::new()
            .command("DSPLOG")
            .actions(vec![
                HelpAction::new("F3", "Back"),
                HelpAction::new("F5", "Refresh"),
                HelpAction::new("F6/F7/F8", "Filters"),
                HelpAction::new("PgUp/PgDn", "Scroll"),
            ])
            .render(frame, area);
    }
}

fn load_log_entries(qhst: &Path, joblog_dir: &Path) -> Vec<LogEntry> {
    let mut entries = Vec::new();
    if let Ok(content) = std::fs::read_to_string(qhst) {
        entries.extend(content.lines().filter_map(parse_qhst_line));
    }
    if let Ok(files) = std::fs::read_dir(joblog_dir) {
        for file in files.flatten() {
            entries.extend(load_joblog_file(file.path()));
        }
    }
    entries.sort_by(|left, right| left.timestamp.cmp(&right.timestamp));
    entries
}

fn parse_qhst_line(line: &str) -> Option<LogEntry> {
    let mut timestamp = String::new();
    let mut event = String::new();
    let mut user = String::new();
    let mut message = String::new();
    for part in line.split_whitespace() {
        if let Some((key, value)) = part.split_once('=') {
            match key {
                "ts" => timestamp = value.to_string(),
                "event" => event = value.to_string(),
                "user" => user = value.to_string(),
                "message" => message = value.replace('_', " "),
                _ => {}
            }
        }
    }
    (!event.is_empty()).then(|| LogEntry {
        source: "QHST".to_string(),
        severity: severity_for(&event, &message),
        timestamp,
        event,
        user,
        message,
    })
}

fn load_joblog_file(path: PathBuf) -> Vec<LogEntry> {
    let Ok(content) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    let source = path
        .file_stem()
        .and_then(|name| name.to_str())
        .map(|name| format!("JOB{name}"))
        .unwrap_or_else(|| "QEZJOBLOG".to_string());
    content
        .lines()
        .enumerate()
        .map(|(index, line)| LogEntry {
            source: source.clone(),
            timestamp: format!("{index:010}"),
            severity: severity_for(line, line),
            event: "JOBLOG".to_string(),
            user: "-".to_string(),
            message: line.to_string(),
        })
        .collect()
}

fn severity_for(event: &str, message: &str) -> String {
    let text = format!("{event} {message}").to_uppercase();
    if text.contains("DENIED") || text.contains("ERROR") || text.contains("FAIL") {
        "ERROR".to_string()
    } else if text.contains("AUTH") || text.contains("WARN") || text.contains("CHANGE") {
        "WARN".to_string()
    } else {
        "INFO".to_string()
    }
}

impl Default for DspLog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dsplog_loads_real_qhst_and_filters() {
        let temp = tempfile::tempdir().expect("tempdir");
        let qhst = temp.path().join("QHST");
        let joblogs = temp.path().join("joblogs");
        std::fs::create_dir_all(&joblogs).expect("joblogs");
        std::fs::write(
            &qhst,
            "ts=2026-05-01T10 event=AUTH_DENIED user=QPGMR object=/x message=no_access\n\
             ts=2026-05-02T11 event=OBJECT_CHANGE user=QSECOFR object=/y message=changed\n",
        )
        .expect("qhst");
        std::fs::write(joblogs.join("42.log"), "started\nspawn_error=boom\n").expect("joblog");

        let mut screen = DspLog {
            entries: load_log_entries(&qhst, &joblogs),
            visible: Vec::new(),
            table: SubfileTable::new(vec!["A"], vec![10]),
            status: String::new(),
            severity_filter: Some("ERROR".to_string()),
            event_filter: None,
            date_filter: Some("2026-05-01".to_string()),
        };
        screen.apply_filters();

        assert_eq!(screen.visible.len(), 1);
        assert_eq!(screen.visible[0].event, "AUTH_DENIED");
    }
}
