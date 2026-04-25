use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::path::PathBuf;
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
    object_spec: Option<String>,
    line_filter: Option<String>,
    pending_action: Option<PendingAction>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PendingAction {
    DeleteObject(String),
}

impl AdminCommandView {
    pub fn object_detail(data: Option<&str>, session: SessionContext) -> Self {
        let object_spec = data
            .and_then(extract_object_spec)
            .unwrap_or_else(|| "QGPL/*ALL".to_string());
        let mut view = Self::new(
            AdminViewKind::ObjectDetail,
            format!("Display Object Detail  {}", object_spec),
            vec![
                format!("DSPOBJD OBJ({object_spec})"),
                format!("DSPOBJAUT OBJ({object_spec})"),
            ],
            session,
        );
        view.object_spec = Some(object_spec);
        view
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
            object_spec: None,
            line_filter: None,
            pending_action: None,
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
                .args(tokenize_cl_command(command))
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
        if let Some(filter) = &self.line_filter {
            let filter = filter.to_uppercase();
            self.lines.retain(|line| {
                line.trim().is_empty()
                    || line.starts_with("==>")
                    || line.to_uppercase().contains(&filter)
            });
            self.lines
                .insert(0, format!("Filtro activo: contiene '{}'", filter));
        }
        if self.lines.is_empty() {
            self.lines.push("Sin datos para mostrar.".to_string());
        }
        self.scroll = 0;
        self.status = match self.kind {
            AdminViewKind::ObjectDetail => {
                "Opciones: 2=Change text 3=Copy 4=Delete 8=Authorities F5=Refresh.".to_string()
            }
            AdminViewKind::UserProfiles => {
                "Opciones: 2=Create QPGMR2 4=Disable QPGMR2 5=Display QPGMR.".to_string()
            }
            AdminViewKind::PolicyAudit => {
                "Opciones: 1=Denied 2=User changes 0=All F5=Refresh.".to_string()
            }
            AdminViewKind::SpoolOutq => {
                "Opciones: 5=Display first spool 0=List F5=Refresh.".to_string()
            }
        };
    }

