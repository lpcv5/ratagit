use super::background_poll::PendingBackgroundTask;
use crate::flux::task_manager::{TaskGeneration, TaskManager};
use std::collections::HashMap;

pub(super) struct BackgroundTaskRunner {
    pub(super) task_manager: TaskManager,
    pub(super) pending_background_tasks: HashMap<TaskGeneration, PendingBackgroundTask>,
    pub(super) commits_requested_limit: usize,
}

impl BackgroundTaskRunner {
    pub(super) fn new(commits_requested_limit: usize) -> Self {
        Self {
            task_manager: TaskManager::new(),
            pending_background_tasks: HashMap::new(),
            commits_requested_limit,
        }
    }
}
