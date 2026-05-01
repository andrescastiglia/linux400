use crossterm::event::{KeyCode, KeyEvent};
use l400::{L400_AUTH_MANIFEST_VERSION, read_audit_records, read_loader_status};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
};

use crate::screens::{Screen, ScreenResult};
use crate::style::*;
use crate::widgets::help_bar::{CpfMessage, HelpAction, HelpBar};
use crate::widgets::subfile_table::SubfileTable;

const RUNTIME_POLICY_VERSION: &str = "auth-v2";
const EXPECTED_EBPF_POLICY_VERSION: &str = "phase3-v1";
const OBJ_TYPES: &[&str] = &[
    "*PGM", "*FILE", "*USRPRF", "*LIB", "*DTAQ", "*CMD", "*SRVPGM", "*OUTQ",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PolicyFilter {
    All,
    AuthDenied,
    UserChanges,
    ObjectChanges,
}

pub struct DspPolicy {
    rows: Vec<Vec<String>>,
    table: SubfileTable,
    status: String,
    filter: PolicyFilter,
}

impl DspPolicy {
    pub fn new() -> Self {
        let mut screen = Self {
            rows: Vec::new(),
            table: SubfileTable::new(
                vec![
                    "ObjType",
                    "Runtime",
                    "eBPF",
                    "RuntimeVer",
                    "PolicyVer",
                    "Gap",
                ],
                vec![10, 12, 12, 12, 14, 38],
            )
            .with_title("Policy enforcement"),
            status: String::new(),
            filter: PolicyFilter::All,
        };
        screen.refresh();
        screen
    }

    fn refresh(&mut self) {
        let loader = read_loader_status().ok();
        let ebpf = loader
            .as_ref()
            .and_then(|status| status.policy_version.clone())
            .unwrap_or_else(|| "unavailable".to_string());
        let ebpf_state = loader
            .as_ref()
            .map(|status| {
                if status.protection_active {
                    "active"
                } else {
                    "degraded"
                }
            })
            .unwrap_or("degraded");
        let loader_gap = loader
            .as_ref()
            .and_then(|status| {
                status
                    .known_gaps
                    .clone()
                    .or_else(|| status.last_error.clone())
            })
            .unwrap_or_else(|| "none reported".to_string());
        let audit_gap = audit_gap_summary(self.filter);
        let gap = if audit_gap == "none" {
            loader_gap
        } else {
            audit_gap
        };

        self.rows = OBJ_TYPES
            .iter()
            .map(|objtype| {
                vec![
                    (*objtype).to_string(),
                    "userspace".to_string(),
                    ebpf_state.to_string(),
                    format!("manifest-v{L400_AUTH_MANIFEST_VERSION}"),
                    ebpf.clone(),
                    gap.clone(),
                ]
            })
            .collect();
        self.table.set_rows(self.rows.clone());
        self.status = format!(
            "Runtime policy {}. Expected eBPF {}. Filter: {}.",
            RUNTIME_POLICY_VERSION,
            EXPECTED_EBPF_POLICY_VERSION,
            filter_label(self.filter)
        );
    }

    fn cycle_filter(&mut self) {
        self.filter = match self.filter {
            PolicyFilter::All => PolicyFilter::AuthDenied,
            PolicyFilter::AuthDenied => PolicyFilter::UserChanges,
            PolicyFilter::UserChanges => PolicyFilter::ObjectChanges,
            PolicyFilter::ObjectChanges => PolicyFilter::All,
        };
        self.refresh();
    }
}

impl Screen for DspPolicy {
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
            KeyCode::F(6) | KeyCode::Char('1') | KeyCode::Char('2') | KeyCode::Char('0') => {
                self.cycle_filter();
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
            _ => ScreenResult::none(),
        }
    }
}

impl DspPolicy {
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(
            Paragraph::new("F5=Refresh F6=Filter")
                .style(STYLE_NORMAL)
                .block(
                    Block::default()
                        .title(" DSPPOLICY ")
                        .style(STYLE_HEADER)
                        .borders(Borders::ALL)
                        .border_style(STYLE_BORDER),
                ),
            area,
        );
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        HelpBar::new()
            .command("DSPPOLICY")
            .actions(vec![
                HelpAction::new("F3", "Back"),
                HelpAction::new("F5", "Refresh"),
                HelpAction::new("F6", "Filter"),
                HelpAction::new("F12", "Cancel"),
            ])
            .render(frame, area);
    }
}

fn audit_gap_summary(filter: PolicyFilter) -> String {
    let Ok(records) = read_audit_records(200) else {
        return "audit unavailable".to_string();
    };
    let count = records
        .iter()
        .filter(|record| match filter {
            PolicyFilter::All => false,
            PolicyFilter::AuthDenied => record.event.contains("DENIED"),
            PolicyFilter::UserChanges => {
                record.event.contains("USER") || record.object.contains("USRPRF")
            }
            PolicyFilter::ObjectChanges => {
                record.event.contains("OBJECT") || record.event.contains("AUTH_CHANGE")
            }
        })
        .count();
    if count == 0 {
        "none".to_string()
    } else {
        format!("{count} matching audit signal(s)")
    }
}

fn filter_label(filter: PolicyFilter) -> &'static str {
    match filter {
        PolicyFilter::All => "all",
        PolicyFilter::AuthDenied => "auth denied",
        PolicyFilter::UserChanges => "user changes",
        PolicyFilter::ObjectChanges => "object changes",
    }
}

impl Default for DspPolicy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_filter_labels_are_operator_visible() {
        assert_eq!(filter_label(PolicyFilter::AuthDenied), "auth denied");
        let mut screen = DspPolicy::new();
        assert!(screen.rows.iter().any(|row| row[0] == "*PGM"));
        screen.cycle_filter();
        assert_eq!(screen.filter, PolicyFilter::AuthDenied);
    }
}
