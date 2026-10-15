use super::props::CommandPaletteProps;
use crate::ui::components::molecules::{card, shortcut_hint};
use crate::ui::panels::{centered_rect, render_overlay_chrome};
use crate::ui::theme::UiTheme;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn render_command_palette(frame: &mut Frame, props: CommandPaletteProps<'_>) {
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

    let input = Paragraph::new(format!(":{}", props.input))
        .block(card(&theme, "Command", true))
        .style(Style::default().fg(theme.text_primary));
    frame.render_widget(input, sections[0]);

    let available = &props.commands;
    let rows: Vec<_> = if available.is_empty() {
        vec![Line::from("No matching commands")]
    } else {
        available
            .iter()
            .map(|command| {
                let keys = if command.keys.is_empty() {
                    "--".into()
                } else {
                    command.keys.join("/")
                };
                let mut spans = shortcut_hint(&keys, command.name, &theme);
                spans.push(Span::styled(
                    format!("  ({})", command.label),
                    Style::default().fg(theme.text_muted),
                ));
                Line::from(spans)
            })
            .collect()
    };
    let commands_title = format!("Commands ({})", available.len());
    let commands = Paragraph::new(rows)
        .style(Style::default().fg(theme.text_muted))
        .block(card(&theme, &commands_title, false));
    frame.render_widget(commands, sections[1]);

    let help = Paragraph::new(Line::from(
        props
            .hints
            .iter()
            .flat_map(|(key, label)| shortcut_hint(key, &format!("{label}  "), &theme))
            .collect::<Vec<_>>(),
    ))
    .style(Style::default().fg(theme.accent));
    frame.render_widget(help, sections[2]);

    let width = sections[0].width.saturating_sub(2).max(1);
    let col = props
        .input
        .chars()
        .count()
        .saturating_add(2)
        .min(width as usize);
    let x = sections[0].x.saturating_add(col as u16);
    let y = sections[0].y.saturating_add(1);
    frame.set_cursor_position((x, y));
}
