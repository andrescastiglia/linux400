use crossterm::event::{KeyCode, KeyEvent};
use l400::{delete_object, list_objects, resolve_l400_root};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::Line,
    text::Text,
    widgets::{Block, Borders, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::screens::{Screen, ScreenId, ScreenResult};
use crate::session::SessionContext;
use crate::style::*;

pub struct ObjectInfo {
    pub library: String,
    pub name: String,
    pub type_: String,
    pub attribute: String,
    pub text: String,
    pub owner: String,
    pub public_auth: String,
}

pub struct ObjectBrowser {
    current_library: String,
    objects: Vec<ObjectInfo>,
    state: TableState,
    using_runtime_data: bool,
    session: SessionContext,
    status_message: Option<String>,
    pending_delete: Option<String>,
}

impl ObjectBrowser {
    pub fn new() -> Self {
        Self::with_session(SessionContext::new(std::process::id() as u64))
    }

    pub fn with_session(session: SessionContext) -> Self {
        let current_library = session.snapshot().current_library;
        let (objects, using_runtime_data) = Self::load_objects(&current_library);
        let status_message = objects
            .is_empty()
            .then(|| "Sin catalogo runtime para esta biblioteca.".to_string());
        Self {
            current_library,
            objects,
            state: TableState::default(),
            using_runtime_data,
            session,
            status_message,
            pending_delete: None,
        }
    }

    fn fallback_objects(library: &str) -> Vec<ObjectInfo> {
        let _ = library;
        Vec::new()
    }

    fn load_objects(library: &str) -> (Vec<ObjectInfo>, bool) {
        let library_path = resolve_l400_root().join(library);
        if let Ok(objects) = list_objects(&library_path) {
            let mapped = objects
                .into_iter()
                .map(|object| ObjectInfo {
                    library: object.library.unwrap_or_else(|| library.to_string()),
                    name: object.name,
                    type_: object.objtype,
                    attribute: object.attribute.unwrap_or_else(|| "-".to_string()),
                    text: object.text.unwrap_or_default(),
                    owner: object.owner.unwrap_or_else(|| "UNKNOWN".to_string()),
                    public_auth: object.public_auth.unwrap_or_else(|| "*NONE".to_string()),
                })
                .collect::<Vec<_>>();
            return (mapped, true);
        }

        (Self::fallback_objects(library), false)
    }

    fn refresh(&mut self) {
        self.current_library = self.session.snapshot().current_library;
        let (objects, using_runtime_data) = Self::load_objects(&self.current_library);
        self.objects = objects;
        self.using_runtime_data = using_runtime_data;
        if self.objects.is_empty() {
            self.state.select(None);
            self.status_message = Some("Sin catalogo runtime para esta biblioteca.".to_string());
        } else if self.state.selected().is_none() {
            self.state.select(Some(0));
            self.status_message = None;
        }
    }

    fn selected_object(&self) -> Option<&ObjectInfo> {
        self.state
            .selected()
            .and_then(|index| self.objects.get(index))
    }

    fn selected_object_spec(&self) -> Option<String> {
        self.selected_object()
            .map(|object| format!("{}/{}", object.library, object.name))
    }

    fn request_delete_selected(&mut self) {
        let Some(spec) = self.selected_object_spec() else {
            self.status_message = Some("No hay objeto seleccionado.".to_string());
            return;
        };
        self.pending_delete = Some(spec.clone());
        self.status_message = Some(format!(
            "Confirmar DLTOBJ {}: presione Enter para borrar o F12 para cancelar.",
            spec
        ));
    }

    fn confirm_delete(&mut self) {
        let Some(spec) = self.pending_delete.take() else {
            return;
        };
        let root = resolve_l400_root();
        let Some((library, object)) = spec.split_once('/') else {
            self.status_message = Some("DLTOBJ cancelado: especificacion invalida.".to_string());
            return;
        };
        match delete_object(&root.join(library).join(object)) {
            Ok(_) => {
                self.status_message = Some(format!("{} borrado.", spec));
                self.refresh();
            }
            Err(error) => {
                self.status_message = Some(format!("Error borrando {}: {}", spec, error));
            }
        }
    }
}

impl Screen for ObjectBrowser {
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
        self.render_objects(frame, chunks[1]);
        self.render_status(frame, chunks[2]);
        self.render_help(frame, chunks[3]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> ScreenResult {
        if self.pending_delete.is_some() {
            match key.code {
                KeyCode::Enter => {
                    self.confirm_delete();
                    return ScreenResult::none();
                }
                KeyCode::F(12) | KeyCode::Esc => {
                    self.pending_delete = None;
                    self.status_message = Some("DLTOBJ cancelado.".to_string());
                    return ScreenResult::none();
                }
                _ => return ScreenResult::none(),
            }
        }
        match key.code {
            KeyCode::F(3) => ScreenResult::goto(ScreenId::MainMenu),
            KeyCode::F(4) => ScreenResult::goto(ScreenId::CommandLine),
            KeyCode::F(5) => {
                self.refresh();
                ScreenResult::none()
            }
            KeyCode::F(12) => ScreenResult::goto(ScreenId::MainMenu),
            KeyCode::Up => {
                self.state
                    .select(Some(self.state.selected().unwrap_or(0).saturating_sub(1)));
                ScreenResult::none()
            }
            KeyCode::Down => {
                let max = self.objects.len().saturating_sub(1);
                let current = self.state.selected().unwrap_or(0);
                self.state.select(Some(current.saturating_add(1).min(max)));
                ScreenResult::none()
            }
            KeyCode::PageUp | KeyCode::PageDown => ScreenResult::none(),
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
                        ScreenId::SystemPanel,
                        format!("DSPPFM FILE({}/{})", object.library, object.name),
                    )
                })
                .unwrap_or_else(|| {
                    self.status_message = Some("Opcion 3 requiere un *FILE PF.".to_string());
                    ScreenResult::none()
                }),
            KeyCode::Char('4') => {
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
                        Some("Opcion 2 requiere un *FILE PF/source file.".to_string());
                    ScreenResult::none()
                }),
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
                    self.status_message = Some("Opcion 8 requiere un *DTAQ.".to_string());
                    ScreenResult::none()
                }),
            _ => ScreenResult::none(),
        }
    }
}

