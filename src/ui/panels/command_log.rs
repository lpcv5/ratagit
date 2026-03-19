use crate::app::App;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render_command_log(frame: &mut Frame, area: Rect, app: &App) {
    let lines: Vec<Line> = if app.command_log.is_empty() {
        vec![Line::from(Span::styled(
            "No commands yet",
            Style::default().fg(Color::DarkGray),
        ))]
    } else {
        app.command_log
            .iter()
            .rev()
            .take(3)
            .map(|entry| {
                let color = if entry.success { Color::Green } else { Color::Red };
                let prefix = if entry.success { "✓ " } else { "✗ " };
                Line::from(Span::styled(
                    format!("{}{}", prefix, entry.command),
                    Style::default().fg(color),
                ))
            })
            .collect()
    };

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Command Log"));

    frame.render_widget(paragraph, area);
}
