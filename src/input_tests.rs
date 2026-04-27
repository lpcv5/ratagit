use super::*;

mod tests {
    use super::*;

    const TEST_DETAILS_SCROLL_LINES: usize = 7;
    const TEST_DETAILS_VISIBLE_LINES: usize = 18;
    const TEST_LEFT_PANEL_VISIBLE_LINES: usize = 8;

    fn map_key(state: &AppContext, code: KeyCode) -> Option<UiAction> {
        ui_action_for_key(
            state,
            code,
            KeyModifiers::NONE,
            TEST_DETAILS_SCROLL_LINES,
            TEST_DETAILS_VISIBLE_LINES,
            TEST_LEFT_PANEL_VISIBLE_LINES,
        )
    }

    fn active_commit_editor_state() -> AppContext {
        let mut state = AppContext::default();
        state.ui.editor.kind = Some(ratagit_core::EditorKind::Commit {
            message: String::new(),
            message_cursor: 0,
            body: String::new(),
            body_cursor: 0,
            active_field: ratagit_core::CommitField::Body,
            intent: ratagit_core::CommitEditorIntent::Create,
        });
        state
    }

    fn active_reset_menu_state() -> AppContext {
        let mut state = AppContext::default();
        state.ui.reset_menu.active = true;
        state
    }

    fn active_discard_confirm_state() -> AppContext {
        let mut state = AppContext::default();
        state.ui.discard_confirm.active = true;
        state.ui.discard_confirm.paths = vec!["a.txt".to_string()];
        state
    }

    fn active_push_force_confirm_state() -> AppContext {
        let mut state = AppContext::default();
        state.ui.push_force_confirm.active = true;
        state.ui.push_force_confirm.reason = "non-fast-forward".to_string();
        state
    }

    fn active_branch_create_state() -> AppContext {
        let mut state = AppContext::default();
        state.ui.branches.create.active = true;
        state
    }

    fn active_branch_delete_menu_state() -> AppContext {
        let mut state = AppContext::default();
        state.ui.branches.delete_menu.active = true;
        state
    }

    fn active_branch_force_delete_state() -> AppContext {
        let mut state = AppContext::default();
        state.ui.branches.force_delete_confirm.active = true;
        state
    }

    fn active_branch_rebase_menu_state() -> AppContext {
        let mut state = AppContext::default();
        state.ui.branches.rebase_menu.active = true;
        state
    }

    fn active_auto_stash_confirm_state() -> AppContext {
        let mut state = AppContext::default();
        state.ui.branches.auto_stash_confirm.active = true;
        state.ui.branches.auto_stash_confirm.operation =
            Some(ratagit_core::AutoStashOperation::Rebase {
                target: "main".to_string(),
                interactive: false,
            });
        state
    }

    #[test]
    fn panel_navigation_uses_h_and_l_not_tab() {
        let state = AppContext::default();
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
        let mut state = AppContext::default();
        state.ui.search.active = true;
        state.ui.search.scope = state.active_search_scope();
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
    fn confirmed_search_query_maps_repeat_navigation_keys() {
        let mut state = AppContext::default();
        state.ui.search.active = false;
        state.ui.search.scope = state.active_search_scope();
        state.ui.search.query = "lib".to_string();
        state.ui.search.current_match = Some(0);

        assert_eq!(
            map_key(&state, KeyCode::Char('n')),
            Some(UiAction::NextSearchMatch)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('N')),
            Some(UiAction::PrevSearchMatch)
        );
        assert_eq!(map_key(&state, KeyCode::Esc), Some(UiAction::CancelSearch));
        assert_eq!(map_key(&state, KeyCode::Char('x')), None);
    }

