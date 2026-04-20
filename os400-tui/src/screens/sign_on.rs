use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::{
    auth::{authenticate_linux_user, DEFAULT_SIGNON_PASSWORD, DEFAULT_SIGNON_USER},
    screens::{Screen, ScreenId, ScreenResult},
    style::*,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActiveField {
    User,
    Password,
}

type AuthValidator = fn(&str, &str) -> Result<(), String>;

pub struct SignOnScreen {
    user: String,
    password: String,
    active_field: ActiveField,
    message: Option<String>,
    validator: AuthValidator,
}

impl SignOnScreen {
    pub fn new() -> Self {
        Self {
            user: DEFAULT_SIGNON_USER.to_string(),
            password: DEFAULT_SIGNON_PASSWORD.to_string(),
            active_field: ActiveField::User,
            message: None,
            validator: authenticate_linux_user,
        }
    }

    #[cfg(test)]
    fn with_validator(validator: AuthValidator) -> Self {
        Self {
            validator,
            ..Self::new()
        }
    }

    fn attempt_sign_on(&mut self) -> ScreenResult {
        let user = self.user.trim().to_uppercase();
        if user.is_empty() {
            self.message = Some("Enter a user profile.".to_string());
            self.active_field = ActiveField::User;
            return ScreenResult::none();
        }

        if user.eq_ignore_ascii_case("ROOT") {
            self.message = Some("Profile ROOT is not available on Linux/400.".to_string());
            self.password.clear();
            self.active_field = ActiveField::User;
            return ScreenResult::none();
        }

        if self.password.is_empty() {
            self.message = Some("Enter a password.".to_string());
            self.active_field = ActiveField::Password;
            return ScreenResult::none();
        }

        match (self.validator)(&user, &self.password) {
            Ok(()) => {
                self.message = None;
                ScreenResult::with_data(ScreenId::MainMenu, user)
            }
            Err(error) => {
                self.message = Some(error);
                self.password.clear();
                self.active_field = ActiveField::Password;
                ScreenResult::none()
            }
        }
    }

    fn move_focus(&mut self) {
        self.active_field = match self.active_field {
            ActiveField::User => ActiveField::Password,
            ActiveField::Password => ActiveField::User,
        };
    }

    fn active_buffer(&mut self) -> &mut String {
        match self.active_field {
            ActiveField::User => &mut self.user,
            ActiveField::Password => &mut self.password,
        }
    }

    fn push_char(&mut self, c: char) {
        if self.active_field == ActiveField::User {
            self.active_buffer().push(c.to_ascii_uppercase());
        } else {
            self.active_buffer().push(c);
        }
    }

    fn masked_password(&self) -> String {
        "*".repeat(self.password.chars().count())
    }

    fn prompt_style(&self, field: ActiveField) -> Style {
        if self.active_field == field {
            STYLE_SELECTION
        } else {
            STYLE_NORMAL
        }
    }
}

impl Screen for SignOnScreen {
    fn render(&mut self, frame: &mut Frame) {
        let area = frame.size();
        frame.render_widget(Block::default().style(STYLE_HEADER), area);

        let overlay = centered_rect(68, 16, area);
        frame.render_widget(Clear, overlay);

        let outer = Block::default()
            .title(" Sign On ")
            .style(STYLE_HEADER)
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);
        frame.render_widget(outer, overlay);

        let inner = Rect::new(
            overlay.x.saturating_add(2),
            overlay.y.saturating_add(2),
            overlay.width.saturating_sub(4),
            overlay.height.saturating_sub(4),
        );

        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(2),
                Constraint::Length(1),
            ])
            .split(inner);

        frame.render_widget(
            Paragraph::new("System . . . . . . . . . . :   Linux/400")
                .style(STYLE_NORMAL)
                .alignment(Alignment::Left),
            sections[0],
        );

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw("User  . . . . . . . . . . :   "),
                Span::styled(
                    pad_field(&self.user, 16),
                    self.prompt_style(ActiveField::User),
                ),
            ]))
            .style(STYLE_NORMAL),
            sections[1],
        );

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw("Password  . . . . . . . . :   "),
                Span::styled(
                    pad_field(&self.masked_password(), 16),
                    self.prompt_style(ActiveField::Password),
                ),
            ]))
            .style(STYLE_NORMAL),
            sections[2],
        );

        frame.render_widget(
            Paragraph::new("Program/procedure . . . . . . :   MENU").style(STYLE_DIM),
            sections[4],
        );

        let message = self
            .message
            .clone()
            .unwrap_or_else(|| "Press Enter to sign on.".to_string());
        let message_style = if self.message.is_some() {
            STYLE_ERROR
        } else {
            STYLE_NORMAL
        };
        frame.render_widget(Paragraph::new(message).style(message_style), sections[5]);

        frame.render_widget(
            Paragraph::new("F3=Exit   Tab=Next field   Enter=Sign on")
                .style(STYLE_HELP)
                .alignment(Alignment::Left)
                .block(
                    Block::default()
                        .borders(Borders::TOP)
                        .border_style(STYLE_BORDER)
                        .style(STYLE_HELP),
                ),
            sections[6],
        );
    }

    fn handle_key(&mut self, key: KeyEvent) -> ScreenResult {
        match key.code {
            KeyCode::F(3) | KeyCode::Esc => ScreenResult::exit(),
            KeyCode::Tab | KeyCode::BackTab | KeyCode::Up | KeyCode::Down => {
                self.move_focus();
                ScreenResult::none()
            }
            KeyCode::Enter => self.attempt_sign_on(),
            KeyCode::Backspace => {
                self.active_buffer().pop();
                self.message = None;
                ScreenResult::none()
            }
            KeyCode::Char(c)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                self.push_char(c);
                self.message = None;
                ScreenResult::none()
            }
            _ => ScreenResult::none(),
        }
    }
}

impl Default for SignOnScreen {
    fn default() -> Self {
        Self::new()
    }
}

fn pad_field(value: &str, width: usize) -> String {
    let mut result = value.chars().take(width).collect::<String>();
    let current = result.chars().count();
    if current < width {
        result.push_str(&" ".repeat(width - current));
    }
    result
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(width.min(area.width)),
            Constraint::Fill(1),
        ])
        .split(area);

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(height.min(area.height)),
            Constraint::Fill(1),
        ])
        .split(horizontal[1]);

    vertical[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::from(code)
    }

    fn ok_validator(_: &str, _: &str) -> Result<(), String> {
        Ok(())
    }

    fn err_validator(_: &str, _: &str) -> Result<(), String> {
        Err("User or password not correct.".to_string())
    }

    #[test]
    fn enter_with_valid_credentials_goes_to_main_menu() {
        let mut screen = SignOnScreen::with_validator(ok_validator);
        let result = screen.handle_key(key(KeyCode::Enter));

        assert_eq!(result.next, Some(ScreenId::MainMenu));
        assert_eq!(result.data.as_deref(), Some("QSECOFR"));
    }

    #[test]
    fn root_profile_is_rejected() {
        let mut screen = SignOnScreen::with_validator(ok_validator);
        screen.user = "ROOT".to_string();
        let result = screen.handle_key(key(KeyCode::Enter));

        assert_eq!(result.next, None);
        assert_eq!(
            screen.message.as_deref(),
            Some("Profile ROOT is not available on Linux/400.")
        );
    }

    #[test]
    fn invalid_credentials_clear_password() {
        let mut screen = SignOnScreen::with_validator(err_validator);
        screen.password = "badpass".to_string();

        let result = screen.handle_key(key(KeyCode::Enter));

        assert_eq!(result.next, None);
        assert!(screen.password.is_empty());
    }
}
