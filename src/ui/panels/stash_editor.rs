use crate::app::InputMode;
use crate::flux::snapshot::AppStateSnapshot;
use crate::ui::panels::centered_rect;
use crate::ui::theme::UiTheme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::Line,
    widgets::{Clear, Paragraph},
    Frame,
};

pub fn render_stash_editor(frame: &mut Frame, snapshot: &AppStateSnapshot<'_>) {
    if snapshot.input_mode != Some(InputMode::StashEditor) {
        return;
    }

    let theme = UiTheme::default();
    let area = centered_rect(frame.area(), 60, 25);
    frame.render_widget(Clear, area);

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
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let title = format!(
        "Stash Editor | Targets: {} | Enter confirm | Esc cancel",
        snapshot.stash_targets.len()
    );
    frame.render_widget(theme.panel_block(&title, true), area);

    let message = Paragraph::new(snapshot.stash_message_buffer)
        .block(theme.panel_block("Title", true))
        .style(Style::default().fg(theme.text_primary));
    frame.render_widget(message, sections[0]);

    let help = Paragraph::new(Line::from(
        "Use visual mode (v) for batch stash, or current cursor item",
    ))
    .style(Style::default().fg(theme.text_muted));
    frame.render_widget(help, sections[1]);

    let targets = Paragraph::new(Line::from(format!(
        "Selected targets: {}",
        snapshot
            .stash_targets
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    )))
    .style(Style::default().fg(theme.text_muted));
    frame.render_widget(targets, sections[2]);

    let width = sections[0].width.saturating_sub(2).max(1);
    let col = snapshot
        .stash_message_buffer
        .chars()
        .count()
        .saturating_add(1)
        .min(width as usize);
    let x = sections[0].x.saturating_add(col as u16);
    let y = sections[0].y.saturating_add(1);
    frame.set_cursor_position((x, y));
}
