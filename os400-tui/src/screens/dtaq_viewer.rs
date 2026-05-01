use crossterm::event::{KeyCode, KeyEvent};
use l400::{DataQueue, resolve_l400_root};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::Line,
    text::Text,
    widgets::{Block, Borders, Paragraph, Row, Table, TableState},
};
use std::time::{Duration, Instant};

use crate::screens::{Screen, ScreenId, ScreenResult};
use crate::style::*;

pub struct DtaqMessage {
    pub key: String,
    pub data: String,
    pub timestamp: String,
    pub length: usize,
}

pub struct DataQueueViewer {
    current_library: String,
    current_dtaq: String,
    messages: Vec<DtaqMessage>,
    state: TableState,
    using_runtime_data: bool,
    status_message: Option<String>,
    auto_refresh: bool,
    last_refresh: Instant,
    send_mode: bool,
    send_buffer: String,
}

impl DataQueueViewer {
    pub fn new() -> Self {
        Self::from_library_dtaq("QUSRSYS", "QEZJOBLOG")
    }

    pub fn from_spec(spec: &str) -> Self {
        if let Some((library, dtaq)) = spec.trim().split_once('/') {
            Self::from_library_dtaq(library, dtaq)
        } else {
            Self::from_library_dtaq("QUSRSYS", spec)
        }
    }

    fn from_library_dtaq(library: &str, dtaq: &str) -> Self {
        let current_library = library.trim().to_uppercase();
        let current_dtaq = dtaq.trim().to_uppercase();
        let (messages, using_runtime_data) = Self::load_messages(&current_library, &current_dtaq);
        let status_message = messages
            .is_empty()
            .then(|| "Sin mensajes runtime para esta DTAQ.".to_string());
        Self {
            current_library,
            current_dtaq,
            messages,
            state: TableState::default(),
            using_runtime_data,
            status_message,
            auto_refresh: false,
            last_refresh: Instant::now(),
            send_mode: false,
            send_buffer: String::new(),
        }
    }

    fn fallback_messages() -> Vec<DtaqMessage> {
        Vec::new()
    }

    fn load_messages(library: &str, dtaq: &str) -> (Vec<DtaqMessage>, bool) {
        let path = resolve_l400_root().join(library).join(dtaq);
        if let Ok(queue) = DataQueue::open(&path)
            && let Ok(messages) = queue.read_all()
        {
            let mapped = messages
                .into_iter()
                .map(|(id, data)| DtaqMessage {
                    key: format!("{id:05}"),
                    data: preview_bytes(&data),
                    timestamp: format!("seq:{id}"),
                    length: data.len(),
                })
                .collect::<Vec<_>>();
            return (mapped, true);
        }

        (Self::fallback_messages(), false)
    }

    fn refresh(&mut self) {
        let (messages, using_runtime_data) =
            Self::load_messages(&self.current_library, &self.current_dtaq);
        self.messages = messages;
        self.using_runtime_data = using_runtime_data;
        if self.messages.is_empty() {
            self.state.select(None);
            self.status_message = Some("Sin mensajes runtime para esta DTAQ.".to_string());
        } else if self.state.selected().is_none() {
            self.state.select(Some(0));
            self.status_message = None;
        }
        self.last_refresh = Instant::now();
    }

    fn show_selected(&mut self) {
        self.status_message = self
            .state
            .selected()
            .and_then(|index| self.messages.get(index))
            .map(|message| format!("{}: {}", message.key, message.data))
            .or_else(|| Some("No hay mensaje seleccionado.".to_string()));
    }
}

impl Screen for DataQueueViewer {
    fn render(&mut self, frame: &mut Frame) {
        if self.auto_refresh && self.last_refresh.elapsed() >= Duration::from_secs(5) {
            self.refresh();
        }
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),
                Constraint::Min(0),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(crate::screens::screen_area(frame));

