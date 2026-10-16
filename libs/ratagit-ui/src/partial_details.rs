use super::diff_rows::{self, DiffRow};
use super::panel_types::PanelLine;
use crate::theme::{RowRole, UiTheme};
use ratagit_core::{
    AppContext, PanelFocus,
    partial::{SelectionMode, cursor_ids, marked_ids, patch_rows},
    patch::{PatchLineKind, PatchSource},
};

pub(crate) fn render(state: &AppContext, max_lines: usize, theme: &UiTheme) -> Vec<PanelLine> {
    if max_lines == 0 {
        return Vec::new();
    }
    let ui = &state.ui.details.patch;
    let mut lines = Vec::new();
    if let Some(error) = &state.repo.details.files_error {
        lines.push(PanelLine::new(error, RowRole::Error));
    }
    if let Some(patch) = state.repo.details.patch.as_deref() {
        let ready = state.ui.focus == PanelFocus::Details
            && state.work.details.patch_request.is_none()
            && state.work.mutation.operation_pending.is_none();
        let cursor: std::collections::HashSet<_> = cursor_ids(state).into_iter().collect();
        let marked: std::collections::HashSet<_> = marked_ids(state).into_iter().collect();
        let rows = patch_rows(patch);
        let first_cursor = rows
            .iter()
            .find_map(|r| r.id.filter(|id| cursor.contains(id)));
        if rows.is_empty() {
            lines.push(PanelLine::new("No remaining changes", RowRole::Muted));
        }
        let rows: Vec<_> = rows
            .into_iter()
            .map(|row| {
                let highlight = ui.mode == SelectionMode::Line
                    || matches!(row.kind, PatchLineKind::Added | PatchLineKind::Removed);
                let focused = ready && row.id.is_some_and(|id| cursor.contains(&id));
                DiffRow {
                    staged: row
                        .id
                        .is_some_and(|id| patch.files[id.file].source == PatchSource::Index),
                    focused: focused && highlight,
                    chosen: ready && highlight && row.id.is_some_and(|id| marked.contains(&id)),
                    included: row.id.is_some_and(|id| {
                        matches!(row.kind, PatchLineKind::Added | PatchLineKind::Removed)
                            && ratagit_core::custom_patch::line_included(
                                state,
                                &patch.files[id.file],
                                id.hunk,
                                id.line,
                            )
                    }),
                    marker: focused && (ui.mode == SelectionMode::Line || row.id == first_cursor),
                    row,
                }
            })
            .collect();
        lines.extend(diff_rows::render(
            &rows,
            state.ui.details.scroll_offset,
            max_lines.saturating_sub(lines.len()),
            theme,
        ));
    } else if state.work.details.patch_request.is_some() {
        lines.push(PanelLine::new("Loading reviewed patch...", RowRole::Notice));
    }
    lines.truncate(max_lines);
    lines
}
