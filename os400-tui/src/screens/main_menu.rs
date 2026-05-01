use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use l400::read_loader_status;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::screens::{Screen, ScreenId, ScreenResult};
use crate::session::SessionContext;
use crate::style::*;
use crate::widgets::command_input::CommandInput;
use crate::widgets::help_bar::{HelpAction, HelpBar};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MenuKind {
    Main,
    CmdObj,
    CmdSql,
    CmdSys,
}

pub struct MainMenu {
    selected_index: usize,
    pending_option: String,
    command_input: CommandInput,
    session: SessionContext,
    kind: MenuKind,
}

impl MainMenu {
    pub fn new() -> Self {
        Self::with_session(SessionContext::new(std::process::id() as u64))
    }

    pub fn with_session(session: SessionContext) -> Self {
        Self {
            selected_index: 0,
            pending_option: String::new(),
            command_input: CommandInput::new(),
            session,
            kind: MenuKind::Main,
        }
    }

    pub fn command_menu(target: &str, session: SessionContext) -> Self {
        let kind = match target.trim().to_uppercase().as_str() {
            "CMDOBJ" => MenuKind::CmdObj,
            "CMDSQL" => MenuKind::CmdSql,
            "CMDSYS" => MenuKind::CmdSys,
            _ => MenuKind::Main,
        };
        Self {
            kind,
            ..Self::with_session(session)
        }
    }

    fn menu_items(&self) -> Vec<(&'static str, &'static str, &'static str)> {
        match self.kind {
            MenuKind::Main => vec![
                ("1", "Work with libraries . . . . . . . . . . .", "WRKLIB"),
                ("2", "Work with objects . . . . . . . . . . . .", "WRKOBJ"),
                ("3", "Work with files . . . . . . . . . . . .", "WRKOBJ"),
                ("4", "Work with jobs . . . . . . . . . . . .", "WRKACTJOB"),
                ("5", "Data queues  . . . . . . . . . . . . .", "DSPDTAQ"),
                ("6", "Command entry . . . . . . . . . . . .", "CMD"),
                ("7", "Programming Development Manager . . . .", "STRPDM"),
                ("8", "System status  . . . . . . . . . . .", "WRKSYSSTS"),
                ("9", "System values  . . . . . . . . . . .", "WRKSYSVAL"),
                (" ", " ", " "),
                ("10", "User profiles . . . . . . . . . . .", "WRKUSRPRF"),
                ("11", "Spool files . . . . . . . . . . . .", "WRKSPLF"),
                ("12", "Policy and audit  . . . . . . . . .", "DSPPOLICY"),
                ("13", "Command groups  . . . . . . . . . . . .", "GO CMDOBJ"),
                ("14", "Submit batch job . . . . . . . . . .", "SBMJOB"),
                ("90", "Power down system . . . . . . . . .", "PWRDWNSYS"),
            ],
            MenuKind::CmdObj => vec![
                ("1", "Work with libraries . . . . . . . . . . .", "WRKLIB"),
                ("2", "Work with objects . . . . . . . . . . . .", "WRKOBJ"),
                ("3", "Display object description . . . . . . .", "DSPOBJD"),
                ("4", "Display object authority . . . . . . . .", "DSPOBJAUT"),
                ("5", "Data queue operations . . . . . . . . .", "DSPDTAQ"),
            ],
            MenuKind::CmdSql => vec![
                ("1", "Start interactive SQL . . . . . . . . .", "STRSQL"),
                (
                    "2",
                    "Show physical files . . . . . . . . . .",
                    "SHOW TABLES",
                ),
                (
                    "3",
                    "Describe physical file . . . . . . . .",
                    "DESCRIBE TABLE",
                ),
            ],
            MenuKind::CmdSys => vec![
                ("1", "Work with active jobs . . . . . . . . .", "WRKACTJOB"),
                ("2", "Work with user profiles . . . . . . . .", "WRKUSRPRF"),
                ("3", "Work with spool files . . . . . . . . .", "WRKSPLF"),
                ("4", "Policy and audit . . . . . . . . . . .", "DSPPOLICY"),
                ("90", "Power down system . . . . . . . . . .", "PWRDWNSYS"),
            ],
        }
    }

    fn handle_option(&self, option: &str) -> ScreenResult {
        let Some((_, _, command)) = self
            .menu_items()
            .into_iter()
            .find(|(item, _, _)| *item == option)
        else {
            return ScreenResult::none();
        };
        self.route_command(command)
    }

