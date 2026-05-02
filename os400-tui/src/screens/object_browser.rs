use crossterm::event::{KeyCode, KeyEvent};
use l400::{create_object_with_metadata, delete_object, list_objects, resolve_l400_root};
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

#[derive(Clone, Debug)]
pub struct ObjectInfo {
    pub library: String,
    pub name: String,
    pub type_: String,
    pub attribute: String,
    pub text: String,
    pub owner: String,
    pub public_auth: String,
    pub size: String,
    pub changed: String,
}

enum EditMode {
    NameFilter,
    TypeFilter,
    Library,
    Rename,
    CreateName,
    CreateType { name: String },
}

pub struct ObjectBrowser {
    current_library: String,
    objects: Vec<ObjectInfo>,
    filtered_indices: Vec<usize>,
    table: SubfileTable,
    name_filter: String,
    type_filter: String,
    edit_mode: Option<EditMode>,
    edit_buffer: String,
    option_buffer: String,
    using_runtime_data: bool,
    session: SessionContext,
    status_message: Option<String>,
    pending_delete: Option<ConfirmDialog>,
    pending_rename: Option<(String, String, ConfirmDialog)>,
}

impl ObjectBrowser {
    pub fn new() -> Self {
        Self::with_session(SessionContext::new(std::process::id() as u64))
    }

    pub fn with_session(session: SessionContext) -> Self {
        let current_library = session.snapshot().current_library;
        Self::for_library(current_library, session)
    }

    pub fn for_library(current_library: String, session: SessionContext) -> Self {
        let mut screen = Self {
            current_library,
            objects: Vec::new(),
            filtered_indices: Vec::new(),
            table: SubfileTable::new(
                vec![
                    "Opt", "Object", "Type", "Attr", "Owner", "*PUBLIC", "Size", "Changed", "Text",
                ],
                vec![4, 12, 10, 8, 10, 9, 8, 12, 24],
            )
            .with_title("Objects"),
            name_filter: String::new(),
            type_filter: String::new(),
            edit_mode: None,
            edit_buffer: String::new(),
            option_buffer: String::new(),
            using_runtime_data: false,
            session,
            status_message: None,
            pending_delete: None,
            pending_rename: None,
        };
        screen.refresh();
        screen
    }

    fn load_objects(library: &str) -> (Vec<ObjectInfo>, bool) {
        let library_path = resolve_l400_root().join(library);
        if let Ok(objects) = list_objects(&library_path) {
            let mapped = objects
                .into_iter()
                .map(|object| {
                    let metadata = std::fs::metadata(&object.path).ok();
                    ObjectInfo {
                        library: object.library.unwrap_or_else(|| library.to_string()),
                        name: object.name,
                        type_: object.objtype,
                        attribute: object.attribute.unwrap_or_else(|| "-".to_string()),
                        text: object.text.unwrap_or_default(),
                        owner: object.owner.unwrap_or_else(|| "UNKNOWN".to_string()),
                        public_auth: object.public_auth.unwrap_or_else(|| "*NONE".to_string()),
                        size: metadata
                            .as_ref()
                            .map(|metadata| metadata.len().to_string())
                            .unwrap_or_else(|| "-".to_string()),
                        changed: metadata
                            .and_then(|metadata| metadata.modified().ok())
                            .map(format_system_time)
                            .unwrap_or_else(|| "-".to_string()),
                    }
                })
                .collect::<Vec<_>>();
            return (mapped, true);
        }

        (Vec::new(), false)
    }

    fn refresh(&mut self) {
        let (objects, using_runtime_data) = Self::load_objects(&self.current_library);
        self.objects = objects;
        self.using_runtime_data = using_runtime_data;
        self.apply_filter();
        if self.objects.is_empty() {
            self.status_message =
                Some("Runtime catalog not available for this library.".to_string());
        } else {
            self.status_message = None;
        }
    }

