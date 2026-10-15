use crate::app::states::PanelState;
use crate::app::{SidePanel, ViewContext};
use crate::file_tree::FileTreeNode;
use crate::git::{BranchInfo, CommitInfo, FileEntry, StashInfo};
use crate::shared::Shared;
use std::collections::HashSet;
use std::path::PathBuf;

const INITIAL_COMMITS_LOAD_LIMIT: usize = 30;

/// Generic tree-navigation state for commit and stash panels.
#[derive(Clone)]
pub struct TreeModeState<T> {
    pub active: bool,
    pub nodes: Shared<Vec<FileTreeNode>>,
    pub files: Shared<Vec<FileEntry>>,
    pub expanded_dirs: Shared<HashSet<PathBuf>>,
    pub selected_source: Option<T>,
}

impl<T> Default for TreeModeState<T> {
    fn default() -> Self {
        Self {
            active: false,
            nodes: Shared::default(),
            files: Shared::default(),
            expanded_dirs: Shared::default(),
            selected_source: None,
        }
    }
}

#[derive(Default, Clone)]
pub struct BranchesPanelState {
    pub panel: PanelState,
    pub items: Shared<Vec<BranchInfo>>,
    pub is_fetching_remote: bool,
    pub commits_subview_active: bool,
    pub commits_subview_loading: bool,
    pub commits_subview_source: Option<String>,
    pub commits_subview: CommitsPanelState,
}

#[derive(Default, Clone)]
pub struct CommitsPanelState {
    pub panel: PanelState,
    pub items: Shared<Vec<CommitInfo>>,
    pub dirty: bool,
    pub tree_mode: TreeModeState<String>,
    pub highlighted_oids: Shared<HashSet<String>>,
}

#[derive(Default, Clone)]
pub struct StashPanelState {
    pub panel: PanelState,
    pub items: Shared<Vec<StashInfo>>,
    pub tree_mode: TreeModeState<usize>,
}

/// Domain revision state; render models exclude load and tree bookkeeping.
#[derive(Default, Clone)]
pub struct RevisionPanels {
    pub branches: BranchesPanelState,
    pub commits: CommitsPanelState,
    pub stash: StashPanelState,
}

pub struct RevisionState {
    pub(super) panels: RevisionPanels,
    pub(super) commits_requested_limit: usize,
}

impl RevisionState {
    pub fn new(
        branches: Vec<BranchInfo>,
        commits: Vec<CommitInfo>,
        stashes: Vec<StashInfo>,
        preloaded: bool,
    ) -> Self {
        Self {
            panels: RevisionPanels {
                branches: BranchesPanelState {
                    items: branches.into(),
                    ..Default::default()
                },
                commits: CommitsPanelState {
                    items: commits.into(),
                    dirty: !preloaded,
                    ..Default::default()
                },
                stash: StashPanelState {
                    items: stashes.into(),
                    ..Default::default()
                },
            },
            commits_requested_limit: INITIAL_COMMITS_LOAD_LIMIT,
        }
    }

    pub fn view(&self) -> &RevisionPanels {
        &self.panels
    }
    pub fn commits_requested_limit(&self) -> usize {
        self.commits_requested_limit
    }
    pub fn selection(&self, active: SidePanel) -> super::RevisionView<'_> {
        super::RevisionView::new(
            ViewContext::from_panels(
                active,
                &self.panels.branches,
                &self.panels.commits,
                &self.panels.stash,
            ),
            self.panels.branches.render_view(),
            self.panels.commits.render_view(),
            self.panels.stash.render_view(),
        )
    }
}

// Compatibility fixture access for existing tests only. Production consumers
// can only borrow the immutable panel projection or reduce feature actions.
#[cfg(test)]
impl std::ops::Deref for RevisionState {
    type Target = RevisionPanels;
    fn deref(&self) -> &Self::Target {
        &self.panels
    }
}
#[cfg(test)]
impl std::ops::DerefMut for RevisionState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.panels
    }
}