impl ObjectBrowser {
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let title = Line::from(vec![format!(
            " Work with Objects  Library: {} ",
            self.current_library
        )
        .into()]);

        let block = Block::default()
            .title(title)
            .style(STYLE_HEADER)
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);

        frame.render_widget(block, area);

        let source_label = if self.using_runtime_data {
            "Runtime catalog"
        } else {
            "Sin catalogo"
        };
        let lines: Vec<Line> = vec![
            Line::from(vec![format!(
                "Source: {}. Options: 2=Members 3=Records 4=Delete 5=Display 8=DTAQ.",
                source_label
            )
            .into()]),
            Line::from(vec![
                "Opt  Object      Type      Attribute   Owner       *PUBLIC   Text".into(),
            ]),
        ];
        let text = Text::from(lines);

        let inner = Rect::new(area.x + 1, area.y + 1, area.width - 2, 2);
        frame.render_widget(Paragraph::new(text).style(STYLE_NORMAL), inner);
    }

    fn render_objects(&mut self, frame: &mut Frame, area: Rect) {
        let header = [
            "Opt",
            "Object",
            "Type",
            "Attribute",
            "Owner",
            "*PUBLIC",
            "Text",
        ];
        let widths = [4u16, 12, 10, 10, 12, 10, 20];

        let rows: Vec<Row> = self
            .objects
            .iter()
            .map(|obj| {
                Row::new(vec![
                    " ".to_string(),
                    obj.name.clone(),
                    obj.type_.clone(),
                    obj.attribute.clone(),
                    obj.owner.clone(),
                    obj.public_auth.clone(),
                    obj.text.clone(),
                ])
            })
            .collect();

        let table = Table::new(rows, widths.iter().map(|w| Constraint::Length(*w)))
            .header(
                Row::new(header.to_vec())
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

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let help_text = Line::from(vec![
            "F3=Exit   ".into(),
            "F4=Prompt   ".into(),
            "F5=Refresh   ".into(),
            "2=Members   ".into(),
            "3=Records   ".into(),
            "4=Delete   ".into(),
            "5=Display   ".into(),
            "8=DTAQ   ".into(),
            "F12=Cancel".into(),
        ]);

        let block = Block::default()
            .style(STYLE_HELP)
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);

        frame.render_widget(block, area);

        let inner = Rect::new(area.x + 1, area.y + 1, area.width - 2, 1);
        frame.render_widget(Paragraph::new(help_text).style(STYLE_HELP), inner);
    }

    fn render_status(&self, frame: &mut Frame, area: Rect) {
        let message = self.status_message.clone().unwrap_or_default();
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(Paragraph::new(message).style(STYLE_NORMAL), inner);
    }
}

impl Default for ObjectBrowser {
    fn default() -> Self {
        Self::new()
    }
}
