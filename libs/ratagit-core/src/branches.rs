use crate::{
    AppState, AutoStashOperation, BranchDeleteMode, BranchRebaseChoice, Command, push_notice,
    with_pending,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CursorMove {
    Left,
    Right,
    Home,
    End,
}

pub(crate) fn open_create_input(state: &mut AppState) {
    let Some(start_point) = selected_branch_name(state) else {
        push_notice(state, "No branch selected");
        return;
    };
    state.editor.kind = None;
    state.reset_menu.active = false;
    close_discard_confirm(state);
    close_popovers(state);
    state.branches.create.active = true;
    state.branches.create.name.clear();
    state.branches.create.cursor = 0;
    state.branches.create.start_point = start_point;
}

pub(crate) fn input_create_char(state: &mut AppState, ch: char) {
    if !state.branches.create.active {
        return;
    }
    insert_char_at_cursor(
        &mut state.branches.create.name,
        &mut state.branches.create.cursor,
        ch,
    );
}

pub(crate) fn backspace_create(state: &mut AppState) {
    if !state.branches.create.active {
        return;
    }
    backspace_at_cursor(
        &mut state.branches.create.name,
        &mut state.branches.create.cursor,
    );
}

pub(crate) fn move_create_cursor_left(state: &mut AppState) {
    move_create_cursor(state, CursorMove::Left);
}

pub(crate) fn move_create_cursor_right(state: &mut AppState) {
    move_create_cursor(state, CursorMove::Right);
}

pub(crate) fn move_create_cursor_home(state: &mut AppState) {
    move_create_cursor(state, CursorMove::Home);
}

pub(crate) fn move_create_cursor_end(state: &mut AppState) {
    move_create_cursor(state, CursorMove::End);
}

pub(crate) fn confirm_create(state: &mut AppState) -> Vec<Command> {
    if !state.branches.create.active {
        return Vec::new();
    }
    let name = state.branches.create.name.trim().to_string();
    let start_point = state.branches.create.start_point.clone();
    if name.is_empty() {
        push_notice(state, "Branch name cannot be empty");
        return Vec::new();
    }
    close_create_input(state);
    with_pending(state, vec![Command::CreateBranch { name, start_point }])
}

pub(crate) fn close_create_input(state: &mut AppState) {
    state.branches.create.active = false;
    state.branches.create.name.clear();
    state.branches.create.cursor = 0;
    state.branches.create.start_point.clear();
}

pub(crate) fn open_delete_menu(state: &mut AppState) {
    let Some(branch) = selected_branch_name(state) else {
        push_notice(state, "No branch selected");
        return;
    };
    state.editor.kind = None;
    state.reset_menu.active = false;
    close_discard_confirm(state);
    close_popovers(state);
    state.branches.delete_menu.active = true;
    state.branches.delete_menu.selected = crate::BranchDeleteChoice::Local;
    state.branches.delete_menu.target_branch = branch;
}

pub(crate) fn confirm_delete_menu(state: &mut AppState) -> Vec<Command> {
    if !state.branches.delete_menu.active {
        return Vec::new();
    }
    let name = state.branches.delete_menu.target_branch.clone();
    let mode = state.branches.delete_menu.selected.delete_mode();
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

pub(crate) fn close_delete_menu(state: &mut AppState) {
    state.branches.delete_menu.active = false;
    state.branches.delete_menu.selected = crate::BranchDeleteChoice::Local;
    state.branches.delete_menu.target_branch.clear();
}

pub(crate) fn open_force_delete_confirm(
    state: &mut AppState,
    name: String,
    mode: BranchDeleteMode,
    reason: String,
) {
    close_popovers(state);
    state.branches.force_delete_confirm.active = true;
    state.branches.force_delete_confirm.target_branch = name;
    state.branches.force_delete_confirm.mode = Some(mode);
    state.branches.force_delete_confirm.reason = reason;
}

pub(crate) fn confirm_force_delete(state: &mut AppState) -> Vec<Command> {
    if !state.branches.force_delete_confirm.active {
        return Vec::new();
    }
    let name = state.branches.force_delete_confirm.target_branch.clone();
    let mode = state
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

pub(crate) fn close_force_delete_confirm(state: &mut AppState) {
    state.branches.force_delete_confirm.active = false;
    state.branches.force_delete_confirm.target_branch.clear();
    state.branches.force_delete_confirm.mode = None;
    state.branches.force_delete_confirm.reason.clear();
}

pub(crate) fn open_rebase_menu(state: &mut AppState) {
    let Some(branch) = selected_branch_name(state) else {
        push_notice(state, "No branch selected");
        return;
    };
    state.editor.kind = None;
    state.reset_menu.active = false;
    close_discard_confirm(state);
    close_popovers(state);
    state.branches.rebase_menu.active = true;
    state.branches.rebase_menu.selected = BranchRebaseChoice::Simple;
    state.branches.rebase_menu.target_branch = branch;
}

pub(crate) fn confirm_rebase_menu(state: &mut AppState) -> Vec<Command> {
    if !state.branches.rebase_menu.active {
        return Vec::new();
    }
    let choice = state.branches.rebase_menu.selected;
    let selected_target = state.branches.rebase_menu.target_branch.clone();
    close_rebase_menu(state);
    let (target, interactive) = match choice {
        BranchRebaseChoice::Simple => (selected_target, false),
        BranchRebaseChoice::Interactive => (selected_target, true),
        BranchRebaseChoice::OriginMain => ("origin/main".to_string(), false),
    };
    rebase_or_confirm_stash(state, target, interactive)
}

pub(crate) fn close_rebase_menu(state: &mut AppState) {
    state.branches.rebase_menu.active = false;
    state.branches.rebase_menu.selected = BranchRebaseChoice::Simple;
    state.branches.rebase_menu.target_branch.clear();
}

pub(crate) fn checkout_selected(state: &mut AppState) -> Vec<Command> {
    if let Some(branch) = selected_branch_name(state) {
        checkout_or_confirm_stash(state, branch)
    } else {
        push_notice(state, "No branch selected");
        Vec::new()
    }
}

pub(crate) fn confirm_auto_stash(state: &mut AppState) -> Vec<Command> {
    if !state.branches.auto_stash_confirm.active {
        return Vec::new();
    }
    let operation = state.branches.auto_stash_confirm.operation.clone();
    close_auto_stash_confirm(state);
    match operation {
        Some(AutoStashOperation::Checkout { branch }) => with_pending(
            state,
            vec![Command::CheckoutBranch {
                name: branch,
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

pub(crate) fn close_auto_stash_confirm(state: &mut AppState) {
    state.branches.auto_stash_confirm.active = false;
    state.branches.auto_stash_confirm.operation = None;
}

pub(crate) fn close_popovers(state: &mut AppState) {
    close_create_input(state);
    close_delete_menu(state);
    close_force_delete_confirm(state);
    close_rebase_menu(state);
    close_auto_stash_confirm(state);
}

fn move_create_cursor(state: &mut AppState, movement: CursorMove) {
    if !state.branches.create.active {
        return;
    }
    move_cursor_in_text(
        &state.branches.create.name,
        &mut state.branches.create.cursor,
        movement,
    );
}

fn checkout_or_confirm_stash(state: &mut AppState, branch: String) -> Vec<Command> {
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
    state: &mut AppState,
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

fn open_auto_stash_confirm(state: &mut AppState, operation: AutoStashOperation) {
    state.editor.kind = None;
    state.reset_menu.active = false;
    close_discard_confirm(state);
    close_create_input(state);
    close_delete_menu(state);
    close_rebase_menu(state);
    state.branches.auto_stash_confirm.active = true;
    state.branches.auto_stash_confirm.operation = Some(operation);
}

fn close_discard_confirm(state: &mut AppState) {
    state.discard_confirm.active = false;
    state.discard_confirm.paths.clear();
}

fn branch_is_current(state: &AppState, name: &str) -> bool {
    state
        .branches
        .items
        .iter()
        .any(|branch| branch.name == name && branch.is_current)
}

fn selected_branch_name(state: &AppState) -> Option<String> {
    state
        .branches
        .items
        .get(state.branches.selected)
        .map(|branch| branch.name.clone())
}

fn repository_has_uncommitted_changes(state: &AppState) -> bool {
    !state.files.items.is_empty()
}

pub(crate) fn delete_mode_includes_local(mode: BranchDeleteMode) -> bool {
    matches!(mode, BranchDeleteMode::Local | BranchDeleteMode::Both)
}

fn insert_char_at_cursor(text: &mut String, cursor: &mut usize, ch: char) {
    *cursor = clamp_to_char_boundary(text, *cursor);
    text.insert(*cursor, ch);
    *cursor += ch.len_utf8();
}

fn backspace_at_cursor(text: &mut String, cursor: &mut usize) {
    *cursor = clamp_to_char_boundary(text, *cursor);
    let Some(previous) = previous_char_boundary(text, *cursor) else {
        return;
    };
    text.drain(previous..*cursor);
    *cursor = previous;
}

fn move_cursor_in_text(text: &str, cursor: &mut usize, movement: CursorMove) {
    *cursor = clamp_to_char_boundary(text, *cursor);
    *cursor = match movement {
        CursorMove::Left => previous_char_boundary(text, *cursor).unwrap_or(0),
        CursorMove::Right => next_char_boundary(text, *cursor).unwrap_or(text.len()),
        CursorMove::Home => 0,
        CursorMove::End => text.len(),
    };
}

fn clamp_to_char_boundary(text: &str, cursor: usize) -> usize {
    if cursor >= text.len() {
        return text.len();
    }
    if text.is_char_boundary(cursor) {
        return cursor;
    }
    text.char_indices()
        .map(|(index, _)| index)
        .take_while(|index| *index < cursor)
        .last()
        .unwrap_or(0)
}

fn previous_char_boundary(text: &str, cursor: usize) -> Option<usize> {
    text.char_indices()
        .map(|(index, _)| index)
        .take_while(|index| *index < cursor)
        .last()
}

fn next_char_boundary(text: &str, cursor: usize) -> Option<usize> {
    text.char_indices()
        .map(|(index, _)| index)
        .find(|index| *index > cursor)
}
