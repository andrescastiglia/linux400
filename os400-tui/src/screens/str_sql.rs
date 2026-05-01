use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use l400::{list_libraries, list_objects, read_pf_schema, resolve_l400_root, run_select_query};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table, TableState},
};

use crate::screens::{Screen, ScreenId, ScreenResult};
use crate::session::SessionContext;
use crate::style::*;
use crate::widgets::help_bar::{HelpAction, HelpBar};

pub struct StrSql {
    input: String,
    cursor: usize,
    results: Vec<Vec<String>>,
    columns: Vec<String>,
    error: Option<String>,
    table_state: TableState,
    column_offset: usize,
    history: Vec<String>,
    history_idx: usize,
    status_message: Option<String>,
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
            column_offset: 0,
            history: Vec::new(),
            history_idx: 0,
            status_message: None,
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
        self.column_offset = 0;

        self.session.apply_env();
        let snapshot = self.session.snapshot();
        let default_library = self
            .default_library
            .as_deref()
            .unwrap_or(snapshot.current_library.as_str());
        if statement.eq_ignore_ascii_case("SHOW TABLES") {
            self.show_tables();
        } else if statement.to_uppercase().starts_with("DESCRIBE TABLE ") {
            self.describe_table(statement["DESCRIBE TABLE ".len()..].trim());
        } else {
            match run_select_query(&statement, Some(default_library)) {
                Ok(result) => {
                    self.columns = result.columns;
                    self.results = result.rows;
                    if !self.results.is_empty() {
                        self.table_state.select(Some(0));
                    }
                    self.status_message = Some(format!("CPF0000: {} rows.", self.results.len()));
                }
                Err(error) => {
                    self.error = Some(format!("CPF9898 at position 1: {error}"));
                    self.status_message = self.error.clone();
                }
            }
        }

