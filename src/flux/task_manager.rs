#![allow(dead_code)]

use std::collections::{HashMap, VecDeque};

const MAX_PENDING_REQUESTS: usize = 256;
const MAX_READY_RESULTS: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TaskGeneration(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskPriority {
    High,
    Normal,
    Low,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TaskKey {
    Status,
    Branches,
    Stashes,
    Commits,
    BranchCommits { branch: String },
    Diff { target: String },
    Write { op: String, scope: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskRequestKind {
    LoadStatus,
    LoadBranches,
    LoadStashes,
    LoadCommits { limit: usize },
    LoadBranchCommits { branch: String, limit: usize },
    LoadDiff { target: String },
    Write { op: String, scope: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskResultKind {
    Finished,
    Failed { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRequest {
    pub key: TaskKey,
    pub generation: TaskGeneration,
    pub priority: TaskPriority,
    pub kind: TaskRequestKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskResult {
    pub key: TaskKey,
    pub generation: TaskGeneration,
    pub kind: TaskResultKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TaskEntry {
    latest_generation: TaskGeneration,
    active_generation: Option<TaskGeneration>,
}

impl TaskEntry {
    fn new(generation: TaskGeneration) -> Self {
        Self {
            latest_generation: generation,
            active_generation: None,
        }
    }
}

pub trait TaskBridge {
    type Command;
    type Message;

    fn enqueue_command(request: TaskRequest) -> Self::Command;
    fn cancel_command(key: TaskKey, generation: TaskGeneration) -> Self::Command;
    fn ready_message(result: TaskResult) -> Self::Message;
}

#[derive(Default)]
pub struct TaskManager {
    next_generation: u64,
    registry: HashMap<TaskKey, TaskEntry>,
    pending: VecDeque<TaskRequest>,
    ready: VecDeque<TaskResult>,
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
            registry: HashMap::new(),
            pending: VecDeque::new(),
            ready: VecDeque::new(),
            metrics: TaskMetrics::default(),
        }
    }

    pub fn enqueue(
        &mut self,
        key: TaskKey,
        priority: TaskPriority,
        kind: TaskRequestKind,
    ) -> TaskRequest {
        let generation = TaskGeneration(self.next_generation);
        self.next_generation += 1;

        let entry = self
            .registry
            .entry(key.clone())
            .or_insert_with(|| TaskEntry::new(generation));
        entry.latest_generation = generation;

        self.pending.retain(|existing| existing.key != key);

        let request = TaskRequest {
            key,
            generation,
            priority,
            kind,
        };
        if self.pending.len() >= MAX_PENDING_REQUESTS {
            if let Some(index) = self
                .pending
                .iter()
                .rposition(|item| matches!(item.priority, TaskPriority::Low))
            {
                self.pending.remove(index);
            } else {
                self.pending.pop_back();
            }
            self.metrics.queue_dropped_total = self.metrics.queue_dropped_total.saturating_add(1);
        }

        match request.priority {
            TaskPriority::High => self.pending.push_front(request.clone()),
            TaskPriority::Normal => {
                let insert_at = self
                    .pending
                    .iter()
                    .position(|item| matches!(item.priority, TaskPriority::Low))
                    .unwrap_or(self.pending.len());
                self.pending.insert(insert_at, request.clone());
            }
            TaskPriority::Low => self.pending.push_back(request.clone()),
        }
        self.metrics.enqueued_total = self.metrics.enqueued_total.saturating_add(1);
        request
    }

    pub fn cancel(&mut self, key: &TaskKey) -> Option<TaskGeneration> {
        let entry = self.registry.get_mut(key)?;
        let pending_before = self.pending.len();
        self.pending.retain(|request| &request.key != key);
        if self.pending.len() != pending_before {
            self.metrics.cancelled_total = self.metrics.cancelled_total.saturating_add(1);
        }
        let cancelled = entry.active_generation.or(Some(entry.latest_generation));
        entry.active_generation = None;
        cancelled
    }

    pub fn mark_started(&mut self, key: &TaskKey, generation: TaskGeneration) {
        let entry = self
            .registry
            .entry(key.clone())
            .or_insert_with(|| TaskEntry::new(generation));
        if generation >= entry.latest_generation {
            entry.latest_generation = generation;
        }
        entry.active_generation = Some(generation);
    }

    pub fn mark_finished(&mut self, key: &TaskKey, generation: TaskGeneration) {
        if let Some(entry) = self.registry.get_mut(key) {
            if entry.active_generation == Some(generation) {
                entry.active_generation = None;
            }
        }
    }

    pub fn take_pending(&mut self) -> Vec<TaskRequest> {
        let drained: Vec<TaskRequest> = self.pending.drain(..).collect();
        self.metrics.dequeued_total = self
            .metrics
            .dequeued_total
            .saturating_add(drained.len() as u64);
        drained
    }

    pub fn submit_result(&mut self, result: TaskResult) {
        if self.ready.len() >= MAX_READY_RESULTS {
            self.ready.pop_front();
            self.metrics.queue_dropped_total = self.metrics.queue_dropped_total.saturating_add(1);
        }
        self.metrics.ready_total = self.metrics.ready_total.saturating_add(1);
        self.ready.push_back(result);
    }

    pub fn collect_ready(&mut self) -> Vec<TaskResult> {
        let mut accepted = Vec::new();
        while let Some(result) = self.ready.pop_front() {
            let is_latest = self
                .registry
                .get(&result.key)
                .map(|entry| entry.latest_generation == result.generation)
                .unwrap_or(false);
            if is_latest {
                self.mark_finished(&result.key, result.generation);
                match result.kind {
                    TaskResultKind::Finished => {
                        self.metrics.finished_total = self.metrics.finished_total.saturating_add(1);
                    }
                    TaskResultKind::Failed { .. } => {
                        self.metrics.failed_total = self.metrics.failed_total.saturating_add(1);
                    }
                }
                accepted.push(result);
            } else {
                self.metrics.stale_dropped_total =
                    self.metrics.stale_dropped_total.saturating_add(1);
            }
        }
        accepted
    }

    pub fn latest_generation(&self, key: &TaskKey) -> Option<TaskGeneration> {
        self.registry.get(key).map(|entry| entry.latest_generation)
    }

    pub fn metrics(&self) -> TaskMetrics {
        self.metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enqueue_increments_generation_for_same_key() {
        let mut manager = TaskManager::new();
        let key = TaskKey::Status;

        let first = manager.enqueue(
            key.clone(),
            TaskPriority::Normal,
            TaskRequestKind::LoadStatus,
        );
        let second = manager.enqueue(key.clone(), TaskPriority::High, TaskRequestKind::LoadStatus);

        assert!(second.generation > first.generation);
        assert_eq!(manager.latest_generation(&key), Some(second.generation));
    }

    #[test]
    fn collect_ready_filters_stale_result_by_generation() {
        let mut manager = TaskManager::new();
        let key = TaskKey::Diff {
            target: "files:a.txt".to_string(),
        };

        let first = manager.enqueue(
            key.clone(),
            TaskPriority::Normal,
            TaskRequestKind::LoadDiff {
                target: "files:a.txt".to_string(),
            },
        );
        let second = manager.enqueue(
            key.clone(),
            TaskPriority::High,
            TaskRequestKind::LoadDiff {
                target: "files:a.txt".to_string(),
            },
        );

        manager.mark_started(&key, first.generation);
        manager.mark_started(&key, second.generation);

        manager.submit_result(TaskResult {
            key: key.clone(),
            generation: first.generation,
            kind: TaskResultKind::Finished,
        });
        manager.submit_result(TaskResult {
            key: key.clone(),
            generation: second.generation,
            kind: TaskResultKind::Finished,
        });

        let ready = manager.collect_ready();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].generation, second.generation);
        assert_eq!(manager.metrics().stale_dropped_total, 1);
    }

    #[test]
    fn cancel_returns_active_or_latest_generation() {
        let mut manager = TaskManager::new();
        let key = TaskKey::Branches;
        let request = manager.enqueue(
            key.clone(),
            TaskPriority::Normal,
            TaskRequestKind::LoadBranches,
        );
        manager.mark_started(&key, request.generation);

        let cancelled = manager.cancel(&key);
        assert_eq!(cancelled, Some(request.generation));
    }

    #[test]
    fn take_pending_respects_priority_order() {
        let mut manager = TaskManager::new();
        let low = manager.enqueue(
            TaskKey::Write {
                op: "low".to_string(),
                scope: "a".to_string(),
            },
            TaskPriority::Low,
            TaskRequestKind::Write {
                op: "low".to_string(),
                scope: "a".to_string(),
            },
        );
        let normal = manager.enqueue(
            TaskKey::Branches,
            TaskPriority::Normal,
            TaskRequestKind::LoadBranches,
        );
        let high = manager.enqueue(
            TaskKey::Status,
            TaskPriority::High,
            TaskRequestKind::LoadStatus,
        );

        let pending = manager.take_pending();
        assert_eq!(pending.len(), 3);
        assert_eq!(pending[0].generation, high.generation);
        assert_eq!(pending[1].generation, normal.generation);
        assert_eq!(pending[2].generation, low.generation);
    }

    #[test]
    fn enqueue_applies_pending_queue_cap() {
        let mut manager = TaskManager::new();
        let total = MAX_PENDING_REQUESTS + 10;
        for i in 0..total {
            manager.enqueue(
                TaskKey::Write {
                    op: "bulk".to_string(),
                    scope: format!("s{}", i),
                },
                TaskPriority::Low,
                TaskRequestKind::Write {
                    op: "bulk".to_string(),
                    scope: format!("s{}", i),
                },
            );
        }

        let pending = manager.take_pending();
        assert_eq!(pending.len(), MAX_PENDING_REQUESTS);
        assert_eq!(manager.metrics().queue_dropped_total, 10);
    }
}
