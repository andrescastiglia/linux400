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
    pending_option: String,
}

impl MainMenu {
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            pending_option: String::new(),
        }
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

    fn option_index(option: &str) -> Option<usize> {
        Self::menu_items()
            .iter()
            .position(|(item_option, _, _)| *item_option == option)
    }

    fn move_selection(&mut self, step: isize) {
        let items = Self::menu_items();
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
        let items = Self::menu_items();
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

        let has_prefix = Self::menu_items()
            .iter()
            .any(|(option, _, _)| option.starts_with(&self.pending_option));
        if !has_prefix {
            self.pending_option.clear();
            return ScreenResult::none();
        }

        if let Some(idx) = Self::option_index(&self.pending_option) {
            self.selected_index = idx;

            let has_longer_match = Self::menu_items()
                .iter()
                .any(|(option, _, _)| option != &self.pending_option && option.starts_with(&self.pending_option));
            if !has_longer_match {
                return self.execute_selected();
            }
        }

        ScreenResult::none()
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
            KeyCode::F(3) => ScreenResult::exit(),
            KeyCode::F(4) => {
                self.pending_option.clear();
                ScreenResult::goto(ScreenId::CommandLine)
            }
            KeyCode::F(12) | KeyCode::Esc => {
                self.pending_option.clear();
                ScreenResult::none()
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
            KeyCode::Enter => self.execute_selected(),
            KeyCode::Backspace => {
                self.pending_option.pop();
                if let Some(idx) = Self::option_index(&self.pending_option) {
                    self.selected_index = idx;
                }
                ScreenResult::none()
            }
            KeyCode::Char(c)
                if c.is_ascii_digit() && !key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.apply_pending_option(c)
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
            "Selection: ".into(),
            self.pending_option.clone().into(),
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
            "Enter=Select".into(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn f3_exits_menu() {
        let mut menu = MainMenu::new();
        let result = menu.handle_key(key(KeyCode::F(3)));
        assert_eq!(result.next, Some(ScreenId::Exit));
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
        assert_eq!(result.next, Some(ScreenId::ObjectBrowser));
    }

    #[test]
    fn option_ten_can_be_selected_by_keyboard() {
        let mut menu = MainMenu::new();
        assert_eq!(menu.handle_key(key(KeyCode::Char('1'))).next, None);
        let result = menu.handle_key(key(KeyCode::Char('0')));
        assert_eq!(result.next, Some(ScreenId::ObjectBrowser));
    }

    #[test]
    fn f4_opens_command_line() {
        let mut menu = MainMenu::new();
        let result = menu.handle_key(key(KeyCode::F(4)));
        assert_eq!(result.next, Some(ScreenId::CommandLine));
    }
}
