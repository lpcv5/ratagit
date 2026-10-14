use super::panel_state::{
    BranchesPanelState, CommitsPanelState, RenderCache, SidePanel, StashPanelState,
};
use crate::app::dirty_flags::DirtyFlags;

/// Shared UI state: panel navigation, visual modes and dirty tracking.
/// Diff focus, hunk selection and scrolling belong to DiffState.
#[derive(Clone)]
pub struct UiState {
    pub active_panel: SidePanel,
    pub branches: BranchesPanelState,
    pub commits: CommitsPanelState,
    pub stash: StashPanelState,
    pub dirty: DirtyFlags,
    pub render_cache: RenderCache,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            active_panel: SidePanel::Files,
            branches: BranchesPanelState::default(),
            commits: CommitsPanelState::default(),
            stash: StashPanelState::default(),
            dirty: DirtyFlags::default(),
            render_cache: RenderCache::default(),
        }
    }
}
