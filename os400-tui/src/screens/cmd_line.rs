use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::Line,
    text::Text,
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::process::Command;

use crate::screens::{Screen, ScreenId, ScreenResult};
use crate::session::SessionContext;
use crate::style::*;

#[derive(Clone, Debug)]
struct PromptField {
    name: &'static str,
    value: String,
    required: bool,
}

pub struct CommandLine {
    command: String,
    history: Vec<String>,
    history_index: usize,
    cursor_position: usize,
    output: Vec<String>,
    show_output: bool,
    session: SessionContext,
    prompt_command: Option<String>,
    prompt_fields: Vec<PromptField>,
    prompt_index: usize,
    prompt_cursor: usize,
    prompt_error: Option<String>,
}

impl CommandLine {
    pub fn new() -> Self {
        Self::with_session(SessionContext::new(std::process::id() as u64))
    }

    pub fn with_session(session: SessionContext) -> Self {
        Self {
            command: String::new(),
            history: vec![
                "WRKACTJOB".to_string(),
                "WRKOBJ".to_string(),
                "DSPDTAQ QUSRSYS QEZJOBLOG".to_string(),
            ],
            history_index: 0,
            cursor_position: 0,
            output: Vec::new(),
            show_output: false,
            session,
            prompt_command: None,
            prompt_fields: Vec::new(),
            prompt_index: 0,
            prompt_cursor: 0,
            prompt_error: None,
        }
    }

    fn execute_command(&mut self) -> ScreenResult {
        let cmd = self.command.trim().to_string();
        if cmd.is_empty() {
            return ScreenResult::none();
        }

        if let Some(route) = self.route_interactive_command(&cmd) {
            if !self.history.iter().any(|h| h == &cmd) {
                self.history.insert(0, cmd.clone());
            }
            self.command.clear();
            self.cursor_position = 0;
            self.history_index = 0;
            return route;
        }

        self.output.clear();
        self.output.push(format!("CMD: {}", cmd));
        self.output.push("".to_string());

        if self.handle_session_command(&cmd) {
            if !self.history.iter().any(|h| h == &cmd) {
                self.history.insert(0, cmd.clone());
            }
            self.show_output = true;
            self.command.clear();
            self.cursor_position = 0;
            self.history_index = 0;
            return ScreenResult::none();
        }

        match cmd.as_str() {
            "WRKACTJOB" => {
                return ScreenResult::goto(ScreenId::WorkManagement);
            }
            "WRKOBJ" | "WRKLIB" => {
                return ScreenResult::goto(ScreenId::ObjectBrowser);
            }
            cmd if cmd.starts_with("DSPDTAQ") => {
                let tokens = tokenize_cl_command(cmd);
                let dtaq = extract_command_arg(&tokens[1..], "DTAQ")
                    .or_else(|| tokens.get(1).map(|value| value.to_string()))
                    .unwrap_or_else(|| "QUSRSYS/QEZJOBLOG".to_string());
                return ScreenResult::with_data(ScreenId::DataQueueViewer, dtaq);
            }
            "HELP" => {
                self.output.push("Available commands:".to_string());
                self.output
                    .push("  WRKACTJOB - Work with active jobs".to_string());
                self.output
                    .push("  WRKOBJ    - Work with objects".to_string());
                self.output
                    .push("  DSPDTAQ   - Display data queue".to_string());
                self.output
                    .push("  STRPDM    - Programming Development Manager".to_string());
                self.output
                    .push("  WRKMBRPDM - Work with source members".to_string());
                self.output
                    .push("  STRSEU    - Edit a source member".to_string());
                self.output
                    .push("  STRSQL    - Interactive SQL".to_string());
                self.output.push("  CALL PGM   - Call program".to_string());
                self.output
                    .push("  DSPSYSVAL - Display system value".to_string());
            }
            _ => {
                let state = self.session.snapshot();
                match Command::new("l400cmd")
                    .args(tokenize_cl_command(&cmd))
                    .env("L400_USER", &state.user_profile)
                    .env("L400_CURLIB", &state.current_library)
                    .env("L400_LIBLIST", state.library_list.join(":"))
                    .output()
                {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        for line in stdout.lines().chain(stderr.lines()) {
                            self.output.push(line.to_string());
                        }
                        if self.output.len() == 2 {
                            self.output.push(format!(
                                "Command '{}' completed with status {}",
                                cmd,
                                output.status.code().unwrap_or_default()
                            ));
                        }
                    }
                    Err(error) => {
                        self.output
                            .push(format!("Command '{}' could not run: {}", cmd, error));
                    }
                }
            }
        }

