use crate::ui::{components::atoms::keycap, theme::UiTheme};
use ratatui::{style::Style, text::Span};

/// Shared by footers and modal action rows; keys come from the caller's keymap.
pub fn shortcut_hint(keys: &str, label: &str, theme: &UiTheme) -> Vec<Span<'static>> {
    vec![
        keycap(keys, theme),
        Span::raw(" "),
        Span::styled(label.to_owned(), Style::default().fg(theme.text_muted)),
    ]
}
