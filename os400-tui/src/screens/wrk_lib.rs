use crossterm::event::{KeyCode, KeyEvent};
use l400::{create_library, delete_object, list_libraries, resolve_l400_root};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph},
};

use crate::screens::{Screen, ScreenId, ScreenResult};
use crate::session::SessionContext;
use crate::style::*;
use crate::widgets::confirm_dialog::ConfirmDialog;
use crate::widgets::help_bar::{CpfMessage, HelpAction, HelpBar};
use crate::widgets::subfile_table::SubfileTable;

enum EditMode {
    Filter,
    Create,
    Rename,
}

pub struct WrkLib {
    libraries: Vec<String>,
    filtered_indices: Vec<usize>,
    table: SubfileTable,
    filter: String,
    edit_mode: Option<EditMode>,
    edit_buffer: String,
    option_buffer: String,
    session: SessionContext,
    status_message: Option<String>,
    pending_delete: Option<ConfirmDialog>,
    pending_rename: Option<(String, String, ConfirmDialog)>,
}

impl WrkLib {
    pub fn new(session: SessionContext) -> Self {
        let mut screen = Self {
            libraries: Vec::new(),
            filtered_indices: Vec::new(),
            table: SubfileTable::new(vec!["Opt", "Library"], vec![4, 16]).with_title("Libraries"),
            filter: String::new(),
            edit_mode: None,
            edit_buffer: String::new(),
            option_buffer: String::new(),
            session,
            status_message: None,
            pending_delete: None,
            pending_rename: None,
        };
        screen.refresh();
        screen
    }

    fn refresh(&mut self) {
        match list_libraries(&resolve_l400_root()) {
            Ok(libraries) => {
                self.libraries = libraries;
                self.status_message = None;
            }
            Err(error) => {
                self.libraries.clear();
                self.status_message = Some(format!("Error listing libraries: {error}"));
            }
        }
        self.apply_filter();
    }

    fn apply_filter(&mut self) {
        let filter = self.filter.trim().to_uppercase();
        self.filtered_indices = self
            .libraries
            .iter()
            .enumerate()
            .filter(|(_, library)| filter.is_empty() || library.contains(&filter))
            .map(|(index, _)| index)
            .collect();
        let rows = self
            .filtered_indices
            .iter()
            .filter_map(|index| self.libraries.get(*index))
            .map(|library| vec![" ".to_string(), library.clone()])
            .collect::<Vec<_>>();
        self.table.set_rows(rows);
    }

    fn selected_library(&self) -> Option<&str> {
        let filtered_index = self.table.selected()?;
        let library_index = *self.filtered_indices.get(filtered_index)?;
        self.libraries.get(library_index).map(String::as_str)
    }

    fn begin_edit(&mut self, mode: EditMode, initial: impl Into<String>) {
        self.edit_mode = Some(mode);
        self.edit_buffer = initial.into();
    }

    fn finish_edit(&mut self) -> ScreenResult {
        let Some(mode) = self.edit_mode.take() else {
            return ScreenResult::none();
        };
        let value = self.edit_buffer.trim().to_uppercase();
        self.edit_buffer.clear();

        match mode {
            EditMode::Filter => {
                self.filter = value;
                self.apply_filter();
                self.status_message =
                    Some(format!("Library filter: {}", display_filter(&self.filter)));
            }
            EditMode::Create => {
                if value.is_empty() {
                    self.status_message = Some("Library name is required.".to_string());
                } else {
                    match create_library(&resolve_l400_root(), &value) {
                        Ok(_) => {
                            self.status_message = Some(format!("Library {value} created."));
                            self.refresh();
                        }
                        Err(error) => {
                            self.status_message = Some(format!("Error creating {value}: {error}"));
                        }
                    }
                }
            }
            EditMode::Rename => {
                let Some(current) = self.selected_library().map(str::to_string) else {
                    self.status_message = Some("No library selected.".to_string());
                    return ScreenResult::none();
                };
                if value.is_empty() {
                    self.status_message = Some("New library name is required.".to_string());
                } else {
                    self.pending_rename = Some((
                        current.clone(),
                        value.clone(),
                        ConfirmDialog::new("RNMLIB", format!("Rename {current} to {value}?")),
                    ));
                }
            }
        }
        ScreenResult::none()
    }