    fn option_index(&self, option: &str) -> Option<usize> {
        self.menu_items()
            .iter()
            .position(|(item_option, _, _)| *item_option == option)
    }

    fn move_selection(&mut self, step: isize) {
        let items = self.menu_items();
        if items.is_empty() {
            return;
        }

        let mut next = self.selected_index as isize;
        loop {
            next = (next + step).clamp(0, (items.len() - 1) as isize);
            let idx = next as usize;
            if !items[idx].0.trim().is_empty() || idx == self.selected_index {
                self.selected_index = idx;
                break;
            }

            if idx == 0 || idx == items.len() - 1 {
                self.selected_index = idx;
                break;
            }
        }
    }

    fn execute_selected(&mut self) -> ScreenResult {
        self.pending_option.clear();
        let items = self.menu_items();
        if self.selected_index < items.len() {
            self.handle_option(items[self.selected_index].0)
        } else {
            ScreenResult::none()
        }
    }

    fn apply_pending_option(&mut self, digit: char) -> ScreenResult {
        if !digit.is_ascii_digit() {
            return ScreenResult::none();
        }

        self.pending_option.push(digit);

        let has_prefix = self
            .menu_items()
            .iter()
            .any(|(option, _, _)| option.starts_with(&self.pending_option));
        if !has_prefix {
            self.pending_option.clear();
            return ScreenResult::none();
        }

        if let Some(idx) = self.option_index(&self.pending_option) {
            self.selected_index = idx;

            let has_longer_match = self.menu_items().iter().any(|(option, _, _)| {
                option != &self.pending_option && option.starts_with(&self.pending_option)
            });
            if !has_longer_match {
                return self.execute_selected();
            }
        }

        ScreenResult::none()
    }

    fn route_command(&self, command: &str) -> ScreenResult {
        let command = command.trim().to_uppercase();
        match command.as_str() {
            "GO MAIN" => ScreenResult::goto(ScreenId::MainMenu),
            "GO CMDOBJ" => ScreenResult::with_data(ScreenId::CommandMenu, "CMDOBJ"),
            "GO CMDSQL" => ScreenResult::with_data(ScreenId::CommandMenu, "CMDSQL"),
            "GO CMDSYS" => ScreenResult::with_data(ScreenId::CommandMenu, "CMDSYS"),
            "WRKLIB" => ScreenResult::goto(ScreenId::WrkLib),
            "WRKOBJ" | "WRKPGM" => ScreenResult::goto(ScreenId::ObjectBrowser),
            "WRKACTJOB" => ScreenResult::goto(ScreenId::WorkManagement),
            "DSPDTAQ" => ScreenResult::goto(ScreenId::DataQueueViewer),
            "CMD" => ScreenResult::goto(ScreenId::CommandLine),
            "SBMJOB" => ScreenResult::goto(ScreenId::SubmitJob),
            "STRPDM" => ScreenResult::goto(ScreenId::PdmBrowser),
            "STRSQL" | "SHOW TABLES" | "DESCRIBE TABLE" => ScreenResult::goto(ScreenId::StrSql),
            "PWRDWNSYS" => ScreenResult::goto(ScreenId::PowerDown),
            "WRKSYSSTS" | "WRKSYSVAL" | "DSPCMD" | "WRKCMD" => {
                ScreenResult::with_data(ScreenId::SystemPanel, command)
            }
            "WRKUSRPRF" => ScreenResult::goto(ScreenId::UserProfiles),
            "WRKSPLF" | "WRKOUTQ" => ScreenResult::goto(ScreenId::SpoolOutq),
            "DSPPOLICY" | "DSPAUD" => ScreenResult::goto(ScreenId::PolicyAudit),
            "DSPOBJD" => ScreenResult::with_data(ScreenId::ObjectDetail, command),
            "DSPOBJAUT" => ScreenResult::with_data(ScreenId::ObjectAuthority, command),
            _ => ScreenResult::with_data(ScreenId::CommandLine, command),
        }
    }
}

