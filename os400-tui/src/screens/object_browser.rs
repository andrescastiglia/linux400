use crossterm::event::{KeyCode, KeyEvent};
use l400::{list_objects, resolve_l400_root};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::Line,
    text::Text,
    widgets::{Block, Borders, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::screens::{Screen, ScreenId, ScreenResult};
use crate::style::*;

pub struct ObjectInfo {
    pub library: String,
    pub name: String,
    pub type_: String,
    pub attribute: String,
    pub text: String,
}

pub struct ObjectBrowser {
    current_library: String,
    objects: Vec<ObjectInfo>,
    state: TableState,
    using_runtime_data: bool,
}

impl ObjectBrowser {
    pub fn new() -> Self {
        let (objects, using_runtime_data) = Self::load_objects("QSYS");
        Self {
            current_library: "QSYS".to_string(),
            objects,
            state: TableState::default(),
            using_runtime_data,
        }
    }

    fn fallback_objects(library: &str) -> Vec<ObjectInfo> {
        match library {
            "QSYS" => vec![
                ObjectInfo {
                    library: "QSYS".to_string(),
                    name: "QCMD".to_string(),
                    type_: "*PGM".to_string(),
                    attribute: "CL".to_string(),
                    text: "Command processing program".to_string(),
                },
                ObjectInfo {
                    library: "QSYS".to_string(),
                    name: "QCPYA".to_string(),
                    type_: "*FILE".to_string(),
                    attribute: "PF".to_string(),
                    text: "Physical file".to_string(),
                },
                ObjectInfo {
                    library: "QSYS".to_string(),
                    name: "QCLSRC".to_string(),
                    type_: "*FILE".to_string(),
                    attribute: "LF".to_string(),
                    text: "Source file".to_string(),
                },
                ObjectInfo {
                    library: "QSYS".to_string(),
                    name: "QSNDDTAQ".to_string(),
                    type_: "*PGM".to_string(),
                    attribute: "RPG".to_string(),
                    text: "Send to data queue".to_string(),
                },
                ObjectInfo {
                    library: "QSYS".to_string(),
                    name: "QCMDEXC".to_string(),
                    type_: "*SRVPGM".to_string(),
                    attribute: "C".to_string(),
                    text: "Command execution".to_string(),
                },
            ],
            _ => vec![ObjectInfo {
                library: library.to_string(),
                name: "TESTPGM".to_string(),
                type_: "*PGM".to_string(),
                attribute: "C".to_string(),
                text: "Test program".to_string(),
            }],
        }
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
                })
                .collect::<Vec<_>>();
            return (mapped, true);
        }

        (Self::fallback_objects(library), false)
    }

    fn refresh(&mut self) {
        let (objects, using_runtime_data) = Self::load_objects(&self.current_library);
        self.objects = objects;
        self.using_runtime_data = using_runtime_data;
        if self.objects.is_empty() {
            self.state.select(None);
        } else if self.state.selected().is_none() {
            self.state.select(Some(0));
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
            ])
            .split(frame.size());

        self.render_header(frame, chunks[0]);
        self.render_objects(frame, chunks[1]);
        self.render_help(frame, chunks[2]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> ScreenResult {
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
            "Bundled sample"
        };
        let lines: Vec<Line> = vec![
            Line::from(vec![format!(
                "Source: {}. Type options, press Enter.",
                source_label
            )
            .into()]),
            Line::from(vec!["Opt  Object      Type      Attribute   Text".into()]),
        ];
        let text = Text::from(lines);

        let inner = Rect::new(area.x + 1, area.y + 1, area.width - 2, 2);
        frame.render_widget(Paragraph::new(text).style(STYLE_NORMAL), inner);
    }

    fn render_objects(&mut self, frame: &mut Frame, area: Rect) {
        let header = ["", "Object", "Type", "Attribute", "Text"];
        let widths = [4u16, 16, 10, 10, 30];

        let rows: Vec<Row> = self
            .objects
            .iter()
            .map(|obj| {
                Row::new(vec![
                    " ".to_string(),
                    obj.name.clone(),
                    obj.type_.clone(),
                    obj.attribute.clone(),
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
            .highlight_style(STYLE_SELECTION);

        frame.render_stateful_widget(table, area, &mut self.state);
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let help_text = Line::from(vec![
            "F3=Exit   ".into(),
            "F4=Prompt   ".into(),
            "F5=Refresh   ".into(),
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
}

impl Default for ObjectBrowser {
    fn default() -> Self {
        Self::new()
    }
}
