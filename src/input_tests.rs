use super::*;

mod tests {
    use super::*;

    const TEST_DETAILS_SCROLL_LINES: usize = 7;
    const TEST_DETAILS_VISIBLE_LINES: usize = 18;

    fn map_key(state: &AppState, code: KeyCode) -> Option<UiAction> {
        ui_action_for_key(
            state,
            code,
            KeyModifiers::NONE,
            TEST_DETAILS_SCROLL_LINES,
            TEST_DETAILS_VISIBLE_LINES,
        )
    }

    fn active_commit_editor_state() -> AppState {
        let mut state = AppState::default();
        state.editor.kind = Some(ratagit_core::EditorKind::Commit {
            message: String::new(),
            message_cursor: 0,
            body: String::new(),
            body_cursor: 0,
            active_field: ratagit_core::CommitField::Body,
            intent: ratagit_core::CommitEditorIntent::Create,
        });
        state
    }

    fn active_reset_menu_state() -> AppState {
        let mut state = AppState::default();
        state.reset_menu.active = true;
        state
    }

    fn active_discard_confirm_state() -> AppState {
        let mut state = AppState::default();
        state.discard_confirm.active = true;
        state.discard_confirm.paths = vec!["a.txt".to_string()];
        state
    }

    fn active_branch_create_state() -> AppState {
        let mut state = AppState::default();
        state.branches.create.active = true;
        state
    }

