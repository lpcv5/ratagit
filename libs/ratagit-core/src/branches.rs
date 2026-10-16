use crate::state::{
    ensure_linear_selection_anchor, enter_linear_range_select, leave_linear_range_select,
    linear_key_at_selection, linear_key_is_selected, move_linear_selection,
    move_linear_selection_in_viewport, refresh_linear_range,
};
use crate::text_edit::{
    CursorMove, backspace_at_cursor, insert_char_at_cursor, move_cursor_in_text,
};
use crate::{
    AppContext, AutoStashOperation, BranchDeleteMode, BranchEntry, BranchInputMode,
    BranchRebaseChoice, BranchesSubview, BranchesUiState, Command, CommitFilesUiState, SearchScope,
    details, initialize_commit_files_tree, mark_commit_file_items_changed, push_notice,
    selected_branch_commit_id, with_pending,
};

pub(crate) fn move_selected_branch(
    items: &[BranchEntry],
    state: &mut BranchesUiState,
    move_up: bool,
) {
    move_linear_selection(&mut state.selection, items.len(), move_up);
    if state.mode == BranchInputMode::MultiSelect {
        refresh_branch_multi_select_range(items, state);
    }
}

pub(crate) fn move_selection(state: &mut AppContext, move_up: bool) {
    match state.ui.branches.subview {
        BranchesSubview::List => {
            move_selected_branch(&state.repo.branches.items, &mut state.ui.branches, move_up);
        }
        BranchesSubview::Commits => {
            crate::move_commit_selected(
                &state.repo.branches.commits,
                &mut state.ui.branches.commits,
                move_up,
            );
        }
        BranchesSubview::CommitFiles => {
            crate::move_commit_file_selected(&mut state.ui.branches.commit_files, move_up);
        }
    }
}

pub(crate) fn move_selection_in_viewport(
    state: &mut AppContext,
    move_up: bool,
    visible_lines: usize,
) {
    match state.ui.branches.subview {
        BranchesSubview::List => move_selected_branch_in_viewport(
            &state.repo.branches.items,
            &mut state.ui.branches,
            move_up,
            visible_lines,
        ),
        BranchesSubview::Commits => crate::move_commit_selected_in_viewport(
            &state.repo.branches.commits,
            &mut state.ui.branches.commits,
            move_up,
            visible_lines,
        ),
        BranchesSubview::CommitFiles => crate::move_commit_file_selected_in_viewport(
            &mut state.ui.branches.commit_files,
            move_up,
            visible_lines,
        ),
    }
}

pub(crate) fn open_commits_panel(state: &mut AppContext) -> Vec<Command> {
    let Some(branch) = selected_branch_name(state) else {
        push_notice(state, "No branch selected");
        return Vec::new();
    };
    leave_multi_select(&mut state.ui.branches);
    state.ui.branches.subview = BranchesSubview::Commits;
    state.ui.branches.subview_branch = Some(branch.clone());
    state.ui.branches.commits = crate::CommitsUiState::default();
    state.ui.branches.commit_files = CommitFilesUiState::default();
    state.repo.branches.commits.clear();
    state.repo.branches.commit_files.items.clear();
    state.repo.details.branch_log.clear();
    state.repo.details.branch_log_error = None;
    state.repo.details.commit_diff.clear();
    state.repo.details.commit_diff_target = None;
    state.repo.details.commit_diff_error = None;
    details::clear_details_pending(state);
    details::reset_scroll(state);
    crate::search::clear_search_if_incompatible(state);
    with_pending(state, vec![Command::RefreshBranchCommits { branch }])
}

pub(crate) fn close_commits_panel(state: &mut AppContext) -> Vec<Command> {
    if state.ui.branches.subview == BranchesSubview::List {
        return Vec::new();
    }
    state.ui.branches.subview = BranchesSubview::List;
    state.ui.branches.subview_branch = None;
    state.ui.branches.commits = crate::CommitsUiState::default();
    state.ui.branches.commit_files = CommitFilesUiState::default();
    state.repo.branches.commits.clear();
    state.repo.branches.commit_files.items.clear();
    state.work.commit_files.commit_files_loading = false;
    crate::search::clear_search_if_incompatible(state);
    details::refresh_for_focus(state)
}

