use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::process::Command;

use crate::screens::{Screen, ScreenId, ScreenResult};
use crate::session::SessionContext;
use crate::style::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdminViewKind {
    ObjectDetail,
    UserProfiles,
    PolicyAudit,
    SpoolOutq,
}

pub struct AdminCommandView {
    kind: AdminViewKind,
    title: String,
    commands: Vec<String>,
    lines: Vec<String>,
    scroll: usize,
    status: String,
    session: SessionContext,
}

impl AdminCommandView {
    pub fn object_detail(data: Option<&str>, session: SessionContext) -> Self {
        let object_spec = data
            .and_then(extract_object_spec)
            .unwrap_or_else(|| "QGPL/*ALL".to_string());
        Self::new(
            AdminViewKind::ObjectDetail,
            format!("Display Object Detail  {}", object_spec),
            vec![
                format!("DSPOBJD OBJ({object_spec})"),
                format!("DSPOBJAUT OBJ({object_spec})"),
            ],
            session,
        )
    }

    pub fn user_profiles(session: SessionContext) -> Self {
        Self::new(
            AdminViewKind::UserProfiles,
            "Work with User Profiles".to_string(),
            vec!["WRKUSRPRF".to_string()],
            session,
        )
    }

    pub fn policy_audit(data: Option<&str>, session: SessionContext) -> Self {
        let commands = match data.map(|value| value.trim().to_uppercase()) {
            Some(command) if command.starts_with("DSPAUD") => vec![command],
            Some(command) if command.starts_with("DSPPOLICY") => vec![command],
            _ => vec!["DSPPOLICY".to_string(), "DSPAUD".to_string()],
        };
        Self::new(
            AdminViewKind::PolicyAudit,
            "Policy and Audit".to_string(),
            commands,
            session,
        )
    }

    pub fn spool_outq(data: Option<&str>, session: SessionContext) -> Self {
        let commands = match data.map(|value| value.trim().to_uppercase()) {
            Some(command) if command.starts_with("WRKOUTQ") => vec![command],
            Some(command) if command.starts_with("WRKSPLF") => vec![command],
            _ => vec!["WRKSPLF".to_string(), "WRKOUTQ".to_string()],
        };
        Self::new(
            AdminViewKind::SpoolOutq,
            "Work with Spool and Output Queues".to_string(),
            commands,
            session,
        )
    }

    fn new(
        kind: AdminViewKind,
        title: String,
        commands: Vec<String>,
        session: SessionContext,
    ) -> Self {
        let mut view = Self {
            kind,
            title,
            commands,
            lines: Vec::new(),
            scroll: 0,
            status: String::new(),
            session,
        };
        view.refresh();
        view
    }

    fn refresh(&mut self) {
        self.lines.clear();
        let state = self.session.snapshot();
        for command in &self.commands {
            self.lines.push(format!("==> {}", command));
            match Command::new("l400cmd")
                .args(command.split_whitespace())
                .env("L400_USER", &state.user_profile)
                .env("L400_CURLIB", &state.current_library)
                .env("L400_LIBLIST", state.library_list.join(":"))
                .output()
            {
                Ok(output) => {
                    self.lines.extend(
                        String::from_utf8_lossy(&output.stdout)
                            .lines()
                            .chain(String::from_utf8_lossy(&output.stderr).lines())
                            .map(str::to_string),
                    );
                    self.lines.push(format!(
                        "Command status: {}",
                        output.status.code().unwrap_or_default()
                    ));
                }
                Err(error) => self
                    .lines
                    .push(format!("No se pudo ejecutar '{}': {}", command, error)),
            }
            self.lines.push(String::new());
        }
        if self.lines.is_empty() {
            self.lines.push("Sin datos para mostrar.".to_string());
        }
        self.scroll = 0;
        self.status = match self.kind {
            AdminViewKind::ObjectDetail => {
                "Opciones: F5=Refresh, F12=Volver, F4=Prompt.".to_string()
            }
            AdminViewKind::UserProfiles => {
                "WRKUSRPRF dedicado. Cree/desactive perfiles desde command prompt.".to_string()
            }
            AdminViewKind::PolicyAudit => "DSPPOLICY/DSPAUD dedicado.".to_string(),
            AdminViewKind::SpoolOutq => "WRKSPLF/WRKOUTQ dedicado.".to_string(),
        };
    }
}

impl Screen for AdminCommandView {
    fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(frame.area());

        self.render_header(frame, chunks[0]);
        self.render_body(frame, chunks[1]);
        self.render_status(frame, chunks[2]);
        self.render_help(frame, chunks[3]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> ScreenResult {
        match key.code {
            KeyCode::F(3) | KeyCode::F(12) => ScreenResult::goto(ScreenId::MainMenu),
            KeyCode::F(4) => ScreenResult::goto(ScreenId::CommandLine),
            KeyCode::F(5) => {
                self.refresh();
                ScreenResult::none()
            }
            KeyCode::Up => {
                self.scroll = self.scroll.saturating_sub(1);
                ScreenResult::none()
            }
            KeyCode::Down => {
                self.scroll = self
                    .scroll
                    .saturating_add(1)
                    .min(self.lines.len().saturating_sub(1));
                ScreenResult::none()
            }
            KeyCode::PageUp => {
                self.scroll = self.scroll.saturating_sub(10);
                ScreenResult::none()
            }
            KeyCode::PageDown => {
                self.scroll = self
                    .scroll
                    .saturating_add(10)
                    .min(self.lines.len().saturating_sub(1));
                ScreenResult::none()
            }
            _ => ScreenResult::none(),
        }
    }
}

impl AdminCommandView {
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(format!(" {} ", self.title))
            .style(STYLE_HEADER)
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(
            Paragraph::new(self.commands.join("  |  ")).style(STYLE_NORMAL),
            inner,
        );
    }

    fn render_body(&self, frame: &mut Frame, area: Rect) {
        let height = area.height.saturating_sub(2) as usize;
        let text = self
            .lines
            .iter()
            .skip(self.scroll)
            .take(height)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(Paragraph::new(text).style(STYLE_NORMAL), inner);
    }

    fn render_status(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(
            Paragraph::new(self.status.clone()).style(STYLE_NORMAL),
            inner,
        );
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let help_text = Line::from(vec![
            "F3=Exit   ".into(),
            "F4=Prompt   ".into(),
            "F5=Refresh   ".into(),
            "PgUp/PgDn=Roll   ".into(),
            "F12=Cancel".into(),
        ]);
        let block = Block::default()
            .style(STYLE_HELP)
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(Paragraph::new(help_text).style(STYLE_HELP), inner);
    }
}

fn extract_object_spec(data: &str) -> Option<String> {
    let value = data.trim();
    if value.contains('/') && !value.contains('(') {
        return Some(value.to_uppercase());
    }
    let upper = value.to_uppercase();
    let start = upper.find("OBJ(")?;
    let rest = &value[start + 4..];
    let end = rest.find(')')?;
    let spec = rest[..end].trim();
    (!spec.is_empty()).then(|| spec.to_uppercase())
}
