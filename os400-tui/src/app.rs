use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::{
    Terminal,
    layout::{Constraint, Direction, Layout},
};

use crate::screens::admin_views::AdminCommandView;
use crate::screens::cmd_line::CommandLine;
use crate::screens::dtaq_viewer::DataQueueViewer;
use crate::screens::main_menu::MainMenu;
use crate::screens::object_browser::ObjectBrowser;
use crate::screens::object_detail::ObjectDetail;
use crate::screens::pdm_browser::PdmBrowser;
use crate::screens::power_down::PowerDownSystem;
use crate::screens::sign_on::SignOnScreen;
use crate::screens::str_seu::StrSeu;
use crate::screens::str_sql::StrSql;
use crate::screens::system_panel::SystemPanel;
use crate::screens::work_mgmt::WorkManagement;
use crate::screens::wrk_lib::WrkLib;
use crate::screens::wrk_mbr_pdm::WrkMbrPdm;
use crate::screens::{Screen, ScreenId};
use crate::session::SessionContext;
use crate::widgets::status_bar::StatusBar;

/// Maximum navigation stack depth to prevent unbounded growth.
const MAX_NAV_STACK: usize = 16;

/// A navigation entry in the screen stack.
#[derive(Clone, Debug)]
struct NavEntry {
    screen_id: ScreenId,
    data: Option<String>,
}

pub struct App {
    current_screen: Box<dyn Screen>,
    current_screen_id: ScreenId,
    should_exit: bool,
    /// Navigation stack for back-navigation (LIFO).
    /// Each entry records the screen we came from and the data it was created with.
    nav_stack: Vec<NavEntry>,
    session: SessionContext,
}

impl App {
    pub fn new() -> Self {
        Self {
            current_screen: Box::new(SignOnScreen::new()),
            current_screen_id: ScreenId::SignOn,
            should_exit: false,
            nav_stack: Vec::new(),
            session: SessionContext::new(std::process::id() as u64),
        }
    }

    pub fn run<T>(&mut self, terminal: &mut Terminal<T>) -> Result<()>
    where
        T: ratatui::backend::Backend,
        T::Error: std::error::Error + Send + Sync + 'static,
    {
        loop {
            if self.should_exit {
                break;
            }

            terminal.draw(|frame| {
                self.current_screen.render(frame);
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(0), Constraint::Length(1)])
                    .split(frame.area());
                StatusBar::new(self.session.clone()).render(frame, chunks[1]);
            })?;

