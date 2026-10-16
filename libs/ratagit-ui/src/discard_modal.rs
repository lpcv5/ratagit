use ratagit_core::AppContext;
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::modal::{ConfirmBody, ModalSpec, ModalTone, render_confirm_body, render_modal};
use crate::theme::UiTheme;

pub(crate) fn render_discard_modal(
    frame: &mut Frame<'_>,
    state: &AppContext,
    area: Rect,
    theme: &UiTheme,
) {
    let partial = state.ui.details.patch.discard.as_ref();
    let operation = state.ui.details.repository_confirm.as_ref();
    if !state.ui.discard_confirm.active && partial.is_none() && operation.is_none() {
        return;
    }

    render_modal(
        frame,
        area,
        ModalSpec::new("Confirm", ModalTone::Danger, 72, 12, 20, 8, 1),
        &[&[
            (
                "Enter",
                if operation.is_some() {
                    "drop"
                } else {
                    "discard"
                },
            ),
            ("Esc", "cancel"),
        ]],
        |frame, content| {
            render_confirm_body(
                frame,
                content,
                ModalTone::Danger,
                ConfirmBody::new(if operation.is_some() {
                    "Drop selected stash?"
                } else if partial.is_some() {
                    "Discard selected lines?"
                } else {
                    "Discard selected file changes?"
                })
                .secondary("This action cannot be undone.")
                .details(
                    operation
                        .map(|op| format!("{op:?}"))
                        .or_else(|| {
                            partial.map(|ids| format!("{} reviewed changed lines", ids.len()))
                        })
                        .unwrap_or_else(|| {
                            format_discard_details(&state.ui.discard_confirm.context)
                        }),
                ),
                theme,
            );
        },
        theme,
    );
}

fn format_discard_details(paths: &[String]) -> String {
    format!(
        "Targets: {}\n{}",
        format_target_count(paths),
        format_target_paths(paths)
    )
}

fn format_target_count(paths: &[String]) -> String {
    match paths {
        [] => "0 files".to_string(),
        [_] => "1 file".to_string(),
        _ => format!("{} files", paths.len()),
    }
}

fn format_target_paths(paths: &[String]) -> String {
    if paths.is_empty() {
        return "No targets selected.".to_string();
    }

    let mut lines = paths
        .iter()
        .take(4)
        .map(|path| format!("- {path}"))
        .collect::<Vec<_>>();
    if paths.len() > lines.len() {
        lines.push(format!("... and {} more", paths.len() - lines.len()));
    }
    lines.join("\n")
}
