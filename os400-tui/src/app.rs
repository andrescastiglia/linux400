use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::Terminal;

use crate::screens::cmd_line::CommandLine;
use crate::screens::dtaq_viewer::DataQueueViewer;
use crate::screens::main_menu::MainMenu;
use crate::screens::object_browser::ObjectBrowser;
use crate::screens::work_mgmt::WorkManagement;
use crate::screens::{Screen, ScreenId};

pub struct App {
    current_screen: Box<dyn Screen>,
    should_exit: bool,
    previous_screen: Option<ScreenId>,
}

impl App {
    pub fn new() -> Self {
        Self {
            current_screen: Box::new(MainMenu::new()),
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

        match result.next {
            Some(ScreenId::MainMenu) => {
                self.previous_screen = Some(ScreenId::MainMenu);
                self.current_screen = Box::new(MainMenu::new());
            }
            Some(ScreenId::WorkManagement) => {
                self.previous_screen = Some(ScreenId::WorkManagement);
                self.current_screen = Box::new(WorkManagement::new());
            }
            Some(ScreenId::ObjectBrowser) => {
                self.previous_screen = Some(ScreenId::ObjectBrowser);
                self.current_screen = Box::new(ObjectBrowser::new());
            }
            Some(ScreenId::DataQueueViewer) => {
                self.previous_screen = Some(ScreenId::DataQueueViewer);
                self.current_screen = Box::new(DataQueueViewer::new());
            }
            Some(ScreenId::CommandLine) => {
                self.previous_screen = Some(ScreenId::CommandLine);
                self.current_screen = Box::new(CommandLine::new());
            }
            Some(ScreenId::Exit) => {
                self.should_exit = true;
            }
            None => {}
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

pub type AppResult<T> = anyhow::Result<T>;