impl Screen for MainMenu {
    fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),
                Constraint::Min(0),
                Constraint::Length(1),
                Constraint::Length(3),
            ])
            .split(frame.area());

        self.render_header(frame, chunks[0]);
        self.render_menu(frame, chunks[1]);
        self.command_input.active = true;
        self.command_input.render(frame, chunks[2]);
        self.render_help(frame, chunks[3]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> ScreenResult {
        match key.code {
            KeyCode::F(3) => ScreenResult::goto(ScreenId::SignOn),
            KeyCode::F(4) => {
                self.pending_option.clear();
                ScreenResult::goto(ScreenId::CommandLine)
            }
            KeyCode::F(12) | KeyCode::Esc => {
                self.pending_option.clear();
                if self.kind == MenuKind::Main {
                    ScreenResult::none()
                } else {
                    ScreenResult::back()
                }
            }
            KeyCode::Up => {
                self.pending_option.clear();
                self.move_selection(-1);
                ScreenResult::none()
            }
            KeyCode::Down => {
                self.pending_option.clear();
                self.move_selection(1);
                ScreenResult::none()
            }
            KeyCode::Enter if !self.command_input.value.trim().is_empty() => {
                if let Some(command) = self.command_input.handle_key(key) {
                    self.pending_option.clear();
                    self.route_command(&command)
                } else {
                    ScreenResult::none()
                }
            }
            KeyCode::Enter => self.execute_selected(),
            KeyCode::Backspace => {
                self.pending_option.pop();
                if let Some(idx) = self.option_index(&self.pending_option) {
                    self.selected_index = idx;
                }
                ScreenResult::none()
            }
            KeyCode::Char(c)
                if c.is_ascii_digit() && !key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.apply_pending_option(c)
            }
            _ => {
                if let Some(command) = self.command_input.handle_key(key) {
                    self.pending_option.clear();
                    self.route_command(&command)
                } else {
                    ScreenResult::none()
                }
            }
        }
    }
}

impl MainMenu {
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let title = Line::from(vec![format!(" {} ", self.title()).into()]);
        let session = self.session.snapshot();

        let block = Block::default()
            .title(title)
            .style(STYLE_HEADER)
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);

        frame.render_widget(block, area);

        let status = loader_status_line();
        let text = Text::from(vec![
            Line::from(vec![
                "System: ".into(),
                "L400   ".into(),
                "User: ".into(),
                session.user_profile.into(),
                "   ".into(),
                "Library: ".into(),
                session.current_library.into(),
                "   ".into(),
                "Job: ".into(),
                session.job_id.to_string().into(),
                "   ".into(),
                "Selection: ".into(),
                self.pending_option.clone().into(),
            ]),
            Line::from(vec![
                status.into(),
                "   ".into(),
                session.last_message.unwrap_or_default().into(),
            ]),
        ]);

        let inner = Rect::new(area.x + 1, area.y + 1, area.width - 2, 2);
        frame.render_widget(Paragraph::new(text).style(STYLE_NORMAL), inner);
    }

    fn render_menu(&self, frame: &mut Frame, area: Rect) {
        let items = self.menu_items();
        let menu_items: Vec<ListItem> = items
            .iter()
            .enumerate()
            .map(|(i, (_, text, cmd))| {
                let style = if i == self.selected_index {
                    STYLE_OPTION_SELECTED
                } else {
                    STYLE_OPTION
                };
                ListItem::new(Line::from(vec![(*text).into(), " ".into(), (*cmd).into()]))
                    .style(style)
            })
            .collect();

        let list = List::new(menu_items)
            .block(
                Block::default()
                    .title(self.menu_panel_title())
                    .borders(Borders::ALL)
                    .border_style(STYLE_BORDER),
            )
            .style(STYLE_NORMAL);

        frame.render_widget(list, area);
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        HelpBar::new()
            .command("GO")
            .actions(vec![
                HelpAction::new("F3", "Signoff"),
                HelpAction::new("F4", "Prompt"),
                HelpAction::new("F12", "Back"),
                HelpAction::new("Enter", "Select"),
                HelpAction::new("Tab", "Complete"),
            ])
            .render(frame, area);
    }

    fn title(&self) -> &'static str {
        match self.kind {
            MenuKind::Main => "L400 Main Menu",
            MenuKind::CmdObj => "Object Commands",
            MenuKind::CmdSql => "SQL Commands",
            MenuKind::CmdSys => "System Commands",
        }
    }

    fn menu_panel_title(&self) -> &'static str {
        match self.kind {
            MenuKind::Main => "Main",
            MenuKind::CmdObj => "GO CMDOBJ",
            MenuKind::CmdSql => "GO CMDSQL",
            MenuKind::CmdSys => "GO CMDSYS",
        }
    }
}

impl Default for MainMenu {
    fn default() -> Self {
        Self::new()
    }
}

