use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders};

#[derive(Debug, Clone, Copy)]
pub struct UiTheme {
    pub accent: Color,
    pub border_inactive: Color,
    pub text_primary: Color,
    pub text_muted: Color,
    pub shortcut_bg: Color,
}

impl Default for UiTheme {
    fn default() -> Self {
        Self {
            accent: Color::Rgb(82, 196, 208),
            border_inactive: Color::DarkGray,
            text_primary: Color::White,
            text_muted: Color::Gray,
            shortcut_bg: Color::Rgb(20, 28, 32),
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
                Style::default().fg(self.accent).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(self.border_inactive)
            })
    }

    pub fn active_highlight(&self) -> Style {
        Style::default()
            .bg(Color::Rgb(34, 52, 58))
            .fg(self.text_primary)
            .add_modifier(Modifier::BOLD)
    }

    pub fn inactive_highlight(&self) -> Style {
        Style::default()
    }
}
