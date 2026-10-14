use super::files::FilesAction;
#[cfg(test)]
use crate::app::{App, SidePanel};
use crate::app::{AppState, ViewContext};

impl AppState {
    pub fn list_down(&mut self) {
        if self.view_context() == ViewContext::Files {
            self.files.update(FilesAction::Next, true);
            return;
        }
        let count = self.active_panel_count();
        if count == 0 {
            return;
        }
        let next = self
            .selection()
            .selected_index()
            .map(|i| (i + 1).min(count - 1))
            .unwrap_or(0);
        self.select_active_panel_index(next);
    }

    pub fn list_up(&mut self) {
        if self.view_context() == ViewContext::Files {
            self.files.update(FilesAction::Previous, true);
            return;
        }
        let count = self.active_panel_count();
        if count == 0 {
            return;
        }
        let prev = self
            .selection()
            .selected_index()
            .map(|i| i.saturating_sub(1))
            .unwrap_or(0);
        self.select_active_panel_index(prev);
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
    use crate::file_tree::{FileTreeNode, FileTreeNodeStatus};
    use crate::flux::stores::test_support::MockRepo;
    use crate::git::FileStatus;
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
        app.files.test_view_mut().tree_nodes = vec![make_node("a.txt"), make_node("b.txt")];
        app.files.test_view_mut().panel.list_state.select(Some(0));
        app.list_down();
        assert_eq!(app.files.view().panel.list_state.selected(), Some(1));
    }

    #[test]
    fn test_list_down_stops_at_end() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        app.files.test_view_mut().tree_nodes = vec![make_node("a.txt")];
        app.files.test_view_mut().panel.list_state.select(Some(0));
        app.list_down();
        assert_eq!(app.files.view().panel.list_state.selected(), Some(0));
    }

    #[test]
    fn test_list_up_decrements_selection() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        app.files.test_view_mut().tree_nodes = vec![make_node("a.txt"), make_node("b.txt")];
        app.files.test_view_mut().panel.list_state.select(Some(1));
        app.list_up();
        assert_eq!(app.files.view().panel.list_state.selected(), Some(0));
    }

    #[test]
    fn test_list_up_stops_at_zero() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        app.files.test_view_mut().tree_nodes = vec![make_node("a.txt")];
        app.files.test_view_mut().panel.list_state.select(Some(0));
        app.list_up();
        assert_eq!(app.files.view().panel.list_state.selected(), Some(0));
    }

    #[test]
    fn test_active_panel_count_files() {
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        app.files.test_view_mut().tree_nodes = vec![make_node("a.txt"), make_node("b.txt")];
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
