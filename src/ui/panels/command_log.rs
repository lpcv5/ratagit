use crate::app::{App, InputMode};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render_command_log(frame: &mut Frame, area: Rect, app: &App) {
    let mut lines: Vec<Line> = Vec::new();

    if let Some(mode) = app.input_mode {
        let prompt = match mode {
            InputMode::CommitMessage => "commit>",
            InputMode::CreateBranch => "branch>",
        };
        lines.push(Line::from(Span::styled(
            format!("{} {}", prompt, app.input_buffer),
            Style::default().fg(Color::Yellow),
        )));
    }

    if app.command_log.is_empty() {
        if lines.is_empty() {
            lines.push(Line::from(Span::styled(
                "No commands yet",
                Style::default().fg(Color::DarkGray),
            )));
        }
    } else {
        let available = 3usize.saturating_sub(lines.len());
        lines.extend(app.command_log.iter().rev().take(available).map(|entry| {
            let color = if entry.success { Color::Green } else { Color::Red };
            let prefix = if entry.success { "✓ " } else { "✗ " };
            Line::from(Span::styled(
                format!("{}{}", prefix, entry.command),
                Style::default().fg(color),
            ))
        }));
    }

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Command Log"));

    frame.render_widget(paragraph, area);
}
