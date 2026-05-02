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
    fields: Vec<InputField>,
    active_field: ActiveField,
    message: Option<String>,
    validator: AuthValidator,
}

impl SignOnScreen {
    pub fn new() -> Self {
        Self {
            fields: vec![
                InputField::new("User  . . . . . . . . . .", 16)
                    .with_value(DEFAULT_SIGNON_USER)
                    .uppercase()
                    .required(),
                InputField::new("Password  . . . . . . . .", 16)
                    .with_value(DEFAULT_SIGNON_PASSWORD)
                    .masked()
                    .required(),
                InputField::new("Current library . . . . .", 16)
                    .with_value("QGPL")
                    .uppercase()
                    .required(),
                InputField::new("Initial menu . . . . . .", 16)
                    .with_value("MAIN")
                    .uppercase()
                    .required(),
            ],
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
        let user = self.fields[0].value.trim().to_uppercase();
        let password = &self.fields[1].value;

        if user.is_empty() {
            self.message = Some("Enter a user profile.".to_string());
            self.active_field = ActiveField::User;
            return ScreenResult::none();
        }

        if user.eq_ignore_ascii_case("ROOT") {
            self.message = Some("Profile ROOT is not available on Linux/400.".to_string());
            self.fields[1].clear();
            self.active_field = ActiveField::User;
            return ScreenResult::none();
        }

        if password.is_empty() {
            self.message = Some("Enter a password.".to_string());
            self.active_field = ActiveField::Password;
            return ScreenResult::none();
        }

        match (self.validator)(&user, password) {
            Ok(()) => {
                self.message = None;
                ScreenResult::with_data(ScreenId::MainMenu, user)
            }
            Err(error) => {
                self.message = Some(error);
                self.fields[1].clear();
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

    fn active_field_mut(&mut self) -> &mut InputField {
        match self.active_field {
            ActiveField::User => &mut self.fields[0],
            ActiveField::Password => &mut self.fields[1],
            ActiveField::CurrentLibrary => &mut self.fields[2],
            ActiveField::InitialMenu => &mut self.fields[3],
        }
    }
}

impl Screen for SignOnScreen {
    fn render(&mut self, frame: &mut Frame) {
        let area = crate::screens::screen_area(frame);
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
                Constraint::Length(2), // System info
                Constraint::Length(1), // Ruler line (top)
                Constraint::Length(1), // User field
                Constraint::Length(1), // Password field
                Constraint::Length(1), // Current library field
                Constraint::Length(1), // Initial menu field
                Constraint::Length(1), // Ruler line (bottom)
                Constraint::Length(1), // Status line
                Constraint::Min(2),    // Message area
                Constraint::Length(1), // Help line
            ])
            .split(inner);

        frame.render_widget(
            Paragraph::new("System . . . . . . . . . . :   Linux/400")
                .style(STYLE_NORMAL)
                .alignment(Alignment::Left),
            sections[0],
        );

        // Top ruler line
        render_ruler(frame, sections[1]);

        for (offset, field_id) in [
            ActiveField::User,
            ActiveField::Password,
            ActiveField::CurrentLibrary,
            ActiveField::InitialMenu,
        ]
        .iter()
        .enumerate()
        {
            let field = &mut self.fields[offset];
            field.active = *field_id == self.active_field;
            field.render(frame, sections[2 + offset]);
        }

        // Bottom ruler line
        render_ruler(frame, sections[6]);

        frame.render_widget(
            Paragraph::new("System mode shown in global status line.").style(STYLE_DIM),
            sections[7],
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
        frame.render_widget(Paragraph::new(message).style(message_style), sections[8]);

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
            sections[9],
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
                self.active_field_mut().delete_back();
                self.message = None;
                ScreenResult::none()
            }
            KeyCode::Delete => {
                self.active_field_mut().delete_forward();
                self.message = None;
                ScreenResult::none()
            }
            KeyCode::Left => {
                self.active_field_mut().move_left();
                ScreenResult::none()
            }
            KeyCode::Right => {
                self.active_field_mut().move_right();
                ScreenResult::none()
            }
            KeyCode::Home => {
                self.active_field_mut().move_home();
                ScreenResult::none()
            }
            KeyCode::End => {
                self.active_field_mut().move_end();
                ScreenResult::none()
            }
            KeyCode::Char(c)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                self.active_field_mut().insert_char(c);
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
        screen.fields[0].value = "ROOT".to_string();
        screen.fields[0].cursor = screen.fields[0].value.len();
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
        screen.fields[1].value = "badpass".to_string();
        screen.fields[1].cursor = screen.fields[1].value.len();

        let result = screen.handle_key(key(KeyCode::Enter));

        assert_eq!(result.next, None);
        assert!(screen.fields[1].value.is_empty());
    }

    #[test]
    fn backspace_deletes_char_in_active_field() {
        let mut screen = SignOnScreen::new();
        screen.fields[0].value = "QSECOFR".to_string();
        screen.fields[0].cursor = screen.fields[0].value.len();
        screen.active_field = ActiveField::User;

        screen.handle_key(key(KeyCode::Backspace));
        assert_eq!(screen.fields[0].value, "QSECOF");
    }

    #[test]
    fn delete_forward_removes_char() {
        let mut screen = SignOnScreen::new();
        screen.fields[0].value = "QSECOFR".to_string();
        screen.fields[0].cursor = 3;
        screen.active_field = ActiveField::User;

        screen.handle_key(key(KeyCode::Delete));
        assert_eq!(screen.fields[0].value, "QSEOFR");
    }

    #[test]
    fn arrow_keys_move_cursor() {
        let mut screen = SignOnScreen::new();
        screen.fields[0].value = "QSECOFR".to_string();
        screen.fields[0].cursor = 4;
        screen.active_field = ActiveField::User;

        screen.handle_key(key(KeyCode::Left));
        assert_eq!(screen.fields[0].cursor, 3);

        screen.handle_key(key(KeyCode::Right));
        assert_eq!(screen.fields[0].cursor, 4);
    }

    #[test]
    fn home_end_keys_work() {
        let mut screen = SignOnScreen::new();
        screen.fields[0].value = "QSECOFR".to_string();
        screen.fields[0].cursor = 4;
        screen.active_field = ActiveField::User;

        screen.handle_key(key(KeyCode::Home));
        assert_eq!(screen.fields[0].cursor, 0);

        screen.handle_key(key(KeyCode::End));
        assert_eq!(screen.fields[0].cursor, 7);
    }
}
