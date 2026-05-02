use crossterm::event::{KeyCode, KeyEvent};
use l400::auth::{
    L400Authority, get_object_authorities, grant_object_authority, revoke_object_authority,
};
use l400::{describe_object, read_string_attr, resolve_l400_root};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
};
use std::path::PathBuf;

use crate::cl_parser::{extract_command_arg, tokenize_cl_command};
use crate::screens::{Screen, ScreenResult};
use crate::session::SessionContext;
use crate::style::*;
use crate::widgets::help_bar::{CpfMessage, HelpAction, HelpBar};
use crate::widgets::subfile_table::SubfileTable;

#[derive(Clone, Debug, PartialEq, Eq)]
struct AuthRow {
    user: String,
    authority: String,
    origin: String,
}

pub struct ObjectAuthority {
    object_spec: String,
    object_path: PathBuf,
    table: SubfileTable,
    rows: Vec<AuthRow>,
    status: String,
    grant_prompt: bool,
    grant_buffer: String,
}

impl ObjectAuthority {
    pub fn new(data: Option<&str>, session: SessionContext) -> Self {
        let object_spec = data
            .and_then(extract_object_spec)
            .unwrap_or_else(|| format!("{}/{}", session.snapshot().current_library, "*ALL"));
        let object_path = object_path(&object_spec, &session);
        let mut screen = Self {
            object_spec,
            object_path,
            table: SubfileTable::new(
                vec!["Opt", "User", "Authority", "Origin"],
                vec![4, 18, 14, 14],
            )
            .with_title("Object authorities"),
            rows: Vec::new(),
            status: String::new(),
            grant_prompt: false,
            grant_buffer: String::new(),
        };
        screen.refresh();
        screen
    }

    fn refresh(&mut self) {
        self.rows.clear();
        match get_object_authorities(&self.object_path) {
            Ok(auths) => {
                let manifest =
                    read_string_attr(&self.object_path, l400::auth::L400_AUTH_MANIFEST_ATTR)
                        .ok()
                        .flatten()
                        .unwrap_or_default();
                self.rows = auths
                    .into_iter()
                    .map(|(user, authority)| AuthRow {
                        origin: manifest_origin(&manifest, &user),
                        user,
                        authority: authority.to_string(),
                    })
                    .collect();
                if let Ok(object) = describe_object(&self.object_path)
                    && let Some(owner) = object.owner
                    && !owner.is_empty()
                    && !self.rows.iter().any(|row| row.user == owner)
                {
                    self.rows.push(AuthRow {
                        user: owner,
                        authority: "*ALL".to_string(),
                        origin: "owner".to_string(),
                    });
                }
                self.rows.sort_by(|left, right| left.user.cmp(&right.user));
                self.sync_table();
                self.status = format!("{} authority entries loaded.", self.rows.len());
            }
            Err(error) => {
                self.rows.clear();
                self.sync_table();
                self.status = format!("Error loading authorities: {error}");
            }
        }
    }

    fn sync_table(&mut self) {
        self.table.set_rows(
            self.rows
                .iter()
                .map(|row| {
                    vec![
                        " ".to_string(),
                        row.user.clone(),
                        row.authority.clone(),
                        row.origin.clone(),
                    ]
                })
                .collect(),
        );
    }

    fn selected_user(&self) -> Option<&str> {
        self.table
            .selected()
            .and_then(|index| self.rows.get(index))
            .map(|row| row.user.as_str())
    }

    fn finish_grant(&mut self) {
        let input = self.grant_buffer.trim().to_uppercase();
        self.grant_prompt = false;
        self.grant_buffer.clear();
        let mut parts = input.split_whitespace();
        let Some(user) = parts.next() else {
            self.status = "Grant requires USER and optional AUT.".to_string();
            return;
        };
        let authority = parts.next().unwrap_or("*USE");
        let Ok(authority) = authority.parse::<L400Authority>() else {
            self.status = format!("Invalid authority {authority}.");
            return;
        };
        match grant_object_authority(&self.object_path, user, authority) {
            Ok(_) => {
                self.status = format!("{authority} granted to {user}.");
                self.refresh();
            }
            Err(error) => self.status = format!("Error granting authority: {error}"),
        }
    }

    fn revoke_selected(&mut self) {
        let Some(user) = self.selected_user().map(str::to_string) else {
            self.status = "No authority row selected.".to_string();
            return;
        };
        if user.starts_with('*')
            || self
                .rows
                .iter()
                .any(|row| row.user == user && row.origin == "owner")
        {
            self.status = format!("{user} is not revocable from this option.");
            return;
        }
        match revoke_object_authority(&self.object_path, &user) {
            Ok(_) => {
                self.status = format!("Authority revoked for {user}.");
                self.refresh();
            }
            Err(error) => self.status = format!("Error revoking authority: {error}"),
        }
    }
}

