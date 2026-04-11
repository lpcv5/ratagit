use super::app::{App, RefreshKind};
use super::diff_cache;
use super::diff_loader;
use super::states::SidePanel;
use crate::flux::branch_backend::BranchBackend;
use crate::flux::task_manager::{TaskGeneration, TaskKey, TaskRequest, TaskResult, TaskResultKind};
use crate::git::{BranchInfo, CommitInfo, DiffLine, GitError, GitStatus, StashInfo};
use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use tracing::debug;

pub(super) enum BackgroundReceiver {
    Status {
        fast: bool,
        rx: Receiver<Result<GitStatus, GitError>>,
    },
    Branches(Receiver<Result<Vec<BranchInfo>, GitError>>),
    Stashes(Receiver<Result<Vec<StashInfo>, GitError>>),
    Commits(Receiver<Result<Vec<CommitInfo>, GitError>>),
    CommitsFast(Receiver<Result<Vec<CommitInfo>, GitError>>),
    BranchCommits {
        branch: String,
        rx: Receiver<Result<Vec<CommitInfo>, GitError>>,
    },
    Diff {
        cache_key: diff_cache::DiffCacheKey,
        rx: Receiver<Result<Vec<DiffLine>, GitError>>,
    },
}

pub(super) enum BackgroundPayload {
    Status {
        status: GitStatus,
        fast: bool,
    },
    Branches(Vec<BranchInfo>),
    Stashes(Vec<StashInfo>),
    Commits(Vec<CommitInfo>),
    CommitsFast(Vec<CommitInfo>),
    BranchCommits {
        branch: String,
        items: Vec<CommitInfo>,
    },
    Diff {
        cache_key: diff_cache::DiffCacheKey,
        diff: Vec<DiffLine>,
    },
}

pub(super) struct PendingBackgroundTask {
    pub(super) request: TaskRequest,
    pub(super) receiver: BackgroundReceiver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DiffRefreshSource {
    Status,
    Branches,
    Stashes,
    Commits,
}

impl App {
    pub(super) fn should_schedule_diff_for_refresh(&self, source: DiffRefreshSource) -> bool {
        use diff_loader::DiffTarget;
        matches!(
            (source, self.selected_diff_target()),
            (DiffRefreshSource::Status, DiffTarget::File { .. })
                | (DiffRefreshSource::Status, DiffTarget::Directory { .. })
                | (DiffRefreshSource::Branches, DiffTarget::Branch { .. })
                | (DiffRefreshSource::Stashes, DiffTarget::Stash { .. })
                | (DiffRefreshSource::Commits, DiffTarget::Commit { .. })
        )
    }

