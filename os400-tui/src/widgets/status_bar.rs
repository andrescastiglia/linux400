use l400::read_loader_status;
use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::session::SessionContext;
use crate::style::*;

/// A global status bar displayed at the bottom of every screen.
///
/// Shows system name, user, current library, job ID, enforcement mode,
/// and the last CPF message. Modeled after the 5250 status line.
pub struct StatusBar {
    session: SessionContext,
}

impl StatusBar {
    pub fn new(session: SessionContext) -> Self {
        Self { session }
    }

    /// Render the status bar into a single-line area.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let state = self.session.snapshot();

        let mode = current_mode();
        let mode_style = match mode {
            SystemMode::Full => STYLE_MODE_FULL,
            SystemMode::Degraded => STYLE_MODE_DEGRADED,
            SystemMode::Dev => STYLE_MODE_DEV,
        };
        let mode_label = match mode {
            SystemMode::Full => "FULL",
            SystemMode::Degraded => "DEGRADED",
            SystemMode::Dev => "DEV",
        };

        let message = state.last_message.clone().unwrap_or_default();

        // Truncate message to fit available width.
        let fixed_width = 60; // approximate fixed portion
        let max_msg = (area.width as usize).saturating_sub(fixed_width);
        let message_display = if message.chars().count() > max_msg {
            message.chars().take(max_msg).collect::<String>() + "..."
        } else {
            message.clone()
        };

        let spans = vec![
            Span::styled("L400", STYLE_STATUS_BAR),
            Span::styled("  ", STYLE_STATUS_BAR),
            Span::styled(format!("User:{:<8}", state.user_profile), STYLE_STATUS_BAR),
            Span::styled("  ", STYLE_STATUS_BAR),
            Span::styled(
                format!("Lib:{:<8}", state.current_library),
                STYLE_STATUS_BAR,
            ),
            Span::styled("  ", STYLE_STATUS_BAR),
            Span::styled(format!("Job:{:<6}", state.job_id), STYLE_STATUS_BAR),
            Span::styled("  ", STYLE_STATUS_BAR),
            Span::styled(format!("[{}]", mode_label), mode_style),
            Span::styled("  ", STYLE_STATUS_BAR),
            Span::styled(message_display, STYLE_STATUS_BAR),
        ];

        let line = Line::from(spans);
        frame.render_widget(Paragraph::new(line).style(STYLE_STATUS_BAR), area);
    }
}

/// Current enforcement mode of the system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemMode {
    Full,
    Degraded,
    Dev,
}

/// Read the current system mode from loader status or environment.
pub fn current_mode() -> SystemMode {
    match read_loader_status() {
        Ok(status) => {
            if status.protection_active {
                SystemMode::Full
            } else {
                match status.mode.to_lowercase().as_str() {
                    "degraded" => SystemMode::Degraded,
                    "dev" => SystemMode::Dev,
                    _ => SystemMode::Degraded,
                }
            }
        }
        Err(_) => SystemMode::Dev,
    }
}

/// A badge that displays the enforcement mode with semantic color.
pub struct ModeIndicator;

impl ModeIndicator {
    /// Render a compact mode indicator.
    pub fn render(frame: &mut Frame, area: Rect) {
        let mode = current_mode();
        let (label, style) = match mode {
            SystemMode::Full => (" FULL ", STYLE_MODE_FULL),
            SystemMode::Degraded => (" DEGRADED ", STYLE_MODE_DEGRADED),
            SystemMode::Dev => (" DEV ", STYLE_MODE_DEV),
        };
        frame.render_widget(Paragraph::new(label).style(style), area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_mode_returns_dev_when_loader_unavailable() {
        // In test environment, loader status file doesn't exist.
        assert_eq!(current_mode(), SystemMode::Dev);
    }
}