    fn apply_filter(&mut self) {
        let name_filter = self.name_filter.trim().to_uppercase();
        let type_filter = self.type_filter.trim().to_uppercase();
        self.filtered_indices = self
            .objects
            .iter()
            .enumerate()
            .filter(|(_, object)| {
                (name_filter.is_empty() || object.name.contains(&name_filter))
                    && (type_filter.is_empty() || object.type_.contains(&type_filter))
            })
            .map(|(index, _)| index)
            .collect();

        let rows = self
            .filtered_indices
            .iter()
            .filter_map(|index| self.objects.get(*index))
            .map(|obj| {
                vec![
                    " ".to_string(),
                    obj.name.clone(),
                    obj.type_.clone(),
                    obj.attribute.clone(),
                    obj.owner.clone(),
                    obj.public_auth.clone(),
                    obj.size.clone(),
                    obj.changed.clone(),
                    obj.text.clone(),
                ]
            })
            .collect::<Vec<_>>();
        self.table.set_rows(rows);
    }

    fn selected_object(&self) -> Option<&ObjectInfo> {
        let filtered_index = self.table.selected()?;
        let object_index = *self.filtered_indices.get(filtered_index)?;
        self.objects.get(object_index)
    }

    fn selected_object_spec(&self) -> Option<String> {
        self.selected_object()
            .map(|object| format!("{}/{}", object.library, object.name))
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
            EditMode::NameFilter => {
                self.name_filter = value;
                self.apply_filter();
                self.status_message = Some(format!(
                    "Name filter: {}",
                    display_filter(&self.name_filter)
                ));
            }
            EditMode::TypeFilter => {
                self.type_filter = value;
                self.apply_filter();
                self.status_message = Some(format!(
                    "Type filter: {}",
                    display_filter(&self.type_filter)
                ));
            }
            EditMode::Library => {
                if value.is_empty() {
                    self.status_message = Some("Library is required.".to_string());
                } else {
                    self.current_library = value;
                    self.session.set_current_library(&self.current_library);
                    self.refresh();
                }
            }
            EditMode::Rename => {
                let Some(spec) = self.selected_object_spec() else {
                    self.status_message = Some("No object selected.".to_string());
                    return ScreenResult::none();
                };
                if value.is_empty() {
                    self.status_message = Some("New object name is required.".to_string());
                } else {
                    self.pending_rename = Some((
                        spec.clone(),
                        value.clone(),
                        ConfirmDialog::new("RNMOBJ", format!("Rename {spec} to {value}?")),
                    ));
                }
            }
            EditMode::CreateName => {
                if value.is_empty() {
                    self.status_message = Some("Object name is required.".to_string());
                } else {
                    self.begin_edit(EditMode::CreateType { name: value }, "*FILE");
                }
            }
            EditMode::CreateType { name } => {
                let objtype = if value.is_empty() {
                    "*FILE".to_string()
                } else {
                    value
                };
                let lib_path = resolve_l400_root().join(&self.current_library);
                let attr = match objtype.as_str() {
                    "*FILE" => Some("PF"),
                    "*DTAQ" => Some("DTAQ"),
                    "*PGM" => Some("ELF"),
                    _ => None,
                };
                match create_object_with_metadata(
                    &lib_path,
                    &name,
                    &objtype,
                    attr,
                    Some("Created from TUI"),
                ) {
                    Ok(_) => {
                        self.status_message =
                            Some(format!("{}/{} created.", self.current_library, name));
                        self.refresh();
                    }
                    Err(error) => {
                        self.status_message = Some(format!("Error creating {name}: {error}"));
                    }
                }
            }
        }

        ScreenResult::none()
    }

    fn request_delete_selected(&mut self) {
        let Some(spec) = self.selected_object_spec() else {
            self.status_message = Some("No object selected.".to_string());
            return;
        };
        self.pending_delete = Some(ConfirmDialog::new("DLTOBJ", format!("Delete {spec}?")));
        self.status_message = Some(format!("DLTOBJ {spec} pending."));
    }

    fn confirm_delete(&mut self) {
        let Some(spec) = self.selected_object_spec() else {
            self.status_message = Some("No object selected.".to_string());
            return;
        };
        let Some((library, object)) = spec.split_once('/') else {
            self.status_message = Some("DLTOBJ cancelled: invalid object spec.".to_string());
            return;
        };
        match delete_object(&resolve_l400_root().join(library).join(object)) {
            Ok(_) => {
                self.status_message = Some(format!("{spec} deleted."));
                self.refresh();
            }
            Err(error) => {
                self.status_message = Some(format!("Error deleting {spec}: {error}"));
            }
        }
    }

    fn confirm_rename(&mut self, spec: String, new_name: String) {
        let Some((library, object)) = spec.split_once('/') else {
            self.status_message = Some("RNMOBJ cancelled: invalid object spec.".to_string());
            return;
        };
        let root = resolve_l400_root();
        match std::fs::rename(
            root.join(library).join(object),
            root.join(library).join(&new_name),
        ) {
            Ok(_) => {
                self.status_message = Some(format!("{spec} renamed to {new_name}."));
                self.refresh();
            }
            Err(error) => {
                self.status_message = Some(format!("Error renaming {spec}: {error}"));
            }
        }
    }
}