pub(crate) fn open_commit_files_panel(state: &mut AppContext) -> Vec<Command> {
    if state.ui.branches.subview != BranchesSubview::Commits {
        return Vec::new();
    }
    let Some(branch) = state.ui.branches.subview_branch.clone() else {
        push_notice(state, "No branch selected");
        return Vec::new();
    };
    let Some(commit_id) = selected_branch_commit_id(state) else {
        push_notice(state, "No commit selected");
        return Vec::new();
    };
    state.ui.branches.subview = BranchesSubview::CommitFiles;
    state.ui.branches.commit_files = CommitFilesUiState {
        active: true,
        commit_id: Some(commit_id.clone()),
        ..CommitFilesUiState::default()
    };
    state.repo.branches.commit_files.items.clear();
    state.repo.details.commit_file_diff.clear();
    state.repo.details.commit_file_diff_target = None;
    state.repo.details.commit_file_diff_error = None;
    details::clear_details_pending(state);
    details::reset_scroll(state);
    crate::search::clear_search_if_incompatible(state);
    with_pending(
        state,
        vec![Command::RefreshBranchCommitFiles { branch, commit_id }],
    )
}

pub(crate) fn close_commit_files_panel(state: &mut AppContext) -> Vec<Command> {
    if state.ui.branches.subview != BranchesSubview::CommitFiles {
        return Vec::new();
    }
    state.ui.branches.subview = BranchesSubview::Commits;
    state.ui.branches.commit_files = CommitFilesUiState::default();
    state.repo.branches.commit_files.items.clear();
    state.work.commit_files.commit_files_loading = false;
    state.repo.details.commit_file_diff.clear();
    state.repo.details.commit_file_diff_target = None;
    state.repo.details.commit_file_diff_error = None;
    details::clear_details_pending(state);
    crate::search::clear_search_if_incompatible(state);
    details::refresh_for_focus(state)
}

pub(crate) fn handle_branch_commits_result(
    state: &mut AppContext,
    branch: String,
    result: Result<Vec<crate::CommitEntry>, String>,
) -> Vec<Command> {
    if state.ui.branches.subview == BranchesSubview::List
        || state.ui.branches.subview_branch.as_deref() != Some(branch.as_str())
    {
        return Vec::new();
    }
    state
        .work
        .refresh
        .pending_refreshes
        .remove(&crate::RefreshTarget::Branches);
    state.work.refresh.refresh_pending = !state.work.refresh.pending_refreshes.is_empty();
    state.work.mark_command_completed("branch_commits");
    match result {
        Ok(commits) => {
            state.repo.branches.commits = commits;
            crate::reconcile_commits_after_items_changed(
                &state.repo.branches.commits,
                &mut state.ui.branches.commits,
            );
            crate::clamp_commit_selection(
                &state.repo.branches.commits,
                &mut state.ui.branches.commits,
            );
            state.repo.status.last_error = None;
            details::refresh_for_focus(state)
        }
        Err(error) => {
            let message = format!("Failed to refresh branch commits: {error}");
            state.repo.status.last_error = Some(message.clone());
            push_notice(state, &message);
            Vec::new()
        }
    }
}

