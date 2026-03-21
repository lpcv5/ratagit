use super::{diff_loader, revision_tree};
use crate::app::{App, SidePanel};
use crate::ui::widgets::file_tree::FileTreeNode;

impl App {
    pub fn selected_tree_node(&self) -> Option<&FileTreeNode> {
        if self.active_panel != SidePanel::Files {
            return None;
        }
        let idx = self.files.panel.list_state.selected()?;
        self.files.tree_nodes.get(idx)
    }

    pub fn selected_branch_name(&self) -> Option<String> {
        if self.active_panel != SidePanel::LocalBranches {
            return None;
        }
        let idx = self.branches.panel.list_state.selected()?;
        self.branches.items.get(idx).map(|b| b.name.clone())
    }

    pub fn selected_commit_oid(&self) -> Option<String> {
        if self.active_panel != SidePanel::Commits {
            return None;
        }
        if self.commits.tree_mode.active {
            return self.commits.tree_mode.selected_source.clone();
        }
        let idx = self.commits.panel.list_state.selected()?;
        self.commits.items.get(idx).map(|c| c.oid.clone())
    }

    pub fn selected_stash_index(&self) -> Option<usize> {
        if self.active_panel != SidePanel::Stash {
            return None;
        }
        if self.stash.tree_mode.active {
            return self.stash.tree_mode.selected_source;
        }
        let idx = self.stash.panel.list_state.selected()?;
        self.stash.items.get(idx).map(|s| s.index)
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
                let path = if self.commits.tree_mode.active {
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
                let path = if self.stash.tree_mode.active {
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
        if self.active_panel != SidePanel::Commits || !self.commits.tree_mode.active {
            return None;
        }
        revision_tree::selected_tree_node(&self.commits.panel.list_state, &self.commits.tree_mode.nodes)
    }

    pub(super) fn selected_stash_tree_node(&self) -> Option<&FileTreeNode> {
        if self.active_panel != SidePanel::Stash || !self.stash.tree_mode.active {
            return None;
        }
        revision_tree::selected_tree_node(&self.stash.panel.list_state, &self.stash.tree_mode.nodes)
    }
}