impl Screen for ObjectBrowser {
    fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5),      // Header
                Constraint::Min(0),          // Table
                Constraint::Length(1),       // Ruler line
                Constraint::Length(3),       // Status
                Constraint::Length(3),       // Help
            ])
            .split(crate::screens::screen_area(frame));

        self.render_header(frame, chunks[0]);
        self.table.render(frame, chunks[1]);
        render_ruler(frame, chunks[2]);
        self.render_status(frame, chunks[3]);
        self.render_help(frame, chunks[4]);

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
                Some(false) => self.status_message = Some("DLTOBJ cancelled.".to_string()),
                None => self.pending_delete = Some(dialog),
            }
            return ScreenResult::none();
        }

        if let Some((spec, new_name, mut dialog)) = self.pending_rename.take() {
            dialog.handle_key(key);
            match dialog.result() {
                Some(true) => self.confirm_rename(spec, new_name),
                Some(false) => self.status_message = Some("RNMOBJ cancelled.".to_string()),
                None => self.pending_rename = Some((spec, new_name, dialog)),
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
            KeyCode::F(17) => {
                self.begin_edit(EditMode::NameFilter, self.name_filter.clone());
                ScreenResult::none()
            }
            KeyCode::F(18) => {
                self.begin_edit(EditMode::TypeFilter, self.type_filter.clone());
                ScreenResult::none()
            }
            KeyCode::Tab => {
                self.begin_edit(EditMode::Library, self.current_library.clone());
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
            KeyCode::Char('1') => {
                self.option_buffer = "1".to_string();
                self.status_message = Some("Option 12 pending.".to_string());
                ScreenResult::none()
            }
            KeyCode::Char('2') if self.option_buffer == "1" => {
                self.option_buffer.clear();
                self.begin_edit(EditMode::CreateName, "");
                ScreenResult::none()
            }
            KeyCode::Char('5') => self
                .selected_object_spec()
                .map(|spec| {
                    ScreenResult::with_data(ScreenId::ObjectDetail, format!("DSPOBJD OBJ({spec})"))
                })
                .unwrap_or_else(ScreenResult::none),
            KeyCode::Char('3') => self
                .selected_object()
                .filter(|object| object.type_ == "*FILE" && object.attribute == "PF")
                .map(|object| {
                    ScreenResult::with_data(
                        ScreenId::DspPfm,
                        format!("DSPPFM FILE({}/{})", object.library, object.name),
                    )
                })
                .unwrap_or_else(|| {
                    self.status_message = Some("Option 3 requires a *FILE PF.".to_string());
                    ScreenResult::none()
                }),
            KeyCode::Char('4') => {
                self.option_buffer.clear();
                self.request_delete_selected();
                ScreenResult::none()
            }
            KeyCode::Char('2') => self
                .selected_object()
                .filter(|object| object.type_ == "*FILE" && object.attribute == "PF")
                .map(|object| {
                    ScreenResult::with_data(
                        ScreenId::WrkMbrPdm,
                        format!("{}/{}", object.library, object.name),
                    )
                })
                .unwrap_or_else(|| {
                    self.status_message =
                        Some("Option 2 requires a *FILE PF/source file.".to_string());
                    ScreenResult::none()
                }),
            KeyCode::Char('7') => {
                self.option_buffer.clear();
                let initial = self
                    .selected_object()
                    .map(|object| object.name.clone())
                    .unwrap_or_default();
                self.begin_edit(EditMode::Rename, initial);
                ScreenResult::none()
            }
            KeyCode::Char('8') => self
                .selected_object()
                .filter(|object| object.type_ == "*DTAQ")
                .map(|object| {
                    ScreenResult::with_data(
                        ScreenId::DataQueueViewer,
                        format!("{}/{}", object.library, object.name),
                    )
                })
                .unwrap_or_else(|| {
                    self.status_message = Some("Option 8 requires a *DTAQ.".to_string());
                    ScreenResult::none()
                }),
            _ => ScreenResult::none(),
        }
    }
}

