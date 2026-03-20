use crate::app::{App, InputMode};
use crate::ui::theme::UiTheme;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn render_command_log(frame: &mut Frame, area: Rect, app: &App) {
    let theme = UiTheme::default();
    let mut lines: Vec<Line> = Vec::new();

    if app.input_mode == Some(InputMode::CreateBranch) {
        lines.push(Line::from(Span::styled(
            format!("branch> {}", app.input_buffer),
            Style::default().fg(Color::Yellow),
        )));
    }

    if app.command_log.is_empty() {
        if lines.is_empty() {
            lines.push(Line::from(Span::styled(
                "No commands yet",
                Style::default().fg(theme.text_muted),
            )));
        }
    } else {
        let available = 3usize.saturating_sub(lines.len());
        lines.extend(app.command_log.iter().rev().take(available).map(|entry| {
            let color = if entry.success {
                Color::Green
            } else {
                Color::Red
            };
            let prefix = if entry.success { "✓ " } else { "✗ " };
            Line::from(Span::styled(
                format!("{}{}", prefix, entry.command),
                Style::default().fg(color),
            ))
        }));
    }

    let paragraph = Paragraph::new(lines).block(theme.panel_block("Command Log", false));

    frame.render_widget(paragraph, area);
}
