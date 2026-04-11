use crate::flux::snapshot::CommitAllConfirmViewState;
use crate::ui::panels::{centered_rect, render_overlay_chrome};
use crate::ui::theme::UiTheme;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Style,
    text::Line,
    widgets::Paragraph,
    Frame,
};

pub fn render_commit_all_confirm(frame: &mut Frame, view: &CommitAllConfirmViewState) {
    if !view.is_open {
        return;
    }

    let theme = UiTheme::default();
    let area = centered_rect(frame.area(), 62, 26);
    let inner = render_overlay_chrome(frame, area, "Commit All Files", &theme);

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
        view.uncommitted_change_count
    )))
    .style(Style::default().fg(theme.text_primary));
    frame.render_widget(warn, sections[0]);

    let detail = Paragraph::new(Line::from("Stage all files and proceed to commit editor?"))
        .style(Style::default().fg(theme.text_muted));
    frame.render_widget(detail, sections[1]);

    let actions = Paragraph::new(Line::from(
        "[Y] Stage All + Commit    [N/Esc] Cancel    [Enter] Confirm Y",
    ))
    .style(Style::default().fg(theme.accent));
    frame.render_widget(actions, sections[2]);
}