    pub fn process_background_refresh_tick(&mut self) {
        if let Some(kind) = self.refresh.pending_refresh.take() {
            let mut deferred_kind = None;
            if self.has_background_task(&TaskKey::Status) {
                deferred_kind = Some(kind);
            } else if !self.refresh.pending_refresh_fast_done {
                if self.start_status_load(true) {
                    self.refresh.pending_refresh_fast_done = true;
                    deferred_kind = Some(kind);
                } else {
                    deferred_kind = Some(RefreshKind::StatusOnly);
                }
            } else {
                if matches!(kind, RefreshKind::StatusAndRefs | RefreshKind::Full)
                    && (!self.start_branches_load() || !self.start_stashes_load())
                {
                    deferred_kind = Some(match deferred_kind {
                        None => RefreshKind::StatusAndRefs,
                        Some(existing) => {
                            Self::max_refresh_kind(existing, RefreshKind::StatusAndRefs)
                        }
                    });
                }
                if matches!(kind, RefreshKind::Full) {
                    self.ui.commits.dirty = true;
                }
                if deferred_kind.is_none() {
                    self.refresh.pending_refresh_fast_done = false;
                }
            }
            if let Some(kind) = deferred_kind {
                self.refresh.pending_refresh = Some(match self.refresh.pending_refresh {
                    None => kind,
                    Some(existing) => Self::max_refresh_kind(existing, kind),
                });
            }
        }

        if self.ui.active_panel == SidePanel::Commits && self.ui.commits.dirty {
            let _ = self.start_commits_load(self.tasks.commits_requested_limit);
        }
        self.maybe_schedule_commits_extend();

        self.start_pending_diff_load();

        let mut schedule_diff = false;

        let mut next_pending_background = HashMap::new();
        let mut finished_payloads: HashMap<(TaskKey, TaskGeneration), BackgroundPayload> =
            HashMap::new();
        for (generation, task) in self.tasks.pending_background_tasks.drain() {
            let PendingBackgroundTask { request, receiver } = task;
            match receiver {
                BackgroundReceiver::Status { fast, rx } => match rx.try_recv() {
                    Ok(Ok(status)) => {
                        self.tasks.task_manager.submit_result(TaskResult {
                            key: request.key.clone(),
                            generation: request.generation,
                            kind: TaskResultKind::Finished,
                        });
                        finished_payloads.insert(
                            (request.key.clone(), request.generation),
                            BackgroundPayload::Status { status, fast },
                        );
                    }
                    Ok(Err(err)) => self.tasks.task_manager.submit_result(TaskResult {
                        key: request.key.clone(),
                        generation: request.generation,
                        kind: TaskResultKind::Failed {
                            reason: format!("status load failed: {}", err),
                        },
                    }),
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        next_pending_background.insert(
                            generation,
                            PendingBackgroundTask {
                                request,
                                receiver: BackgroundReceiver::Status { fast, rx },
                            },
                        );
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        self.tasks.task_manager.submit_result(TaskResult {
                            key: request.key.clone(),
                            generation: request.generation,
                            kind: TaskResultKind::Failed {
                                reason: "status load disconnected".to_string(),
                            },
                        });
                    }
                },
                BackgroundReceiver::Branches(rx) => match rx.try_recv() {
                    Ok(Ok(items)) => {
                        self.tasks.task_manager.submit_result(TaskResult {
                            key: request.key.clone(),
                            generation: request.generation,
                            kind: TaskResultKind::Finished,
                        });
                        finished_payloads.insert(
                            (request.key.clone(), request.generation),
                            BackgroundPayload::Branches(items),
                        );
                    }
                    Ok(Err(err)) => self.tasks.task_manager.submit_result(TaskResult {
                        key: request.key.clone(),
                        generation: request.generation,
                        kind: TaskResultKind::Failed {
                            reason: format!("branches load failed: {}", err),
                        },
                    }),
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        next_pending_background.insert(
                            generation,
                            PendingBackgroundTask {
                                request,
                                receiver: BackgroundReceiver::Branches(rx),
                            },
                        );
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        self.tasks.task_manager.submit_result(TaskResult {
                            key: request.key.clone(),
                            generation: request.generation,
                            kind: TaskResultKind::Failed {
                                reason: "branches load disconnected".to_string(),
                            },
                        });
                    }
                },
                BackgroundReceiver::Stashes(rx) => match rx.try_recv() {
                    Ok(Ok(items)) => {
                        self.tasks.task_manager.submit_result(TaskResult {
                            key: request.key.clone(),
                            generation: request.generation,
                            kind: TaskResultKind::Finished,
                        });
                        finished_payloads.insert(
                            (request.key.clone(), request.generation),
                            BackgroundPayload::Stashes(items),
                        );
                    }
                    Ok(Err(err)) => self.tasks.task_manager.submit_result(TaskResult {
                        key: request.key.clone(),
                        generation: request.generation,
                        kind: TaskResultKind::Failed {
                            reason: format!("stashes load failed: {}", err),
                        },
                    }),
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        next_pending_background.insert(
                            generation,
                            PendingBackgroundTask {
                                request,
                                receiver: BackgroundReceiver::Stashes(rx),
                            },
                        );
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        self.tasks.task_manager.submit_result(TaskResult {
                            key: request.key.clone(),
                            generation: request.generation,
                            kind: TaskResultKind::Failed {
                                reason: "stashes load disconnected".to_string(),
                            },
                        });
                    }
                },
                BackgroundReceiver::Commits(rx) => match rx.try_recv() {
                    Ok(Ok(items)) => {
                        self.tasks.task_manager.submit_result(TaskResult {
                            key: request.key.clone(),
                            generation: request.generation,
                            kind: TaskResultKind::Finished,
                        });
                        finished_payloads.insert(
                            (request.key.clone(), request.generation),
                            BackgroundPayload::Commits(items),
                        );
                    }
                    Ok(Err(err)) => self.tasks.task_manager.submit_result(TaskResult {
                        key: request.key.clone(),
                        generation: request.generation,
                        kind: TaskResultKind::Failed {
                            reason: format!("commits load failed: {}", err),
                        },
                    }),
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        next_pending_background.insert(
                            generation,
                            PendingBackgroundTask {
                                request,
                                receiver: BackgroundReceiver::Commits(rx),
                            },
                        );
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        self.tasks.task_manager.submit_result(TaskResult {
                            key: request.key.clone(),
                            generation: request.generation,
                            kind: TaskResultKind::Failed {
                                reason: "commits load disconnected".to_string(),
                            },
                        });
                    }
                },
                BackgroundReceiver::CommitsFast(rx) => match rx.try_recv() {
                    Ok(Ok(items)) => {
                        self.tasks.task_manager.submit_result(TaskResult {
                            key: request.key.clone(),
                            generation: request.generation,
                            kind: TaskResultKind::Finished,
                        });
                        finished_payloads.insert(
                            (request.key.clone(), request.generation),
                            BackgroundPayload::CommitsFast(items),
                        );
                    }
                    Ok(Err(err)) => self.tasks.task_manager.submit_result(TaskResult {
                        key: request.key.clone(),
                        generation: request.generation,
                        kind: TaskResultKind::Failed {
                            reason: format!("commits load failed: {}", err),
                        },
                    }),
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        next_pending_background.insert(
                            generation,
                            PendingBackgroundTask {
                                request,
                                receiver: BackgroundReceiver::CommitsFast(rx),
                            },
                        );
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        self.tasks.task_manager.submit_result(TaskResult {
                            key: request.key.clone(),
                            generation: request.generation,
                            kind: TaskResultKind::Failed {
                                reason: "commits load disconnected".to_string(),
                            },
                        });
                    }
                },
                BackgroundReceiver::BranchCommits { branch, rx } => match rx.try_recv() {
                    Ok(Ok(items)) => {
                        self.tasks.task_manager.submit_result(TaskResult {
                            key: request.key.clone(),
                            generation: request.generation,
                            kind: TaskResultKind::Finished,
                        });
                        finished_payloads.insert(
                            (request.key.clone(), request.generation),
                            BackgroundPayload::BranchCommits { branch, items },
                        );
                    }
                    Ok(Err(err)) => self.tasks.task_manager.submit_result(TaskResult {
                        key: request.key.clone(),
                        generation: request.generation,
                        kind: TaskResultKind::Failed {
                            reason: format!("branch commits load failed: {}", err),
                        },
                    }),
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        next_pending_background.insert(
                            generation,
                            PendingBackgroundTask {
                                request,
                                receiver: BackgroundReceiver::BranchCommits { branch, rx },
                            },
                        );
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        self.tasks.task_manager.submit_result(TaskResult {
                            key: request.key.clone(),
                            generation: request.generation,
                            kind: TaskResultKind::Failed {
                                reason: "branch commits load disconnected".to_string(),
                            },
                        });
                    }
                },
                BackgroundReceiver::Diff { cache_key, rx } => match rx.try_recv() {
                    Ok(Ok(diff)) => {
                        self.tasks.task_manager.submit_result(TaskResult {
                            key: request.key.clone(),
                            generation: request.generation,
                            kind: TaskResultKind::Finished,
                        });
                        finished_payloads.insert(
                            (request.key.clone(), request.generation),
                            BackgroundPayload::Diff { cache_key, diff },
                        );
                    }
                    Ok(Err(err)) => self.tasks.task_manager.submit_result(TaskResult {
                        key: request.key.clone(),
                        generation: request.generation,
                        kind: TaskResultKind::Failed {
                            reason: format!("diff load failed: {}", err),
                        },
                    }),
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        next_pending_background.insert(
                            generation,
                            PendingBackgroundTask {
                                request,
                                receiver: BackgroundReceiver::Diff { cache_key, rx },
                            },
                        );
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        self.tasks.task_manager.submit_result(TaskResult {
                            key: request.key.clone(),
                            generation: request.generation,
                            kind: TaskResultKind::Failed {
                                reason: "diff load disconnected".to_string(),
                            },
                        });
                    }
                },
            }
        }
        self.tasks.pending_background_tasks = next_pending_background;

        for task in self.tasks.task_manager.collect_ready() {
            match task.kind {
                TaskResultKind::Finished => {
                    let Some(payload) = finished_payloads.remove(&(task.key, task.generation))
                    else {
                        continue;
                    };
                    match payload {
                        BackgroundPayload::Status { status, fast } => {
                            debug!(
                                event = "status_load_finished",
                                mode = if fast { "fast" } else { "full" },
                                staged = status.staged.len(),
                                unstaged = status.unstaged.len(),
                                untracked = status.untracked.len(),
                                "status load finished"
                            );
                            self.apply_status_refresh(status);
                            if fast && self.refresh.pending_full_status_after_fast {
                                if self.start_status_load(false) {
                                    self.refresh.pending_full_status_after_fast = false;
                                } else {
                                    self.refresh.pending_refresh =
                                        Some(match self.refresh.pending_refresh {
                                            None => RefreshKind::StatusOnly,
                                            Some(existing) => Self::max_refresh_kind(
                                                existing,
                                                RefreshKind::StatusOnly,
                                            ),
                                        });
                                }
                            }
                            if self.should_schedule_diff_for_refresh(DiffRefreshSource::Status) {
                                schedule_diff = true;
                            }
                            self.ui.dirty.mark_all();
                        }
                        BackgroundPayload::Branches(items) => {
                            let view = BranchBackend::refresh_branches(
                                &items,
                                self.current_branches_view_state(),
                            );
                            self.apply_branches_backend_view(view);
                            if self.should_schedule_diff_for_refresh(DiffRefreshSource::Branches) {
                                schedule_diff = true;
                            }
                            self.ui.dirty.mark_all();
                        }
                        BackgroundPayload::Stashes(items) => {
                            self.ui.stash.items = items;
                            if self.should_schedule_diff_for_refresh(DiffRefreshSource::Stashes) {
                                schedule_diff = true;
                            }
                            self.ui.dirty.mark_all();
                        }
                        BackgroundPayload::Commits(items) => {
                            debug!(
                                event = "commits_load_finished",
                                count = items.len(),
                                requested_limit = self.tasks.commits_requested_limit,
                                "commits load finished"
                            );
                            self.ui.commits.items = items;
                            self.ui.commits.dirty = false;
                            self.ui.dirty.mark_all();
                            if self.should_schedule_diff_for_refresh(DiffRefreshSource::Commits) {
                                schedule_diff = true;
                            }
                        }
                        BackgroundPayload::CommitsFast(items) => {
                            debug!(
                                event = "commits_load_finished",
                                mode = "fast",
                                count = items.len(),
                                requested_limit = self.tasks.commits_requested_limit,
                                "commits fast load finished"
                            );
                            self.ui.commits.items = items;
                            self.ui.commits.dirty = true;
                            self.ui.dirty.mark_all();
                            if self.should_schedule_diff_for_refresh(DiffRefreshSource::Commits) {
                                schedule_diff = true;
                            }
                        }
                        BackgroundPayload::BranchCommits { branch, items } => {
                            if self.ui.branches.commits_subview_source.as_deref()
                                == Some(branch.as_str())
                                && self.ui.branches.commits_subview_active
                            {
                                let view = BranchBackend::apply_commits_subview_loaded(
                                    self.current_branches_view_state(),
                                    &branch,
                                    items,
                                );
                                self.apply_branches_backend_view(view);
                                self.ui.dirty.mark_all();
                                if self.should_schedule_diff_for_refresh(DiffRefreshSource::Commits)
                                {
                                    schedule_diff = true;
                                }
                            }
                        }
                        BackgroundPayload::Diff { cache_key, diff } => {
                            self.diff_mgr.in_flight_diff_key = None;
                            self.diff_mgr.cache.insert(cache_key.clone(), diff.clone());
                            self.diff_mgr.last_diff_key = Some(cache_key);
                            if self.ui.active_panel != SidePanel::LocalBranches {
                                self.git.current_diff = diff;
                                if !self.refresh.pending_diff_reload {
                                    self.refresh.pending_diff_reload_at = None;
                                }
                                self.ui.dirty.mark_diff();
                            }
                        }
                    }
                }
                TaskResultKind::Failed { reason } => {
                    if matches!(task.key, TaskKey::Status) {
                        self.refresh.pending_refresh_fast_done = false;
                    }
                    if let TaskKey::BranchCommits { branch } = &task.key {
                        let view = BranchBackend::fail_commits_subview_load(
                            self.current_branches_view_state(),
                            branch,
                        );
                        self.apply_branches_backend_view(view);
                    }
                    if matches!(task.key, TaskKey::Diff { .. }) {
                        self.diff_mgr.in_flight_diff_key = None;
                    }
                    if matches!(task.key, TaskKey::Diff { .. }) && !self.refresh.pending_diff_reload
                    {
                        self.refresh.pending_diff_reload_at = None;
                    }
                    self.push_log(reason, false);
                }
            }
        }

        if schedule_diff {
            self.schedule_diff_reload();
        }
    }
}
