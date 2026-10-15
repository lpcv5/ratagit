use super::{
    overlay_utils::{action_lines, dialog_rect, dialog_sections},
    props::BranchEditorProps,
    render_overlay_chrome,
};
use crate::ui::{
    components::molecules::{render_input, InputField},
    theme::UiTheme,
};
use ratatui::{
    layout::{Constraint, Layout},
    style::Style,
    widgets::{Paragraph, Wrap},
    Frame,
};

pub fn render_branch_editor(frame: &mut Frame, props: BranchEditorProps<'_>) {
    let theme = UiTheme::default();
    let area = dialog_rect(frame.area(), 64, 12);
    let inner = render_overlay_chrome(frame, area, "Create Branch", &theme);
    let hints = vec![
        ("Esc".into(), "Cancel".into()),
        ("Enter".into(), "Create branch".into()),
    ];
    let actions = action_lines(&hints, inner.width, &theme);
    let (body, footer) = dialog_sections(inner, actions.len() as u16);
    let sections = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(3),
        Constraint::Min(0),
    ])
    .split(body);
    frame.render_widget(
        Paragraph::new(
            props
                .error
                .unwrap_or("Create a branch from the current HEAD."),
        )
        .style(Style::default().fg(if props.error.is_some() {
            theme.error
        } else {
            theme.text_muted
        }))
        .wrap(Wrap { trim: false }),
        sections[0],
    );
    render_input(
        frame,
        sections[1],
        InputField {
            title: "Branch name *",
            value: props.name,
            placeholder: "feature/my-change",
            focused: true,
        },
        &theme,
    );
    frame.render_widget(Paragraph::new(actions), footer);
}
