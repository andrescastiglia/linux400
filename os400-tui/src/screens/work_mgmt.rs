use crossterm::event::{KeyCode, KeyEvent};
use l400::{
    end_job, get_workload_params, is_cgroup_v2_available, list_jobs, subsystem_descriptions,
    WorkloadJob, WorkloadType,
};
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
    pub pid: u64,
    pub command: String,
    pub log_path: String,
}

pub struct WorkManagement {
    jobs: Vec<JobInfo>,
    state: TableState,
    scroll_offset: usize,
    subsystem_filter: Option<String>,
    detail: Option<String>,
}

impl WorkManagement {
    pub fn new() -> Self {
        let jobs = Self::load_filtered_jobs(None);
        let mut state = TableState::default();
        if !jobs.is_empty() {
            state.select(Some(0));
        }
        Self {
            jobs,
            state,
            scroll_offset: 0,
            subsystem_filter: None,
            detail: None,
        }
    }

    fn map_job(job: WorkloadJob) -> JobInfo {
        JobInfo {
            name: job.name,
            user: job.user,
            type_: match job.workload {
                l400::WorkloadType::Interactive => "INTERACT".to_string(),
                l400::WorkloadType::Batch => "BATCH".to_string(),
            },
            status: job.status.to_string(),
            subsystem: job.subsystem,
            pid: job.pid,
            command: job.command,
            log_path: job
                .log_path
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "-".to_string()),
        }
    }

    fn load_filtered_jobs(filter: Option<&str>) -> Vec<JobInfo> {
        if let Ok(jobs) = list_jobs() {
            return jobs
                .into_iter()
                .filter(|job| {
                    filter
                        .map(|value| job.subsystem.eq_ignore_ascii_case(value))
                        .unwrap_or(true)
                })
                .map(Self::map_job)
                .collect();
        }
        Vec::new()
    }

    fn refresh(&mut self) {
        self.jobs = Self::load_filtered_jobs(self.subsystem_filter.as_deref());
        if self.jobs.is_empty() {
            self.state.select(None);
        } else if self.state.selected().is_none() {
            self.state.select(Some(0));
        }
    }

    fn selected_job(&self) -> Option<&JobInfo> {
        self.state.selected().and_then(|index| self.jobs.get(index))
    }

    fn cycle_subsystem_filter(&mut self) {
        self.subsystem_filter = match self.subsystem_filter.as_deref() {
            None => Some("QINTER".to_string()),
            Some("QINTER") => Some("QBATCH".to_string()),
            _ => None,
        };
        self.refresh();
    }

    fn show_detail(&mut self) {
        self.detail = self.selected_job().map(|job| {
            format!(
                "Job: {}  PID: {}  User: {}  Status: {}  SBS: {}\nCommand: {}\nLog: {}",
                job.name, job.pid, job.user, job.status, job.subsystem, job.command, job.log_path
            )
        });
    }

    fn end_selected_job(&mut self) {
        let Some(job) = self.selected_job() else {
            self.detail = Some("No job selected.".to_string());
            return;
        };
        if job.status != "ACTIVE" {
            self.detail = Some(format!("Job {} is not ACTIVE.", job.name));
            return;
        }
        let pid = job.pid;
        let name = job.name.clone();
        match end_job(pid) {
            Ok(_) => self.detail = Some(format!("Job {} PID={} ended.", name, pid)),
            Err(error) => self.detail = Some(format!("Error ending job {}: {}", name, error)),
        }
        self.refresh();
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
            .split(frame.area());

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
            KeyCode::F(6) => {
                self.cycle_subsystem_filter();
                ScreenResult::none()
            }
            KeyCode::Enter | KeyCode::F(11) => {
                self.show_detail();
                ScreenResult::none()
            }
            KeyCode::Char('5') => {
                self.show_detail();
                ScreenResult::none()
            }
            KeyCode::F(10) => {
                self.end_selected_job();
                ScreenResult::none()
            }
            KeyCode::Char('4') => {
                self.end_selected_job();
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

        let source = if is_cgroup_v2_available() {
            "Runtime workloads"
        } else {
            "Runtime workloads (degraded: cgroups unavailable)"
        };
        let filter = self.subsystem_filter.as_deref().unwrap_or("*ALL");
        let lines: Vec<Line> = vec![
            Line::from(vec![format!(
                "Source: {}. Filter SBS({}). Type options, press Enter.",
                source, filter
            )
            .into()]),
            Line::from(vec![format!(
                "{}   {}",
                subsystem_summary(),
                cgroup_summary()
            )
            .into()]),
        ];
        let text = ratatui::text::Text::from(lines);

        let inner = Rect::new(area.x + 1, area.y + 1, area.width - 2, 2);
        frame.render_widget(Paragraph::new(text).style(STYLE_NORMAL), inner);
    }

    fn render_jobs(&mut self, frame: &mut Frame, area: Rect) {
        let header = ["Opt", "Job", "User", "Type", "Status", "Subsystem", "PID"];
        let widths = [3u16, 14, 12, 10, 14, 12, 8];

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
                    job.pid.to_string(),
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
            .row_highlight_style(STYLE_SELECTION);

        frame.render_stateful_widget(table, area, &mut self.state);

        if let Some(detail) = &self.detail {
            let popup = Rect::new(
                area.x.saturating_add(2),
                area.y.saturating_add(area.height.saturating_sub(5)),
                area.width.saturating_sub(4),
                5.min(area.height),
            );
            let block = Block::default()
                .title(" Job detail ")
                .borders(Borders::ALL)
                .border_style(STYLE_BORDER);
            let inner = block.inner(popup);
            frame.render_widget(block, popup);
            frame.render_widget(Paragraph::new(detail.clone()).style(STYLE_NORMAL), inner);
        }
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let help_text = Line::from(vec![
            "F3=Exit   ".into(),
            "F4=Prompt   ".into(),
            "F5=Refresh   ".into(),
            "F6=Filter   ".into(),
            "F10=End   ".into(),
            "4=End   ".into(),
            "5=Detail   ".into(),
            "F11/Enter=Detail   ".into(),
            "F12=Cancel   ".into(),
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

fn subsystem_summary() -> String {
    subsystem_descriptions()
        .into_iter()
        .map(|(name, text)| format!("{name}={text}"))
        .collect::<Vec<_>>()
        .join("  ")
}

fn cgroup_summary() -> String {
    let qinter = get_workload_params(WorkloadType::Interactive)
        .map(|params| format!("QINTER cpu={} mem={}", params.cpu_weight, params.memory_max))
        .unwrap_or_else(|_| "QINTER degraded".to_string());
    let qbatch = get_workload_params(WorkloadType::Batch)
        .map(|params| format!("QBATCH cpu={} mem={}", params.cpu_weight, params.memory_max))
        .unwrap_or_else(|_| "QBATCH degraded".to_string());
    format!("{qinter}  {qbatch}")
}

impl Default for WorkManagement {
    fn default() -> Self {
        Self::new()
    }
}