        if !self.history.iter().any(|h| h == &cmd) {
            self.history.insert(0, cmd.clone());
        }

        self.show_output = true;
        self.command.clear();
        self.cursor_position = 0;
        self.history_index = 0;
        ScreenResult::none()
    }

    fn start_prompt(&mut self) {
        let action = self
            .command
            .split_whitespace()
            .next()
            .map(str::to_uppercase)
            .unwrap_or_else(|| "WRKOBJ".to_string());
        let fields = if let Some(metadata) = l400::command_metadata(&action) {
            metadata
                .parameters
                .iter()
                .map(|parameter| PromptField {
                    name: parameter.name,
                    value: parameter.default.to_string(),
                    required: parameter.required,
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let fields = if !fields.is_empty() {
            fields
        } else {
            match action.as_str() {
                "WRKOBJ" | "WRKLIB" => vec![
                    PromptField {
                        name: "OBJ",
                        value: "QGPL/*ALL".to_string(),
                        required: true,
                    },
                    PromptField {
                        name: "OBJTYPE",
                        value: "*ALL".to_string(),
                        required: false,
                    },
                ],
                "DLTOBJ" => vec![
                    PromptField {
                        name: "OBJ",
                        value: "QGPL/OBJECT".to_string(),
                        required: true,
                    },
                    PromptField {
                        name: "OBJTYPE",
                        value: "*ALL".to_string(),
                        required: false,
                    },
                    PromptField {
                        name: "CONFIRM",
                        value: "*YES".to_string(),
                        required: true,
                    },
                ],
                "DSPPFM" | "CLRPFM" | "ADDPFM" | "WRTPFM" => vec![
                    PromptField {
                        name: "FILE",
                        value: "QGPL/CUSTOMERS".to_string(),
                        required: true,
                    },
                    PromptField {
                        name: "MBR",
                        value: "*FIRST".to_string(),
                        required: false,
                    },
                ],
                "DSPDTAQ" | "SNDDTAQ" | "RCVDTAQ" | "CRTDTAQ" => vec![PromptField {
                    name: "DTAQ",
                    value: "QUSRSYS/QEZJOBLOG".to_string(),
                    required: true,
                }],
                "STRSEU" | "WRKMBRPDM" => vec![
                    PromptField {
                        name: "FILE",
                        value: "QGPL/QCLSRC".to_string(),
                        required: true,
                    },
                    PromptField {
                        name: "MBR",
                        value: "HELLO.CLP".to_string(),
                        required: action == "STRSEU",
                    },
                ],
                "CRTCLPGM" => vec![
                    PromptField {
                        name: "PGM",
                        value: "QGPL/HELLO".to_string(),
                        required: true,
                    },
                    PromptField {
                        name: "SRCFILE",
                        value: "QGPL/QCLSRC".to_string(),
                        required: true,
                    },
                    PromptField {
                        name: "SRCMBR",
                        value: "HELLO.CLP".to_string(),
                        required: true,
                    },
                ],
                "CALL" => vec![PromptField {
                    name: "PGM",
                    value: "QGPL/HELLO".to_string(),
                    required: true,
                }],
                _ => vec![PromptField {
                    name: "OBJ",
                    value: "QGPL/*ALL".to_string(),
                    required: true,
                }],
            }
        };

        self.prompt_command = Some(action);
        self.prompt_fields = fields;
        self.prompt_index = 0;
        self.prompt_cursor = self
            .prompt_fields
            .first()
            .map(|field| field.value.len())
            .unwrap_or_default();
        self.prompt_error = None;
    }

    fn finish_prompt(&mut self) -> bool {
        if let Some(field) = self
            .prompt_fields
            .iter()
            .find(|field| field.required && field.value.trim().is_empty())
        {
            self.prompt_error = Some(format!("{} es requerido.", field.name));
            return false;
        }
        let command = self
            .prompt_command
            .clone()
            .unwrap_or_else(|| "WRKOBJ".to_string());
        let params = self
            .prompt_fields
            .iter()
            .filter(|field| !field.value.trim().is_empty())
            .map(|field| format!("{}({})", field.name, field.value.trim()))
            .collect::<Vec<_>>()
            .join(" ");
        self.command = format!("{command} {params}");
        self.cursor_position = self.command.len();
        self.prompt_command = None;
        self.prompt_fields.clear();
        self.prompt_error = None;
        true
    }

    fn handle_prompt_key(&mut self, key: KeyEvent) -> ScreenResult {
        match key.code {
            KeyCode::F(12) | KeyCode::Esc => {
                self.prompt_command = None;
                self.prompt_fields.clear();
                self.prompt_error = None;
            }
            KeyCode::Tab | KeyCode::Down if !self.prompt_fields.is_empty() => {
                self.prompt_index = (self.prompt_index + 1) % self.prompt_fields.len();
                self.prompt_cursor = self.prompt_fields[self.prompt_index].value.len();
            }
            KeyCode::BackTab | KeyCode::Up if !self.prompt_fields.is_empty() => {
                self.prompt_index = if self.prompt_index == 0 {
                    self.prompt_fields.len() - 1
                } else {
                    self.prompt_index - 1
                };
                self.prompt_cursor = self.prompt_fields[self.prompt_index].value.len();
            }
            KeyCode::Enter if self.finish_prompt() => {
                return self.execute_command();
            }
            KeyCode::Backspace => {
                if let Some(field) = self.prompt_fields.get_mut(self.prompt_index) {
                    if self.prompt_cursor > 0 {
                        self.prompt_cursor -= 1;
                        field.value.remove(self.prompt_cursor);
                    }
                }
            }
            KeyCode::Delete => {
                if let Some(field) = self.prompt_fields.get_mut(self.prompt_index) {
                    if self.prompt_cursor < field.value.len() {
                        field.value.remove(self.prompt_cursor);
                    }
                }
            }
            KeyCode::Left => {
                self.prompt_cursor = self.prompt_cursor.saturating_sub(1);
            }
            KeyCode::Right => {
                if let Some(field) = self.prompt_fields.get(self.prompt_index) {
                    self.prompt_cursor =
                        self.prompt_cursor.saturating_add(1).min(field.value.len());
                }
            }
            KeyCode::Char(c) => {
                if let Some(field) = self.prompt_fields.get_mut(self.prompt_index) {
                    field.value.insert(self.prompt_cursor, c);
                    self.prompt_cursor += 1;
                }
            }
            _ => {}
        }
        ScreenResult::none()
    }

    fn route_interactive_command(&mut self, command: &str) -> Option<ScreenResult> {
        let tokens = tokenize_cl_command(command);
        let action = tokens.first()?.to_uppercase();

        match action.as_str() {
            "GO" => {
                let target = tokens
                    .get(1)
                    .map(|value| value.trim().to_uppercase())
                    .unwrap_or_default();
                if target == "MAIN" {
                    Some(ScreenResult::goto(ScreenId::MainMenu))
                } else {
                    self.show_usage_error("Usage: GO MAIN");
                    Some(ScreenResult::none())
                }
            }
            "SIGNOFF" => Some(ScreenResult::goto(ScreenId::SignOn)),
            "STRPDM" => Some(ScreenResult::goto(ScreenId::PdmBrowser)),
            "STRSQL" => Some(ScreenResult::goto(ScreenId::StrSql)),
            "WRKACTJOB" => Some(ScreenResult::goto(ScreenId::WorkManagement)),
            "WRKOBJ" | "WRKLIB" => Some(ScreenResult::goto(ScreenId::ObjectBrowser)),
            "WRKUSRPRF" => Some(ScreenResult::goto(ScreenId::UserProfiles)),
            "DSPPOLICY" | "DSPAUD" => Some(ScreenResult::with_data(ScreenId::PolicyAudit, command)),
            "DSPCMD" | "WRKCMD" => Some(ScreenResult::with_data(ScreenId::SystemPanel, command)),
            "WRKSPLF" | "WRKOUTQ" => Some(ScreenResult::with_data(ScreenId::SpoolOutq, command)),
            "WRKSYSSTS" | "WRKSYSVAL" => {
                Some(ScreenResult::with_data(ScreenId::SystemPanel, command))
            }
            "DSPOBJD" | "DSPOBJAUT" => {
                Some(ScreenResult::with_data(ScreenId::ObjectDetail, command))
            }
            "WRKMBRPDM" => {
                let file = extract_command_arg(&tokens[1..], "FILE").or_else(|| {
                    tokens
                        .get(1)
                        .filter(|token| !token.contains('('))
                        .map(|value| value.to_string())
                });
                match file.filter(|value| !value.trim().is_empty()) {
                    Some(file) => Some(ScreenResult::with_data(
                        ScreenId::WrkMbrPdm,
                        normalize_file_spec(&file, &self.session),
                    )),
                    None => {
                        self.show_usage_error("Usage: WRKMBRPDM FILE(QGPL/QCLSRC)");
                        Some(ScreenResult::none())
                    }
                }
            }
            "STRSEU" => {
                let file = extract_command_arg(&tokens[1..], "FILE").or_else(|| {
                    tokens
                        .get(1)
                        .filter(|token| !token.contains('('))
                        .map(|value| value.to_string())
                });
                let member = extract_command_arg(&tokens[1..], "MBR").or_else(|| {
                    tokens
                        .get(2)
                        .filter(|token| !token.contains('('))
                        .map(|value| value.to_string())
                });
                match (file, member) {
                    (Some(file), Some(member)) => Some(ScreenResult::with_data(
                        ScreenId::StrSeu,
                        format!(
                            "{}/{}",
                            normalize_file_spec(&file, &self.session),
                            member.trim().to_uppercase()
                        ),
                    )),
                    _ => {
                        self.show_usage_error("Usage: STRSEU FILE(QGPL/QCLSRC) MBR(HELLO.CLP)");
                        Some(ScreenResult::none())
                    }
                }
            }
            _ => None,
        }
    }

    fn show_usage_error(&mut self, message: &str) {
        self.output.clear();
        self.output.push(message.to_string());
        self.show_output = true;
    }

    fn handle_session_command(&mut self, command: &str) -> bool {
        let tokens = tokenize_cl_command(command);
        let Some(action) = tokens.first().map(|value| value.to_uppercase()) else {
            return false;
        };
        match action.as_str() {
            "CHGCURLIB" => {
                if let Some(library) = extract_command_arg(&tokens[1..], "CURLIB")
                    .or_else(|| extract_command_arg(&tokens[1..], "LIB"))
                    .or_else(|| tokens.get(1).map(|value| value.to_string()))
                {
                    self.session.set_current_library(&library);
                    self.output.push(format!(
                        "Current library changed to {}",
                        library.to_uppercase()
                    ));
                } else {
                    self.output.push("Usage: CHGCURLIB LIB(QGPL)".to_string());
                }
                true
            }
            "ADDLIBLE" => {
                if let Some(library) = extract_command_arg(&tokens[1..], "LIB")
                    .or_else(|| tokens.get(1).map(|value| value.to_string()))
                {
                    self.session.add_library(&library);
                    self.output
                        .push(format!("{} added to library list", library.to_uppercase()));
                } else {
                    self.output.push("Usage: ADDLIBLE LIB(QGPL)".to_string());
                }
                true
            }
            "DSPLIBL" => {
                let state = self.session.snapshot();
                self.output
                    .push(format!("Current library: {}", state.current_library));
                self.output.push("Library list:".to_string());
                for library in state.library_list {
                    self.output.push(format!("  {library}"));
                }
                true
            }
            _ => false,
        }
    }
}

impl Screen for CommandLine {
    fn render(&mut self, frame: &mut Frame) {
        if self.prompt_command.is_some() {
            self.render_prompt(frame);
            return;
        }
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(frame.area());

        self.render_command_line(frame, chunks[0]);
        if self.show_output {
            self.render_output(frame, chunks[1]);
        }
        self.render_help(frame, chunks[2]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> ScreenResult {
        if self.prompt_command.is_some() {
            return self.handle_prompt_key(key);
        }
        if self.show_output {
            match key.code {
                KeyCode::F(3) => return ScreenResult::goto(ScreenId::MainMenu),
                KeyCode::Enter | KeyCode::Esc => {
                    self.show_output = false;
                    return ScreenResult::none();
                }
                _ => return ScreenResult::none(),
            }
        }

        match key.code {
            KeyCode::F(3) => ScreenResult::goto(ScreenId::MainMenu),
            KeyCode::F(4) => {
                self.start_prompt();
                ScreenResult::none()
            }
            KeyCode::F(12) => ScreenResult::goto(ScreenId::MainMenu),
            KeyCode::Enter => self.execute_command(),
            KeyCode::Backspace => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                    self.command.remove(self.cursor_position);
                }
                ScreenResult::none()
            }
            KeyCode::Delete => {
                if self.cursor_position < self.command.len() {
                    self.command.remove(self.cursor_position);
                }
                ScreenResult::none()
            }
            KeyCode::Left => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
                ScreenResult::none()
            }
            KeyCode::Right => {
                if self.cursor_position < self.command.len() {
                    self.cursor_position += 1;
                }
                ScreenResult::none()
            }
            KeyCode::Home => {
                self.cursor_position = 0;
                ScreenResult::none()
            }
            KeyCode::End => {
                self.cursor_position = self.command.len();
                ScreenResult::none()
            }
            KeyCode::Up => {
                if self.history_index < self.history.len().saturating_sub(1) {
                    self.history_index += 1;
                    self.command = self.history[self.history_index].clone();
                    self.cursor_position = self.command.len();
                }
                ScreenResult::none()
            }
            KeyCode::Down => {
                if self.history_index > 0 {
                    self.history_index -= 1;
                    self.command = self.history[self.history_index].clone();
                    self.cursor_position = self.command.len();
                } else {
                    self.history_index = 0;
                    self.command.clear();
                    self.cursor_position = 0;
                }
                ScreenResult::none()
            }
            KeyCode::Char(c) => {
                self.command.insert(self.cursor_position, c);
                self.cursor_position += 1;
                ScreenResult::none()
            }
            _ => ScreenResult::none(),
        }
    }
}

fn extract_command_arg(tokens: &[String], key: &str) -> Option<String> {
    tokens.iter().find_map(|token| {
        let token = token.trim();
        if !token.to_uppercase().starts_with(&format!("{key}(")) || !token.ends_with(')') {
            return None;
        }
        Some(token[key.len() + 1..token.len() - 1].trim().to_string())
    })
}

fn tokenize_cl_command(command: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    let mut in_single = false;
    let mut in_double = false;

    for ch in command.chars() {
        match ch {
            '\'' if !in_double => {
                in_single = !in_single;
                current.push(ch);
            }
            '"' if !in_single => {
                in_double = !in_double;
                current.push(ch);
            }
            '(' if !in_single && !in_double => {
                depth += 1;
                current.push(ch);
            }
            ')' if !in_single && !in_double => {
                depth = depth.saturating_sub(1);
                current.push(ch);
            }
            ch if ch.is_whitespace() && depth == 0 && !in_single && !in_double => {
                if !current.trim().is_empty() {
                    tokens.push(current.trim().to_string());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }

    if !current.trim().is_empty() {
        tokens.push(current.trim().to_string());
    }

    tokens
}

fn normalize_file_spec(spec: &str, session: &SessionContext) -> String {
    let spec = spec.trim();
    if let Some((library, file)) = spec.split_once('/') {
        format!(
            "{}/{}",
            library.trim().to_uppercase(),
            file.trim().to_uppercase()
        )
    } else {
        format!(
            "{}/{}",
            session.snapshot().current_library,
            spec.to_uppercase()
        )
    }
}

impl CommandLine {
    fn render_prompt(&self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(frame.area());

        let command = self.prompt_command.as_deref().unwrap_or("WRKOBJ");
        let block = Block::default()
            .title(format!(" Prompt {} ", command))
            .style(STYLE_HEADER)
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);
        let inner = block.inner(chunks[0]);
        frame.render_widget(block, chunks[0]);
        frame.render_widget(
            Paragraph::new(
                self.prompt_error
                    .clone()
                    .unwrap_or_else(|| "Tab/Shift-Tab cambia de campo. Enter ejecuta.".to_string()),
            )
            .style(STYLE_NORMAL),
            inner,
        );

        let lines = self
            .prompt_fields
            .iter()
            .enumerate()
            .map(|(index, field)| {
                let marker = if index == self.prompt_index { ">" } else { " " };
                Line::from(format!(
                    "{} {:<10} {}{}",
                    marker,
                    field.name,
                    field.value,
                    if field.required { "  *REQ" } else { "" }
                ))
            })
            .collect::<Vec<_>>();
        let field_block = Block::default()
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);
        let field_inner = field_block.inner(chunks[1]);
        frame.render_widget(field_block, chunks[1]);
        frame.render_widget(
            Paragraph::new(Text::from(lines)).style(STYLE_NORMAL),
            field_inner,
        );

        let help_text = Line::from(vec![
            "Enter=Run   ".into(),
            "Tab=Next   ".into(),
            "Shift-Tab=Prev   ".into(),
            "F12=Cancel".into(),
        ]);
        let help = Block::default()
            .style(STYLE_HELP)
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);
        let help_inner = Rect::new(chunks[2].x + 1, chunks[2].y + 1, chunks[2].width - 2, 1);
        frame.render_widget(help, chunks[2]);
        frame.render_widget(Paragraph::new(help_text).style(STYLE_HELP), help_inner);

        let cursor_y = field_inner.y + self.prompt_index as u16;
        let cursor_x = field_inner.x + 13 + self.prompt_cursor as u16;
        if cursor_y < field_inner.y + field_inner.height
            && cursor_x < field_inner.x + field_inner.width
        {
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }

    fn render_command_line(&self, frame: &mut Frame, area: Rect) {
        let display = format!("> {}", self.command);

        let block = Block::default()
            .style(STYLE_HEADER)
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);

        frame.render_widget(block, area);

        let text = Paragraph::new(display.as_str()).style(STYLE_NORMAL);
        let inner = Rect::new(area.x + 1, area.y, area.width - 2, 1);
        frame.render_widget(text, inner);

        let cursor_x = self.cursor_position + 2;
        if cursor_x < area.width as usize - 1 {
            frame.set_cursor_position((area.x + cursor_x as u16, area.y));
        }
    }

    fn render_output(&self, frame: &mut Frame, area: Rect) {
        let text: Text = self
            .output
            .iter()
            .map(|line| Line::from(line.clone()))
            .collect();

        let block = Block::default()
            .title(" Command Output ")
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);

        frame.render_widget(block, area);

        let inner = Rect::new(area.x + 1, area.y + 1, area.width - 2, area.height - 2);
        frame.render_widget(Paragraph::new(text).style(STYLE_NORMAL), inner);
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let help_text = Line::from(vec![
            "F3=Exit   ".into(),
            "F4=Prompt   ".into(),
            "F12=Cancel   ".into(),
            "Enter=Execute   ".into(),
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

impl Default for CommandLine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signoff_returns_to_signon_screen() {
        let mut cmd = CommandLine::new();
        cmd.command = "SIGNOFF".to_string();

        let result = cmd.execute_command();

        assert_eq!(result.next, Some(ScreenId::SignOn));
    }

    #[test]
    fn f4_prompt_builds_field_based_command() {
        let mut cmd = CommandLine::new();
        cmd.command = "CALL".to_string();
        cmd.start_prompt();

        assert_eq!(cmd.prompt_command.as_deref(), Some("CALL"));
        assert_eq!(cmd.prompt_fields[0].name, "PGM");

        cmd.prompt_fields[0].value = "QGPL/HELLO".to_string();
        assert!(cmd.finish_prompt());
        assert_eq!(cmd.command, "CALL PGM(QGPL/HELLO)");
    }

    #[test]
    fn tokenizer_preserves_keyword_values_with_spaces() {
        let tokens = tokenize_cl_command("CHGOBJD OBJ(QGPL/DEMO) TEXT('Demo object')");

        assert_eq!(
            tokens,
            vec![
                "CHGOBJD".to_string(),
                "OBJ(QGPL/DEMO)".to_string(),
                "TEXT('Demo object')".to_string()
            ]
        );
        assert_eq!(
            extract_command_arg(&tokens[1..], "TEXT").as_deref(),
            Some("'Demo object'")
        );
    }
}
