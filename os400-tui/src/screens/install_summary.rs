use std::fs;
use std::path::Path;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

use crate::screens::{Screen, ScreenId, ScreenResult};
use crate::style::{STYLE_HELP, STYLE_NORMAL, STYLE_TITLE};

/// Read the boot mode from /run/l400/boot-mode or /proc/cmdline.
pub fn detect_boot_mode() -> String {
    if let Ok(mode) = fs::read_to_string("/run/l400/boot-mode") {
        return mode.trim().to_string();
    }
    if let Ok(cmdline) = fs::read_to_string("/proc/cmdline") {
        if cmdline.contains("l400.installed=1") {
            return "installed".to_string();
        }
        if cmdline.contains("l400.installed=0") || cmdline.contains("l400.live=1") {
            return "live".to_string();
        }
    }
    "unknown".to_string()
}

/// Read /l400 installation metadata if available.
fn read_install_metadata() -> Vec<(String, String)> {
    let mut items = Vec::new();
    let root = Path::new("/l400");

    if !root.exists() {
        items.push(("L400_ROOT".to_string(), "not found".to_string()));
        return items;
    }

    // Read version file if it exists
    let version_file = root.join("VERSION");
    if let Ok(ver) = fs::read_to_string(&version_file) {
        items.push(("Version".to_string(), ver.trim().to_string()));
    } else {
        items.push(("Version".to_string(), "unknown".to_string()));
    }

    // Read build id if available
    let build_file = root.join("BUILD_ID");
    if let Ok(build) = fs::read_to_string(&build_file) {
        items.push(("Build ID".to_string(), build.trim().to_string()));
    }

    // Check metadata version via xattr
    if let Ok(Some(meta)) = l400::storage::read_string_attr(root, "user.l400.version") {
        items.push(("Metadata Version".to_string(), meta));
    }

    // Check platform profile
    if let Ok(Some(profile)) = l400::storage::read_string_attr(root, "user.l400.profile") {
        items.push(("Platform Profile".to_string(), profile));
    }

    items.push(("Root Path".to_string(), root.display().to_string()));
    items
}

pub struct InstallSummary {
    boot_mode: String,
    install_metadata: Vec<(String, String)>,
}

impl Default for InstallSummary {
    fn default() -> Self {
        Self::new()
    }
}

impl InstallSummary {
    pub fn new() -> Self {
        Self {
            boot_mode: detect_boot_mode(),
            install_metadata: read_install_metadata(),
        }
    }
}

impl Screen for InstallSummary {
    fn render(&mut self, frame: &mut Frame) {
        let area = crate::screens::screen_area(frame);

        // Clear the area first
        frame.render_widget(Clear, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
            ])
            .split(area);

        // Title
        let title = Paragraph::new(Text::from(vec![
            Line::from(vec![Span::styled(
                "Linux/400 Installation Summary",
                STYLE_TITLE,
            )]),
            Line::from(vec![Span::styled(
                format!("Boot Mode: {}", self.boot_mode),
                STYLE_TITLE,
            )]),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(STYLE_TITLE),
        );
        frame.render_widget(title, chunks[0]);

        // Installation details
        let items: Vec<ListItem> = self
            .install_metadata
            .iter()
            .map(|(k, v)| ListItem::new(format!("{}: {}", k, v)))
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Installation Details")
                    .borders(Borders::ALL),
            )
            .style(STYLE_NORMAL);
        frame.render_widget(list, chunks[1]);

        // Help text
        let help = Paragraph::new("Press any key to continue to sign-on screen")
            .style(STYLE_HELP)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(help, chunks[2]);
    }

    fn handle_key(&mut self, _key: crossterm::event::KeyEvent) -> ScreenResult {
        // Any key press goes to sign-on screen
        ScreenResult::goto(ScreenId::SignOn)
    }
}
