use super::{diff_loader, revision_tree};
use crate::app::{App, SidePanel};
use crate::ui::widgets::file_tree::FileTreeNode;

impl App {
    pub fn selected_tree_node(&self) -> Option<&FileTreeNode> {
        if self.active_panel != SidePanel::Files {
            return None;
        }
        let idx = self.files_panel.list_state.selected()?;
        self.file_tree_nodes.get(idx)
    }

    pub fn selected_branch_name(&self) -> Option<String> {
        if self.active_panel != SidePanel::LocalBranches {
            return None;
        }
        let idx = self.branches_panel.list_state.selected()?;
        self.branches.get(idx).map(|b| b.name.clone())
    }

    pub fn selected_commit_oid(&self) -> Option<String> {
        if self.active_panel != SidePanel::Commits {
            return None;
        }
        if self.commit_tree_mode {
            return self.commit_tree_commit_oid.clone();
        }
        let idx = self.commits_panel.list_state.selected()?;
        self.commits.get(idx).map(|c| c.oid.clone())
    }

    pub fn selected_stash_index(&self) -> Option<usize> {
        if self.active_panel != SidePanel::Stash {
            return None;
        }
        if self.stash_tree_mode {
            return self.stash_tree_stash_index;
        }
        let idx = self.stash_panel.list_state.selected()?;
        self.stashes.get(idx).map(|s| s.index)
    }

    pub(super) fn selected_diff_target(&self) -> diff_loader::DiffTarget {
        match self.active_panel {
            SidePanel::Files => {
                let Some(node) = self.selected_tree_node() else {
                    return diff_loader::DiffTarget::None;
                };
                if node.is_dir {
                    diff_loader::DiffTarget::Directory {
                        path: node.path.clone(),
                    }
                } else {
                    diff_loader::DiffTarget::File {
                        path: node.path.clone(),
                        status: node.status.clone(),
                    }
                }
            }
            SidePanel::Commits => {
                let Some(oid) = self.selected_commit_oid() else {
                    return diff_loader::DiffTarget::None;
                };
                let path = if self.commit_tree_mode {
                    self.selected_commit_tree_node().map(|n| n.path.clone())
                } else {
                    None
                };
                diff_loader::DiffTarget::Commit { oid, path }
            }
            SidePanel::Stash => {
                let Some(index) = self.selected_stash_index() else {
                    return diff_loader::DiffTarget::None;
                };
                let path = if self.stash_tree_mode {
                    self.selected_stash_tree_node().map(|n| n.path.clone())
                } else {
                    None
                };
                diff_loader::DiffTarget::Stash { index, path }
            }
            SidePanel::LocalBranches => diff_loader::DiffTarget::None,
        }
    }

    pub(super) fn selected_commit_tree_node(&self) -> Option<&FileTreeNode> {
        if self.active_panel != SidePanel::Commits || !self.commit_tree_mode {
            return None;
        }
        revision_tree::selected_tree_node(&self.commits_panel.list_state, &self.commit_tree_nodes)
    }

    pub(super) fn selected_stash_tree_node(&self) -> Option<&FileTreeNode> {
        if self.active_panel != SidePanel::Stash || !self.stash_tree_mode {
            return None;
        }
        revision_tree::selected_tree_node(&self.stash_panel.list_state, &self.stash_tree_nodes)
    }
}
