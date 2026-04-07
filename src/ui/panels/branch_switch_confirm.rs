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

pub fn render_branch_switch_confirm(frame: &mut Frame, snapshot: &AppStateSnapshot<'_>) {
    if snapshot.input_mode != Some(InputMode::BranchSwitchConfirm) {
        return;
    }

    let theme = UiTheme::default();
    let area = centered_rect(frame.area(), 62, 26);
    let target = snapshot.branch_switch_target.unwrap_or("<unknown>");
    let title = format!("Switch Branch: {}", target);
    let inner = render_overlay_chrome(frame, area, &title, &theme);

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(1),
        ])
        .split(inner);

    let warn = Paragraph::new(Line::from(format!(
        "Detected {} uncommitted file(s).",
        snapshot.uncommitted_change_count
    )))
    .style(Style::default().fg(theme.text_primary));
    frame.render_widget(warn, sections[0]);

    let detail = Paragraph::new(Line::from(
        "Auto stash and switch? This will run stash -u, switch, then stash pop.",
    ))
    .style(Style::default().fg(theme.text_muted));
    frame.render_widget(detail, sections[1]);

    let actions = Paragraph::new(Line::from(
        "[Y] Auto Stash + Switch    [N/Esc] Cancel    [Enter] Confirm Y",
    ))
    .style(Style::default().fg(theme.accent));
    frame.render_widget(actions, sections[2]);
}
