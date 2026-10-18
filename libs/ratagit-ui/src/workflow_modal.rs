use crate::modal::{ModalSpec, ModalTone, command_menu_line, render_menu_lines, render_modal};
use crate::theme::UiTheme;
use ratagit_core::AppContext;
use ratagit_core::workflow::{Dialog, Operation, prompt_title};
use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};
use unicode_width::UnicodeWidthStr;

pub(crate) fn render(frame: &mut Frame<'_>, state: &AppContext, area: Rect, theme: &UiTheme) {
    if state.ui.workflow.remote_request.is_some() {
        render_modal(
            frame,
            area,
            ModalSpec::new("Remote branches", ModalTone::Info, 76, 10, 20, 6, 1),
            &[&[("Esc", "cancel")]],
            |frame, content| {
                frame.render_widget(
                    Paragraph::new("Loading remote branches...").style(theme.modal_text_style()),
                    content,
                );
            },
            theme,
        );
        return;
    }
    let Some(dialog) = &state.ui.workflow.dialog else {
        return;
    };
    let (title, tone, height) = match dialog {
        Dialog::Menu { title, choices, .. } => {
            (*title, ModalTone::Info, choices.len().min(16) as u16 + 6)
        }
        Dialog::Prompt { kind, .. } => (prompt_title(*kind), ModalTone::Info, 11),
        Dialog::Confirm(_) => ("Confirm repository change", ModalTone::Danger, 13),
    };
    render_modal(
        frame,
        area,
        ModalSpec::new(title, tone, 88, height, 20, 7, 1),
        &[if matches!(dialog, Dialog::Menu { .. }) {
            &[
                ("j/k ↑/↓", "select"),
                ("Enter", "confirm"),
                ("Esc", "cancel"),
            ]
        } else {
            &[("Enter", "confirm"), ("Esc", "cancel")]
        }],
        |frame, content| match dialog {
            Dialog::Menu {
                title,
                choices,
                selected,
            } => {
                let visible = content.height as usize;
                let start = selected.saturating_sub(visible.saturating_sub(1));
                let lines: Vec<_> = choices
                    .iter()
                    .enumerate()
                    .skip(start)
                    .take(visible)
                    .map(|(index, choice)| {
                        if *title == ratagit_core::custom_patch::OPTIONS_TITLE
                            && let Some((key, label)) = choice.label.split_once("  ")
                        {
                            return command_menu_line(
                                key,
                                label.trim_end_matches(" (Tab returns)"),
                                index == *selected,
                                theme,
                            );
                        }
                        Line::styled(
                            choice.label.clone(),
                            if index == *selected {
                                theme.modal_selected_row_style()
                            } else {
                                theme.modal_text_style()
                            },
                        )
                    })
                    .collect();
                render_menu_lines(frame, content, lines, theme);
            }
            Dialog::Prompt {
                target,
                text,
                cursor,
                ..
            } => {
                let width = content.width.saturating_sub(1) as usize;
                let before = &text[..*cursor];
                let mut start = 0;
                while UnicodeWidthStr::width(&before[start..]) > width {
                    start += before[start..]
                        .chars()
                        .next()
                        .map(char::len_utf8)
                        .unwrap_or(0);
                }
                frame.render_widget(
                    Paragraph::new(vec![
                        Line::from(format!("Target: {target}")),
                        Line::from(""),
                        Line::from(Span::styled(
                            text[start..].to_string(),
                            theme.modal_text_style(),
                        )),
                    ]),
                    content,
                );
                if content.height > 2 && content.width > 0 {
                    frame.set_cursor_position((
                        content.x + UnicodeWidthStr::width(&before[start..]) as u16,
                        content.y + 2,
                    ));
                }
            }
            Dialog::Confirm(operation) => {
                let mut text = format!(
                    "{}\n\n{}\nEsc leaves the repository unchanged.",
                    operation.description(),
                    operation.consequence()
                );
                if let Operation::Resolve { paths, .. } = operation {
                    text.push_str("\n\nTargets:");
                    for path in paths.iter().take(2) {
                        let preview: String = path.chars().take(40).collect();
                        text.push_str(&format!(
                            "\n{preview}{}",
                            if path.chars().count() > 40 { "…" } else { "" }
                        ));
                    }
                    if paths.len() > 2 {
                        text.push_str(&format!("\n… and {} more", paths.len() - 2));
                    }
                }
                frame.render_widget(
                    Paragraph::new(text)
                        .style(theme.modal_text_style())
                        .wrap(Wrap { trim: false }),
                    content,
                );
            }
        },
        theme,
    );
}