fn loader_status_line() -> String {
    match read_loader_status() {
        Ok(status) => {
            let protection = if status.protection_active {
                "ACTIVE"
            } else {
                "INACTIVE"
            };
            let mut line = format!(
                "Protection: {}   Loader mode: {}   Phase: {}",
                protection,
                status.mode.to_uppercase(),
                status.phase
            );
            if let Some(error) = status.last_error {
                line.push_str("   Last error: ");
                line.push_str(&truncate_status_field(&error, 48));
            }
            if let Some(policy_version) = status.policy_version {
                line.push_str("   Policy: ");
                line.push_str(&policy_version);
            }
            line
        }
        Err(_) => "Protection: UNKNOWN   Loader mode: unavailable".to_string(),
    }
}

fn truncate_status_field(value: &str, max_len: usize) -> String {
    if value.chars().count() <= max_len {
        return value.to_string();
    }
    value.chars().take(max_len).collect::<String>() + "..."
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn truncates_status_field_when_needed() {
        let truncated = truncate_status_field("abcdefghijklmnopqrstuvwxyz", 10);
        assert_eq!(truncated, "abcdefghij...");
    }

    #[test]
    fn f3_signs_off_to_login() {
        let mut menu = MainMenu::new();
        let result = menu.handle_key(key(KeyCode::F(3)));
        assert_eq!(result.next, Some(ScreenId::SignOn));
    }

    #[test]
    fn digit_three_opens_object_browser() {
        let mut menu = MainMenu::new();
        let result = menu.handle_key(key(KeyCode::Char('3')));
        assert_eq!(result.next, Some(ScreenId::ObjectBrowser));
    }

    #[test]
    fn digit_one_waits_for_enter_because_of_option_ten() {
        let mut menu = MainMenu::new();
        let result = menu.handle_key(key(KeyCode::Char('1')));
        assert_eq!(result.next, None);
        assert_eq!(menu.pending_option, "1");

        let result = menu.handle_key(key(KeyCode::Enter));
        assert_eq!(result.next, Some(ScreenId::WrkLib));
    }

    #[test]
    fn option_ten_can_be_selected_by_keyboard() {
        let mut menu = MainMenu::new();
        assert_eq!(menu.handle_key(key(KeyCode::Char('1'))).next, None);
        let result = menu.handle_key(key(KeyCode::Char('0')));
        assert_eq!(result.next, Some(ScreenId::UserProfiles));
        assert_eq!(result.data.as_deref(), None);
    }

    #[test]
    fn f4_opens_command_line() {
        let mut menu = MainMenu::new();
        let result = menu.handle_key(key(KeyCode::F(4)));
        assert_eq!(result.next, Some(ScreenId::CommandLine));
    }

    #[test]
    fn digit_seven_opens_pdm_browser() {
        let mut menu = MainMenu::new();
        let result = menu.handle_key(key(KeyCode::Char('7')));
        assert_eq!(result.next, Some(ScreenId::PdmBrowser));
    }

    #[test]
    fn option_twelve_opens_policy_audit_without_demo_stub() {
        let mut menu = MainMenu::new();
        assert_eq!(menu.handle_key(key(KeyCode::Char('1'))).next, None);
        let result = menu.handle_key(key(KeyCode::Char('2')));
        assert_eq!(result.next, Some(ScreenId::PolicyAudit));
    }

    #[test]
    fn embedded_command_input_routes_go_submenus() {
        let mut menu = MainMenu::new();
        for ch in "GO CMDOBJ".chars() {
            assert_eq!(menu.handle_key(key(KeyCode::Char(ch))).next, None);
        }
        let result = menu.handle_key(key(KeyCode::Enter));
        assert_eq!(result.next, Some(ScreenId::CommandMenu));
        assert_eq!(result.data.as_deref(), Some("CMDOBJ"));
    }

    #[test]
    fn command_submenu_f12_pops_navigation_stack() {
        let mut menu = MainMenu::command_menu("CMDSYS", SessionContext::new(911));
        let result = menu.handle_key(key(KeyCode::F(12)));
        assert_eq!(result.next, Some(ScreenId::Back));
    }

    #[test]
    fn option_ninety_opens_power_down_confirmation() {
        let mut menu = MainMenu::new();
        assert_eq!(menu.handle_key(key(KeyCode::Char('9'))).next, None);
        let result = menu.handle_key(key(KeyCode::Char('0')));
        assert_eq!(result.next, Some(ScreenId::PowerDown));
    }
}
