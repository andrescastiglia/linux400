use crossterm::event::{KeyCode, KeyEvent};
use l400::{
    WorkloadJob, WorkloadType, end_job, get_workload_params, hold_job, is_cgroup_v2_available,
    kill_job, list_jobs, release_job, subsystem_descriptions,
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::Line,
    widgets::{Block, Borders, Paragraph},
};
use std::time::{Duration, Instant};

use crate::screens::{Screen, ScreenId, ScreenResult};
use crate::style::*;
use crate::widgets::help_bar::{HelpAction, HelpBar};
use crate::widgets::subfile_table::SubfileTable;

#[derive(Clone, Debug)]
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
    filtered_indices: Vec<usize>,
    table: SubfileTable,
    subsystem_filter: Option<String>,
    user_filter: Option<String>,
    status_filter: Option<String>,
    auto_refresh: bool,
    last_refresh: Instant,
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
        let mut screen = Self {
            jobs: Vec::new(),
            filtered_indices: Vec::new(),
            table: SubfileTable::new(
                vec!["Opt", "Job", "User", "Type", "Status", "Subsystem", "PID"],
                vec![3, 14, 12, 10, 14, 12, 8],
            )
            .with_title("Active jobs"),
            subsystem_filter: None,
            user_filter: None,
            status_filter: None,
            auto_refresh: true,
            last_refresh: Instant::now(),
            detail: None,
            pending_action: None,
        };
        screen.refresh();
        screen
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

    fn load_jobs() -> Vec<JobInfo> {
        list_jobs()
            .map(|jobs| jobs.into_iter().map(Self::map_job).collect())
            .unwrap_or_default()
    }

    fn refresh(&mut self) {
        self.jobs = Self::load_jobs();
        self.apply_filters();
        self.last_refresh = Instant::now();
    }

    fn apply_filters(&mut self) {
        self.filtered_indices = self
            .jobs
            .iter()
            .enumerate()
            .filter(|(_, job)| {
                self.subsystem_filter
                    .as_deref()
                    .map(|value| job.subsystem.eq_ignore_ascii_case(value))
                    .unwrap_or(true)
                    && self
                        .user_filter
                        .as_deref()
                        .map(|value| job.user.eq_ignore_ascii_case(value))
                        .unwrap_or(true)
                    && self
                        .status_filter
                        .as_deref()
                        .map(|value| job.status.eq_ignore_ascii_case(value))
                        .unwrap_or(true)
            })
            .map(|(index, _)| index)
            .collect();

        let rows = self
            .filtered_indices
            .iter()
            .filter_map(|index| self.jobs.get(*index))
            .map(|job| {
                vec![
                    " ".to_string(),
                    job.name.clone(),
                    job.user.clone(),
                    job.type_.clone(),
                    job.status.clone(),
                    job.subsystem.clone(),
                    job.pid.to_string(),
                ]
            })
            .collect::<Vec<_>>();
        self.table.set_rows(rows);
    }

    fn selected_job(&self) -> Option<&JobInfo> {
        let visible_index = self.table.selected()?;
        let job_index = *self.filtered_indices.get(visible_index)?;
        self.jobs.get(job_index)
    }

    fn cycle_subsystem_filter(&mut self) {
        self.subsystem_filter = match self.subsystem_filter.as_deref() {
            None => Some("QINTER".to_string()),
            Some("QINTER") => Some("QBATCH".to_string()),
            _ => None,
        };
        self.apply_filters();
    }

    fn cycle_status_filter(&mut self) {
        self.status_filter = match self.status_filter.as_deref() {
            None => Some("ACTIVE".to_string()),
            Some("ACTIVE") => Some("HELD".to_string()),
            _ => None,
        };
        self.apply_filters();
    }

    fn cycle_user_filter(&mut self) {
        self.user_filter = match self.user_filter.as_deref() {
            None => Some(l400::current_l400_user()),
            Some(_) => None,
        };
        self.apply_filters();
    }

    fn show_detail(&mut self) {
        self.detail = self.selected_job().map(|job| {
            format!(
                "Job: {}  PID: {}  User: {}  Status: {}  SBS: {}\nCommand: {}\nLog: {}",
                job.name, job.pid, job.user, job.status, job.subsystem, job.command, job.log_path
            )
        });
        if self.detail.is_none() {
            self.detail = Some("No job selected.".to_string());
        }
    }

    fn hold_selected_job(&mut self) {
        let Some(job) = self.selected_job() else {
            self.detail = Some("No job selected.".to_string());
            return;
        };
        let pid = job.pid;
        let name = job.name.clone();
        self.detail = match hold_job(pid) {
            Ok(_) => Some(format!("Job {name} PID={pid} held.")),
            Err(error) => Some(format!("Error holding job {name}: {error}")),
        };
        self.refresh();
    }

    fn release_selected_job(&mut self) {
        let Some(job) = self.selected_job() else {
            self.detail = Some("No job selected.".to_string());
            return;
        };
        let pid = job.pid;
        let name = job.name.clone();
        self.detail = match release_job(pid) {
            Ok(_) => Some(format!("Job {name} PID={pid} released.")),
            Err(error) => Some(format!("Error releasing job {name}: {error}")),
        };
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
            "Confirm {} end for job {} PID={}. Enter=Confirm F12=Cancel.",
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
        self.detail = match result {
            Ok(_) => Some(format!("Job {name} PID={pid} ended.")),
            Err(error) => Some(format!("Error ending job {name}: {error}")),
        };
        self.refresh();
    }
}

