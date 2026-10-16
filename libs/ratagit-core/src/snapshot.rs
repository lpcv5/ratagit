use crate::{
    AppContext, BranchEntry, BranchesSubview, CommitEntry, CommitFilesUiState,
    DetailsRequestTarget, FilesSnapshot, RepoSnapshot, StashEntry, clamp_commit_selection,
    clamp_file_selection, details, initialize_tree_with_initial_expansion, mark_file_items_changed,
    reconcile_after_items_changed, reconcile_commits_after_items_changed,
};

pub(crate) fn apply_snapshot(state: &mut AppContext, snapshot: RepoSnapshot) {
    let index_entry_count = snapshot.files.len();
    apply_files_snapshot(
        state,
        FilesSnapshot {
            status_summary: snapshot.status_summary,
            current_branch: snapshot.current_branch,
            detached_head: snapshot.detached_head,
            files: snapshot.files,
            index_entry_count,
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
    state.ui.search.clear();
}

pub(crate) fn apply_split_snapshot(
    state: &mut AppContext,
    files: FilesSnapshot,
    branches: Vec<BranchEntry>,
    commits: Vec<CommitEntry>,
    stashes: Vec<StashEntry>,
) {
    apply_files_snapshot(state, files);
    apply_commits_snapshot(state, commits);
    apply_branches_snapshot(state, branches);
    apply_stashes_snapshot(state, stashes);
    details::reset_after_snapshot(state);
    state.ui.search.clear();
}

pub(crate) fn apply_files_snapshot(state: &mut AppContext, snapshot: FilesSnapshot) {
    let was_large_repo_mode = state.repo.status.large_repo_mode;
    state.repo.status.summary = snapshot.status_summary;
    state.repo.status.current_branch = snapshot.current_branch;
    state.repo.status.detached_head = snapshot.detached_head;
    state.repo.status.index_entry_count = snapshot.index_entry_count;
    state.repo.status.large_repo_mode = snapshot.large_repo_mode;
    state.repo.status.status_truncated = snapshot.status_truncated;
    state.repo.status.status_scan_skipped = snapshot.status_scan_skipped;
    state.repo.status.untracked_scan_skipped = snapshot.untracked_scan_skipped;
    state.repo.files.items = snapshot.files;
    mark_file_items_changed(&state.repo.files.items, &mut state.ui.files);
    initialize_tree_with_initial_expansion(
        &state.repo.files.items,
        &mut state.ui.files,
        !snapshot.large_repo_mode,
    );
    if snapshot.status_scan_skipped {
        state.ui.files.expanded_dirs.clear();
        state.ui.files.lightweight_tree_projection = true;
        state.ui.files.tree_initialized = true;
        crate::refresh_tree_projection(&state.repo.files.items, &mut state.ui.files);
    } else if snapshot.large_repo_mode && !was_large_repo_mode {
        state.ui.files.expanded_dirs.clear();
        state.ui.files.lightweight_tree_projection = true;
        crate::refresh_tree_projection(&state.repo.files.items, &mut state.ui.files);
    } else if !snapshot.large_repo_mode && was_large_repo_mode {
        state.ui.files.lightweight_tree_projection = false;
        state.ui.files.expanded_dirs = crate::collect_directories(&state.repo.files.items);
        crate::refresh_tree_projection(&state.repo.files.items, &mut state.ui.files);
    }
    reconcile_after_items_changed(&state.repo.files.items, &mut state.ui.files);
    clamp_file_selection(&state.repo.files.items, &mut state.ui.files);
    state.repo.details.files_diff.clear();
    state.repo.details.files_error = None;
    state.repo.details.files_targets =
        crate::selected_target_paths(&state.repo.files.items, &state.ui.files);
    state.repo.details.files_diff_truncated_from = None;
    state.repo.details.cached_files_diffs.clear();
    details::clear_details_pending_if(state, |target| {
        matches!(target, DetailsRequestTarget::FilesDiff { .. })
    });
    details::reset_scroll(state);
    state.ui.search.clear();
}

pub(crate) fn apply_commits_snapshot(state: &mut AppContext, commits: Vec<CommitEntry>) {
    state.repo.commits.items = commits;
    state.ui.commits.files = CommitFilesUiState::default();
    state.repo.commits.has_more = state.repo.commits.items.len() >= crate::COMMITS_PAGE_SIZE;
    state.work.pagination.commits_loading_more = false;
    state.work.pagination.commits_pending_select_after_load = false;
    state.repo.commits.pagination_epoch = state.repo.commits.pagination_epoch.wrapping_add(1);
    reconcile_commits_after_items_changed(&state.repo.commits.items, &mut state.ui.commits);
    clamp_commit_selection(&state.repo.commits.items, &mut state.ui.commits);
    state.repo.details.commit_diff.clear();
    state.repo.details.commit_diff_error = None;
    state.repo.details.commit_diff_target = crate::selected_commit_id(state);
    state.repo.details.commit_file_diff.clear();
    state.repo.details.commit_file_diff_error = None;
    state.repo.details.commit_file_diff_target = None;
    state.repo.details.cached_commit_diffs.clear();
    details::clear_details_pending_if(state, |target| {
        matches!(
            target,
            DetailsRequestTarget::CommitDiff { .. } | DetailsRequestTarget::CommitFileDiff { .. }
        )
    });
    details::reset_scroll(state);
    state.ui.search.clear();
}

pub(crate) fn apply_branches_snapshot(state: &mut AppContext, branches: Vec<BranchEntry>) {
    state.repo.branches.items = branches;
    state.ui.branches.selected =
        clamp_index(state.ui.branches.selected, state.repo.branches.items.len());
    state.ui.branches.scroll_offset = 0;
    state.ui.branches.subview = BranchesSubview::List;
    state.ui.branches.subview_branch = None;
    state.ui.branches.commits = crate::CommitsUiState::default();
    state.ui.branches.commit_files = CommitFilesUiState::default();
    state.repo.branches.commits.clear();
    state.repo.branches.commit_files.items.clear();
    crate::branches::leave_multi_select(&mut state.ui.branches);
    state.repo.details.branch_log.clear();
    state.repo.details.branch_log_error = None;
    state.repo.details.branch_log_target = crate::selected_branch_name(state);
    state.repo.details.cached_branch_logs.clear();
    details::clear_details_pending_if(state, |target| {
        matches!(target, DetailsRequestTarget::BranchLog { .. })
    });
    details::reset_scroll(state);
    state.ui.search.clear();
}

pub(crate) fn apply_stashes_snapshot(state: &mut AppContext, stashes: Vec<StashEntry>) {
    state.repo.stash.items = stashes;
    state.ui.stash.selected = clamp_index(state.ui.stash.selected, state.repo.stash.items.len());
    state.ui.stash.scroll_offset = 0;
    state.ui.search.clear();
}

fn clamp_index(index: usize, len: usize) -> usize {
    if len == 0 { 0 } else { index.min(len - 1) }
}
