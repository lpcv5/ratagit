use crate::{
    AppState, CommitFilesPanelState, RepoSnapshot, clamp_commit_selection, clamp_file_selection,
    details, initialize_tree_if_needed, reconcile_after_items_changed,
    reconcile_commits_after_items_changed,
};

pub(crate) fn apply_snapshot(state: &mut AppState, snapshot: RepoSnapshot) {
    state.status.summary = snapshot.status_summary;
    state.status.current_branch = snapshot.current_branch;
    state.status.detached_head = snapshot.detached_head;
    state.files.items = snapshot.files;
    initialize_tree_if_needed(&mut state.files);
    reconcile_after_items_changed(&mut state.files);
    state.commits.items = snapshot.commits;
    state.commits.files = CommitFilesPanelState::default();
    state.commits.has_more = state.commits.items.len() >= crate::COMMITS_PAGE_SIZE;
    state.commits.loading_more = false;
    state.commits.pending_select_after_load = false;
    state.commits.pagination_epoch = state.commits.pagination_epoch.wrapping_add(1);
    reconcile_commits_after_items_changed(&mut state.commits);
    state.branches.items = snapshot.branches;
    state.stash.items = snapshot.stashes;
    clamp_selection_indexes(state);
    details::reset_after_snapshot(state);
    state.search.clear();
}

pub(crate) fn clamp_selection_indexes(state: &mut AppState) {
    clamp_file_selection(&mut state.files);
    clamp_commit_selection(&mut state.commits);
    state.branches.selected = clamp_index(state.branches.selected, state.branches.items.len());
    state.stash.selected = clamp_index(state.stash.selected, state.stash.items.len());
}

fn clamp_index(index: usize, len: usize) -> usize {
    if len == 0 { 0 } else { index.min(len - 1) }
}
