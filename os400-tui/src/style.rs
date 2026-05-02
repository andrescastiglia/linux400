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

// -- 5250 input field styles --

/// Active (focused) input field: green text on dark background with underline.
pub const STYLE_INPUT_ACTIVE: Style = Style::new()
    .bg(Color::Black)
    .fg(Color::White)
    .add_modifier(Modifier::UNDERLINED);

/// Protected (inactive) input field: dim text, no underline.
pub const STYLE_INPUT_PROTECTED: Style = Style::new().bg(Color::Black).fg(COLOR_DIM);

/// Subfile separator line.
pub const STYLE_SUBFILE_SEPARATOR: Style = Style::new().bg(COLOR_SCREEN_BG).fg(COLOR_BORDER);

/// Ruler line (5250 style).
pub const STYLE_RULER: Style = Style::new().bg(COLOR_SCREEN_BG).fg(COLOR_NORMAL);

// -- Status bar styles --

/// Global status bar background.
pub const STYLE_STATUS_BAR: Style = Style::new().bg(Color::DarkGray).fg(Color::White);

// -- Enforcement mode indicators --

/// Full enforcement mode: green badge.
pub const STYLE_MODE_FULL: Style = Style::new()
    .bg(Color::Green)
    .fg(Color::Black)
    .add_modifier(Modifier::BOLD);

/// Degraded enforcement mode: yellow badge.
pub const STYLE_MODE_DEGRADED: Style = Style::new()
    .bg(Color::Yellow)
    .fg(Color::Black)
    .add_modifier(Modifier::BOLD);

/// Dev mode (no enforcement): red badge.
pub const STYLE_MODE_DEV: Style = Style::new()
    .bg(Color::Red)
    .fg(Color::White)
    .add_modifier(Modifier::BOLD);
