use crate::ui::theme::UiTheme;
use ratatui::{
    style::{Modifier, Style},
    text::Span,
};

#[derive(Clone, Copy)]
pub enum Tone {
    Neutral,
    Accent,
    Success,
    Warning,
    Danger,
}

/// A compact semantic label with its own background and horizontal inset.
pub fn badge(label: impl Into<String>, tone: Tone, theme: &UiTheme) -> Span<'static> {
    let (fg, bg) = match tone {
        Tone::Neutral => (theme.text_muted, theme.surface_raised),
        Tone::Accent => (theme.accent, theme.selection_bg),
        Tone::Success => (theme.success, theme.diff_added_bg),
        Tone::Warning => (theme.warning, theme.surface_raised),
        Tone::Danger => (theme.error, theme.diff_removed_bg),
    };
    Span::styled(
        format!(" {} ", label.into()),
        Style::default().fg(fg).bg(bg),
    )
}

/// Key labels are display-only; callers retain all input and keymap behavior.
pub fn keycap(label: impl Into<String>, theme: &UiTheme) -> Span<'static> {
    badge(label, Tone::Accent, theme).style(
        Style::default()
            .fg(theme.accent)
            .bg(theme.surface_raised)
            .add_modifier(Modifier::BOLD),
    )
}