    #[test]
    fn panel_navigation_uses_h_and_l_not_tab() {
        let state = AppState::default();
        assert_eq!(
            map_key(&state, KeyCode::Char('l')),
            Some(UiAction::FocusNext)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('h')),
            Some(UiAction::FocusPrev)
        );
        assert_eq!(map_key(&state, KeyCode::Tab), None);
        assert_eq!(map_key(&state, KeyCode::BackTab), None);
    }

    #[test]
    fn search_input_maps_text_until_confirm_or_escape() {
        let mut state = AppState::default();
        state.search.active = true;
        state.search.scope = state.active_search_scope();
        assert_eq!(
            map_key(&state, KeyCode::Char('r')),
            Some(UiAction::InputSearchChar('r'))
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('D')),
            Some(UiAction::InputSearchChar('D'))
        );
        assert_eq!(
            map_key(&state, KeyCode::Enter),
            Some(UiAction::ConfirmSearch)
        );
        assert_eq!(map_key(&state, KeyCode::Esc), Some(UiAction::CancelSearch));
    }

    #[test]
    fn files_panel_git_keys_map_to_file_actions() {
        let state = AppState::default();
        assert_eq!(
            map_key(&state, KeyCode::Char(' ')),
            Some(UiAction::ToggleSelectedFileStage)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('c')),
            Some(UiAction::OpenCommitEditor)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('s')),
            Some(UiAction::OpenStashEditor)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('d')),
            Some(UiAction::OpenDiscardConfirm)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('D')),
            Some(UiAction::OpenResetMenu)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('/')),
            Some(UiAction::StartSearch)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('v')),
            Some(UiAction::ToggleFilesMultiSelect)
        );
    }

    #[test]
    fn focused_panel_git_keys_map_to_panel_actions() {
        let mut state = AppState {
            focus: PanelFocus::Branches,
            last_left_focus: PanelFocus::Branches,
            ..AppState::default()
        };
        assert_eq!(
            map_key(&state, KeyCode::Char('n')),
            Some(UiAction::OpenBranchCreateInput)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char(' ')),
            Some(UiAction::CheckoutSelectedBranch)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('d')),
            Some(UiAction::OpenBranchDeleteMenu)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('r')),
            Some(UiAction::OpenBranchRebaseMenu)
        );

        state.focus = PanelFocus::Commits;
        assert_eq!(
            map_key(&state, KeyCode::Char('s')),
            Some(UiAction::SquashSelectedCommits)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('f')),
            Some(UiAction::FixupSelectedCommits)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('r')),
            Some(UiAction::OpenCommitRewordEditor)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('d')),
            Some(UiAction::DeleteSelectedCommits)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char(' ')),
            Some(UiAction::CheckoutSelectedCommitDetached)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('v')),
            Some(UiAction::ToggleCommitsMultiSelect)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('c')),
            Some(UiAction::OpenCommitEditor)
        );
        assert_eq!(
            map_key(&state, KeyCode::Enter),
            Some(UiAction::OpenCommitFilesPanel)
        );
        state.commits.files.active = true;
        assert_eq!(
            map_key(&state, KeyCode::Esc),
            Some(UiAction::CloseCommitFilesPanel)
        );
        state.search.active = true;
        state.search.scope = state.active_search_scope();
        state.search.query = "lib".to_string();
        assert_eq!(map_key(&state, KeyCode::Esc), Some(UiAction::CancelSearch));
        state.search.clear();
        assert_eq!(
            map_key(&state, KeyCode::Enter),
            Some(UiAction::ToggleCommitFilesDirectory)
        );
        assert_eq!(map_key(&state, KeyCode::Char('s')), None);

        state.focus = PanelFocus::Stash;
        assert_eq!(
            map_key(&state, KeyCode::Char('p')),
            Some(UiAction::StashPush {
                message: "savepoint".to_string()
            })
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('O')),
            Some(UiAction::StashPopSelected)
        );
    }

    #[test]
    fn editor_mode_maps_keys_before_any_other_mode() {
        let mut state = active_commit_editor_state();
        state.search.active = true;
        state.search.scope = state.active_search_scope();

        assert_eq!(
            map_key(&state, KeyCode::Enter),
            Some(UiAction::EditorConfirm)
        );
        assert_eq!(
            map_key(&state, KeyCode::Tab),
            Some(UiAction::EditorNextField)
        );
        assert_eq!(
            map_key(&state, KeyCode::Left),
            Some(UiAction::EditorMoveCursorLeft)
        );
        assert_eq!(
            map_key(&state, KeyCode::Right),
            Some(UiAction::EditorMoveCursorRight)
        );
        assert_eq!(
            map_key(&state, KeyCode::Home),
            Some(UiAction::EditorMoveCursorHome)
        );
        assert_eq!(
            map_key(&state, KeyCode::End),
            Some(UiAction::EditorMoveCursorEnd)
        );
        assert_eq!(
            ui_action_for_key(
                &state,
                KeyCode::Char('j'),
                KeyModifiers::CONTROL,
                TEST_DETAILS_SCROLL_LINES,
                TEST_DETAILS_VISIBLE_LINES
            ),
            Some(UiAction::EditorInsertNewline)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('m')),
            Some(UiAction::EditorInputChar('m'))
        );
    }

    #[test]
    fn editor_mode_q_dispatches_instead_of_quitting() {
        let state = active_commit_editor_state();

        assert_eq!(
            key_effect_for_key(
                &state,
                KeyCode::Char('q'),
                KeyModifiers::NONE,
                TEST_DETAILS_SCROLL_LINES,
                TEST_DETAILS_VISIBLE_LINES
            ),
            KeyEffect::Dispatch(UiAction::EditorInputChar('q'))
        );
    }

    #[test]
    fn ctrl_u_and_ctrl_d_map_to_global_details_scroll() {
        let state = AppState::default();

        assert_eq!(
            ui_action_for_key(
                &state,
                KeyCode::Char('u'),
                KeyModifiers::CONTROL,
                TEST_DETAILS_SCROLL_LINES,
                TEST_DETAILS_VISIBLE_LINES
            ),
            Some(UiAction::DetailsScrollUp {
                lines: TEST_DETAILS_SCROLL_LINES
            })
        );
        assert_eq!(
            ui_action_for_key(
                &state,
                KeyCode::Char('d'),
                KeyModifiers::CONTROL,
                TEST_DETAILS_SCROLL_LINES,
                TEST_DETAILS_VISIBLE_LINES
            ),
            Some(UiAction::DetailsScrollDown {
                lines: TEST_DETAILS_SCROLL_LINES,
                visible_lines: TEST_DETAILS_VISIBLE_LINES
            })
        );
    }

    #[test]
    fn global_details_scroll_keys_work_while_editor_is_active() {
        let state = active_commit_editor_state();

        assert_eq!(
            ui_action_for_key(
                &state,
                KeyCode::Char('d'),
                KeyModifiers::CONTROL,
                TEST_DETAILS_SCROLL_LINES,
                TEST_DETAILS_VISIBLE_LINES
            ),
            Some(UiAction::DetailsScrollDown {
                lines: TEST_DETAILS_SCROLL_LINES,
                visible_lines: TEST_DETAILS_VISIBLE_LINES
            })
        );
    }

    #[test]
    fn reset_menu_maps_navigation_confirm_and_cancel_before_panels() {
        let state = active_reset_menu_state();

        assert_eq!(
            map_key(&state, KeyCode::Down),
            Some(UiAction::MoveResetMenuDown)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('j')),
            Some(UiAction::MoveResetMenuDown)
        );
        assert_eq!(
            map_key(&state, KeyCode::Up),
            Some(UiAction::MoveResetMenuUp)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('k')),
            Some(UiAction::MoveResetMenuUp)
        );
        assert_eq!(
            map_key(&state, KeyCode::Enter),
            Some(UiAction::ConfirmResetMenu)
        );
        assert_eq!(
            map_key(&state, KeyCode::Esc),
            Some(UiAction::CancelResetMenu)
        );
    }

    #[test]
    fn branch_create_input_maps_text_until_confirm_or_cancel() {
        let state = active_branch_create_state();

        assert_eq!(
            map_key(&state, KeyCode::Enter),
            Some(UiAction::ConfirmBranchCreate)
        );
        assert_eq!(
            map_key(&state, KeyCode::Backspace),
            Some(UiAction::BranchCreateBackspace)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('x')),
            Some(UiAction::BranchCreateInputChar('x'))
        );
        assert_eq!(
            map_key(&state, KeyCode::Esc),
            Some(UiAction::CancelBranchCreate)
        );
    }

    #[test]
    fn discard_confirm_maps_confirm_and_cancel_before_panels() {
        let state = active_discard_confirm_state();

        assert_eq!(
            map_key(&state, KeyCode::Enter),
            Some(UiAction::ConfirmDiscard)
        );
        assert_eq!(map_key(&state, KeyCode::Esc), Some(UiAction::CancelDiscard));
        assert_eq!(map_key(&state, KeyCode::Char('d')), None);
    }

    #[test]
    fn ctrl_c_quits_even_when_editor_is_active() {
        let state = active_commit_editor_state();

        assert_eq!(
            key_effect_for_key(
                &state,
                KeyCode::Char('c'),
                KeyModifiers::CONTROL,
                TEST_DETAILS_SCROLL_LINES,
                TEST_DETAILS_VISIBLE_LINES
            ),
            KeyEffect::Quit
        );
    }
}
