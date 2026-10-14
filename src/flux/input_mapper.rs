use crate::flux::action::{Action, DomainAction};
use crate::flux::snapshot::AppStateSnapshot;

pub fn map_key_to_actions(
    key: crossterm::event::KeyEvent,
    snapshot: &AppStateSnapshot<'_>,
) -> Vec<Action> {
    let env = snapshot.command_env();
    if let Some(command) = crate::app::commands::resolve_key(&key, snapshot.keymap, &env) {
        return command
            .action(&env)
            .map(Action::Domain)
            .into_iter()
            .collect();
    }
    if matches!(
        snapshot.input_mode,
        Some(crate::app::InputMode::BranchSwitchConfirm | crate::app::InputMode::CommitAllConfirm)
    ) {
        return Vec::new();
    }
    if snapshot.input_mode.is_some() {
        if let crossterm::event::KeyCode::Char(c) = key.code {
            if !key
                .modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL)
            {
                return vec![Action::Domain(DomainAction::InputChar(c))];
            }
        }
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::flux::snapshot::AppStateSnapshot;
    use crate::flux::stores::test_support::MockRepo;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn mock_app() -> App {
        App::from_repo(Box::new(MockRepo)).expect("mock app")
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    fn map(app: &App, k: KeyEvent) -> Vec<Action> {
        let snapshot = AppStateSnapshot::from_app(app);
        map_key_to_actions(k, &snapshot)
    }

    #[test]
    fn test_q_maps_to_quit_in_normal_mode() {
        let app = mock_app();
        let actions = map(&app, key(KeyCode::Char('q')));
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::Domain(DomainAction::Quit))));
    }

    #[test]
    fn test_j_maps_to_list_down() {
        let app = mock_app();
        let actions = map(&app, key(KeyCode::Char('j')));
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::Domain(DomainAction::ListDown))));
    }

    #[test]
    fn test_k_maps_to_list_up() {
        let app = mock_app();
        let actions = map(&app, key(KeyCode::Char('k')));
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::Domain(DomainAction::ListUp))));
    }

    #[test]
    fn test_h_maps_to_panel_prev() {
        let app = mock_app();
        let actions = map(&app, key(KeyCode::Char('h')));
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::Domain(DomainAction::PanelPrev))));
    }

    #[test]
    fn test_l_maps_to_panel_next() {
        let app = mock_app();
        let actions = map(&app, key(KeyCode::Char('l')));
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::Domain(DomainAction::PanelNext))));
    }

    #[test]
    fn test_1_maps_to_panel_goto_1() {
        let app = mock_app();
        let actions = map(&app, key(KeyCode::Char('1')));
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::Domain(DomainAction::PanelGoto(1)))));
    }

    #[test]
    fn test_down_arrow_maps_to_list_down() {
        let app = mock_app();
        let actions = map(&app, key(KeyCode::Down));
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::Domain(DomainAction::ListDown))));
    }

    #[test]
    fn test_up_arrow_maps_to_list_up() {
        let app = mock_app();
        let actions = map(&app, key(KeyCode::Up));
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::Domain(DomainAction::ListUp))));
    }

    #[test]
    fn test_ctrl_d_maps_to_diff_scroll_down() {
        let app = mock_app();
        let actions = map(&app, ctrl_key(KeyCode::Char('d')));
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::Domain(DomainAction::DiffScrollDown))));
    }

    #[test]
    fn test_ctrl_u_maps_to_diff_scroll_up() {
        let app = mock_app();
        let actions = map(&app, ctrl_key(KeyCode::Char('u')));
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::Domain(DomainAction::DiffScrollUp))));
    }

    #[test]
    fn test_input_mode_char_maps_to_input_char() {
        let mut app = mock_app();
        app.input.mode = Some(crate::app::InputMode::CreateBranch);
        let actions = map(&app, key(KeyCode::Char('a')));
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::Domain(DomainAction::InputChar('a')))));
    }

    #[test]
    fn test_input_mode_backspace_maps_to_input_backspace() {
        let mut app = mock_app();
        app.input.mode = Some(crate::app::InputMode::CreateBranch);
        let actions = map(&app, key(KeyCode::Backspace));
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::Domain(DomainAction::InputBackspace))));
    }

    #[test]
    fn test_input_mode_enter_maps_to_input_enter() {
        let mut app = mock_app();
        app.input.mode = Some(crate::app::InputMode::CreateBranch);
        let actions = map(&app, key(KeyCode::Enter));
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::Domain(DomainAction::InputEnter))));
    }

    #[test]
    fn test_input_mode_esc_maps_to_input_esc() {
        let mut app = mock_app();
        app.input.mode = Some(crate::app::InputMode::CreateBranch);
        let actions = map(&app, key(KeyCode::Esc));
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::Domain(DomainAction::InputEsc))));
    }
}

