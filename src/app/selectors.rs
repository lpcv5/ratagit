use super::{diff_loader, revision_tree};
use super::states::{PanelState, TreeModeState};
use crate::app::{App, SidePanel};
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

    pub fn selected_branch_subview_commit_oid(&self) -> Option<String> {
        if self.ui.active_panel != SidePanel::LocalBranches || !self.ui.branches.commits_subview_active {
            return None;
        }
        let idx = self.ui.branches.commits_subview.panel.list_state.selected()?;
        self.ui.branches
            .commits_subview
            .items
            .get(idx)
            .map(|c| c.oid.clone())
    }

    pub fn selected_commit_oid(&self) -> Option<String> {
        if self.ui.active_panel != SidePanel::Commits {
            return None;
        }
        if self.ui.commits.tree_mode.active {
            return self.ui.commits.tree_mode.selected_source.clone();
        }
        let idx = self.ui.commits.panel.list_state.selected()?;
        self.ui.commits.items.get(idx).map(|c| c.oid.clone())
    }

    pub fn selected_stash_index(&self) -> Option<usize> {
        if self.ui.active_panel != SidePanel::Stash {
            return None;
        }
        if self.ui.stash.tree_mode.active {
            return self.ui.stash.tree_mode.selected_source;
        }
        let idx = self.ui.stash.panel.list_state.selected()?;
        self.ui.stash.items.get(idx).map(|s| s.index)
    }

    pub(super) fn selected_diff_target(&self) -> diff_loader::DiffTarget {
        match self.ui.active_panel {
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
                let path = if self.ui.commits.tree_mode.active {
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
                let path = if self.ui.stash.tree_mode.active {
                    self.selected_stash_tree_node().map(|n| n.path.clone())
                } else {
                    None
                };
                diff_loader::DiffTarget::Stash { index, path }
            }
            SidePanel::LocalBranches => {
                if self.ui.branches.commits_subview_active {
                    let Some(oid) = self.selected_branch_subview_commit_oid() else {
                        return diff_loader::DiffTarget::None;
                    };
                    return diff_loader::DiffTarget::Commit { oid, path: None };
                }
                let Some(name) = self.selected_branch_name() else {
                    return diff_loader::DiffTarget::None;
                };
                diff_loader::DiffTarget::Branch { name }
            }
        }
    }

    pub(super) fn selected_commit_tree_node(&self) -> Option<&FileTreeNode> {
        self.selected_revision_tree_node(
            SidePanel::Commits,
            &self.ui.commits.panel,
            &self.ui.commits.tree_mode,
        )
    }

    pub(super) fn selected_stash_tree_node(&self) -> Option<&FileTreeNode> {
        self.selected_revision_tree_node(SidePanel::Stash, &self.ui.stash.panel, &self.ui.stash.tree_mode)
    }

    fn selected_revision_tree_node<'a, T>(
        &'a self,
        expected: SidePanel,
        panel: &PanelState,
        tree: &'a TreeModeState<T>,
    ) -> Option<&'a FileTreeNode> {
        if self.ui.active_panel != expected || !tree.active {
            return None;
        }
        revision_tree::selected_tree_node(&panel.list_state, &tree.nodes)
    }
}
