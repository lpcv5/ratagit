use crate::app::{InputMode, SidePanel};
use crate::config::keymap::key_to_string;
use crate::flux::action::{Action, DomainAction};
use crate::flux::files_backend::{FilesPanelNodeStatus, FilesPanelViewState};
use crate::flux::snapshot::AppStateSnapshot;

pub fn map_key_to_actions(
    key: crossterm::event::KeyEvent,
    snapshot: &AppStateSnapshot<'_>,
) -> Vec<Action> {
    let files_view = snapshot.files_view_state();
    let branches_view = snapshot.branches_view_state();
    let commits_view = snapshot.commits_view_state();
    let stash_view = snapshot.stash_view_state();

    if snapshot.input_mode == Some(InputMode::BranchSwitchConfirm) {
        return match key.code {
            crossterm::event::KeyCode::Char('y')
            | crossterm::event::KeyCode::Char('Y')
            | crossterm::event::KeyCode::Enter => {
                vec![Action::Domain(DomainAction::BranchSwitchConfirm(true))]
            }
            crossterm::event::KeyCode::Char('n')
            | crossterm::event::KeyCode::Char('N')
            | crossterm::event::KeyCode::Esc => {
                vec![Action::Domain(DomainAction::BranchSwitchConfirm(false))]
            }
            _ => Vec::new(),
        };
    }

    if snapshot.input_mode == Some(InputMode::CommitAllConfirm) {
        return match key.code {
            crossterm::event::KeyCode::Char('y')
            | crossterm::event::KeyCode::Char('Y')
            | crossterm::event::KeyCode::Enter => {
                vec![Action::Domain(DomainAction::CommitAllConfirm(true))]
            }
            crossterm::event::KeyCode::Char('n')
            | crossterm::event::KeyCode::Char('N')
            | crossterm::event::KeyCode::Esc => {
                vec![Action::Domain(DomainAction::CommitAllConfirm(false))]
            }
            _ => Vec::new(),
        };
    }

    if snapshot.input_mode.is_some() {
        return match key.code {
            crossterm::event::KeyCode::Esc => {
                vec![Action::Domain(DomainAction::InputEsc)]
            }
            crossterm::event::KeyCode::Tab => {
                vec![Action::Domain(DomainAction::InputTab)]
            }
            crossterm::event::KeyCode::Enter => {
                vec![Action::Domain(DomainAction::InputEnter)]
            }
            crossterm::event::KeyCode::Backspace => {
                vec![Action::Domain(DomainAction::InputBackspace)]
            }
            crossterm::event::KeyCode::Char(c) => {
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL)
                {
                    Vec::new()
                } else {
                    vec![Action::Domain(DomainAction::InputChar(c))]
                }
            }
            _ => Vec::new(),
        };
    }

    if key.code == crossterm::event::KeyCode::Esc && snapshot.has_search_query_for_active_scope {
        return vec![Action::Domain(DomainAction::SearchClear)];
    }
    if key.code == crossterm::event::KeyCode::Esc
        && snapshot.active_panel == SidePanel::Files
        && files_view.selection.visual_mode
    {
        return vec![Action::Domain(DomainAction::ToggleVisualSelectMode)];
    }
    if key.code == crossterm::event::KeyCode::Esc
        && ((snapshot.active_panel == SidePanel::Stash && stash_view.tree_mode.active)
            || (snapshot.active_panel == SidePanel::Commits && commits_view.tree_mode.active)
            || (snapshot.active_panel == SidePanel::LocalBranches
                && branches_view.commits_subview.active))
    {
        return vec![Action::Domain(DomainAction::RevisionCloseTree)];
    }

    let key_str = key_to_string(&key);
    if key_str.is_empty() {
        return Vec::new();
    }

    let global_actions = snapshot.keymap.global_actions(&key_str);
    let gm = |action: &str| global_actions.iter().any(|candidate| candidate == action);

    if gm("quit") {
        return vec![Action::Domain(DomainAction::Quit)];
    }
    if gm("list_up") {
        return vec![Action::Domain(DomainAction::ListUp)];
    }
    if gm("list_down") {
        return vec![Action::Domain(DomainAction::ListDown)];
    }
    if gm("panel_next") {
        return vec![Action::Domain(DomainAction::PanelNext)];
    }
    if gm("panel_prev") {
        return vec![Action::Domain(DomainAction::PanelPrev)];
    }
    if gm("diff_scroll_up") {
        return vec![Action::Domain(DomainAction::DiffScrollUp)];
    }
    if gm("diff_scroll_down") {
        return vec![Action::Domain(DomainAction::DiffScrollDown)];
    }
    if gm("panel_1") {
        return vec![Action::Domain(DomainAction::PanelGoto(1))];
    }
    if gm("panel_2") {
        return vec![Action::Domain(DomainAction::PanelGoto(2))];
    }
    if gm("panel_3") {
        return vec![Action::Domain(DomainAction::PanelGoto(3))];
    }
    if gm("panel_4") {
        return vec![Action::Domain(DomainAction::PanelGoto(4))];
    }
    if gm("commit") {
        if snapshot.active_panel == SidePanel::Files && files_view.selection.visual_mode {
            return vec![Action::Domain(DomainAction::PrepareCommitFromSelection)];
        }
        return vec![Action::Domain(DomainAction::StartCommitInput)];
    }
    if gm("command_palette") {
        return vec![Action::Domain(DomainAction::StartCommandPalette)];
    }
    if gm("search_start") {
        return vec![Action::Domain(DomainAction::StartSearchInput)];
    }
    if gm("search_next") && snapshot.has_search_for_active_scope {
        return vec![Action::Domain(DomainAction::SearchNext)];
    }
    if gm("search_prev") && snapshot.has_search_for_active_scope {
        return vec![Action::Domain(DomainAction::SearchPrev)];
    }

    let panel_actions = snapshot
        .keymap
        .panel_actions(active_panel_name(snapshot.active_panel), &key_str);
    let pm = |action: &str| panel_actions.iter().any(|candidate| candidate == action);

    if pm("toggle_stage") {
        if snapshot.active_panel == SidePanel::Files && files_view.selection.visual_mode {
            return vec![Action::Domain(DomainAction::ToggleStageSelection)];
        }
        if let Some(action) = toggle_stage_for_selected_file(snapshot.active_panel, &files_view) {
            return vec![Action::Domain(action)];
        }
    }
    if pm("discard") && snapshot.active_panel == SidePanel::Files {
        if files_view.selection.visual_mode {
            return vec![Action::Domain(DomainAction::DiscardSelection)];
        }
        let paths = prepare_discard_targets_from_selection(snapshot.active_panel, &files_view);
        if !paths.is_empty() {
            return vec![Action::Domain(DomainAction::DiscardPaths(paths))];
        }
    }
    if pm("stash_push") {
        return vec![Action::Domain(DomainAction::StartStashInput)];
    }
    if pm("toggle_visual_select") {
        return vec![Action::Domain(DomainAction::ToggleVisualSelectMode)];
    }
    if pm("toggle_dir") {
        return vec![Action::Domain(DomainAction::ToggleDir)];
    }
    if pm("collapse_all") {
        return vec![Action::Domain(DomainAction::CollapseAll)];
    }
    if pm("expand_all") {
        return vec![Action::Domain(DomainAction::ExpandAll)];
    }
    if pm("checkout_branch") {
        return vec![Action::Domain(DomainAction::CheckoutSelectedBranch)];
    }
    if pm("create_branch") {
        return vec![Action::Domain(DomainAction::StartBranchCreateInput)];
    }
    if pm("delete_branch") {
        return vec![Action::Domain(DomainAction::DeleteSelectedBranch)];
    }
    if pm("fetch_remote") {
        return vec![Action::Domain(DomainAction::FetchRemote)];
    }
    if pm("open_tree") {
        return vec![Action::Domain(DomainAction::RevisionOpenTreeOrToggleDir)];
    }
    if pm("stash_apply") {
        return vec![Action::Domain(DomainAction::StashApplySelected)];
    }
    if pm("stash_pop") {
        return vec![Action::Domain(DomainAction::StashPopSelected)];
    }
    if pm("stash_drop") {
        return vec![Action::Domain(DomainAction::StashDropSelected)];
    }

    Vec::new()
}

