use crate::git::{DiffLine, GitStatus};

/// Git data state - all data fetched from repository
#[derive(Clone)]
pub struct GitState {
    pub status: GitStatus,
    pub current_diff: Vec<DiffLine>,
}

impl Default for GitState {
    fn default() -> Self {
        Self {
            status: GitStatus::default(),
            current_diff: Vec::new(),
        }
    }
}
