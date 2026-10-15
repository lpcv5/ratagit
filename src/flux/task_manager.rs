//! Authoritative registration, admission and acceptance for background reads.
//! Execution order belongs to CommandOutput, EffectExecutor and repository workers.
use crate::background::{BackgroundCompleted, BackgroundLoad};
use std::collections::HashMap;

#[allow(unused_imports)]
pub use crate::background::{TaskGeneration, TaskKey, TaskPriority, TaskRequest};

const MAX_NORMAL_PRIORITY_TASKS: usize = 6;

pub struct TaskManager {
    next_generation: u64,
    latest: HashMap<TaskKey, TaskGeneration>,
    active: HashMap<TaskGeneration, TaskKey>,
    metrics: TaskMetrics,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TaskMetrics {
    pub enqueued_total: u64,
    pub dequeued_total: u64,
    pub ready_total: u64,
    pub finished_total: u64,
    pub failed_total: u64,
    pub cancelled_total: u64,
    pub stale_dropped_total: u64,
    pub queue_dropped_total: u64,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            next_generation: 1,
            latest: HashMap::new(),
            active: HashMap::new(),
            metrics: TaskMetrics::default(),
        }
    }

    /// Register before publishing the effect; identity comes only from its load.
    pub fn start(&mut self, load: BackgroundLoad, priority: TaskPriority) -> TaskRequest {
        let key = load.task_key();
        let generation = TaskGeneration(self.next_generation);
        self.next_generation += 1;
        self.latest.insert(key.clone(), generation);
        // Superseded work remains counted until completion or explicit cancellation.
        self.active.insert(generation, key.clone());
        self.metrics.enqueued_total += 1;
        self.metrics.dequeued_total += 1;
        TaskRequest {
            key,
            generation,
            priority,
            load,
        }
    }

    pub fn has_task(&self, key: &TaskKey) -> bool {
        self.active.values().any(|active| active == key)
    }

    pub fn can_start(&self, priority: TaskPriority) -> bool {
        matches!(priority, TaskPriority::High) || self.active.len() < MAX_NORMAL_PRIORITY_TASKS
    }

    pub fn is_empty(&self) -> bool {
        self.active.is_empty()
    }

    pub(crate) fn is_current(&self, completed: &BackgroundCompleted) -> bool {
        self.active.contains_key(&completed.request.generation)
            && self.latest.get(&completed.request.key) == Some(&completed.request.generation)
    }

    pub fn cancel(&mut self, key: &TaskKey) {
        self.active.retain(|_, active| active != key);
    }

    /// Retire work once, rejecting superseded generations before payload use.
    pub fn accept(&mut self, completed: &BackgroundCompleted) -> bool {
        let request = &completed.request;
        if self.active.remove(&request.generation).is_none() {
            return false;
        }
        self.metrics.ready_total += 1;
        if self.latest.get(&request.key) != Some(&request.generation) {
            self.metrics.stale_dropped_total += 1;
            return false;
        }
        if completed.result.is_ok() {
            self.metrics.finished_total += 1;
        } else {
            self.metrics.failed_total += 1;
        }
        true
    }

    #[allow(dead_code)]
    pub fn metrics(&self) -> TaskMetrics {
        self.metrics
    }
}

#[cfg(test)]
mod tests;
