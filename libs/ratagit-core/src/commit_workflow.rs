use crate::actions::with_pending;
use crate::search::{clear_search_if_incompatible, recompute_search_matches};
use crate::selectors::{repository_has_uncommitted_changes, selected_commit_id};
use crate::{
    AppContext, AutoStashOperation, Command, CommitEntry, CommitFilesUiState, CommitHashStatus,
    CommitInputMode, PanelFocus, SearchScope, StageAllOperation, branches, commit_key, details,
    initialize_commit_files_tree, leave_commit_multi_select, mark_commit_file_items_changed,
    move_commit_file_selected, move_commit_file_selected_in_viewport, move_commit_selected,
    move_commit_selected_in_viewport, operations, push_notice,
    reconcile_commits_after_items_appended, selected_commit, selected_commit_ids, selected_commits,
};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommitRewriteKind {
    Squash,
    Fixup,
    Delete,
}

pub(crate) fn squash_selected_commits(state: &mut AppContext) -> Vec<Command> {
    rewrite_selected_commits(state, CommitRewriteKind::Squash, "squash", |commit_ids| {
        Command::SquashCommits { commit_ids }
    })
}

pub(crate) fn fixup_selected_commits(state: &mut AppContext) -> Vec<Command> {
    rewrite_selected_commits(state, CommitRewriteKind::Fixup, "fixup", |commit_ids| {
        Command::FixupCommits { commit_ids }
    })
}

pub(crate) fn delete_selected_commits(state: &mut AppContext) -> Vec<Command> {
    rewrite_selected_commits(state, CommitRewriteKind::Delete, "delete", |commit_ids| {
        Command::DeleteCommits { commit_ids }
    })
}

pub(crate) fn create_commit(state: &mut AppContext, message: String) -> Vec<Command> {
    if repository_has_staged_changes(state) {
        return with_pending(state, vec![Command::CreateCommit { message }]);
    }
    open_stage_all_confirm(state, StageAllOperation::CreateCommit { message })
}

pub(crate) fn amend_staged_changes(state: &mut AppContext) -> Vec<Command> {
    if repository_has_staged_changes(state)
        && state.repo.files.items.iter().any(|entry| !entry.staged)
    {
        push_notice(state, "Amend requires only staged changes");
        return Vec::new();
    }

    let commit_id = if state.ui.focus == PanelFocus::Commits && !state.ui.commits.files.active {
        let Some(commit) = selected_commit(&state.repo.commits.items, &state.ui.commits) else {
            push_notice(state, "No commit selected");
            return Vec::new();
        };
        let selected_index = state.ui.commits.selected;
        if state
            .repo
            .commits
            .items
            .iter()
            .take(selected_index.saturating_add(1))
            .any(|commit| commit.hash_status != CommitHashStatus::Unpushed)
        {
            push_notice(state, "Amend only supports unpushed commits");
            return Vec::new();
        }
        if state
            .repo
            .commits
            .items
            .iter()
            .take(selected_index.saturating_add(1))
            .any(|commit| commit.is_merge)
        {
            push_notice(state, "Amend does not support merge commits yet");
            return Vec::new();
        }
        commit_key(&commit)
    } else {
        "HEAD".to_string()
    };

    if !repository_has_staged_changes(state) {
        return open_stage_all_confirm(state, StageAllOperation::AmendStagedChanges { commit_id });
    }

    push_notice(state, &format!("Queued amend into {commit_id}"));
    with_pending(state, vec![Command::AmendStagedChanges { commit_id }])
}

pub(crate) fn confirm_stage_all(state: &mut AppContext) -> Vec<Command> {
    if !state.ui.stage_all_confirm.active {
        return Vec::new();
    }
    let operation = state.ui.stage_all_confirm.context.operation.clone();
    let paths = state.ui.stage_all_confirm.context.paths.clone();
    close_stage_all_confirm(state);
    if paths.is_empty() {
        push_notice(state, "No file changes to stage");
        return Vec::new();
    }
    match operation {
        Some(StageAllOperation::CreateCommit { message }) => with_pending(
            state,
            vec![Command::StageAllThenCreateCommit { message, paths }],
        ),
        Some(StageAllOperation::AmendStagedChanges { commit_id }) => with_pending(
            state,
            vec![Command::StageAllThenAmendStagedChanges { commit_id, paths }],
        ),
        None => Vec::new(),
    }
}

pub(crate) fn close_stage_all_confirm(state: &mut AppContext) {
    state.ui.stage_all_confirm.active = false;
    state.ui.stage_all_confirm.context.operation = None;
    state.ui.stage_all_confirm.context.paths.clear();
}

