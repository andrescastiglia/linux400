pub mod main_menu;
pub mod work_mgmt;
pub mod object_browser;
pub mod dtaq_viewer;
pub mod cmd_line;

use ratatui::Frame;
use crossterm::event::KeyEvent;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenId {
    MainMenu,
    WorkManagement,
    ObjectBrowser,
    DataQueueViewer,
    CommandLine,
    Exit,
}

#[derive(Clone, Debug)]
pub struct ScreenResult {
    pub next: Option<ScreenId>,
    pub data: Option<String>,
}

impl ScreenResult {
    pub fn none() -> Self {
        Self {
            next: None,
            data: None,
        }
    }

    pub fn goto(screen: ScreenId) -> Self {
        Self {
            next: Some(screen),
            data: None,
        }
    }

    pub fn exit() -> Self {
        Self {
            next: Some(ScreenId::Exit),
            data: None,
        }
    }
}

pub trait Screen {
    fn render(&mut self, frame: &mut Frame);
    fn handle_key(&mut self, key: KeyEvent) -> ScreenResult;
}
