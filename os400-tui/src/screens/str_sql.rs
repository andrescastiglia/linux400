use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use l400::run_select_query;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::Line,
    widgets::{Block, Borders, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::screens::{Screen, ScreenId, ScreenResult};
use crate::session::SessionContext;
use crate::style::*;

pub struct StrSql {
    input: String,
    cursor: usize,
    results: Vec<Vec<String>>,
    columns: Vec<String>,
    error: Option<String>,
    table_state: TableState,
    history: Vec<String>,
    history_idx: usize,
    default_library: Option<String>,
    return_to: ScreenId,
    return_data: Option<String>,
    session: SessionContext,
}

impl StrSql {
    pub fn new() -> Self {
        Self::with_session(
            None,
            ScreenId::MainMenu,
            None,
            SessionContext::new(std::process::id() as u64),
        )
    }

    pub fn with_session(
        context: Option<String>,
        return_to: ScreenId,
        return_data: Option<String>,
        session: SessionContext,
    ) -> Self {
        let (default_library, input) = match context {
            Some(context) => {
                let (library, file) = parse_context(&context);
                (
                    Some(library.clone()),
                    format!("SELECT * FROM {library}/{file}"),
                )
            }
            None => (Some(session.snapshot().current_library), String::new()),
        };

        let cursor = input.len();
        Self {
            input,
            cursor,
            results: Vec::new(),
            columns: Vec::new(),
            error: None,
            table_state: TableState::default(),
            history: Vec::new(),
            history_idx: 0,
            default_library,
            return_to,
            return_data,
            session,
        }
    }

    fn back_result(&self) -> ScreenResult {
        ScreenResult {
            next: Some(self.return_to),
            data: self.return_data.clone(),
        }
    }

    fn execute(&mut self) {
        let statement = self.input.trim().to_string();
        if statement.is_empty() {
            return;
        }

        if !self.history.iter().any(|entry| entry == &statement) {
            self.history.insert(0, statement.clone());
        }
        self.history_idx = 0;
        self.results.clear();
        self.columns.clear();
        self.error = None;
        self.table_state.select(None);

        self.session.apply_env();
        let snapshot = self.session.snapshot();
        let default_library = self
            .default_library
            .as_deref()
            .unwrap_or(snapshot.current_library.as_str());
        match run_select_query(&statement, Some(default_library)) {
            Ok(result) => {
                self.columns = result.columns;
                self.results = result.rows;
                if !self.results.is_empty() {
                    self.table_state.select(Some(0));
                }
            }
            Err(error) => {
                self.error = Some(error.to_string());
            }
        }

        self.input.clear();
        self.cursor = 0;
    }
}

impl Default for StrSql {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for StrSql {
    fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(frame.area());

        self.render_header(frame, chunks[0]);
        self.render_results(frame, chunks[1]);
        self.render_input(frame, chunks[2]);
        self.render_help(frame, chunks[3]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> ScreenResult {
        match key.code {
            KeyCode::F(3) | KeyCode::F(12) | KeyCode::Esc => self.back_result(),
            KeyCode::F(5) => {
                self.results.clear();
                self.columns.clear();
                self.error = None;
                ScreenResult::none()
            }
            KeyCode::Enter => {
                self.execute();
                ScreenResult::none()
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.input.remove(self.cursor);
                }
                ScreenResult::none()
            }
            KeyCode::Delete => {
                if self.cursor < self.input.len() {
                    self.input.remove(self.cursor);
                }
                ScreenResult::none()
            }
            KeyCode::Left => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
                ScreenResult::none()
            }
            KeyCode::Right => {
                if self.cursor < self.input.len() {
                    self.cursor += 1;
                }
                ScreenResult::none()
            }
            KeyCode::Home => {
                self.cursor = 0;
                ScreenResult::none()
            }
            KeyCode::End => {
                self.cursor = self.input.len();
                ScreenResult::none()
            }
            KeyCode::Up => {
                if !self.results.is_empty() {
                    let next = self.table_state.selected().unwrap_or(0).saturating_sub(1);
                    self.table_state.select(Some(next));
                } else if self.history_idx < self.history.len().saturating_sub(1) {
                    self.history_idx += 1;
                    self.input = self.history[self.history_idx].clone();
                    self.cursor = self.input.len();
                }
                ScreenResult::none()
            }
            KeyCode::Down => {
                if !self.results.is_empty() {
                    let max = self.results.len().saturating_sub(1);
                    let next = self
                        .table_state
                        .selected()
                        .unwrap_or(0)
                        .saturating_add(1)
                        .min(max);
                    self.table_state.select(Some(next));
                } else if self.history_idx > 0 {
                    self.history_idx -= 1;
                    self.input = self.history[self.history_idx].clone();
                    self.cursor = self.input.len();
                }
                ScreenResult::none()
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.insert(self.cursor, c);
                self.cursor += 1;
                ScreenResult::none()
            }
            _ => ScreenResult::none(),
        }
    }
}

impl StrSql {
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let snapshot = self.session.snapshot();
        let library = self
            .default_library
            .as_deref()
            .unwrap_or(snapshot.current_library.as_str());
        let block = Block::default()
            .title(format!(" STRSQL - Interactive SQL  Library: {} ", library))
            .style(STYLE_HEADER)
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);
        frame.render_widget(block, area);

        let hint = "  Supported: SELECT [*|KEY|DATA] FROM [LIB/]FILE [WHERE KEY='X']";
        let inner = Rect::new(area.x + 1, area.y + 1, area.width - 2, 1);
        frame.render_widget(Paragraph::new(hint).style(STYLE_NORMAL), inner);
    }

    fn render_results(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Results ")
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);

        if let Some(error) = &self.error {
            let inner = block.inner(area);
            frame.render_widget(block, area);
            frame.render_widget(
                Paragraph::new(format!("  ERROR: {}", error))
                    .style(ratatui::style::Style::default().fg(ratatui::style::Color::Red)),
                inner,
            );
            return;
        }

        if self.columns.is_empty() {
            let inner = block.inner(area);
            frame.render_widget(block, area);
            frame.render_widget(
                Paragraph::new("  Execute a SELECT statement to see results.").style(STYLE_NORMAL),
                inner,
            );
            return;
        }

        let widths = self
            .columns
            .iter()
            .map(|_| Constraint::Min(20))
            .collect::<Vec<_>>();
        let rows = self.results.iter().map(|row| Row::new(row.clone()));

        let table = Table::new(rows, widths)
            .header(
                Row::new(self.columns.clone())
                    .style(STYLE_TABLE_HEADER)
                    .height(1),
            )
            .block(block)
            .style(STYLE_NORMAL)
            .row_highlight_style(STYLE_SELECTION);

        frame.render_stateful_widget(table, area, &mut self.table_state);
    }

    fn render_input(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(
            Paragraph::new(format!("SQL> {}", self.input)).style(STYLE_NORMAL),
            inner,
        );

        let cursor_x = inner.x + 5 + self.cursor as u16;
        if cursor_x < inner.x + inner.width {
            frame.set_cursor_position((cursor_x, inner.y));
        }
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let help_text = Line::from(vec![
            "F3=Exit   ".into(),
            "F5=Clear   ".into(),
            "F12=Cancel   ".into(),
            "Enter=Run   ".into(),
            "Up/Down=History".into(),
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

fn parse_context(context: &str) -> (String, String) {
    if let Some((library, file)) = context.split_once('/') {
        (library.trim().to_uppercase(), file.trim().to_uppercase())
    } else {
        ("QSYS".to_string(), context.trim().to_uppercase())
    }
}