        self.render_header(frame, chunks[0]);
        self.render_messages(frame, chunks[1]);
        self.render_status(frame, chunks[2]);
        self.render_help(frame, chunks[3]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> ScreenResult {
        if self.send_mode {
            return match key.code {
                KeyCode::Enter => {
                    self.send_message();
                    ScreenResult::none()
                }
                KeyCode::Esc | KeyCode::F(12) => {
                    self.send_mode = false;
                    self.send_buffer.clear();
                    ScreenResult::none()
                }
                KeyCode::Backspace => {
                    self.send_buffer.pop();
                    ScreenResult::none()
                }
                KeyCode::Char(c) => {
                    self.send_buffer.push(c);
                    ScreenResult::none()
                }
                _ => ScreenResult::none(),
            };
        }
        match key.code {
            KeyCode::F(3) => ScreenResult::goto(ScreenId::MainMenu),
            KeyCode::F(4) => ScreenResult::goto(ScreenId::CommandLine),
            KeyCode::F(12) => ScreenResult::goto(ScreenId::MainMenu),
            KeyCode::Up => {
                self.state
                    .select(Some(self.state.selected().unwrap_or(0).saturating_sub(1)));
                ScreenResult::none()
            }
            KeyCode::Down => {
                let max = self.messages.len().saturating_sub(1);
                let current = self.state.selected().unwrap_or(0);
                self.state.select(Some(current.saturating_add(1).min(max)));
                ScreenResult::none()
            }
            KeyCode::F(5) => {
                self.refresh();
                ScreenResult::none()
            }
            KeyCode::F(6) => {
                self.send_mode = true;
                self.send_buffer.clear();
                ScreenResult::none()
            }
            KeyCode::F(7) => {
                self.receive_message();
                ScreenResult::none()
            }
            KeyCode::F(21) => {
                self.auto_refresh = !self.auto_refresh;
                self.status_message = Some(format!(
                    "Auto-refresh {}.",
                    if self.auto_refresh { "on" } else { "off" }
                ));
                ScreenResult::none()
            }
            KeyCode::Enter | KeyCode::Char('5') => {
                self.show_selected();
                ScreenResult::none()
            }
            _ => ScreenResult::none(),
        }
    }
}

impl DataQueueViewer {
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let title = Line::from(vec![
            format!(
                " Data Queue Viewer  DTAQ: {}/{} ",
                self.current_library, self.current_dtaq
            )
            .into(),
        ]);

        let block = Block::default()
            .title(title)
            .style(STYLE_HEADER)
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);

        frame.render_widget(block, area);

        let source_label = if self.using_runtime_data {
            "Runtime queue"
        } else {
            "Sin catalogo"
        };
        let lines: Vec<Line> = vec![
            Line::from(vec![
                format!("Source: {}. Options: 5=Display message.", source_label).into(),
            ]),
            Line::from(vec!["Opt  Key      Data".into()]),
        ];
        let text = Text::from(lines);

        let inner = Rect::new(area.x + 1, area.y + 1, area.width - 2, 2);
        frame.render_widget(Paragraph::new(text).style(STYLE_NORMAL), inner);
    }

    fn render_messages(&mut self, frame: &mut Frame, area: Rect) {
        let header = ["", "Key", "Timestamp", "Len", "Preview"];
        let widths = [4u16, 8, 16, 8, 50];

        let rows: Vec<Row> = self
            .messages
            .iter()
            .map(|msg| {
                Row::new(vec![
                    " ".to_string(),
                    msg.key.clone(),
                    msg.timestamp.clone(),
                    msg.length.to_string(),
                    msg.data.clone(),
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
            "F6=Send   ".into(),
            "F7=Receive   ".into(),
            "F21=Auto   ".into(),
            "F12=Cancel   ".into(),
            "5/Enter=Display".into(),
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
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(
            Paragraph::new(if self.send_mode {
                format!("SNDDTAQ MSG: {}", self.send_buffer)
            } else {
                self.status_message.clone().unwrap_or_default()
            })
            .style(STYLE_NORMAL),
            inner,
        );
    }

    fn send_message(&mut self) {
        let path = resolve_l400_root()
            .join(&self.current_library)
            .join(&self.current_dtaq);
        match DataQueue::open(&path).and_then(|queue| queue.snddtaq(self.send_buffer.as_bytes())) {
            Ok(_) => {
                self.status_message = Some("Message sent.".to_string());
                self.send_mode = false;
                self.send_buffer.clear();
                self.refresh();
            }
            Err(error) => self.status_message = Some(format!("Error sending message: {error}")),
        }
    }

    fn receive_message(&mut self) {
        let path = resolve_l400_root()
            .join(&self.current_library)
            .join(&self.current_dtaq);
        match DataQueue::open(&path).and_then(|queue| queue.rcvdtaq(0)) {
            Ok(message) => {
                self.status_message = Some(format!(
                    "Received {} bytes: {}",
                    message.len(),
                    preview_bytes(&message)
                ));
                self.refresh();
            }
            Err(error) => self.status_message = Some(format!("Error receiving message: {error}")),
        }
    }
}

fn preview_bytes(data: &[u8]) -> String {
    let text = String::from_utf8_lossy(data).to_string();
    if text.chars().count() > 64 {
        format!("{}...", text.chars().take(64).collect::<String>())
    } else {
        text
    }
}

impl Default for DataQueueViewer {
    fn default() -> Self {
        Self::new()
    }
}