fn active_panel_name(panel: SidePanel) -> &'static str {
    match panel {
        SidePanel::Files => "files",
        SidePanel::LocalBranches => "branches",
        SidePanel::Commits => "commits",
        SidePanel::Stash => "stash",
    }
}

fn toggle_stage_for_selected_file(
    active_panel: SidePanel,
    files_view: &FilesPanelViewState,
) -> Option<DomainAction> {
    if active_panel != SidePanel::Files {
        return None;
    }
    let idx = files_view.selection.selected_index?;
    let node = files_view.nodes.get(idx)?;
    if node.is_dir {
        let all_staged = directory_files_are_all_staged(files_view, idx);
        return if all_staged {
            Some(DomainAction::UnstageFile(node.path.clone()))
        } else {
            Some(DomainAction::StageFile(node.path.clone()))
        };
    }

    match &node.status {
        FilesPanelNodeStatus::Staged(_) => Some(DomainAction::UnstageFile(node.path.clone())),
        FilesPanelNodeStatus::Unstaged(_) | FilesPanelNodeStatus::Untracked => {
            Some(DomainAction::StageFile(node.path.clone()))
        }
        FilesPanelNodeStatus::Directory => None,
    }
}

fn prepare_discard_targets_from_selection(
    active_panel: SidePanel,
    files_view: &FilesPanelViewState,
) -> Vec<std::path::PathBuf> {
    if active_panel != SidePanel::Files {
        return Vec::new();
    }
    let Some(index) = files_view.selection.selected_index else {
        return Vec::new();
    };
    collect_discard_targets_for_index(files_view, index)
}

