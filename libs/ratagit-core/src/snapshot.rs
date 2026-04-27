use crate::{
    AppState, BranchEntry, CommitEntry, CommitFilesPanelState, FilesSnapshot, RepoSnapshot,
    StashEntry, clamp_commit_selection, clamp_file_selection, details,
    initialize_tree_with_initial_expansion, mark_file_items_changed, reconcile_after_items_changed,
    reconcile_commits_after_items_changed,
};

pub(crate) fn apply_snapshot(state: &mut AppState, snapshot: RepoSnapshot) {
    apply_files_snapshot(
        state,
        FilesSnapshot {
            status_summary: snapshot.status_summary,
            current_branch: snapshot.current_branch,
            detached_head: snapshot.detached_head,
            files: snapshot.files,
            index_entry_count: 0,
            large_repo_mode: false,
            status_truncated: false,
            status_scan_skipped: false,
            untracked_scan_skipped: false,
        },
    );
    apply_commits_snapshot(state, snapshot.commits);
    apply_branches_snapshot(state, snapshot.branches);
    apply_stashes_snapshot(state, snapshot.stashes);
    details::reset_after_snapshot(state);
    state.search.clear();
}

pub(crate) fn apply_files_snapshot(state: &mut AppState, snapshot: FilesSnapshot) {
    let was_large_repo_mode = state.status.large_repo_mode;
    state.status.summary = snapshot.status_summary;
    state.status.current_branch = snapshot.current_branch;
    state.status.detached_head = snapshot.detached_head;
    state.status.index_entry_count = snapshot.index_entry_count;
    state.status.large_repo_mode = snapshot.large_repo_mode;
    state.status.status_truncated = snapshot.status_truncated;
    state.status.status_scan_skipped = snapshot.status_scan_skipped;
    state.status.untracked_scan_skipped = snapshot.untracked_scan_skipped;
    state.files.items = snapshot.files;
    mark_file_items_changed(&mut state.files);
    initialize_tree_with_initial_expansion(&mut state.files, !snapshot.large_repo_mode);
    if snapshot.status_scan_skipped {
        state.files.expanded_dirs.clear();
        state.files.lightweight_tree_projection = true;
        state.files.tree_initialized = true;
        crate::refresh_tree_projection(&mut state.files);
    } else if snapshot.large_repo_mode && !was_large_repo_mode {
        state.files.expanded_dirs.clear();
        state.files.lightweight_tree_projection = true;
        crate::refresh_tree_projection(&mut state.files);
    } else if !snapshot.large_repo_mode && was_large_repo_mode {
        state.files.lightweight_tree_projection = false;
        state.files.expanded_dirs = crate::collect_directories(&state.files.items);
        crate::refresh_tree_projection(&mut state.files);
    }
    reconcile_after_items_changed(&mut state.files);
    clamp_file_selection(&mut state.files);
    state.details.files_diff.clear();
    state.details.files_error = None;
    state.details.files_targets = crate::selected_target_paths(&state.files);
    state.details.files_diff_truncated_from = None;
    state.details.cached_files_diffs.clear();
    details::reset_scroll(state);
    state.search.clear();
}

pub(crate) fn apply_commits_snapshot(state: &mut AppState, commits: Vec<CommitEntry>) {
    state.commits.items = commits;
    state.commits.files = CommitFilesPanelState::default();
    state.commits.has_more = state.commits.items.len() >= crate::COMMITS_PAGE_SIZE;
    state.commits.loading_more = false;
    state.commits.pending_select_after_load = false;
    state.commits.pagination_epoch = state.commits.pagination_epoch.wrapping_add(1);
    reconcile_commits_after_items_changed(&mut state.commits);
    clamp_commit_selection(&mut state.commits);
    state.details.commit_diff.clear();
    state.details.commit_diff_error = None;
    state.details.commit_diff_target = crate::selected_commit_id(state);
    state.details.commit_file_diff.clear();
    state.details.commit_file_diff_error = None;
    state.details.commit_file_diff_target = None;
    state.details.cached_commit_diffs.clear();
    details::reset_scroll(state);
    state.search.clear();
}

pub(crate) fn apply_branches_snapshot(state: &mut AppState, branches: Vec<BranchEntry>) {
    state.branches.items = branches;
    state.branches.selected = clamp_index(state.branches.selected, state.branches.items.len());
    state.branches.scroll_offset = 0;
    crate::branches::leave_multi_select(&mut state.branches);
    state.details.branch_log.clear();
    state.details.branch_log_error = None;
    state.details.branch_log_target = crate::selected_branch_name(state);
    state.details.cached_branch_logs.clear();
    details::reset_scroll(state);
    state.search.clear();
}

pub(crate) fn apply_stashes_snapshot(state: &mut AppState, stashes: Vec<StashEntry>) {
    state.stash.items = stashes;
    state.stash.selected = clamp_index(state.stash.selected, state.stash.items.len());
    state.stash.scroll_offset = 0;
    state.search.clear();
}

fn clamp_index(index: usize, len: usize) -> usize {
    if len == 0 { 0 } else { index.min(len - 1) }
}