pub(crate) fn handle_branch_commit_files_result(
    state: &mut AppContext,
    branch: String,
    commit_id: String,
    result: Result<Vec<crate::CommitFileEntry>, String>,
) -> Vec<Command> {
    if state.ui.branches.subview != BranchesSubview::CommitFiles
        || state.ui.branches.subview_branch.as_deref() != Some(branch.as_str())
        || state.ui.branches.commit_files.commit_id.as_deref() != Some(commit_id.as_str())
    {
        return Vec::new();
    }
    state.work.commit_files.commit_files_loading = false;
    state.work.mark_command_completed("branch_commit_files");
    match result {
        Ok(files) => {
            state.repo.branches.commit_files.items = files;
            mark_commit_file_items_changed(
                &state.repo.branches.commit_files.items,
                &mut state.ui.branches.commit_files,
            );
            state.ui.branches.commit_files.selected = 0;
            state.ui.branches.commit_files.scroll_offset = 0;
            initialize_commit_files_tree(
                &state.repo.branches.commit_files.items,
                &mut state.ui.branches.commit_files,
            );
            if state.ui.search.scope == Some(SearchScope::CommitFiles)
                && !state.ui.search.query.is_empty()
            {
                crate::search::recompute_search_matches(state);
            }
            state.repo.status.last_error = None;
            details::refresh_for_focus(state)
        }
        Err(error) => {
            let message = format!("Failed to refresh branch commit files: {error}");
            state.repo.status.last_error = Some(message.clone());
            state.repo.details.commit_file_diff.clear();
            state.repo.details.commit_file_diff_error = Some(message.clone());
            push_notice(state, &message);
            Vec::new()
        }
    }
}

pub(crate) fn move_selected_branch_in_viewport(
    items: &[BranchEntry],
    state: &mut BranchesUiState,
    move_up: bool,
    visible_lines: usize,
) {
    move_linear_selection_in_viewport(&mut state.selection, items.len(), move_up, visible_lines);
    if state.mode == BranchInputMode::MultiSelect {
        refresh_branch_multi_select_range(items, state);
    }
}

pub(crate) fn enter_multi_select(items: &[BranchEntry], state: &mut BranchesUiState) {
    if items.is_empty() {
        return;
    }
    let keys = branch_keys(items);
    let Some(key) = linear_key_at_selection(&state.selection, &keys) else {
        return;
    };
    state.mode = BranchInputMode::MultiSelect;
    enter_linear_range_select(&mut state.selection, Some(key));
}

pub(crate) fn leave_multi_select(state: &mut BranchesUiState) {
    state.mode = BranchInputMode::Normal;
    leave_linear_range_select(&mut state.selection);
}

pub fn branch_is_selected_for_batch(state: &BranchesUiState, branch_name: &str) -> bool {
    state.mode == BranchInputMode::MultiSelect
        && linear_key_is_selected(&state.selection, branch_name)
}

fn refresh_branch_multi_select_range(items: &[BranchEntry], state: &mut BranchesUiState) {
    if items.is_empty() {
        leave_multi_select(state);
        return;
    }
    let keys = branch_keys(items);
    ensure_linear_selection_anchor(&mut state.selection, &keys);
    refresh_linear_range(&mut state.selection, &keys);
}

fn branch_keys(items: &[BranchEntry]) -> Vec<String> {
    items.iter().map(|branch| branch.name.clone()).collect()
}

pub(crate) fn open_create_input(state: &mut AppContext) {
    let Some(start_point) = selected_branch_name(state) else {
        push_notice(state, "No branch selected");
        return;
    };
    state.ui.editor.kind = None;
    state.ui.reset_menu.menu.active = false;
    close_discard_confirm(state);
    close_popovers(state);
    state.ui.branches.create.active = true;
    state.ui.branches.create.name.clear();
    state.ui.branches.create.cursor = 0;
    state.ui.branches.create.start_point = start_point;
}

pub(crate) fn input_create_char(state: &mut AppContext, ch: char) {
    if !state.ui.branches.create.active {
        return;
    }
    insert_char_at_cursor(
        &mut state.ui.branches.create.name,
        &mut state.ui.branches.create.cursor,
        ch,
    );
}

pub(crate) fn backspace_create(state: &mut AppContext) {
    if !state.ui.branches.create.active {
        return;
    }
    backspace_at_cursor(
        &mut state.ui.branches.create.name,
        &mut state.ui.branches.create.cursor,
    );
}

