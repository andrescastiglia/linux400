use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use l400::{SourceMemberInfo, create_source_member, list_members, resolve_l400_root};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::Line,
    widgets::{Block, Borders, Paragraph, Row, Table, TableState},
};
use std::process::Command;

use crate::screens::{Screen, ScreenId, ScreenResult};
use crate::style::*;
use crate::widgets::help_bar::{CpfMessage, HelpAction, HelpBar};

pub struct MemberInfo {
    pub file_name: String,
    pub name: String,
    pub type_: String,
    pub text: String,
}

pub struct WrkMbrPdm {
    library: String,
    file: String,
    members: Vec<MemberInfo>,
    state: TableState,
    create_mode: bool,
    create_input: String,
    status_message: Option<String>,
}

impl WrkMbrPdm {
    pub fn new(library: String, file: String) -> Self {
        let mut screen = Self {
            library,
            file,
            members: Vec::new(),
            state: TableState::default(),
            create_mode: false,
            create_input: String::new(),
            status_message: None,
        };
        screen.refresh();
        screen
    }

    fn lib_path(&self) -> std::path::PathBuf {
        resolve_l400_root().join(&self.library)
    }

    fn load_members(&self) -> Result<Vec<MemberInfo>, String> {
        list_members(&self.lib_path(), &self.file)
            .map(|members| members.into_iter().map(Self::map_member).collect())
            .map_err(|error| error.to_string())
    }

    fn map_member(member: SourceMemberInfo) -> MemberInfo {
        MemberInfo {
            file_name: member.file_name,
            name: member.name,
            type_: member.type_,
            text: member.text,
        }
    }

    fn refresh(&mut self) {
        match self.load_members() {
            Ok(members) => {
                self.members = members;
                self.status_message = None;
            }
            Err(error) => {
                self.members.clear();
                self.status_message = Some(error);
            }
        }

        if self.members.is_empty() {
            self.state.select(None);
        } else {
            let selection = self
                .state
                .selected()
                .unwrap_or(0)
                .min(self.members.len() - 1);
            self.state.select(Some(selection));
        }
    }

    fn select_member_by_file_name(&mut self, file_name: &str) {
        if let Some(index) = self
            .members
            .iter()
            .position(|member| member.file_name.eq_ignore_ascii_case(file_name))
        {
            self.state.select(Some(index));
        }
    }

    fn selected_member_spec(&self) -> Option<String> {
        self.state
            .selected()
            .and_then(|index| self.members.get(index))
            .map(|member| format!("{}/{}/{}", self.library, self.file, member.file_name))
    }

    fn selected_program_spec(&self) -> Option<String> {
        self.state
            .selected()
            .and_then(|index| self.members.get(index))
            .map(|member| {
                let stem = member.file_name.split('.').next().unwrap_or(&member.name);
                format!("{}/{}", self.library, stem.to_uppercase())
            })
    }

    fn selected_compile_command(&self) -> Option<String> {
        let member = self
            .state
            .selected()
            .and_then(|index| self.members.get(index))?;
        let pgm = self.selected_program_spec()?;
        if member.type_.eq_ignore_ascii_case("C") {
            Some(format!(
                "CRTPGM PGM({pgm}) SRCFILE({}/{}) SRCMBR({})",
                self.library, self.file, member.file_name
            ))
        } else {
            Some(format!(
                "CRTCLPGM PGM({pgm}) SRCFILE({}/{}) SRCMBR({})",
                self.library, self.file, member.file_name
            ))
        }
    }

    fn run_toolchain_command(&mut self, command: String) {
        match Command::new("l400cmd")
            .args(crate::cl_parser::tokenize_cl_command(&command))
            .output()
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let summary = stdout
                    .lines()
                    .chain(stderr.lines())
                    .last()
                    .unwrap_or("No compiler output.")
                    .to_string();
                self.status_message = Some(format!(
                    "{} status={} {}",
                    command,
                    output.status.code().unwrap_or_default(),
                    summary
                ));
            }
            Err(error) => {
                self.status_message = Some(format!("Error running {}: {}", command, error));
            }
        }
        self.refresh();
    }

    fn normalized_new_member_name(&self) -> String {
        let trimmed = self.create_input.trim().to_uppercase();
        if trimmed.contains('.') {
            trimmed
        } else {
            format!("{trimmed}.CLP")
        }
    }

    fn handle_create_mode_key(&mut self, key: KeyEvent) -> ScreenResult {
        match key.code {
            KeyCode::Esc | KeyCode::F(12) => {
                self.create_mode = false;
                self.create_input.clear();
                self.status_message = Some("Create member cancelled.".to_string());
                ScreenResult::none()
            }
            KeyCode::Enter => {
                let member_name = self.normalized_new_member_name();
                if member_name == ".CLP" {
                    self.status_message = Some("Enter a member name.".to_string());
                    return ScreenResult::none();
                }

                match create_source_member(&self.lib_path(), &self.file, &member_name) {
                    Ok(_) => {
                        self.create_mode = false;
                        self.create_input.clear();
                        self.refresh();
                        self.select_member_by_file_name(&member_name);
                        self.status_message = Some(format!("Member {} created.", member_name));
                    }
                    Err(error) => {
                        self.status_message = Some(format!("Error creating member: {}", error));
                    }
                }
                ScreenResult::none()
            }
            KeyCode::Backspace => {
                self.create_input.pop();
                ScreenResult::none()
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.create_input.push(c);
                ScreenResult::none()
            }
            _ => ScreenResult::none(),
        }
    }
}

impl Default for WrkMbrPdm {
    fn default() -> Self {
        Self::new("QSYS".to_string(), "QCLSRC".to_string())
    }
}

