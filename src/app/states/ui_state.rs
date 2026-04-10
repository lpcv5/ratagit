use super::panel_state::{
    BranchesPanelState, CommitsPanelState, FilesPanelState, RenderCache, SidePanel, StashPanelState,
};
use crate::app::dirty_flags::DirtyFlags;

/// UI state — panel navigation, selection, visual modes, dirty tracking.
#[derive(Clone)]
pub struct UiState {
    pub active_panel: SidePanel,
    pub files: FilesPanelState,
    pub branches: BranchesPanelState,
    pub commits: CommitsPanelState,
    pub stash: StashPanelState,
    pub diff_scroll: usize,
    pub dirty: DirtyFlags,
    pub render_cache: RenderCache,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            active_panel: SidePanel::Files,
            files: FilesPanelState::default(),
            branches: BranchesPanelState::default(),
            commits: CommitsPanelState::default(),
            stash: StashPanelState::default(),
            diff_scroll: 0,
            dirty: DirtyFlags::default(),
            render_cache: RenderCache::default(),
        }
    }
}
