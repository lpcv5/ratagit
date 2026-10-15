use super::{
    overlay_utils::{action_lines, dialog_rect, dialog_sections},
    props::StashEditorProps,
    render_overlay_chrome,
};
use crate::ui::{
    components::molecules::{render_input, InputField},
    theme::UiTheme,
};
use ratatui::{
    layout::{Constraint, Layout},
    style::Style,
    text::Line,
    widgets::Paragraph,
    Frame,
};

pub fn render_stash_editor(frame: &mut Frame, props: StashEditorProps<'_>) {
    let theme = UiTheme::default();
    let area = dialog_rect(frame.area(), 72, props.targets.len().min(5) as u16 + 12);
    let inner = render_overlay_chrome(frame, area, "Stash Changes", &theme);
    let hints = vec![
        ("Esc".into(), "Cancel".into()),
        ("Enter".into(), "Save stash".into()),
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
        Paragraph::new(props.error.map(str::to_owned).unwrap_or_else(|| {
            format!(
                "Save changes from {} selected target(s).",
                props.targets.len()
            )
        }))
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
            title: "Stash title *",
            value: props.message,
            placeholder: "Work in progress",
            focused: true,
        },
        &theme,
    );
    // The field viewport is bounded; the target count always describes all paths.
    let mut targets = vec![Line::from("Selected targets")];
    let visible = usize::from(sections[2].height.saturating_sub(1));
    for (index, path) in props.targets.iter().take(visible).enumerate() {
        if index + 1 == visible && props.targets.len() > visible {
            targets.push(Line::from(format!(
                "… and {} more",
                props.targets.len() - index
            )));
        } else {
            targets.push(Line::from(format!("  {path}")));
        }
    }
    frame.render_widget(
        Paragraph::new(targets).style(Style::default().fg(theme.text_muted)),
        sections[2],
    );
    frame.render_widget(Paragraph::new(actions), footer);
}
