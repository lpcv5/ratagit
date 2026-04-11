use crate::flux::snapshot::ShortcutBarViewState;
use crate::ui::components::loading_indicator::LoadingIndicator;
use crate::ui::theme::UiTheme;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn render_shortcut_bar(frame: &mut Frame, area: Rect, view: &ShortcutBarViewState) {
    let theme = UiTheme::default();
    let hints = &view.hints;

    let mut spans = Vec::new();

    // Show animated spinner when background Git tasks are running.
    if view.has_background_tasks {
        // Derive frame index from time so the spinner animates without storing
        // per-render mutable state. 10 frames at ~100ms each = ~1 s cycle.
        let frame_index = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_millis()
            / 100) as usize
            % 10;
        let mut indicator = LoadingIndicator::new();
        // Fast-forward to the correct frame.
        for _ in 0..frame_index {
            indicator.tick();
        }
        spans.push(indicator.current_span());
        spans.push(Span::raw("   "));
    }

    if let Some(search_input) = &view.search_input {
        spans.push(Span::styled(
            format!("/{}", search_input),
            Style::default().fg(theme.accent),
        ));
        spans.push(Span::raw("   "));
    }
    for (idx, entry) in hints.iter().enumerate() {
        if idx > 0 {
            spans.push(Span::raw("   "));
        }
        spans.push(Span::styled(
            format!("[{}]", entry.keys),
            Style::default().fg(theme.accent),
        ));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            entry.description.as_str(),
            Style::default().fg(theme.text_primary),
        ));
    }

    let line = Line::from(spans).style(Style::default().bg(theme.shortcut_bg));
    frame.render_widget(Paragraph::new(line), area);
}
