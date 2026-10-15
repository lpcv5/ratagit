//! Single composition entry for the Git2 backend.
//! Capability modules own their implementations and retain the existing worker routes.

mod commit;
mod diagnostics;
mod history;
mod operations;
mod patch;
mod references;
mod shared;
mod stash;
mod worker;
mod worktree;

use crate::git::GitError;
pub use diagnostics::enable_git_job_log;
use diagnostics::write_git_job_log;
use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use worker::{external_worker_pool_size, RepoWorker, RepoWorkerExternal};

/// Worker-local repository and history cache; never shared between worker threads.
struct Git2RepoInner {
    repo: git2::Repository,
    reachability_cache: RefCell<history::ReachabilityCache>,
}

/// One backend handle composing all capabilities over the persistent worker lanes.
pub struct Git2Repository {
    worker: Arc<RepoWorker>,
    external_workers: Arc<Vec<RepoWorkerExternal>>,
    external_next: AtomicUsize,
    repo_root: PathBuf,
}

impl Git2Repository {
    /// Open (or discover) a repository and start the persistent worker thread.
    pub fn discover() -> Result<Self, GitError> {
        let repo = git2::Repository::discover(".")?;
        let repo_root = if let Some(workdir) = repo.workdir() {
            workdir.to_path_buf()
        } else {
            repo.path()
                .parent()
                .map(|p| p.to_path_buf())
                .ok_or(GitError::InvalidState)?
        };
        drop(repo); // worker thread will re-open
        let worker = Arc::new(RepoWorker::spawn(repo_root.clone())?);
        let worker_count = external_worker_pool_size();
        let mut external_workers = Vec::with_capacity(worker_count);
        for id in 0..worker_count {
            external_workers.push(RepoWorkerExternal::spawn(repo_root.clone(), id)?);
        }
        write_git_job_log(&format!("external_worker_pool size={}", worker_count));
        Ok(Self {
            worker,
            external_workers: Arc::new(external_workers),
            external_next: AtomicUsize::new(0),
            repo_root,
        })
    }

    /// Open a repository at an explicit path and start the persistent worker thread.
    #[cfg(test)]
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self, GitError> {
        let repo_root = path.as_ref().to_path_buf();
        let worker = Arc::new(RepoWorker::spawn(repo_root.clone())?);
        let worker_count = external_worker_pool_size();
        let mut external_workers = Vec::with_capacity(worker_count);
        for id in 0..worker_count {
            external_workers.push(RepoWorkerExternal::spawn(repo_root.clone(), id)?);
        }
        write_git_job_log(&format!("external_worker_pool size={}", worker_count));
        Ok(Self {
            worker,
            external_workers: Arc::new(external_workers),
            external_next: AtomicUsize::new(0),
            repo_root,
        })
    }

    /// Send a job to the persistent worker and return a receiver for the result.
    fn spawn_repo_job<T, F>(
        &self,
        label: &'static str,
        job: F,
    ) -> Result<Receiver<Result<T, GitError>>, GitError>
    where
        T: Send + 'static,
        F: FnOnce(&mut Git2RepoInner) -> Result<T, GitError> + Send + 'static,
    {
        self.worker.run(label, job)
    }

    /// Send a job to the external git worker and return a receiver for the result.
    fn spawn_repo_job_external<T, F>(
        &self,
        label: &'static str,
        job: F,
    ) -> Result<Receiver<Result<T, GitError>>, GitError>
    where
        T: Send + 'static,
        F: FnOnce(&mut Git2RepoInner) -> Result<T, GitError> + Send + 'static,
    {
        let idx = self.external_next.fetch_add(1, Ordering::Relaxed) % self.external_workers.len();
        self.external_workers[idx].run(label, job)
    }

    #[allow(dead_code)]
    fn repo_root(&self) -> &PathBuf {
        &self.repo_root
    }
}

#[cfg(test)]
mod tests;
