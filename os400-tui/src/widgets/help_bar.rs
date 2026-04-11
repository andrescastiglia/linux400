use ratatui::{layout::Rect, widgets::Widget};

pub struct HelpBar;

impl HelpBar {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HelpBar {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for HelpBar {
    fn render(self, _area: Rect, _buf: &mut ratatui::buffer::Buffer) {
        // Placeholder - actual rendering handled by screens
    }
}
