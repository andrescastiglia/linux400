use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
};
use std::process::Command;

use crate::cl_parser::tokenize_cl_command;
use crate::screens::{Screen, ScreenId, ScreenResult};
use crate::session::SessionContext;
use crate::style::*;
use crate::widgets::help_bar::{HelpAction, HelpBar};
use crate::widgets::input_field::InputField;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Field {
    Cmd,
    Job,
    Jobq,
    User,
}

pub struct SubmitJob {
    cmd: InputField,
    job: InputField,
    jobq: InputField,
    user: InputField,
    active: Field,
    status: String,
    session: SessionContext,
}

impl SubmitJob {
    pub fn new(session: SessionContext) -> Self {
        let user = session.snapshot().user_profile;
        Self {
            cmd: InputField::new("CMD", 60)
                .with_value("WRKSYSSTS")
                .required(),
            job: InputField::new("JOB", 16)
                .with_value("BATCHJOB")
                .uppercase()
                .required(),
            jobq: InputField::new("JOBQ", 16)
                .with_value("QBATCH")
                .uppercase()
                .required(),
            user: InputField::new("USER", 16)
                .with_value(user)
                .uppercase()
                .required(),
            active: Field::Cmd,
            status: "Enter job parameters.".to_string(),
            session,
        }
    }

    fn active_field_mut(&mut self) -> &mut InputField {
        match self.active {
            Field::Cmd => &mut self.cmd,
            Field::Job => &mut self.job,
            Field::Jobq => &mut self.jobq,
            Field::User => &mut self.user,
        }
    }

    fn move_focus(&mut self) {
        self.active = match self.active {
            Field::Cmd => Field::Job,
            Field::Job => Field::Jobq,
            Field::Jobq => Field::User,
            Field::User => Field::Cmd,
        };
    }

    fn submit(&mut self) -> ScreenResult {
        let command = format!(
            "SBMJOB CMD({}) JOB({}) JOBQ({}) USER({})",
            self.cmd.value.trim(),
            self.job.value.trim(),
            self.jobq.value.trim(),
            self.user.value.trim()
        );
        let state = self.session.snapshot();
        match Command::new("l400cmd")
            .args(tokenize_cl_command(&command))
            .env("L400_USER", &state.user_profile)
            .env("L400_CURLIB", &state.current_library)
            .env("L400_LIBLIST", state.library_list.join(":"))
            .output()
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let tail = stdout
                    .lines()
                    .chain(stderr.lines())
                    .last()
                    .unwrap_or("Job submitted.");
                self.status = format!(
                    "Job {} submitted to {}. {}",
                    self.job.value.trim(),
                    self.jobq.value.trim(),
                    tail
                );
                self.session.set_last_message(self.status.clone());
                ScreenResult::goto(ScreenId::WorkManagement)
            }
            Err(error) => {
                self.status = format!("Error submitting job: {error}");
                ScreenResult::none()
            }
        }
    }
}

impl Screen for SubmitJob {
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
        self.render_fields(frame, chunks[1]);
        self.render_status(frame, chunks[2]);
        self.render_help(frame, chunks[3]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> ScreenResult {
        match key.code {
            KeyCode::F(3) | KeyCode::F(12) | KeyCode::Esc => ScreenResult::back(),
            KeyCode::Tab | KeyCode::Down | KeyCode::Up => {
                self.move_focus();
                ScreenResult::none()
            }
            KeyCode::Enter => self.submit(),
            KeyCode::Backspace => {
                self.active_field_mut().delete_back();
                ScreenResult::none()
            }
            KeyCode::Char(c) => {
                self.active_field_mut().insert_char(c);
                ScreenResult::none()
            }
            _ => ScreenResult::none(),
        }
    }
}

impl SubmitJob {
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(
            Block::default()
                .title(" SBMJOB ")
                .style(STYLE_HEADER)
                .borders(Borders::ALL)
                .border_style(STYLE_BORDER),
            area,
        );
    }

    fn render_fields(&mut self, frame: &mut Frame, area: Rect) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(area);
        self.cmd.active = self.active == Field::Cmd;
        self.job.active = self.active == Field::Job;
        self.jobq.active = self.active == Field::Jobq;
        self.user.active = self.active == Field::User;
        self.cmd.render(frame, rows[0]);
        self.job.render(frame, rows[1]);
        self.jobq.render(frame, rows[2]);
        self.user.render(frame, rows[3]);
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
            .command("SBMJOB")
            .actions(vec![
                HelpAction::new("Enter", "Submit"),
                HelpAction::new("Tab", "Next"),
                HelpAction::new("F3", "Back"),
                HelpAction::new("F12", "Cancel"),
            ])
            .render(frame, area);
    }
}
