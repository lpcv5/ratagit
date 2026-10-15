use super::diff::DiffTarget;
use super::files::FilesRenderView;
use super::revisions::{BranchesRenderView, CommitsRenderView, StashRenderView};
use super::{AppState, FileSelection, UiContext, ViewContext};
use crate::file_tree::FileTreeNode;

/// Shared, immutable selectors over state or a render snapshot.
pub struct SelectionView<'a> {
    pub context: ViewContext,
    files: FilesRenderView<'a>,
    revisions: super::revisions::RevisionView<'a>,
}

impl<'a> SelectionView<'a> {
    pub fn new(
        context: ViewContext,
        files: FilesRenderView<'a>,
        branches: BranchesRenderView<'a>,
        commits: CommitsRenderView<'a>,
        stash: StashRenderView<'a>,
    ) -> Self {
        Self {
            revisions: super::revisions::RevisionView::new(
                context.clone(),
                branches,
                commits,
                stash,
            ),
            context,
            files,
        }
    }

    pub fn file_selection(&self) -> FileSelection<'a> {
        FileSelection::new(self.files, self.context == ViewContext::Files)
    }

    pub fn selected_branch_name(&self) -> Option<&str> {
        self.revisions.selected_branch_name()
    }

    pub fn selected_commit_oid(&self) -> Option<&str> {
        self.revisions.selected_commit_oid()
    }

    pub fn selected_stash_index(&self) -> Option<usize> {
        self.revisions.selected_stash_index()
    }

    pub fn selected_revision_node(&self) -> Option<&'a FileTreeNode> {
        self.revisions.selected_revision_node()
    }

    pub fn selected_index(&self) -> Option<usize> {
        if self.context == ViewContext::Files {
            self.files.panel.selected()
        } else {
            self.revisions.selected_index()
        }
    }
    pub fn visible_count(&self) -> usize {
        if self.context == ViewContext::Files {
            self.files.tree_nodes.len()
        } else {
            self.revisions.visible_count()
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
            &self.revisions.view().branches,
            &self.revisions.view().commits,
            &self.revisions.view().stash,
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
            self.files.view().render_view(),
            self.revisions.view().branches.render_view(),
            self.revisions.view().commits.render_view(),
            self.revisions.view().stash.render_view(),
        )
    }

    #[cfg(test)]
    pub fn selected_tree_node(&self) -> Option<&FileTreeNode> {
        self.file_selection().selected_tree_node()
    }

    pub fn selected_branch_name(&self) -> Option<String> {
        self.selection().selected_branch_name().map(str::to_owned)
    }

    pub fn selected_stash_index(&self) -> Option<usize> {
        self.selection().selected_stash_index()
    }
}
