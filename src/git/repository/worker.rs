//! Persistent worker lanes, bounded queues and pool sizing.

use super::{diagnostics::write_git_job_log, history::ReachabilityCache, Git2RepoInner};
use crate::git::GitError;
use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::thread;

/// Job closure sent to the repo worker thread.
type RepoJob = Box<dyn FnOnce(&mut Git2RepoInner) + Send + 'static>;
/// Job closure sent to the external git worker thread.
type ExternalRepoJob = Box<dyn FnOnce(&mut Git2RepoInner) + Send + 'static>;

/// Persistent background worker that owns a single `git2::Repository` instance.
/// Primary-lane operations are serialized through this worker,
/// so each worker opens its repository exactly once.
pub(super) struct RepoWorker {
    tx: mpsc::SyncSender<RepoJob>,
}

/// Persistent independent repository worker for external-lane reads and fetch.
pub(super) struct RepoWorkerExternal {
    id: usize,
    tx: mpsc::SyncSender<ExternalRepoJob>,
}

pub(super) fn external_worker_pool_size() -> usize {
    const DEFAULT_MIN: usize = 2;
    const DEFAULT_MAX: usize = 6;
    let default = std::thread::available_parallelism()
        .map(|n| (n.get() / 2).clamp(DEFAULT_MIN, DEFAULT_MAX))
        .unwrap_or(DEFAULT_MIN);
    std::env::var("RATAGIT_EXTERNAL_WORKERS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .map(|v| v.clamp(1, 12))
        .unwrap_or(default)
}

impl RepoWorker {
    /// Spawn the worker thread and return a handle.
    pub(super) fn spawn(repo_root: PathBuf) -> Result<Self, GitError> {
        let (tx, rx) = mpsc::sync_channel::<RepoJob>(128);
        thread::Builder::new()
            .name("ratagit-repo-worker".to_string())
            .spawn(move || {
                let t0 = std::time::Instant::now();
                let mut inner = match git2::Repository::open(&repo_root) {
                    Ok(repo) => Git2RepoInner {
                        repo,
                        reachability_cache: RefCell::new(ReachabilityCache::default()),
                    },
                    Err(err) => {
                        write_git_job_log(&format!("repo_open_failed err={}", err));
                        return;
                    }
                };
                write_git_job_log(&format!(
                    "repo_opened worker=primary ms={} path={}",
                    t0.elapsed().as_millis(),
                    repo_root.display()
                ));
                while let Ok(job) = rx.recv() {
                    job(&mut inner);
                }
                write_git_job_log("repo_worker_exited");
            })
            .map_err(|e| GitError::Git2(format!("failed to spawn repo worker: {}", e)))?;
        Ok(Self { tx })
    }

    /// Send a job and return a receiver for its result.
    pub(super) fn run<T, F>(
        &self,
        label: &'static str,
        job: F,
    ) -> Result<Receiver<Result<T, GitError>>, GitError>
    where
        T: Send + 'static,
        F: FnOnce(&mut Git2RepoInner) -> Result<T, GitError> + Send + 'static,
    {
        let (result_tx, result_rx) = mpsc::channel();
        let enqueue_at = std::time::Instant::now();
        let boxed: RepoJob = Box::new(move |inner| {
            let queue_ms = enqueue_at.elapsed().as_millis();
            let exec_start = std::time::Instant::now();
            let result = job(inner);
            let exec_ms = exec_start.elapsed().as_millis();
            write_git_job_log(&format!(
                "git_job worker=primary label={} queue_ms={} exec_ms={} ok={}",
                label,
                queue_ms,
                exec_ms,
                result.is_ok()
            ));
            let _ = result_tx.send(result);
        });
        self.tx
            .send(boxed)
            .map_err(|_| GitError::Git2("repo worker channel closed".to_string()))?;
        Ok(result_rx)
    }
}

impl RepoWorkerExternal {
    pub(super) fn spawn(repo_root: PathBuf, id: usize) -> Result<Self, GitError> {
        let (tx, rx) = mpsc::sync_channel::<ExternalRepoJob>(128);
        thread::Builder::new()
            .name(format!("ratagit-external-worker-{}", id))
            .spawn(move || {
                let t0 = std::time::Instant::now();
                let mut inner = match git2::Repository::open(&repo_root) {
                    Ok(repo) => Git2RepoInner {
                        repo,
                        reachability_cache: RefCell::new(ReachabilityCache::default()),
                    },
                    Err(err) => {
                        write_git_job_log(&format!("external_repo_open_failed err={}", err));
                        return;
                    }
                };
                write_git_job_log(&format!(
                    "external_repo_opened worker=external-{} ms={} path={}",
                    id,
                    t0.elapsed().as_millis(),
                    repo_root.display()
                ));
                while let Ok(job) = rx.recv() {
                    job(&mut inner);
                }
                write_git_job_log(&format!(
                    "external_repo_worker_exited worker=external-{}",
                    id
                ));
            })
            .map_err(|e| GitError::Git2(format!("failed to spawn external worker: {}", e)))?;
        Ok(Self { id, tx })
    }

    pub(super) fn run<T, F>(
        &self,
        label: &'static str,
        job: F,
    ) -> Result<Receiver<Result<T, GitError>>, GitError>
    where
        T: Send + 'static,
        F: FnOnce(&mut Git2RepoInner) -> Result<T, GitError> + Send + 'static,
    {
        let worker_id = self.id;
        let (result_tx, result_rx) = mpsc::channel();
        let enqueue_at = std::time::Instant::now();
        let boxed: ExternalRepoJob = Box::new(move |inner| {
            let queue_ms = enqueue_at.elapsed().as_millis();
            let exec_start = std::time::Instant::now();
            let result = job(inner);
            let exec_ms = exec_start.elapsed().as_millis();
            write_git_job_log(&format!(
                "git_job worker=external-{} label={} queue_ms={} exec_ms={} ok={}",
                worker_id,
                label,
                queue_ms,
                exec_ms,
                result.is_ok()
            ));
            let _ = result_tx.send(result);
        });
        self.tx
            .send(boxed)
            .map_err(|_| GitError::Git2("external worker channel closed".to_string()))?;
        Ok(result_rx)
    }
}