pub(crate) fn checkout_selected_commit_detached(state: &mut AppContext) -> Vec<Command> {
    if state.ui.commits.mode == CommitInputMode::MultiSelect {
        push_notice(state, "Detached checkout supports one commit at a time");
        return Vec::new();
    }
    let Some(commit) = selected_commit(&state.repo.commits.items, &state.ui.commits) else {
        push_notice(state, "No commit selected");
        return Vec::new();
    };
    let commit_id = commit_key(&commit);
    if repository_has_uncommitted_changes(state) {
        branches::open_auto_stash_confirm(
            state,
            AutoStashOperation::CheckoutCommitDetached { commit_id },
        );
        return Vec::new();
    }
    with_pending(
        state,
        vec![Command::CheckoutCommitDetached {
            commit_id,
            auto_stash: false,
        }],
    )
}

pub(crate) fn load_more_commits_command(
    state: &mut AppContext,
    select_first_new: bool,
) -> Vec<Command> {
    if !state.repo.commits.has_more || state.repo.commits.items.is_empty() {
        return Vec::new();
    }
    if state.work.pagination.commits_loading_more {
        state.work.pagination.commits_pending_select_after_load |= select_first_new;
        return Vec::new();
    }
    state.work.pagination.commits_pending_select_after_load |= select_first_new;
    with_pending(
        state,
        vec![Command::LoadMoreCommits {
            offset: state.repo.commits.items.len(),
            limit: crate::COMMITS_PAGE_SIZE,
            epoch: state.repo.commits.pagination_epoch,
        }],
    )
}

pub(crate) fn handle_commits_page_result(
    state: &mut AppContext,
    offset: usize,
    limit: usize,
    epoch: u64,
    result: Result<Vec<CommitEntry>, String>,
) -> Vec<Command> {
    if epoch != state.repo.commits.pagination_epoch {
        return Vec::new();
    }
    state.work.pagination.commits_loading_more = false;
    state.work.mark_command_completed("load_more_commits");
    match result {
        Ok(mut commits) => {
            if offset != state.repo.commits.items.len() {
                state.work.pagination.commits_pending_select_after_load = false;
                return Vec::new();
            }
            let first_new_index = state.repo.commits.items.len();
            let loaded = commits.len();
            state.repo.commits.items.append(&mut commits);
            state.repo.commits.has_more = loaded >= limit;
            if state.work.pagination.commits_pending_select_after_load && loaded > 0 {
                state.ui.commits.selected = first_new_index;
            }
            state.work.pagination.commits_pending_select_after_load = false;
            state.repo.status.last_error = None;
            reconcile_commits_after_items_appended(
                &state.repo.commits.items,
                &mut state.ui.commits,
            );
        }
        Err(error) => {
            state.work.pagination.commits_pending_select_after_load = false;
            let message = format!("Failed to load more commits: {error}");
            state.repo.status.last_error = Some(message.clone());
            push_notice(state, &message);
        }
    }
    Vec::new()
}

pub(crate) fn move_commit_selection(state: &mut AppContext, move_up: bool) -> Vec<Command> {
    if state.ui.commits.files.active {
        move_commit_file_selected(&mut state.ui.commits.files, move_up);
        return Vec::new();
    }
    let was_at_loaded_end = !move_up
        && !state.repo.commits.items.is_empty()
        && state.ui.commits.selected + 1 >= state.repo.commits.items.len();
    move_commit_selected(&state.repo.commits.items, &mut state.ui.commits, move_up);
    if was_at_loaded_end {
        load_more_commits_command(state, true)
    } else if should_prefetch_commits(state, move_up) {
        load_more_commits_command(state, false)
    } else {
        Vec::new()
    }
}

pub(crate) fn move_commit_selection_in_viewport(
    state: &mut AppContext,
    move_up: bool,
    visible_lines: usize,
) -> Vec<Command> {
    if state.ui.commits.files.active {
        move_commit_file_selected_in_viewport(&mut state.ui.commits.files, move_up, visible_lines);
        return Vec::new();
    }
    let was_at_loaded_end = !move_up
        && !state.repo.commits.items.is_empty()
        && state.ui.commits.selected + 1 >= state.repo.commits.items.len();
    move_commit_selected_in_viewport(
        &state.repo.commits.items,
        &mut state.ui.commits,
        move_up,
        visible_lines,
    );
    if was_at_loaded_end {
        load_more_commits_command(state, true)
    } else if should_prefetch_commits(state, move_up) {
        load_more_commits_command(state, false)
    } else {
        Vec::new()
    }
}

pub(crate) fn open_commit_files_panel(state: &mut AppContext) -> Vec<Command> {
    let Some(commit_id) = selected_commit_id(state) else {
        push_notice(state, "No commit selected");
        return Vec::new();
    };
    state.ui.commits.files = CommitFilesUiState {
        active: true,
        commit_id: Some(commit_id.clone()),
        ..CommitFilesUiState::default()
    };
    clear_search_if_incompatible(state);
    state.repo.details.commit_file_diff.clear();
    state.repo.details.commit_file_diff_target = None;
    state.repo.details.commit_file_diff_error = None;
    details::clear_details_pending(state);
    details::reset_scroll(state);
    with_pending(state, vec![Command::RefreshCommitFiles { commit_id }])
}

