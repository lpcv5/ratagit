use super::panel_types::{PanelLine, PanelSpan};
use crate::theme::RowRole;
use ratagit_core::{
    AppContext,
    partial::{patch_rows, selected_ids},
    patch::PatchLineKind,
};

pub(crate) fn render(state: &AppContext, max_lines: usize) -> Vec<PanelLine> {
    if max_lines == 0 {
        return Vec::new();
    }
    let ui = &state.ui.details.patch;
    let mut lines = vec![PanelLine::new(
        format!(
            "{:?} | {} | a:mode v:range Tab:side Space:apply d:discard Esc:back",
            ui.mode,
            if ui.staged { "staged" } else { "unstaged" }
        ),
        RowRole::Notice,
    )];
    if state.work.details.patch_request.is_some() {
        lines[0] = PanelLine::new("Loading reviewed patch...", RowRole::Notice);
    }
    if state.ui.last_left_focus == ratagit_core::PanelFocus::Stash {
        lines[0] = PanelLine::new(
            "Stash diff (read only) | j/k:select Esc:back",
            RowRole::Notice,
        );
    }
    if let Some(error) = &state.repo.details.files_error {
        lines.push(PanelLine::new(error, RowRole::Error));
    }
    if let Some(patch) = &state.repo.details.patch {
        let selected: std::collections::HashSet<_> = selected_ids(state).into_iter().collect();
        let rows = patch_rows(patch);
        if rows.is_empty() {
            lines.push(PanelLine::new("No remaining changes", RowRole::Muted));
        }
        lines.extend(
            rows.into_iter()
                .skip(state.ui.details.scroll_offset)
                .take(max_lines.saturating_sub(lines.len()))
                .map(|row| {
                    let role = match row.kind {
                        PatchLineKind::Added => RowRole::DiffAdd,
                        PatchLineKind::Removed => RowRole::DiffRemove,
                        _ => RowRole::DiffMeta,
                    };
                    let chosen = row.id.is_some_and(|id| selected.contains(&id));
                    if chosen {
                        PanelLine::from_spans(
                            vec![PanelSpan {
                                text: row.text,
                                style: crate::theme::selected_row_style(),
                            }],
                            role,
                        )
                        .selected(true)
                    } else {
                        PanelLine::new(row.text, role)
                    }
                }),
        );
    }
    lines.truncate(max_lines);
    lines
}
