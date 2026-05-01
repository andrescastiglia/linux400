use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::{
    auth::{DEFAULT_SIGNON_PASSWORD, DEFAULT_SIGNON_USER, authenticate_linux_user},
    screens::{Screen, ScreenId, ScreenResult},
    style::*,
    widgets::input_field::InputField,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActiveField {
    User,
    Password,
    CurrentLibrary,
    InitialMenu,
}

type AuthValidator = fn(&str, &str) -> Result<(), String>;

pub struct SignOnScreen {
    user: String,
    password: String,
    current_library: String,
    initial_menu: String,
    active_field: ActiveField,
    message: Option<String>,
    validator: AuthValidator,
}

impl SignOnScreen {
    pub fn new() -> Self {
        Self {
            user: DEFAULT_SIGNON_USER.to_string(),
            password: DEFAULT_SIGNON_PASSWORD.to_string(),
            current_library: "QGPL".to_string(),
            initial_menu: "MAIN".to_string(),
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
            ActiveField::Password => ActiveField::CurrentLibrary,
            ActiveField::CurrentLibrary => ActiveField::InitialMenu,
            ActiveField::InitialMenu => ActiveField::User,
        };
    }

    fn active_buffer(&mut self) -> &mut String {
        match self.active_field {
            ActiveField::User => &mut self.user,
            ActiveField::Password => &mut self.password,
            ActiveField::CurrentLibrary => &mut self.current_library,
            ActiveField::InitialMenu => &mut self.initial_menu,
        }
    }

    fn push_char(&mut self, c: char) {
        if self.active_field != ActiveField::Password {
            self.active_buffer().push(c.to_ascii_uppercase());
        } else {
            self.active_buffer().push(c);
        }
    }
}

impl Screen for SignOnScreen {
    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();
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

        let fields = [
            (
                ActiveField::User,
                InputField::new("User  . . . . . . . . . .", 16)
                    .with_value(&self.user)
                    .uppercase()
                    .required(),
            ),
            (
                ActiveField::Password,
                InputField::new("Password  . . . . . . . .", 16)
                    .with_value(&self.password)
                    .masked()
                    .required(),
            ),
            (
                ActiveField::CurrentLibrary,
                InputField::new("Current library . . . . .", 16)
                    .with_value(&self.current_library)
                    .uppercase()
                    .required(),
            ),
            (
                ActiveField::InitialMenu,
                InputField::new("Initial menu . . . . . .", 16)
                    .with_value(&self.initial_menu)
                    .uppercase()
                    .required(),
            ),
        ];

        for (offset, (field_id, mut field)) in fields.into_iter().enumerate() {
            field.active = self.active_field == field_id;
            field.render(frame, sections[1 + offset]);
        }

        frame.render_widget(
            Paragraph::new("System mode shown in global status line.").style(STYLE_DIM),
            sections[5],
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
        frame.render_widget(Paragraph::new(message).style(message_style), sections[7]);

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
            sections[8],
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
