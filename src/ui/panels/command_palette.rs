use crate::app::InputMode;
use crate::flux::snapshot::AppStateSnapshot;
use crate::ui::theme::UiTheme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::Line,
    widgets::{Clear, Paragraph},
    Frame,
};

pub fn render_command_palette(frame: &mut Frame, snapshot: &AppStateSnapshot<'_>) {
    if snapshot.input_mode != Some(InputMode::CommandPalette) {
        return;
    }

    let theme = UiTheme::default();
    let area = centered_rect(frame.area(), 70, 35);
    frame.render_widget(Clear, area);
    frame.render_widget(theme.panel_block("Command Palette", true), area);

    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(4),
            Constraint::Length(1),
        ])
        .split(inner);

    let input = Paragraph::new(format!(":{}", snapshot.input_buffer))
        .block(theme.panel_block("Command", true))
        .style(Style::default().fg(theme.text_primary));
    frame.render_widget(input, sections[0]);

    let commands = Paragraph::new(vec![
        Line::from("refresh | quit | commit | search"),
        Line::from("stash | branch new | fetch"),
        Line::from("panel files|branches|commits|stash"),
    ])
    .style(Style::default().fg(theme.text_muted))
    .block(theme.panel_block("Examples", false));
    frame.render_widget(commands, sections[1]);

    let help = Paragraph::new(Line::from("Enter run | Esc cancel"))
        .style(Style::default().fg(theme.accent));
    frame.render_widget(help, sections[2]);

    let width = sections[0].width.saturating_sub(2).max(1);
    let col = snapshot
        .input_buffer
        .chars()
        .count()
        .saturating_add(2)
        .min(width as usize);
    let x = sections[0].x.saturating_add(col as u16);
    let y = sections[0].y.saturating_add(1);
    frame.set_cursor_position((x, y));
}

fn centered_rect(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
