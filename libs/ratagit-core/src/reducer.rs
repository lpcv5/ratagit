use crate::actions::with_pending;
use crate::search::{
    backspace_search, cancel_search, clear_search_if_incompatible, confirm_search,
    input_search_char, jump_search_match, start_search,
};
use crate::text_edit::CursorMove;
use crate::{
    Action, AppContext, Command, GitResult, UiAction, branches, commit_workflow, details, editor,
    navigation, results, toggle_commit_files_directory, worktree,
};

pub fn update(state: &mut AppContext, action: Action) -> Vec<Command> {
    match action {
        Action::Ui(ui_action) => update_ui(state, ui_action),
        Action::GitResult(git_result) => update_git_result(state, git_result),
    }
}

fn update_ui(state: &mut AppContext, action: UiAction) -> Vec<Command> {
    match action {
        UiAction::RefreshAll => with_pending(state, Command::refresh_all_commands()),
        UiAction::Pull => with_pending(state, vec![Command::Pull]),
        UiAction::Push => with_pending(state, vec![Command::Push { force: false }]),
        UiAction::ConfirmForcePush => {
            state.ui.push_force_confirm.active = false;
            state.ui.push_force_confirm.reason.clear();
            with_pending(state, vec![Command::Push { force: true }])
        }
        UiAction::CancelForcePush => {
            state.ui.push_force_confirm.active = false;
            state.ui.push_force_confirm.reason.clear();
            Vec::new()
        }
        UiAction::OpenCommitEditor => {
            editor::open_commit_editor(state);
            Vec::new()
        }
        UiAction::OpenStashEditor => {
            editor::open_stash_editor(state);
            Vec::new()
        }
        UiAction::OpenBranchCreateInput => {
            branches::open_create_input(state);
            Vec::new()
        }
        UiAction::BranchCreateInputChar(ch) => {
            branches::input_create_char(state, ch);
            Vec::new()
        }
        UiAction::BranchCreateBackspace => {
            branches::backspace_create(state);
            Vec::new()
        }
        UiAction::BranchCreateMoveCursorLeft => {
            branches::move_create_cursor_left(state);
            Vec::new()
        }
        UiAction::BranchCreateMoveCursorRight => {
            branches::move_create_cursor_right(state);
            Vec::new()
        }
        UiAction::BranchCreateMoveCursorHome => {
            branches::move_create_cursor_home(state);
            Vec::new()
        }
        UiAction::BranchCreateMoveCursorEnd => {
            branches::move_create_cursor_end(state);
            Vec::new()
        }
        UiAction::ConfirmBranchCreate => branches::confirm_create(state),
        UiAction::CancelBranchCreate => {
            branches::close_create_input(state);
            Vec::new()
        }
        UiAction::OpenBranchDeleteMenu => {
            branches::open_delete_menu(state);
            Vec::new()
        }
        UiAction::MoveBranchDeleteMenuUp => {
            state.ui.branches.delete_menu.selected = state.ui.branches.delete_menu.selected.prev();
            Vec::new()
        }
        UiAction::MoveBranchDeleteMenuDown => {
            state.ui.branches.delete_menu.selected = state.ui.branches.delete_menu.selected.next();
            Vec::new()
        }
        UiAction::ConfirmBranchDeleteMenu => branches::confirm_delete_menu(state),
        UiAction::CancelBranchDeleteMenu => {
            branches::close_delete_menu(state);
            Vec::new()
        }
        UiAction::ConfirmBranchForceDelete => branches::confirm_force_delete(state),
        UiAction::CancelBranchForceDelete => {
            branches::close_force_delete_confirm(state);
            Vec::new()
        }
        UiAction::OpenBranchRebaseMenu => {
            branches::open_rebase_menu(state);
            Vec::new()
        }
        UiAction::MoveBranchRebaseMenuUp => {
            state.ui.branches.rebase_menu.selected = state.ui.branches.rebase_menu.selected.prev();
            Vec::new()
        }
        UiAction::MoveBranchRebaseMenuDown => {
            state.ui.branches.rebase_menu.selected = state.ui.branches.rebase_menu.selected.next();
            Vec::new()
        }
        UiAction::ConfirmBranchRebaseMenu => branches::confirm_rebase_menu(state),
        UiAction::CancelBranchRebaseMenu => {
            branches::close_rebase_menu(state);
            Vec::new()
        }
        UiAction::ConfirmAutoStash => branches::confirm_auto_stash(state),
        UiAction::CancelAutoStash => {
            branches::close_auto_stash_confirm(state);
            Vec::new()
        }
        UiAction::OpenResetMenu => {
            worktree::open_reset_menu(state);
            Vec::new()
        }
        UiAction::MoveResetMenuUp => {
            state.ui.reset_menu.selected = state.ui.reset_menu.selected.prev();
            Vec::new()
        }
        UiAction::MoveResetMenuDown => {
            state.ui.reset_menu.selected = state.ui.reset_menu.selected.next();
            Vec::new()
        }
        UiAction::ConfirmResetMenu => worktree::confirm_reset_menu(state),
        UiAction::CancelResetMenu => {
            state.ui.reset_menu.active = false;
            Vec::new()
        }
        UiAction::OpenDiscardConfirm => {
            worktree::open_discard_confirm(state);
            Vec::new()
        }
        UiAction::ConfirmDiscard => worktree::confirm_discard(state),
        UiAction::CancelDiscard => {
            worktree::close_discard_confirm(state);
            Vec::new()
        }
        UiAction::EditorInputChar(ch) => {
            editor::input_char(state, ch);
            Vec::new()
        }
        UiAction::EditorBackspace => {
            editor::backspace(state);
            Vec::new()
        }
        UiAction::EditorMoveCursorLeft => {
            editor::move_cursor(state, CursorMove::Left);
            Vec::new()
        }
        UiAction::EditorMoveCursorRight => {
            editor::move_cursor(state, CursorMove::Right);
            Vec::new()
        }
        UiAction::EditorMoveCursorHome => {
            editor::move_cursor(state, CursorMove::Home);
            Vec::new()
        }
        UiAction::EditorMoveCursorEnd => {
            editor::move_cursor(state, CursorMove::End);
            Vec::new()
        }
        UiAction::EditorNextField => {
            editor::switch_field(state, false);
            Vec::new()
        }
        UiAction::EditorPrevField => {
            editor::switch_field(state, true);
            Vec::new()
        }
        UiAction::EditorInsertNewline => {
            editor::insert_newline(state);
            Vec::new()
        }
        UiAction::EditorConfirm => editor::confirm(state),
        UiAction::EditorCancel => {
            state.ui.editor.kind = None;
            Vec::new()
        }
        UiAction::FocusNext => {
            state.ui.focus = state.ui.focus.next_left();
            state.ui.last_left_focus = state.ui.focus;
            clear_search_if_incompatible(state);
            details::refresh_on_focus(state)
        }
        UiAction::FocusPrev => {
            state.ui.focus = state.ui.focus.prev_left();
            state.ui.last_left_focus = state.ui.focus;
            clear_search_if_incompatible(state);
            details::refresh_on_focus(state)
        }
        UiAction::FocusPanel { panel } => {
            state.ui.focus = panel;
            if panel.is_left_panel() {
                state.ui.last_left_focus = panel;
            }
            clear_search_if_incompatible(state);
            details::refresh_on_focus(state)
        }
        UiAction::MoveUp => move_selection_and_refresh_details(state, true),
        UiAction::MoveDown => move_selection_and_refresh_details(state, false),
        UiAction::MoveUpInViewport { visible_lines } => {
            move_selection_in_viewport_and_refresh_details(state, true, visible_lines)
        }
        UiAction::MoveDownInViewport { visible_lines } => {
            move_selection_in_viewport_and_refresh_details(state, false, visible_lines)
        }
        UiAction::DetailsScrollUp { lines } => {
            details::scroll_up(state, lines);
            Vec::new()
        }
        UiAction::DetailsScrollDown {
            lines,
            visible_lines,
        } => {
            details::scroll_down(state, lines, visible_lines);
            Vec::new()
        }
        UiAction::ToggleSelectedDirectory => {
            if navigation::toggle_selected_directory_or_notice(state) {
                details::refresh_files_details(state)
            } else {
                Vec::new()
            }
        }
        UiAction::ToggleSelectedFileStage => worktree::toggle_selected_file_stage(state),
        UiAction::EnterFilesMultiSelect => {
            worktree::enter_files_multi_select(state);
            Vec::new()
        }
        UiAction::ExitFilesMultiSelect => {
            worktree::exit_files_multi_select(state);
            Vec::new()
        }
        UiAction::ToggleCurrentFileSelection => {
            worktree::toggle_current_file_selection(state);
            Vec::new()
        }
        UiAction::StartSearch => {
            start_search(state);
            Vec::new()
        }
        UiAction::InputSearchChar(ch) => {
            input_search_char(state, ch);
            Vec::new()
        }
        UiAction::BackspaceSearch => {
            backspace_search(state);
            Vec::new()
        }
        UiAction::ConfirmSearch => {
            if confirm_search(state) {
                details::refresh_for_focus(state)
            } else {
                Vec::new()
            }
        }
        UiAction::CancelSearch => {
            cancel_search(state);
            Vec::new()
        }
        UiAction::NextSearchMatch => {
            if jump_search_match(state, false) {
                details::refresh_for_focus(state)
            } else {
                Vec::new()
            }
        }
        UiAction::PrevSearchMatch => {
            if jump_search_match(state, true) {
                details::refresh_for_focus(state)
            } else {
                Vec::new()
            }
        }
        UiAction::StageSelectedFile => worktree::stage_selected_file(state),
        UiAction::UnstageSelectedFile => worktree::unstage_selected_file(state),
        UiAction::StashSelectedFiles => worktree::stash_selected_files(state),
        UiAction::CreateCommit { message } => {
            state.ui.commits.draft_message = message.clone();
            with_pending(state, vec![Command::CreateCommit { message }])
        }
        UiAction::OpenCommitFilesPanel => commit_workflow::open_commit_files_panel(state),
        UiAction::CloseCommitFilesPanel => commit_workflow::close_commit_files_panel(state),
        UiAction::ToggleCommitFilesDirectory => {
            if toggle_commit_files_directory(
                &state.repo.commits.files.items,
                &mut state.ui.commits.files,
            ) {
                details::refresh_commit_file_diff(state)
            } else {
                push_notice(state, "Selected commit file is not a directory");
                Vec::new()
            }
        }
        UiAction::EnterCommitFilesMultiSelect => {
            crate::enter_commit_files_multi_select(
                &state.repo.commits.files.items,
                &mut state.ui.commits.files,
            );
            Vec::new()
        }
        UiAction::ExitCommitFilesMultiSelect => {
            crate::leave_commit_files_multi_select(
                &state.repo.commits.files.items,
                &mut state.ui.commits.files,
            );
            Vec::new()
        }
        UiAction::EnterCommitsMultiSelect => {
            crate::enter_commit_multi_select(&state.repo.commits.items, &mut state.ui.commits);
            Vec::new()
        }
        UiAction::ExitCommitsMultiSelect => {
            crate::leave_commit_multi_select(&mut state.ui.commits);
            Vec::new()
        }
        UiAction::SquashSelectedCommits => commit_workflow::squash_selected_commits(state),
        UiAction::FixupSelectedCommits => commit_workflow::fixup_selected_commits(state),
        UiAction::OpenCommitRewordEditor => {
            editor::open_commit_reword_editor(state);
            Vec::new()
        }
        UiAction::DeleteSelectedCommits => commit_workflow::delete_selected_commits(state),
        UiAction::CheckoutSelectedCommitDetached => {
            commit_workflow::checkout_selected_commit_detached(state)
        }
        UiAction::EnterBranchesMultiSelect => {
            branches::enter_multi_select(&state.repo.branches.items, &mut state.ui.branches);
            Vec::new()
        }
        UiAction::ExitBranchesMultiSelect => {
            branches::leave_multi_select(&mut state.ui.branches);
            Vec::new()
        }
        UiAction::CreateBranch { name, start_point } => {
            with_pending(state, vec![Command::CreateBranch { name, start_point }])
        }
        UiAction::CheckoutSelectedBranch => branches::checkout_selected(state),
        UiAction::StashPush { message } => {
            with_pending(state, vec![Command::StashPush { message }])
        }
        UiAction::StashPopSelected => {
            if let Some(stash_id) = crate::selected_stash_id(state) {
                with_pending(state, vec![Command::StashPop { stash_id }])
            } else {
                push_notice(state, "No stash selected");
                Vec::new()
            }
        }
    }
}

fn update_git_result(state: &mut AppContext, result: GitResult) -> Vec<Command> {
    results::update_git_result(state, result)
}

fn move_selection_and_refresh_details(state: &mut AppContext, move_up: bool) -> Vec<Command> {
    let mut commands = navigation::move_selection(state, move_up);
    commands.extend(details::refresh_on_navigation(state));
    commands
}

fn move_selection_in_viewport_and_refresh_details(
    state: &mut AppContext,
    move_up: bool,
    visible_lines: usize,
) -> Vec<Command> {
    let mut commands = navigation::move_selection_in_viewport(state, move_up, visible_lines);
    commands.extend(details::refresh_on_navigation(state));
    commands
}

pub(crate) fn push_notice(state: &mut AppContext, message: &str) {
    state.notices.push(message.to_string());
    if state.notices.len() > 10 {
        let keep_from = state.notices.len() - 10;
        state.notices.drain(0..keep_from);
    }
}
