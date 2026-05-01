use crossterm::event::{KeyCode, KeyEvent};
use l400::{
    copy_object, create_object_with_metadata, describe_object, list_objects, read_string_attr,
    resolve_l400_root, write_string_attr,
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
};
use std::path::PathBuf;

use crate::screens::{Screen, ScreenResult};
use crate::style::*;
use crate::widgets::help_bar::{CpfMessage, HelpAction, HelpBar};
use crate::widgets::subfile_table::SubfileTable;

#[derive(Clone, Debug)]
struct ProfileInfo {
    name: String,
    uid: String,
    status: String,
    text: String,
    last_signon: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditMode {
    Create,
    Copy,
    Rename,
}

pub struct WrkUsrPrf {
    root_path: PathBuf,
    profiles: Vec<ProfileInfo>,
    table: SubfileTable,
    edit_mode: Option<EditMode>,
    edit_buffer: String,
    detail: Option<String>,
    status: String,
}

impl WrkUsrPrf {
    pub fn new() -> Self {
        Self::with_root(resolve_l400_root())
    }

    fn with_root(root_path: PathBuf) -> Self {
        let mut screen = Self {
            root_path,
            profiles: Vec::new(),
            table: SubfileTable::new(
                vec!["Opt", "Profile", "UID", "Status", "Last signon", "Text"],
                vec![4, 16, 8, 12, 14, 28],
            )
            .with_title("User profiles"),
            edit_mode: None,
            edit_buffer: String::new(),
            detail: None,
            status: String::new(),
        };
        screen.refresh();
        screen
    }

    fn qsys_path(&self) -> PathBuf {
        self.root_path.join("QSYS")
    }

    fn refresh(&mut self) {
        self.profiles = list_objects(&self.qsys_path())
            .unwrap_or_default()
            .into_iter()
            .filter(|object| object.objtype == "*USRPRF")
            .map(|object| {
                let disabled = read_string_attr(&object.path, "user.l400.disabled")
                    .ok()
                    .flatten()
                    .is_some();
                let uid = read_string_attr(&object.path, l400::object::L400_OWNER_UID_ATTR)
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| "-".to_string());
                let last_signon = read_string_attr(&object.path, "user.l400.last_signon")
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| "-".to_string());
                ProfileInfo {
                    name: object.name,
                    uid,
                    status: if disabled { "*DISABLED" } else { "*ENABLED" }.to_string(),
                    text: object.text.unwrap_or_default(),
                    last_signon,
                }
            })
            .collect();
        self.profiles
            .sort_by(|left, right| left.name.cmp(&right.name));
        self.sync_table();
    }

    fn sync_table(&mut self) {
        let rows = self
            .profiles
            .iter()
            .map(|profile| {
                vec![
                    " ".to_string(),
                    profile.name.clone(),
                    profile.uid.clone(),
                    profile.status.clone(),
                    profile.last_signon.clone(),
                    profile.text.clone(),
                ]
            })
            .collect::<Vec<_>>();
        self.table.set_rows(rows);
    }

    fn selected_profile(&self) -> Option<&ProfileInfo> {
        self.table
            .selected()
            .and_then(|index| self.profiles.get(index))
            .or_else(|| self.profiles.first())
    }

    fn selected_path(&self) -> Option<std::path::PathBuf> {
        self.selected_profile()
            .map(|profile| self.qsys_path().join(&profile.name))
    }

    fn begin_edit(&mut self, mode: EditMode, initial: impl Into<String>) {
        self.edit_mode = Some(mode);
        self.edit_buffer = initial.into();
    }

    fn finish_edit(&mut self) {
        let Some(mode) = self.edit_mode.take() else {
            return;
        };
        let value = self.edit_buffer.trim().to_uppercase();
        self.edit_buffer.clear();
        if value.is_empty() {
            self.status = "Profile name is required.".to_string();
            return;
        }

        match mode {
            EditMode::Create => {
                match create_object_with_metadata(
                    &self.qsys_path(),
                    &value,
                    "*USRPRF",
                    Some("USRPRF"),
                    Some("Linux/400 user profile"),
                ) {
                    Ok(_) => self.status = format!("Profile {value} created."),
                    Err(error) => self.status = format!("Error creating profile {value}: {error}"),
                }
            }
            EditMode::Copy => {
                let Some(source) = self.selected_path() else {
                    self.status = "No profile selected.".to_string();
                    return;
                };
                match copy_object(&source, &self.qsys_path().join(&value)) {
                    Ok(_) => self.status = format!("Profile copied to {value}."),
                    Err(error) => self.status = format!("Error copying profile: {error}"),
                }
            }
            EditMode::Rename => {
                let Some(source) = self.selected_path() else {
                    self.status = "No profile selected.".to_string();
                    return;
                };
                match std::fs::rename(&source, self.qsys_path().join(&value)) {
                    Ok(_) => self.status = format!("Profile renamed to {value}."),
                    Err(error) => self.status = format!("Error renaming profile: {error}"),
                }
            }
        }
        self.refresh();
    }

    fn disable_selected(&mut self) {
        let Some(path) = self.selected_path() else {
            self.status = "No profile selected.".to_string();
            return;
        };
        match write_string_attr(&path, "user.l400.disabled", "yes") {
            Ok(_) => self.status = "Profile disabled.".to_string(),
            Err(error) => self.status = format!("Error disabling profile: {error}"),
        }
        self.refresh();
    }

    fn show_detail(&mut self) {
        let Some(path) = self.selected_path() else {
            self.status = "No profile selected.".to_string();
            return;
        };
        self.detail = match describe_object(&path) {
            Ok(object) => Some(format!(
                "Profile: {}\nUID: {}\nStatus: {}\nAuthorities: owner={} public={}\nLast signon: {}",
                object.name,
                self.selected_profile()
                    .map(|profile| profile.uid.as_str())
                    .unwrap_or("-"),
                self.selected_profile()
                    .map(|profile| profile.status.as_str())
                    .unwrap_or("-"),
                object.owner.unwrap_or_else(|| "-".to_string()),
                object.public_auth.unwrap_or_else(|| "*NONE".to_string()),
                self.selected_profile()
                    .map(|profile| profile.last_signon.as_str())
                    .unwrap_or("-")
            )),
            Err(error) => Some(format!("Error displaying profile: {error}")),
        };
    }
}

