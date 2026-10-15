use super::{
    overlay_utils::{action_lines, dialog_rect, dialog_sections},
    props::CommitEditorProps,
    render_overlay_chrome,
};
use crate::app::CommitFieldFocus;
use crate::ui::{
    components::molecules::{render_input, InputField},
    theme::UiTheme,
};
use ratatui::{
    layout::{Constraint, Layout},
    style::Style,
    widgets::Paragraph,
    Frame,
};

pub fn render_commit_editor(frame: &mut Frame, props: CommitEditorProps<'_>) {
    let theme = UiTheme::default();
    let area = dialog_rect(frame.area(), 84, 23);
    let inner = render_overlay_chrome(frame, area, "Commit Editor", &theme);
    let actions = action_lines(props.hints, inner.width, &theme);
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
                .unwrap_or("Describe the staged changes. * Required"),
        )
        .style(Style::default().fg(if props.error.is_some() {
            theme.error
        } else {
            theme.text_muted
        })),
        sections[0],
    );
    render_input(
        frame,
        sections[1],
        InputField {
            title: "Message *",
            value: props.message,
            placeholder: "A short summary of this change",
            focused: props.focus == CommitFieldFocus::Message,
        },
        &theme,
    );
    render_input(
        frame,
        sections[2],
        InputField {
            title: "Description · optional",
            value: props.description,
            placeholder: "Add context, motivation or details…",
            focused: props.focus == CommitFieldFocus::Description,
        },
        &theme,
    );
    frame.render_widget(Paragraph::new(actions), footer);
}