pub(crate) fn move_create_cursor_left(state: &mut AppContext) {
    move_create_cursor(state, CursorMove::Left);
}

pub(crate) fn move_create_cursor_right(state: &mut AppContext) {
    move_create_cursor(state, CursorMove::Right);
}

pub(crate) fn move_create_cursor_home(state: &mut AppContext) {
    move_create_cursor(state, CursorMove::Home);
}

pub(crate) fn move_create_cursor_end(state: &mut AppContext) {
    move_create_cursor(state, CursorMove::End);
}

pub(crate) fn confirm_create(state: &mut AppContext) -> Vec<Command> {
    if !state.ui.branches.create.active {
        return Vec::new();
    }
    let name = state.ui.branches.create.name.trim().to_string();
    let start_point = state.ui.branches.create.start_point.clone();
    if name.is_empty() {
        push_notice(state, "Branch name cannot be empty");
        return Vec::new();
    }
    close_create_input(state);
    with_pending(state, vec![Command::CreateBranch { name, start_point }])
}

pub(crate) fn close_create_input(state: &mut AppContext) {
    state.ui.branches.create.active = false;
    state.ui.branches.create.name.clear();
    state.ui.branches.create.cursor = 0;
    state.ui.branches.create.start_point.clear();
}

pub(crate) fn open_delete_menu(state: &mut AppContext) {
    let Some(branch) = selected_branch_name(state) else {
        push_notice(state, "No branch selected");
        return;
    };
    state.ui.editor.kind = None;
    state.ui.reset_menu.menu.active = false;
    close_discard_confirm(state);
    close_popovers(state);
    state.ui.branches.delete_menu.menu.active = true;
    state.ui.branches.delete_menu.menu.selected = crate::BranchDeleteChoice::Local;
    state.ui.branches.delete_menu.target_branch = branch;
}

pub(crate) fn confirm_delete_menu(state: &mut AppContext) -> Vec<Command> {
    if !state.ui.branches.delete_menu.menu.active {
        return Vec::new();
    }
    let name = state.ui.branches.delete_menu.target_branch.clone();
    let mode = state.ui.branches.delete_menu.menu.selected.delete_mode();
    if delete_mode_includes_local(mode) && branch_is_current(state, &name) {
        close_delete_menu(state);
        push_notice(
            state,
            "Cannot delete current branch; checkout another branch first",
        );
        return Vec::new();
    }
    close_delete_menu(state);
    if matches!(mode, BranchDeleteMode::Remote | BranchDeleteMode::Both) {
        open_delete_confirm(state, name, mode);
        return Vec::new();
    }
    with_pending(
        state,
        vec![Command::DeleteBranch {
            name,
            mode,
            force: false,
        }],
    )
}

pub(crate) fn open_delete_confirm(state: &mut AppContext, name: String, mode: BranchDeleteMode) {
    close_popovers(state);
    state.ui.branches.delete_confirm.active = true;
    state.ui.branches.delete_confirm.context.target_branch = name;
    state.ui.branches.delete_confirm.context.mode = Some(mode);
}

pub(crate) fn confirm_delete_danger(state: &mut AppContext) -> Vec<Command> {
    if !state.ui.branches.delete_confirm.active {
        return Vec::new();
    }
    let name = state
        .ui
        .branches
        .delete_confirm
        .context
        .target_branch
        .clone();
    let mode = state
        .ui
        .branches
        .delete_confirm
        .context
        .mode
        .unwrap_or(BranchDeleteMode::Remote);
    close_delete_confirm(state);
    with_pending(
        state,
        vec![Command::DeleteBranch {
            name,
            mode,
            force: false,
        }],
    )
}

pub(crate) fn close_delete_confirm(state: &mut AppContext) {
    state.ui.branches.delete_confirm.active = false;
    state
        .ui
        .branches
        .delete_confirm
        .context
        .target_branch
        .clear();
    state.ui.branches.delete_confirm.context.mode = None;
}

