use crossterm::event::{KeyCode, KeyEvent};
use l400::{
    L400_STORAGE_BACKEND_ATTR, catalog_object, describe_object, read_string_attr, resolve_l400_root,
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph},
};

use crate::cl_parser::{extract_command_arg, tokenize_cl_command};
use crate::screens::{Screen, ScreenId, ScreenResult};
use crate::session::SessionContext;
use crate::style::*;
use crate::widgets::help_bar::{CpfMessage, HelpAction, HelpBar};

pub struct ObjectDetail {
    object_spec: String,
    fields: Vec<(String, String)>,
    session: SessionContext,
    status_message: Option<String>,
    editing_text: bool,
    edit_buffer: String,
}

impl ObjectDetail {
    pub fn new(data: Option<&str>, session: SessionContext) -> Self {
        let object_spec = data
            .and_then(extract_object_spec)
            .unwrap_or_else(|| format!("{}/{}", session.snapshot().current_library, "*ALL"));
        let mut screen = Self {
            object_spec,
            fields: Vec::new(),
            session,
            status_message: None,
            editing_text: false,
            edit_buffer: String::new(),
        };
        screen.refresh();
        screen
    }

    fn refresh(&mut self) {
        self.fields.clear();
        let path = object_path(&self.object_spec, &self.session);
        match describe_object(&path) {
            Ok(object) => {
                let metadata = std::fs::metadata(&path).ok();
                let storage_backend = read_string_attr(&path, L400_STORAGE_BACKEND_ATTR)
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| {
                        if path.is_dir() {
                            "directory".to_string()
                        } else {
                            "plain-file".to_string()
                        }
                    });
                let auth_summary = read_string_attr(&path, l400::auth::L400_AUTH_MANIFEST_ATTR)
                    .ok()
                    .flatten()
                    .map(|manifest| format!("{} bytes", manifest.len()))
                    .unwrap_or_else(|| "No manifest".to_string());

                self.fields = vec![
                    ("Object".to_string(), object.name),
                    (
                        "Library".to_string(),
                        object.library.unwrap_or_else(|| "-".to_string()),
                    ),
                    ("Type".to_string(), object.objtype),
                    (
                        "Attribute".to_string(),
                        object.attribute.unwrap_or_else(|| "-".to_string()),
                    ),
                    (
                        "Owner".to_string(),
                        object.owner.unwrap_or_else(|| "-".to_string()),
                    ),
                    ("Text".to_string(), object.text.unwrap_or_default()),
                    (
                        "Created".to_string(),
                        metadata_time(metadata.as_ref(), true),
                    ),
                    (
                        "Changed".to_string(),
                        metadata_time(metadata.as_ref(), false),
                    ),
                    ("Last used".to_string(), "-".to_string()),
                    (
                        "Size".to_string(),
                        metadata
                            .as_ref()
                            .map(|metadata| metadata.len().to_string())
                            .unwrap_or_else(|| "-".to_string()),
                    ),
                    ("Storage backend".to_string(), storage_backend),
                    (
                        "Public authority".to_string(),
                        object.public_auth.unwrap_or_else(|| "*NONE".to_string()),
                    ),
                    ("Auth manifest".to_string(), auth_summary),
                ];
                self.status_message = None;
            }
            Err(error) => {
                self.fields = vec![
                    ("Object".to_string(), self.object_spec.clone()),
                    ("Error".to_string(), error.to_string()),
                ];
                self.status_message =
                    Some(format!("Error displaying {}: {}", self.object_spec, error));
            }
        }
    }

    fn current_text(&self) -> String {
        self.fields
            .iter()
            .find(|(name, _)| name == "Text")
            .map(|(_, value)| value.clone())
            .unwrap_or_default()
    }

    fn save_text(&mut self) {
        let path = object_path(&self.object_spec, &self.session);
        match describe_object(&path).and_then(|object| {
            catalog_object(
                &path,
                &object.objtype,
                object.attribute.as_deref(),
                Some(self.edit_buffer.trim()),
            )
        }) {
            Ok(_) => {
                self.status_message = Some("Object text changed.".to_string());
                self.refresh();
            }
            Err(error) => {
                self.status_message = Some(format!("Error changing text: {error}"));
            }
        }
        self.editing_text = false;
        self.edit_buffer.clear();
    }
}

impl Screen for ObjectDetail {
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
        if self.editing_text {
            return match key.code {
                KeyCode::Enter => {
                    self.save_text();
                    ScreenResult::none()
                }
                KeyCode::F(12) | KeyCode::Esc => {
                    self.editing_text = false;
                    self.edit_buffer.clear();
                    ScreenResult::none()
                }
                KeyCode::Backspace => {
                    self.edit_buffer.pop();
                    ScreenResult::none()
                }
                KeyCode::Char(c) => {
                    self.edit_buffer.push(c);
                    ScreenResult::none()
                }
                _ => ScreenResult::none(),
            };
        }

        match key.code {
            KeyCode::F(3) | KeyCode::F(12) => ScreenResult::back(),
            KeyCode::F(5) => {
                self.refresh();
                ScreenResult::none()
            }
            KeyCode::Char('2') => {
                self.editing_text = true;
                self.edit_buffer = self.current_text();
                ScreenResult::none()
            }
            KeyCode::Char('8') => ScreenResult::with_data(
                ScreenId::ObjectAuthority,
                format!("DSPOBJAUT OBJ({})", self.object_spec),
            ),
            _ => ScreenResult::none(),
        }
    }
}

impl ObjectDetail {
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(format!(" Display Object Detail  {} ", self.object_spec))
            .style(STYLE_HEADER)
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);
        frame.render_widget(block, area);
        let line = if self.editing_text {
            format!("Text: {}", self.edit_buffer)
        } else {
            "Options: 2=Change text 8=Authorities".to_string()
        };
        let inner = Rect::new(area.x + 1, area.y + 1, area.width.saturating_sub(2), 2);
        frame.render_widget(Paragraph::new(line).style(STYLE_NORMAL), inner);
    }

    fn render_fields(&self, frame: &mut Frame, area: Rect) {
        let lines = self
            .fields
            .iter()
            .map(|(name, value)| Line::from(format!("{name:<18}: {value}")))
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
            .command("DSPOBJD")
            .actions(vec![
                HelpAction::new("F3", "Back"),
                HelpAction::new("F5", "Refresh"),
                HelpAction::new("2", "Text"),
                HelpAction::new("8", "Auth"),
                HelpAction::new("F12", "Cancel"),
            ])
            .render(frame, area);
    }
}

fn extract_object_spec(command: &str) -> Option<String> {
    let tokens = tokenize_cl_command(command);
    extract_command_arg(&tokens[1..], "OBJ")
        .or_else(|| tokens.get(1).cloned())
        .map(|value| value.trim().to_uppercase())
}

fn object_path(spec: &str, session: &SessionContext) -> std::path::PathBuf {
    let root = resolve_l400_root();
    if let Some((library, object)) = spec.split_once('/') {
        root.join(library.trim()).join(object.trim())
    } else {
        root.join(session.snapshot().current_library)
            .join(spec.trim())
    }
}

fn metadata_time(metadata: Option<&std::fs::Metadata>, created: bool) -> String {
    let time = if created {
        metadata.and_then(|metadata| metadata.created().ok())
    } else {
        metadata.and_then(|metadata| metadata.modified().ok())
    };
    time.and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|| "-".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_object_spec_from_dspobjd_command() {
        assert_eq!(
            extract_object_spec("DSPOBJD OBJ(QGPL/HELLO)").as_deref(),
            Some("QGPL/HELLO")
        );
    }
}
