use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph, Row, Table, TableState},
};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cl_parser::tokenize_cl_command;
use crate::screens::{Screen, ScreenId, ScreenResult};
use crate::session::SessionContext;
use crate::style::*;
use crate::widgets::help_bar::{CpfMessage, HelpAction, HelpBar};

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
    spool_files: Vec<SpoolInfo>,
    spool_state: TableState,
    spool_status_filter: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PendingAction {
    DeleteObject(String),
    DeleteSpool(PathBuf),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SpoolInfo {
    file_name: String,
    path: PathBuf,
    size: u64,
    status: String,
    job: String,
    user: String,
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
            spool_files: Vec::new(),
            spool_state: TableState::default(),
            spool_status_filter: None,
        };
        view.refresh();
        view
    }

    fn refresh(&mut self) {
        if self.kind == AdminViewKind::SpoolOutq {
            self.refresh_spool();
            return;
        }
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
                "Options: 2=Change text 3=Copy 4=Delete 8=Authorities F5=Refresh.".to_string()
            }
            AdminViewKind::UserProfiles => {
                "Options: 2=Create QPGMR2 4=Disable QPGMR2 5=Display QPGMR.".to_string()
            }
            AdminViewKind::PolicyAudit => {
                "Options: 1=Denied 2=User changes 0=All F5=Refresh.".to_string()
            }
            AdminViewKind::SpoolOutq => {
                "Options: 5=Display first spool 0=List F5=Refresh.".to_string()
            }
        };
    }

    fn refresh_spool(&mut self) {
        self.spool_files = list_spool_files()
            .into_iter()
            .filter(|spool| {
                self.spool_status_filter
                    .as_deref()
                    .map(|status| spool.status.eq_ignore_ascii_case(status))
                    .unwrap_or(true)
            })
            .collect();
        if self.spool_files.is_empty() {
            self.spool_state.select(None);
        } else if self
            .spool_state
            .selected()
            .is_none_or(|index| index >= self.spool_files.len())
        {
            self.spool_state.select(Some(0));
        }
        self.lines.clear();
        self.lines.push("==> WRKSPLF".to_string());
        self.lines.push(format!(
            "{} spool file(s), filtro STATUS({})",
            self.spool_files.len(),
            self.spool_status_filter.as_deref().unwrap_or("*ALL")
        ));
        self.status = format!(
            "Options: 5=Display 6=Hold 7=Release 8=Save 4=Delete F6=Status({}).",
            self.spool_status_filter.as_deref().unwrap_or("*ALL")
        );
        self.scroll = 0;
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
        let Some(path) = self
            .selected_spool()
            .map(|spool| spool.path.clone())
            .or_else(first_spool_file)
        else {
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

    fn selected_spool(&self) -> Option<&SpoolInfo> {
        self.spool_state
            .selected()
            .and_then(|index| self.spool_files.get(index))
    }

    fn change_selected_spool_status(&mut self, status: &str) {
        let Some(path) = self.selected_spool().map(|spool| spool.path.clone()) else {
            self.status = "No spool file selected.".to_string();
            return;
        };
        self.run_l400_command(&format!(
            "CHGSPLFA FILE({}) STATUS(*{})",
            path.display(),
            status
        ));
        self.refresh_spool();
        self.status = format!("{} status={}", path.display(), status);
    }

    fn request_delete_selected_spool(&mut self) {
        let Some(path) = self.selected_spool().map(|spool| spool.path.clone()) else {
            self.status = "No hay spool seleccionado.".to_string();
            return;
        };
        self.pending_action = Some(PendingAction::DeleteSpool(path.clone()));
        self.status = format!(
            "DLTSPLF {} pending. Enter=confirm visual delete, F12=cancel.",
            path.display()
        );
    }

    fn cycle_spool_status_filter(&mut self) {
        self.spool_status_filter = match self.spool_status_filter.as_deref() {
            None => Some("READY".to_string()),
            Some("READY") => Some("HELD".to_string()),
            Some("HELD") => Some("SAVED".to_string()),
            _ => None,
        };
        self.refresh_spool();
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
                        PendingAction::DeleteSpool(path) => {
                            self.run_l400_command(&format!(
                                "DLTSPLF FILE({}) CONFIRM(*YES)",
                                path.display()
                            ));
                            self.pending_action = None;
                            self.refresh_spool();
                        }
                    }
                    return ScreenResult::none();
                }
                KeyCode::F(12) | KeyCode::Esc => {
                    self.pending_action = None;
                    self.status = "Action cancelled.".to_string();
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
            KeyCode::F(6) if self.kind == AdminViewKind::SpoolOutq => {
                self.cycle_spool_status_filter();
                ScreenResult::none()
            }
            KeyCode::Up => {
                if self.kind == AdminViewKind::SpoolOutq {
                    let current = self.spool_state.selected().unwrap_or(0);
                    self.spool_state.select(Some(current.saturating_sub(1)));
                } else {
                    self.scroll = self.scroll.saturating_sub(1);
                }
                ScreenResult::none()
            }
            KeyCode::Down => {
                if self.kind == AdminViewKind::SpoolOutq {
                    let max = self.spool_files.len().saturating_sub(1);
                    let current = self.spool_state.selected().unwrap_or(0);
                    self.spool_state
                        .select(Some(current.saturating_add(1).min(max)));
                } else {
                    self.scroll = self
                        .scroll
                        .saturating_add(1)
                        .min(self.lines.len().saturating_sub(1));
                }
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
            KeyCode::Char('6') if self.kind == AdminViewKind::SpoolOutq => {
                self.change_selected_spool_status("HELD");
                ScreenResult::none()
            }
            KeyCode::Char('7') if self.kind == AdminViewKind::SpoolOutq => {
                self.change_selected_spool_status("READY");
                ScreenResult::none()
            }
            KeyCode::Char('8') if self.kind == AdminViewKind::SpoolOutq => {
                self.change_selected_spool_status("SAVED");
                ScreenResult::none()
            }
            KeyCode::Char('4') if self.kind == AdminViewKind::SpoolOutq => {
                self.request_delete_selected_spool();
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

    fn render_body(&mut self, frame: &mut Frame, area: Rect) {
        if self.kind == AdminViewKind::SpoolOutq && !self.spool_files.is_empty() {
            self.render_spool_table(frame, area);
            return;
        }
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

    fn render_spool_table(&mut self, frame: &mut Frame, area: Rect) {
        let rows = self.spool_files.iter().map(|spool| {
            Row::new(vec![
                " ".to_string(),
                spool.file_name.clone(),
                spool.job.clone(),
                spool.user.clone(),
                spool.status.clone(),
                spool.size.to_string(),
            ])
        });
        let table = Table::new(
            rows,
            [
                Constraint::Length(3),
                Constraint::Length(24),
                Constraint::Length(12),
                Constraint::Length(12),
                Constraint::Length(10),
                Constraint::Length(10),
            ],
        )
        .header(
            Row::new(vec!["Opt", "File", "Job", "User", "Status", "Size"])
                .style(STYLE_TABLE_HEADER),
        )
        .block(
            Block::default()
                .title(" Spooled files ")
                .borders(Borders::ALL)
                .border_style(STYLE_BORDER),
        )
        .style(STYLE_NORMAL)
        .row_highlight_style(STYLE_SELECTION);
        frame.render_stateful_widget(table, area, &mut self.spool_state);
    }

    fn render_status(&self, frame: &mut Frame, area: Rect) {
        let cpf = if self.status.to_ascii_lowercase().contains("error") {
            CpfMessage::error("CPF9898", self.status.clone())
        } else {
            CpfMessage::info("CPF0000", self.status.clone())
        };
        cpf.render(frame, area);
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let command = match self.kind {
            AdminViewKind::ObjectDetail => "DSPOBJD",
            AdminViewKind::UserProfiles => "WRKUSRPRF",
            AdminViewKind::PolicyAudit => "DSPPOLICY",
            AdminViewKind::SpoolOutq => "WRKSPLF",
        };
        let mut actions = vec![
            HelpAction::new("F3", "Exit"),
            HelpAction::new("F4", "Prompt"),
            HelpAction::new("F5", "Refresh"),
        ];
        if self.kind == AdminViewKind::SpoolOutq {
            actions.push(HelpAction::new("F6", "Filter"));
            actions.push(HelpAction::new("4/5/6/7/8", "Spool opts"));
        } else {
            actions.push(HelpAction::new("2/3/4/5/8", "Options"));
        }
        actions.push(HelpAction::new("PgUp/PgDn", "Roll"));
        actions.push(HelpAction::new("F12", "Cancel"));

        HelpBar::new()
            .command(command)
            .actions(actions)
            .render(frame, area);
    }
}

// tokenize_cl_command is now in crate::cl_parser

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
    list_spool_files()
        .into_iter()
        .next()
        .map(|spool| spool.path)
}

fn list_spool_files() -> Vec<SpoolInfo> {
    let root = l400::resolve_l400_root();
    let candidates = [
        std::env::var("L400_SPOOL_DIR").ok().map(PathBuf::from),
        Some(root.join("QUSRSYS").join("QSPL")),
        Some(root.join("spool")),
    ];
    let mut files = candidates
        .into_iter()
        .flatten()
        .filter(|dir| dir.exists())
        .flat_map(|dir| {
            std::fs::read_dir(dir)
                .into_iter()
                .flatten()
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.is_file())
                .collect::<Vec<_>>()
        })
        .map(|path| spool_info(path.as_path()))
        .collect::<Vec<_>>();
    files.sort_by(|left, right| left.file_name.cmp(&right.file_name));
    files
}

fn spool_info(path: &Path) -> SpoolInfo {
    let content = std::fs::read_to_string(path).unwrap_or_default();
    let metadata = path.metadata().ok();
    SpoolInfo {
        file_name: path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string()),
        path: path.to_path_buf(),
        size: metadata.map(|metadata| metadata.len()).unwrap_or_default(),
        status: latest_spool_field(&content, "status").unwrap_or_else(|| "READY".to_string()),
        job: latest_spool_field(&content, "job").unwrap_or_else(|| "-".to_string()),
        user: latest_spool_field(&content, "user").unwrap_or_else(|| "-".to_string()),
    }
}

fn latest_spool_field(content: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}=");
    content
        .lines()
        .filter_map(|line| {
            line.split_whitespace()
                .find_map(|field| field.strip_prefix(&prefix))
        })
        .next_back()
        .map(|value| value.trim_start_matches('*').to_uppercase())
}

