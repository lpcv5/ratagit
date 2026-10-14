use super::states::PanelState;
#[cfg(test)]
use crate::app::{App, SidePanel};
use crate::app::{AppState, ViewContext};

impl AppState {
    pub fn active_panel_state_mut(&mut self) -> &mut PanelState {
        match self.view_context() {
            ViewContext::Files => &mut self.ui.files.panel,
            ViewContext::Branches => &mut self.ui.branches.panel,
            ViewContext::BranchCommits { .. } => &mut self.ui.branches.commits_subview.panel,
            ViewContext::Commits | ViewContext::CommitFiles { .. } => &mut self.ui.commits.panel,
            ViewContext::Stashes | ViewContext::StashFiles { .. } => &mut self.ui.stash.panel,
        }
    }

    pub fn list_down(&mut self) {
        let count = self.active_panel_count();
        if count == 0 {
            return;
        }
        let state = self.active_panel_state_mut();
        let next = state
            .list_state
            .selected()
            .map(|i| (i + 1).min(count - 1))
            .unwrap_or(0);
        state.list_state.select(Some(next));
    }

    pub fn list_up(&mut self) {
        let count = self.active_panel_count();
        if count == 0 {
            return;
        }
        let state = self.active_panel_state_mut();
        let prev = state
            .list_state
            .selected()
            .map(|i| i.saturating_sub(1))
            .unwrap_or(0);
        state.list_state.select(Some(prev));
    }

    pub(super) fn active_panel_count(&self) -> usize {
        self.selection().visible_count()
    }

    #[cfg(test)]
    pub(super) fn active_panel_name(&self) -> &'static str {
        self.view_context().panel().keymap_name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flux::stores::test_support::MockRepo;
    use crate::git::FileStatus;
    use crate::ui::widgets::file_tree::{FileTreeNode, FileTreeNodeStatus};
    use pretty_assertions::assert_eq;

    fn mock_app() -> App {
        App::from_repo(Box::new(MockRepo)).expect("app")
    }

    fn make_node(path: &str) -> FileTreeNode {
        FileTreeNode {
            path: path.into(),
            depth: 0,
            is_dir: false,
            is_expanded: false,
            status: FileTreeNodeStatus::Unstaged(FileStatus::Modified),
        }
    }

    #[test]
    fn test_list_down_increments_selection() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        app.ui.files.tree_nodes = vec![make_node("a.txt"), make_node("b.txt")];
        app.ui.files.panel.list_state.select(Some(0));
        app.list_down();
        assert_eq!(app.ui.files.panel.list_state.selected(), Some(1));
    }

    #[test]
    fn test_list_down_stops_at_end() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        app.ui.files.tree_nodes = vec![make_node("a.txt")];
        app.ui.files.panel.list_state.select(Some(0));
        app.list_down();
        assert_eq!(app.ui.files.panel.list_state.selected(), Some(0));
    }

    #[test]
    fn test_list_up_decrements_selection() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        app.ui.files.tree_nodes = vec![make_node("a.txt"), make_node("b.txt")];
        app.ui.files.panel.list_state.select(Some(1));
        app.list_up();
        assert_eq!(app.ui.files.panel.list_state.selected(), Some(0));
    }

    #[test]
    fn test_list_up_stops_at_zero() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        app.ui.files.tree_nodes = vec![make_node("a.txt")];
        app.ui.files.panel.list_state.select(Some(0));
        app.list_up();
        assert_eq!(app.ui.files.panel.list_state.selected(), Some(0));
    }

    #[test]
    fn test_active_panel_count_files() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        app.ui.files.tree_nodes = vec![make_node("a.txt"), make_node("b.txt")];
        assert_eq!(app.active_panel_count(), 2);
    }

    #[test]
    fn test_active_panel_count_branches() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::LocalBranches;
        app.ui.branches.items = vec![
            crate::git::BranchInfo {
                name: "main".to_string(),
                is_current: true,
            },
            crate::git::BranchInfo {
                name: "dev".to_string(),
                is_current: false,
            },
        ];
        assert_eq!(app.active_panel_count(), 2);
    }

    #[test]
    fn test_active_panel_name() {
        let app = mock_app();
        assert_eq!(app.active_panel_name(), "files");
    }
}
