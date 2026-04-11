use crate::app::{App, SidePanel};
use crate::flux::commits_backend::{CommitsBackend, CommitsPanelDiffRequest};
use crate::flux::git_backend::stash::StashBackend;
use crate::ui::widgets::file_tree::FileTreeNode;

impl App {
    pub fn selected_tree_node(&self) -> Option<&FileTreeNode> {
        if self.ui.active_panel != SidePanel::Files {
            return None;
        }
        let idx = self.ui.files.panel.list_state.selected()?;
        self.ui.files.tree_nodes.get(idx)
    }

    pub fn selected_branch_name(&self) -> Option<String> {
        if self.ui.active_panel != SidePanel::LocalBranches {
            return None;
        }
        if self.ui.branches.commits_subview_active {
            return self.ui.branches.commits_subview_source.clone();
        }
        let idx = self.ui.branches.panel.list_state.selected()?;
        self.ui.branches.items.get(idx).map(|b| b.name.clone())
    }

    pub fn selected_commit_oid(&self) -> Option<String> {
        if self.ui.active_panel != SidePanel::Commits {
            return None;
        }
        match CommitsBackend::selected_diff_request(&self.current_commits_view_state()) {
            CommitsPanelDiffRequest::Commit { oid, .. } => Some(oid),
            CommitsPanelDiffRequest::None => None,
        }
    }

    pub fn selected_stash_index(&self) -> Option<usize> {
        if self.ui.active_panel != SidePanel::Stash {
            return None;
        }
        StashBackend::selected_stash_index(&self.current_stash_view_state())
    }
}
