use crate::app::{App, SidePanel};
use std::path::PathBuf;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::test_dispatch::dispatch_test_action;
    use crate::file_tree::{FileTreeNode, FileTreeNodeStatus};
    use crate::flux::action::DomainAction;
    use crate::flux::stores::test_support::MockRepo;
    use crate::git::FileStatus;
    use pretty_assertions::assert_eq;
    use std::collections::HashSet;

    fn mock_app() -> App {
        App::from_repo(Box::new(MockRepo)).expect("app")
    }

    fn make_node(
        path: &str,
        status: FileTreeNodeStatus,
        is_dir: bool,
        depth: usize,
    ) -> FileTreeNode {
        FileTreeNode {
            path: path.into(),
            status,
            depth,
            is_dir,
            is_expanded: !is_dir,
        }
    }

    #[test]
    fn test_visual_selected_indices_empty_when_not_in_files_panel() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::LocalBranches;
        app.files.test_view_mut().visual_mode = true;
        assert!(app.visual_selected_indices().is_empty());
    }

    #[test]
    fn test_visual_selected_indices_empty_when_visual_mode_off() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        app.files.test_view_mut().visual_mode = false;
        assert!(app.visual_selected_indices().is_empty());
    }

    #[test]
    fn visual_selected_indices_same_anchor_and_cursor_returns_single_index() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        app.files.test_view_mut().visual_mode = true;
        app.files.test_view_mut().panel.select(Some(2));
        app.files.test_view_mut().visual_anchor = Some(2);
        let selected = app.visual_selected_indices();
        assert_eq!(selected, HashSet::from([2]));
    }

    #[test]
    fn visual_selected_indices_forward_range_returns_inclusive_index_set() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        app.files.test_view_mut().visual_mode = true;
        app.files.test_view_mut().panel.select(Some(4));
        app.files.test_view_mut().visual_anchor = Some(2);
        let selected = app.visual_selected_indices();
        assert_eq!(selected, HashSet::from([2, 3, 4]));
    }

    #[test]
    fn visual_selected_indices_reversed_range_returns_inclusive_index_set() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        app.files.test_view_mut().visual_mode = true;
        app.files.test_view_mut().panel.select(Some(1));
        app.files.test_view_mut().visual_anchor = Some(3);
        let selected = app.visual_selected_indices();
        assert_eq!(selected, HashSet::from([1, 2, 3]));
    }

    #[test]
    fn test_prepare_discard_targets_empty_in_non_files_panel() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::LocalBranches;
        assert!(app.prepare_discard_targets_from_selection().is_empty());
    }

    #[test]
    fn test_prepare_discard_targets_for_unstaged_file() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        app.files.test_view_mut().tree_nodes = vec![make_node(
            "foo.txt",
            FileTreeNodeStatus::Unstaged(FileStatus::Modified),
            false,
            0,
        )]
        .into();
        app.files.test_view_mut().panel.select(Some(0));
        let targets = app.prepare_discard_targets_from_selection();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0], PathBuf::from("foo.txt"));
    }

    #[test]
    fn test_prepare_stash_targets_empty_in_non_files_panel() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Commits;
        assert!(app.prepare_stash_targets_from_selection().is_empty());
    }

    #[test]
    fn test_toggle_visual_mode_on() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        assert!(!app.files.view().visual_mode);
        dispatch_test_action(&mut app, DomainAction::ToggleVisualSelectMode);
        assert!(app.files.view().visual_mode);
    }

    #[test]
    fn test_toggle_visual_mode_off() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        app.files.test_view_mut().visual_mode = true;
        dispatch_test_action(&mut app, DomainAction::ToggleVisualSelectMode);
        assert!(!app.files.view().visual_mode);
    }
}

#[cfg(test)]
mod more_tests {
    use super::*;
    use crate::file_tree::{FileTreeNode, FileTreeNodeStatus};
    use crate::flux::stores::test_support::MockRepo;
    use crate::git::FileStatus;
    use pretty_assertions::assert_eq;

    fn mock_app() -> App {
        App::from_repo(Box::new(MockRepo)).expect("app")
    }

    fn file_node(path: &str, staged: bool) -> FileTreeNode {
        FileTreeNode {
            path: path.into(),
            status: if staged {
                FileTreeNodeStatus::Staged(FileStatus::Modified)
            } else {
                FileTreeNodeStatus::Unstaged(FileStatus::Modified)
            },
            depth: 0,
            is_dir: false,
            is_expanded: false,
        }
    }

    #[test]
    fn test_prepare_discard_targets_for_staged_file() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        app.files.test_view_mut().tree_nodes = vec![file_node("foo.txt", true)].into();
        app.files.test_view_mut().panel.select(Some(0));
        let targets = app.prepare_discard_targets_from_selection();
        assert_eq!(targets.len(), 1);
    }

    #[test]
    fn test_prepare_discard_targets_skips_untracked() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        app.files.test_view_mut().tree_nodes = vec![FileTreeNode {
            path: "new.txt".into(),
            status: FileTreeNodeStatus::Untracked,
            depth: 0,
            is_dir: false,
            is_expanded: false,
        }]
        .into();
        app.files.test_view_mut().panel.select(Some(0));
        let targets = app.prepare_discard_targets_from_selection();
        assert!(targets.is_empty());
    }

    #[test]
    fn test_prepare_stash_targets_single_file() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        app.files.test_view_mut().tree_nodes = vec![file_node("bar.txt", false)].into();
        app.files.test_view_mut().panel.select(Some(0));
        let targets = app.prepare_stash_targets_from_selection();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0], PathBuf::from("bar.txt"));
    }

    #[test]
    fn test_visual_selection_stage_toggle() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        app.files.test_view_mut().visual_mode = true;
        app.files.test_view_mut().tree_nodes =
            vec![file_node("a.txt", false), file_node("b.txt", false)].into();
        app.files.test_view_mut().panel.select(Some(0));
        app.files.test_view_mut().visual_anchor = Some(0);
        let result = app.toggle_stage_visual_selection();
        assert!(result.is_ok());
        let (staged, unstaged) = result.unwrap();
        assert_eq!(staged, 1);
        assert_eq!(unstaged, 0);
    }

    #[test]
    fn test_subtree_end_index_for_file_node_returns_self() {
        let mut app = mock_app();
        app.files.test_view_mut().tree_nodes = vec![file_node("a.txt", false)].into();
        // subtree_end_index is private but exercised via prepare_discard_targets
        app.ui.active_panel = SidePanel::Files;
        app.files.test_view_mut().panel.select(Some(0));
        let targets = app.prepare_discard_targets_from_selection();
        assert_eq!(targets.len(), 1);
    }

    #[test]
    fn test_prepare_discard_targets_visual_selection_multiple() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        app.files.test_view_mut().visual_mode = true;
        app.files.test_view_mut().tree_nodes =
            vec![file_node("a.txt", false), file_node("b.txt", true)].into();
        app.files.test_view_mut().panel.select(Some(1));
        app.files.test_view_mut().visual_anchor = Some(0);
        let targets = app.prepare_discard_targets_from_selection();
        assert_eq!(targets.len(), 2);
    }

    #[test]
    fn test_prepare_commit_from_visual_selection_empty_returns_zero() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        app.files.test_view_mut().visual_mode = false;
        let result = app.prepare_commit_from_visual_selection();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }
}
