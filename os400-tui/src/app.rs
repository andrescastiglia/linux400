use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::Terminal;

use crate::screens::cmd_line::CommandLine;
use crate::screens::dtaq_viewer::DataQueueViewer;
use crate::screens::main_menu::MainMenu;
use crate::screens::object_browser::ObjectBrowser;
use crate::screens::pdm_browser::PdmBrowser;
use crate::screens::str_seu::StrSeu;
use crate::screens::str_sql::StrSql;
use crate::screens::work_mgmt::WorkManagement;
use crate::screens::wrk_mbr_pdm::WrkMbrPdm;
use crate::screens::{Screen, ScreenId};

pub struct App {
    current_screen: Box<dyn Screen>,
    current_screen_id: ScreenId,
    should_exit: bool,
    previous_screen: Option<ScreenId>,
}

impl App {
    pub fn new() -> Self {
        Self {
            current_screen: Box::new(MainMenu::new()),
            current_screen_id: ScreenId::MainMenu,
            should_exit: false,
            previous_screen: None,
        }
    }

    pub fn run<T: ratatui::backend::Backend>(&mut self, terminal: &mut Terminal<T>) -> Result<()> {
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
        self.current_screen_id = next;

        self.current_screen = match next {
            ScreenId::MainMenu => Box::new(MainMenu::new()),
            ScreenId::WorkManagement => Box::new(WorkManagement::new()),
            ScreenId::ObjectBrowser => Box::new(ObjectBrowser::new()),
            ScreenId::DataQueueViewer => Box::new(DataQueueViewer::new()),
            ScreenId::CommandLine => Box::new(CommandLine::new()),
            ScreenId::PdmBrowser => Box::new(PdmBrowser::new()),
            ScreenId::WrkMbrPdm => {
                let (library, file) = parse_library_file_spec(data.as_deref());
                Box::new(WrkMbrPdm::new(library, file))
            }
            ScreenId::StrSeu => {
                let (library, file, member) = parse_member_spec(data.as_deref());
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
                let context = data.as_deref().map(normalize_library_file_spec);
                let return_data = if origin == ScreenId::WrkMbrPdm {
                    context.clone()
                } else {
                    None
                };
                Box::new(StrSql::with_context(context, origin, return_data))
            }
            ScreenId::Exit => {
                self.should_exit = true;
                Box::new(MainMenu::new())
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

fn normalize_library_file_spec(spec: &str) -> String {
    let (library, file) = parse_library_file_spec(Some(spec));
    format!("{library}/{file}")
}

fn parse_library_file_spec(spec: Option<&str>) -> (String, String) {
    let spec = spec.unwrap_or("").trim();
    if let Some((library, file)) = spec.split_once('/') {
        let library = library.trim().to_uppercase();
        let file = file.trim().to_uppercase();
        if !library.is_empty() && !file.is_empty() {
            return (library, file);
        }
    } else if !spec.is_empty() {
        return (spec.to_uppercase(), "QCLSRC".to_string());
    }

    ("QSYS".to_string(), "QCLSRC".to_string())
}

fn parse_member_spec(spec: Option<&str>) -> (String, String, String) {
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
            "QSYS".to_string(),
            file.to_uppercase(),
            member.to_uppercase(),
        ),
        [member] => (
            "QSYS".to_string(),
            "QCLSRC".to_string(),
            member.to_uppercase(),
        ),
        _ => (
            "QSYS".to_string(),
            "QCLSRC".to_string(),
            "NEWMBR.CLP".to_string(),
        ),
    }
}
