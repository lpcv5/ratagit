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
        let (startup_tx, startup_rx) = mpsc::channel();
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
                        let _ =
                            startup_tx.send(Err(GitError::from(err).context("open_repository")));
                        return;
                    }
                };
                write_git_job_log(&format!(
                    "repo_opened worker=primary ms={} path={}",
                    t0.elapsed().as_millis(),
                    repo_root.display()
                ));
                let _ = startup_tx.send(Ok(()));
                while let Ok(job) = rx.recv() {
                    job(&mut inner);
                }
                write_git_job_log("repo_worker_exited");
            })
            .map_err(|e| GitError::Worker(format!("failed to spawn repo worker: {}", e)))?;
        startup_rx
            .recv()
            .map_err(|error| GitError::Worker(error.to_string()))??;
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
        let trace = crate::latency::Trace::current();
        if let Some(trace) = &trace {
            trace.worker_event(
                "git_queued",
                std::time::Duration::ZERO,
                Some(label),
                Some(0),
            );
        }
        let boxed: RepoJob = Box::new(move |inner| {
            if let Some(trace) = &trace {
                trace.worker_event("git_started", enqueue_at.elapsed(), Some(label), Some(0));
            }
            let queue_ms = enqueue_at.elapsed().as_millis();
            let exec_start = std::time::Instant::now();
            let result = job(inner).map_err(|error| error.context(label));
            let exec_ms = exec_start.elapsed().as_millis();
            if let Some(trace) = &trace {
                trace.worker_event("git_finished", exec_start.elapsed(), Some(label), Some(0));
            }
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
            .map_err(|_| GitError::Worker("repo worker channel closed".to_string()))?;
        Ok(result_rx)
    }
}

impl RepoWorkerExternal {
    pub(super) fn spawn(repo_root: PathBuf, id: usize) -> Result<Self, GitError> {
        let (tx, rx) = mpsc::sync_channel::<ExternalRepoJob>(128);
        let (startup_tx, startup_rx) = mpsc::channel();
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
                        let _ =
                            startup_tx.send(Err(GitError::from(err).context("open_repository")));
                        return;
                    }
                };
                write_git_job_log(&format!(
                    "external_repo_opened worker=external-{} ms={} path={}",
                    id,
                    t0.elapsed().as_millis(),
                    repo_root.display()
                ));
                let _ = startup_tx.send(Ok(()));
                while let Ok(job) = rx.recv() {
                    job(&mut inner);
                }
                write_git_job_log(&format!(
                    "external_repo_worker_exited worker=external-{}",
                    id
                ));
            })
            .map_err(|e| GitError::Worker(format!("failed to spawn external worker: {}", e)))?;
        startup_rx
            .recv()
            .map_err(|error| GitError::Worker(error.to_string()))??;
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
        let trace = crate::latency::Trace::current();
        if let Some(trace) = &trace {
            trace.worker_event(
                "git_queued",
                std::time::Duration::ZERO,
                Some(label),
                Some(worker_id + 1),
            );
        }
        let boxed: ExternalRepoJob = Box::new(move |inner| {
            if let Some(trace) = &trace {
                trace.worker_event(
                    "git_started",
                    enqueue_at.elapsed(),
                    Some(label),
                    Some(worker_id + 1),
                );
            }
            let queue_ms = enqueue_at.elapsed().as_millis();
            let exec_start = std::time::Instant::now();
            let result = job(inner).map_err(|error| error.context(label));
            let exec_ms = exec_start.elapsed().as_millis();
            if let Some(trace) = &trace {
                trace.worker_event(
                    "git_finished",
                    exec_start.elapsed(),
                    Some(label),
                    Some(worker_id + 1),
                );
            }
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
            .map_err(|_| GitError::Worker("external worker channel closed".to_string()))?;
        Ok(result_rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::GitErrorCategory;
    use std::time::Duration;

    #[test]
    fn test_both_native_worker_lanes_preserve_structured_failures() {
        let directory = tempfile::tempdir().unwrap();
        git2::Repository::init(directory.path()).unwrap();
        let primary = RepoWorker::spawn(directory.path().into()).unwrap();
        let external = RepoWorkerExternal::spawn(directory.path().into(), 0).unwrap();
        for code in [
            git2::ErrorCode::Auth,
            git2::ErrorCode::Conflict,
            git2::ErrorCode::Locked,
        ] {
            let error = GitError::from(git2::Error::new(
                code,
                git2::ErrorClass::Repository,
                "same message",
            ));
            let for_primary = error.clone();
            let for_external = error.clone();
            let receivers = [
                primary
                    .run("test_error", move |_| Err::<(), _>(for_primary))
                    .unwrap(),
                external
                    .run("test_error", move |_| Err::<(), _>(for_external))
                    .unwrap(),
            ];
            for receiver in receivers {
                assert_eq!(
                    receiver.recv_timeout(Duration::from_secs(10)).unwrap(),
                    Err(error.clone().context("test_error"))
                );
            }
        }
    }

    #[test]
    fn test_native_worker_startup_returns_original_open_failure() {
        let directory = tempfile::tempdir().unwrap();
        let errors = [
            RepoWorker::spawn(directory.path().into()).err().unwrap(),
            RepoWorkerExternal::spawn(directory.path().into(), 0)
                .err()
                .unwrap(),
        ];
        for error in errors {
            assert_eq!(error.category(), GitErrorCategory::NotFound);
            assert!(matches!(
                error,
                GitError::Context {
                    operation: "open_repository",
                    ..
                }
            ));
        }
    }
}
