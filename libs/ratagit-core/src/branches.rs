use crate::text_edit::{
    CursorMove, backspace_at_cursor, insert_char_at_cursor, move_cursor_in_text,
};
use crate::{
    AppContext, AutoStashOperation, BranchDeleteMode, BranchEntry, BranchInputMode,
    BranchRebaseChoice, BranchesUiState, Command, push_notice, with_pending,
};

pub(crate) fn move_selected_branch(
    items: &[BranchEntry],
    state: &mut BranchesUiState,
    move_up: bool,
) {
    crate::scroll::move_selected_index(&mut state.selected, items.len(), move_up);
    if state.mode == BranchInputMode::MultiSelect {
        refresh_branch_multi_select_range(items, state);
    }
}

pub(crate) fn move_selected_branch_in_viewport(
    items: &[BranchEntry],
    state: &mut BranchesUiState,
    move_up: bool,
    visible_lines: usize,
) {
    crate::scroll::move_selected_index_with_scroll_offset(
        &mut state.selected,
        &mut state.scroll_offset,
        items.len(),
        move_up,
        visible_lines,
    );
    if state.mode == BranchInputMode::MultiSelect {
        refresh_branch_multi_select_range(items, state);
    }
}

pub(crate) fn enter_multi_select(items: &[BranchEntry], state: &mut BranchesUiState) {
    if items.is_empty() {
        return;
    }
    let Some(key) = selected_branch_key(items, state) else {
        return;
    };
    state.mode = BranchInputMode::MultiSelect;
    state.selection_anchor = Some(key.clone());
    state.selected_rows.clear();
    state.selected_rows.insert(key);
}

pub(crate) fn leave_multi_select(state: &mut BranchesUiState) {
    state.mode = BranchInputMode::Normal;
    state.selection_anchor = None;
    state.selected_rows.clear();
}

pub fn branch_is_selected_for_batch(state: &BranchesUiState, branch_name: &str) -> bool {
    state.mode == BranchInputMode::MultiSelect && state.selected_rows.contains(branch_name)
}

fn refresh_branch_multi_select_range(items: &[BranchEntry], state: &mut BranchesUiState) {
    if items.is_empty() {
        leave_multi_select(state);
        return;
    }
    ensure_valid_branch_selection_anchor(items, state);
    let Some(anchor) = state.selection_anchor.clone() else {
        return;
    };
    let Some(current) = selected_branch_key(items, state) else {
        return;
    };
    let Some(anchor_index) = branch_index_for_key(items, &anchor) else {
        return;
    };
    let Some(current_index) = branch_index_for_key(items, &current) else {
        return;
    };
    let (start, end) = if anchor_index <= current_index {
        (anchor_index, current_index)
    } else {
        (current_index, anchor_index)
    };
    state.selected_rows.clear();
    for branch in &items[start..=end] {
        state.selected_rows.insert(branch.name.clone());
    }
}

fn ensure_valid_branch_selection_anchor(items: &[BranchEntry], state: &mut BranchesUiState) {
    if state
        .selection_anchor
        .as_ref()
        .is_some_and(|anchor| branch_index_for_key(items, anchor).is_some())
    {
        return;
    }
    state.selection_anchor = selected_branch_key(items, state);
}

fn selected_branch_key(items: &[BranchEntry], state: &BranchesUiState) -> Option<String> {
    items.get(state.selected).map(|branch| branch.name.clone())
}

fn branch_index_for_key(items: &[BranchEntry], key: &str) -> Option<usize> {
    items.iter().position(|branch| branch.name == key)
}

pub(crate) fn open_create_input(state: &mut AppContext) {
    let Some(start_point) = selected_branch_name(state) else {
        push_notice(state, "No branch selected");
        return;
    };
    state.ui.editor.kind = None;
    state.ui.reset_menu.active = false;
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
    state.ui.reset_menu.active = false;
    close_discard_confirm(state);
    close_popovers(state);
    state.ui.branches.delete_menu.active = true;
    state.ui.branches.delete_menu.selected = crate::BranchDeleteChoice::Local;
    state.ui.branches.delete_menu.target_branch = branch;
}

pub(crate) fn confirm_delete_menu(state: &mut AppContext) -> Vec<Command> {
    if !state.ui.branches.delete_menu.active {
        return Vec::new();
    }
    let name = state.ui.branches.delete_menu.target_branch.clone();
    let mode = state.ui.branches.delete_menu.selected.delete_mode();
    if delete_mode_includes_local(mode) && branch_is_current(state, &name) {
        close_delete_menu(state);
        push_notice(
            state,
            "Cannot delete current branch; checkout another branch first",
        );
        return Vec::new();
    }
    close_delete_menu(state);
    with_pending(
        state,
        vec![Command::DeleteBranch {
            name,
            mode,
            force: false,
        }],
    )
}

pub(crate) fn close_delete_menu(state: &mut AppContext) {
    state.ui.branches.delete_menu.active = false;
    state.ui.branches.delete_menu.selected = crate::BranchDeleteChoice::Local;
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
    state.ui.branches.force_delete_confirm.target_branch = name;
    state.ui.branches.force_delete_confirm.mode = Some(mode);
    state.ui.branches.force_delete_confirm.reason = reason;
}

pub(crate) fn confirm_force_delete(state: &mut AppContext) -> Vec<Command> {
    if !state.ui.branches.force_delete_confirm.active {
        return Vec::new();
    }
    let name = state.ui.branches.force_delete_confirm.target_branch.clone();
    let mode = state
        .ui
        .branches
        .force_delete_confirm
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
    state.ui.branches.force_delete_confirm.target_branch.clear();
    state.ui.branches.force_delete_confirm.mode = None;
    state.ui.branches.force_delete_confirm.reason.clear();
}

pub(crate) fn open_rebase_menu(state: &mut AppContext) {
    let Some(branch) = selected_branch_name(state) else {
        push_notice(state, "No branch selected");
        return;
    };
    state.ui.editor.kind = None;
    state.ui.reset_menu.active = false;
    close_discard_confirm(state);
    close_popovers(state);
    state.ui.branches.rebase_menu.active = true;
    state.ui.branches.rebase_menu.selected = BranchRebaseChoice::Simple;
    state.ui.branches.rebase_menu.target_branch = branch;
}

pub(crate) fn confirm_rebase_menu(state: &mut AppContext) -> Vec<Command> {
    if !state.ui.branches.rebase_menu.active {
        return Vec::new();
    }
    let choice = state.ui.branches.rebase_menu.selected;
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
    state.ui.branches.rebase_menu.active = false;
    state.ui.branches.rebase_menu.selected = BranchRebaseChoice::Simple;
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
    let operation = state.ui.branches.auto_stash_confirm.operation.clone();
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
    state.ui.branches.auto_stash_confirm.operation = None;
}

pub(crate) fn close_popovers(state: &mut AppContext) {
    close_create_input(state);
    close_delete_menu(state);
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
    state.ui.reset_menu.active = false;
    close_discard_confirm(state);
    close_create_input(state);
    close_delete_menu(state);
    close_rebase_menu(state);
    state.ui.branches.auto_stash_confirm.active = true;
    state.ui.branches.auto_stash_confirm.operation = Some(operation);
}

fn close_discard_confirm(state: &mut AppContext) {
    state.ui.discard_confirm.active = false;
    state.ui.discard_confirm.paths.clear();
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