impl Screen for WrkUsrPrf {
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
        self.table.render(frame, chunks[1]);
        self.render_status(frame, chunks[2]);
        self.render_help(frame, chunks[3]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> ScreenResult {
        if self.edit_mode.is_some() {
            return match key.code {
                KeyCode::Enter => {
                    self.finish_edit();
                    ScreenResult::none()
                }
                KeyCode::F(12) | KeyCode::Esc => {
                    self.edit_mode = None;
                    self.edit_buffer.clear();
                    ScreenResult::none()
                }
                KeyCode::Backspace => {
                    self.edit_buffer.pop();
                    ScreenResult::none()
                }
                KeyCode::Char(c) => {
                    self.edit_buffer.push(c.to_ascii_uppercase());
                    ScreenResult::none()
                }
                _ => ScreenResult::none(),
            };
        }

        match key.code {
            KeyCode::F(3) | KeyCode::F(12) | KeyCode::Esc => ScreenResult::back(),
            KeyCode::F(5) => {
                self.refresh();
                ScreenResult::none()
            }
            KeyCode::Up => {
                self.table.select_prev();
                ScreenResult::none()
            }
            KeyCode::Down => {
                self.table.select_next();
                ScreenResult::none()
            }
            KeyCode::Char('2') => {
                self.begin_edit(EditMode::Create, "");
                ScreenResult::none()
            }
            KeyCode::Char('3') => {
                self.begin_edit(EditMode::Copy, "");
                ScreenResult::none()
            }
            KeyCode::Char('4') => {
                self.disable_selected();
                ScreenResult::none()
            }
            KeyCode::Char('5') => {
                self.show_detail();
                ScreenResult::none()
            }
            KeyCode::Char('7') => {
                let initial = self
                    .selected_profile()
                    .map(|profile| profile.name.clone())
                    .unwrap_or_default();
                self.begin_edit(EditMode::Rename, initial);
                ScreenResult::none()
            }
            _ => ScreenResult::none(),
        }
    }
}

impl WrkUsrPrf {
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let edit = match self.edit_mode {
            Some(EditMode::Create) => format!("Create profile: {}", self.edit_buffer),
            Some(EditMode::Copy) => format!("Copy to profile: {}", self.edit_buffer),
            Some(EditMode::Rename) => format!("Rename to profile: {}", self.edit_buffer),
            None => "Options: 2=Create 3=Copy 4=Disable 5=Display 7=Rename".to_string(),
        };
        frame.render_widget(
            Paragraph::new(edit).style(STYLE_NORMAL).block(
                Block::default()
                    .title(" WRKUSRPRF ")
                    .style(STYLE_HEADER)
                    .borders(Borders::ALL)
                    .border_style(STYLE_BORDER),
            ),
            area,
        );
    }

    fn render_status(&self, frame: &mut Frame, area: Rect) {
        let message = self.detail.clone().unwrap_or_else(|| self.status.clone());
        let cpf = if message.to_ascii_lowercase().contains("error") {
            CpfMessage::error("CPF9898", message)
        } else {
            CpfMessage::info("CPF0000", message)
        };
        cpf.render(frame, area);
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        HelpBar::new()
            .command("WRKUSRPRF")
            .actions(vec![
                HelpAction::new("F3", "Back"),
                HelpAction::new("F5", "Refresh"),
                HelpAction::new("2/3/4/5/7", "Options"),
                HelpAction::new("F12", "Cancel"),
            ])
            .render(frame, area);
    }
}

impl Default for WrkUsrPrf {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_disable_profile_from_tui() {
        let root = tempfile::tempdir().expect("tempdir");
        l400::bootstrap_l400_root(root.path()).expect("bootstrap");

        let mut screen = WrkUsrPrf::with_root(root.path().to_path_buf());
        screen.begin_edit(EditMode::Create, "TSTUSR");
        screen.finish_edit();
        assert!(screen.qsys_path().join("TSTUSR").exists());

        screen.refresh();
        let idx = screen
            .profiles
            .iter()
            .position(|profile| profile.name == "TSTUSR")
            .expect("profile");
        screen.table.set_rows(
            screen
                .profiles
                .iter()
                .map(|profile| vec![" ".to_string(), profile.name.clone()])
                .collect(),
        );
        for _ in 0..idx {
            screen.table.select_next();
        }
        screen.disable_selected();
        assert!(
            l400::read_string_attr(&screen.qsys_path().join("TSTUSR"), "user.l400.disabled")
                .expect("xattr")
                .is_some()
        );
    }
}
