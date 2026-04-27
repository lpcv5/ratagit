use crossterm::event::{KeyCode, KeyModifiers};
use ratagit_core::{
    AppState, BranchInputMode, CommitInputMode, FileInputMode, PanelFocus, UiAction,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum KeyEffect {
    Quit,
    Dispatch(UiAction),
    Ignore,
}

pub(crate) fn key_effect_for_key(
    state: &AppState,
    code: KeyCode,
    modifiers: KeyModifiers,
    details_scroll_lines: usize,
    details_visible_lines: usize,
    left_panel_visible_lines: usize,
) -> KeyEffect {
    if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
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
    state: &AppState,
    code: KeyCode,
    modifiers: KeyModifiers,
    details_scroll_lines: usize,
    details_visible_lines: usize,
    left_panel_visible_lines: usize,
) -> Option<UiAction> {
    if modifiers.contains(KeyModifiers::CONTROL) {
        match code {
            KeyCode::Char('u') => {
                return Some(UiAction::DetailsScrollUp {
                    lines: details_scroll_lines,
                });
            }
            KeyCode::Char('d') => {
                return Some(UiAction::DetailsScrollDown {
                    lines: details_scroll_lines,
                    visible_lines: details_visible_lines,
                });
            }
            _ => {}
        }
    }

    if state.editor.is_active() {
        return match code {
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
            KeyCode::Char(ch)
                if !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                Some(UiAction::EditorInputChar(ch))
            }
            _ => None,
        };
    }

    if state.branches.create.active {
        return match code {
            KeyCode::Enter => Some(UiAction::ConfirmBranchCreate),
            KeyCode::Esc => Some(UiAction::CancelBranchCreate),
            KeyCode::Backspace => Some(UiAction::BranchCreateBackspace),
            KeyCode::Left => Some(UiAction::BranchCreateMoveCursorLeft),
            KeyCode::Right => Some(UiAction::BranchCreateMoveCursorRight),
            KeyCode::Home => Some(UiAction::BranchCreateMoveCursorHome),
            KeyCode::End => Some(UiAction::BranchCreateMoveCursorEnd),
            KeyCode::Char(ch)
                if !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                Some(UiAction::BranchCreateInputChar(ch))
            }
            _ => None,
        };
    }

    if state.branches.delete_menu.active {
        return match code {
            KeyCode::Enter => Some(UiAction::ConfirmBranchDeleteMenu),
            KeyCode::Esc => Some(UiAction::CancelBranchDeleteMenu),
            KeyCode::Up | KeyCode::Char('k') => Some(UiAction::MoveBranchDeleteMenuUp),
            KeyCode::Down | KeyCode::Char('j') => Some(UiAction::MoveBranchDeleteMenuDown),
            _ => None,
        };
    }

    if state.branches.force_delete_confirm.active {
        return match code {
            KeyCode::Enter => Some(UiAction::ConfirmBranchForceDelete),
            KeyCode::Esc => Some(UiAction::CancelBranchForceDelete),
            _ => None,
        };
    }

    if state.branches.rebase_menu.active {
        return match code {
            KeyCode::Enter => Some(UiAction::ConfirmBranchRebaseMenu),
            KeyCode::Esc => Some(UiAction::CancelBranchRebaseMenu),
            KeyCode::Up | KeyCode::Char('k') => Some(UiAction::MoveBranchRebaseMenuUp),
            KeyCode::Down | KeyCode::Char('j') => Some(UiAction::MoveBranchRebaseMenuDown),
            _ => None,
        };
    }

    if state.branches.auto_stash_confirm.active {
        return match code {
            KeyCode::Enter => Some(UiAction::ConfirmAutoStash),
            KeyCode::Esc => Some(UiAction::CancelAutoStash),
            _ => None,
        };
    }

    if state.reset_menu.active {
        return match code {
            KeyCode::Enter => Some(UiAction::ConfirmResetMenu),
            KeyCode::Esc => Some(UiAction::CancelResetMenu),
            KeyCode::Up | KeyCode::Char('k') => Some(UiAction::MoveResetMenuUp),
            KeyCode::Down | KeyCode::Char('j') => Some(UiAction::MoveResetMenuDown),
            _ => None,
        };
    }

    if state.discard_confirm.active {
        return match code {
            KeyCode::Enter => Some(UiAction::ConfirmDiscard),
            KeyCode::Esc => Some(UiAction::CancelDiscard),
            _ => None,
        };
    }

    if search_input_is_current(state) {
        return match code {
            KeyCode::Enter => Some(UiAction::ConfirmSearch),
            KeyCode::Esc => Some(UiAction::CancelSearch),
            KeyCode::Backspace => Some(UiAction::BackspaceSearch),
            KeyCode::Char(ch) => Some(UiAction::InputSearchChar(ch)),
            _ => None,
        };
    }

    if search_query_is_current(state) {
        match code {
            KeyCode::Char('n') => return Some(UiAction::NextSearchMatch),
            KeyCode::Char('N') => return Some(UiAction::PrevSearchMatch),
            KeyCode::Esc => return Some(UiAction::CancelSearch),
            _ => {}
        }
    }

    if state.active_search_scope().is_some() && code == KeyCode::Char('/') {
        return Some(UiAction::StartSearch);
    }

    if state.focus == PanelFocus::Files {
        if state.files.mode == FileInputMode::MultiSelect && code == KeyCode::Esc {
            return Some(UiAction::ExitFilesMultiSelect);
        }
        match code {
            // TODO(files-hunks): map Enter to hunk-level editing/partial stage workflow.
            KeyCode::Enter => return Some(UiAction::ToggleSelectedDirectory),
            KeyCode::Char(' ') => return Some(UiAction::ToggleSelectedFileStage),
            KeyCode::Char('v') if state.files.mode != FileInputMode::MultiSelect => {
                return Some(UiAction::EnterFilesMultiSelect);
            }
            KeyCode::Char('c') => return Some(UiAction::OpenCommitEditor),
            KeyCode::Char('s') => return Some(UiAction::OpenStashEditor),
            KeyCode::Char('d') => return Some(UiAction::OpenDiscardConfirm),
            KeyCode::Char('D') => return Some(UiAction::OpenResetMenu),
            _ => {}
        }
    }

    if state.focus == PanelFocus::Branches {
        if state.branches.mode == BranchInputMode::MultiSelect && code == KeyCode::Esc {
            return Some(UiAction::ExitBranchesMultiSelect);
        }
        match code {
            KeyCode::Char(' ') => return Some(UiAction::CheckoutSelectedBranch),
            KeyCode::Char('v') if state.branches.mode != BranchInputMode::MultiSelect => {
                return Some(UiAction::EnterBranchesMultiSelect);
            }
            KeyCode::Char('n') => return Some(UiAction::OpenBranchCreateInput),
            KeyCode::Char('d') => return Some(UiAction::OpenBranchDeleteMenu),
            KeyCode::Char('r') => return Some(UiAction::OpenBranchRebaseMenu),
            _ => {}
        }
    }

    if state.focus == PanelFocus::Commits && state.commits.files.active {
        if state.commits.files.mode == FileInputMode::MultiSelect && code == KeyCode::Esc {
            return Some(UiAction::ExitCommitFilesMultiSelect);
        }
        if code == KeyCode::Esc {
            return Some(UiAction::CloseCommitFilesPanel);
        }
        if state.commits.files.mode != FileInputMode::MultiSelect && code == KeyCode::Char('v') {
            return Some(UiAction::EnterCommitFilesMultiSelect);
        }
        if code == KeyCode::Enter {
            return Some(UiAction::ToggleCommitFilesDirectory);
        }
        // TODO(commit-files-shortcuts): add more commit-files local file actions in a later slice.
    } else if state.focus == PanelFocus::Commits {
        if state.commits.mode == CommitInputMode::MultiSelect && code == KeyCode::Esc {
            return Some(UiAction::ExitCommitsMultiSelect);
        }
        match code {
            KeyCode::Enter => return Some(UiAction::OpenCommitFilesPanel),
            KeyCode::Char(' ') => return Some(UiAction::CheckoutSelectedCommitDetached),
            KeyCode::Char('c') => return Some(UiAction::OpenCommitEditor),
            KeyCode::Char('v') if state.commits.mode != CommitInputMode::MultiSelect => {
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
        KeyCode::Char('p') => Some(UiAction::StashPush {
            message: "savepoint".to_string(),
        }),
        KeyCode::Char('O') => Some(UiAction::StashPopSelected),
        KeyCode::Tab | KeyCode::BackTab => None,
        _ => None,
    }
}

fn search_input_is_current(state: &AppState) -> bool {
    state
        .active_search_scope()
        .is_some_and(|scope| state.search.is_input_active_for(scope))
}

fn search_query_is_current(state: &AppState) -> bool {
    state
        .active_search_scope()
        .is_some_and(|scope| state.search.has_query_for(scope))
}

#[cfg(test)]
#[path = "input_tests.rs"]
mod input_tests;
