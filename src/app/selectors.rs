use super::diff_loader;
use crate::app::{App, SidePanel};
use crate::flux::commits_backend::{CommitsBackend, CommitsPanelDiffRequest};
use crate::flux::files_backend::{FilesBackend, FilesPanelDiffRequest};
use crate::flux::git_backend::stash::{StashBackend, StashPanelDiffRequest};
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
        if self.ui.active_panel != SidePanel::LocalBranches
            || !self.ui.branches.commits_subview_active
        {
            return None;
        }
        let idx = self
            .ui
            .branches
            .commits_subview
            .panel
            .list_state
            .selected()?;
        self.ui
            .branches
            .commits_subview
            .items
            .get(idx)
            .map(|c| c.oid.clone())
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

    pub(super) fn selected_diff_target(&self) -> diff_loader::DiffTarget {
        match self.ui.active_panel {
            SidePanel::Files => {
                match FilesBackend::selected_diff_request(&self.current_files_view_state()) {
                    FilesPanelDiffRequest::None => diff_loader::DiffTarget::None,
                    FilesPanelDiffRequest::Directory { path } => {
                        diff_loader::DiffTarget::Directory { path }
                    }
                    FilesPanelDiffRequest::File { path, staged } => diff_loader::DiffTarget::File {
                        path,
                        status: if staged {
                            crate::ui::widgets::file_tree::FileTreeNodeStatus::Staged(
                                crate::git::FileStatus::Modified,
                            )
                        } else {
                            match self.selected_tree_node().map(|node| node.status.clone()) {
                                Some(status) => status,
                                None => {
                                    crate::ui::widgets::file_tree::FileTreeNodeStatus::Unstaged(
                                        crate::git::FileStatus::Modified,
                                    )
                                }
                            }
                        },
                    },
                }
            }
            SidePanel::Commits => {
                match CommitsBackend::selected_diff_request(&self.current_commits_view_state()) {
                    CommitsPanelDiffRequest::None => diff_loader::DiffTarget::None,
                    CommitsPanelDiffRequest::Commit { oid, path } => {
                        diff_loader::DiffTarget::Commit { oid, path }
                    }
                }
            }
            SidePanel::Stash => {
                match StashBackend::selected_diff_request(&self.current_stash_view_state()) {
                    StashPanelDiffRequest::None => diff_loader::DiffTarget::None,
                    StashPanelDiffRequest::Stash { index, path } => {
                        diff_loader::DiffTarget::Stash { index, path }
                    }
                }
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
}
