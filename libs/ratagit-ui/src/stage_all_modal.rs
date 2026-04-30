use ratagit_core::{AppContext, StageAllOperation};
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::modal::{ConfirmBody, ModalSpec, ModalTone, render_confirm_body, render_modal};

pub(crate) fn render_stage_all_modal(frame: &mut Frame<'_>, state: &AppContext, area: Rect) {
    if !state.ui.stage_all_confirm.active {
        return;
    }

    let operation = state
        .ui
        .stage_all_confirm
        .context
        .operation
        .as_ref()
        .map(operation_description)
        .unwrap_or_else(|| "Continue the operation".to_string());
    let file_count = state.ui.stage_all_confirm.context.paths.len();
    let details = format!(
        "{} will be staged.",
        if file_count == 1 {
            "1 file".to_string()
        } else {
            format!("{file_count} files")
        }
    );
    let body = ConfirmBody::new("Stage all files and continue?")
        .secondary(operation)
        .details(details);

    render_modal(
        frame,
        area,
        ModalSpec::new("Stage All", ModalTone::Warning, 72, 10, 32, 7, 1),
        &[&[("Enter", "stage all"), ("Esc", "cancel")]],
        |frame, area| render_confirm_body(frame, area, ModalTone::Warning, body),
    );
}

fn operation_description(operation: &StageAllOperation) -> String {
    match operation {
        StageAllOperation::CreateCommit { .. } => "Commit all current file changes".to_string(),
        StageAllOperation::AmendStagedChanges { commit_id } => {
            format!("Amend all current file changes into {commit_id}")
        }
    }
}
