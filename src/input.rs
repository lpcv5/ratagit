use crossterm::event::{KeyCode, KeyModifiers};
use ratagit_core::{
    AppContext, BranchInputMode, BranchesSubview, CommitInputMode, FileInputMode, MenuDirection,
    MenuKind, PanelFocus, UiAction,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum KeyEffect {
    Quit,
    Dispatch(UiAction),
    Ignore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InputMode {
    Editor,
    BranchCreate,
    BranchDeleteMenu,
    BranchDeleteConfirm,
    BranchForceDeleteConfirm,
    BranchRebaseMenu,
    AutoStashConfirm,
    ResetMenu,
    ResetDangerConfirm,
    DiscardConfirm,
    ForcePushConfirm,
    StageAllConfirm,
    CommandPalette,
    SearchInput,
    SearchQuery,
    Panel,
}

pub(crate) fn input_mode_for_state(state: &AppContext) -> InputMode {
    if state.ui.editor.is_active() {
        InputMode::Editor
    } else if state.ui.branches.create.active {
        InputMode::BranchCreate
    } else if state.ui.branches.delete_menu.menu.active {
        InputMode::BranchDeleteMenu
    } else if state.ui.branches.delete_confirm.active {
        InputMode::BranchDeleteConfirm
    } else if state.ui.branches.force_delete_confirm.active {
        InputMode::BranchForceDeleteConfirm
    } else if state.ui.branches.rebase_menu.menu.active {
        InputMode::BranchRebaseMenu
    } else if state.ui.branches.auto_stash_confirm.active {
        InputMode::AutoStashConfirm
    } else if state.ui.reset_menu.menu.active {
        InputMode::ResetMenu
    } else if state.ui.reset_menu.danger_confirm.is_some() {
        InputMode::ResetDangerConfirm
    } else if state.ui.discard_confirm.active {
        InputMode::DiscardConfirm
    } else if state.ui.push_force_confirm.active {
        InputMode::ForcePushConfirm
    } else if state.ui.stage_all_confirm.active {
        InputMode::StageAllConfirm
    } else if state.ui.command_palette.active {
        InputMode::CommandPalette
    } else if search_input_is_current(state) {
        InputMode::SearchInput
    } else if search_query_is_current(state) {
        InputMode::SearchQuery
    } else {
        InputMode::Panel
    }
}

pub(crate) fn key_effect_for_key(
    state: &AppContext,
    code: KeyCode,
    modifiers: KeyModifiers,
    details_scroll_lines: usize,
    details_visible_lines: usize,
    left_panel_visible_lines: usize,
) -> KeyEffect {
    if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
        return KeyEffect::Quit;
    }

    if command_palette_selection_quits(state, code, modifiers) {
        return KeyEffect::Quit;
    }

    if let Some(action) = ui_action_for_key(
        state,
        code,
        modifiers,
        details_scroll_lines,
        details_visible_lines,
        left_panel_visible_lines,
    ) {
        return KeyEffect::Dispatch(action);
    }

    if code == KeyCode::Char('q') {
        return KeyEffect::Quit;
    }

    KeyEffect::Ignore
}

fn ui_action_for_key(
    state: &AppContext,
    code: KeyCode,
    modifiers: KeyModifiers,
    details_scroll_lines: usize,
    details_visible_lines: usize,
    left_panel_visible_lines: usize,
) -> Option<UiAction> {
    if let Some(action) =
        global_control_action_for_key(code, modifiers, details_scroll_lines, details_visible_lines)
    {
        return Some(action);
    }

    match input_mode_for_state(state) {
        InputMode::Editor => return editor_action_for_key(code, modifiers),
        InputMode::BranchCreate => return branch_create_action_for_key(code, modifiers),
        InputMode::BranchDeleteMenu => return branch_delete_menu_action_for_key(code),
        InputMode::BranchDeleteConfirm => return branch_delete_confirm_action_for_key(code),
        InputMode::BranchForceDeleteConfirm => return branch_force_delete_action_for_key(code),
        InputMode::BranchRebaseMenu => return branch_rebase_menu_action_for_key(code),
        InputMode::AutoStashConfirm => return auto_stash_action_for_key(code),
        InputMode::ResetMenu => return reset_menu_action_for_key(code),
        InputMode::ResetDangerConfirm => return reset_danger_action_for_key(code),
        InputMode::DiscardConfirm => return discard_confirm_action_for_key(code),
        InputMode::ForcePushConfirm => return force_push_action_for_key(code),
        InputMode::StageAllConfirm => return stage_all_action_for_key(code),
        InputMode::CommandPalette => {
            return command_palette_action_for_key(
                code,
                details_scroll_lines,
                details_visible_lines,
            );
        }
        InputMode::SearchInput => return search_input_action_for_key(code),
        InputMode::SearchQuery => {
            if let Some(action) = search_query_action_for_key(code) {
                return Some(action);
            }
        }
        InputMode::Panel => {}
    }

    panel_action_for_key(state, code, left_panel_visible_lines)
}

fn global_control_action_for_key(
    code: KeyCode,
    modifiers: KeyModifiers,
    details_scroll_lines: usize,
    details_visible_lines: usize,
) -> Option<UiAction> {
    if !modifiers.contains(KeyModifiers::CONTROL) {
        return None;
    }

    match code {
        KeyCode::Char('u') => Some(UiAction::DetailsScrollUp {
            lines: details_scroll_lines,
        }),
        KeyCode::Char('d') => Some(UiAction::DetailsScrollDown {
            lines: details_scroll_lines,
            visible_lines: details_visible_lines,
        }),
        _ => None,
    }
}

fn editor_action_for_key(code: KeyCode, modifiers: KeyModifiers) -> Option<UiAction> {
    match code {
        KeyCode::Enter => Some(UiAction::EditorConfirm),
        KeyCode::Esc => Some(UiAction::EditorCancel),
        KeyCode::Backspace => Some(UiAction::EditorBackspace),
        KeyCode::Left => Some(UiAction::EditorMoveCursorLeft),
        KeyCode::Right => Some(UiAction::EditorMoveCursorRight),
        KeyCode::Home => Some(UiAction::EditorMoveCursorHome),
        KeyCode::End => Some(UiAction::EditorMoveCursorEnd),
        KeyCode::Tab => Some(UiAction::EditorNextField),
        KeyCode::BackTab => Some(UiAction::EditorPrevField),
        KeyCode::Char('j') if modifiers.contains(KeyModifiers::CONTROL) => {
            Some(UiAction::EditorInsertNewline)
        }
        KeyCode::Char(ch) if !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
            Some(UiAction::EditorInputChar(ch))
        }
        _ => None,
    }
}