impl Screen for WorkManagement {
    fn render(&mut self, frame: &mut Frame) {
        if self.auto_refresh && self.last_refresh.elapsed() >= Duration::from_secs(5) {
            self.refresh();
        }

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
            return match key.code {
                KeyCode::Enter => {
                    self.confirm_end_job();
                    ScreenResult::none()
                }
                KeyCode::F(12) | KeyCode::Esc => {
                    self.pending_action = None;
                    self.detail = Some("End job cancelled.".to_string());
                    ScreenResult::none()
                }
                _ => ScreenResult::none(),
            };
        }

        match key.code {
            KeyCode::F(3) | KeyCode::F(12) => ScreenResult::back(),
            KeyCode::F(4) => ScreenResult::goto(ScreenId::CommandLine),
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
            KeyCode::F(5) => {
                self.refresh();
                ScreenResult::none()
            }
            KeyCode::F(6) => {
                self.cycle_subsystem_filter();
                ScreenResult::none()
            }
            KeyCode::F(7) => {
                self.cycle_status_filter();
                ScreenResult::none()
            }
            KeyCode::F(8) => {
                self.cycle_user_filter();
                ScreenResult::none()
            }
            KeyCode::F(21) => {
                self.auto_refresh = !self.auto_refresh;
                self.detail = Some(format!(
                    "Auto-refresh {}.",
                    if self.auto_refresh {
                        "enabled"
                    } else {
                        "disabled"
                    }
                ));
                ScreenResult::none()
            }
            KeyCode::Char('8') => self
                .selected_job()
                .map(|job| ScreenResult::with_data(ScreenId::WrkJob, job.pid.to_string()))
                .unwrap_or_else(|| {
                    self.detail = Some("No job selected.".to_string());
                    ScreenResult::none()
                }),
            KeyCode::Enter | KeyCode::F(11) | KeyCode::Char('5') => {
                self.show_detail();
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
        let block = Block::default()
            .title(" Work Management ")
            .style(STYLE_HEADER)
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);
        frame.render_widget(block, area);

        let source = if is_cgroup_v2_available() {
            "Runtime workloads"
        } else {
            "Runtime workloads (degraded)"
        };
        let active = self
            .jobs
            .iter()
            .filter(|job| job.status == "ACTIVE")
            .count();
        let lines = vec![
            Line::from(format!(
                "{source}. Active/Total: {}/{} Auto-refresh:{}",
                active,
                self.jobs.len(),
                if self.auto_refresh { "ON" } else { "OFF" }
            )),
            Line::from(format!(
                "Filters SBS({}) USER({}) STATUS({})   {}   {}",
                self.subsystem_filter.as_deref().unwrap_or("*ALL"),
                self.user_filter.as_deref().unwrap_or("*ALL"),
                self.status_filter.as_deref().unwrap_or("*ALL"),
                subsystem_summary(),
                cgroup_summary()
            )),
        ];
        let text = ratatui::text::Text::from(lines);
        let inner = Rect::new(area.x + 1, area.y + 1, area.width.saturating_sub(2), 2);
        frame.render_widget(Paragraph::new(text).style(STYLE_NORMAL), inner);
    }

    fn render_jobs(&mut self, frame: &mut Frame, area: Rect) {
        self.table.render(frame, area);

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
        HelpBar::new()
            .command("WRKACTJOB")
            .actions(vec![
                HelpAction::new("F3", "Back"),
                HelpAction::new("F5", "Refresh"),
                HelpAction::new("F6/F7/F8", "Filters"),
                HelpAction::new("F21", "Auto"),
                HelpAction::new("3/4/5/6/8", "Options"),
                HelpAction::new("F10", "End immed"),
                HelpAction::new("F12", "Cancel"),
            ])
            .render(frame, area);
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
        screen.apply_filters();

        screen.handle_key(KeyEvent::from(KeyCode::Char('3')));
        assert_eq!(screen.detail.as_deref(), Some("No job selected."));

        screen.handle_key(KeyEvent::from(KeyCode::Char('6')));
        assert_eq!(screen.detail.as_deref(), Some("No job selected."));
    }

    #[test]
    fn auto_refresh_toggle_changes_state() {
        let mut screen = WorkManagement::new();
        assert!(screen.auto_refresh);
        screen.handle_key(KeyEvent::from(KeyCode::F(21)));
        assert!(!screen.auto_refresh);
    }
}
