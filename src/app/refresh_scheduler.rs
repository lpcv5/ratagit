use super::app::RefreshKind;

pub(super) struct RefreshScheduler {
    pub(super) pending_refresh: Option<RefreshKind>,
    pub(super) pending_full_status_after_fast: bool,
    pub(super) pending_refresh_fast_done: bool,
}

impl RefreshScheduler {
    pub(super) fn new(pending_full_status_after_fast: bool) -> Self {
        Self {
            pending_refresh: None,
            pending_full_status_after_fast,
            pending_refresh_fast_done: false,
        }
    }
}