            self.handle_events()?;
        }

        Ok(())
    }

    fn handle_events(&mut self) -> Result<()> {
        use crossterm::event::{Event, poll, read};

        if poll(std::time::Duration::from_millis(100))? {
            match read()? {
                Event::Key(key) => {
                    self.handle_key(key);
                }
                Event::Mouse(_) => {}
                Event::Resize(_, _) => {
                    let _ = self
                        .current_screen
                        .handle_key(KeyEvent::from(crossterm::event::KeyCode::Null));
                }
                Event::FocusGained => {}
                Event::FocusLost => {}
                Event::Paste(_) => {}
            }
        }

        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) {
        let result = self.current_screen.handle_key(key);

        match result.next {
            Some(ScreenId::Back) => {
                self.navigate_back();
            }
            Some(next) => {
                self.switch_screen(next, result.data);
            }
            None => {}
        }
    }

    /// Navigate back to the previous screen in the stack.
    fn navigate_back(&mut self) {
        if let Some(entry) = self.nav_stack.pop() {
            self.current_screen_id = entry.screen_id;
            self.current_screen = self.create_screen(entry.screen_id, entry.data, None);
        }
        // If stack is empty, stay on current screen.
    }

    fn switch_screen(&mut self, next: ScreenId, data: Option<String>) {
        let origin = self.current_screen_id;

        if next == ScreenId::SignOn {
            self.session.sign_off();
            self.nav_stack.clear();
        }

        if next == ScreenId::MainMenu
            && let Some(user) = data.clone()
        {
            self.session.sign_on(&user);
            self.nav_stack.clear(); // fresh start after sign-on
        }

        // Push current screen onto the nav stack (unless navigating to SignOn or Exit).
        if next != ScreenId::SignOn && next != ScreenId::Exit {
            self.nav_stack.push(NavEntry {
                screen_id: origin,
                data: None, // we don't track the original creation data of the current screen
            });
            // Enforce stack limit.
            if self.nav_stack.len() > MAX_NAV_STACK {
                self.nav_stack.remove(0);
            }
        }

        self.current_screen_id = next;
        self.current_screen = self.create_screen(next, data, Some(origin));

        if next == ScreenId::Exit {
            self.should_exit = true;
            self.session.sign_off();
        }
    }

    fn create_screen(
        &self,
        screen_id: ScreenId,
        data: Option<String>,
        origin: Option<ScreenId>,
    ) -> Box<dyn Screen> {
        match screen_id {
            ScreenId::SignOn => Box::new(SignOnScreen::new()),
            ScreenId::MainMenu => Box::new(MainMenu::with_session(self.session.clone())),
            ScreenId::CommandMenu => Box::new(MainMenu::command_menu(
                data.as_deref().unwrap_or("MAIN"),
                self.session.clone(),
            )),
            ScreenId::PowerDown => Box::new(PowerDownSystem::new(self.session.clone())),
            ScreenId::WorkManagement => Box::new(WorkManagement::new()),
            ScreenId::WrkLib => Box::new(WrkLib::new(self.session.clone())),
            ScreenId::ObjectBrowser => {
                if let Some(library) = data {
                    Box::new(ObjectBrowser::for_library(library, self.session.clone()))
                } else {
                    Box::new(ObjectBrowser::with_session(self.session.clone()))
                }
            }
            ScreenId::DataQueueViewer => Box::new(
                data.as_deref()
                    .map(DataQueueViewer::from_spec)
                    .unwrap_or_default(),
            ),
            ScreenId::CommandLine => Box::new(CommandLine::with_session(self.session.clone())),
            ScreenId::PdmBrowser => Box::new(PdmBrowser::with_session(self.session.clone())),
            ScreenId::WrkMbrPdm => {
                let (library, file) = parse_library_file_spec(data.as_deref(), &self.session);
                Box::new(WrkMbrPdm::new(library, file))
            }
            ScreenId::StrSeu => {
                let (library, file, member) = parse_member_spec(data.as_deref(), &self.session);
                let origin = origin.unwrap_or(ScreenId::MainMenu);
                let return_data = if origin == ScreenId::WrkMbrPdm {
                    Some(format!("{library}/{file}"))
                } else {
                    None
                };
                Box::new(StrSeu::from_member_spec(
                    &library,
                    &file,
                    &member,
                    origin,
                    return_data,
                ))
            }
            ScreenId::StrSql => {
                let context = data
                    .as_deref()
                    .map(|value| normalize_library_file_spec(value, &self.session));
                let origin = origin.unwrap_or(ScreenId::MainMenu);
                let return_data = if origin == ScreenId::WrkMbrPdm {
                    context.clone()
                } else {
                    None
                };
                Box::new(StrSql::with_session(
                    context,
                    origin,
                    return_data,
                    self.session.clone(),
                ))
            }
            ScreenId::ObjectDetail => {
                Box::new(ObjectDetail::new(data.as_deref(), self.session.clone()))
            }
            ScreenId::UserProfiles => {
                Box::new(AdminCommandView::user_profiles(self.session.clone()))
            }
            ScreenId::PolicyAudit => Box::new(AdminCommandView::policy_audit(
                data.as_deref(),
                self.session.clone(),
            )),
            ScreenId::SpoolOutq => Box::new(AdminCommandView::spool_outq(
                data.as_deref(),
                self.session.clone(),
            )),
            ScreenId::SystemPanel => Box::new(SystemPanel::new(
                data.unwrap_or_else(|| "WRKSYSSTS".to_string()),
                self.session.clone(),
            )),
            ScreenId::Exit | ScreenId::Back => {
                // Exit is handled in switch_screen; Back is handled in handle_key.
                // This branch should not be reached.
                Box::new(MainMenu::with_session(self.session.clone()))
            }
        }
    }

    /// Current depth of the navigation stack (for testing).
    #[cfg(test)]
    pub fn nav_depth(&self) -> usize {
        self.nav_stack.len()
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

pub type AppResult<T> = anyhow::Result<T>;

fn normalize_library_file_spec(spec: &str, session: &SessionContext) -> String {
    let (library, file) = parse_library_file_spec(Some(spec), session);
    format!("{library}/{file}")
}

fn parse_library_file_spec(spec: Option<&str>, session: &SessionContext) -> (String, String) {
    let spec = spec.unwrap_or("").trim();
    if let Some((library, file)) = spec.split_once('/') {
        let library = library.trim().to_uppercase();
        let file = file.trim().to_uppercase();
        if !library.is_empty() && !file.is_empty() {
            return (library, file);
        }
    } else if !spec.is_empty() {
        return (session.snapshot().current_library, spec.to_uppercase());
    }

    (session.snapshot().current_library, "QCLSRC".to_string())
}

fn parse_member_spec(spec: Option<&str>, session: &SessionContext) -> (String, String, String) {
    let spec = spec.unwrap_or("").trim();
    let parts = spec
        .split('/')
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();

    match parts.as_slice() {
        [library, file, member] => (
            library.to_uppercase(),
            file.to_uppercase(),
            member.to_uppercase(),
        ),
        [file, member] => (
            session.snapshot().current_library,
            file.to_uppercase(),
            member.to_uppercase(),
        ),
        [member] => (
            session.snapshot().current_library,
            "QCLSRC".to_string(),
            member.to_uppercase(),
        ),
        _ => (
            session.snapshot().current_library,
            "QCLSRC".to_string(),
            "NEWMBR.CLP".to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_stack_pops_through_four_levels() {
        let mut app = App::new();

        app.switch_screen(ScreenId::MainMenu, None);
        app.switch_screen(ScreenId::CommandMenu, Some("CMDOBJ".to_string()));
        app.switch_screen(ScreenId::CommandLine, None);
        app.switch_screen(ScreenId::ObjectBrowser, None);

        assert_eq!(app.nav_depth(), 4);

        app.navigate_back();
        assert_eq!(app.current_screen_id, ScreenId::CommandLine);
        app.navigate_back();
        assert_eq!(app.current_screen_id, ScreenId::CommandMenu);
        app.navigate_back();
        assert_eq!(app.current_screen_id, ScreenId::MainMenu);
        app.navigate_back();
        assert_eq!(app.current_screen_id, ScreenId::SignOn);
    }

    #[test]
    fn navigation_stack_is_limited() {
        let mut app = App::new();
        for _ in 0..32 {
            app.switch_screen(ScreenId::CommandLine, None);
        }
        assert_eq!(app.nav_depth(), MAX_NAV_STACK);
    }
}