        self.input.clear();
        self.cursor = 0;
    }

    fn show_tables(&mut self) {
        self.columns = vec![
            "Library".to_string(),
            "Table".to_string(),
            "Text".to_string(),
        ];
        let root = resolve_l400_root();
        let mut rows = Vec::new();
        if let Ok(libraries) = list_libraries(&root) {
            for library in libraries {
                let lib_path = root.join(&library);
                if let Ok(objects) = list_objects(&lib_path) {
                    rows.extend(
                        objects
                            .into_iter()
                            .filter(|object| object.objtype == "*FILE")
                            .map(|object| {
                                vec![
                                    library.clone(),
                                    object.name,
                                    object.text.unwrap_or_default(),
                                ]
                            }),
                    );
                }
            }
        }
        self.results = rows;
        if !self.results.is_empty() {
            self.table_state.select(Some(0));
        }
        self.status_message = Some(format!("CPF0000: {} tables.", self.results.len()));
    }

    fn describe_table(&mut self, table: &str) {
        let snapshot = self.session.snapshot();
        let (library, file) = parse_table_name(table, &snapshot.current_library);
        let path = resolve_l400_root().join(&library).join(&file);
        self.columns = vec![
            "Field".to_string(),
            "Type".to_string(),
            "Length".to_string(),
            "Text".to_string(),
        ];
        match read_pf_schema(&path) {
            Ok(schema) => {
                self.results = schema
                    .fields
                    .into_iter()
                    .map(|field| {
                        vec![
                            field.name,
                            field.type_,
                            field.length.to_string(),
                            field.text.unwrap_or_default(),
                        ]
                    })
                    .collect();
                self.status_message = Some(format!("CPF0000: schema for {library}/{file}."));
            }
            Err(error) => {
                self.results.clear();
                self.error = Some(format!("CPF9898 at position 16: {error}"));
                self.status_message = self.error.clone();
            }
        }
        if !self.results.is_empty() {
            self.table_state.select(Some(0));
        }
    }

    fn autocomplete_table(&mut self) {
        let prefix = self.input[..self.cursor]
            .split_whitespace()
            .last()
            .unwrap_or("")
            .to_uppercase();
        if prefix.is_empty() {
            return;
        }
        let root = resolve_l400_root();
        let snapshot = self.session.snapshot();
        let default_library = self
            .default_library
            .as_deref()
            .unwrap_or(snapshot.current_library.as_str());
        let Ok(objects) = list_objects(&root.join(default_library)) else {
            return;
        };
        if let Some(candidate) = objects
            .into_iter()
            .filter(|object| object.objtype == "*FILE")
            .map(|object| object.name)
            .find(|name| name.starts_with(&prefix))
        {
            let start = self.cursor.saturating_sub(prefix.len());
            self.input.replace_range(start..self.cursor, &candidate);
            self.cursor = start + candidate.len();
            self.status_message = Some(format!("CPF0000: completed {candidate}."));
        }
    }

    fn result_text(&self) -> String {
        let mut lines = Vec::new();
        if !self.columns.is_empty() {
            lines.push(self.columns.join("\t"));
        }
        lines.extend(self.results.iter().map(|row| row.join("\t")));
        lines.join("\n")
    }

    fn copy_result(&mut self) {
        let path = l400::l400_run_dir().join("strsql.clipboard");
        match std::fs::write(&path, self.result_text()) {
            Ok(_) => {
                self.status_message = Some(format!("CPF0000: result copied to {}.", path.display()))
            }
            Err(error) => self.status_message = Some(format!("CPF9898: clipboard error: {error}")),
        }
    }

    fn export_spool(&mut self) {
        let spool_dir = std::env::var_os("L400_SPOOL_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| resolve_l400_root().join("QUSRSYS").join("QSPL"));
        let _ = std::fs::create_dir_all(&spool_dir);
        let path = spool_dir.join(format!("STRSQL_{}.spl", std::process::id()));
        let payload = format!(
            "spool_version=1 status=READY command=STRSQL\n{}\n",
            self.result_text()
        );
        match std::fs::write(&path, payload) {
            Ok(_) => {
                self.status_message =
                    Some(format!("CPF0000: result exported to {}.", path.display()))
            }
            Err(error) => {
                self.status_message = Some(format!("CPF9898: spool export error: {error}"))
            }
        }
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
                self.column_offset = 0;
                ScreenResult::none()
            }
            KeyCode::F(7) => {
                self.column_offset = self.column_offset.saturating_sub(1);
                ScreenResult::none()
            }
            KeyCode::F(8) => {
                if self.column_offset + 1 < self.columns.len() {
                    self.column_offset += 1;
                }
                ScreenResult::none()
            }
            KeyCode::F(18) => {
                self.copy_result();
                ScreenResult::none()
            }
            KeyCode::F(19) => {
                self.export_spool();
                ScreenResult::none()
            }
            KeyCode::Tab => {
                self.autocomplete_table();
                ScreenResult::none()
            }
            KeyCode::Enter => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.input.insert(self.cursor, '\n');
                    self.cursor += 1;
                } else {
                    self.execute();
                }
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

        let hint = self
            .status_message
            .as_deref()
            .unwrap_or("Supported: SELECT [*|KEY|DATA] FROM [LIB/]FILE [WHERE KEY='X']");
        let inner = Rect::new(area.x + 1, area.y + 1, area.width - 2, 1);
        frame.render_widget(
            Paragraph::new(format!("  {hint}")).style(STYLE_NORMAL),
            inner,
        );
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
                Paragraph::new(format!("  {}", error))
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

        let visible_columns = self
            .columns
            .iter()
            .skip(self.column_offset)
            .take(4)
            .cloned()
            .collect::<Vec<_>>();
        let widths = visible_columns
            .iter()
            .map(|_| Constraint::Min(20))
            .collect::<Vec<_>>();
        let rows = self.results.iter().map(|row| {
            Row::new(
                row.iter()
                    .skip(self.column_offset)
                    .take(4)
                    .cloned()
                    .collect::<Vec<_>>(),
            )
        });

        let table = Table::new(rows, widths)
            .header(
                Row::new(visible_columns)
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
            Paragraph::new(Line::from(sql_spans(&format!("SQL> {}", self.input))))
                .style(STYLE_NORMAL),
            inner,
        );

        let cursor_x = inner.x + 5 + self.cursor as u16;
        if cursor_x < inner.x + inner.width {
            frame.set_cursor_position((cursor_x, inner.y));
        }
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        HelpBar::new()
            .command("STRSQL")
            .actions(vec![
                HelpAction::new("F3", "Exit"),
                HelpAction::new("F5", "Clear"),
                HelpAction::new("F7/F8", "Cols"),
                HelpAction::new("F18", "Copy"),
                HelpAction::new("F19", "Spool"),
                HelpAction::new("Tab", "Complete"),
                HelpAction::new("F12", "Cancel"),
                HelpAction::new("Enter", "Run"),
                HelpAction::new("Up/Down", "History"),
            ])
            .render(frame, area);
    }
}

