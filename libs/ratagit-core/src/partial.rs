//! Partial staging state transitions. All Git I/O is returned as commands.
pub use crate::patch::PatchRow;
use crate::patch::{LineId, Patch, PatchDirection, PatchLineKind, PatchRequest, PatchSource};
use crate::{AppContext, Command, FileRowKind, PanelFocus, push_notice, with_pending};

mod position;
mod rows;
pub use position::PatchPosition;
mod selection;
mod viewport;
pub use rows::patch_rows;
pub(crate) use selection::{clear_marks, toggle_mark, toggle_range};
pub use selection::{cursor_ids, groups, marked_ids, marked_indices, selected_ids};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectionMode {
    #[default]
    Block,
    Hunk,
    Line,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PatchUiState {
    pub scroll_offsets: [usize; 2],
    pub active: bool,
    pub mode: SelectionMode,
    pub staged: bool,
    pub selected: [usize; 2],
    pub anchor: Option<usize>,
    pub marked: std::collections::BTreeSet<usize>,
    pub discard: Option<Vec<LineId>>,
    pub write_position: Option<PatchPosition>,
}

pub(crate) fn open(state: &mut AppContext) -> Vec<Command> {
    if state.work.mutation.operation_pending.is_some() {
        return Vec::new();
    }
    let Some(row) = crate::selected_row(&state.repo.files.items, &state.ui.files) else {
        return Vec::new();
    };
    if row.kind == FileRowKind::Directory {
        crate::toggle_selected_directory(&state.repo.files.items, &mut state.ui.files);
        return crate::details::refresh_files_details(state);
    }
    state.ui.details.patch = PatchUiState {
        active: true,
        ..PatchUiState::default()
    };
    state.ui.focus = PanelFocus::Details;
    state.ui.last_left_focus = PanelFocus::Files;
    state.ui.details.scroll_offset = 0;
    state.repo.details.patch = None;
    crate::details::clear_details_pending(state);
    load(state, PatchRequest::File(row.path.into()))
}

fn load(state: &mut AppContext, request: PatchRequest) -> Vec<Command> {
    let id = state.work.details.next_patch_request_id;
    state.work.details.next_patch_request_id = id.wrapping_add(1);
    state.work.details.patch_request = Some((id, request.clone()));
    vec![Command::LoadPatch {
        request_id: id,
        request,
    }]
}

pub(crate) fn close(state: &mut AppContext) {
    state.ui.details.patch = PatchUiState::default();
    state.repo.details.patch = None;
    state.work.details.patch_request = None;
    state.ui.focus = state.ui.last_left_focus;
    state.ui.details.scroll_offset = 0;
}

pub(crate) fn preview_stash(state: &mut AppContext) -> Vec<Command> {
    let Some(id) = crate::selected_stash_id(state) else {
        state.ui.details.patch.active = false;
        state.repo.details.patch = None;
        state.work.details.patch_request = None;
        return Vec::new();
    };
    let Some(index) = id
        .strip_prefix("stash@{")
        .and_then(|id| id.strip_suffix('}'))
        .and_then(|id| id.parse().ok())
    else {
        return Vec::new();
    };
    let request = PatchRequest::Stash { index, path: None };
    state.ui.details.patch.active = true;
    state.repo.details.patch = None;
    state.ui.details.scroll_offset = 0;
    load(state, request)
}

pub(crate) fn open_stash(state: &mut AppContext) -> Vec<Command> {
    let commands = preview_stash(state);
    if state.ui.details.patch.active {
        state.ui.focus = PanelFocus::Details;
        state.ui.last_left_focus = PanelFocus::Stash;
    }
    commands
}

pub(crate) fn loaded(
    state: &mut AppContext,
    id: u64,
    request: PatchRequest,
    result: Result<Box<Patch>, String>,
) -> Vec<Command> {
    if !state.ui.details.patch.active
        || state.work.details.patch_request.as_ref() != Some(&(id, request.clone()))
    {
        return Vec::new();
    }
    state.work.details.patch_request = None;
    match result {
        Ok(patch) if patch.request == request => {
            let first_load = state.repo.details.patch.is_none();
            if state.repo.details.patch.as_deref() != Some(patch.as_ref()) {
                clear_marks(state);
            }
            state.repo.details.patch = Some(patch);
            state.repo.details.files_error = None;
            position::restore(state);
            let patch = state.repo.details.patch.as_ref().unwrap();
            for staged in [false, true] {
                let len = groups(patch, state.ui.details.patch.mode, staged).len();
                let selected = &mut state.ui.details.patch.selected[usize::from(staged)];
                *selected = (*selected).min(len.saturating_sub(1));
            }
            if first_load && selected_ids(state).is_empty() {
                state.ui.details.patch.staged = !state.ui.details.patch.staged;
            }
            let rows = patch_rows(state.repo.details.patch.as_ref().unwrap()).len();
            state.ui.details.scroll_offset = state
                .ui
                .details
                .scroll_offset
                .min(rows.saturating_sub(state.ui.details.viewport_lines.max(1)));
            ensure_visible(state);
        }
        Ok(_) => push_notice(state, "Patch response target mismatch"),
        Err(error) => {
            state.ui.details.patch.write_position = None;
            state.repo.details.patch = None;
            state.repo.details.files_error = Some(error.clone());
            push_notice(state, &error);
        }
    }
    Vec::new()
}

pub(crate) fn move_selection(state: &mut AppContext, up: bool, visible_lines: usize) {
    let Some(patch) = &state.repo.details.patch else {
        return;
    };
    let ui = &mut state.ui.details.patch;
    let groups = groups(patch, ui.mode, ui.staged);
    let selected = &mut ui.selected[usize::from(ui.staged)];
    let previous = *selected;
    *selected = if up {
        selected.saturating_sub(1)
    } else {
        selected
            .saturating_add(1)
            .min(groups.len().saturating_sub(1))
    };
    if *selected != previous {
        ui.write_position = None;
        viewport::scroll_to_cursor(state, visible_lines);
    }
}

pub(crate) fn cycle_mode(state: &mut AppContext) {
    let mode = match state.ui.details.patch.mode {
        SelectionMode::Block => SelectionMode::Line,
        SelectionMode::Hunk => SelectionMode::Line,
        SelectionMode::Line => SelectionMode::Block,
    };
    set_mode(state, mode);
}

fn set_mode(state: &mut AppContext, mode: SelectionMode) {
    let Some(patch) = &state.repo.details.patch else {
        return;
    };
    let ui = &mut state.ui.details.patch;
    ui.write_position = None;
    for staged in [false, true] {
        let old = groups(patch, ui.mode, staged);
        let id = old
            .get(ui.selected[usize::from(staged)])
            .and_then(|ids| ids.first());
        let next = groups(patch, mode, staged);
        ui.selected[usize::from(staged)] = id
            .and_then(|id| next.iter().position(|ids| ids.contains(id)))
            .unwrap_or(0);
    }
    ui.mode = mode;
    ui.anchor = None;
    ui.marked.clear();
    ensure_visible(state);
}

pub(crate) fn switch_source(state: &mut AppContext) {
    let ui = &mut state.ui.details.patch;
    ui.write_position = None;
    ui.scroll_offsets[usize::from(ui.staged)] = state.ui.details.scroll_offset;
    state.ui.details.patch.staged = !state.ui.details.patch.staged;
    state.ui.details.patch.anchor = None;
    state.ui.details.patch.marked.clear();
    state.ui.details.scroll_offset =
        state.ui.details.patch.scroll_offsets[usize::from(state.ui.details.patch.staged)];
    ensure_visible(state);
}

pub(crate) fn ensure_visible(state: &mut AppContext) {
    if !state.ui.details.patch.active || state.ui.details.viewport_lines == 0 {
        return;
    }
    viewport::scroll_to_cursor(state, state.ui.details.viewport_lines);
}

pub(crate) fn next_file(state: &mut AppContext, up: bool) -> Vec<Command> {
    if state.ui.last_left_focus != PanelFocus::Files {
        return Vec::new();
    }
    close(state);
    let original = state.ui.files.selected;
    loop {
        let before = state.ui.files.selected;
        crate::move_selected(&mut state.ui.files, up);
        if before == state.ui.files.selected {
            state.ui.files.selected = original;
            break;
        }
        if crate::selected_row(&state.repo.files.items, &state.ui.files)
            .is_some_and(|row| row.kind == FileRowKind::File)
        {
            break;
        }
    }
    open(state)
}

pub(crate) fn reload(state: &mut AppContext) -> Vec<Command> {
    if !state.ui.details.patch.active {
        return Vec::new();
    }
    if let Some(patch) = &state.repo.details.patch {
        return load(state, patch.request.clone());
    }
    Vec::new()
}

pub(crate) fn request_discard(state: &mut AppContext) {
    if state.ui.details.patch.staged
        || state.work.details.patch_request.is_some()
        || state.work.mutation.operation_pending.is_some()
    {
        return;
    }
    let ids = selected_ids(state);
    if let Some(patch) = &state.repo.details.patch {
        if let Err(error) = patch.validate_lines(&ids, PatchDirection::Discard) {
            push_notice(state, &error);
        } else {
            state.ui.details.patch.discard = Some(ids);
        }
    }
}

pub(crate) fn apply(state: &mut AppContext, discard: bool) -> Vec<Command> {
    if !state.ui.details.patch.active
        || state.work.details.patch_request.is_some()
        || state.work.mutation.operation_pending.is_some()
    {
        return Vec::new();
    }
    let ids = if discard {
        state.ui.details.patch.discard.take().unwrap_or_default()
    } else {
        selected_ids(state)
    };
    let direction = if discard {
        PatchDirection::Discard
    } else if state.ui.details.patch.staged {
        PatchDirection::Unstage
    } else {
        PatchDirection::Stage
    };
    let Some(patch) = &state.repo.details.patch else {
        return Vec::new();
    };
    if let Err(error) = patch.validate_lines(&ids, direction) {
        push_notice(state, &error);
        return Vec::new();
    }
    let patch = patch.clone();
    state.ui.details.patch.write_position = position::capture(state, direction, &ids);
    with_pending(
        state,
        vec![Command::ApplyPatch {
            patch,
            ids,
            direction,
        }],
    )
}

pub(crate) fn applied(
    state: &mut AppContext,
    direction: PatchDirection,
    result: Result<(), String>,
) -> Vec<Command> {
    state.work.record_operation_completed("apply_patch");
    state.last_operation = Some(format!("partial_{direction:?}").to_lowercase());
    state.ui.details.patch.anchor = None;
    state.ui.details.patch.marked.clear();
    crate::details::clear_caches(state);
    match result {
        Ok(()) => {
            state.repo.status.last_error = None;
            push_notice(state, &format!("Partial {direction:?} completed"));
        }
        Err(error) => {
            state.ui.details.patch.write_position = None;
            state.repo.status.last_error = Some(error.clone());
            push_notice(state, &error);
        }
    }
    with_pending(state, vec![Command::RefreshFiles])
}

#[cfg(test)]
mod tests;