fn branch_create_action_for_key(code: KeyCode, modifiers: KeyModifiers) -> Option<UiAction> {
    match code {
        KeyCode::Enter => Some(UiAction::ConfirmBranchCreate),
        KeyCode::Esc => Some(UiAction::CancelBranchCreate),
        KeyCode::Backspace => Some(UiAction::BranchCreateBackspace),
        KeyCode::Left => Some(UiAction::BranchCreateMoveCursorLeft),
        KeyCode::Right => Some(UiAction::BranchCreateMoveCursorRight),
        KeyCode::Home => Some(UiAction::BranchCreateMoveCursorHome),
        KeyCode::End => Some(UiAction::BranchCreateMoveCursorEnd),
        KeyCode::Char(ch) if !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
            Some(UiAction::BranchCreateInputChar(ch))
        }
        _ => None,
    }
}

fn branch_delete_menu_action_for_key(code: KeyCode) -> Option<UiAction> {
    menu_keys(
        code,
        MenuKind::BranchDelete,
        UiAction::ConfirmBranchDeleteMenu,
        UiAction::CancelBranchDeleteMenu,
    )
}

fn branch_delete_confirm_action_for_key(code: KeyCode) -> Option<UiAction> {
    confirm_keys(
        code,
        UiAction::ConfirmBranchDeleteDanger,
        UiAction::CancelBranchDeleteDanger,
    )
}

fn branch_force_delete_action_for_key(code: KeyCode) -> Option<UiAction> {
    confirm_keys(
        code,
        UiAction::ConfirmBranchForceDelete,
        UiAction::CancelBranchForceDelete,
    )
}

