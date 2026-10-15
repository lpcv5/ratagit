use super::{BranchesRenderView, CommitsRenderView, StashRenderView};
use crate::app::ViewContext;
use crate::file_tree::FileTreeNode;

/// Shared selection rules for reducer state and immutable render snapshots.
pub struct RevisionView<'a> {
    pub context: ViewContext,
    branches: BranchesRenderView<'a>,
    commits: CommitsRenderView<'a>,
    stash: StashRenderView<'a>,
}

impl<'a> RevisionView<'a> {
    pub fn new(
        context: ViewContext,
        branches: BranchesRenderView<'a>,
        commits: CommitsRenderView<'a>,
        stash: StashRenderView<'a>,
    ) -> Self {
        Self {
            context,
            branches,
            commits,
            stash,
        }
    }
    pub fn selected_branch_name(&self) -> Option<&str> {
        match &self.context {
            ViewContext::Branches => self
                .branches
                .items
                .get(self.branches.panel.selected()?)
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
            .get(self.branches.commits_subview.panel.selected()?)
            .map(|c| c.oid.as_str())
    }

    pub fn selected_commit_oid(&self) -> Option<&str> {
        match &self.context {
            ViewContext::Commits => self
                .commits
                .items
                .get(self.commits.panel.selected()?)
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
                .get(self.stash.panel.selected()?)
                .map(|s| s.index),
            ViewContext::StashFiles { index } => index,
            _ => None,
        }
    }

    pub fn selected_revision_node(&self) -> Option<&'a FileTreeNode> {
        match self.context {
            ViewContext::CommitFiles { .. } => self
                .commits
                .panel
                .selected()
                .and_then(|index| self.commits.tree_mode.nodes.get(index)),
            ViewContext::StashFiles { .. } => self
                .stash
                .panel
                .selected()
                .and_then(|index| self.stash.tree_mode.nodes.get(index)),
            _ => None,
        }
    }

    pub fn selected_index(&self) -> Option<usize> {
        match self.context {
            ViewContext::Files => None,
            ViewContext::Branches => self.branches.panel.selected(),
            ViewContext::BranchCommits { .. } => self.branches.commits_subview.panel.selected(),
            ViewContext::Commits | ViewContext::CommitFiles { .. } => self.commits.panel.selected(),
            ViewContext::Stashes | ViewContext::StashFiles { .. } => self.stash.panel.selected(),
        }
    }

    pub fn visible_count(&self) -> usize {
        match self.context {
            ViewContext::Files => 0,
            ViewContext::Branches => self.branches.items.len(),
            ViewContext::BranchCommits { .. } => self.branches.commits_subview.items.len(),
            ViewContext::Commits => self.commits.items.len(),
            ViewContext::CommitFiles { .. } => self.commits.tree_mode.nodes.len(),
            ViewContext::Stashes => self.stash.items.len(),
            ViewContext::StashFiles { .. } => self.stash.tree_mode.nodes.len(),
        }
    }
}
