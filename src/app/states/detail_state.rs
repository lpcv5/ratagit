use crate::flux::git_backend::detail::DetailPanelState;

/// Detail data state for the right-side detail/diff surface.
#[derive(Clone, Default)]
pub struct DetailState {
    pub panel: DetailPanelState,
}