impl Screen for WrkMbrPdm {
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
        self.render_table(frame, chunks[1]);
        self.render_status(frame, chunks[2]);
        self.render_help(frame, chunks[3]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> ScreenResult {
        if self.create_mode {
            return self.handle_create_mode_key(key);
        }

        match key.code {
            KeyCode::F(3) | KeyCode::F(12) => ScreenResult::goto(ScreenId::PdmBrowser),
            KeyCode::F(5) => {
                self.refresh();
                ScreenResult::none()
            }
            KeyCode::F(6) => {
                self.create_mode = true;
                self.create_input.clear();
                self.status_message = Some("New member: type a name and press Enter.".to_string());
                ScreenResult::none()
            }
            KeyCode::F(16) => {
                if let Some(pgm) = self.selected_program_spec() {
                    self.run_toolchain_command(format!("CALL PGM({pgm})"));
                } else {
                    self.status_message = Some("No program selected.".to_string());
                }
                ScreenResult::none()
            }
            KeyCode::F(14) => {
                if let Some(command) = self.selected_compile_command() {
                    self.run_toolchain_command(command);
                } else {
                    self.status_message = Some("No source member selected.".to_string());
                }
                ScreenResult::none()
            }
            KeyCode::F(17) => {
                if let Some(pgm) = self.selected_program_spec() {
                    self.run_toolchain_command(format!("CRTPGM PGM({pgm})"));
                } else {
                    self.status_message = Some("No source member selected.".to_string());
                }
                ScreenResult::none()
            }
            KeyCode::Up => {
                let next = self.state.selected().unwrap_or(0).saturating_sub(1);
                self.state.select(Some(next));
                ScreenResult::none()
            }
            KeyCode::Down => {
                let max = self.members.len().saturating_sub(1);
                let next = self
                    .state
                    .selected()
                    .unwrap_or(0)
                    .saturating_add(1)
                    .min(max);
                self.state.select(Some(next));
                ScreenResult::none()
            }
            KeyCode::Enter | KeyCode::F(15) => self
                .selected_member_spec()
                .map(|member| ScreenResult::with_data(ScreenId::StrSeu, member))
                .unwrap_or_else(ScreenResult::none),
            KeyCode::Char('2') | KeyCode::Char('5') => self
                .selected_member_spec()
                .map(|member| ScreenResult::with_data(ScreenId::StrSeu, member))
                .unwrap_or_else(ScreenResult::none),
            _ => ScreenResult::none(),
        }
    }
}

impl WrkMbrPdm {
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(format!(
                " WRKMBRPDM - Work with Members {} / {} ",
                self.library, self.file
            ))
            .style(STYLE_HEADER)
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);
        frame.render_widget(block, area);

        let text = Line::from(format!(
            "  File: {}/{}    Members: {}",
            self.library,
            self.file,
            self.members.len()
        ));
        let inner = Rect::new(area.x + 1, area.y + 1, area.width - 2, 1);
        frame.render_widget(Paragraph::new(text).style(STYLE_NORMAL), inner);
    }

    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        let widths = [
            Constraint::Length(4),
            Constraint::Length(20),
            Constraint::Length(12),
            Constraint::Min(10),
        ];
        let rows = self.members.iter().map(|member| {
            Row::new(vec![
                " ".to_string(),
                member.name.clone(),
                typed_member_label(&member.type_),
                member.text.clone(),
            ])
        });

        let table = Table::new(rows, widths)
            .header(
                Row::new(vec!["Opt", "Member", "Type", "Text"])
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
    }

    fn render_status(&self, frame: &mut Frame, area: Rect) {
        let message = if self.create_mode {
            format!("Create member> {}", self.create_input)
        } else {
            self.status_message.clone().unwrap_or_default()
        };
        let cpf = if message.to_ascii_lowercase().contains("error") {
            CpfMessage::error("CPF9898", message)
        } else {
            CpfMessage::info("CPF0000", message)
        };
        cpf.render(frame, area);
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        if self.create_mode {
            HelpBar::new()
                .command("WRKMBRPDM")
                .actions(vec![
                    HelpAction::new("Enter", "Create"),
                    HelpAction::new("F12", "Cancel"),
                    HelpAction::new("Esc", "Cancel"),
                ])
                .render(frame, area);
        } else {
            HelpBar::new()
                .command("WRKMBRPDM")
                .actions(vec![
                    HelpAction::new("F3", "Exit"),
                    HelpAction::new("F5", "Refresh"),
                    HelpAction::new("F6", "Create"),
                    HelpAction::new("2/5", "Edit"),
                    HelpAction::new("F14", "CRTCLPGM"),
                    HelpAction::new("F15", "Edit"),
                    HelpAction::new("F16", "CALL"),
                ])
                .render(frame, area);
        }
    }
}

fn typed_member_label(type_: &str) -> String {
    match type_.to_uppercase().as_str() {
        "CLP" => "CLP source".to_string(),
        "C" => "C source".to_string(),
        "TXT" => "Text".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f14_command_uses_member_type() {
        let mut screen = WrkMbrPdm::new("QGPL".to_string(), "QCSRC".to_string());
        screen.members = vec![MemberInfo {
            file_name: "HELLO.C".to_string(),
            name: "HELLO".to_string(),
            type_: "C".to_string(),
            text: String::new(),
        }];
        screen.state.select(Some(0));

        assert!(
            screen
                .selected_compile_command()
                .expect("compile command")
                .starts_with("CRTPGM")
        );
    }

    #[test]
    fn typed_member_label_is_user_friendly() {
        assert_eq!(typed_member_label("CLP"), "CLP source");
        assert_eq!(typed_member_label("C"), "C source");
    }
}
