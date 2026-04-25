use ratatui::style::{Color, Modifier, Style};

pub const COLOR_SCREEN_BG: Color = Color::Black;
pub const COLOR_HEADER_BG: Color = Color::Black;
pub const COLOR_HEADER_FG: Color = Color::Green;
pub const COLOR_BORDER: Color = Color::Green;
pub const COLOR_SELECTION_BG: Color = Color::Green;
pub const COLOR_SELECTION_FG: Color = Color::Black;
pub const COLOR_ERROR: Color = Color::White;
pub const COLOR_ERROR_BG: Color = Color::Red;
pub const COLOR_WARNING: Color = Color::Yellow;
pub const COLOR_HELP_BG: Color = Color::Green;
pub const COLOR_HELP_FG: Color = Color::Black;
pub const COLOR_NORMAL: Color = Color::Green;
pub const COLOR_DIM: Color = Color::DarkGray;
pub const COLOR_TITLE: Color = Color::White;

pub const STYLE_HEADER: Style = Style::new()
    .bg(COLOR_HEADER_BG)
    .fg(COLOR_HEADER_FG)
    .add_modifier(Modifier::BOLD);

pub const STYLE_BORDER: Style = Style::new().fg(COLOR_BORDER);

pub const STYLE_SELECTION: Style = Style::new().bg(COLOR_SELECTION_BG).fg(COLOR_SELECTION_FG);

pub const STYLE_ERROR: Style = Style::new()
    .bg(COLOR_ERROR_BG)
    .fg(COLOR_ERROR)
    .add_modifier(Modifier::BOLD);

pub const STYLE_WARNING: Style = Style::new().fg(COLOR_WARNING);

pub const STYLE_HELP: Style = Style::new().bg(COLOR_HELP_BG).fg(COLOR_HELP_FG);

pub const STYLE_NORMAL: Style = Style::new().bg(COLOR_SCREEN_BG).fg(COLOR_NORMAL);

pub const STYLE_DIM: Style = Style::new().fg(COLOR_DIM);

pub const STYLE_TITLE: Style = Style::new()
    .bg(COLOR_SCREEN_BG)
    .fg(COLOR_TITLE)
    .add_modifier(Modifier::BOLD);

pub const STYLE_OPTION: Style = Style::new().bg(COLOR_SCREEN_BG).fg(COLOR_NORMAL);

pub const STYLE_OPTION_SELECTED: Style = Style::new().bg(COLOR_SELECTION_BG).fg(COLOR_SELECTION_FG);

pub const STYLE_TABLE_HEADER: Style = Style::new()
    .bg(COLOR_SCREEN_BG)
    .fg(COLOR_TITLE)
    .add_modifier(Modifier::BOLD);

pub const STYLE_TABLE_ROW: Style = Style::new().bg(COLOR_SCREEN_BG).fg(COLOR_NORMAL);

pub const STYLE_TABLE_ROW_ALT: Style = Style::new().bg(COLOR_SCREEN_BG).fg(COLOR_DIM);
