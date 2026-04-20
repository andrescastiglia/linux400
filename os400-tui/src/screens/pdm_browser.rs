use crossterm::event::{KeyCode, KeyEvent};
use l400::{list_libraries, list_objects, resolve_l400_root};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::Line,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::screens::{Screen, ScreenId, ScreenResult};
use crate::style::*;

pub struct PdmBrowser {
    libraries: Vec<String>,
    state: ListState,
}

impl PdmBrowser {
    pub fn new() -> Self {
        let mut s = Self {
            libraries: Self::load_libraries(),
            state: ListState::default(),
        };
        if !s.libraries.is_empty() {
            s.state.select(Some(0));
        }
        s
    }

    fn load_libraries() -> Vec<String> {
        let root = resolve_l400_root();
        match list_libraries(&root) {
            Ok(libraries) => libraries,
            Err(_) => vec!["QSYS".to_string(), "QGPL".to_string()],
        }
    }

    fn default_source_file(library: &str) -> String {
        let library_path = resolve_l400_root().join(library);
        match list_objects(&library_path) {
            Ok(objects) => objects
                .into_iter()
                .find(|object| {
                    object.objtype == "*FILE"
                        && (object.name.ends_with("SRC")
                            || object
                                .attribute
                                .as_deref()
                                .is_some_and(|attribute| attribute.contains("SRC")))
                })
                .map(|object| object.name)
                .unwrap_or_else(|| "QCLSRC".to_string()),
            Err(_) => "QCLSRC".to_string(),
        }
    }

    fn refresh(&mut self) {
        self.libraries = Self::load_libraries();
        if self.libraries.is_empty() {
            self.state.select(None);
        } else {
            let sel = self
                .state
                .selected()
                .unwrap_or(0)
                .min(self.libraries.len() - 1);
            self.state.select(Some(sel));
        }
    }

    fn selected_library(&self) -> Option<&str> {
        self.state
            .selected()
            .and_then(|i| self.libraries.get(i).map(|s| s.as_str()))
    }
}

impl Default for PdmBrowser {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for PdmBrowser {
    fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(frame.area());

        self.render_header(frame, chunks[0]);
        self.render_list(frame, chunks[1]);
        self.render_help(frame, chunks[2]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> ScreenResult {
        match key.code {
            KeyCode::F(3) | KeyCode::F(12) => ScreenResult::goto(ScreenId::MainMenu),
            KeyCode::F(5) => {
                self.refresh();
                ScreenResult::none()
            }
            KeyCode::Up => {
                let i = self.state.selected().unwrap_or(0).saturating_sub(1);
                self.state.select(Some(i));
                ScreenResult::none()
            }
            KeyCode::Down => {
                let max = self.libraries.len().saturating_sub(1);
                let i = self
                    .state
                    .selected()
                    .unwrap_or(0)
                    .saturating_add(1)
                    .min(max);
                self.state.select(Some(i));
                ScreenResult::none()
            }
            KeyCode::Enter => {
                if let Some(lib) = self.selected_library() {
                    ScreenResult::with_data(
                        ScreenId::WrkMbrPdm,
                        format!("{}/{}", lib, Self::default_source_file(lib)),
                    )
                } else {
                    ScreenResult::none()
                }
            }
            _ => ScreenResult::none(),
        }
    }
}

impl PdmBrowser {
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" STRPDM - Programming Development Manager ")
            .style(STYLE_HEADER)
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);
        frame.render_widget(block, area);

        let text = Line::from("  Select a library and press Enter.");
        let inner = Rect::new(area.x + 1, area.y + 1, area.width - 2, 2);
        frame.render_widget(
            Paragraph::new(vec![Line::from(""), text]).style(STYLE_NORMAL),
            inner,
        );
    }

    fn render_list(&mut self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .libraries
            .iter()
            .map(|name| ListItem::new(Line::from(format!("  {}", name))))
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title(format!(" Libraries ({}) ", self.libraries.len()))
                    .borders(Borders::ALL)
                    .border_style(STYLE_BORDER),
            )
            .style(STYLE_NORMAL)
            .highlight_style(STYLE_SELECTION);

        frame.render_stateful_widget(list, area, &mut self.state);
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let help_text = Line::from(vec![
            "F3=Exit   ".into(),
            "F5=Refresh   ".into(),
            "F12=Cancel   ".into(),
            "Enter=Select Library".into(),
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