impl Screen for ObjectAuthority {
    fn render(&mut self, frame: &mut Frame) {
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
        self.table.render(frame, chunks[1]);
        self.render_status(frame, chunks[2]);
        self.render_help(frame, chunks[3]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> ScreenResult {
        if self.grant_prompt {
            return match key.code {
                KeyCode::Enter => {
                    self.finish_grant();
                    ScreenResult::none()
                }
                KeyCode::F(12) | KeyCode::Esc => {
                    self.grant_prompt = false;
                    self.grant_buffer.clear();
                    ScreenResult::none()
                }
                KeyCode::Backspace => {
                    self.grant_buffer.pop();
                    ScreenResult::none()
                }
                KeyCode::Char(c) => {
                    self.grant_buffer.push(c.to_ascii_uppercase());
                    ScreenResult::none()
                }
                _ => ScreenResult::none(),
            };
        }

        match key.code {
            KeyCode::F(3) | KeyCode::F(12) | KeyCode::Esc => ScreenResult::back(),
            KeyCode::F(5) => {
                self.refresh();
                ScreenResult::none()
            }
            KeyCode::Up => {
                self.table.select_prev();
                ScreenResult::none()
            }
            KeyCode::Down => {
                self.table.select_next();
                ScreenResult::none()
            }
            KeyCode::PageUp => {
                self.table.page_up();
                ScreenResult::none()
            }
            KeyCode::PageDown => {
                self.table.page_down();
                ScreenResult::none()
            }
            KeyCode::Char('1') => {
                self.grant_prompt = true;
                self.grant_buffer.clear();
                ScreenResult::none()
            }
            KeyCode::Char('4') => {
                self.revoke_selected();
                ScreenResult::none()
            }
            _ => ScreenResult::none(),
        }
    }
}

impl ObjectAuthority {
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let line = if self.grant_prompt {
            format!("Grant USER AUT: {}", self.grant_buffer)
        } else {
            "Options: 1=Grant 4=Revoke".to_string()
        };
        frame.render_widget(
            Paragraph::new(line).style(STYLE_NORMAL).block(
                Block::default()
                    .title(format!(" DSPOBJAUT {} ", self.object_spec))
                    .style(STYLE_HEADER)
                    .borders(Borders::ALL)
                    .border_style(STYLE_BORDER),
            ),
            area,
        );
    }

    fn render_status(&self, frame: &mut Frame, area: Rect) {
        let cpf = if self.status.to_ascii_lowercase().contains("error")
            || self.status.to_ascii_lowercase().contains("invalid")
        {
            CpfMessage::error("CPF9898", self.status.clone())
        } else {
            CpfMessage::info("CPF0000", self.status.clone())
        };
        cpf.render(frame, area);
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        HelpBar::new()
            .command("DSPOBJAUT")
            .actions(vec![
                HelpAction::new("F3", "Back"),
                HelpAction::new("F5", "Refresh"),
                HelpAction::new("1", "Grant"),
                HelpAction::new("4", "Revoke"),
                HelpAction::new("F12", "Cancel"),
            ])
            .render(frame, area);
    }
}

fn extract_object_spec(command: &str) -> Option<String> {
    let tokens = tokenize_cl_command(command);
    extract_command_arg(&tokens[1..], "OBJ")
        .or_else(|| tokens.get(1).cloned())
        .map(|value| value.trim().to_uppercase())
}

fn object_path(spec: &str, session: &SessionContext) -> PathBuf {
    let root = resolve_l400_root();
    if let Some((library, object)) = spec.split_once('/') {
        root.join(library.trim()).join(object.trim())
    } else {
        root.join(session.snapshot().current_library)
            .join(spec.trim())
    }
}

fn manifest_origin(manifest: &str, user: &str) -> String {
    let prefix = format!("{user}:");
    manifest
        .split("entries=")
        .nth(1)
        .and_then(|entries| entries.split(";flat=").next())
        .and_then(|entries| {
            entries
                .split(',')
                .find(|entry| entry.starts_with(&prefix))
                .and_then(|entry| entry.rsplit(':').next())
        })
        .unwrap_or_else(|| {
            if user == "*PUBLIC" {
                "public"
            } else {
                "explicit"
            }
        })
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_origin_reads_explicit_entry_origin() {
        let manifest = "version=2;entries=*PUBLIC:-:*USE:public,QPGMR:1000:*ALL:explicit;flat=*PUBLIC:*USE,QPGMR:*ALL";
        assert_eq!(manifest_origin(manifest, "QPGMR"), "explicit");
        assert_eq!(manifest_origin(manifest, "*PUBLIC"), "public");
    }

    #[test]
    fn grant_and_revoke_updates_real_authority_matrix() {
        let root = tempfile::tempdir().expect("tempdir");
        l400::bootstrap_l400_root(root.path()).expect("bootstrap");
        let object_path = l400::create_object_with_metadata(
            &root.path().join("QGPL"),
            "AUTHOBJ",
            "*PGM",
            Some("CL"),
            Some("hello"),
        )
        .expect("object");

        let mut screen = ObjectAuthority {
            object_spec: "QGPL/AUTHOBJ".to_string(),
            object_path,
            table: SubfileTable::new(
                vec!["Opt", "User", "Authority", "Origin"],
                vec![4, 18, 14, 14],
            ),
            rows: Vec::new(),
            status: String::new(),
            grant_prompt: false,
            grant_buffer: "QPGMR *ALL".to_string(),
        };

        screen.finish_grant();
        assert!(
            screen
                .rows
                .iter()
                .any(|row| row.user == "QPGMR" && row.authority == "*ALL")
        );

        let qpgmr_index = screen
            .rows
            .iter()
            .position(|row| row.user == "QPGMR")
            .expect("QPGMR row");
        for _ in 0..qpgmr_index {
            screen.table.select_next();
        }
        screen.revoke_selected();
        assert!(!screen.rows.iter().any(|row| row.user == "QPGMR"));
    }
}
