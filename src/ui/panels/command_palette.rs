use crate::app::InputMode;
use crate::flux::snapshot::AppStateSnapshot;
use crate::ui::panels::{centered_rect, render_overlay_chrome};
use crate::ui::theme::UiTheme;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Style,
    text::Line,
    widgets::Paragraph,
    Frame,
};

pub fn render_command_palette(frame: &mut Frame, snapshot: &AppStateSnapshot<'_>) {
    if snapshot.input_mode != Some(InputMode::CommandPalette) {
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

    let input = Paragraph::new(format!(":{}", snapshot.input_buffer))
        .block(theme.panel_block("Command", true))
        .style(Style::default().fg(theme.text_primary));
    frame.render_widget(input, sections[0]);

    let query = crate::app::commands::normalize_input(snapshot.input_buffer);
    let available: Vec<_> = snapshot
        .available_commands
        .iter()
        .filter(|command| {
            command.aliases.iter().any(|alias| alias.contains(&query))
                || command.label.to_lowercase().contains(&query)
        })
        .collect();
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
                Line::from(format!("{}  {}  ({})", keys, command.name, command.label))
            })
            .collect()
    };
    let commands_title = format!("Commands ({})", available.len());
    let commands = Paragraph::new(rows)
        .style(Style::default().fg(theme.text_muted))
        .block(theme.panel_block(&commands_title, false));
    frame.render_widget(commands, sections[1]);

    let help = Paragraph::new(Line::from(
        snapshot
            .shortcut_hints
            .iter()
            .map(|(key, label)| format!("{key} {label}"))
            .collect::<Vec<_>>()
            .join(" | "),
    ))
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