    #[test]
    fn files_panel_git_keys_map_to_file_actions() {
        let state = AppContext::default();
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
            Some(UiAction::EnterFilesMultiSelect)
        );
    }

    #[test]
    fn files_multiselect_uses_escape_to_exit_and_ignores_v() {
        let mut state = AppContext::default();
        state.ui.files.mode = ratagit_core::FileInputMode::MultiSelect;

        assert_eq!(
            map_key(&state, KeyCode::Esc),
            Some(UiAction::ExitFilesMultiSelect)
        );
        assert_eq!(map_key(&state, KeyCode::Char('v')), None);
    }

    #[test]
    fn focused_panel_git_keys_map_to_panel_actions() {
        let mut state = AppContext::default();
        state.ui.focus = PanelFocus::Branches;
        state.ui.last_left_focus = PanelFocus::Branches;
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
        assert_eq!(
            map_key(&state, KeyCode::Char('v')),
            Some(UiAction::EnterBranchesMultiSelect)
        );

        state.ui.focus = PanelFocus::Commits;
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
            Some(UiAction::EnterCommitsMultiSelect)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('c')),
            Some(UiAction::OpenCommitEditor)
        );
        assert_eq!(
            map_key(&state, KeyCode::Enter),
            Some(UiAction::OpenCommitFilesPanel)
        );
        state.ui.commits.files.active = true;
        assert_eq!(
            map_key(&state, KeyCode::Esc),
            Some(UiAction::CloseCommitFilesPanel)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('v')),
            Some(UiAction::EnterCommitFilesMultiSelect)
        );
        state.ui.search.active = true;
        state.ui.search.scope = state.active_search_scope();
        state.ui.search.query = "lib".to_string();
        assert_eq!(map_key(&state, KeyCode::Esc), Some(UiAction::CancelSearch));
        state.ui.search.clear();
        assert_eq!(
            map_key(&state, KeyCode::Enter),
            Some(UiAction::ToggleCommitFilesDirectory)
        );
        assert_eq!(map_key(&state, KeyCode::Char('s')), None);
        state.ui.commits.files.mode = ratagit_core::FileInputMode::MultiSelect;
        assert_eq!(
            map_key(&state, KeyCode::Esc),
            Some(UiAction::ExitCommitFilesMultiSelect)
        );
        assert_eq!(map_key(&state, KeyCode::Char('v')), None);

        state.ui.commits.files.active = false;
        state.ui.commits.mode = ratagit_core::CommitInputMode::MultiSelect;
        assert_eq!(
            map_key(&state, KeyCode::Esc),
            Some(UiAction::ExitCommitsMultiSelect)
        );
        assert_eq!(map_key(&state, KeyCode::Char('v')), None);

        state.ui.focus = PanelFocus::Stash;
        assert_eq!(map_key(&state, KeyCode::Char('p')), Some(UiAction::Pull));
        assert_eq!(map_key(&state, KeyCode::Char('P')), Some(UiAction::Push));
        assert_eq!(
            map_key(&state, KeyCode::Char('O')),
            Some(UiAction::StashPopSelected)
        );
    }

    #[test]
    fn branches_multiselect_uses_escape_to_exit_and_ignores_v() {
        let mut state = AppContext::default();
        state.ui.focus = PanelFocus::Branches;
        state.ui.last_left_focus = PanelFocus::Branches;
        state.ui.branches.mode = ratagit_core::BranchInputMode::MultiSelect;

        assert_eq!(
            map_key(&state, KeyCode::Esc),
            Some(UiAction::ExitBranchesMultiSelect)
        );
        assert_eq!(map_key(&state, KeyCode::Char('v')), None);
    }

    #[test]
    fn editor_mode_maps_keys_before_any_other_mode() {
        let mut state = active_commit_editor_state();
        state.ui.search.active = true;
        state.ui.search.scope = state.active_search_scope();

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
                TEST_DETAILS_VISIBLE_LINES,
                TEST_LEFT_PANEL_VISIBLE_LINES,
            ),
            Some(UiAction::EditorInsertNewline)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('m')),
            Some(UiAction::EditorInputChar('m'))
        );
        assert_eq!(map_key(&state, KeyCode::Esc), Some(UiAction::EditorCancel));
        assert_eq!(
            map_key(&state, KeyCode::Backspace),
            Some(UiAction::EditorBackspace)
        );
        assert_eq!(
            map_key(&state, KeyCode::BackTab),
            Some(UiAction::EditorPrevField)
        );
        assert_eq!(map_key(&state, KeyCode::F(1)), None);
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
                TEST_DETAILS_VISIBLE_LINES,
                TEST_LEFT_PANEL_VISIBLE_LINES,
            ),
            KeyEffect::Dispatch(UiAction::EditorInputChar('q'))
        );
    }

    #[test]
    fn ctrl_u_and_ctrl_d_map_to_global_details_scroll() {
        let state = AppContext::default();

        assert_eq!(
            ui_action_for_key(
                &state,
                KeyCode::Char('u'),
                KeyModifiers::CONTROL,
                TEST_DETAILS_SCROLL_LINES,
                TEST_DETAILS_VISIBLE_LINES,
                TEST_LEFT_PANEL_VISIBLE_LINES,
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
                TEST_DETAILS_VISIBLE_LINES,
                TEST_LEFT_PANEL_VISIBLE_LINES,
            ),
            Some(UiAction::DetailsScrollDown {
                lines: TEST_DETAILS_SCROLL_LINES,
                visible_lines: TEST_DETAILS_VISIBLE_LINES
            })
        );
    }

    #[test]
    fn p_and_shift_p_map_to_global_pull_and_push() {
        let mut state = AppContext::default();
        assert_eq!(map_key(&state, KeyCode::Char('p')), Some(UiAction::Pull));
        assert_eq!(map_key(&state, KeyCode::Char('P')), Some(UiAction::Push));

        state.ui.focus = PanelFocus::Details;
        assert_eq!(map_key(&state, KeyCode::Char('p')), Some(UiAction::Pull));
        assert_eq!(map_key(&state, KeyCode::Char('P')), Some(UiAction::Push));
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
                TEST_DETAILS_VISIBLE_LINES,
                TEST_LEFT_PANEL_VISIBLE_LINES,
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
    fn force_push_confirm_maps_confirm_and_cancel_before_global_keys() {
        let state = active_push_force_confirm_state();

        assert_eq!(
            map_key(&state, KeyCode::Enter),
            Some(UiAction::ConfirmForcePush)
        );
        assert_eq!(
            map_key(&state, KeyCode::Esc),
            Some(UiAction::CancelForcePush)
        );
        assert_eq!(map_key(&state, KeyCode::Char('p')), None);
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
            map_key(&state, KeyCode::Left),
            Some(UiAction::BranchCreateMoveCursorLeft)
        );
        assert_eq!(
            map_key(&state, KeyCode::Right),
            Some(UiAction::BranchCreateMoveCursorRight)
        );
        assert_eq!(
            map_key(&state, KeyCode::Home),
            Some(UiAction::BranchCreateMoveCursorHome)
        );
        assert_eq!(
            map_key(&state, KeyCode::End),
            Some(UiAction::BranchCreateMoveCursorEnd)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('x')),
            Some(UiAction::BranchCreateInputChar('x'))
        );
        assert_eq!(
            map_key(&state, KeyCode::Esc),
            Some(UiAction::CancelBranchCreate)
        );
        assert_eq!(map_key(&state, KeyCode::F(1)), None);
    }

    #[test]
    fn branch_modals_map_navigation_confirm_and_cancel_before_panels() {
        let state = active_branch_delete_menu_state();
        assert_eq!(
            map_key(&state, KeyCode::Enter),
            Some(UiAction::ConfirmBranchDeleteMenu)
        );
        assert_eq!(
            map_key(&state, KeyCode::Esc),
            Some(UiAction::CancelBranchDeleteMenu)
        );
        assert_eq!(
            map_key(&state, KeyCode::Up),
            Some(UiAction::MoveBranchDeleteMenuUp)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('k')),
            Some(UiAction::MoveBranchDeleteMenuUp)
        );
        assert_eq!(
            map_key(&state, KeyCode::Down),
            Some(UiAction::MoveBranchDeleteMenuDown)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('j')),
            Some(UiAction::MoveBranchDeleteMenuDown)
        );
        assert_eq!(map_key(&state, KeyCode::Char('d')), None);

        let state = active_branch_force_delete_state();
        assert_eq!(
            map_key(&state, KeyCode::Enter),
            Some(UiAction::ConfirmBranchForceDelete)
        );
        assert_eq!(
            map_key(&state, KeyCode::Esc),
            Some(UiAction::CancelBranchForceDelete)
        );
        assert_eq!(map_key(&state, KeyCode::Char('d')), None);

        let state = active_branch_rebase_menu_state();
        assert_eq!(
            map_key(&state, KeyCode::Enter),
            Some(UiAction::ConfirmBranchRebaseMenu)
        );
        assert_eq!(
            map_key(&state, KeyCode::Esc),
            Some(UiAction::CancelBranchRebaseMenu)
        );
        assert_eq!(
            map_key(&state, KeyCode::Up),
            Some(UiAction::MoveBranchRebaseMenuUp)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('k')),
            Some(UiAction::MoveBranchRebaseMenuUp)
        );
        assert_eq!(
            map_key(&state, KeyCode::Down),
            Some(UiAction::MoveBranchRebaseMenuDown)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('j')),
            Some(UiAction::MoveBranchRebaseMenuDown)
        );
        assert_eq!(map_key(&state, KeyCode::Char('r')), None);

        let state = active_auto_stash_confirm_state();
        assert_eq!(
            map_key(&state, KeyCode::Enter),
            Some(UiAction::ConfirmAutoStash)
        );
        assert_eq!(
            map_key(&state, KeyCode::Esc),
            Some(UiAction::CancelAutoStash)
        );
        assert_eq!(map_key(&state, KeyCode::Char('r')), None);
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
                TEST_DETAILS_VISIBLE_LINES,
                TEST_LEFT_PANEL_VISIBLE_LINES,
            ),
            KeyEffect::Quit
        );
    }

    #[test]
    fn key_effect_handles_plain_quit_ignored_and_global_navigation() {
        let state = AppContext::default();

        assert_eq!(
            key_effect_for_key(
                &state,
                KeyCode::Char('q'),
                KeyModifiers::NONE,
                TEST_DETAILS_SCROLL_LINES,
                TEST_DETAILS_VISIBLE_LINES,
                TEST_LEFT_PANEL_VISIBLE_LINES,
            ),
            KeyEffect::Quit
        );
        assert_eq!(
            key_effect_for_key(
                &state,
                KeyCode::F(1),
                KeyModifiers::NONE,
                TEST_DETAILS_SCROLL_LINES,
                TEST_DETAILS_VISIBLE_LINES,
                TEST_LEFT_PANEL_VISIBLE_LINES,
            ),
            KeyEffect::Ignore
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('r')),
            Some(UiAction::RefreshAll)
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('j')),
            Some(UiAction::MoveDownInViewport {
                visible_lines: TEST_LEFT_PANEL_VISIBLE_LINES
            })
        );
        assert_eq!(
            map_key(&state, KeyCode::Down),
            Some(UiAction::MoveDownInViewport {
                visible_lines: TEST_LEFT_PANEL_VISIBLE_LINES
            })
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('k')),
            Some(UiAction::MoveUpInViewport {
                visible_lines: TEST_LEFT_PANEL_VISIBLE_LINES
            })
        );
        assert_eq!(
            map_key(&state, KeyCode::Up),
            Some(UiAction::MoveUpInViewport {
                visible_lines: TEST_LEFT_PANEL_VISIBLE_LINES
            })
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('1')),
            Some(UiAction::FocusPanel {
                panel: PanelFocus::Files
            })
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('2')),
            Some(UiAction::FocusPanel {
                panel: PanelFocus::Branches
            })
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('3')),
            Some(UiAction::FocusPanel {
                panel: PanelFocus::Commits
            })
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('4')),
            Some(UiAction::FocusPanel {
                panel: PanelFocus::Stash
            })
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('5')),
            Some(UiAction::FocusPanel {
                panel: PanelFocus::Details
            })
        );
        assert_eq!(
            map_key(&state, KeyCode::Char('6')),
            Some(UiAction::FocusPanel {
                panel: PanelFocus::Log
            })
        );
    }
}
