use crossterm::event::{KeyCode, KeyEvent};
use l400::list_jobs;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::Line,
    widgets::{Block, Borders, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::screens::{Screen, ScreenId, ScreenResult};
use crate::style::*;

pub struct JobInfo {
    pub name: String,
    pub user: String,
    pub type_: String,
    pub status: String,
    pub subsystem: String,
}

pub struct WorkManagement {
    jobs: Vec<JobInfo>,
    state: TableState,
    scroll_offset: usize,
    using_runtime_data: bool,
}

impl WorkManagement {
    pub fn new() -> Self {
        let (jobs, using_runtime_data) = Self::load_jobs();
        Self {
            jobs,
            state: TableState::default(),
            scroll_offset: 0,
            using_runtime_data,
        }
    }

    fn fallback_jobs() -> Vec<JobInfo> {
        vec![
            JobInfo {
                name: "QINTER".to_string(),
                user: "QSYS".to_string(),
                type_: "INTERACT".to_string(),
                status: "ACTIVE".to_string(),
                subsystem: "QINTER".to_string(),
            },
            JobInfo {
                name: "QCMD".to_string(),
                user: "QSYS".to_string(),
                type_: "INTERACT".to_string(),
                status: "ACTIVE".to_string(),
                subsystem: "QINTER".to_string(),
            },
            JobInfo {
                name: "QP0ZSPWT".to_string(),
                user: "QSYS".to_string(),
                type_: "SYS".to_string(),
                status: "ACTIVE".to_string(),
                subsystem: "QSYSWRK".to_string(),
            },
            JobInfo {
                name: "QDBSRV01".to_string(),
                user: "QSYS".to_string(),
                type_: "BATCH".to_string(),
                status: "JOBQ".to_string(),
                subsystem: "QBATCH".to_string(),
            },
            JobInfo {
                name: "QDOCSRV".to_string(),
                user: "QSYS".to_string(),
                type_: "BATCH".to_string(),
                status: "ACTIVE".to_string(),
                subsystem: "QBATCH".to_string(),
            },
        ]
    }

    fn load_jobs() -> (Vec<JobInfo>, bool) {
        if let Ok(jobs) = list_jobs() {
            if !jobs.is_empty() {
                let mapped = jobs
                    .into_iter()
                    .map(|job| JobInfo {
                        name: job.name,
                        user: job.user,
                        type_: match job.workload {
                            l400::WorkloadType::Interactive => "INTERACT".to_string(),
                            l400::WorkloadType::Batch => "BATCH".to_string(),
                        },
                        status: job.status,
                        subsystem: job.subsystem,
                    })
                    .collect::<Vec<_>>();
                return (mapped, true);
            }
        }

        (Self::fallback_jobs(), false)
    }

    fn refresh(&mut self) {
        let (jobs, using_runtime_data) = Self::load_jobs();
        self.jobs = jobs;
        self.using_runtime_data = using_runtime_data;
    }
}

impl Screen for WorkManagement {
    fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(frame.size());

        self.render_header(frame, chunks[0]);
        self.render_jobs(frame, chunks[1]);
        self.render_help(frame, chunks[2]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> ScreenResult {
        match key.code {
            KeyCode::F(3) => ScreenResult::goto(ScreenId::MainMenu),
            KeyCode::F(4) => ScreenResult::goto(ScreenId::CommandLine),
            KeyCode::F(12) | KeyCode::Char('q')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                ScreenResult::goto(ScreenId::MainMenu)
            }
            KeyCode::Up => {
                self.state
                    .select(Some(self.state.selected().unwrap_or(0).saturating_sub(1)));
                ScreenResult::none()
            }
            KeyCode::Down => {
                let max = self.jobs.len().saturating_sub(1);
                let current = self.state.selected().unwrap_or(0);
                self.state.select(Some(current.saturating_add(1).min(max)));
                ScreenResult::none()
            }
            KeyCode::PageUp => {
                self.scroll_offset = self.scroll_offset.saturating_sub(10);
                ScreenResult::none()
            }
            KeyCode::PageDown => {
                self.scroll_offset += 10;
                ScreenResult::none()
            }
            KeyCode::F(5) => {
                self.refresh();
                ScreenResult::none()
            }
            _ => ScreenResult::none(),
        }
    }
}

impl WorkManagement {
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let title = Line::from(vec![" Work Management ".into()]);

        let block = Block::default()
            .title(title)
            .style(STYLE_HEADER)
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);

        frame.render_widget(block, area);

        let source = if self.using_runtime_data {
            "Runtime workloads"
        } else {
            "Bundled sample"
        };
        let lines: Vec<Line> = vec![
            Line::from(vec![format!(
                "Source: {}. Type options, press Enter.",
                source
            )
            .into()]),
            Line::from(vec![
                "Opt  Job         User        Type      Status    Subsystem".into(),
            ]),
        ];
        let text = ratatui::text::Text::from(lines);

        let inner = Rect::new(area.x + 1, area.y + 1, area.width - 2, 2);
        frame.render_widget(Paragraph::new(text).style(STYLE_NORMAL), inner);
    }

    fn render_jobs(&mut self, frame: &mut Frame, area: Rect) {
        let header = ["", "Job", "User", "Type", "Status", "Subsystem"];
        let widths = [3u16, 14, 12, 10, 14, 12];

        let rows: Vec<Row> = self
            .jobs
            .iter()
            .map(|job| {
                Row::new(vec![
                    " ".to_string(),
                    job.name.clone(),
                    job.user.clone(),
                    job.type_.clone(),
                    job.status.clone(),
                    job.subsystem.clone(),
                ])
            })
            .collect();

        let table = Table::new(rows, widths.iter().map(|w| Constraint::Length(*w)))
            .header(
                Row::new(header.to_vec())
                    .style(STYLE_TABLE_HEADER)
                    .height(1),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(STYLE_BORDER),
            )
            .style(STYLE_NORMAL)
            .highlight_style(STYLE_SELECTION);

        frame.render_stateful_widget(table, area, &mut self.state);
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let help_text = Line::from(vec![
            "F3=Exit   ".into(),
            "F4=Prompt   ".into(),
            "F5=Refresh   ".into(),
            "F12=Cancel   ".into(),
            "Enter=Select   ".into(),
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

impl Default for WorkManagement {
    fn default() -> Self {
        Self::new()
    }
}
