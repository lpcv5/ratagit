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

pub fn render_commit_all_confirm(frame: &mut Frame, snapshot: &AppStateSnapshot<'_>) {
    if snapshot.input_mode != Some(InputMode::CommitAllConfirm) {
        return;
    }

    let theme = UiTheme::default();
    let area = centered_rect(frame.area(), 62, 26);
    frame.render_widget(Clear, area);

    let total_count = snapshot.uncommitted_change_count;
    let title = "Commit All Files";
    frame.render_widget(theme.panel_block(title, true), area);

    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(1),
        ])
        .split(inner);

    let warn = Paragraph::new(Line::from(format!(
        "No files staged. Found {} file(s) to commit.",
        total_count
    )))
    .style(Style::default().fg(theme.text_primary));
    frame.render_widget(warn, sections[0]);

    let detail = Paragraph::new(Line::from(
        "Stage all files and proceed to commit editor?",
    ))
    .style(Style::default().fg(theme.text_muted));
    frame.render_widget(detail, sections[1]);

    let actions = Paragraph::new(Line::from(
        "[Y] Stage All + Commit    [N/Esc] Cancel    [Enter] Confirm Y",
    ))
    .style(Style::default().fg(theme.accent));
    frame.render_widget(actions, sections[2]);
}

