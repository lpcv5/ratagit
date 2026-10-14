use super::diff::DiffTarget;
use super::states::{BranchesPanelState, CommitsPanelState, StashPanelState};
use super::{AppState, FileSelection, FilesPanelState, SidePanel, UiContext, ViewContext};
use crate::file_tree::FileTreeNode;

/// Shared, immutable selectors over state or a render snapshot.
pub struct SelectionView<'a> {
    pub context: ViewContext,
    files: &'a FilesPanelState,
    branches: &'a BranchesPanelState,
    commits: &'a CommitsPanelState,
    stash: &'a StashPanelState,
}

impl<'a> SelectionView<'a> {
    pub fn new(
        context: ViewContext,
        files: &'a FilesPanelState,
        branches: &'a BranchesPanelState,
        commits: &'a CommitsPanelState,
        stash: &'a StashPanelState,
    ) -> Self {
        Self {
            context,
            files,
            branches,
            commits,
            stash,
        }
    }

    pub fn file_selection(&self) -> FileSelection<'a> {
        FileSelection::new(self.files, self.context == ViewContext::Files)
    }

    pub fn selected_branch_name(&self) -> Option<&str> {
        match &self.context {
            ViewContext::Branches => self
                .branches
                .items
                .get(self.branches.panel.list_state.selected()?)
                .map(|b| b.name.as_str()),
            ViewContext::BranchCommits { branch } => {
                (!branch.is_empty()).then_some(branch.as_str())
            }
            _ => None,
        }
    }

    pub fn selected_branch_subview_commit_oid(&self) -> Option<&'a str> {
        if !matches!(self.context, ViewContext::BranchCommits { .. }) {
            return None;
        }
        self.branches
            .commits_subview
            .items
            .get(self.branches.commits_subview.panel.list_state.selected()?)
            .map(|c| c.oid.as_str())
    }

    pub fn selected_commit_oid(&self) -> Option<&str> {
        match &self.context {
            ViewContext::Commits => self
                .commits
                .items
                .get(self.commits.panel.list_state.selected()?)
                .map(|c| c.oid.as_str()),
            ViewContext::CommitFiles { oid } => (!oid.is_empty()).then_some(oid.as_str()),
            ViewContext::BranchCommits { .. } => self.selected_branch_subview_commit_oid(),
            _ => None,
        }
    }

    pub fn selected_stash_index(&self) -> Option<usize> {
        match self.context {
            ViewContext::Stashes => self
                .stash
                .items
                .get(self.stash.panel.list_state.selected()?)
                .map(|s| s.index),
            ViewContext::StashFiles { index } => index,
            _ => None,
        }
    }

    pub fn selected_revision_node(&self) -> Option<&'a FileTreeNode> {
        match self.context {
            ViewContext::CommitFiles { .. } => super::revision_tree::selected_tree_node(
                &self.commits.panel.list_state,
                &self.commits.tree_mode.nodes,
            ),
            ViewContext::StashFiles { .. } => super::revision_tree::selected_tree_node(
                &self.stash.panel.list_state,
                &self.stash.tree_mode.nodes,
            ),
            _ => None,
        }
    }

    pub fn selected_index(&self) -> Option<usize> {
        match self.context {
            ViewContext::Files => self.files.panel.list_state.selected(),
            ViewContext::Branches => self.branches.panel.list_state.selected(),
            ViewContext::BranchCommits { .. } => {
                self.branches.commits_subview.panel.list_state.selected()
            }
            ViewContext::Commits | ViewContext::CommitFiles { .. } => {
                self.commits.panel.list_state.selected()
            }
            ViewContext::Stashes | ViewContext::StashFiles { .. } => {
                self.stash.panel.list_state.selected()
            }
        }
    }

    pub fn visible_count(&self) -> usize {
        match self.context {
            ViewContext::Files => self.files.tree_nodes.len(),
            ViewContext::Branches => self.branches.items.len(),
            ViewContext::BranchCommits { .. } => self.branches.commits_subview.items.len(),
            ViewContext::Commits => self.commits.items.len(),
            ViewContext::CommitFiles { .. } => self.commits.tree_mode.nodes.len(),
            ViewContext::Stashes => self.stash.items.len(),
            ViewContext::StashFiles { .. } => self.stash.tree_mode.nodes.len(),
        }
    }

    pub fn diff_target(&self) -> DiffTarget {
        match &self.context {
            ViewContext::Files => match self.file_selection().selected_tree_node() {
                Some(node) if node.is_dir => DiffTarget::Directory {
                    path: node.path.clone(),
                },
                Some(node) => DiffTarget::File {
                    path: node.path.clone(),
                    status: node.status.clone(),
                },
                None => DiffTarget::None,
            },
            ViewContext::Branches => self
                .selected_branch_name()
                .map(|name| DiffTarget::Branch { name: name.into() })
                .unwrap_or(DiffTarget::None),
            ViewContext::Commits
            | ViewContext::BranchCommits { .. }
            | ViewContext::CommitFiles { .. } => self
                .selected_commit_oid()
                .map(|oid| DiffTarget::Commit {
                    oid: oid.into(),
                    path: self.selected_revision_node().map(|node| node.path.clone()),
                })
                .unwrap_or(DiffTarget::None),
            ViewContext::Stashes | ViewContext::StashFiles { .. } => self
                .selected_stash_index()
                .map(|index| DiffTarget::Stash {
                    index,
                    path: self.selected_revision_node().map(|node| node.path.clone()),
                })
                .unwrap_or(DiffTarget::None),
        }
    }
}

impl AppState {
    pub fn view_context(&self) -> ViewContext {
        ViewContext::from_panels(
            self.ui.active_panel,
            &self.ui.branches,
            &self.ui.commits,
            &self.ui.stash,
        )
    }

    pub fn ui_context(&self) -> UiContext {
        UiContext {
            diff_focused: self.diff.interaction().focused,
            view: self.view_context(),
            input: self.input.mode,
        }
    }

    pub fn selection(&self) -> SelectionView<'_> {
        SelectionView::new(
            self.view_context(),
            self.files.view(),
            &self.ui.branches,
            &self.ui.commits,
            &self.ui.stash,
        )
    }

    #[cfg(test)]
    pub fn selected_tree_node(&self) -> Option<&FileTreeNode> {
        self.file_selection().selected_tree_node()
    }

    pub fn selected_branch_name(&self) -> Option<String> {
        self.selection().selected_branch_name().map(str::to_owned)
    }

    pub fn selected_commit_oid(&self) -> Option<String> {
        // The commit panel's actions intentionally remain scoped to its window.
        if self.ui.active_panel != SidePanel::Commits {
            return None;
        }
        self.selection().selected_commit_oid().map(str::to_owned)
    }

    pub fn selected_stash_index(&self) -> Option<usize> {
        self.selection().selected_stash_index()
    }

    pub(super) fn selected_commit_tree_node(&self) -> Option<&FileTreeNode> {
        if self.ui.active_panel != SidePanel::Commits {
            return None;
        }
        self.selection().selected_revision_node()
    }

    pub(super) fn selected_stash_tree_node(&self) -> Option<&FileTreeNode> {
        if self.ui.active_panel != SidePanel::Stash {
            return None;
        }
        self.selection().selected_revision_node()
    }
}