    fn request_delete_selected(&mut self) {
        let Some(library) = self.selected_library().map(str::to_string) else {
            self.status_message = Some("No library selected.".to_string());
            return;
        };
        self.pending_delete = Some(ConfirmDialog::new("DLTLIB", format!("Delete {library}?")));
        self.status_message = Some(format!("DLTLIB {library} pending."));
    }

    fn confirm_delete(&mut self) {
        let Some(library) = self.selected_library().map(str::to_string) else {
            self.status_message = Some("No library selected.".to_string());
            return;
        };
        match delete_object(&resolve_l400_root().join(&library)) {
            Ok(_) => {
                self.status_message = Some(format!("Library {library} deleted."));
                self.refresh();
            }
            Err(error) => {
                self.status_message = Some(format!("Error deleting {library}: {error}"));
            }
        }
    }

    fn confirm_rename(&mut self, current: String, new_name: String) {
        let root = resolve_l400_root();
        match std::fs::rename(root.join(&current), root.join(&new_name)) {
            Ok(_) => {
                self.status_message = Some(format!("Library {current} renamed to {new_name}."));
                self.refresh();
            }
            Err(error) => {
                self.status_message = Some(format!("Error renaming {current}: {error}"));
            }
        }
    }
}

impl Screen for WrkLib {
    fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5),
                Constraint::Min(0),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(crate::screens::screen_area(frame));

        self.render_header(frame, chunks[0]);
        self.table.render(frame, chunks[1]);
        self.render_status(frame, chunks[2]);
        self.render_help(frame, chunks[3]);

        if let Some(dialog) = &self.pending_delete {
            dialog.render(frame, crate::screens::screen_area(frame));
        }
        if let Some((_, _, dialog)) = &self.pending_rename {
            dialog.render(frame, crate::screens::screen_area(frame));
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> ScreenResult {
        if let Some(mut dialog) = self.pending_delete.take() {
            dialog.handle_key(key);
            match dialog.result() {
                Some(true) => self.confirm_delete(),
                Some(false) => self.status_message = Some("DLTLIB cancelled.".to_string()),
                None => self.pending_delete = Some(dialog),
            }
            return ScreenResult::none();
        }

        if let Some((current, new_name, mut dialog)) = self.pending_rename.take() {
            dialog.handle_key(key);
            match dialog.result() {
                Some(true) => self.confirm_rename(current, new_name),
                Some(false) => self.status_message = Some("RNMLIB cancelled.".to_string()),
                None => self.pending_rename = Some((current, new_name, dialog)),
            }
            return ScreenResult::none();
        }

        if self.edit_mode.is_some() {
            return match key.code {
                KeyCode::Enter => self.finish_edit(),
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
            KeyCode::F(3) | KeyCode::F(12) => ScreenResult::back(),
            KeyCode::F(4) => ScreenResult::goto(ScreenId::CommandLine),
            KeyCode::F(5) => {
                self.refresh();
                ScreenResult::none()
            }
            KeyCode::F(6) => {
                self.begin_edit(EditMode::Create, "");
                ScreenResult::none()
            }
            KeyCode::Char('1') => {
                self.option_buffer = "1".to_string();
                self.status_message = Some("Option 12 pending.".to_string());
                ScreenResult::none()
            }
            KeyCode::Char('2') if self.option_buffer == "1" => {
                self.option_buffer.clear();
                self.begin_edit(EditMode::Create, "");
                ScreenResult::none()
            }
            KeyCode::F(17) => {
                self.begin_edit(EditMode::Filter, self.filter.clone());
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
            KeyCode::PageUp => {
                self.table.page_up();
                ScreenResult::none()
            }
            KeyCode::PageDown => {
                self.table.page_down();
                ScreenResult::none()
            }
            KeyCode::Char('3') => self
                .selected_library()
                .map(|library| ScreenResult::with_data(ScreenId::ObjectBrowser, library))
                .unwrap_or_else(ScreenResult::none),
            KeyCode::Char('4') => {
                self.option_buffer.clear();
                self.request_delete_selected();
                ScreenResult::none()
            }
            KeyCode::Char('5') => self
                .selected_library()
                .map(|library| {
                    ScreenResult::with_data(
                        ScreenId::ObjectDetail,
                        format!("DSPOBJD OBJ({library})"),
                    )
                })
                .unwrap_or_else(ScreenResult::none),
            KeyCode::Char('7') => {
                self.option_buffer.clear();
                let initial = self.selected_library().unwrap_or_default().to_string();
                self.begin_edit(EditMode::Rename, initial);
                ScreenResult::none()
            }
            KeyCode::Char('2') => {
                if let Some(library) = self.selected_library() {
                    self.session.set_current_library(library);
                    self.status_message = Some(format!("Current library changed to {library}."));
                }
                ScreenResult::none()
            }
            _ => ScreenResult::none(),
        }
    }
}

impl WrkLib {
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Work with Libraries ")
            .style(STYLE_HEADER)
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);
        frame.render_widget(block, area);
        let mode = match &self.edit_mode {
            Some(EditMode::Filter) => format!("Filter: {}", self.edit_buffer),
            Some(EditMode::Create) => format!("New library: {}", self.edit_buffer),
            Some(EditMode::Rename) => format!("New name: {}", self.edit_buffer),
            None => format!(
                "Options: 2=Change current 3=Contents 4=Delete 5=Display 7=Rename 12=Create   Filter={}",
                display_filter(&self.filter)
            ),
        };
        let text = Text::from(vec![Line::from(mode)]);
        let inner = Rect::new(area.x + 1, area.y + 1, area.width.saturating_sub(2), 2);
        frame.render_widget(Paragraph::new(text).style(STYLE_NORMAL), inner);
    }

    fn render_status(&self, frame: &mut Frame, area: Rect) {
        let message = self.status_message.clone().unwrap_or_default();
        let cpf = if message.to_ascii_lowercase().contains("error") {
            CpfMessage::error("CPF9898", message)
        } else {
            CpfMessage::info("CPF0000", message)
        };
        cpf.render(frame, area);
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        HelpBar::new()
            .command("WRKLIB")
            .actions(vec![
                HelpAction::new("F3", "Back"),
                HelpAction::new("F4", "Prompt"),
                HelpAction::new("F5", "Refresh"),
                HelpAction::new("F6", "Create"),
                HelpAction::new("F17", "Filter"),
                HelpAction::new("F12", "Cancel"),
            ])
            .render(frame, area);
    }
}

fn display_filter(value: &str) -> &str {
    if value.trim().is_empty() {
        "*ALL"
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_limits_visible_libraries() {
        let mut screen = WrkLib::new(SessionContext::new(940));
        screen.libraries = vec![
            "QGPL".to_string(),
            "QUSRSYS".to_string(),
            "TESTLIB".to_string(),
        ];
        screen.filter = "TEST".to_string();
        screen.apply_filter();

        assert_eq!(screen.filtered_indices, vec![2]);
    }

    #[test]
    fn option_twelve_starts_create_prompt() {
        let mut screen = WrkLib::new(SessionContext::new(941));
        assert_eq!(
            screen.handle_key(KeyEvent::from(KeyCode::Char('1'))).next,
            None
        );
        assert_eq!(
            screen.handle_key(KeyEvent::from(KeyCode::Char('2'))).next,
            None
        );
        assert!(matches!(screen.edit_mode, Some(EditMode::Create)));
    }
}
