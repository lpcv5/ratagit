use crate::app::{App, InputMode};
use crate::ui::theme::UiTheme;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn render_shortcut_bar(frame: &mut Frame, area: Rect, app: &App) {
    let theme = UiTheme::default();
    let hints = app.shortcut_hints();

    let mut spans = Vec::new();
    if app.input_mode == Some(InputMode::Search) {
        spans.push(Span::styled(
            format!("/{}", app.input_buffer),
            Style::default().fg(theme.accent),
        ));
        spans.push(Span::raw("   "));
    }
    for (idx, (keys, desc)) in hints.iter().enumerate() {
        if idx > 0 {
            spans.push(Span::raw("   "));
        }
        spans.push(Span::styled(
            format!("[{}]", keys),
            Style::default().fg(theme.accent),
        ));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            desc.as_str(),
            Style::default().fg(theme.text_primary),
        ));
    }

    let line = Line::from(spans).style(Style::default().bg(theme.shortcut_bg));
    frame.render_widget(Paragraph::new(line), area);
}
