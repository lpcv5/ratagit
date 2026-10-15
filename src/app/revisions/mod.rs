//! Revision lists, file trees, navigation and load decisions.
//! Mutable state is private to this feature. Repository I/O, task generations
//! and cross-feature coordination remain in the application layer.

pub(crate) mod highlight;
mod render;
mod selectors;
mod state;
#[cfg(test)]
mod tests;
pub(super) mod tree;
mod update;

pub use render::{
    BranchesRenderModel, BranchesRenderView, CommitsRenderModel, CommitsRenderView,
    StashRenderModel, StashRenderView,
};
pub use selectors::RevisionView;
pub use state::{
    BranchesPanelState, CommitsPanelState, RevisionState, StashPanelState, TreeModeState,
};
pub use update::{RevisionAction, RevisionCancel, RevisionRead, RevisionUpdate};
