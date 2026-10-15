use super::*;
use std::collections::BTreeSet;

pub fn groups(patch: &Patch, mode: SelectionMode, staged: bool) -> Vec<Vec<LineId>> {
    let mut result = Vec::new();
    for (file, entry) in patch.files.iter().enumerate() {
        if (entry.source == PatchSource::Index) != staged {
            continue;
        }
        for (hunk, entry) in entry.hunks.iter().enumerate() {
            let mut group = Vec::new();
            for (line, entry) in entry.lines.iter().enumerate() {
                if entry.kind == PatchLineKind::NoNewline {
                    continue;
                }
                let id = LineId { file, hunk, line };
                if mode == SelectionMode::Line {
                    result.push(vec![id]);
                } else if entry.kind != PatchLineKind::Context {
                    group.push(id);
                } else if mode == SelectionMode::Block && !group.is_empty() {
                    result.push(std::mem::take(&mut group));
                }
            }
            if !group.is_empty() {
                result.push(group);
            }
        }
    }
    // Combined previews may interleave index and worktree coordinates.
    if let Some(rows) = &patch.preview {
        let positions: std::collections::HashMap<_, _> = rows
            .iter()
            .enumerate()
            .filter_map(|(row, entry)| entry.id.map(|id| (id, row)))
            .collect();
        if mode == SelectionMode::Line {
            result.retain(|group| positions.contains_key(&group[0]));
        }
        result.sort_by_key(|group| positions.get(&group[0]).copied().unwrap_or(usize::MAX));
    }
    result
}

pub fn marked_indices(state: &AppContext) -> BTreeSet<usize> {
    let ui = &state.ui.details.patch;
    if let Some(anchor) = ui.anchor {
        let cursor = ui.selected[usize::from(ui.staged)];
        (anchor.min(cursor)..=anchor.max(cursor)).collect()
    } else {
        ui.marked.clone()
    }
}

fn ids_at(state: &AppContext, indices: BTreeSet<usize>) -> Vec<LineId> {
    let Some(patch) = &state.repo.details.patch else {
        return Vec::new();
    };
    let ui = &state.ui.details.patch;
    let groups = groups(patch, ui.mode, ui.staged);
    indices
        .into_iter()
        .filter_map(|index| groups.get(index))
        .flatten()
        .copied()
        .collect()
}

pub fn cursor_ids(state: &AppContext) -> Vec<LineId> {
    ids_at(
        state,
        BTreeSet::from([
            state.ui.details.patch.selected[usize::from(state.ui.details.patch.staged)]
        ]),
    )
}

pub fn marked_ids(state: &AppContext) -> Vec<LineId> {
    ids_at(state, marked_indices(state))
}

pub fn selected_ids(state: &AppContext) -> Vec<LineId> {
    let marked = marked_indices(state);
    let ids = if marked.is_empty() {
        cursor_ids(state)
    } else {
        ids_at(state, marked)
    };
    let Some(patch) = &state.repo.details.patch else {
        return Vec::new();
    };
    ids.into_iter()
        .filter(|id| {
            matches!(
                patch.files[id.file].hunks[id.hunk].lines[id.line].kind,
                PatchLineKind::Added | PatchLineKind::Removed
            )
        })
        .collect()
}

pub(crate) fn clear_marks(state: &mut AppContext) {
    state.ui.details.patch.anchor = None;
    state.ui.details.patch.marked.clear();
}

pub(crate) fn toggle_mark(state: &mut AppContext) {
    let marked = marked_indices(state);
    let ui = &mut state.ui.details.patch;
    ui.anchor = None;
    ui.marked = marked;
    let cursor = ui.selected[usize::from(ui.staged)];
    if !ui.marked.remove(&cursor) {
        ui.marked.insert(cursor);
    }
}

pub(crate) fn toggle_range(state: &mut AppContext) {
    if state.ui.details.patch.anchor.is_some() {
        let marked = marked_indices(state);
        clear_marks(state);
        state.ui.details.patch.marked = marked;
    } else {
        clear_marks(state);
        state.ui.details.patch.anchor =
            Some(state.ui.details.patch.selected[usize::from(state.ui.details.patch.staged)]);
    }
}
