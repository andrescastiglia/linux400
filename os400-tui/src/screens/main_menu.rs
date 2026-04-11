use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::screens::{Screen, ScreenId, ScreenResult};
use crate::style::*;

pub struct MainMenu {
    selected_index: usize,
}

impl MainMenu {
    pub fn new() -> Self {
        Self { selected_index: 0 }
    }

    fn menu_items() -> Vec<(&'static str, &'static str, &'static str)> {
        vec![
            ("1", "Work with libraries . . . . . . . . . . .", "WRKLIB"),
            ("2", "Work with programs  . . . . . . . . . .", "WRKPGM"),
            ("3", "Work with files . . . . . . . . . . . .", "WRKOBJ"),
            ("4", "Work with jobs . . . . . . . . . . . .", "WRKACTJOB"),
            ("5", "Data queues  . . . . . . . . . . . . .", "DSPDTAQ"),
            ("6", "Command entry . . . . . . . . . . . .", "CMD"),
            (" ", " ", " "),
            ("10", "System configuration  . . . . . . . .", "CFG"),
        ]
    }

    fn handle_option(&self, option: &str) -> ScreenResult {
        match option {
            "1" | "2" | "3" => ScreenResult::goto(ScreenId::ObjectBrowser),
            "4" => ScreenResult::goto(ScreenId::WorkManagement),
            "5" => ScreenResult::goto(ScreenId::DataQueueViewer),
            "6" => ScreenResult::goto(ScreenId::CommandLine),
            "10" => ScreenResult::goto(ScreenId::ObjectBrowser),
            _ => ScreenResult::none(),
        }
    }
}

impl Screen for MainMenu {
    fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(frame.size());

        self.render_header(frame, chunks[0]);
        self.render_menu(frame, chunks[1]);
        self.render_help(frame, chunks[2]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> ScreenResult {
        match key.code {
            KeyCode::Esc | KeyCode::Char('3') if key.modifiers.contains(KeyModifiers::ALT) => {
                ScreenResult::exit()
            }
            KeyCode::Char('3') => ScreenResult::exit(),
            KeyCode::Up => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
                ScreenResult::none()
            }
            KeyCode::Down => {
                let max = Self::menu_items().len() - 1;
                if self.selected_index < max {
                    self.selected_index += 1;
                }
                ScreenResult::none()
            }
            KeyCode::Enter => {
                let items = Self::menu_items();
                if self.selected_index < items.len() {
                    self.handle_option(items[self.selected_index].0)
                } else {
                    ScreenResult::none()
                }
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                let option = c.to_string();
                self.handle_option(&option)
            }
            _ => ScreenResult::none(),
        }
    }
}

impl MainMenu {
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let title = Line::from(vec![" L400 Main Menu ".into()]);

        let block = Block::default()
            .title(title)
            .style(STYLE_HEADER)
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);

        frame.render_widget(block, area);

        let text = Text::from(vec![Line::from(vec![
            "System: ".into(),
            "L400   ".into(),
            "Library: ".into(),
            "QSYS   ".into(),
        ])]);

        let inner = Rect::new(area.x + 1, area.y + 1, area.width - 2, 1);
        frame.render_widget(Paragraph::new(text).style(STYLE_NORMAL), inner);
    }

    fn render_menu(&self, frame: &mut Frame, area: Rect) {
        let items = Self::menu_items();
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
                    .title("Work with objects")
                    .borders(Borders::ALL)
                    .border_style(STYLE_BORDER),
            )
            .style(STYLE_NORMAL);

        frame.render_widget(list, area);
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let help_text = Line::from(vec![
            "F3=Exit   ".into(),
            "F4=Prompt   ".into(),
            "F12=Cancel   ".into(),
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

impl Default for MainMenu {
    fn default() -> Self {
        Self::new()
    }
}
