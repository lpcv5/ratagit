use super::props::ShortcutBarProps;
use crate::ui::components::loading_indicator::LoadingIndicator;
use crate::ui::components::molecules::shortcut_hint;
use crate::ui::theme::UiTheme;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn render_shortcut_bar(frame: &mut Frame, area: Rect, props: ShortcutBarProps<'_>) {
    let theme = UiTheme::default();
    let hints = props.hints;

    let mut spans = vec![Span::raw(" ")];

    if let Some(frame_index) = props.loading_frame {
        let mut indicator = LoadingIndicator::new();
        // Fast-forward to the correct frame.
        for _ in 0..frame_index {
            indicator.tick();
        }
        spans.push(indicator.current_span());
        spans.push(Span::raw("   "));
    }

    if let Some(input) = props.search_input {
        spans.push(Span::styled(
            format!("/{input}"),
            Style::default().fg(theme.accent),
        ));
        spans.push(Span::raw("   "));
    }
    for (idx, (keys, desc)) in hints.iter().enumerate() {
        if idx > 0 {
            spans.push(Span::raw("  "));
        }
        spans.extend(shortcut_hint(keys, desc, &theme));
    }

    let line = Line::from(spans);
    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(theme.shortcut_bg)),
        area,
    );
}
