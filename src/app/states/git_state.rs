use crate::app::DetailState;
use crate::git::{DiffLine, GitStatus};

/// Git data state - all data fetched from repository
#[derive(Clone, Default)]
pub struct GitState {
    pub status: GitStatus,
    pub current_diff: Vec<DiffLine>,
    pub detail: DetailState,
}