fn parse_context(context: &str) -> (String, String) {
    if let Some((library, file)) = context.split_once('/') {
        (library.trim().to_uppercase(), file.trim().to_uppercase())
    } else {
        ("QSYS".to_string(), context.trim().to_uppercase())
    }
}

fn parse_table_name(table: &str, default_library: &str) -> (String, String) {
    let trimmed = table.trim().trim_end_matches(';').to_uppercase();
    if let Some((library, file)) = trimmed.split_once('/') {
        (library.trim().to_string(), file.trim().to_string())
    } else {
        (default_library.to_uppercase(), trimmed)
    }
}

fn sql_spans(value: &str) -> Vec<Span<'static>> {
    value
        .split_inclusive(' ')
        .map(|word| {
            let bare = word.trim().trim_matches(';').to_uppercase();
            if matches!(
                bare.as_str(),
                "SELECT" | "FROM" | "WHERE" | "SHOW" | "TABLES" | "DESCRIBE" | "TABLE"
            ) {
                Span::styled(word.to_string(), STYLE_TABLE_HEADER)
            } else if bare.starts_with('\'') {
                Span::styled(word.to_string(), STYLE_WARNING)
            } else {
                Span::raw(word.to_string())
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EnvGuard {
        key: &'static str,
        original: Option<std::ffi::OsString>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &std::path::Path) -> Self {
            let original = std::env::var_os(key);
            unsafe {
                std::env::set_var(key, value);
            }
            Self { key, original }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            unsafe {
                if let Some(original) = &self.original {
                    std::env::set_var(self.key, original);
                } else {
                    std::env::remove_var(self.key);
                }
            }
        }
    }

    #[test]
    fn show_tables_lists_real_file_objects_and_spools_results() {
        let temp = tempfile::tempdir().expect("tempdir");
        let _root_guard = EnvGuard::set("L400_ROOT", temp.path());
        let spool = temp.path().join("spool");
        let _spool_guard = EnvGuard::set("L400_SPOOL_DIR", &spool);
        l400::bootstrap_l400_root(temp.path()).expect("bootstrap");
        let qgpl = temp.path().join("QGPL");
        let _ = l400::create_object_with_metadata(
            &qgpl,
            "CUSTOMERS",
            "*FILE",
            Some("PF"),
            Some("Customer table"),
        )
        .expect("create pf");

        let mut sql = StrSql::new();
        sql.input = "SHOW TABLES".to_string();
        sql.cursor = sql.input.len();
        sql.execute();

        assert!(sql.results.iter().any(|row| row[1] == "CUSTOMERS"));
        sql.export_spool();
        assert!(
            spool
                .join(format!("STRSQL_{}.spl", std::process::id()))
                .exists()
        );
    }

    #[test]
    fn parse_table_name_uses_default_library() {
        assert_eq!(
            parse_table_name("customers", "qgpl"),
            ("QGPL".to_string(), "CUSTOMERS".to_string())
        );
    }
}