pub(crate) fn close_delete_menu(state: &mut AppContext) {
    state.ui.branches.delete_menu.menu.active = false;
    state.ui.branches.delete_menu.menu.selected = crate::BranchDeleteChoice::Local;
    state.ui.branches.delete_menu.target_branch.clear();
}

pub(crate) fn open_force_delete_confirm(
    state: &mut AppContext,
    name: String,
    mode: BranchDeleteMode,
    reason: String,
) {
    close_popovers(state);
    state.ui.branches.force_delete_confirm.active = true;
    state.ui.branches.force_delete_confirm.context.target_branch = name;
    state.ui.branches.force_delete_confirm.context.mode = Some(mode);
    state.ui.branches.force_delete_confirm.context.reason = reason;
}

pub(crate) fn confirm_force_delete(state: &mut AppContext) -> Vec<Command> {
    if !state.ui.branches.force_delete_confirm.active {
        return Vec::new();
    }
    let name = state
        .ui
        .branches
        .force_delete_confirm
        .context
        .target_branch
        .clone();
    let mode = state
        .ui
        .branches
        .force_delete_confirm
        .context
        .mode
        .unwrap_or(BranchDeleteMode::Local);
    close_force_delete_confirm(state);
    with_pending(
        state,
        vec![Command::DeleteBranch {
            name,
            mode,
            force: true,
        }],
    )
}

pub(crate) fn close_force_delete_confirm(state: &mut AppContext) {
    state.ui.branches.force_delete_confirm.active = false;
    state
        .ui
        .branches
        .force_delete_confirm
        .context
        .target_branch
        .clear();
    state.ui.branches.force_delete_confirm.context.mode = None;
    state
        .ui
        .branches
        .force_delete_confirm
        .context
        .reason
        .clear();
}

pub(crate) fn open_rebase_menu(state: &mut AppContext) {
    let Some(branch) = selected_branch_name(state) else {
        push_notice(state, "No branch selected");
        return;
    };
    state.ui.editor.kind = None;
    state.ui.reset_menu.menu.active = false;
    close_discard_confirm(state);
    close_popovers(state);
    state.ui.branches.rebase_menu.menu.active = true;
    state.ui.branches.rebase_menu.menu.selected = BranchRebaseChoice::Simple;
    state.ui.branches.rebase_menu.target_branch = branch;
}

pub(crate) fn confirm_rebase_menu(state: &mut AppContext) -> Vec<Command> {
    if !state.ui.branches.rebase_menu.menu.active {
        return Vec::new();
    }
    let choice = state.ui.branches.rebase_menu.menu.selected;
    let selected_target = state.ui.branches.rebase_menu.target_branch.clone();
    close_rebase_menu(state);
    let (target, interactive) = match choice {
        BranchRebaseChoice::Simple => (selected_target, false),
        BranchRebaseChoice::Interactive => (selected_target, true),
        BranchRebaseChoice::OriginMain => ("origin/main".to_string(), false),
    };
    rebase_or_confirm_stash(state, target, interactive)
}

pub(crate) fn close_rebase_menu(state: &mut AppContext) {
    state.ui.branches.rebase_menu.menu.active = false;
    state.ui.branches.rebase_menu.menu.selected = BranchRebaseChoice::Simple;
    state.ui.branches.rebase_menu.target_branch.clear();
}

pub(crate) fn checkout_selected(state: &mut AppContext) -> Vec<Command> {
    if let Some(branch) = selected_branch_name(state) {
        checkout_or_confirm_stash(state, branch)
    } else {
        push_notice(state, "No branch selected");
        Vec::new()
    }
}

