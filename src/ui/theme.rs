use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders};

#[derive(Debug, Clone, Copy)]
pub struct UiTheme {
    pub accent: Color,
    pub border_inactive: Color,
    pub text_primary: Color,
    pub text_muted: Color,
    pub shortcut_bg: Color,

    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,

    pub selection_bg: Color,
    pub visual_selection_bg: Color,
}

impl Default for UiTheme {
    fn default() -> Self {
        Self {
            accent: Color::Rgb(82, 196, 208),
            border_inactive: Color::DarkGray,
            text_primary: Color::White,
            text_muted: Color::Gray,
            shortcut_bg: Color::Rgb(20, 28, 32),

            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            info: Color::Cyan,

            selection_bg: Color::Rgb(34, 52, 58),
            visual_selection_bg: Color::Rgb(60, 80, 100),
        }
    }
}

impl UiTheme {
    pub fn panel_block<'a>(&self, title: &'a str, is_active: bool) -> Block<'a> {
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(title)
            .border_style(if is_active {
                Style::default().fg(self.accent)
            } else {
                Style::default().fg(self.border_inactive)
            })
            .title_style(if is_active {
                Style::default()
                    .fg(self.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(self.border_inactive)
            })
    }

    pub fn active_highlight(&self) -> Style {
        Style::default()
            .bg(self.selection_bg)
            .fg(self.text_primary)
            .add_modifier(Modifier::BOLD)
    }

    pub fn inactive_highlight(&self) -> Style {
        Style::default()
    }

    pub fn highlight_for(&self, is_active: bool) -> Style {
        if is_active {
            self.active_highlight()
        } else {
            self.inactive_highlight()
        }
    }
}
