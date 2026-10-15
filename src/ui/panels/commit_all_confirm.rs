use super::props::CommitAllConfirmProps;
use crate::ui::components::molecules::shortcut_hint;
use crate::ui::panels::{centered_rect, render_overlay_chrome};
use crate::ui::theme::UiTheme;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Style,
    text::Line,
    widgets::Paragraph,
    Frame,
};

pub fn render_commit_all_confirm(frame: &mut Frame, props: CommitAllConfirmProps) {
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
        props.change_count
    )))
    .style(Style::default().fg(theme.text_primary));
    frame.render_widget(warn, sections[0]);

    let detail = Paragraph::new(Line::from("Stage all files and proceed to commit editor?"))
        .style(Style::default().fg(theme.text_muted));
    frame.render_widget(detail, sections[1]);

    let actions = Paragraph::new(Line::from(
        [
            shortcut_hint("Y", "Stage All + Commit  ", &theme),
            shortcut_hint("N/Esc", "Cancel  ", &theme),
            shortcut_hint("Enter", "Confirm Y", &theme),
        ]
        .concat(),
    ));
    frame.render_widget(actions, sections[2]);
}
