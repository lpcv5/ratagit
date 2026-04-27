use crate::actions::with_pending;
use crate::search::{clear_search_if_incompatible, recompute_search_matches};
use crate::selectors::{repository_has_uncommitted_changes, selected_commit_id};
use crate::{
    AppState, AutoStashOperation, Command, CommitEntry, CommitFilesPanelState, CommitHashStatus,
    CommitInputMode, PanelFocus, SearchScope, branches, commit_key, details,
    initialize_commit_files_tree, leave_commit_multi_select, mark_commit_file_items_changed,
    move_commit_file_selected, move_commit_selected, operations, push_notice,
    reconcile_commits_after_items_appended, selected_commit, selected_commit_ids, selected_commits,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommitRewriteKind {
    Squash,
    Fixup,
    Delete,
}

pub(crate) fn squash_selected_commits(state: &mut AppState) -> Vec<Command> {
    rewrite_selected_commits(state, CommitRewriteKind::Squash, "squash", |commit_ids| {
        Command::SquashCommits { commit_ids }
    })
}

pub(crate) fn fixup_selected_commits(state: &mut AppState) -> Vec<Command> {
    rewrite_selected_commits(state, CommitRewriteKind::Fixup, "fixup", |commit_ids| {
        Command::FixupCommits { commit_ids }
    })
}

pub(crate) fn delete_selected_commits(state: &mut AppState) -> Vec<Command> {
    rewrite_selected_commits(state, CommitRewriteKind::Delete, "delete", |commit_ids| {
        Command::DeleteCommits { commit_ids }
    })
}

pub(crate) fn checkout_selected_commit_detached(state: &mut AppState) -> Vec<Command> {
    if state.commits.mode == CommitInputMode::MultiSelect {
        push_notice(state, "Detached checkout supports one commit at a time");
        return Vec::new();
    }
    let Some(commit) = selected_commit(&state.commits) else {
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
    state: &mut AppState,
    select_first_new: bool,
) -> Vec<Command> {
    if !state.commits.has_more || state.commits.items.is_empty() {
        return Vec::new();
    }
    if state.commits.loading_more {
        state.commits.pending_select_after_load |= select_first_new;
        return Vec::new();
    }
    state.commits.pending_select_after_load |= select_first_new;
    with_pending(
        state,
        vec![Command::LoadMoreCommits {
            offset: state.commits.items.len(),
            limit: crate::COMMITS_PAGE_SIZE,
            epoch: state.commits.pagination_epoch,
        }],
    )
}

pub(crate) fn handle_commits_page_result(
    state: &mut AppState,
    offset: usize,
    limit: usize,
    epoch: u64,
    result: Result<Vec<CommitEntry>, String>,
) -> Vec<Command> {
    if epoch != state.commits.pagination_epoch {
        return Vec::new();
    }
    state.commits.loading_more = false;
    state.work.last_completed_command = Some("load_more_commits".to_string());
    match result {
        Ok(mut commits) => {
            if offset != state.commits.items.len() {
                state.commits.pending_select_after_load = false;
                return Vec::new();
            }
            let first_new_index = state.commits.items.len();
            let loaded = commits.len();
            state.commits.items.append(&mut commits);
            state.commits.has_more = loaded >= limit;
            if state.commits.pending_select_after_load && loaded > 0 {
                state.commits.selected = first_new_index;
            }
            state.commits.pending_select_after_load = false;
            state.status.last_error = None;
            reconcile_commits_after_items_appended(&mut state.commits);
        }
        Err(error) => {
            state.commits.pending_select_after_load = false;
            let message = format!("Failed to load more commits: {error}");
            state.status.last_error = Some(message.clone());
            push_notice(state, &message);
        }
    }
    Vec::new()
}

pub(crate) fn move_commit_selection(state: &mut AppState, move_up: bool) -> Vec<Command> {
    if state.commits.files.active {
        move_commit_file_selected(&mut state.commits.files, move_up);
        return Vec::new();
    }
    let was_at_loaded_end = !move_up
        && !state.commits.items.is_empty()
        && state.commits.selected + 1 >= state.commits.items.len();
    move_commit_selected(&mut state.commits, move_up);
    if was_at_loaded_end {
        load_more_commits_command(state, true)
    } else if should_prefetch_commits(state, move_up) {
        load_more_commits_command(state, false)
    } else {
        Vec::new()
    }
}

pub(crate) fn open_commit_files_panel(state: &mut AppState) -> Vec<Command> {
    let Some(commit_id) = selected_commit_id(state) else {
        push_notice(state, "No commit selected");
        return Vec::new();
    };
    state.commits.files = CommitFilesPanelState {
        active: true,
        commit_id: Some(commit_id.clone()),
        loading: true,
        ..CommitFilesPanelState::default()
    };
    clear_search_if_incompatible(state);
    state.details.commit_file_diff.clear();
    state.details.commit_file_diff_target = None;
    state.details.commit_file_diff_error = None;
    details::reset_scroll(state);
    with_pending(state, vec![Command::RefreshCommitFiles { commit_id }])
}

pub(crate) fn close_commit_files_panel(state: &mut AppState) -> Vec<Command> {
    if !state.commits.files.active {
        return Vec::new();
    }
    state.commits.files.active = false;
    state.commits.files.loading = false;
    clear_search_if_incompatible(state);
    state.details.commit_file_diff.clear();
    state.details.commit_file_diff_target = None;
    state.details.commit_file_diff_error = None;
    state.work.details_pending = false;
    details::refresh_commit_diff(state)
}

pub(crate) fn handle_commit_files_result(
    state: &mut AppState,
    commit_id: String,
    result: Result<Vec<crate::CommitFileEntry>, String>,
) -> Vec<Command> {
    if !state.commits.files.active
        || state.commits.files.commit_id.as_deref() != Some(commit_id.as_str())
    {
        return Vec::new();
    }
    state.commits.files.loading = false;
    state.work.last_completed_command = Some("commit_files".to_string());
    match result {
        Ok(files) => {
            state.commits.files.items = files;
            mark_commit_file_items_changed(&mut state.commits.files);
            state.commits.files.selected = 0;
            state.commits.files.scroll_direction = None;
            state.commits.files.scroll_direction_origin = 0;
            initialize_commit_files_tree(&mut state.commits.files);
            if state.search.scope == Some(SearchScope::CommitFiles)
                && !state.search.query.is_empty()
            {
                recompute_search_matches(state);
            }
            state.status.last_error = None;
            details::refresh_commit_file_diff(state)
        }
        Err(error) => {
            let message = format!("Failed to refresh commit files: {error}");
            state.status.last_error = Some(message.clone());
            state.details.commit_file_diff.clear();
            state.details.commit_file_diff_error = Some(message.clone());
            push_notice(state, &message);
            Vec::new()
        }
    }
}

fn rewrite_selected_commits(
    state: &mut AppState,
    _kind: CommitRewriteKind,
    action_label: &str,
    command: impl FnOnce(Vec<String>) -> Command,
) -> Vec<Command> {
    if repository_has_uncommitted_changes(state) {
        push_notice(state, "Commit rewrite requires a clean working tree");
        return Vec::new();
    }
    let commits = selected_commits(&state.commits);
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
    let commit_ids = selected_commit_ids(&state.commits);
    if commit_ids.is_empty() {
        push_notice(state, "No commit selected");
        return Vec::new();
    }
    if state.commits.mode == CommitInputMode::MultiSelect {
        leave_commit_multi_select(&mut state.commits);
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

fn should_prefetch_commits(state: &AppState, move_up: bool) -> bool {
    !move_up
        && state.focus == PanelFocus::Commits
        && state.commits.has_more
        && !state.commits.items.is_empty()
        && state
            .commits
            .items
            .len()
            .saturating_sub(1)
            .saturating_sub(state.commits.selected)
            <= crate::COMMITS_PREFETCH_THRESHOLD
}