pub(crate) fn confirm_auto_stash(state: &mut AppContext) -> Vec<Command> {
    if !state.ui.branches.auto_stash_confirm.active {
        return Vec::new();
    }
    let operation = state.ui.branches.auto_stash_confirm.context.clone();
    close_auto_stash_confirm(state);
    match operation {
        Some(AutoStashOperation::Checkout { branch }) => with_pending(
            state,
            vec![Command::CheckoutBranch {
                name: branch,
                auto_stash: true,
            }],
        ),
        Some(AutoStashOperation::CheckoutCommitDetached { commit_id }) => with_pending(
            state,
            vec![Command::CheckoutCommitDetached {
                commit_id,
                auto_stash: true,
            }],
        ),
        Some(AutoStashOperation::Rebase {
            target,
            interactive,
        }) => with_pending(
            state,
            vec![Command::RebaseBranch {
                target,
                interactive,
                auto_stash: true,
            }],
        ),
        None => Vec::new(),
    }
}

pub(crate) fn close_auto_stash_confirm(state: &mut AppContext) {
    state.ui.branches.auto_stash_confirm.active = false;
    state.ui.branches.auto_stash_confirm.context = None;
}

pub(crate) fn close_popovers(state: &mut AppContext) {
    state.ui.reset_menu.danger_confirm = None;
    close_create_input(state);
    close_delete_menu(state);
    close_delete_confirm(state);
    close_force_delete_confirm(state);
    close_rebase_menu(state);
    close_auto_stash_confirm(state);
}

fn move_create_cursor(state: &mut AppContext, movement: CursorMove) {
    if !state.ui.branches.create.active {
        return;
    }
    move_cursor_in_text(
        &state.ui.branches.create.name,
        &mut state.ui.branches.create.cursor,
        movement,
    );
}

fn checkout_or_confirm_stash(state: &mut AppContext, branch: String) -> Vec<Command> {
    if branch_is_current(state, &branch) {
        push_notice(state, "Branch already checked out");
        return Vec::new();
    }
    if repository_has_uncommitted_changes(state) {
        open_auto_stash_confirm(state, AutoStashOperation::Checkout { branch });
        return Vec::new();
    }
    with_pending(
        state,
        vec![Command::CheckoutBranch {
            name: branch,
            auto_stash: false,
        }],
    )
}

fn rebase_or_confirm_stash(
    state: &mut AppContext,
    target: String,
    interactive: bool,
) -> Vec<Command> {
    if repository_has_uncommitted_changes(state) {
        open_auto_stash_confirm(
            state,
            AutoStashOperation::Rebase {
                target,
                interactive,
            },
        );
        return Vec::new();
    }
    with_pending(
        state,
        vec![Command::RebaseBranch {
            target,
            interactive,
            auto_stash: false,
        }],
    )
}

pub(crate) fn open_auto_stash_confirm(state: &mut AppContext, operation: AutoStashOperation) {
    state.ui.editor.kind = None;
    state.ui.reset_menu.menu.active = false;
    close_discard_confirm(state);
    close_create_input(state);
    close_delete_menu(state);
    close_rebase_menu(state);
    state.ui.branches.auto_stash_confirm.active = true;
    state.ui.branches.auto_stash_confirm.context = Some(operation);
}

fn close_discard_confirm(state: &mut AppContext) {
    state.ui.discard_confirm.active = false;
    state.ui.discard_confirm.context.clear();
}

fn branch_is_current(state: &AppContext, name: &str) -> bool {
    state
        .repo
        .branches
        .items
        .iter()
        .any(|branch| branch.name == name && branch.is_current)
}

fn selected_branch_name(state: &AppContext) -> Option<String> {
    state
        .repo
        .branches
        .items
        .get(state.ui.branches.selected)
        .map(|branch| branch.name.clone())
}

fn repository_has_uncommitted_changes(state: &AppContext) -> bool {
    !state.repo.files.items.is_empty()
}

pub(crate) fn delete_mode_includes_local(mode: BranchDeleteMode) -> bool {
    matches!(mode, BranchDeleteMode::Local | BranchDeleteMode::Both)
}
