use crate::git::{DiffLine, GitStatus};

/// Git data state - all data fetched from repository
#[derive(Clone, Default)]
pub struct GitState {
    pub status: GitStatus,
    pub operation: crate::git::operation::OperationMachine,
    pub current_patch: Option<std::sync::Arc<crate::git::patch::Patch>>,
    pub current_diff: Vec<DiffLine>,
}
