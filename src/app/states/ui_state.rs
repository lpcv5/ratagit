use super::panel_state::{RenderCache, SidePanel};
use crate::app::dirty_flags::DirtyFlags;

/// Shared focus, dirty tracking and render caches. Feature-owned navigation,
/// visual selection and diff interaction live in files, revisions and diff.
#[derive(Clone)]
pub struct UiState {
    pub active_panel: SidePanel,
    pub dirty: DirtyFlags,
    pub render_cache: RenderCache,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            active_panel: SidePanel::Files,
            dirty: DirtyFlags::default(),
            render_cache: RenderCache::default(),
        }
    }
}