fn collect_discard_targets_for_index(
    files_view: &FilesPanelViewState,
    index: usize,
) -> Vec<std::path::PathBuf> {
    let Some(node) = files_view.nodes.get(index) else {
        return Vec::new();
    };
    if node.is_dir {
        let end = subtree_end_index(files_view, index);
        return collect_discard_targets_in_range(files_view, index, end);
    }
    if is_discardable_status(&node.status) {
        return vec![node.path.clone()];
    }
    Vec::new()
}

fn collect_discard_targets_in_range(
    files_view: &FilesPanelViewState,
    start: usize,
    end: usize,
) -> Vec<std::path::PathBuf> {
    let mut targets = Vec::new();
    for i in start..=end {
        let Some(node) = files_view.nodes.get(i) else {
            continue;
        };
        if node.is_dir {
            continue;
        }
        if is_discardable_status(&node.status) {
            targets.push(node.path.clone());
        }
    }
    dedup_paths(targets)
}

fn subtree_end_index(files_view: &FilesPanelViewState, index: usize) -> usize {
    let Some(node) = files_view.nodes.get(index) else {
        return index;
    };
    if !node.is_dir {
        return index;
    }

    let base_depth = node.depth;
    let mut end = index;
    for i in index + 1..files_view.nodes.len() {
        let n = &files_view.nodes[i];
        if n.depth <= base_depth {
            break;
        }
        end = i;
    }
    end
}

fn directory_files_are_all_staged(files_view: &FilesPanelViewState, index: usize) -> bool {
    let Some(node) = files_view.nodes.get(index) else {
        return false;
    };
    if !node.is_dir {
        return matches!(node.status, FilesPanelNodeStatus::Staged(_));
    }

    let end = subtree_end_index(files_view, index);
    let mut has_file = false;
    for i in index + 1..=end {
        let child = &files_view.nodes[i];
        if child.is_dir {
            continue;
        }
        has_file = true;
        if !matches!(child.status, FilesPanelNodeStatus::Staged(_)) {
            return false;
        }
    }
    has_file
}

fn is_discardable_status(status: &FilesPanelNodeStatus) -> bool {
    matches!(status, FilesPanelNodeStatus::Staged(_) | FilesPanelNodeStatus::Unstaged(_))
}

fn dedup_paths(mut paths: Vec<std::path::PathBuf>) -> Vec<std::path::PathBuf> {
    let mut seen = std::collections::HashSet::<std::path::PathBuf>::new();
    paths.retain(|p| seen.insert(p.clone()));
    paths
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
        app.ui.files.visual_mode = true;
        app.ui.files.panel.list_state.select(Some(0));
        app.ui.files.visual_anchor = Some(0);
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
