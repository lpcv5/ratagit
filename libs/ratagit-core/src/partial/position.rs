//! Cursor relocation: source-local hunk IDs change after index writes.
use super::*;
use crate::patch::PatchHunk;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchPosition {
    path: PathBuf,
    staged: bool,
    start: u32,
    changes: Vec<(PatchLineKind, Vec<u8>)>,
    row: usize,
    scroll: usize,
}

pub(super) fn capture(
    state: &AppContext,
    direction: PatchDirection,
    ids: &[LineId],
) -> Option<PatchPosition> {
    if state.ui.details.patch.mode == SelectionMode::Line {
        return None;
    }
    let patch = state.repo.details.patch.as_ref()?;
    let cursor = cursor_ids(state);
    let id = *cursor.first()?;
    let file = &patch.files[id.file];
    let selected: std::collections::HashSet<_> = ids.iter().copied().collect();
    let applied = cursor.iter().any(|id| selected.contains(id));
    if applied && direction == PatchDirection::Discard {
        return None;
    }
    let staged = if applied {
        direction == PatchDirection::Stage
    } else {
        file.source == PatchSource::Index
    };
    let changes = cursor
        .into_iter()
        .map(|id| {
            let line = &patch.files[id.file].hunks[id.hunk].lines[id.line];
            (line.kind, line.content.clone())
        })
        .collect();
    Some(PatchPosition {
        path: file.new.path.as_ref().or(file.old.path.as_ref())?.clone(),
        staged,
        start: index_coordinate(
            &file.hunks[id.hunk],
            file.source == PatchSource::Index,
            id.line,
        ),
        changes,
        row: patch_rows(patch)
            .iter()
            .position(|row| row.id == Some(id))?,
        scroll: state.ui.details.scroll_offset,
    })
}

pub(super) fn restore(state: &mut AppContext) {
    let Some(position) = state.ui.details.patch.write_position.take() else {
        return;
    };
    let Some(patch) = &state.repo.details.patch else {
        return;
    };
    let mut matches = Vec::new();
    for (file, entry) in patch.files.iter().enumerate() {
        if (entry.source == PatchSource::Index) != position.staged
            || entry.new.path.as_ref().or(entry.old.path.as_ref()) != Some(&position.path)
        {
            continue;
        }
        for (hunk, entry) in entry.hunks.iter().enumerate() {
            let changed: Vec<_> = entry
                .lines
                .iter()
                .enumerate()
                .filter(|(_, line)| {
                    matches!(line.kind, PatchLineKind::Added | PatchLineKind::Removed)
                })
                .collect();
            for window in changed.windows(position.changes.len().max(1)) {
                if window.len() != position.changes.len()
                    || !window
                        .iter()
                        .zip(&position.changes)
                        .all(|((_, line), (kind, bytes))| {
                            line.kind == *kind && line.content == *bytes
                        })
                    || entry.lines[window[0].0..=window[window.len() - 1].0]
                        .iter()
                        .any(|line| line.kind == PatchLineKind::Context)
                {
                    continue;
                }
                let line = window[0].0;
                matches.push((
                    index_coordinate(entry, position.staged, line).abs_diff(position.start),
                    LineId { file, hunk, line },
                ));
            }
        }
    }
    let Some((_, cursor)) = matches.into_iter().min_by_key(|(distance, _)| *distance) else {
        return;
    };
    let Some(index) = groups(patch, state.ui.details.patch.mode, position.staged)
        .iter()
        .position(|group| group.contains(&cursor))
    else {
        return;
    };
    let Some(row) = patch_rows(patch)
        .iter()
        .position(|row| row.id == Some(cursor))
    else {
        return;
    };
    state.ui.details.patch.staged = position.staged;
    state.ui.details.patch.selected[usize::from(position.staged)] = index;
    state.ui.details.scroll_offset = if row >= position.row {
        position.scroll.saturating_add(row - position.row)
    } else {
        position.scroll.saturating_sub(position.row - row)
    };
}

fn index_coordinate(hunk: &PatchHunk, staged: bool, line: usize) -> u32 {
    let (start, included) = if staged {
        (hunk.new_start, PatchLineKind::Added)
    } else {
        (hunk.old_start, PatchLineKind::Removed)
    };
    start.saturating_add(
        hunk.lines[..line]
            .iter()
            .filter(|line| line.kind == PatchLineKind::Context || line.kind == included)
            .count() as u32,
    )
}
