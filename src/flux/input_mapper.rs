use crate::app::{InputMode, SidePanel};
use crate::config::keymap::key_to_string;
use crate::flux::action::{Action, DomainAction};
use crate::flux::snapshot::AppStateSnapshot;
use crate::ui::widgets::file_tree::FileTreeNodeStatus;

pub fn map_key_to_actions(
    key: crossterm::event::KeyEvent,
    snapshot: &AppStateSnapshot<'_>,
) -> Vec<Action> {
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
        && snapshot.files.visual_mode
    {
        return vec![Action::Domain(DomainAction::ToggleVisualSelectMode)];
    }
    if key.code == crossterm::event::KeyCode::Esc
        && ((snapshot.active_panel == SidePanel::Stash && snapshot.stash.tree_mode.active)
            || (snapshot.active_panel == SidePanel::Commits && snapshot.commits.tree_mode.active))
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
    if gm("refresh") {
        return vec![Action::Domain(DomainAction::RefreshStatus)];
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
        if snapshot.active_panel == SidePanel::Files && snapshot.files.visual_mode {
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
        if snapshot.active_panel == SidePanel::Files && snapshot.files.visual_mode {
            return vec![Action::Domain(DomainAction::ToggleStageSelection)];
        }
        if let Some(action) = toggle_stage_for_selected_file(snapshot) {
            return vec![Action::Domain(action)];
        }
    }
    if pm("discard") && snapshot.active_panel == SidePanel::Files {
        if snapshot.files.visual_mode {
            return vec![Action::Domain(DomainAction::DiscardSelection)];
        }
        let paths = prepare_discard_targets_from_selection(snapshot);
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

fn toggle_stage_for_selected_file(snapshot: &AppStateSnapshot<'_>) -> Option<DomainAction> {
    if snapshot.active_panel != SidePanel::Files {
        return None;
    }
    let idx = snapshot.files.panel.list_state.selected()?;
    let node = snapshot.files.tree_nodes.get(idx)?;
    if node.is_dir {
        let all_staged = directory_files_are_all_staged(snapshot, idx);
        return if all_staged {
            Some(DomainAction::UnstageFile(node.path.clone()))
        } else {
            Some(DomainAction::StageFile(node.path.clone()))
        };
    }

    match &node.status {
        FileTreeNodeStatus::Staged(_) => Some(DomainAction::UnstageFile(node.path.clone())),
        FileTreeNodeStatus::Unstaged(_) | FileTreeNodeStatus::Untracked => {
            Some(DomainAction::StageFile(node.path.clone()))
        }
        FileTreeNodeStatus::Directory => None,
    }
}

fn prepare_discard_targets_from_selection(
    snapshot: &AppStateSnapshot<'_>,
) -> Vec<std::path::PathBuf> {
    if snapshot.active_panel != SidePanel::Files {
        return Vec::new();
    }
    let Some(index) = snapshot.files.panel.list_state.selected() else {
        return Vec::new();
    };
    collect_discard_targets_for_index(snapshot, index)
}

fn collect_discard_targets_for_index(
    snapshot: &AppStateSnapshot<'_>,
    index: usize,
) -> Vec<std::path::PathBuf> {
    let Some(node) = snapshot.files.tree_nodes.get(index) else {
        return Vec::new();
    };
    if node.is_dir {
        let end = subtree_end_index(snapshot, index);
        return collect_discard_targets_in_range(snapshot, index, end);
    }
    if is_discardable_status(&node.status) {
        return vec![node.path.clone()];
    }
    Vec::new()
}

fn collect_discard_targets_in_range(
    snapshot: &AppStateSnapshot<'_>,
    start: usize,
    end: usize,
) -> Vec<std::path::PathBuf> {
    let mut targets = Vec::new();
    for i in start..=end {
        let Some(node) = snapshot.files.tree_nodes.get(i) else {
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

fn subtree_end_index(snapshot: &AppStateSnapshot<'_>, index: usize) -> usize {
    let Some(node) = snapshot.files.tree_nodes.get(index) else {
        return index;
    };
    if !node.is_dir {
        return index;
    }

    let base_depth = node.depth;
    let mut end = index;
    for i in index + 1..snapshot.files.tree_nodes.len() {
        let n = &snapshot.files.tree_nodes[i];
        if n.depth <= base_depth {
            break;
        }
        end = i;
    }
    end
}

fn directory_files_are_all_staged(snapshot: &AppStateSnapshot<'_>, index: usize) -> bool {
    let Some(node) = snapshot.files.tree_nodes.get(index) else {
        return false;
    };
    if !node.is_dir {
        return matches!(node.status, FileTreeNodeStatus::Staged(_));
    }

    let end = subtree_end_index(snapshot, index);
    let mut has_file = false;
    for i in index + 1..=end {
        let child = &snapshot.files.tree_nodes[i];
        if child.is_dir {
            continue;
        }
        has_file = true;
        if !matches!(child.status, FileTreeNodeStatus::Staged(_)) {
            return false;
        }
    }
    has_file
}

fn is_discardable_status(status: &FileTreeNodeStatus) -> bool {
    matches!(
        status,
        FileTreeNodeStatus::Staged(_) | FileTreeNodeStatus::Unstaged(_)
    )
}

fn dedup_paths(mut paths: Vec<std::path::PathBuf>) -> Vec<std::path::PathBuf> {
    let mut seen = std::collections::HashSet::<std::path::PathBuf>::new();
    paths.retain(|p| seen.insert(p.clone()));
    paths
}
