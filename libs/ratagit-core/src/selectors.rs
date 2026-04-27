use crate::{AppState, commit_key, selected_commit};

pub(crate) fn selected_branch_name(state: &AppState) -> Option<String> {
    state
        .branches
        .items
        .get(state.branches.selected)
        .map(|branch| branch.name.clone())
}

pub(crate) fn selected_commit_id(state: &AppState) -> Option<String> {
    selected_commit(&state.commits).map(|commit| commit_key(&commit))
}

pub(crate) fn selected_targets_are_all_staged(state: &AppState, paths: &[String]) -> bool {
    !paths.is_empty()
        && paths
            .iter()
            .all(|path| file_staged(state, path).unwrap_or(false))
}

pub(crate) fn file_staged(state: &AppState, path: &str) -> Option<bool> {
    state
        .files
        .items
        .iter()
        .find(|entry| entry.path == path)
        .map(|entry| entry.staged)
}

pub(crate) fn repository_has_uncommitted_changes(state: &AppState) -> bool {
    !state.files.items.is_empty()
}

pub(crate) fn selected_stash_id(state: &AppState) -> Option<String> {
    state
        .stash
        .items
        .get(state.stash.selected)
        .map(|stash| stash.id.clone())
}