    fn run_l400_command(&mut self, command: &str) {
        let state = self.session.snapshot();
        self.lines.clear();
        self.lines.push(format!("==> {}", command));
        match Command::new("l400cmd")
            .args(tokenize_cl_command(command))
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
                self.status = format!(
                    "{} status={}",
                    command,
                    output.status.code().unwrap_or_default()
                );
            }
            Err(error) => {
                self.lines
                    .push(format!("No se pudo ejecutar '{}': {}", command, error));
                self.status = "Runtime no disponible.".to_string();
            }
        }
        self.scroll = 0;
    }

    fn display_first_spool_file(&mut self) {
        let Some(path) = first_spool_file() else {
            self.lines = vec!["No hay spool files para visualizar.".to_string()];
            self.status = "Spool vacio.".to_string();
            return;
        };
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                self.lines = std::iter::once(format!("==> DSPSPLF {}", path.display()))
                    .chain(content.lines().map(str::to_string))
                    .collect();
                self.status = format!("Mostrando {}", path.display());
            }
            Err(error) => {
                self.lines = vec![format!("No se pudo leer {}: {}", path.display(), error)];
                self.status = "Error leyendo spool.".to_string();
            }
        }
        self.scroll = 0;
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
        if let Some(action) = self.pending_action.clone() {
            match key.code {
                KeyCode::Enter => {
                    match action {
                        PendingAction::DeleteObject(spec) => {
                            self.run_l400_command(&format!("DLTOBJ OBJ({spec}) CONFIRM(*YES)"));
                            self.pending_action = None;
                        }
                    }
                    return ScreenResult::none();
                }
                KeyCode::F(12) | KeyCode::Esc => {
                    self.pending_action = None;
                    self.status = "Accion cancelada.".to_string();
                    return ScreenResult::none();
                }
                _ => return ScreenResult::none(),
            }
        }

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
            KeyCode::Char('0') => {
                self.line_filter = None;
                self.refresh();
                ScreenResult::none()
            }
            KeyCode::Char('1') if self.kind == AdminViewKind::PolicyAudit => {
                self.line_filter = Some("AUTH_DENIED".to_string());
                self.refresh();
                ScreenResult::none()
            }
            KeyCode::Char('2') if self.kind == AdminViewKind::PolicyAudit => {
                self.line_filter = Some("USRPRF_CHANGE".to_string());
                self.refresh();
                ScreenResult::none()
            }
            KeyCode::Char('2') if self.kind == AdminViewKind::ObjectDetail => {
                if let Some(spec) = self.object_spec.clone() {
                    self.run_l400_command(&format!("CHGOBJD OBJ({spec}) TEXT('Changed from TUI')"));
                }
                ScreenResult::none()
            }
            KeyCode::Char('3') if self.kind == AdminViewKind::ObjectDetail => {
                if let Some(spec) = self.object_spec.clone() {
                    let to_spec = format!("{spec}_COPY");
                    self.run_l400_command(&format!("CPYOBJ OBJ({spec}) TOOBJ({to_spec})"));
                }
                ScreenResult::none()
            }
            KeyCode::Char('4') if self.kind == AdminViewKind::ObjectDetail => {
                if let Some(spec) = self.object_spec.clone() {
                    self.pending_action = Some(PendingAction::DeleteObject(spec.clone()));
                    self.status = format!("Confirmar DLTOBJ {spec}: Enter=Confirm F12=Cancel.");
                }
                ScreenResult::none()
            }
            KeyCode::Char('8') if self.kind == AdminViewKind::ObjectDetail => {
                if let Some(spec) = self.object_spec.clone() {
                    self.run_l400_command(&format!("DSPOBJAUT OBJ({spec})"));
                }
                ScreenResult::none()
            }
            KeyCode::Char('2') if self.kind == AdminViewKind::UserProfiles => {
                self.run_l400_command("WRKUSRPRF USRPRF(QPGMR2) ACTION(*CREATE)");
                ScreenResult::none()
            }
            KeyCode::Char('4') if self.kind == AdminViewKind::UserProfiles => {
                self.run_l400_command("WRKUSRPRF USRPRF(QPGMR2) ACTION(*DISABLE)");
                ScreenResult::none()
            }
            KeyCode::Char('5') if self.kind == AdminViewKind::UserProfiles => {
                self.run_l400_command("WRKUSRPRF USRPRF(QPGMR)");
                ScreenResult::none()
            }
            KeyCode::Char('5') if self.kind == AdminViewKind::SpoolOutq => {
                self.display_first_spool_file();
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
            "2/3/4/5/8=Options   ".into(),
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

fn tokenize_cl_command(command: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    let mut in_single = false;
    let mut in_double = false;

    for ch in command.chars() {
        match ch {
            '\'' if !in_double => {
                in_single = !in_single;
                current.push(ch);
            }
            '"' if !in_single => {
                in_double = !in_double;
                current.push(ch);
            }
            '(' if !in_single && !in_double => {
                depth += 1;
                current.push(ch);
            }
            ')' if !in_single && !in_double => {
                depth = depth.saturating_sub(1);
                current.push(ch);
            }
            ch if ch.is_whitespace() && depth == 0 && !in_single && !in_double => {
                if !current.trim().is_empty() {
                    tokens.push(current.trim().to_string());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }

    if !current.trim().is_empty() {
        tokens.push(current.trim().to_string());
    }

    tokens
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

fn first_spool_file() -> Option<PathBuf> {
    let root = l400::resolve_l400_root();
    let candidates = [
        std::env::var("L400_SPOOL_DIR").ok().map(PathBuf::from),
        Some(root.join("QUSRSYS").join("QSPL")),
        Some(root.join("spool")),
    ];
    candidates
        .into_iter()
        .flatten()
        .filter(|dir| dir.exists())
        .find_map(|dir| {
            std::fs::read_dir(dir)
                .ok()?
                .flatten()
                .map(|entry| entry.path())
                .find(|path| path.is_file())
        })
}
