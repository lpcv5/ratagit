use crate::git::GitRepository;
use std::sync::Arc;

/// Execution dependencies. Effects receive this handle and cannot access application state.
#[derive(Clone)]
pub struct AppRuntime {
    pub repo: Arc<dyn GitRepository>,
    pub desktop: Arc<dyn crate::desktop::Desktop>,
}

impl AppRuntime {
    pub fn new(repo: Box<dyn GitRepository>) -> Self {
        Self {
            repo: Arc::from(repo),
            desktop: Arc::new(crate::desktop::SystemDesktop::default()),
        }
    }
}
