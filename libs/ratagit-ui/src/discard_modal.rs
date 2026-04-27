use ratagit_core::AppContext;
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::modal::{ConfirmBody, ModalSpec, ModalTone, render_confirm_body, render_modal};

pub(crate) fn render_discard_modal(frame: &mut Frame<'_>, state: &AppContext, area: Rect) {
    if !state.ui.discard_confirm.active {
        return;
    }

    render_modal(
        frame,
        area,
        ModalSpec::new("Confirm", ModalTone::Danger, 72, 12, 20, 8, 1),
        &[&[("Enter", "discard"), ("Esc", "cancel")]],
        |frame, content| {
            render_confirm_body(
                frame,
                content,
                ModalTone::Danger,
                ConfirmBody::new("Discard selected file changes?")
                    .secondary("This action cannot be undone.")
                    .details(format_discard_details(&state.ui.discard_confirm.paths)),
            );
        },
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