fn branch_rebase_menu_action_for_key(code: KeyCode) -> Option<UiAction> {
    menu_keys(
        code,
        MenuKind::BranchRebase,
        UiAction::ConfirmBranchRebaseMenu,
        UiAction::CancelBranchRebaseMenu,
    )
}

fn auto_stash_action_for_key(code: KeyCode) -> Option<UiAction> {
    confirm_keys(code, UiAction::ConfirmAutoStash, UiAction::CancelAutoStash)
}

fn reset_menu_action_for_key(code: KeyCode) -> Option<UiAction> {
    menu_keys(
        code,
        MenuKind::Reset,
        UiAction::ConfirmResetMenu,
        UiAction::CancelResetMenu,
    )
}

fn reset_danger_action_for_key(code: KeyCode) -> Option<UiAction> {
    confirm_keys(
        code,
        UiAction::ConfirmResetDanger,
        UiAction::CancelResetDanger,
    )
}

fn move_menu_action(menu: MenuKind, direction: MenuDirection) -> UiAction {
    UiAction::MoveMenuSelection { menu, direction }
}

fn confirm_keys(code: KeyCode, confirm: UiAction, cancel: UiAction) -> Option<UiAction> {
    match code {
        KeyCode::Enter => Some(confirm),
        KeyCode::Esc => Some(cancel),
        _ => None,
    }
}

fn menu_keys(
    code: KeyCode,
    menu: MenuKind,
    confirm: UiAction,
    cancel: UiAction,
) -> Option<UiAction> {
    match code {
        KeyCode::Enter => Some(confirm),
        KeyCode::Esc => Some(cancel),
        KeyCode::Up | KeyCode::Char('k') => Some(move_menu_action(menu, MenuDirection::Up)),
        KeyCode::Down | KeyCode::Char('j') => Some(move_menu_action(menu, MenuDirection::Down)),
        _ => None,
    }
}

fn discard_confirm_action_for_key(code: KeyCode) -> Option<UiAction> {
    confirm_keys(code, UiAction::ConfirmDiscard, UiAction::CancelDiscard)
}

fn force_push_action_for_key(code: KeyCode) -> Option<UiAction> {
    confirm_keys(code, UiAction::ConfirmForcePush, UiAction::CancelForcePush)
}

fn stage_all_action_for_key(code: KeyCode) -> Option<UiAction> {
    confirm_keys(code, UiAction::ConfirmStageAll, UiAction::CancelStageAll)
}

fn command_palette_action_for_key(
    code: KeyCode,
    details_scroll_lines: usize,
    details_visible_lines: usize,
) -> Option<UiAction> {
    match code {
        KeyCode::Enter => Some(UiAction::ExecuteCommandPalette {
            details_scroll_lines,
            details_visible_lines,
        }),
        KeyCode::Esc => Some(UiAction::CloseCommandPalette),
        KeyCode::Up | KeyCode::Char('k') => Some(UiAction::MoveCommandPaletteUp),
        KeyCode::Down | KeyCode::Char('j') => Some(UiAction::MoveCommandPaletteDown),
        _ => None,
    }
}

fn search_input_action_for_key(code: KeyCode) -> Option<UiAction> {
    match code {
        KeyCode::Enter => Some(UiAction::ConfirmSearch),
        KeyCode::Esc => Some(UiAction::CancelSearch),
        KeyCode::Backspace => Some(UiAction::BackspaceSearch),
        KeyCode::Char(ch) => Some(UiAction::InputSearchChar(ch)),
        _ => None,
    }
}

fn search_query_action_for_key(code: KeyCode) -> Option<UiAction> {
    match code {
        KeyCode::Char('n') => Some(UiAction::NextSearchMatch),
        KeyCode::Char('N') => Some(UiAction::PrevSearchMatch),
        KeyCode::Esc => Some(UiAction::CancelSearch),
        _ => None,
    }
}