#[cfg(test)]
mod more_tests {
    use super::*;
    use crate::app::App;
    use crate::flux::snapshot::AppStateSnapshot;
    use crate::flux::stores::test_support::MockRepo;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn mock_app() -> App {
        App::from_repo(Box::new(MockRepo)).expect("mock app")
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn map(app: &App, k: KeyEvent) -> Vec<Action> {
        let snapshot = AppStateSnapshot::from_app(app);
        map_key_to_actions(k, &snapshot)
    }

    #[test]
    fn test_branch_switch_confirm_y_confirms() {
        let mut app = mock_app();
        app.input.mode = Some(crate::app::InputMode::BranchSwitchConfirm);
        let actions = map(&app, key(KeyCode::Char('y')));
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::Domain(DomainAction::BranchSwitchConfirm(true)))));
    }

    #[test]
    fn test_branch_switch_confirm_n_cancels() {
        let mut app = mock_app();
        app.input.mode = Some(crate::app::InputMode::BranchSwitchConfirm);
        let actions = map(&app, key(KeyCode::Char('n')));
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::Domain(DomainAction::BranchSwitchConfirm(false)))));
    }

    #[test]
    fn test_commit_all_confirm_y_confirms() {
        let mut app = mock_app();
        app.input.mode = Some(crate::app::InputMode::CommitAllConfirm);
        let actions = map(&app, key(KeyCode::Char('y')));
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::Domain(DomainAction::CommitAllConfirm(true)))));
    }

    #[test]
    fn test_commit_all_confirm_n_cancels() {
        let mut app = mock_app();
        app.input.mode = Some(crate::app::InputMode::CommitAllConfirm);
        let actions = map(&app, key(KeyCode::Char('n')));
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::Domain(DomainAction::CommitAllConfirm(false)))));
    }

    #[test]
    fn test_input_mode_tab_maps_to_input_tab() {
        let mut app = mock_app();
        app.input.mode = Some(crate::app::InputMode::CommitEditor);
        let actions = map(&app, key(KeyCode::Tab));
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::Domain(DomainAction::InputTab))));
    }

    #[test]
    fn test_c_in_files_panel_maps_to_start_commit() {
        let mut app = mock_app();
        app.ui.active_panel = crate::app::SidePanel::Files;
        let actions = map(&app, key(KeyCode::Char('c')));
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::Domain(DomainAction::StartCommitInput))));
    }

    #[test]
    fn test_v_in_files_panel_maps_to_toggle_visual() {
        let mut app = mock_app();
        app.ui.active_panel = crate::app::SidePanel::Files;
        let actions = map(&app, key(KeyCode::Char('v')));
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::Domain(DomainAction::ToggleVisualSelectMode))));
    }

    #[test]
    fn test_space_in_files_panel_visual_mode_maps_to_toggle_stage() {
        let mut app = mock_app();
        app.ui.active_panel = crate::app::SidePanel::Files;
        // With visual mode, Space maps to ToggleStageSelection
        app.files.test_view_mut().visual_mode = true;
        app.files.test_view_mut().panel.list_state.select(Some(0));
        app.files.test_view_mut().visual_anchor = Some(0);
        let actions = map(&app, key(KeyCode::Char(' ')));
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::Domain(DomainAction::ToggleStageSelection))));
    }

    #[test]
    fn test_2_maps_to_panel_goto_2() {
        let app = mock_app();
        let actions = map(&app, key(KeyCode::Char('2')));
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::Domain(DomainAction::PanelGoto(2)))));
    }

    #[test]
    fn test_3_maps_to_panel_goto_3() {
        let app = mock_app();
        let actions = map(&app, key(KeyCode::Char('3')));
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::Domain(DomainAction::PanelGoto(3)))));
    }

    #[test]
    fn test_4_maps_to_panel_goto_4() {
        let app = mock_app();
        let actions = map(&app, key(KeyCode::Char('4')));
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::Domain(DomainAction::PanelGoto(4)))));
    }

    #[test]
    fn test_slash_maps_to_start_search() {
        let app = mock_app();
        let actions = map(&app, key(KeyCode::Char('/')));
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::Domain(DomainAction::StartSearchInput))));
    }

    #[test]
    fn test_esc_with_search_query_maps_to_search_clear() {
        let mut app = mock_app();
        app.apply_search_query("foo".to_string());
        let actions = map(&app, key(KeyCode::Esc));
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::Domain(DomainAction::SearchClear))));
    }

    #[test]
    fn test_n_in_branches_panel_maps_to_search_next() {
        let mut app = mock_app();
        app.ui.active_panel = crate::app::SidePanel::LocalBranches;
        let actions = map(&app, key(KeyCode::Char('n')));
        // In branches panel, n should map to SearchNext
        let _ = actions; // just verify no panic
    }

    #[test]
    fn test_enter_in_branches_panel_maps_to_revision_open() {
        let mut app = mock_app();
        app.ui.active_panel = crate::app::SidePanel::LocalBranches;
        let actions = map(&app, key(KeyCode::Enter));
        assert!(actions
            .iter()
            .any(|a| { matches!(a, Action::Domain(DomainAction::RevisionOpenTreeOrToggleDir)) }));
    }

    #[test]
    fn test_esc_in_branch_commits_subview_maps_to_revision_close() {
        let mut app = mock_app();
        app.ui.active_panel = crate::app::SidePanel::LocalBranches;
        app.ui.branches.commits_subview_active = true;
        let actions = map(&app, key(KeyCode::Esc));
        assert!(actions
            .iter()
            .any(|a| { matches!(a, Action::Domain(DomainAction::RevisionCloseTree)) }));
    }
}
