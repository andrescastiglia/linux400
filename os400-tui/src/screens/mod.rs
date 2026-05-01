pub mod admin_views;
pub mod cmd_line;
pub mod dtaq_viewer;
pub mod main_menu;
pub mod object_authority;
pub mod object_browser;
pub mod object_detail;
pub mod pdm_browser;
pub mod power_down;
pub mod sign_on;
pub mod str_seu;
pub mod str_sql;
pub mod submit_job;
pub mod system_panel;
pub mod work_mgmt;
pub mod wrk_job;
pub mod wrk_lib;
pub mod wrk_mbr_pdm;
pub mod wrk_usrprf;

use crossterm::event::KeyEvent;
use ratatui::Frame;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenId {
    SignOn,
    MainMenu,
    CommandMenu,
    PowerDown,
    WorkManagement,
    WrkJob,
    WrkLib,
    ObjectBrowser,
    DataQueueViewer,
    CommandLine,
    PdmBrowser,
    WrkMbrPdm,
    StrSeu,
    StrSql,
    SubmitJob,
    ObjectDetail,
    ObjectAuthority,
    UserProfiles,
    PolicyAudit,
    SpoolOutq,
    SystemPanel,
    Exit,
    /// Pop the navigation stack to return to the previous screen.
    Back,
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

    pub fn with_data(screen: ScreenId, data: impl Into<String>) -> Self {
        Self {
            next: Some(screen),
            data: Some(data.into()),
        }
    }

    pub fn exit() -> Self {
        Self {
            next: Some(ScreenId::Exit),
            data: None,
        }
    }

    /// Navigate back to the previous screen in the stack.
    pub fn back() -> Self {
        Self {
            next: Some(ScreenId::Back),
            data: None,
        }
    }
}

pub trait Screen {
    fn render(&mut self, frame: &mut Frame);
    fn handle_key(&mut self, key: KeyEvent) -> ScreenResult;
}
