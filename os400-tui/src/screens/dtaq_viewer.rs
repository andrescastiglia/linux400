use crossterm::event::{KeyCode, KeyEvent};
use l400::{resolve_l400_root, DataQueue};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::Line,
    text::Text,
    widgets::{Block, Borders, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::screens::{Screen, ScreenId, ScreenResult};
use crate::style::*;

pub struct DtaqMessage {
    pub key: String,
    pub data: String,
    pub timestamp: String,
}

pub struct DataQueueViewer {
    current_library: String,
    current_dtaq: String,
    messages: Vec<DtaqMessage>,
    state: TableState,
    using_runtime_data: bool,
}

impl DataQueueViewer {
    pub fn new() -> Self {
        let current_library = "QUSRSYS".to_string();
        let current_dtaq = "QEZJOBLOG".to_string();
        let (messages, using_runtime_data) = Self::load_messages(&current_library, &current_dtaq);
        Self {
            current_library,
            current_dtaq,
            messages,
            state: TableState::default(),
            using_runtime_data,
        }
    }

    fn fallback_messages() -> Vec<DtaqMessage> {
        vec![
            DtaqMessage {
                key: "00001".to_string(),
                data: "Job started at 08:00:00".to_string(),
                timestamp: "08:00:00".to_string(),
            },
            DtaqMessage {
                key: "00002".to_string(),
                data: "Processing batch job BATCH001".to_string(),
                timestamp: "08:01:23".to_string(),
            },
            DtaqMessage {
                key: "00003".to_string(),
                data: "File opened: CUSTMAST".to_string(),
                timestamp: "08:02:45".to_string(),
            },
            DtaqMessage {
                key: "00004".to_string(),
                data: "Record count: 1500".to_string(),
                timestamp: "08:03:12".to_string(),
            },
            DtaqMessage {
                key: "00005".to_string(),
                data: "Batch job completed successfully".to_string(),
                timestamp: "08:05:00".to_string(),
            },
        ]
    }

    fn load_messages(library: &str, dtaq: &str) -> (Vec<DtaqMessage>, bool) {
        let path = resolve_l400_root().join(library).join(dtaq);
        if let Ok(queue) = DataQueue::open(&path) {
            if let Ok(messages) = queue.read_all() {
                let mapped = messages
                    .into_iter()
                    .map(|(id, data)| DtaqMessage {
                        key: format!("{id:05}"),
                        data: String::from_utf8_lossy(&data).to_string(),
                        timestamp: "runtime".to_string(),
                    })
                    .collect::<Vec<_>>();
                return (mapped, true);
            }
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
        } else if self.state.selected().is_none() {
            self.state.select(Some(0));
        }
    }
}

impl Screen for DataQueueViewer {
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
        self.render_messages(frame, chunks[1]);
        self.render_help(frame, chunks[2]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> ScreenResult {
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
            _ => ScreenResult::none(),
        }
    }
}

impl DataQueueViewer {
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let title = Line::from(vec![format!(
            " Data Queue Viewer  DTAQ: {}/{} ",
            self.current_library, self.current_dtaq
        )
        .into()]);

        let block = Block::default()
            .title(title)
            .style(STYLE_HEADER)
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);

        frame.render_widget(block, area);

        let source_label = if self.using_runtime_data {
            "Runtime queue"
        } else {
            "Bundled sample"
        };
        let lines: Vec<Line> = vec![
            Line::from(vec![format!("Source: {}. Type options, press Enter.", source_label).into()]),
            Line::from(vec!["Opt  Key      Data".into()]),
        ];
        let text = Text::from(lines);

        let inner = Rect::new(area.x + 1, area.y + 1, area.width - 2, 2);
        frame.render_widget(Paragraph::new(text).style(STYLE_NORMAL), inner);
    }

    fn render_messages(&mut self, frame: &mut Frame, area: Rect) {
        let header = ["", "Key", "Timestamp", "Data"];
        let widths = [4u16, 8, 12, 50];

        let rows: Vec<Row> = self
            .messages
            .iter()
            .map(|msg| {
                Row::new(vec![
                    " ".to_string(),
                    msg.key.clone(),
                    msg.timestamp.clone(),
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
            .highlight_style(STYLE_SELECTION);

        frame.render_stateful_widget(table, area, &mut self.state);
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let help_text = Line::from(vec![
            "F3=Exit   ".into(),
            "F4=Prompt   ".into(),
            "F5=Refresh   ".into(),
            "F12=Cancel   ".into(),
            "Enter=Display".into(),
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

impl Default for DataQueueViewer {
    fn default() -> Self {
        Self::new()
    }
}