fn panel_action_for_key(
    state: &AppContext,
    code: KeyCode,
    left_panel_visible_lines: usize,
) -> Option<UiAction> {
    if state.active_search_scope().is_some() && code == KeyCode::Char('/') {
        return Some(UiAction::StartSearch);
    }

    if code == KeyCode::Char('?') {
        return Some(UiAction::OpenCommandPalette);
    }

    match code {
        KeyCode::Char('p') => return Some(UiAction::Pull),
        KeyCode::Char('P') => return Some(UiAction::Push),
        _ => {}
    }

    if state.ui.focus == PanelFocus::Files {
        if state.ui.files.mode == FileInputMode::MultiSelect && code == KeyCode::Esc {
            return Some(UiAction::ExitFilesMultiSelect);
        }
        match code {
            // TODO(files-hunks): map Enter to hunk-level editing/partial stage workflow.
            KeyCode::Enter => return Some(UiAction::ToggleSelectedDirectory),
            KeyCode::Char(' ') => return Some(UiAction::ToggleSelectedFileStage),
            KeyCode::Char('v') if state.ui.files.mode != FileInputMode::MultiSelect => {
                return Some(UiAction::EnterFilesMultiSelect);
            }
            KeyCode::Char('A') => return Some(UiAction::AmendStagedChanges),
            KeyCode::Char('c') => return Some(UiAction::OpenCommitEditor),
            KeyCode::Char('s') => return Some(UiAction::OpenStashEditor),
            KeyCode::Char('d') => return Some(UiAction::OpenDiscardConfirm),
            KeyCode::Char('D') => return Some(UiAction::OpenResetMenu),
            _ => {}
        }
    }

    if state.ui.focus == PanelFocus::Branches {
        match state.ui.branches.subview {
            BranchesSubview::CommitFiles => {
                if state.ui.branches.commit_files.mode == FileInputMode::MultiSelect
                    && code == KeyCode::Esc
                {
                    return Some(UiAction::ExitCommitFilesMultiSelect);
                }
                if code == KeyCode::Esc {
                    return Some(UiAction::CloseBranchCommitFilesPanel);
                }
                if state.ui.branches.commit_files.mode != FileInputMode::MultiSelect
                    && code == KeyCode::Char('v')
                {
                    return Some(UiAction::EnterCommitFilesMultiSelect);
                }
                if code == KeyCode::Enter {
                    return Some(UiAction::ToggleBranchCommitFilesDirectory);
                }
            }
            BranchesSubview::Commits => {
                if state.ui.branches.commits.mode == CommitInputMode::MultiSelect
                    && code == KeyCode::Esc
                {
                    return Some(UiAction::ExitCommitsMultiSelect);
                }
                if code == KeyCode::Esc {
                    return Some(UiAction::CloseBranchCommitsPanel);
                }
                if state.ui.branches.commits.mode != CommitInputMode::MultiSelect
                    && code == KeyCode::Char('v')
                {
                    return Some(UiAction::EnterCommitsMultiSelect);
                }
                if code == KeyCode::Enter {
                    return Some(UiAction::OpenBranchCommitFilesPanel);
                }
            }
            BranchesSubview::List => {}
        }
        if state.ui.branches.subview == BranchesSubview::List {
            if state.ui.branches.mode == BranchInputMode::MultiSelect && code == KeyCode::Esc {
                return Some(UiAction::ExitBranchesMultiSelect);
            }
            match code {
                KeyCode::Enter => return Some(UiAction::OpenBranchCommitsPanel),
                KeyCode::Char(' ') => return Some(UiAction::CheckoutSelectedBranch),
                KeyCode::Char('v') if state.ui.branches.mode != BranchInputMode::MultiSelect => {
                    return Some(UiAction::EnterBranchesMultiSelect);
                }
                KeyCode::Char('n') => return Some(UiAction::OpenBranchCreateInput),
                KeyCode::Char('d') => return Some(UiAction::OpenBranchDeleteMenu),
                KeyCode::Char('r') => return Some(UiAction::OpenBranchRebaseMenu),
                _ => {}
            }
        }
    }

    if state.ui.focus == PanelFocus::Commits && state.ui.commits.files.active {
        if state.ui.commits.files.mode == FileInputMode::MultiSelect && code == KeyCode::Esc {
            return Some(UiAction::ExitCommitFilesMultiSelect);
        }
        if code == KeyCode::Esc {
            return Some(UiAction::CloseCommitFilesPanel);
        }
        if state.ui.commits.files.mode != FileInputMode::MultiSelect && code == KeyCode::Char('v') {
            return Some(UiAction::EnterCommitFilesMultiSelect);
        }
        if code == KeyCode::Enter {
            return Some(UiAction::ToggleCommitFilesDirectory);
        }
        // TODO(commit-files-shortcuts): add more commit-files local file actions in a later slice.
    } else if state.ui.focus == PanelFocus::Commits {
        if state.ui.commits.mode == CommitInputMode::MultiSelect && code == KeyCode::Esc {
            return Some(UiAction::ExitCommitsMultiSelect);
        }
        match code {
            KeyCode::Enter => return Some(UiAction::OpenCommitFilesPanel),
            KeyCode::Char(' ') => return Some(UiAction::CheckoutSelectedCommitDetached),
            KeyCode::Char('c') => return Some(UiAction::OpenCommitEditor),
            KeyCode::Char('A') => return Some(UiAction::AmendStagedChanges),
            KeyCode::Char('v') if state.ui.commits.mode != CommitInputMode::MultiSelect => {
                return Some(UiAction::EnterCommitsMultiSelect);
            }
            KeyCode::Char('s') => return Some(UiAction::SquashSelectedCommits),
            KeyCode::Char('f') => return Some(UiAction::FixupSelectedCommits),
            KeyCode::Char('r') => return Some(UiAction::OpenCommitRewordEditor),
            KeyCode::Char('d') => return Some(UiAction::DeleteSelectedCommits),
            _ => {}
        }
    }

    match code {
        KeyCode::Char('r') => Some(UiAction::RefreshAll),
        KeyCode::Char('A') => Some(UiAction::AmendStagedChanges),
        KeyCode::Char('l') => Some(UiAction::FocusNext),
        KeyCode::Char('h') => Some(UiAction::FocusPrev),
        KeyCode::Down | KeyCode::Char('j') => Some(UiAction::MoveDownInViewport {
            visible_lines: left_panel_visible_lines,
        }),
        KeyCode::Up | KeyCode::Char('k') => Some(UiAction::MoveUpInViewport {
            visible_lines: left_panel_visible_lines,
        }),
        KeyCode::Char('1') => Some(UiAction::FocusPanel {
            panel: PanelFocus::Files,
        }),
        KeyCode::Char('2') => Some(UiAction::FocusPanel {
            panel: PanelFocus::Branches,
        }),
        KeyCode::Char('3') => Some(UiAction::FocusPanel {
            panel: PanelFocus::Commits,
        }),
        KeyCode::Char('4') => Some(UiAction::FocusPanel {
            panel: PanelFocus::Stash,
        }),
        KeyCode::Char('5') => Some(UiAction::FocusPanel {
            panel: PanelFocus::Details,
        }),
        KeyCode::Char('6') => Some(UiAction::FocusPanel {
            panel: PanelFocus::Log,
        }),
        KeyCode::Char('c') => Some(UiAction::CreateCommit {
            message: "mvp commit".to_string(),
        }),
        KeyCode::Char('O') => Some(UiAction::StashPopSelected),
        KeyCode::Tab | KeyCode::BackTab => None,
        _ => None,
    }
}

fn command_palette_selection_quits(
    state: &AppContext,
    code: KeyCode,
    modifiers: KeyModifiers,
) -> bool {
    code == KeyCode::Enter
        && modifiers == KeyModifiers::NONE
        && input_mode_for_state(state) == InputMode::CommandPalette
        && state
            .selected_command_palette_entry()
            .is_some_and(|entry| entry.command.is_quit())
}

fn search_input_is_current(state: &AppContext) -> bool {
    state
        .active_search_scope()
        .is_some_and(|scope| state.ui.search.is_input_active_for(scope))
}

fn search_query_is_current(state: &AppContext) -> bool {
    state
        .active_search_scope()
        .is_some_and(|scope| state.ui.search.has_query_for(scope))
}

#[cfg(test)]
#[path = "input_tests.rs"]
mod input_tests;
