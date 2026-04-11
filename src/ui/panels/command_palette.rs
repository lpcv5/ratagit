use crate::flux::snapshot::CommandPaletteViewState;
use crate::ui::panels::{centered_rect, render_overlay_chrome};
use crate::ui::theme::UiTheme;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Style,
    text::Line,
    widgets::Paragraph,
    Frame,
};

pub fn render_command_palette(frame: &mut Frame, view: &CommandPaletteViewState) {
    if !view.is_open {
        return;
    }

    let theme = UiTheme::default();
    let area = centered_rect(frame.area(), 70, 35);
    let inner = render_overlay_chrome(frame, area, "Command Palette", &theme);

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(4),
            Constraint::Length(1),
        ])
        .split(inner);

    let input = Paragraph::new(format!(":{}", view.input))
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
    let col = view
        .input
        .chars()
        .count()
        .saturating_add(2)
        .min(width as usize);
    let x = sections[0].x.saturating_add(col as u16);
    let y = sections[0].y.saturating_add(1);
    frame.set_cursor_position((x, y));
}
