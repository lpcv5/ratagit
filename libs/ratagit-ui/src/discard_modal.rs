use ratagit_core::AppState;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Paragraph, Wrap};

use crate::modal::{
    ModalSpec, ModalTone, render_action_footer, render_modal_frame, render_muted_text,
    render_section_label, render_warning_text,
};

pub(crate) fn render_discard_modal(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    if !state.discard_confirm.active {
        return;
    }

    let Some(modal) = render_modal_frame(
        frame,
        area,
        ModalSpec::new("Discard Changes", ModalTone::Danger, 72, 12, 20, 8, 1),
    ) else {
        return;
    };
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(2),
        ])
        .split(modal.content);

    render_warning_text(
        frame,
        rows[0],
        ModalTone::Danger,
        "Discard selected file changes?",
    );
    render_section_label(
        frame,
        rows[1],
        format!(
            "Targets: {}",
            format_target_count(&state.discard_confirm.paths)
        ),
    );
    frame.render_widget(
        Paragraph::new(format_target_paths(&state.discard_confirm.paths))
            .wrap(Wrap { trim: false }),
        rows[2],
    );
    render_muted_text(
        frame,
        rows[3],
        "This removes tracked changes and deletes untracked targets.",
    );
    if let Some(footer) = modal.footer {
        render_action_footer(
            frame,
            footer,
            ModalTone::Danger,
            &[("Enter", "discard"), ("Esc", "cancel")],
        );
    }
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
