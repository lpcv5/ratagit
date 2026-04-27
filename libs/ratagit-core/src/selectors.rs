use crate::{AppContext, commit_key, selected_commit};

pub(crate) fn selected_branch_name(state: &AppContext) -> Option<String> {
    state
        .repo
        .branches
        .items
        .get(state.ui.branches.selected)
        .map(|branch| branch.name.clone())
}

pub(crate) fn selected_commit_id(state: &AppContext) -> Option<String> {
    selected_commit(&state.repo.commits.items, &state.ui.commits).map(|commit| commit_key(&commit))
}

pub(crate) fn selected_targets_are_all_staged(state: &AppContext, paths: &[String]) -> bool {
    !paths.is_empty()
        && paths
            .iter()
            .all(|path| file_staged(state, path).unwrap_or(false))
}

pub(crate) fn file_staged(state: &AppContext, path: &str) -> Option<bool> {
    state
        .repo
        .files
        .items
        .iter()
        .find(|entry| entry.path == path)
        .map(|entry| entry.staged)
}

pub(crate) fn repository_has_uncommitted_changes(state: &AppContext) -> bool {
    !state.repo.files.items.is_empty()
}

pub(crate) fn selected_stash_id(state: &AppContext) -> Option<String> {
    state
        .repo
        .stash
        .items
        .get(state.ui.stash.selected)
        .map(|stash| stash.id.clone())
}