pub(crate) fn close_commit_files_panel(state: &mut AppContext) -> Vec<Command> {
    if !state.ui.commits.files.active {
        return Vec::new();
    }
    state.ui.commits.files.active = false;
    state.work.commit_files.commit_files_loading = false;
    clear_search_if_incompatible(state);
    state.repo.details.commit_file_diff.clear();
    state.repo.details.commit_file_diff_target = None;
    state.repo.details.commit_file_diff_error = None;
    details::clear_details_pending(state);
    details::refresh_commit_diff(state)
}

pub(crate) fn handle_commit_files_result(
    state: &mut AppContext,
    commit_id: String,
    result: Result<Vec<crate::CommitFileEntry>, String>,
) -> Vec<Command> {
    if !state.ui.commits.files.active
        || state.ui.commits.files.commit_id.as_deref() != Some(commit_id.as_str())
    {
        return Vec::new();
    }
    state.work.commit_files.commit_files_loading = false;
    state.work.mark_command_completed("commit_files");
    match result {
        Ok(files) => {
            state.repo.commits.files.items = files;
            mark_commit_file_items_changed(
                &state.repo.commits.files.items,
                &mut state.ui.commits.files,
            );
            state.ui.commits.files.selected = 0;
            state.ui.commits.files.scroll_offset = 0;
            initialize_commit_files_tree(
                &state.repo.commits.files.items,
                &mut state.ui.commits.files,
            );
            if state.ui.search.scope == Some(SearchScope::CommitFiles)
                && !state.ui.search.query.is_empty()
            {
                recompute_search_matches(state);
            }
            state.repo.status.last_error = None;
            details::refresh_commit_file_diff(state)
        }
        Err(error) => {
            let message = format!("Failed to refresh commit files: {error}");
            state.repo.status.last_error = Some(message.clone());
            state.repo.details.commit_file_diff.clear();
            state.repo.details.commit_file_diff_error = Some(message.clone());
            push_notice(state, &message);
            Vec::new()
        }
    }
}

fn rewrite_selected_commits(
    state: &mut AppContext,
    _kind: CommitRewriteKind,
    action_label: &str,
    command: impl FnOnce(Vec<String>) -> Command,
) -> Vec<Command> {
    if repository_has_uncommitted_changes(state) {
        push_notice(state, "Commit rewrite requires a clean working tree");
        return Vec::new();
    }
    let commits = selected_commits(&state.repo.commits.items, &state.ui.commits);
    if commits.is_empty() {
        push_notice(state, "No commit selected");
        return Vec::new();
    }
    if commits
        .iter()
        .any(|commit| commit.hash_status != CommitHashStatus::Unpushed)
    {
        push_notice(state, "Commit rewrite only supports unpushed commits");
        return Vec::new();
    }
    if commits.iter().any(|commit| commit.is_merge) {
        push_notice(state, "Commit rewrite does not support merge commits yet");
        return Vec::new();
    }
    let commit_ids = selected_commit_ids(&state.repo.commits.items, &state.ui.commits);
    if commit_ids.is_empty() {
        push_notice(state, "No commit selected");
        return Vec::new();
    }
    if state.ui.commits.mode == CommitInputMode::MultiSelect {
        leave_commit_multi_select(&mut state.ui.commits);
    }
    push_notice(
        state,
        &format!(
            "Queued {action_label} for {}",
            operations::format_commit_count(commit_ids.len())
        ),
    );
    with_pending(state, vec![command(commit_ids)])
}

fn should_prefetch_commits(state: &AppContext, move_up: bool) -> bool {
    !move_up
        && state.ui.focus == PanelFocus::Commits
        && state.repo.commits.has_more
        && !state.repo.commits.items.is_empty()
        && state
            .repo
            .commits
            .items
            .len()
            .saturating_sub(1)
            .saturating_sub(state.ui.commits.selected)
            <= crate::COMMITS_PREFETCH_THRESHOLD
}

fn open_stage_all_confirm(state: &mut AppContext, operation: StageAllOperation) -> Vec<Command> {
    let paths = all_file_paths(state);
    if paths.is_empty() {
        push_notice(state, "No file changes to stage");
        return Vec::new();
    }
    state.ui.reset_menu.menu.active = false;
    state.ui.reset_menu.danger_confirm = None;
    state.ui.discard_confirm.active = false;
    state.ui.discard_confirm.context.clear();
    branches::close_popovers(state);
    state.ui.stage_all_confirm.active = true;
    state.ui.stage_all_confirm.context.operation = Some(operation);
    state.ui.stage_all_confirm.context.paths = paths;
    Vec::new()
}

fn repository_has_staged_changes(state: &AppContext) -> bool {
    state.repo.files.items.iter().any(|entry| entry.staged)
}

fn all_file_paths(state: &AppContext) -> Vec<String> {
    state
        .repo
        .files
        .items
        .iter()
        .map(|entry| entry.path.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