#[cfg(test)]
mod tests {
    use super::{latest_spool_field, list_spool_files};
    use std::io::Write;

    struct EnvGuard {
        key: &'static str,
        original: Option<std::ffi::OsString>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let original = std::env::var_os(key);
            unsafe {
                std::env::set_var(key, value);
            }
            Self { key, original }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.original {
                Some(value) => unsafe {
                    std::env::set_var(self.key, value);
                },
                None => unsafe {
                    std::env::remove_var(self.key);
                },
            }
        }
    }

    #[test]
    fn latest_spool_field_reads_last_status() {
        let content = "job=JOB1 user=QPGMR status=RUN\nstatus=*HELD changed_at=1\n";
        assert_eq!(
            latest_spool_field(content, "status").as_deref(),
            Some("HELD")
        );
        assert_eq!(latest_spool_field(content, "job").as_deref(), Some("JOB1"));
    }

    #[test]
    fn list_spool_files_uses_configured_spool_dir() {
        let root = tempfile::tempdir().expect("tempdir");
        let _spool = EnvGuard::set("L400_SPOOL_DIR", root.path().to_str().expect("utf8"));
        let mut file = std::fs::File::create(root.path().join("demo.splf")).expect("create splf");
        writeln!(file, "spool_version=1 job=JOB1 user=QPGMR status=READY").expect("write splf");

        let files = list_spool_files();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_name, "demo.splf");
        assert_eq!(files[0].job, "JOB1");
        assert_eq!(files[0].user, "QPGMR");
        assert_eq!(files[0].status, "READY");
    }

    #[test]
    fn spool_status_filter_cycles_like_work_screen_subset() {
        let mut view =
            super::AdminCommandView::spool_outq(None, crate::session::SessionContext::new(1));

        assert_eq!(view.spool_status_filter, None);
        view.cycle_spool_status_filter();
        assert_eq!(view.spool_status_filter.as_deref(), Some("READY"));
        view.cycle_spool_status_filter();
        assert_eq!(view.spool_status_filter.as_deref(), Some("HELD"));
        view.cycle_spool_status_filter();
        assert_eq!(view.spool_status_filter.as_deref(), Some("SAVED"));
        view.cycle_spool_status_filter();
        assert_eq!(view.spool_status_filter, None);
    }
}
