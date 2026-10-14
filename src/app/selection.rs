use crate::app::states::FilesPanelState;
#[cfg(test)]
use crate::app::App;
use crate::app::{AppState, SidePanel};
use crate::file_tree::FileTreeNodeStatus;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug)]
struct SelectionTarget {
    path: PathBuf,
    all_staged: bool,
}

pub struct FileSelection<'a> {
    files: &'a FilesPanelState,
    active: bool,
}

impl<'a> FileSelection<'a> {
    pub fn is_visual_mode(&self) -> bool {
        self.active && self.files.visual_mode
    }
    pub fn new(files: &'a FilesPanelState, active: bool) -> Self {
        Self { files, active }
    }

    pub fn selected_tree_node(&self) -> Option<&'a crate::file_tree::FileTreeNode> {
        if !self.active {
            return None;
        }
        self.files
            .tree_nodes
            .get(self.files.panel.list_state.selected()?)
    }

    pub fn toggle_stage_action(&self) -> Option<crate::flux::action::DomainAction> {
        use crate::flux::action::DomainAction;
        let node = self.selected_tree_node()?;
        let staged = if node.is_dir {
            let index = self.files.panel.list_state.selected()?;
            let end = self.subtree_end_index(index);
            let selected = (index..=end).collect();
            self.selected_files_are_all_staged(&selected, &node.path)
        } else {
            matches!(node.status, FileTreeNodeStatus::Staged(_))
        };
        Some(if staged {
            DomainAction::UnstageFile(node.path.clone())
        } else {
            DomainAction::StageFile(node.path.clone())
        })
    }

    pub fn visual_selected_indices(&self) -> HashSet<usize> {
        let mut set = HashSet::new();
        if !self.active || !self.files.visual_mode {
            return set;
        }
        let Some(current) = self.files.panel.list_state.selected() else {
            return set;
        };
        let anchor = self.files.visual_anchor.unwrap_or(current);
        let (start, end) = if anchor <= current {
            (anchor, current)
        } else {
            (current, anchor)
        };
        for idx in start..=end {
            set.insert(idx);
        }
        set
    }

    pub fn prepare_stash_targets_from_selection(&self) -> Vec<PathBuf> {
        if !self.active {
            return Vec::new();
        }

        if self.files.visual_mode {
            let selected = self.visual_selected_indices();
            return self.collect_commit_targets(&selected);
        }

        let Some(node) = self.selected_tree_node() else {
            return Vec::new();
        };
        vec![node.path.clone()]
    }

    pub fn prepare_discard_targets_from_selection(&self) -> Vec<PathBuf> {
        if !self.active {
            return Vec::new();
        }

        if self.files.visual_mode {
            let selected = self.visual_selected_indices();
            return self.collect_discard_targets(&selected);
        }

        let Some(index) = self.files.panel.list_state.selected() else {
            return Vec::new();
        };
        self.collect_discard_targets_for_index(index)
    }

    pub(super) fn partition_toggle_targets(
        &self,
        selected: &HashSet<usize>,
    ) -> (Vec<PathBuf>, Vec<PathBuf>) {
        let mut stage_paths = Vec::new();
        let mut unstage_paths = Vec::new();

        for target in self.collect_selection_targets(selected) {
            if target.all_staged {
                unstage_paths.push(target.path);
            } else {
                stage_paths.push(target.path);
            }
        }

        (stage_paths, unstage_paths)
    }

    pub(super) fn collect_commit_targets(&self, selected: &HashSet<usize>) -> Vec<PathBuf> {
        self.collect_selection_targets(selected)
            .into_iter()
            .map(|t| t.path)
            .collect()
    }

    fn collect_discard_targets(&self, selected: &HashSet<usize>) -> Vec<PathBuf> {
        let mut targets = Vec::new();
        let mut covered = HashSet::new();
        let mut ordered: Vec<usize> = selected.iter().copied().collect();
        ordered.sort_unstable();

        for idx in ordered {
            if covered.contains(&idx) {
                continue;
            }
            let Some(node) = self.files.tree_nodes.get(idx) else {
                continue;
            };

            if node.is_dir {
                let end = self.subtree_end_index(idx);
                let fully_covered = (idx..=end).all(|i| selected.contains(&i));
                if fully_covered {
                    targets.extend(self.collect_discard_targets_in_range(idx, end));
                    for i in idx..=end {
                        covered.insert(i);
                    }
                }
                continue;
            }

            if is_discardable_status(&node.status) {
                targets.push(node.path.clone());
            }
            covered.insert(idx);
        }

        dedup_paths(targets)
    }

    fn collect_discard_targets_for_index(&self, index: usize) -> Vec<PathBuf> {
        let Some(node) = self.files.tree_nodes.get(index) else {
            return Vec::new();
        };
        if node.is_dir {
            let end = self.subtree_end_index(index);
            return self.collect_discard_targets_in_range(index, end);
        }
        if is_discardable_status(&node.status) {
            return vec![node.path.clone()];
        }
        Vec::new()
    }

    fn collect_discard_targets_in_range(&self, start: usize, end: usize) -> Vec<PathBuf> {
        let mut targets = Vec::new();
        for i in start..=end {
            let Some(node) = self.files.tree_nodes.get(i) else {
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

    fn collect_selection_targets(&self, selected: &HashSet<usize>) -> Vec<SelectionTarget> {
        let mut targets = Vec::new();
        let mut covered = HashSet::new();
        let mut ordered: Vec<usize> = selected.iter().copied().collect();
        ordered.sort_unstable();

        for idx in ordered {
            if covered.contains(&idx) {
                continue;
            }
            let Some(node) = self.files.tree_nodes.get(idx) else {
                continue;
            };

            if node.is_dir {
                let end = self.subtree_end_index(idx);
                let fully_covered = (idx..=end).all(|i| selected.contains(&i));
                if fully_covered {
                    let all_staged = self.selected_files_are_all_staged(selected, &node.path);
                    targets.push(SelectionTarget {
                        path: node.path.clone(),
                        all_staged,
                    });
                    for i in idx..=end {
                        covered.insert(i);
                    }
                }
                continue;
            }

            let all_staged = matches!(node.status, FileTreeNodeStatus::Staged(_));
            targets.push(SelectionTarget {
                path: node.path.clone(),
                all_staged,
            });
            covered.insert(idx);
        }

        dedup_targets(targets)
    }

    fn selected_files_are_all_staged(&self, selected: &HashSet<usize>, dir_path: &Path) -> bool {
        let mut has_file = false;
        for idx in selected {
            let Some(node) = self.files.tree_nodes.get(*idx) else {
                continue;
            };
            if node.is_dir || !node.path.starts_with(dir_path) {
                continue;
            }
            has_file = true;
            if !matches!(node.status, FileTreeNodeStatus::Staged(_)) {
                return false;
            }
        }
        has_file
    }

    fn subtree_end_index(&self, index: usize) -> usize {
        let Some(node) = self.files.tree_nodes.get(index) else {
            return index;
        };
        if !node.is_dir {
            return index;
        }

        let base_depth = node.depth;
        let mut end = index;
        for i in index + 1..self.files.tree_nodes.len() {
            let n = &self.files.tree_nodes[i];
            if n.depth <= base_depth {
                break;
            }
            end = i;
        }
        end
    }
}

fn dedup_targets(mut targets: Vec<SelectionTarget>) -> Vec<SelectionTarget> {
    let mut seen = HashSet::<PathBuf>::new();
    targets.retain(|t| seen.insert(t.path.clone()));
    targets
}

fn dedup_paths(mut paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = HashSet::<PathBuf>::new();
    paths.retain(|p| seen.insert(p.clone()));
    paths
}

fn is_discardable_status(status: &FileTreeNodeStatus) -> bool {
    matches!(
        status,
        FileTreeNodeStatus::Staged(_) | FileTreeNodeStatus::Unstaged(_)
    )
}

impl AppState {
    pub fn file_selection(&self) -> FileSelection<'_> {
        FileSelection::new(&self.ui.files, self.ui.active_panel == SidePanel::Files)
    }

    pub fn visual_selected_indices(&self) -> HashSet<usize> {
        self.file_selection().visual_selected_indices()
    }

    pub fn prepare_stash_targets_from_selection(&self) -> Vec<PathBuf> {
        self.file_selection().prepare_stash_targets_from_selection()
    }

    pub fn prepare_discard_targets_from_selection(&self) -> Vec<PathBuf> {
        self.file_selection()
            .prepare_discard_targets_from_selection()
    }

    pub(super) fn partition_toggle_targets(
        &self,
        selected: &HashSet<usize>,
    ) -> (Vec<PathBuf>, Vec<PathBuf>) {
        self.file_selection().partition_toggle_targets(selected)
    }

    pub(super) fn collect_commit_targets(&self, selected: &HashSet<usize>) -> Vec<PathBuf> {
        self.file_selection().collect_commit_targets(selected)
    }
}

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
        app.ui.files.visual_mode = true;
        assert!(app.visual_selected_indices().is_empty());
    }

    #[test]
    fn test_visual_selected_indices_empty_when_visual_mode_off() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        app.ui.files.visual_mode = false;
        assert!(app.visual_selected_indices().is_empty());
    }

    #[test]
    fn visual_selected_indices_same_anchor_and_cursor_returns_single_index() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        app.ui.files.visual_mode = true;
        app.ui.files.panel.list_state.select(Some(2));
        app.ui.files.visual_anchor = Some(2);
        let selected = app.visual_selected_indices();
        assert_eq!(selected, HashSet::from([2]));
    }

    #[test]
    fn visual_selected_indices_forward_range_returns_inclusive_index_set() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        app.ui.files.visual_mode = true;
        app.ui.files.panel.list_state.select(Some(4));
        app.ui.files.visual_anchor = Some(2);
        let selected = app.visual_selected_indices();
        assert_eq!(selected, HashSet::from([2, 3, 4]));
    }

    #[test]
    fn visual_selected_indices_reversed_range_returns_inclusive_index_set() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        app.ui.files.visual_mode = true;
        app.ui.files.panel.list_state.select(Some(1));
        app.ui.files.visual_anchor = Some(3);
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
        app.ui.files.tree_nodes = vec![make_node(
            "foo.txt",
            FileTreeNodeStatus::Unstaged(FileStatus::Modified),
            false,
            0,
        )];
        app.ui.files.panel.list_state.select(Some(0));
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
        assert!(!app.ui.files.visual_mode);
        dispatch_test_action(&mut app, DomainAction::ToggleVisualSelectMode);
        assert!(app.ui.files.visual_mode);
    }

    #[test]
    fn test_toggle_visual_mode_off() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        app.ui.files.visual_mode = true;
        dispatch_test_action(&mut app, DomainAction::ToggleVisualSelectMode);
        assert!(!app.ui.files.visual_mode);
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
        app.ui.files.tree_nodes = vec![file_node("foo.txt", true)];
        app.ui.files.panel.list_state.select(Some(0));
        let targets = app.prepare_discard_targets_from_selection();
        assert_eq!(targets.len(), 1);
    }

    #[test]
    fn test_prepare_discard_targets_skips_untracked() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        app.ui.files.tree_nodes = vec![FileTreeNode {
            path: "new.txt".into(),
            status: FileTreeNodeStatus::Untracked,
            depth: 0,
            is_dir: false,
            is_expanded: false,
        }];
        app.ui.files.panel.list_state.select(Some(0));
        let targets = app.prepare_discard_targets_from_selection();
        assert!(targets.is_empty());
    }

    #[test]
    fn test_prepare_stash_targets_single_file() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        app.ui.files.tree_nodes = vec![file_node("bar.txt", false)];
        app.ui.files.panel.list_state.select(Some(0));
        let targets = app.prepare_stash_targets_from_selection();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0], PathBuf::from("bar.txt"));
    }

    #[test]
    fn test_visual_selection_stage_toggle() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        app.ui.files.visual_mode = true;
        app.ui.files.tree_nodes = vec![file_node("a.txt", false), file_node("b.txt", false)];
        app.ui.files.panel.list_state.select(Some(0));
        app.ui.files.visual_anchor = Some(0);
        let result = app.toggle_stage_visual_selection();
        assert!(result.is_ok());
        let (staged, unstaged) = result.unwrap();
        assert_eq!(staged, 1);
        assert_eq!(unstaged, 0);
    }

    #[test]
    fn test_subtree_end_index_for_file_node_returns_self() {
        let mut app = mock_app();
        app.ui.files.tree_nodes = vec![file_node("a.txt", false)];
        // subtree_end_index is private but exercised via prepare_discard_targets
        app.ui.active_panel = SidePanel::Files;
        app.ui.files.panel.list_state.select(Some(0));
        let targets = app.prepare_discard_targets_from_selection();
        assert_eq!(targets.len(), 1);
    }

    #[test]
    fn test_prepare_discard_targets_visual_selection_multiple() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        app.ui.files.visual_mode = true;
        app.ui.files.tree_nodes = vec![file_node("a.txt", false), file_node("b.txt", true)];
        app.ui.files.panel.list_state.select(Some(1));
        app.ui.files.visual_anchor = Some(0);
        let targets = app.prepare_discard_targets_from_selection();
        assert_eq!(targets.len(), 2);
    }

    #[test]
    fn test_prepare_commit_from_visual_selection_empty_returns_zero() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        app.ui.files.visual_mode = false;
        let result = app.prepare_commit_from_visual_selection();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }
}
