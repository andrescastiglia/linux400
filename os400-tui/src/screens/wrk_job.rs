use crossterm::event::{KeyCode, KeyEvent};
use l400::{WorkloadJob, list_jobs};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph},
};

use crate::screens::{Screen, ScreenResult};
use crate::style::*;
use crate::widgets::help_bar::{HelpAction, HelpBar};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum JobTab {
    Detail,
    Log,
    Spool,
    CallStack,
}

pub struct WrkJob {
    pid: u64,
    job: Option<WorkloadJob>,
    tab: JobTab,
    scroll: usize,
    status: String,
}

impl WrkJob {
    pub fn new(pid: u64) -> Self {
        let mut screen = Self {
            pid,
            job: None,
            tab: JobTab::Detail,
            scroll: 0,
            status: String::new(),
        };
        screen.refresh();
        screen
    }

    fn refresh(&mut self) {
        self.job = list_jobs()
            .ok()
            .and_then(|jobs| jobs.into_iter().find(|job| job.pid == self.pid));
        self.status = if self.job.is_some() {
            "Job refreshed.".to_string()
        } else {
            format!("Job PID {} not found.", self.pid)
        };
    }

    fn next_tab(&mut self) {
        self.tab = match self.tab {
            JobTab::Detail => JobTab::Log,
            JobTab::Log => JobTab::Spool,
            JobTab::Spool => JobTab::CallStack,
            JobTab::CallStack => JobTab::Detail,
        };
        self.scroll = 0;
    }

    fn prev_tab(&mut self) {
        self.tab = match self.tab {
            JobTab::Detail => JobTab::CallStack,
            JobTab::Log => JobTab::Detail,
            JobTab::Spool => JobTab::Log,
            JobTab::CallStack => JobTab::Spool,
        };
        self.scroll = 0;
    }

    fn body_lines(&self) -> Vec<String> {
        let Some(job) = &self.job else {
            return vec![self.status.clone()];
        };
        match self.tab {
            JobTab::Detail => vec![
                format!("Name       : {}", job.name),
                format!("User       : {}", job.user),
                format!("PID        : {}", job.pid),
                format!("Status     : {}", job.status),
                format!("Subsystem  : {}", job.subsystem),
                format!("Command    : {}", job.command),
                format!(
                    "Submitted  : {}",
                    job.submitted_at.as_deref().unwrap_or("-")
                ),
                format!("Started    : {}", job.started_at.as_deref().unwrap_or("-")),
                format!("Ended      : {}", job.ended_at.as_deref().unwrap_or("-")),
                format!(
                    "Log path   : {}",
                    job.log_path
                        .as_ref()
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|| "-".to_string())
                ),
            ],
            JobTab::Log => job
                .log_path
                .as_ref()
                .and_then(|path| std::fs::read_to_string(path).ok())
                .map(|content| content.lines().map(str::to_string).collect())
                .unwrap_or_else(|| vec!["No job log available.".to_string()]),
            JobTab::Spool => list_job_spool(&job.name),
            JobTab::CallStack => vec![
                "Native Linux process call stack is not captured yet.".to_string(),
                format!("Command: {}", job.command),
            ],
        }
    }
}

impl Screen for WrkJob {
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
        self.render_body(frame, chunks[1]);
        self.render_status(frame, chunks[2]);
        self.render_help(frame, chunks[3]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> ScreenResult {
        match key.code {
            KeyCode::F(3) | KeyCode::Esc => ScreenResult::back(),
            KeyCode::F(5) => {
                self.refresh();
                ScreenResult::none()
            }
            KeyCode::F(11) => {
                self.next_tab();
                ScreenResult::none()
            }
            KeyCode::F(12) => {
                self.prev_tab();
                ScreenResult::none()
            }
            KeyCode::Up => {
                self.scroll = self.scroll.saturating_sub(1);
                ScreenResult::none()
            }
            KeyCode::Down => {
                self.scroll = self.scroll.saturating_add(1);
                ScreenResult::none()
            }
            _ => ScreenResult::none(),
        }
    }
}

impl WrkJob {
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(format!(" WRKJOB PID {} ", self.pid))
            .style(STYLE_HEADER)
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);
        frame.render_widget(block, area);
        let tabs = "Detail   Log   Spool   Call stack";
        let inner = Rect::new(area.x + 1, area.y + 1, area.width.saturating_sub(2), 2);
        frame.render_widget(
            Paragraph::new(format!("Tab: {:?}    {tabs}", self.tab)).style(STYLE_NORMAL),
            inner,
        );
    }

    fn render_body(&self, frame: &mut Frame, area: Rect) {
        let height = area.height.saturating_sub(2) as usize;
        let lines = self
            .body_lines()
            .into_iter()
            .skip(self.scroll)
            .take(height)
            .map(Line::from)
            .collect::<Vec<_>>();
        frame.render_widget(
            Paragraph::new(Text::from(lines)).style(STYLE_NORMAL).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(STYLE_BORDER),
            ),
            area,
        );
    }

    fn render_status(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(
            Paragraph::new(self.status.clone())
                .style(STYLE_NORMAL)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(STYLE_BORDER),
                ),
            area,
        );
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        HelpBar::new()
            .command("WRKJOB")
            .actions(vec![
                HelpAction::new("F3", "Back"),
                HelpAction::new("F5", "Refresh"),
                HelpAction::new("F11/F12", "Tabs"),
                HelpAction::new("Up/Down", "Roll"),
                HelpAction::new("Esc", "Cancel"),
            ])
            .render(frame, area);
    }
}

fn list_job_spool(job_name: &str) -> Vec<String> {
    let root = l400::resolve_l400_root();
    let candidates = [
        std::env::var_os("L400_SPOOL_DIR").map(std::path::PathBuf::from),
        Some(root.join("QUSRSYS").join("QSPL")),
    ];
    let mut lines = Vec::new();
    for dir in candidates.into_iter().flatten().filter(|dir| dir.exists()) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file()
                    && std::fs::read_to_string(&path)
                        .map(|content| content.contains(&format!("job={job_name}")))
                        .unwrap_or(false)
                {
                    lines.push(path.display().to_string());
                }
            }
        }
    }
    if lines.is_empty() {
        lines.push("No spool files for this job.".to_string());
    }
    lines
}
