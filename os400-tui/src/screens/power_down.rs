use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph},
};
use std::process::Command;

use crate::screens::{Screen, ScreenResult};
use crate::session::SessionContext;
use crate::style::*;
use crate::widgets::confirm_dialog::ConfirmDialog;

pub struct PowerDownSystem {
    session: SessionContext,
    confirm: Option<ConfirmDialog>,
    message: String,
    option: String,
    confirm_param: String,
}

impl PowerDownSystem {
    pub fn with_params(session: SessionContext, option: String, confirm_param: String) -> Self {
        Self {
            session,
            confirm: if confirm_param == "*YES" {
                None
            } else {
                Some(ConfirmDialog::new(
                    "PWRDWNSYS",
                    "Confirm power down of Linux/400?",
                ))
            },
            message: "Enter=Confirm   F12=Cancel".to_string(),
            option,
            confirm_param,
        }
    }
}

impl PowerDownSystem {
    pub fn new(session: SessionContext) -> Self {
        Self {
            session,
            confirm: Some(ConfirmDialog::new(
                "PWRDWNSYS",
                "Confirm power down of Linux/400?",
            )),
            message: "Enter=Confirm   F12=Cancel".to_string(),
            option: "POWEROFF".to_string(),
            confirm_param: "*NO".to_string(),
        }
    }

    fn execute(&mut self) {
        let state = self.session.snapshot();
        if state.user_profile != "QSECOFR" {
            self.message = "CPF2204: User lacks *ALLOBJ authority.".to_string();
            self.session.set_last_message(&self.message);
            return;
        }

        if std::env::var("L400_PWRDWNSYS_DRY_RUN").as_deref() == Ok("1") {
            self.message = "CPF0000: PWRDWNSYS dry-run completed.".to_string();
            self.session.set_last_message(&self.message);
            return;
        }

        let system_command = match self.option.as_str() {
            "*RESTART" => "reboot",
            _ => "poweroff",
        };

        match Command::new(system_command).output() {
            Ok(output) if output.status.success() => {
                self.message = format!("CPF0000: {} completed.", system_command);
            }
            Ok(output) => {
                self.message = format!(
                    "CPF9999: {} failed with status {}.",
                    system_command,
                    output.status.code().unwrap_or_default()
                );
            }
            Err(error) => {
                self.message = format!("CPF9999: {} unavailable: {}", system_command, error);
            }
        }
        self.session.set_last_message(&self.message);
    }
}

impl Screen for PowerDownSystem {
    fn render(&mut self, frame: &mut Frame) {
        let area = crate::screens::screen_area(frame);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(area);

        let block = Block::default()
            .title(" PWRDWNSYS ")
            .style(STYLE_HEADER)
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);
        frame.render_widget(block, chunks[0]);
        frame.render_widget(
            Paragraph::new("Power down Linux/400")
                .style(STYLE_NORMAL)
                .alignment(Alignment::Center),
            chunks[1],
        );
        frame.render_widget(
            Paragraph::new(self.message.clone())
                .style(STYLE_WARNING)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(STYLE_BORDER),
                ),
            chunks[2],
        );

        if let Some(confirm) = &self.confirm {
            confirm.render(frame, area);
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> ScreenResult {
        if let Some(confirm) = &mut self.confirm {
            confirm.handle_key(key);
            match confirm.result() {
                Some(true) => {
                    self.confirm = None;
                    self.execute();
                    return ScreenResult::none();
                }
                Some(false) => return ScreenResult::back(),
                None => return ScreenResult::none(),
            }
        }

        match key.code {
            KeyCode::F(3) | KeyCode::F(12) | KeyCode::Esc => ScreenResult::back(),
            _ => ScreenResult::none(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    struct EnvGuard {
        key: &'static str,
        original: Option<OsString>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let original = std::env::var_os(key);
            unsafe {
                std::env::set_var(key, value);
            }
            Self { key, original }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.original {
                Some(value) => unsafe {
                    std::env::set_var(self.key, value);
                },
                None => unsafe {
                    std::env::remove_var(self.key);
                },
            }
        }
    }

    #[test]
    fn f12_cancels_with_back_navigation() {
        let mut screen = PowerDownSystem::new(SessionContext::new(920));
        let result = screen.handle_key(KeyEvent::from(KeyCode::F(12)));
        assert_eq!(result.next, Some(crate::screens::ScreenId::Back));
    }

    #[test]
    fn enter_executes_dry_run_after_confirmation() {
        let _guard = EnvGuard::set("L400_PWRDWNSYS_DRY_RUN", "1");
        let mut screen = PowerDownSystem::new(SessionContext::new(921));

        let result = screen.handle_key(KeyEvent::from(KeyCode::Enter));

        assert_eq!(result.next, None);
        assert!(screen.message.contains("dry-run completed"));
    }
}