impl ObjectBrowser {
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let title = Line::from(vec![
            format!(" Work with Objects  Library: {} ", self.current_library).into(),
        ]);
        let block = Block::default()
            .title(title)
            .style(STYLE_HEADER)
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);
        frame.render_widget(block, area);

        let source_label = if self.using_runtime_data {
            "Runtime catalog"
        } else {
            "No catalog"
        };
        let edit = match &self.edit_mode {
            Some(EditMode::NameFilter) => format!("Name filter: {}", self.edit_buffer),
            Some(EditMode::TypeFilter) => format!("Type filter: {}", self.edit_buffer),
            Some(EditMode::Library) => format!("Library: {}", self.edit_buffer),
            Some(EditMode::Rename) => format!("New name: {}", self.edit_buffer),
            Some(EditMode::CreateName) => format!("New object: {}", self.edit_buffer),
            Some(EditMode::CreateType { name }) => format!("Type for {name}: {}", self.edit_buffer),
            None => format!(
                "Source: {source_label}   Filter: name={} type={}",
                display_filter(&self.name_filter),
                display_filter(&self.type_filter)
            ),
        };
        let text = Text::from(vec![
            Line::from("Options: 1=Create 2=Members 3=Records 4=Delete 5=Display 7=Rename 8=DTAQ"),
            Line::from(edit),
        ]);
        let inner = Rect::new(area.x + 1, area.y + 1, area.width.saturating_sub(2), 3);
        frame.render_widget(Paragraph::new(text).style(STYLE_NORMAL), inner);
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        HelpBar::new()
            .command("WRKOBJ")
            .actions(vec![
                HelpAction::new("F3", "Back"),
                HelpAction::new("F4", "Prompt"),
                HelpAction::new("F5", "Refresh"),
                HelpAction::new("F17", "Name"),
                HelpAction::new("F18", "Type"),
                HelpAction::new("Tab", "Library"),
                HelpAction::new("F12", "Cancel"),
            ])
            .render(frame, area);
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
}

impl Default for ObjectBrowser {
    fn default() -> Self {
        Self::new()
    }
}

fn display_filter(value: &str) -> &str {
    if value.trim().is_empty() {
        "*ALL"
    } else {
        value
    }
}

fn format_system_time(time: std::time::SystemTime) -> String {
    let secs = time
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{secs}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_by_name_and_type() {
        let mut browser = ObjectBrowser::for_library("QGPL".to_string(), SessionContext::new(930));
        browser.objects = vec![
            ObjectInfo {
                library: "QGPL".to_string(),
                name: "CUSTOMERS".to_string(),
                type_: "*FILE".to_string(),
                attribute: "PF".to_string(),
                text: String::new(),
                owner: "QPGMR".to_string(),
                public_auth: "*USE".to_string(),
                size: "0".to_string(),
                changed: "-".to_string(),
            },
            ObjectInfo {
                library: "QGPL".to_string(),
                name: "HELLO".to_string(),
                type_: "*PGM".to_string(),
                attribute: "ELF".to_string(),
                text: String::new(),
                owner: "QPGMR".to_string(),
                public_auth: "*USE".to_string(),
                size: "0".to_string(),
                changed: "-".to_string(),
            },
        ];
        browser.name_filter = "CUST".to_string();
        browser.type_filter = "*FILE".to_string();
        browser.apply_filter();

        assert_eq!(browser.filtered_indices, vec![0]);
    }

    #[test]
    fn option_twelve_starts_create_prompt() {
        let mut browser = ObjectBrowser::for_library("QGPL".to_string(), SessionContext::new(931));
        assert_eq!(
            browser.handle_key(KeyEvent::from(KeyCode::Char('1'))).next,
            None
        );
        assert_eq!(
            browser.handle_key(KeyEvent::from(KeyCode::Char('2'))).next,
            None
        );
        assert!(matches!(browser.edit_mode, Some(EditMode::CreateName)));
    }
}
