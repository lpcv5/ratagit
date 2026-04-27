use crate::actions::with_pending;
use crate::selectors::{file_staged, selected_targets_are_all_staged};
use crate::{
    AppState, Command, FileInputMode, ResetChoice, StashScope, branches, enter_multi_select,
    leave_multi_select, push_notice, selected_target_paths, toggle_current_row_selection,
};

pub(crate) fn toggle_selected_file_stage(state: &mut AppState) -> Vec<Command> {
    let paths = selected_target_paths(&state.files);
    if paths.is_empty() {
        push_notice(state, "No file selected");
        return Vec::new();
    }
    if selected_targets_are_all_staged(state, &paths) {
        with_pending(state, vec![Command::UnstageFiles { paths }])
    } else {
        let unstaged_paths = paths
            .into_iter()
            .filter(|path| file_staged(state, path) == Some(false))
            .collect::<Vec<_>>();
        with_pending(
            state,
            vec![Command::StageFiles {
                paths: unstaged_paths,
            }],
        )
    }
}

pub(crate) fn toggle_files_multi_select(state: &mut AppState) {
    if state.files.mode == FileInputMode::MultiSelect {
        leave_multi_select(&mut state.files);
    } else {
        enter_multi_select(&mut state.files);
    }
}

pub(crate) fn toggle_current_file_selection(state: &mut AppState) {
    toggle_current_row_selection(&mut state.files);
}

pub(crate) fn stage_selected_file(state: &mut AppState) -> Vec<Command> {
    let paths = selected_target_paths(&state.files)
        .into_iter()
        .filter(|path| file_staged(state, path) == Some(false))
        .collect::<Vec<_>>();
    if !paths.is_empty() {
        with_pending(state, vec![Command::StageFiles { paths }])
    } else {
        push_notice(state, "No unstaged file selected");
        Vec::new()
    }
}

pub(crate) fn unstage_selected_file(state: &mut AppState) -> Vec<Command> {
    let paths = selected_target_paths(&state.files)
        .into_iter()
        .filter(|path| file_staged(state, path) == Some(true))
        .collect::<Vec<_>>();
    if !paths.is_empty() {
        with_pending(state, vec![Command::UnstageFiles { paths }])
    } else {
        push_notice(state, "No staged file selected");
        Vec::new()
    }
}

pub(crate) fn stash_selected_files(state: &mut AppState) -> Vec<Command> {
    let paths = selected_target_paths(&state.files);
    if paths.is_empty() {
        push_notice(state, "No file selected");
        Vec::new()
    } else {
        with_pending(
            state,
            vec![Command::StashFiles {
                message: "savepoint".to_string(),
                paths,
            }],
        )
    }
}

pub(crate) fn stash_scope_for_current_files_selection(state: &AppState) -> StashScope {
    if state.files.mode == FileInputMode::MultiSelect {
        let paths = selected_target_paths(&state.files);
        if !paths.is_empty() {
            return StashScope::SelectedPaths(paths);
        }
    }
    StashScope::All
}

pub(crate) fn open_reset_menu(state: &mut AppState) {
    state.editor.kind = None;
    close_discard_confirm(state);
    branches::close_popovers(state);
    state.reset_menu.active = true;
    state.reset_menu.selected = ResetChoice::Mixed;
}

pub(crate) fn confirm_reset_menu(state: &mut AppState) -> Vec<Command> {
    if !state.reset_menu.active {
        return Vec::new();
    }
    let choice = state.reset_menu.selected;
    state.reset_menu.active = false;
    if choice == ResetChoice::Nuke {
        with_pending(state, vec![Command::Nuke])
    } else if let Some(mode) = choice.reset_mode() {
        with_pending(state, vec![Command::Reset { mode }])
    } else {
        Vec::new()
    }
}

pub(crate) fn open_discard_confirm(state: &mut AppState) {
    let paths = selected_target_paths(&state.files);
    if paths.is_empty() {
        push_notice(state, "No file selected");
        return;
    }

    state.editor.kind = None;
    state.reset_menu.active = false;
    branches::close_popovers(state);
    state.discard_confirm.active = true;
    state.discard_confirm.paths = paths;
}

pub(crate) fn confirm_discard(state: &mut AppState) -> Vec<Command> {
    if !state.discard_confirm.active {
        return Vec::new();
    }
    let paths = state.discard_confirm.paths.clone();
    close_discard_confirm(state);
    if paths.is_empty() {
        push_notice(state, "No file selected");
        Vec::new()
    } else {
        with_pending(state, vec![Command::DiscardFiles { paths }])
    }
}

pub(crate) fn close_discard_confirm(state: &mut AppState) {
    state.discard_confirm.active = false;
    state.discard_confirm.paths.clear();
}
