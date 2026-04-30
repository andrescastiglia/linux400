use crossterm::event::{KeyCode, KeyEvent};
use l400::{
    WorkloadJob, WorkloadType, end_job, get_workload_params, hold_job, is_cgroup_v2_available,
    kill_job, list_jobs, release_job, subsystem_descriptions,
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::Line,
    widgets::{Block, Borders, Paragraph, Row, Table, TableState},
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
    pending_action: Option<PendingJobAction>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PendingJobAction {
    EndControlled(u64),
    EndImmediate(u64),
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
            pending_action: None,
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

    fn show_selected_log(&mut self) {
        self.detail = self.selected_job().map(|job| {
            if job.log_path == "-" {
                return format!("Job {} no tiene log persistido.", job.name);
            }
            match std::fs::read_to_string(&job.log_path) {
                Ok(content) => {
                    let tail = content
                        .lines()
                        .rev()
                        .take(3)
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect::<Vec<_>>()
                        .join("\n");
                    format!("Log: {}\n{}", job.log_path, tail)
                }
                Err(error) => format!("No se pudo leer log {}: {}", job.log_path, error),
            }
        });
    }

    fn hold_selected_job(&mut self) {
        let Some(job) = self.selected_job() else {
            self.detail = Some("No job selected.".to_string());
            return;
        };
        let pid = job.pid;
        let name = job.name.clone();
        match hold_job(pid) {
            Ok(_) => self.detail = Some(format!("Job {} PID={} held.", name, pid)),
            Err(error) => self.detail = Some(format!("Error holding job {}: {}", name, error)),
        }
        self.refresh();
    }

    fn release_selected_job(&mut self) {
        let Some(job) = self.selected_job() else {
            self.detail = Some("No job selected.".to_string());
            return;
        };
        let pid = job.pid;
        let name = job.name.clone();
        match release_job(pid) {
            Ok(_) => self.detail = Some(format!("Job {} PID={} released.", name, pid)),
            Err(error) => self.detail = Some(format!("Error releasing job {}: {}", name, error)),
        }
        self.refresh();
    }

    fn request_end_selected_job(&mut self, immediate: bool) {
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
        self.pending_action = Some(if immediate {
            PendingJobAction::EndImmediate(pid)
        } else {
            PendingJobAction::EndControlled(pid)
        });
        self.detail = Some(format!(
            "Confirm {} end for job {} PID={}. Press Enter to confirm or F12 to cancel.",
            if immediate { "*IMMED" } else { "*CNTRLD" },
            name,
            pid
        ));
    }

    fn confirm_end_job(&mut self) {
        let Some(action) = self.pending_action.take() else {
            return;
        };
        let pid = match action {
            PendingJobAction::EndControlled(pid) | PendingJobAction::EndImmediate(pid) => pid,
        };
        let name = self
            .jobs
            .iter()
            .find(|job| job.pid == pid)
            .map(|job| job.name.clone())
            .unwrap_or_else(|| pid.to_string());
        let result = match action {
            PendingJobAction::EndControlled(_) => end_job(pid),
            PendingJobAction::EndImmediate(_) => kill_job(pid),
        };
        match result {
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
        if self.pending_action.is_some() {
            match key.code {
                KeyCode::Enter => {
                    self.confirm_end_job();
                    return ScreenResult::none();
                }
                KeyCode::F(12) | KeyCode::Esc => {
                    self.pending_action = None;
                    self.detail = Some("End job cancelled.".to_string());
                    return ScreenResult::none();
                }
                _ => return ScreenResult::none(),
            }
        }
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
            KeyCode::Char('9') => {
                self.show_selected_log();
                ScreenResult::none()
            }
            KeyCode::F(10) => {
                self.request_end_selected_job(true);
                ScreenResult::none()
            }
            KeyCode::Char('3') => {
                self.hold_selected_job();
                ScreenResult::none()
            }
            KeyCode::Char('4') => {
                self.request_end_selected_job(false);
                ScreenResult::none()
            }
            KeyCode::Char('6') => {
                self.release_selected_job();
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
            Line::from(vec![
                format!(
                    "Source: {}. Filter SBS({}). Type options, press Enter.",
                    source, filter
                )
                .into(),
            ]),
            Line::from(vec![
                format!("{}   {}", subsystem_summary(), cgroup_summary()).into(),
            ]),
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
            "3=Hold   ".into(),
            "4=End   ".into(),
            "5=Detail   ".into(),
            "6=Release   ".into(),
            "9=Log   ".into(),
            "F10=End immed   ".into(),
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

#[cfg(test)]
mod tests {
    use super::WorkManagement;
    use crate::screens::Screen;
    use crossterm::event::{KeyCode, KeyEvent};

    #[test]
    fn job_options_without_selection_show_operator_message() {
        let mut screen = WorkManagement::new();
        screen.jobs.clear();
        screen.state.select(None);

        screen.handle_key(KeyEvent::from(KeyCode::Char('3')));
        assert_eq!(screen.detail.as_deref(), Some("No job selected."));

        screen.handle_key(KeyEvent::from(KeyCode::Char('6')));
        assert_eq!(screen.detail.as_deref(), Some("No job selected."));
    }
}
