use super::{
    overlay_utils::{action_lines, dialog_rect, dialog_sections},
    props::CommandPaletteProps,
    render_overlay_chrome,
};
use crate::ui::{
    components::{
        molecules::{card, render_input, shortcut_hint, EmptyState, InputField},
        scroll::scroll_offset,
    },
    theme::UiTheme,
};
use ratatui::{
    layout::{Constraint, Layout},
    style::Style,
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph},
    Frame,
};

pub fn render_command_palette(frame: &mut Frame, props: CommandPaletteProps<'_>) {
    let theme = UiTheme::default();
    let height = props.commands.len().clamp(3, 9) as u16 + 11;
    let area = dialog_rect(frame.area(), 84, height);
    let inner = render_overlay_chrome(frame, area, "Command Palette", &theme);
    let mut hints = vec![("↑/↓".into(), "Select".into())];
    hints.extend(props.hints.iter().cloned());
    let actions = action_lines(&hints, inner.width, &theme);
    let (body, footer) = dialog_sections(inner, actions.len() as u16);
    let sections = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(body);
    render_input(
        frame,
        sections[0],
        InputField {
            title: "Search commands",
            value: props.input,
            placeholder: "Type a command or keyword…",
            focused: true,
        },
        &theme,
    );
    let available = &props.commands;
    let title = format!("Commands ({})", available.len());
    let block = card(&theme, &title, false);
    let list_area = block.inner(sections[1]);
    frame.render_widget(block, sections[1]);
    if available.is_empty() {
        frame.render_widget(
            EmptyState::new("No matching commands · Try another keyword", &theme),
            list_area,
        );
    } else {
        let rows = available
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
                ListItem::new(Line::from(spans))
            })
            .collect::<Vec<_>>();
        let selected = props.selected.min(available.len() - 1);
        let mut state = ListState::default().with_selected(Some(selected));
        *state.offset_mut() = scroll_offset(
            0,
            Some(selected),
            available.len(),
            usize::from(list_area.height),
        );
        frame.render_stateful_widget(
            List::new(rows)
                .highlight_symbol("› ")
                .highlight_style(theme.active_highlight()),
            list_area,
            &mut state,
        );
    }
    frame.render_widget(Paragraph::new(actions), footer);
}
