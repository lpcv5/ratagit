use super::states::{BranchesPanelState, CommitsPanelState, StashPanelState};
use super::{InputMode, SidePanel};

/// Identity of the view displayed in a panel. Derived from reducer state, so
/// input, search, selectors and rendering never maintain a second focus state.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum ViewContext {
    #[default]
    Files,
    Branches,
    BranchCommits {
        branch: String,
    },
    Commits,
    CommitFiles {
        oid: String,
    },
    Stashes,
    StashFiles {
        index: Option<usize>,
    },
}

impl ViewContext {
    pub fn from_panels(
        panel: SidePanel,
        branches: &BranchesPanelState,
        commits: &CommitsPanelState,
        stash: &StashPanelState,
    ) -> Self {
        match panel {
            SidePanel::Files => Self::Files,
            SidePanel::LocalBranches if branches.commits_subview_active => Self::BranchCommits {
                branch: branches.commits_subview_source.clone().unwrap_or_default(),
            },
            SidePanel::LocalBranches => Self::Branches,
            SidePanel::Commits if commits.tree_mode.active => Self::CommitFiles {
                oid: commits
                    .tree_mode
                    .selected_source
                    .clone()
                    .unwrap_or_default(),
            },
            SidePanel::Commits => Self::Commits,
            SidePanel::Stash if stash.tree_mode.active => Self::StashFiles {
                index: stash.tree_mode.selected_source,
            },
            SidePanel::Stash => Self::Stashes,
        }
    }

    pub fn panel(&self) -> SidePanel {
        match self {
            Self::Files => SidePanel::Files,
            Self::Branches | Self::BranchCommits { .. } => SidePanel::LocalBranches,
            Self::Commits | Self::CommitFiles { .. } => SidePanel::Commits,
            Self::Stashes | Self::StashFiles { .. } => SidePanel::Stash,
        }
    }

    pub fn parent(&self) -> Option<Self> {
        match self {
            Self::BranchCommits { .. } => Some(Self::Branches),
            Self::CommitFiles { .. } => Some(Self::Commits),
            Self::StashFiles { .. } => Some(Self::Stashes),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiContext {
    pub diff_focused: bool,
    pub view: ViewContext,
    pub input: Option<InputMode>,
}

impl SidePanel {
    pub const ALL: [Self; 4] = [Self::Files, Self::LocalBranches, Self::Commits, Self::Stash];

    pub fn keymap_name(self) -> &'static str {
        match self {
            Self::Files => "files",
            Self::LocalBranches => "branches",
            Self::Commits => "commits",
            Self::Stash => "stash",
        }
    }
}
