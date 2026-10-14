use crate::git::GitRepository;
use std::sync::Arc;

/// Execution dependencies. Effects receive this handle and cannot access application state.
#[derive(Clone)]
pub struct AppRuntime {
    pub repo: Arc<dyn GitRepository>,
}

impl AppRuntime {
    pub fn new(repo: Box<dyn GitRepository>) -> Self {
        Self {
            repo: Arc::from(repo),
        }
    }
}
