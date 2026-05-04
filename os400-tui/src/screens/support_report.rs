use std::path::Path;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

use crate::screens::{Screen, ScreenResult};
use crate::style::{STYLE_HELP, STYLE_NORMAL, STYLE_TITLE};
use l400::read_loader_status;
use l400_ebpf_common::L400_POLICY_VERSION;

/// Phase 9: Support report showing effective mode, BTF, kernel, cgroups, xattrs
pub struct SupportReport {
    items: Vec<(String, String)>,
    status_message: Option<String>,
}

impl SupportReport {
    pub fn new() -> Self {
        let mut report = Self {
            items: Vec::new(),
            status_message: None,
        };
        report.refresh();
        report
    }

    fn refresh(&mut self) {
        self.items.clear();

        // Read loader status for platform information
        let loader = read_loader_status().ok();

        // Effective mode
        let effective_mode = loader
            .as_ref()
            .and_then(|status| status.effective_mode.clone())
            .unwrap_or_else(|| "unknown".to_string());
        self.items
            .push(("Effective Mode".to_string(), effective_mode));

        // Protection active
        let protection = loader
            .as_ref()
            .map(|status| {
                if status.protection_active {
                    "active"
                } else {
                    "inactive"
                }
            })
            .unwrap_or("unknown");
        self.items
            .push(("Kernel Enforcement".to_string(), protection.to_string()));

        // BTF availability
        let btf = loader
            .as_ref()
            .and_then(|status| status.btf_available)
            .map(|b| if b { "available" } else { "unavailable" })
            .unwrap_or("unknown");
        self.items.push(("BTF".to_string(), btf.to_string()));

        // Kernel version
        let kernel = loader
            .as_ref()
            .and_then(|status| status.kernel_version.clone())
            .unwrap_or_else(|| "unknown".to_string());
        self.items.push(("Kernel Version".to_string(), kernel));

        // Cgroups v2
        let cgroups = loader
            .as_ref()
            .and_then(|status| status.cgroups_v2)
            .map(|c| if c { "v2 available" } else { "not available" }.to_string())
            .unwrap_or("unknown".to_string());
        self.items.push(("Cgroups".to_string(), cgroups));

        // Xattrs support
        let xattrs = loader
            .as_ref()
            .and_then(|status| status.xattrs_supported)
            .map(|x| if x { "supported" } else { "not supported" }.to_string())
            .unwrap_or("unknown".to_string());
        self.items.push(("Xattrs".to_string(), xattrs));

        // Policy version
        let policy_ver = loader
            .as_ref()
            .and_then(|status| status.policy_version.clone())
            .unwrap_or_else(|| L400_POLICY_VERSION.to_string());
        self.items
            .push(("eBPF Policy Version".to_string(), policy_ver));

        // Runtime version
        let runtime_ver = loader
            .as_ref()
            .and_then(|status| status.runtime_version.clone())
            .unwrap_or_else(|| "unknown".to_string());
        self.items
            .push(("Runtime Version".to_string(), runtime_ver));

        // eBPF version
        let ebpf_ver = loader
            .as_ref()
            .and_then(|status| status.ebpf_version.clone())
            .unwrap_or_else(|| "unknown".to_string());
        self.items
            .push(("eBPF Loader Version".to_string(), ebpf_ver));

        // Attached hooks
        let hooks = loader
            .as_ref()
            .and_then(|status| status.attached_hooks.clone())
            .unwrap_or_else(|| "none".to_string());
        self.items.push(("Attached Hooks".to_string(), hooks));

        // Known gaps
        let gaps = loader
            .as_ref()
            .and_then(|status| status.known_gaps.clone())
            .unwrap_or_else(|| "none reported".to_string());
        self.items.push(("Known Gaps".to_string(), gaps));

        // Last error
        let last_error = loader
            .as_ref()
            .and_then(|status| status.last_error.clone())
            .unwrap_or_else(|| "none".to_string());
        self.items.push(("Last Error".to_string(), last_error));

        // /l400 metadata
        self.add_l400_metadata();

        self.status_message = Some("F5=Refresh".to_string());
    }

    fn add_l400_metadata(&mut self) {
        let root = Path::new("/l400");

        // Check if /l400 exists
        if !root.exists() {
            self.items
                .push(("L400 Root".to_string(), "not found".to_string()));
            return;
        }

        self.items
            .push(("L400 Root".to_string(), "found".to_string()));

        // Read metadata version via xattr
        if let Ok(Some(meta)) = l400::storage::read_string_attr(root, "user.l400.version") {
            self.items.push(("Metadata Version".to_string(), meta));
        }

        // Check platform profile
        if let Ok(Some(profile)) = l400::storage::read_string_attr(root, "user.l400.profile") {
            self.items.push(("Platform Profile".to_string(), profile));
        }

        // Check storage backend
        if let Ok(Some(backend)) = l400::storage::read_string_attr(root, "user.l400.storage") {
            self.items.push(("Storage Backend".to_string(), backend));
        }
    }
}

impl Default for SupportReport {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for SupportReport {
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
            Line::from(vec![Span::styled("Linux/400 Support Report", STYLE_TITLE)]),
            Line::from(vec![Span::styled(
                "Phase 9: Platform Profiles and Kernel Security",
                STYLE_TITLE,
            )]),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(STYLE_TITLE),
        );
        frame.render_widget(title, chunks[0]);

        // Support details
        let items: Vec<ListItem> = self
            .items
            .iter()
            .map(|(k, v)| ListItem::new(format!("{:<25}: {}", k, v)))
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("System Information")
                    .borders(Borders::ALL),
            )
            .style(STYLE_NORMAL);
        frame.render_widget(list, chunks[1]);

        // Help text
        let help = Paragraph::new(self.status_message.clone().unwrap_or_default())
            .style(STYLE_HELP)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(help, chunks[2]);
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> ScreenResult {
        match key.code {
            crossterm::event::KeyCode::F(3)
            | crossterm::event::KeyCode::F(12)
            | crossterm::event::KeyCode::Esc => ScreenResult::back(),
            crossterm::event::KeyCode::F(5) => {
                self.refresh();
                ScreenResult::none()
            }
            _ => ScreenResult::none(),
        }
    }
}
