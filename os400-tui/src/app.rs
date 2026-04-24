use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::Terminal;

use crate::screens::cmd_line::CommandLine;
use crate::screens::dtaq_viewer::DataQueueViewer;
use crate::screens::main_menu::MainMenu;
use crate::screens::object_browser::ObjectBrowser;
use crate::screens::pdm_browser::PdmBrowser;
use crate::screens::sign_on::SignOnScreen;
use crate::screens::str_seu::StrSeu;
use crate::screens::str_sql::StrSql;
use crate::screens::work_mgmt::WorkManagement;
use crate::screens::wrk_mbr_pdm::WrkMbrPdm;
use crate::screens::{Screen, ScreenId};
use crate::session::SessionContext;

pub struct App {
    current_screen: Box<dyn Screen>,
    current_screen_id: ScreenId,
    should_exit: bool,
    previous_screen: Option<ScreenId>,
    session: SessionContext,
}

impl App {
    pub fn new() -> Self {
        Self {
            current_screen: Box::new(SignOnScreen::new()),
            current_screen_id: ScreenId::SignOn,
            should_exit: false,
            previous_screen: None,
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
            })?;

            self.handle_events()?;
        }

        Ok(())
    }

    fn handle_events(&mut self) -> Result<()> {
        use crossterm::event::{poll, read, Event};

        if poll(std::time::Duration::from_millis(100))? {
            match read()? {
                Event::Key(key) => {
                    self.handle_key(key);
                }
                Event::Mouse(_) => {}
                Event::Resize(_, _) => {}
                Event::FocusGained => {}
                Event::FocusLost => {}
                Event::Paste(_) => {}
            }
        }

        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) {
        let result = self.current_screen.handle_key(key);

        if let Some(next) = result.next {
            self.switch_screen(next, result.data);
        }
    }

    fn switch_screen(&mut self, next: ScreenId, data: Option<String>) {
        let origin = self.current_screen_id;
        self.previous_screen = Some(origin);
        if next == ScreenId::SignOn {
            self.session.sign_off();
        }
        if next == ScreenId::MainMenu {
            if let Some(user) = data.clone() {
                self.session.sign_on(&user);
            }
        }
        self.current_screen_id = next;

        self.current_screen = match next {
            ScreenId::SignOn => Box::new(SignOnScreen::new()),
            ScreenId::MainMenu => Box::new(MainMenu::with_session(self.session.clone())),
            ScreenId::WorkManagement => Box::new(WorkManagement::new()),
            ScreenId::ObjectBrowser => Box::new(ObjectBrowser::with_session(self.session.clone())),
            ScreenId::DataQueueViewer => Box::new(DataQueueViewer::new()),
            ScreenId::CommandLine => Box::new(CommandLine::with_session(self.session.clone())),
            ScreenId::PdmBrowser => Box::new(PdmBrowser::with_session(self.session.clone())),
            ScreenId::WrkMbrPdm => {
                let (library, file) = parse_library_file_spec(data.as_deref(), &self.session);
                Box::new(WrkMbrPdm::new(library, file))
            }
            ScreenId::StrSeu => {
                let (library, file, member) = parse_member_spec(data.as_deref(), &self.session);
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
            ScreenId::Exit => {
                self.should_exit = true;
                self.session.sign_off();
                Box::new(MainMenu::with_session(self.session.clone()))
            }
        };
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
