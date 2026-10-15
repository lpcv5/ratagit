use crate::ui::theme::UiTheme;
use ratatui::{
    style::{Modifier, Style},
    widgets::{Block, BorderType, Borders},
};

/// Shared panel/input chrome. Keep a one-cell border so every caller can use
/// `Block::inner` for content and cursor geometry.
pub fn card(theme: &UiTheme, title: &str, focused: bool) -> Block<'static> {
    Block::default()
        .style(Style::default().fg(theme.text_primary).bg(theme.surface))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(format!(" {title} "))
        .border_style(Style::default().fg(if focused {
            theme.accent
        } else {
            theme.border_inactive
        }))
        .title_style(if focused {
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text_muted)
        })
}
