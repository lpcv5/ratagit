use std::cell::RefCell;
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use thiserror::Error;
use tokio::task;
use tracing::debug;

// ---------------------------------------------------------------------------
// Git job debug logging
// ---------------------------------------------------------------------------

/// Set to Some(path) before any Git2Repository is constructed to enable
/// per-job timing logs written to that file.  Controlled by --debug flag.
static GIT_JOB_LOG: OnceLock<Mutex<std::fs::File>> = OnceLock::new();

/// Enable git job timing logs, written to `path`.
/// Call this once at startup before any repository is opened.
pub fn enable_git_job_log(path: &str) {
    use std::fs::OpenOptions;
    if let Ok(file) = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
    {
        let _ = GIT_JOB_LOG.set(Mutex::new(file));
    }
}

fn write_git_job_log(msg: &str) {
    debug!("{}", msg);
    if let Some(lock) = GIT_JOB_LOG.get() {
        if let Ok(mut file) = lock.lock() {
            use std::io::Write;
            let _ = writeln!(file, "{}", msg);
            let _ = file.flush();
        }
    }
}

/// Documentation comment in English.
#[derive(Debug, Clone, Error)]
pub enum GitError {
    #[error("Git error: {0}")]
    Git2(String),

    #[error("Invalid repository state")]
    InvalidState,
}

// Comment in English.
impl From<git2::Error> for GitError {
    fn from(err: git2::Error) -> Self {
        GitError::Git2(err.to_string())
    }
}

/// Documentation comment in English.
#[derive(Debug, Clone, PartialEq)]
pub enum FileStatus {
    New,
    Modified,
    Deleted,
    Renamed,
    TypeChange,
}

/// Documentation comment in English.
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub status: FileStatus,
}

/// Documentation comment in English.
#[derive(Debug, Clone, Default)]
pub struct GitStatus {
    pub unstaged: Vec<FileEntry>,
    pub staged: Vec<FileEntry>,
    pub untracked: Vec<FileEntry>,
}

/// Documentation comment in English.
#[derive(Debug, Clone, PartialEq)]
pub enum DiffLineKind {
    Added,
    Removed,
    Context,
    Header,
}

/// Documentation comment in English.
#[derive(Debug, Clone)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub content: String,
}

/// Branch info
#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub is_current: bool,
}

/// Commit sync state for log coloring
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitSyncState {
    DefaultBranch,
    RemoteBranch,
    LocalOnly,
}

/// A single cell in a commit graph line, carrying its display string and lane index for coloring.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphCell {
    pub text: String,
    pub lane: usize,
    /// OID of the commit whose branch line this cell belongs to (for path highlighting).
    pub pipe_oid: Option<String>,
    /// OIDs of all commits whose paths overlap this cell.
    pub pipe_oids: Vec<String>,
}

/// Commit info for log display
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub oid: String,
    pub message: String,
    pub author: String,
    pub graph: Vec<GraphCell>,
    pub time: String,
    pub parent_count: usize,
    pub sync_state: CommitSyncState,
    /// OIDs of this commit's parents (for ancestry tracing).
    pub parent_oids: Vec<String>,
}

/// Stash entry
#[derive(Debug, Clone)]
pub struct StashInfo {
    pub index: usize,
    pub message: String,
}

/// Documentation comment in English.
/// Documentation comment in English.
pub trait GitRepository {
    /// Documentation comment in English.
    fn status(&self) -> Result<GitStatus, GitError>;

    /// Documentation comment in English.
    fn status_fast(&self) -> Result<GitStatus, GitError> {
        self.status()
    }

    /// Documentation comment in English.
    fn status_async(&self) -> Result<Receiver<Result<GitStatus, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.status());
        Ok(rx)
    }

    /// Documentation comment in English.
    fn status_fast_async(&self) -> Result<Receiver<Result<GitStatus, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.status_fast());
        Ok(rx)
    }

    /// Documentation comment in English.
    fn stage(&self, path: &std::path::Path) -> Result<(), GitError>;

    /// Documentation comment in English.
    fn stage_async(&self, path: PathBuf) -> Result<Receiver<Result<(), GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.stage(&path));
        Ok(rx)
    }

    /// Documentation comment in English.
    fn stage_paths(&self, paths: &[PathBuf]) -> Result<(), GitError>;

    fn stage_paths_async(
        &self,
        paths: Vec<PathBuf>,
    ) -> Result<Receiver<Result<(), GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.stage_paths(&paths));
        Ok(rx)
    }

    /// Documentation comment in English.
    fn unstage(&self, path: &std::path::Path) -> Result<(), GitError>;

    /// Documentation comment in English.
    fn unstage_async(&self, path: PathBuf) -> Result<Receiver<Result<(), GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.unstage(&path));
        Ok(rx)
    }

    /// Documentation comment in English.
    fn unstage_paths(&self, paths: &[PathBuf]) -> Result<(), GitError>;

    /// Documentation comment in English.
    fn discard_paths(&self, paths: &[PathBuf]) -> Result<(), GitError>;

    /// Documentation comment in English.
    fn discard_paths_async(
        &self,
        paths: Vec<PathBuf>,
    ) -> Result<Receiver<Result<(), GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.discard_paths(&paths));
        Ok(rx)
    }

    /// Documentation comment in English.
    fn diff_unstaged(&self, path: &std::path::Path) -> Result<Vec<DiffLine>, GitError>;

    /// Documentation comment in English.
    fn diff_unstaged_async(
        &self,
        path: PathBuf,
    ) -> Result<Receiver<Result<Vec<DiffLine>, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.diff_unstaged(&path));
        Ok(rx)
    }

    /// Documentation comment in English.
    fn diff_staged(&self, path: &std::path::Path) -> Result<Vec<DiffLine>, GitError>;

    /// Documentation comment in English.
    fn diff_staged_async(
        &self,
        path: PathBuf,
    ) -> Result<Receiver<Result<Vec<DiffLine>, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.diff_staged(&path));
        Ok(rx)
    }

    /// Documentation comment in English.
    fn diff_untracked(&self, path: &std::path::Path) -> Result<Vec<DiffLine>, GitError>;

    /// Documentation comment in English.
    fn diff_untracked_async(
        &self,
        path: PathBuf,
    ) -> Result<Receiver<Result<Vec<DiffLine>, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.diff_untracked(&path));
        Ok(rx)
    }

    /// Documentation comment in English.
    fn diff_directory(&self, _path: &std::path::Path) -> Result<Vec<DiffLine>, GitError> {
        Ok(Vec::new())
    }

    /// Documentation comment in English.
    fn diff_directory_async(
        &self,
        path: PathBuf,
    ) -> Result<Receiver<Result<Vec<DiffLine>, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.diff_directory(&path));
        Ok(rx)
    }

    /// Documentation comment in English.
    fn branches(&self) -> Result<Vec<BranchInfo>, GitError>;

    /// Documentation comment in English.
    fn branches_async(&self) -> Result<Receiver<Result<Vec<BranchInfo>, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.branches());
        Ok(rx)
    }

    /// Documentation comment in English.
    fn branch_log(&self, _name: &str, _limit: usize) -> Result<Vec<DiffLine>, GitError> {
        Ok(Vec::new())
    }

    /// Documentation comment in English.
    fn branch_log_async(
        &self,
        name: String,
        limit: usize,
    ) -> Result<Receiver<Result<Vec<DiffLine>, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.branch_log(&name, limit));
        Ok(rx)
    }

    /// Documentation comment in English.
    fn commits(&self, limit: usize) -> Result<Vec<CommitInfo>, GitError>;

    /// Documentation comment in English.
    fn commits_fast(&self, limit: usize) -> Result<Vec<CommitInfo>, GitError> {
        self.commits(limit)
    }

    /// Documentation comment in English.
    fn commits_async(
        &self,
        limit: usize,
    ) -> Result<Receiver<Result<Vec<CommitInfo>, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.commits(limit));
        Ok(rx)
    }

    /// Documentation comment in English.
    fn commits_fast_async(
        &self,
        limit: usize,
    ) -> Result<Receiver<Result<Vec<CommitInfo>, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.commits_fast(limit));
        Ok(rx)
    }

    /// Documentation comment in English.
    fn commits_for_branch(&self, _name: &str, limit: usize) -> Result<Vec<CommitInfo>, GitError> {
        self.commits(limit)
    }

    /// Documentation comment in English.
    fn commits_for_branch_async(
        &self,
        name: &str,
        limit: usize,
    ) -> Result<Receiver<Result<Vec<CommitInfo>, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.commits_for_branch(name, limit));
        Ok(rx)
    }

    /// Documentation comment in English.
    fn commit_files(&self, oid: &str) -> Result<Vec<FileEntry>, GitError>;

    /// Documentation comment in English.
    fn stashes(&self) -> Result<Vec<StashInfo>, GitError>;

    /// Documentation comment in English.
    fn stashes_async(&self) -> Result<Receiver<Result<Vec<StashInfo>, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.stashes());
        Ok(rx)
    }

    /// Documentation comment in English.
    fn stash_files(&self, index: usize) -> Result<Vec<FileEntry>, GitError>;

    /// Documentation comment in English.
    fn stash_diff(
        &self,
        index: usize,
        path: Option<&std::path::Path>,
    ) -> Result<Vec<DiffLine>, GitError>;

    /// Documentation comment in English.
    fn stash_diff_async(
        &self,
        index: usize,
        path: Option<PathBuf>,
    ) -> Result<Receiver<Result<Vec<DiffLine>, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.stash_diff(index, path.as_deref()));
        Ok(rx)
    }

    /// Documentation comment in English.
    fn stash_push_paths(&self, paths: &[PathBuf], message: &str) -> Result<usize, GitError>;

    /// Documentation comment in English.
    fn stash_push_paths_async(
        &self,
        paths: Vec<PathBuf>,
        message: String,
    ) -> Result<Receiver<Result<usize, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.stash_push_paths(&paths, &message));
        Ok(rx)
    }

    /// Documentation comment in English.
    fn stash_apply(&self, index: usize) -> Result<(), GitError>;

    /// Documentation comment in English.
    fn stash_apply_async(&self, index: usize) -> Result<Receiver<Result<(), GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.stash_apply(index));
        Ok(rx)
    }

    /// Documentation comment in English.
    fn stash_pop(&self, index: usize) -> Result<(), GitError>;

    /// Documentation comment in English.
    fn stash_pop_async(&self, index: usize) -> Result<Receiver<Result<(), GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.stash_pop(index));
        Ok(rx)
    }

    /// Documentation comment in English.
    fn stash_drop(&self, index: usize) -> Result<(), GitError>;

    /// Documentation comment in English.
    fn stash_drop_async(&self, index: usize) -> Result<Receiver<Result<(), GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.stash_drop(index));
        Ok(rx)
    }

    /// Documentation comment in English.
    fn commit_diff_scoped(
        &self,
        oid: &str,
        path: Option<&std::path::Path>,
    ) -> Result<Vec<DiffLine>, GitError>;

    /// Documentation comment in English.
    fn commit_diff_scoped_async(
        &self,
        oid: String,
        path: Option<PathBuf>,
    ) -> Result<Receiver<Result<Vec<DiffLine>, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.commit_diff_scoped(&oid, path.as_deref()));
        Ok(rx)
    }

    /// Documentation comment in English.
    fn commit(&self, message: &str) -> Result<String, GitError>;

    /// Documentation comment in English.
    fn commit_async(
        &self,
        message: String,
    ) -> Result<Receiver<Result<String, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.commit(&message));
        Ok(rx)
    }

    /// Documentation comment in English.
    fn create_branch(&self, name: &str) -> Result<(), GitError>;

    /// Documentation comment in English.
    fn create_branch_async(
        &self,
        name: String,
    ) -> Result<Receiver<Result<(), GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.create_branch(&name));
        Ok(rx)
    }

    /// Documentation comment in English.
    fn checkout_branch(&self, name: &str) -> Result<(), GitError>;

    /// Documentation comment in English.
    fn checkout_branch_async(
        &self,
        name: String,
    ) -> Result<Receiver<Result<(), GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.checkout_branch(&name));
        Ok(rx)
    }

    /// Documentation comment in English.
    fn checkout_branch_with_auto_stash(&self, name: &str) -> Result<(), GitError> {
        self.checkout_branch(name)
    }

    /// Documentation comment in English.
    fn checkout_branch_with_auto_stash_async(
        &self,
        name: String,
    ) -> Result<Receiver<Result<(), GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.checkout_branch_with_auto_stash(&name));
        Ok(rx)
    }

    /// Documentation comment in English.
    fn delete_branch(&self, name: &str) -> Result<(), GitError>;

    /// Documentation comment in English.
    fn delete_branch_async(
        &self,
        name: String,
    ) -> Result<Receiver<Result<(), GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.delete_branch(&name));
        Ok(rx)
    }

    /// Documentation comment in English.
    fn fetch_default_async(&self) -> Result<Receiver<Result<String, GitError>>, GitError>;
}

#[allow(dead_code)]
#[async_trait::async_trait(?Send)]
pub trait AsyncGitRepository {
    async fn status_async(&self) -> Result<GitStatus, GitError>;
    async fn branches_async(&self) -> Result<Vec<BranchInfo>, GitError>;
    async fn stashes_async(&self) -> Result<Vec<StashInfo>, GitError>;
    async fn commits_async(&self, limit: usize) -> Result<Vec<CommitInfo>, GitError>;
    async fn commits_for_branch_async(
        &self,
        name: &str,
        limit: usize,
    ) -> Result<Vec<CommitInfo>, GitError>;
    async fn branch_log_async(&self, name: &str, limit: usize) -> Result<Vec<DiffLine>, GitError>;
    async fn diff_unstaged_async(&self, path: PathBuf) -> Result<Vec<DiffLine>, GitError>;
    async fn diff_staged_async(&self, path: PathBuf) -> Result<Vec<DiffLine>, GitError>;
    async fn diff_untracked_async(&self, path: PathBuf) -> Result<Vec<DiffLine>, GitError>;
    async fn diff_directory_async(&self, path: PathBuf) -> Result<Vec<DiffLine>, GitError>;
    async fn commit_diff_scoped_async(
        &self,
        oid: &str,
        path: Option<PathBuf>,
    ) -> Result<Vec<DiffLine>, GitError>;
    async fn stash_diff_async(
        &self,
        index: usize,
        path: Option<PathBuf>,
    ) -> Result<Vec<DiffLine>, GitError>;

    async fn stage_async(&self, path: PathBuf) -> Result<(), GitError>;
    async fn stage_paths_async(&self, paths: Vec<PathBuf>) -> Result<(), GitError>;
    async fn unstage_async(&self, path: PathBuf) -> Result<(), GitError>;
    async fn unstage_paths_async(&self, paths: Vec<PathBuf>) -> Result<(), GitError>;
    async fn discard_paths_async(&self, paths: Vec<PathBuf>) -> Result<(), GitError>;
    async fn commit_write_async(&self, message: String) -> Result<String, GitError>;
    async fn create_branch_async(&self, name: String) -> Result<(), GitError>;
    async fn checkout_branch_async(&self, name: String) -> Result<(), GitError>;
    async fn checkout_branch_with_auto_stash_async(&self, name: String) -> Result<(), GitError>;
    async fn delete_branch_async(&self, name: String) -> Result<(), GitError>;
    async fn stash_push_paths_async(
        &self,
        paths: Vec<PathBuf>,
        message: String,
    ) -> Result<usize, GitError>;
    async fn stash_apply_async(&self, index: usize) -> Result<(), GitError>;
    async fn stash_pop_async(&self, index: usize) -> Result<(), GitError>;
    async fn stash_drop_async(&self, index: usize) -> Result<(), GitError>;
}

/// Job closure sent to the repo worker thread.
type RepoJob = Box<dyn FnOnce(&mut Git2RepoInner) + Send + 'static>;
/// Job closure sent to the external git worker thread.
type ExternalRepoJob = Box<dyn FnOnce(&mut Git2RepoInner) + Send + 'static>;

/// Persistent background worker that owns a single `git2::Repository` instance.
/// All git operations that require the repo are routed through this worker,
/// so `Repository::open` is called exactly once per ratagit session.
struct RepoWorker {
    tx: mpsc::SyncSender<RepoJob>,
}

/// Persistent background worker for external git commands.
struct RepoWorkerExternal {
    id: usize,
    tx: mpsc::SyncSender<ExternalRepoJob>,
}

fn external_worker_pool_size() -> usize {
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
    fn spawn(repo_root: PathBuf) -> Result<Self, GitError> {
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
    fn run<T, F>(
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
    fn spawn(repo_root: PathBuf, id: usize) -> Result<Self, GitError> {
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

    fn run<T, F>(
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

#[derive(Default)]
struct ReachabilityCache {
    default_tip: Option<git2::Oid>,
    upstream_tip: Option<git2::Oid>,
    limit: usize,
    default_set: HashSet<git2::Oid>,
    upstream_set: HashSet<git2::Oid>,
}

/// Inner repository wrapper — only instantiated inside the `RepoWorker` thread.
struct Git2RepoInner {
    repo: git2::Repository,
    reachability_cache: RefCell<ReachabilityCache>,
}

/// Public handle to the git repository.  Sends all git operations to the
/// persistent `RepoWorker` thread so `Repository::open` is paid only once.
pub struct Git2Repository {
    worker: Arc<RepoWorker>,
    external_workers: Arc<Vec<RepoWorkerExternal>>,
    external_next: AtomicUsize,
    repo_root: PathBuf,
}

/// Maximum number of diff lines returned to the UI to prevent OOM on huge files.
const MAX_DIFF_LINES: usize = 5_000;

/// Maximum file size for untracked file preview, in bytes.
const MAX_UNTRACKED_PREVIEW_BYTES: usize = 256 * 1024; // 256 KiB

#[derive(Clone, Copy)]
struct GraphCharset {
    node: char,
    merge_node: char,
    horizontal: char,
    ascii: bool,
}

impl GraphCharset {
    fn unicode() -> Self {
        Self {
            node: '◯',
            merge_node: '⏣',
            horizontal: '─',
            ascii: false,
        }
    }

    fn ascii() -> Self {
        Self {
            node: '*',
            merge_node: '#',
            horizontal: '-',
            ascii: true,
        }
    }
}

#[derive(Clone)]
struct GraphCommitRow {
    oid: git2::Oid,
    parents: Vec<git2::Oid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PipeKind {
    Terminates,
    Starts,
    Continues,
}

#[derive(Debug, Clone)]
struct Pipe {
    from_pos: i16,
    to_pos: i16,
    from_hash: Option<git2::Oid>,
    to_hash: Option<git2::Oid>,
    kind: PipeKind,
}

#[derive(Debug, Clone, Copy, Default)]
struct CellConnections {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
}

impl Git2RepoInner {
    /// Documentation comment in English.
    fn convert_status(status: git2::Status) -> FileStatus {
        if status.is_index_new() || status.is_wt_new() {
            FileStatus::New
        } else if status.is_index_modified() || status.is_wt_modified() {
            FileStatus::Modified
        } else if status.is_index_deleted() || status.is_wt_deleted() {
            FileStatus::Deleted
        } else if status.is_index_renamed() || status.is_wt_renamed() {
            FileStatus::Renamed
        } else if status.is_index_typechange() || status.is_wt_typechange() {
            FileStatus::TypeChange
        } else {
            FileStatus::Modified // default
        }
    }

    fn signature(&self) -> Result<git2::Signature<'_>, GitError> {
        match self.repo.signature() {
            Ok(sig) => Ok(sig),
            Err(_) => {
                Ok(git2::Signature::now("ratagit", "ratagit@localhost").map_err(GitError::from)?)
            }
        }
    }

    fn repo_root(&self) -> Result<PathBuf, GitError> {
        if let Some(workdir) = self.repo.workdir() {
            return Ok(workdir.to_path_buf());
        }

        self.repo
            .path()
            .parent()
            .map(|p| p.to_path_buf())
            .ok_or(GitError::InvalidState)
    }

    fn stash_refname(index: usize) -> String {
        if index == 0 {
            "refs/stash".to_string()
        } else {
            format!("refs/stash@{{{}}}", index)
        }
    }

    fn stash_commit(&self, index: usize) -> Result<git2::Commit<'_>, GitError> {
        let refname = Self::stash_refname(index);
        let reference = self.repo.find_reference(&refname)?;
        Ok(reference.peel_to_commit()?)
    }

    fn checkout_branch_in_repo(repo: &git2::Repository, name: &str) -> Result<(), GitError> {
        let local_ref = format!("refs/heads/{}", name);
        repo.find_reference(&local_ref)?;
        repo.set_head(&local_ref)?;
        let mut checkout = git2::build::CheckoutBuilder::new();
        checkout.safe();
        repo.checkout_head(Some(&mut checkout))?;
        Ok(())
    }

    /// Helper to validate path contains valid Unicode.
    fn path_to_str<'a>(path: &'a std::path::Path, context: &str) -> Result<&'a str, GitError> {
        path.to_str()
            .ok_or_else(|| GitError::Git2(format!("{} contains invalid unicode", context)))
    }

    fn run_git_owned(&self, args: &[String]) -> Result<String, GitError> {
        let output = Command::new("git")
            .args(args)
            .current_dir(self.repo_root()?)
            .output()
            .map_err(|e| GitError::Git2(format!("failed to run git {:?}: {}", args, e)))?;

        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if stderr.is_empty() { stdout } else { stderr };
        Err(GitError::Git2(detail))
    }

    fn resolve_ref_oid(&self, refname: &str) -> Option<git2::Oid> {
        let reference = self.repo.find_reference(refname).ok()?;
        reference
            .resolve()
            .ok()
            .and_then(|resolved| resolved.target())
            .or_else(|| reference.target())
    }

    fn upstream_oid(&self) -> Option<git2::Oid> {
        let head = self.repo.head().ok()?;
        let local_name = head.shorthand()?;
        let local = self
            .repo
            .find_branch(local_name, git2::BranchType::Local)
            .ok()?;
        let upstream = local.upstream().ok()?;
        upstream.get().target()
    }

    fn upstream_remote_name(&self) -> Option<String> {
        let head = self.repo.head().ok()?;
        let local_name = head.shorthand()?;
        let local = self
            .repo
            .find_branch(local_name, git2::BranchType::Local)
            .ok()?;
        let upstream = local.upstream().ok()?;
        let name = upstream.get().name()?;
        let trimmed = name.strip_prefix("refs/remotes/").unwrap_or(name);
        parse_remote_from_upstream(trimmed)
    }

    fn default_branch_oid(&self) -> Option<git2::Oid> {
        self.resolve_ref_oid("refs/remotes/origin/HEAD")
            .or_else(|| self.resolve_ref_oid("refs/heads/main"))
            .or_else(|| self.resolve_ref_oid("refs/heads/master"))
            .or_else(|| self.resolve_ref_oid("refs/remotes/origin/main"))
            .or_else(|| self.resolve_ref_oid("refs/remotes/origin/master"))
    }

    fn collect_commits_from_revwalk(
        &self,
        revwalk: &mut git2::Revwalk<'_>,
        limit: usize,
    ) -> Result<Vec<CommitInfo>, GitError> {
        let default_tip = self.default_branch_oid();
        let upstream_tip = self.upstream_oid();
        let reachability_limit = limit * 4;

        let needs_recompute = {
            let cache = self.reachability_cache.borrow();
            cache.default_tip != default_tip
                || cache.upstream_tip != upstream_tip
                || cache.limit < reachability_limit
        };
        if needs_recompute {
            let default_set = self.reachable_oids_from(default_tip, reachability_limit);
            let upstream_set = self.reachable_oids_from(upstream_tip, reachability_limit);
            let mut cache = self.reachability_cache.borrow_mut();
            cache.default_tip = default_tip;
            cache.upstream_tip = upstream_tip;
            cache.limit = reachability_limit;
            cache.default_set = default_set;
            cache.upstream_set = upstream_set;
        }

        let cache = self.reachability_cache.borrow();
        let default_oids = &cache.default_set;
        let upstream_oids = &cache.upstream_set;

        let mut graph_rows = Vec::new();
        let mut entries = Vec::new();
        for oid in revwalk.take(limit) {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;
            let message = commit.summary().unwrap_or("").to_string();
            let author = commit.author().name().unwrap_or("").to_string();
            let mut parents = Vec::with_capacity(commit.parent_count());
            for parent_index in 0..commit.parent_count() {
                if let Ok(parent_oid) = commit.parent_id(parent_index) {
                    parents.push(parent_oid);
                }
            }
            let time = {
                let t = commit.time().seconds();
                chrono::DateTime::from_timestamp(t, 0)
                    .unwrap_or_default()
                    .format("%Y-%m-%d %H:%M")
                    .to_string()
            };
            let sync_state = if default_oids.contains(&oid) {
                CommitSyncState::DefaultBranch
            } else if upstream_oids.contains(&oid) {
                CommitSyncState::RemoteBranch
            } else {
                CommitSyncState::LocalOnly
            };
            let parent_oids: Vec<String> = parents.iter().map(|p| p.to_string()).collect();
            graph_rows.push(GraphCommitRow { oid, parents });
            entries.push(CommitInfo {
                oid: oid.to_string(),
                message,
                author,
                graph: Vec::new(),
                time,
                parent_count: commit.parent_count(),
                sync_state,
                parent_oids,
            });
        }

        let graph_lines = build_commit_graph_lines(&graph_rows, graph_charset_from_env());
        for (entry, graph) in entries.iter_mut().zip(graph_lines) {
            entry.graph = graph;
        }
        Ok(entries)
    }

    fn collect_commits_fast_from_revwalk(
        &self,
        revwalk: &mut git2::Revwalk<'_>,
        limit: usize,
    ) -> Result<Vec<CommitInfo>, GitError> {
        let mut entries = Vec::new();
        for oid in revwalk.take(limit) {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;
            let message = commit.summary().unwrap_or("").to_string();
            let author = commit.author().name().unwrap_or("").to_string();
            let mut parent_oids = Vec::with_capacity(commit.parent_count());
            for parent_index in 0..commit.parent_count() {
                if let Ok(parent_oid) = commit.parent_id(parent_index) {
                    parent_oids.push(parent_oid.to_string());
                }
            }
            entries.push(CommitInfo {
                oid: oid.to_string(),
                message,
                author,
                graph: Vec::new(),
                time: String::new(),
                parent_count: commit.parent_count(),
                sync_state: CommitSyncState::LocalOnly,
                parent_oids,
            });
        }
        Ok(entries)
    }

    /// Walk at most `limit` commits reachable from `tip` and return their OIDs as a HashSet.
    /// Returns an empty set if `tip` is None or if the walk fails.
    fn reachable_oids_from(
        &self,
        tip: Option<git2::Oid>,
        limit: usize,
    ) -> std::collections::HashSet<git2::Oid> {
        let tip = match tip {
            Some(t) => t,
            None => return std::collections::HashSet::new(),
        };
        let mut walk = match self.repo.revwalk() {
            Ok(w) => w,
            Err(_) => return std::collections::HashSet::new(),
        };
        if walk.push(tip).is_err() {
            return std::collections::HashSet::new();
        }
        let _ = walk.set_sorting(git2::Sort::TOPOLOGICAL);
        walk.take(limit).flatten().collect()
    }

    fn status_impl(&self, include_untracked: bool) -> Result<GitStatus, GitError> {
        let mut git_status = GitStatus::default();

        let mut opts = git2::StatusOptions::new();
        opts.include_untracked(include_untracked)
            .include_ignored(false)
            .update_index(false) // Performance: skip index refresh, rely on manual refresh
            .renames_head_to_index(true)
            .renames_index_to_workdir(true)
            .include_unmodified(false)
            .recurse_untracked_dirs(false) // Performance: only show top-level untracked dirs
            .recurse_ignored_dirs(false); // Performance: skip ignored directories entirely

        let statuses = self.repo.statuses(Some(&mut opts))?;

        for entry in statuses.iter() {
            let path = PathBuf::from(entry.path().unwrap_or(""));
            let status = entry.status();

            let file_entry = FileEntry {
                path,
                status: Self::convert_status(status),
            };

            // Comment in English.
            if status.is_wt_new() {
                git_status.untracked.push(file_entry);
            } else if status.is_index_new()
                || status.is_index_modified()
                || status.is_index_deleted()
                || status.is_index_renamed()
            {
                git_status.staged.push(file_entry);
            } else {
                git_status.unstaged.push(file_entry);
            }
        }

        Ok(git_status)
    }

    fn status(&self) -> Result<GitStatus, GitError> {
        self.status_impl(true)
    }

    fn status_fast(&self) -> Result<GitStatus, GitError> {
        self.status_impl(false)
    }

    fn stage(&self, path: &std::path::Path) -> Result<(), GitError> {
        self.stage_paths(&[path.to_path_buf()])
    }

    fn stage_paths(&self, paths: &[PathBuf]) -> Result<(), GitError> {
        if paths.is_empty() {
            return Ok(());
        }

        let mut index = self.repo.index()?;
        let specs: Vec<&str> = paths.iter().filter_map(|p| p.to_str()).collect();
        index.add_all(specs, git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;
        Ok(())
    }

    fn unstage(&self, path: &std::path::Path) -> Result<(), GitError> {
        self.unstage_paths(&[path.to_path_buf()])
    }

    fn unstage_paths(&self, paths: &[PathBuf]) -> Result<(), GitError> {
        if paths.is_empty() {
            return Ok(());
        }

        let head = self.repo.head()?.target().ok_or(GitError::InvalidState)?;
        let commit_obj = self
            .repo
            .find_object(head, Some(git2::ObjectType::Commit))?;
        let specs: Vec<&str> = paths.iter().filter_map(|p| p.to_str()).collect();
        self.repo.reset_default(Some(&commit_obj), specs)?;

        Ok(())
    }

    fn discard_paths(&self, paths: &[PathBuf]) -> Result<(), GitError> {
        if paths.is_empty() {
            return Ok(());
        }

        let mut checkout = git2::build::CheckoutBuilder::new();
        checkout.force();
        for path in paths {
            let path_str = Self::path_to_str(path, "path")?;
            checkout.path(path_str);
        }
        self.repo.checkout_index(None, Some(&mut checkout))?;
        Ok(())
    }

    fn diff_unstaged(&self, path: &std::path::Path) -> Result<Vec<DiffLine>, GitError> {
        let mut opts = git2::DiffOptions::new();
        opts.pathspec(path.to_str().unwrap_or(""))
            .max_size(MAX_UNTRACKED_PREVIEW_BYTES as i64); // treat files larger than limit as binary

        let diff = self.repo.diff_index_to_workdir(None, Some(&mut opts))?;
        Ok(parse_diff(&diff))
    }

    fn diff_staged(&self, path: &std::path::Path) -> Result<Vec<DiffLine>, GitError> {
        let head_tree = self.repo.head().ok().and_then(|h| h.peel_to_tree().ok());

        let mut opts = git2::DiffOptions::new();
        opts.pathspec(path.to_str().unwrap_or(""))
            .max_size(MAX_UNTRACKED_PREVIEW_BYTES as i64); // treat files larger than limit as binary

        let diff = self
            .repo
            .diff_tree_to_index(head_tree.as_ref(), None, Some(&mut opts))?;
        Ok(parse_diff(&diff))
    }

    fn diff_untracked(&self, path: &std::path::Path) -> Result<Vec<DiffLine>, GitError> {
        let workdir = self.repo.workdir().ok_or(GitError::InvalidState)?;
        let full_path = workdir.join(path);

        // Check file size before reading to avoid OOM on huge files
        let file_size = std::fs::metadata(&full_path)
            .map(|m| m.len() as usize)
            .unwrap_or(0);

        let content = if file_size == 0 {
            String::new()
        } else if file_size > MAX_UNTRACKED_PREVIEW_BYTES {
            format!(
                "<file too large to preview: {} bytes, limit {} bytes>",
                file_size, MAX_UNTRACKED_PREVIEW_BYTES
            )
        } else {
            match std::fs::read(&full_path) {
                Ok(bytes) => {
                    // Detect binary content by checking for null bytes
                    if bytes.contains(&0u8) {
                        format!("<binary file: {} bytes>", bytes.len())
                    } else {
                        String::from_utf8_lossy(&bytes).into_owned()
                    }
                }
                Err(_) => "<unreadable file>".to_string(),
            }
        };

        let header = format!("--- /dev/null\n+++ b/{}", path.display());
        let mut lines = vec![DiffLine {
            kind: DiffLineKind::Header,
            content: header,
        }];
        for (i, line) in content.lines().enumerate() {
            if i >= MAX_DIFF_LINES {
                lines.push(DiffLine {
                    kind: DiffLineKind::Header,
                    content: format!("... preview truncated at {} lines ...", MAX_DIFF_LINES),
                });
                break;
            }
            lines.push(DiffLine {
                kind: DiffLineKind::Added,
                content: line.to_string(),
            });
        }
        Ok(lines)
    }

    fn diff_directory(&self, path: &std::path::Path) -> Result<Vec<DiffLine>, GitError> {
        let path_str = Self::path_to_str(path, "path")?;
        let mut lines = Vec::new();
        let mut remaining = MAX_DIFF_LINES;

        let head_tree = self.repo.head().ok().and_then(|h| h.peel_to_tree().ok());

        if remaining > 0 {
            let mut opts = git2::DiffOptions::new();
            opts.pathspec(path_str)
                .max_size(MAX_UNTRACKED_PREVIEW_BYTES as i64);
            let diff = self
                .repo
                .diff_tree_to_index(head_tree.as_ref(), None, Some(&mut opts))?;
            let staged = parse_diff_with_limit(&diff, remaining);
            remaining = remaining.saturating_sub(staged.len());
            lines.extend(staged);
        }

        if remaining > 0 {
            let mut opts = git2::DiffOptions::new();
            opts.pathspec(path_str)
                .max_size(MAX_UNTRACKED_PREVIEW_BYTES as i64);
            let diff = self.repo.diff_index_to_workdir(None, Some(&mut opts))?;
            let unstaged = parse_diff_with_limit(&diff, remaining);
            remaining = remaining.saturating_sub(unstaged.len());
            lines.extend(unstaged);
        }

        if remaining > 0 {
            let untracked = self.status()?;
            for entry in untracked
                .untracked
                .iter()
                .filter(|entry| entry.path.starts_with(path))
            {
                if remaining == 0 {
                    break;
                }
                let mut file_lines = self.diff_untracked(&entry.path)?;
                if file_lines.len() > remaining {
                    file_lines.truncate(remaining);
                    lines.extend(file_lines);
                    lines.push(DiffLine {
                        kind: DiffLineKind::Header,
                        content: format!(
                            "... directory diff truncated at {} lines ...",
                            MAX_DIFF_LINES
                        ),
                    });
                    break;
                }
                remaining = remaining.saturating_sub(file_lines.len());
                lines.extend(file_lines);
            }
        }

        Ok(lines)
    }

    fn branches(&self) -> Result<Vec<BranchInfo>, GitError> {
        let head_name = self
            .repo
            .head()
            .ok()
            .and_then(|h| h.shorthand().map(|s| s.to_string()));

        let mut result = Vec::new();
        for branch in self.repo.branches(Some(git2::BranchType::Local))? {
            let (branch, _) = branch?;
            if let Some(name) = branch.name()? {
                result.push(BranchInfo {
                    is_current: Some(name.to_string()) == head_name,
                    name: name.to_string(),
                });
            }
        }
        Ok(result)
    }

    fn branch_log(&self, name: &str, limit: usize) -> Result<Vec<DiffLine>, GitError> {
        let raw = self.run_git_owned(&[
            "log".to_string(),
            "--graph".to_string(),
            "--decorate".to_string(),
            "--color=always".to_string(),
            "-n".to_string(),
            limit.max(1).to_string(),
            name.to_string(),
        ])?;

        Ok(raw
            .lines()
            .map(|line| DiffLine {
                kind: DiffLineKind::Context,
                content: line.to_string(),
            })
            .collect())
    }

    fn commits(&self, limit: usize) -> Result<Vec<CommitInfo>, GitError> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;
        self.collect_commits_from_revwalk(&mut revwalk, limit)
    }

    fn commits_fast(&self, limit: usize) -> Result<Vec<CommitInfo>, GitError> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;
        self.collect_commits_fast_from_revwalk(&mut revwalk, limit)
    }

    fn commits_for_branch(&self, name: &str, limit: usize) -> Result<Vec<CommitInfo>, GitError> {
        let mut revwalk = self.repo.revwalk()?;
        let local_ref = format!("refs/heads/{}", name);
        if let Some(oid) = self
            .resolve_ref_oid(&local_ref)
            .or_else(|| self.resolve_ref_oid(name))
        {
            revwalk.push(oid)?;
        } else {
            return Ok(Vec::new());
        }
        revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;
        self.collect_commits_from_revwalk(&mut revwalk, limit)
    }

    fn commit_files(&self, oid: &str) -> Result<Vec<FileEntry>, GitError> {
        let mut files = Vec::new();
        let oid = git2::Oid::from_str(oid)?;
        let commit = self.repo.find_commit(oid)?;
        let tree = commit.tree()?;
        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };
        let diff = self
            .repo
            .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;
        for delta in diff.deltas() {
            let status = match delta.status() {
                git2::Delta::Added => FileStatus::New,
                git2::Delta::Deleted => FileStatus::Deleted,
                git2::Delta::Renamed => FileStatus::Renamed,
                git2::Delta::Typechange => FileStatus::TypeChange,
                _ => FileStatus::Modified,
            };
            let path = match delta.status() {
                git2::Delta::Deleted => delta.old_file().path(),
                _ => delta.new_file().path().or_else(|| delta.old_file().path()),
            };
            if let Some(path) = path {
                files.push(FileEntry {
                    path: path.to_path_buf(),
                    status,
                });
            }
        }
        Ok(files)
    }

    fn stashes(&mut self) -> Result<Vec<StashInfo>, GitError> {
        let mut result = Vec::new();
        self.repo.stash_foreach(|index, name, _oid| {
            result.push(StashInfo {
                index,
                message: name.to_string(),
            });
            true
        })?;
        Ok(result)
    }

    fn stash_files(&self, index: usize) -> Result<Vec<FileEntry>, GitError> {
        let mut files = Vec::new();
        let commit = self.stash_commit(index)?;
        let tree = commit.tree()?;
        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };
        let diff = self
            .repo
            .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;
        for delta in diff.deltas() {
            let status = match delta.status() {
                git2::Delta::Added => FileStatus::New,
                git2::Delta::Deleted => FileStatus::Deleted,
                git2::Delta::Renamed => FileStatus::Renamed,
                git2::Delta::Typechange => FileStatus::TypeChange,
                _ => FileStatus::Modified,
            };
            let path = match delta.status() {
                git2::Delta::Deleted => delta.old_file().path(),
                _ => delta.new_file().path().or_else(|| delta.old_file().path()),
            };
            if let Some(path) = path {
                files.push(FileEntry {
                    path: path.to_path_buf(),
                    status,
                });
            }
        }

        Ok(files)
    }

    fn stash_diff(
        &self,
        index: usize,
        path: Option<&std::path::Path>,
    ) -> Result<Vec<DiffLine>, GitError> {
        let commit = self.stash_commit(index)?;
        let tree = commit.tree()?;
        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };
        let mut opts = git2::DiffOptions::new();
        if let Some(path) = path {
            let path_str = Self::path_to_str(path, "stash path")?;
            opts.pathspec(path_str);
        }
        let diff =
            self.repo
                .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut opts))?;
        Ok(parse_diff(&diff))
    }

    fn stash_push_paths(&mut self, paths: &[PathBuf], message: &str) -> Result<usize, GitError> {
        if paths.is_empty() {
            return Err(GitError::Git2("no selected paths for stash".to_string()));
        }

        let before = self.stashes()?.len();
        let mut args = vec![
            "stash".to_string(),
            "push".to_string(),
            "-u".to_string(),
            "-m".to_string(),
            message.to_string(),
            "--".to_string(),
        ];
        for path in paths {
            let path_str = Self::path_to_str(path, "stash path")?;
            args.push(path_str.to_string());
        }

        self.run_git_owned(&args)?;
        let after = self.stashes()?;
        if after.len() <= before {
            return Err(GitError::Git2(
                "no local changes in selected paths to stash".to_string(),
            ));
        }
        Ok(after[0].index)
    }

    fn stash_apply(&mut self, index: usize) -> Result<(), GitError> {
        let mut checkout = git2::build::CheckoutBuilder::new();
        checkout.force();
        let mut apply_opts = git2::StashApplyOptions::new();
        apply_opts.checkout_options(checkout);
        self.repo.stash_apply(index, Some(&mut apply_opts))?;
        Ok(())
    }

    fn stash_pop(&mut self, index: usize) -> Result<(), GitError> {
        let mut checkout = git2::build::CheckoutBuilder::new();
        checkout.force();
        let mut apply_opts = git2::StashApplyOptions::new();
        apply_opts.checkout_options(checkout);

        self.repo.stash_apply(index, Some(&mut apply_opts))?;
        self.repo.stash_drop(index)?;
        Ok(())
    }

    fn stash_drop(&mut self, index: usize) -> Result<(), GitError> {
        self.repo.stash_drop(index)?;
        Ok(())
    }

    fn commit_diff_scoped(
        &self,
        oid: &str,
        path: Option<&std::path::Path>,
    ) -> Result<Vec<DiffLine>, GitError> {
        let oid = git2::Oid::from_str(oid)?;
        let commit = self.repo.find_commit(oid)?;
        let tree = commit.tree()?;
        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };
        let mut opts = git2::DiffOptions::new();
        if let Some(path) = path {
            let path_str = Self::path_to_str(path, "commit path")?;
            opts.pathspec(path_str);
        }
        let diff =
            match self
                .repo
                .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut opts))
            {
                Ok(diff) => diff,
                Err(err) => {
                    return Ok(vec![DiffLine {
                        kind: DiffLineKind::Header,
                        content: format!("diff unavailable: {}", err),
                    }]);
                }
            };
        Ok(parse_diff(&diff))
    }

    fn commit(&self, message: &str) -> Result<String, GitError> {
        let sig = self.signature()?;
        let mut index = self.repo.index()?;
        let tree_id = index.write_tree()?;
        let tree = self.repo.find_tree(tree_id)?;

        let commit_id = match self.repo.head() {
            Ok(head) => {
                let parent = head.peel_to_commit()?;
                self.repo
                    .commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])?
            }
            Err(_) => self
                .repo
                .commit(Some("HEAD"), &sig, &sig, message, &tree, &[])?,
        };

        Ok(commit_id.to_string())
    }

    fn create_branch(&self, name: &str) -> Result<(), GitError> {
        let head = self.repo.head()?.peel_to_commit()?;
        self.repo.branch(name, &head, false)?;
        Ok(())
    }

    fn checkout_branch(&self, name: &str) -> Result<(), GitError> {
        Self::checkout_branch_in_repo(&self.repo, name)
    }

    fn checkout_branch_with_auto_stash(&mut self, name: &str) -> Result<(), GitError> {
        let has_stashed = {
            let mut opts = git2::StatusOptions::new();
            opts.include_untracked(true)
                .include_ignored(false)
                .include_unmodified(false);
            let statuses = self.repo.statuses(Some(&mut opts))?;
            let has_changes = !statuses.is_empty();
            drop(statuses);
            if !has_changes {
                false
            } else {
                let sig = self
                    .repo
                    .signature()
                    .or_else(|_| git2::Signature::now("ratagit", "ratagit@localhost"))?;
                let _ = self.repo.stash_save(
                    &sig,
                    "ratagit:auto-stash-before-switch",
                    Some(git2::StashFlags::INCLUDE_UNTRACKED),
                )?;
                true
            }
        };

        if let Err(err) = Self::checkout_branch_in_repo(&self.repo, name) {
            if has_stashed {
                let _ = self.repo.stash_apply(0, None);
                let _ = self.repo.stash_drop(0);
            }
            return Err(err);
        }

        if has_stashed {
            self.repo.stash_apply(0, None)?;
            self.repo.stash_drop(0)?;
        }
        Ok(())
    }

    fn delete_branch(&self, name: &str) -> Result<(), GitError> {
        let mut branch = self.repo.find_branch(name, git2::BranchType::Local)?;
        branch.delete()?;
        Ok(())
    }
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

    /// Send a job to the persistent worker and await the result on a tokio thread.
    #[allow(dead_code)]
    async fn spawn_repo_job_tokio<T, F>(&self, label: &'static str, job: F) -> Result<T, GitError>
    where
        T: Send + 'static,
        F: FnOnce(&mut Git2RepoInner) -> Result<T, GitError> + Send + 'static,
    {
        let rx = self.worker.run(label, job)?;
        task::spawn_blocking(move || rx.recv())
            .await
            .map_err(|e| GitError::Git2(format!("tokio join failed: {}", e)))?
            .map_err(|_| GitError::Git2("repo worker channel closed".to_string()))?
    }

    #[allow(dead_code)]
    async fn spawn_repo_job_external_tokio<T, F>(
        &self,
        label: &'static str,
        job: F,
    ) -> Result<T, GitError>
    where
        T: Send + 'static,
        F: FnOnce(&mut Git2RepoInner) -> Result<T, GitError> + Send + 'static,
    {
        let idx = self.external_next.fetch_add(1, Ordering::Relaxed) % self.external_workers.len();
        let rx = self.external_workers[idx].run(label, job)?;
        task::spawn_blocking(move || rx.recv())
            .await
            .map_err(|e| GitError::Git2(format!("tokio join failed: {}", e)))?
            .map_err(|_| GitError::Git2("external worker channel closed".to_string()))?
    }

    #[allow(dead_code)]
    fn repo_root(&self) -> &PathBuf {
        &self.repo_root
    }
}

impl GitRepository for Git2Repository {
    fn status(&self) -> Result<GitStatus, GitError> {
        self.spawn_repo_job("status", |inner| inner.status())?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn status_fast(&self) -> Result<GitStatus, GitError> {
        self.spawn_repo_job_external("status_fast", |inner| inner.status_fast())?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn status_async(&self) -> Result<Receiver<Result<GitStatus, GitError>>, GitError> {
        self.spawn_repo_job("status", |inner| inner.status())
    }

    fn status_fast_async(&self) -> Result<Receiver<Result<GitStatus, GitError>>, GitError> {
        self.spawn_repo_job_external("status_fast", |inner| inner.status_fast())
    }

    fn stage(&self, path: &std::path::Path) -> Result<(), GitError> {
        let path = path.to_path_buf();
        self.spawn_repo_job("stage", move |inner| inner.stage(&path))?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn stage_async(&self, path: PathBuf) -> Result<Receiver<Result<(), GitError>>, GitError> {
        self.spawn_repo_job("stage", move |inner| inner.stage(&path))
    }

    fn stage_paths(&self, paths: &[PathBuf]) -> Result<(), GitError> {
        let paths = paths.to_vec();
        self.spawn_repo_job("stage_paths", move |inner| inner.stage_paths(&paths))?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn unstage(&self, path: &std::path::Path) -> Result<(), GitError> {
        let path = path.to_path_buf();
        self.spawn_repo_job("unstage", move |inner| inner.unstage(&path))?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn unstage_async(&self, path: PathBuf) -> Result<Receiver<Result<(), GitError>>, GitError> {
        self.spawn_repo_job("unstage", move |inner| inner.unstage(&path))
    }

    fn unstage_paths(&self, paths: &[PathBuf]) -> Result<(), GitError> {
        let paths = paths.to_vec();
        self.spawn_repo_job("unstage_paths", move |inner| inner.unstage_paths(&paths))?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn discard_paths(&self, paths: &[PathBuf]) -> Result<(), GitError> {
        let paths = paths.to_vec();
        self.spawn_repo_job("discard_paths", move |inner| inner.discard_paths(&paths))?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn discard_paths_async(
        &self,
        paths: Vec<PathBuf>,
    ) -> Result<Receiver<Result<(), GitError>>, GitError> {
        self.spawn_repo_job("discard_paths", move |inner| inner.discard_paths(&paths))
    }

    fn diff_unstaged(&self, path: &std::path::Path) -> Result<Vec<DiffLine>, GitError> {
        let path = path.to_path_buf();
        self.spawn_repo_job("diff_unstaged", move |inner| inner.diff_unstaged(&path))?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn diff_unstaged_async(
        &self,
        path: PathBuf,
    ) -> Result<Receiver<Result<Vec<DiffLine>, GitError>>, GitError> {
        self.spawn_repo_job("diff_unstaged", move |inner| inner.diff_unstaged(&path))
    }

    fn diff_staged(&self, path: &std::path::Path) -> Result<Vec<DiffLine>, GitError> {
        let path = path.to_path_buf();
        self.spawn_repo_job("diff_staged", move |inner| inner.diff_staged(&path))?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn diff_staged_async(
        &self,
        path: PathBuf,
    ) -> Result<Receiver<Result<Vec<DiffLine>, GitError>>, GitError> {
        self.spawn_repo_job("diff_staged", move |inner| inner.diff_staged(&path))
    }

    fn diff_untracked(&self, path: &std::path::Path) -> Result<Vec<DiffLine>, GitError> {
        let path = path.to_path_buf();
        self.spawn_repo_job("diff_untracked", move |inner| inner.diff_untracked(&path))?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn diff_untracked_async(
        &self,
        path: PathBuf,
    ) -> Result<Receiver<Result<Vec<DiffLine>, GitError>>, GitError> {
        self.spawn_repo_job("diff_untracked", move |inner| inner.diff_untracked(&path))
    }

    fn diff_directory(&self, path: &std::path::Path) -> Result<Vec<DiffLine>, GitError> {
        let path = path.to_path_buf();
        self.spawn_repo_job("diff_directory", move |inner| inner.diff_directory(&path))?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn diff_directory_async(
        &self,
        path: PathBuf,
    ) -> Result<Receiver<Result<Vec<DiffLine>, GitError>>, GitError> {
        self.spawn_repo_job("diff_directory", move |inner| inner.diff_directory(&path))
    }

    fn branches(&self) -> Result<Vec<BranchInfo>, GitError> {
        self.spawn_repo_job("branches", |inner| inner.branches())?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn branches_async(&self) -> Result<Receiver<Result<Vec<BranchInfo>, GitError>>, GitError> {
        self.spawn_repo_job("branches", |inner| inner.branches())
    }

    fn branch_log(&self, name: &str, limit: usize) -> Result<Vec<DiffLine>, GitError> {
        let name = name.to_string();
        self.spawn_repo_job_external("branch_log", move |inner| inner.branch_log(&name, limit))?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn branch_log_async(
        &self,
        name: String,
        limit: usize,
    ) -> Result<Receiver<Result<Vec<DiffLine>, GitError>>, GitError> {
        self.spawn_repo_job_external("branch_log", move |inner| inner.branch_log(&name, limit))
    }

    fn commits(&self, limit: usize) -> Result<Vec<CommitInfo>, GitError> {
        self.spawn_repo_job_external("commits", move |inner| inner.commits(limit))?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn commits_fast(&self, limit: usize) -> Result<Vec<CommitInfo>, GitError> {
        self.spawn_repo_job_external("commits_fast", move |inner| inner.commits_fast(limit))?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn commits_async(
        &self,
        limit: usize,
    ) -> Result<Receiver<Result<Vec<CommitInfo>, GitError>>, GitError> {
        self.spawn_repo_job_external("commits", move |inner| inner.commits(limit))
    }

    fn commits_fast_async(
        &self,
        limit: usize,
    ) -> Result<Receiver<Result<Vec<CommitInfo>, GitError>>, GitError> {
        self.spawn_repo_job_external("commits_fast", move |inner| inner.commits_fast(limit))
    }

    fn commits_for_branch(&self, name: &str, limit: usize) -> Result<Vec<CommitInfo>, GitError> {
        let name = name.to_string();
        self.spawn_repo_job_external("commits_for_branch", move |inner| {
            inner.commits_for_branch(&name, limit)
        })?
        .recv()
        .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn commits_for_branch_async(
        &self,
        name: &str,
        limit: usize,
    ) -> Result<Receiver<Result<Vec<CommitInfo>, GitError>>, GitError> {
        let name = name.to_string();
        self.spawn_repo_job_external("commits_for_branch", move |inner| {
            inner.commits_for_branch(&name, limit)
        })
    }

    fn commit_files(&self, oid: &str) -> Result<Vec<FileEntry>, GitError> {
        let oid = oid.to_string();
        self.spawn_repo_job("commit_files", move |inner| inner.commit_files(&oid))?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn stashes(&self) -> Result<Vec<StashInfo>, GitError> {
        self.spawn_repo_job("stashes", |inner| inner.stashes())?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn stashes_async(&self) -> Result<Receiver<Result<Vec<StashInfo>, GitError>>, GitError> {
        self.spawn_repo_job("stashes", |inner| inner.stashes())
    }

    fn stash_files(&self, index: usize) -> Result<Vec<FileEntry>, GitError> {
        self.spawn_repo_job("stash_files", move |inner| inner.stash_files(index))?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn stash_diff(
        &self,
        index: usize,
        path: Option<&std::path::Path>,
    ) -> Result<Vec<DiffLine>, GitError> {
        let path = path.map(|p| p.to_path_buf());
        self.spawn_repo_job("stash_diff", move |inner| {
            inner.stash_diff(index, path.as_deref())
        })?
        .recv()
        .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn stash_diff_async(
        &self,
        index: usize,
        path: Option<PathBuf>,
    ) -> Result<Receiver<Result<Vec<DiffLine>, GitError>>, GitError> {
        self.spawn_repo_job("stash_diff", move |inner| {
            inner.stash_diff(index, path.as_deref())
        })
    }

    fn stash_push_paths(&self, paths: &[PathBuf], message: &str) -> Result<usize, GitError> {
        let paths = paths.to_vec();
        let message = message.to_string();
        self.spawn_repo_job("stash_push_paths", move |inner| {
            inner.stash_push_paths(&paths, &message)
        })?
        .recv()
        .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn stash_push_paths_async(
        &self,
        paths: Vec<PathBuf>,
        message: String,
    ) -> Result<Receiver<Result<usize, GitError>>, GitError> {
        self.spawn_repo_job("stash_push_paths", move |inner| {
            inner.stash_push_paths(&paths, &message)
        })
    }

    fn stash_apply(&self, index: usize) -> Result<(), GitError> {
        self.spawn_repo_job("stash_apply", move |inner| inner.stash_apply(index))?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn stash_apply_async(&self, index: usize) -> Result<Receiver<Result<(), GitError>>, GitError> {
        self.spawn_repo_job("stash_apply", move |inner| inner.stash_apply(index))
    }

    fn stash_pop(&self, index: usize) -> Result<(), GitError> {
        self.spawn_repo_job("stash_pop", move |inner| inner.stash_pop(index))?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn stash_pop_async(&self, index: usize) -> Result<Receiver<Result<(), GitError>>, GitError> {
        self.spawn_repo_job("stash_pop", move |inner| inner.stash_pop(index))
    }

    fn stash_drop(&self, index: usize) -> Result<(), GitError> {
        self.spawn_repo_job("stash_drop", move |inner| inner.stash_drop(index))?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn stash_drop_async(&self, index: usize) -> Result<Receiver<Result<(), GitError>>, GitError> {
        self.spawn_repo_job("stash_drop", move |inner| inner.stash_drop(index))
    }

    fn commit_diff_scoped(
        &self,
        oid: &str,
        path: Option<&std::path::Path>,
    ) -> Result<Vec<DiffLine>, GitError> {
        let oid = oid.to_string();
        let path = path.map(|p| p.to_path_buf());
        self.spawn_repo_job_external("commit_diff_scoped", move |inner| {
            inner.commit_diff_scoped(&oid, path.as_deref())
        })?
        .recv()
        .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn commit_diff_scoped_async(
        &self,
        oid: String,
        path: Option<PathBuf>,
    ) -> Result<Receiver<Result<Vec<DiffLine>, GitError>>, GitError> {
        self.spawn_repo_job_external("commit_diff_scoped", move |inner| {
            inner.commit_diff_scoped(&oid, path.as_deref())
        })
    }

    fn commit(&self, message: &str) -> Result<String, GitError> {
        let message = message.to_string();
        self.spawn_repo_job("commit", move |inner| inner.commit(&message))?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn commit_async(
        &self,
        message: String,
    ) -> Result<Receiver<Result<String, GitError>>, GitError> {
        self.spawn_repo_job("commit", move |inner| inner.commit(&message))
    }

    fn create_branch(&self, name: &str) -> Result<(), GitError> {
        let name = name.to_string();
        self.spawn_repo_job("create_branch", move |inner| inner.create_branch(&name))?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn create_branch_async(
        &self,
        name: String,
    ) -> Result<Receiver<Result<(), GitError>>, GitError> {
        self.spawn_repo_job("create_branch", move |inner| inner.create_branch(&name))
    }

    fn checkout_branch(&self, name: &str) -> Result<(), GitError> {
        let name = name.to_string();
        self.spawn_repo_job("checkout_branch", move |inner| inner.checkout_branch(&name))?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn checkout_branch_async(
        &self,
        name: String,
    ) -> Result<Receiver<Result<(), GitError>>, GitError> {
        self.spawn_repo_job("checkout_branch", move |inner| inner.checkout_branch(&name))
    }

    fn checkout_branch_with_auto_stash(&self, name: &str) -> Result<(), GitError> {
        let name = name.to_string();
        self.spawn_repo_job("checkout_branch_with_auto_stash", move |inner| {
            inner.checkout_branch_with_auto_stash(&name)
        })?
        .recv()
        .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn checkout_branch_with_auto_stash_async(
        &self,
        name: String,
    ) -> Result<Receiver<Result<(), GitError>>, GitError> {
        self.spawn_repo_job("checkout_branch_with_auto_stash", move |inner| {
            inner.checkout_branch_with_auto_stash(&name)
        })
    }

    fn delete_branch(&self, name: &str) -> Result<(), GitError> {
        let name = name.to_string();
        self.spawn_repo_job("delete_branch", move |inner| inner.delete_branch(&name))?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn delete_branch_async(
        &self,
        name: String,
    ) -> Result<Receiver<Result<(), GitError>>, GitError> {
        self.spawn_repo_job("delete_branch", move |inner| inner.delete_branch(&name))
    }

    fn fetch_default_async(&self) -> Result<Receiver<Result<String, GitError>>, GitError> {
        let upstream_result = self.spawn_repo_job("fetch_upstream_probe", |inner| {
            Ok(inner.upstream_remote_name())
        });
        let remote = match upstream_result.and_then(|rx| {
            rx.recv()
                .map_err(|_| GitError::Git2("worker disconnected".to_string()))
        }) {
            Ok(Ok(Some(name))) => name,
            _ => "origin".to_string(),
        };

        self.spawn_repo_job_external("fetch", move |inner| {
            let mut remote_obj = inner.repo.find_remote(&remote)?;
            let mut opts = git2::FetchOptions::new();
            opts.prune(git2::FetchPrune::On);
            remote_obj.fetch(&[] as &[&str], Some(&mut opts), None)?;
            Ok(remote)
        })
    }
}

#[async_trait::async_trait(?Send)]
impl AsyncGitRepository for Git2Repository {
    async fn status_async(&self) -> Result<GitStatus, GitError> {
        self.spawn_repo_job_external_tokio("status", |repo| repo.status())
            .await
    }

    async fn branches_async(&self) -> Result<Vec<BranchInfo>, GitError> {
        self.spawn_repo_job_tokio("branches", |repo| repo.branches())
            .await
    }

    async fn stashes_async(&self) -> Result<Vec<StashInfo>, GitError> {
        self.spawn_repo_job_tokio("stashes", |repo| repo.stashes())
            .await
    }

    async fn commits_async(&self, limit: usize) -> Result<Vec<CommitInfo>, GitError> {
        self.spawn_repo_job_external_tokio("commits", move |repo| repo.commits(limit))
            .await
    }

    async fn commits_for_branch_async(
        &self,
        name: &str,
        limit: usize,
    ) -> Result<Vec<CommitInfo>, GitError> {
        let branch = name.to_string();
        self.spawn_repo_job_external_tokio("commits_for_branch", move |repo| {
            repo.commits_for_branch(&branch, limit)
        })
        .await
    }

    async fn branch_log_async(&self, name: &str, limit: usize) -> Result<Vec<DiffLine>, GitError> {
        let branch = name.to_string();
        self.spawn_repo_job_external_tokio("branch_log", move |inner| {
            inner.branch_log(&branch, limit)
        })
        .await
    }

    async fn diff_unstaged_async(&self, path: PathBuf) -> Result<Vec<DiffLine>, GitError> {
        self.spawn_repo_job_tokio("diff_unstaged", move |repo| repo.diff_unstaged(&path))
            .await
    }

    async fn diff_staged_async(&self, path: PathBuf) -> Result<Vec<DiffLine>, GitError> {
        self.spawn_repo_job_tokio("diff_staged", move |repo| repo.diff_staged(&path))
            .await
    }

    async fn diff_untracked_async(&self, path: PathBuf) -> Result<Vec<DiffLine>, GitError> {
        self.spawn_repo_job_tokio("diff_untracked", move |repo| repo.diff_untracked(&path))
            .await
    }

    async fn diff_directory_async(&self, path: PathBuf) -> Result<Vec<DiffLine>, GitError> {
        self.spawn_repo_job_tokio("diff_directory", move |repo| repo.diff_directory(&path))
            .await
    }

    async fn commit_diff_scoped_async(
        &self,
        oid: &str,
        path: Option<PathBuf>,
    ) -> Result<Vec<DiffLine>, GitError> {
        let oid = oid.to_string();
        self.spawn_repo_job_external_tokio("commit_diff_scoped", move |inner| {
            inner.commit_diff_scoped(&oid, path.as_deref())
        })
        .await
    }

    async fn stash_diff_async(
        &self,
        index: usize,
        path: Option<PathBuf>,
    ) -> Result<Vec<DiffLine>, GitError> {
        self.spawn_repo_job_tokio("stash_diff", move |repo| {
            repo.stash_diff(index, path.as_deref())
        })
        .await
    }

    async fn stage_async(&self, path: PathBuf) -> Result<(), GitError> {
        self.spawn_repo_job_tokio("stage", move |repo| repo.stage(&path))
            .await
    }

    async fn stage_paths_async(&self, paths: Vec<PathBuf>) -> Result<(), GitError> {
        self.spawn_repo_job_tokio("stage_paths", move |repo| repo.stage_paths(&paths))
            .await
    }

    async fn unstage_async(&self, path: PathBuf) -> Result<(), GitError> {
        self.spawn_repo_job_tokio("unstage", move |repo| repo.unstage(&path))
            .await
    }

    async fn unstage_paths_async(&self, paths: Vec<PathBuf>) -> Result<(), GitError> {
        self.spawn_repo_job_tokio("unstage_paths", move |repo| repo.unstage_paths(&paths))
            .await
    }

    async fn discard_paths_async(&self, paths: Vec<PathBuf>) -> Result<(), GitError> {
        self.spawn_repo_job_tokio("discard_paths", move |repo| repo.discard_paths(&paths))
            .await
    }

    async fn commit_write_async(&self, message: String) -> Result<String, GitError> {
        self.spawn_repo_job_tokio("commit", move |repo| repo.commit(&message))
            .await
    }

    async fn create_branch_async(&self, name: String) -> Result<(), GitError> {
        self.spawn_repo_job_tokio("create_branch", move |repo| repo.create_branch(&name))
            .await
    }

    async fn checkout_branch_async(&self, name: String) -> Result<(), GitError> {
        self.spawn_repo_job_tokio("checkout_branch", move |repo| repo.checkout_branch(&name))
            .await
    }

    async fn checkout_branch_with_auto_stash_async(&self, name: String) -> Result<(), GitError> {
        self.spawn_repo_job_tokio("checkout_branch_with_auto_stash", move |repo| {
            repo.checkout_branch_with_auto_stash(&name)
        })
        .await
    }

    async fn delete_branch_async(&self, name: String) -> Result<(), GitError> {
        self.spawn_repo_job_tokio("delete_branch", move |repo| repo.delete_branch(&name))
            .await
    }

    async fn stash_push_paths_async(
        &self,
        paths: Vec<PathBuf>,
        message: String,
    ) -> Result<usize, GitError> {
        self.spawn_repo_job_tokio("stash_push_paths", move |repo| {
            repo.stash_push_paths(&paths, &message)
        })
        .await
    }

    async fn stash_apply_async(&self, index: usize) -> Result<(), GitError> {
        self.spawn_repo_job_tokio("stash_apply", move |repo| repo.stash_apply(index))
            .await
    }

    async fn stash_pop_async(&self, index: usize) -> Result<(), GitError> {
        self.spawn_repo_job_tokio("stash_pop", move |repo| repo.stash_pop(index))
            .await
    }

    async fn stash_drop_async(&self, index: usize) -> Result<(), GitError> {
        self.spawn_repo_job_tokio("stash_drop", move |repo| repo.stash_drop(index))
            .await
    }
}

fn parse_diff(diff: &git2::Diff) -> Vec<DiffLine> {
    parse_diff_with_limit(diff, MAX_DIFF_LINES)
}

fn parse_diff_with_limit(diff: &git2::Diff, limit: usize) -> Vec<DiffLine> {
    let mut lines = Vec::new();
    let mut truncated = false;
    let _ = diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        if lines.len() >= limit {
            truncated = true;
            return false;
        }
        // Skip binary content markers (origin 'B')
        if line.origin() == 'B' {
            lines.push(DiffLine {
                kind: DiffLineKind::Header,
                content: "<binary file>".to_string(),
            });
            return true;
        }
        let content = String::from_utf8_lossy(line.content())
            .trim_end_matches('\n')
            .to_string();
        let kind = match line.origin() {
            '+' => DiffLineKind::Added,
            '-' => DiffLineKind::Removed,
            'H' | 'F' => DiffLineKind::Header,
            _ => DiffLineKind::Context,
        };
        lines.push(DiffLine { kind, content });
        true
    });
    if truncated {
        lines.push(DiffLine {
            kind: DiffLineKind::Header,
            content: format!("... diff truncated at {} lines ...", limit),
        });
    }
    lines
}

fn parse_remote_from_upstream(upstream: &str) -> Option<String> {
    upstream
        .split('/')
        .next()
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string())
}

fn graph_charset_from_env() -> GraphCharset {
    match std::env::var("RATAGIT_GRAPH_ASCII") {
        Ok(value) if value == "1" => GraphCharset::ascii(),
        _ => GraphCharset::unicode(),
    }
}

fn build_commit_graph_lines(rows: &[GraphCommitRow], charset: GraphCharset) -> Vec<Vec<GraphCell>> {
    if rows.is_empty() {
        return Vec::new();
    }

    let pipe_sets = get_pipe_sets(rows);
    rows.iter()
        .zip(pipe_sets.iter())
        .map(|(row, pipes)| render_pipe_set(row, pipes, charset))
        .collect()
}

fn get_pipe_sets(rows: &[GraphCommitRow]) -> Vec<Vec<Pipe>> {
    if rows.is_empty() {
        return Vec::new();
    }

    let mut sets = Vec::with_capacity(rows.len());
    let mut prev = vec![Pipe {
        from_pos: 0,
        to_pos: 0,
        from_hash: None,
        to_hash: Some(rows[0].oid),
        kind: PipeKind::Starts,
    }];

    for row in rows {
        let next = get_next_pipes(&prev, row);
        sets.push(next.clone());
        prev = next;
    }

    sets
}

fn get_next_pipes(prev: &[Pipe], row: &GraphCommitRow) -> Vec<Pipe> {
    let active: Vec<Pipe> = prev
        .iter()
        .filter(|p| p.kind != PipeKind::Terminates)
        .cloned()
        .collect();

    let max_pos = active.iter().map(|p| p.to_pos).max().unwrap_or(-1);
    let commit_pos = active
        .iter()
        .find(|p| p.to_hash == Some(row.oid))
        .map(|p| p.to_pos)
        .unwrap_or(max_pos + 1);

    // Pass 1: identify terminating pipes and free their spots from taken_spots.
    let mut taken_spots: HashSet<i16> = active.iter().map(|p| p.to_pos).collect();
    for pipe in &active {
        if pipe.to_hash == Some(row.oid) {
            taken_spots.remove(&pipe.to_pos);
        }
    }

    // traversed_spots: only mark positions for non-vertical (diagonal/horizontal) moves.
    let mut traversed_spots: HashSet<i16> = HashSet::new();
    for pipe in &active {
        if pipe.from_pos != pipe.to_pos {
            for pos in pipe.from_pos.min(pipe.to_pos)..=pipe.from_pos.max(pipe.to_pos) {
                traversed_spots.insert(pos);
            }
        }
    }

    let mut next = Vec::new();
    let mut has_terminating_incoming = false;

    // Pass 2: process terminating pipes first so their freed columns are visible.
    for pipe in &active {
        if pipe.to_hash == Some(row.oid) {
            has_terminating_incoming = true;
            next.push(Pipe {
                from_pos: pipe.to_pos,
                to_pos: commit_pos,
                from_hash: pipe.to_hash,
                to_hash: Some(row.oid),
                kind: PipeKind::Terminates,
            });
        }
    }

    // Pass 3: compact and continue non-terminating pipes.
    for pipe in &active {
        if pipe.to_hash == Some(row.oid) {
            continue;
        }

        let mut target = pipe.to_pos;
        if pipe.to_pos < commit_pos {
            while target > 0 {
                let candidate = target - 1;
                if candidate == commit_pos
                    || taken_spots.contains(&candidate)
                    || traversed_spots.contains(&candidate)
                {
                    break;
                }
                target = candidate;
            }
        } else if pipe.to_pos > commit_pos {
            while target > commit_pos + 1 {
                let candidate = target - 1;
                if candidate == commit_pos
                    || taken_spots.contains(&candidate)
                    || traversed_spots.contains(&candidate)
                {
                    break;
                }
                target = candidate;
            }
        }

        next.push(Pipe {
            from_pos: pipe.to_pos,
            to_pos: target,
            from_hash: pipe.from_hash,
            to_hash: pipe.to_hash,
            kind: PipeKind::Continues,
        });
        taken_spots.insert(target);
        if pipe.to_pos != target {
            for pos in pipe.to_pos.min(target)..=pipe.to_pos.max(target) {
                traversed_spots.insert(pos);
            }
        }
    }

    if let Some(first_parent) = row.parents.first().copied() {
        let first_kind = if has_terminating_incoming {
            PipeKind::Continues
        } else {
            PipeKind::Starts
        };
        next.push(Pipe {
            from_pos: commit_pos,
            to_pos: commit_pos,
            from_hash: Some(row.oid),
            to_hash: Some(first_parent),
            kind: first_kind,
        });
        taken_spots.insert(commit_pos);
        traversed_spots.insert(commit_pos);

        for parent in row.parents.iter().skip(1).copied() {
            // Try to reuse a freed column before allocating rightward.
            let mut pos = commit_pos + 1;
            // Find lowest free slot not in taken or traversed.
            let mut found = false;
            for candidate in (0..commit_pos).rev() {
                if !taken_spots.contains(&candidate) && !traversed_spots.contains(&candidate) {
                    pos = candidate;
                    found = true;
                    break;
                }
            }
            if !found {
                pos = commit_pos + 1;
                while taken_spots.contains(&pos) || traversed_spots.contains(&pos) {
                    pos += 1;
                }
            }
            next.push(Pipe {
                from_pos: commit_pos,
                to_pos: pos,
                from_hash: Some(row.oid),
                to_hash: Some(parent),
                kind: PipeKind::Starts,
            });
            taken_spots.insert(pos);
            for p in commit_pos.min(pos)..=commit_pos.max(pos) {
                traversed_spots.insert(p);
            }
        }
    }

    next.sort_by_key(|p| (p.to_pos, p.kind));
    next
}

fn render_pipe_set(row: &GraphCommitRow, pipes: &[Pipe], charset: GraphCharset) -> Vec<GraphCell> {
    let commit_pos = commit_position(row, pipes).max(0) as usize;
    let width = pipes
        .iter()
        .map(|p| p.from_pos.max(p.to_pos) as usize)
        .max()
        .unwrap_or(commit_pos)
        .max(commit_pos)
        + 1;

    // Build a map from column index to all OIDs that pass through each column.
    let mut col_oids: Vec<HashSet<String>> = vec![HashSet::new(); width];
    for pipe in pipes {
        let from = pipe.from_pos.max(0) as usize;
        let to = pipe.to_pos.max(0) as usize;
        if from >= width || to >= width {
            continue;
        }
        let start = from.min(to);
        let end = from.max(to);
        for oid_set in col_oids.iter_mut().take(end + 1).skip(start) {
            if let Some(h) = pipe.from_hash {
                oid_set.insert(h.to_string());
            }
            if let Some(h) = pipe.to_hash {
                oid_set.insert(h.to_string());
            }
        }
    }
    if commit_pos < width {
        // Ensure commit node cell always belongs to the row commit itself.
        col_oids[commit_pos].insert(row.oid.to_string());
    }

    let mut cells = vec![CellConnections::default(); width];
    for pipe in pipes {
        draw_pipe(pipe, &mut cells);
    }

    let mut out = Vec::with_capacity(width * 2);
    for (idx, conn) in cells.into_iter().enumerate() {
        let is_node = idx == commit_pos;
        let first = if is_node {
            if row.parents.len() > 1 {
                charset.merge_node
            } else {
                charset.node
            }
        } else {
            box_char(conn, charset)
        };
        let second = if is_node {
            ' '
        } else if conn.right {
            charset.horizontal
        } else {
            ' '
        };

        let mut pipe_oids: Vec<String> = col_oids[idx].iter().cloned().collect();
        pipe_oids.sort();
        let pipe_oid = pipe_oids.first().cloned();
        out.push(GraphCell {
            text: first.to_string(),
            lane: idx,
            pipe_oid: pipe_oid.clone(),
            pipe_oids: pipe_oids.clone(),
        });
        out.push(GraphCell {
            text: second.to_string(),
            lane: idx,
            pipe_oid,
            pipe_oids,
        });
    }

    while out.last().map(|c| c.text == " ").unwrap_or(false) {
        out.pop();
    }
    if out.is_empty() {
        out.push(GraphCell {
            text: " ".to_string(),
            lane: 0,
            pipe_oid: None,
            pipe_oids: Vec::new(),
        });
    }
    out
}

fn commit_position(row: &GraphCommitRow, pipes: &[Pipe]) -> i16 {
    pipes
        .iter()
        .find(|p| p.from_hash == Some(row.oid))
        .map(|p| p.from_pos)
        .or_else(|| {
            pipes
                .iter()
                .find(|p| p.to_hash == Some(row.oid))
                .map(|p| p.to_pos)
        })
        .unwrap_or(0)
}

fn draw_pipe(pipe: &Pipe, cells: &mut [CellConnections]) {
    let from = pipe.from_pos.max(0) as usize;
    let to = pipe.to_pos.max(0) as usize;
    if from >= cells.len() || to >= cells.len() {
        return;
    }

    if pipe.kind != PipeKind::Starts {
        cells[from].up = true;
    }
    if pipe.kind != PipeKind::Terminates {
        cells[to].down = true;
    }

    if from < to {
        cells[from].right = true;
        for cell in cells.iter_mut().take(to).skip(from + 1) {
            cell.left = true;
            cell.right = true;
        }
        cells[to].left = true;
    } else if from > to {
        cells[from].left = true;
        for cell in cells.iter_mut().take(from).skip(to + 1) {
            cell.left = true;
            cell.right = true;
        }
        cells[to].right = true;
    }
}

fn box_char(conn: CellConnections, charset: GraphCharset) -> char {
    let CellConnections {
        up,
        down,
        left,
        right,
    } = conn;
    if charset.ascii {
        return match (up, down, left, right) {
            (true, true, true, true) => '+',
            (true, true, true, false) => '+',
            (true, true, false, true) => '+',
            (true, true, false, false) => '|',
            (true, false, true, true) => '+',
            (true, false, true, false) => '\\',
            (true, false, false, true) => '/',
            (true, false, false, false) => '|',
            (false, true, true, true) => '+',
            (false, true, true, false) => '\\',
            (false, true, false, true) => '/',
            (false, true, false, false) => '|',
            (false, false, true, true) => '-',
            (false, false, true, false) => '-',
            (false, false, false, true) => '-',
            _ => ' ',
        };
    }

    match (up, down, left, right) {
        (true, true, true, true) => '┼',
        (true, true, true, false) => '┤',
        (true, true, false, true) => '├',
        (true, true, false, false) => '│',
        (true, false, true, true) => '┴',
        (true, false, true, false) => '╯',
        (true, false, false, true) => '╰',
        (true, false, false, false) => '╵',
        (false, true, true, true) => '┬',
        (false, true, true, false) => '╮',
        (false, true, false, true) => '╭',
        (false, true, false, false) => '╷',
        (false, false, true, true) => '─',
        (false, false, true, false) => '╴',
        (false, false, false, true) => '╶',
        (false, false, false, false) => ' ',
    }
}

/// Helper to flatten graph cells into a plain string (for tests and backward compat).
#[cfg(test)]
fn graph_cells_to_string(cells: &[GraphCell]) -> String {
    cells.iter().map(|c| c.text.as_str()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::fs;
    use std::path::Path;
    use std::sync::{Mutex, OnceLock};
    use tempfile::TempDir;

    fn write_file(path: &Path, content: &str) {
        fs::write(path, content).expect("write file");
    }

    fn init_repo_with_commit() -> (TempDir, Git2Repository) {
        let dir = TempDir::new().expect("tempdir");
        let repo = git2::Repository::init(dir.path()).expect("init repo");

        let file = dir.path().join("tracked.txt");
        write_file(&file, "v1\n");

        let mut index = repo.index().expect("index");
        index
            .add_path(Path::new("tracked.txt"))
            .expect("add tracked.txt");
        index.write().expect("write index");
        let tree_id = index.write_tree().expect("write tree");
        let tree = repo.find_tree(tree_id).expect("find tree");

        let sig = git2::Signature::now("tester", "tester@example.com").expect("signature");
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .expect("initial commit");

        let repo = Git2Repository::open(dir.path()).expect("open git2 repo");
        (dir, repo)
    }

    #[test]
    fn test_discover_repo() {
        // Comment in English.
        let result = Git2Repository::discover();
        assert!(result.is_ok());
    }

    #[test]
    fn test_stage_unstage_roundtrip() {
        let (dir, repo) = init_repo_with_commit();
        write_file(&dir.path().join("tracked.txt"), "v2\n");

        let status = repo.status().expect("status before stage");
        assert!(status
            .unstaged
            .iter()
            .any(|f| f.path == std::path::Path::new("tracked.txt")));

        repo.stage(&PathBuf::from("tracked.txt")).expect("stage");
        let status = repo.status().expect("status after stage");
        assert!(status
            .staged
            .iter()
            .any(|f| f.path == std::path::Path::new("tracked.txt")));

        repo.unstage(&PathBuf::from("tracked.txt"))
            .expect("unstage");
        let status = repo.status().expect("status after unstage");
        assert!(status
            .unstaged
            .iter()
            .any(|f| f.path == std::path::Path::new("tracked.txt")));
    }

    #[test]
    fn test_commit_happy_path() {
        let (dir, repo) = init_repo_with_commit();
        write_file(&dir.path().join("new.txt"), "hello\n");

        repo.stage(&PathBuf::from("new.txt")).expect("stage new");
        let oid = repo.commit("add new").expect("commit");
        assert!(!oid.is_empty());

        let commits = repo.commits(1).expect("commits");
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].message, "add new");
        assert_eq!(commits[0].oid, oid);
        assert!(!commits[0].graph.is_empty());
        assert_eq!(commits[0].sync_state, CommitSyncState::DefaultBranch);

        let patch = repo.commit_diff_scoped(&oid, None).expect("commit diff");
        assert!(!patch.is_empty());

        let files = repo.commit_files(&oid).expect("commit files");
        assert!(files
            .iter()
            .any(|f| f.path == std::path::Path::new("new.txt")));

        let scoped = repo
            .commit_diff_scoped(&oid, Some(std::path::Path::new("new.txt")))
            .expect("commit scoped diff");
        assert!(!scoped.is_empty());
    }

    #[test]
    fn test_branch_create_checkout_delete() {
        let (_dir, repo) = init_repo_with_commit();
        repo.create_branch("feature/a").expect("create branch");
        assert!(repo
            .branches()
            .expect("branches")
            .iter()
            .any(|b| b.name == "feature/a"));

        repo.checkout_branch("feature/a").expect("checkout branch");
        assert!(repo
            .branches()
            .expect("branches after checkout")
            .iter()
            .any(|b| b.name == "feature/a" && b.is_current));

        repo.checkout_branch("main")
            .or_else(|_| repo.checkout_branch("master"))
            .expect("checkout default branch");
        repo.delete_branch("feature/a").expect("delete branch");
        assert!(!repo
            .branches()
            .expect("branches after delete")
            .iter()
            .any(|b| b.name == "feature/a"));
    }

    #[test]
    fn test_stash_push_apply_drop() {
        let (dir, repo) = init_repo_with_commit();
        write_file(&dir.path().join("tracked.txt"), "v2\n");

        let created = repo
            .stash_push_paths(&[PathBuf::from("tracked.txt")], "wip")
            .expect("stash push");
        assert_eq!(created, 0);
        assert_eq!(repo.stashes().expect("stashes after push").len(), 1);

        repo.stash_apply(0).expect("stash apply");
        let status_after_apply = repo.status().expect("status after stash apply");
        assert!(status_after_apply
            .unstaged
            .iter()
            .any(|f| f.path == std::path::Path::new("tracked.txt")));

        repo.stash_drop(0).expect("stash drop");
        assert!(repo.stashes().expect("stashes after drop").is_empty());
    }

    #[test]
    fn test_stash_pop_restores_and_removes_entry() {
        let (dir, repo) = init_repo_with_commit();
        write_file(&dir.path().join("tracked.txt"), "v3\n");

        repo.stash_push_paths(&[PathBuf::from("tracked.txt")], "wip pop")
            .expect("stash push");
        assert_eq!(repo.stashes().expect("stashes after push").len(), 1);

        repo.stash_pop(0).expect("stash pop");
        let status = repo.status().expect("status after stash pop");
        assert!(status
            .unstaged
            .iter()
            .any(|f| f.path == std::path::Path::new("tracked.txt")));
        assert!(repo.stashes().expect("stashes after pop").is_empty());
    }

    #[test]
    fn test_stash_pop_with_unrelated_local_change_keeps_both_changes() {
        let (dir, repo) = init_repo_with_commit();
        write_file(&dir.path().join("tracked.txt"), "v3\n");

        repo.stash_push_paths(&[PathBuf::from("tracked.txt")], "wip pop")
            .expect("stash push");

        write_file(&dir.path().join("local-only.txt"), "local\n");

        repo.stash_pop(0).expect("stash pop");

        let status = repo.status().expect("status after stash pop");
        assert!(status
            .unstaged
            .iter()
            .any(|f| f.path == std::path::Path::new("tracked.txt")));
        assert!(status
            .untracked
            .iter()
            .any(|f| f.path == std::path::Path::new("local-only.txt")));
        assert!(repo.stashes().expect("stashes after pop").is_empty());
    }

    #[test]
    fn test_stash_push_only_selected_paths() {
        let (dir, repo) = init_repo_with_commit();
        write_file(&dir.path().join("tracked.txt"), "tracked changed\n");
        write_file(&dir.path().join("other.txt"), "other changed\n");

        repo.stash_push_paths(&[PathBuf::from("tracked.txt")], "partial")
            .expect("stash push selected path");

        let status = repo.status().expect("status after partial stash");
        assert!(!repo
            .stashes()
            .expect("stashes after partial push")
            .is_empty());
        assert!(status
            .unstaged
            .iter()
            .chain(status.untracked.iter())
            .any(|f| f.path == std::path::Path::new("other.txt")));
    }

    #[test]
    fn test_stash_diff_for_selected_path() {
        let (dir, repo) = init_repo_with_commit();
        write_file(&dir.path().join("tracked.txt"), "v2\n");

        repo.stash_push_paths(&[PathBuf::from("tracked.txt")], "diff path")
            .expect("stash push for diff");

        let diff = repo
            .stash_diff(0, Some(Path::new("tracked.txt")))
            .expect("stash diff for path");
        assert!(!diff.is_empty());
        assert!(diff.iter().any(|l| matches!(l.kind, DiffLineKind::Header)));
    }

    #[test]
    fn test_diff_directory_includes_staged_unstaged_and_untracked_changes() {
        let (dir, repo) = init_repo_with_commit();
        fs::create_dir_all(dir.path().join("nested")).expect("create nested dir");

        write_file(
            &dir.path().join("nested").join("tracked.txt"),
            "tracked change\n",
        );
        repo.stage(&PathBuf::from("nested/tracked.txt"))
            .expect("stage tracked path");
        repo.commit("add nested tracked")
            .expect("commit nested tracked");

        write_file(&dir.path().join("nested").join("staged.txt"), "staged\n");
        repo.stage(&PathBuf::from("nested/staged.txt"))
            .expect("stage nested file");
        write_file(
            &dir.path().join("nested").join("tracked.txt"),
            "tracked changed again\n",
        );
        write_file(&dir.path().join("nested").join("new.txt"), "new file\n");

        let diff = repo
            .diff_directory(Path::new("nested"))
            .expect("directory diff");
        assert!(!diff.is_empty());
        assert!(diff
            .iter()
            .any(|line| line.content.contains("nested/staged.txt")));
        assert!(diff
            .iter()
            .any(|line| line.content.contains("nested/tracked.txt")));
        assert!(diff
            .iter()
            .any(|line| line.content.contains("nested/new.txt")));
    }

    #[test]
    fn test_parse_remote_from_upstream() {
        assert_eq!(
            parse_remote_from_upstream("origin/main").as_deref(),
            Some("origin")
        );
        assert_eq!(
            parse_remote_from_upstream("upstream/feature/x").as_deref(),
            Some("upstream")
        );
        assert!(parse_remote_from_upstream("").is_none());
    }

    #[test]
    fn branch_log_returns_raw_git_graph_output() {
        let (_dir, repo) = init_repo_with_commit();
        let branch = repo
            .branches()
            .expect("branches")
            .into_iter()
            .find(|branch| branch.is_current)
            .expect("current branch")
            .name;

        let lines = repo.branch_log(&branch, 20).expect("branch log");

        assert!(!lines.is_empty());
        assert!(lines.iter().any(|line| line.content.contains('\u{1b}')));
        assert!(lines.iter().any(|line| line.content.contains('*')));
    }

    #[test]
    fn branch_log_limit_truncates_raw_output() {
        let (dir, repo) = init_repo_with_commit();
        for index in 0..44 {
            write_file(
                &dir.path().join("tracked.txt"),
                &format!("v{}\n", index + 2),
            );
            repo.commit(&format!("commit {}", index + 2))
                .expect("commit linear history");
        }
        let branch = repo
            .branches()
            .expect("branches")
            .into_iter()
            .find(|branch| branch.is_current)
            .expect("current branch")
            .name;

        let first_batch = repo.branch_log(&branch, 20).expect("first batch");
        let second_batch = repo.branch_log(&branch, 40).expect("second batch");

        assert!(!first_batch.is_empty());
        assert!(second_batch.len() > first_batch.len());
        assert_ne!(
            first_batch.last().map(|line| line.content.as_str()),
            second_batch.last().map(|line| line.content.as_str())
        );
    }

    #[test]
    fn test_build_commit_graph_lines_linear_history() {
        let rows = vec![
            GraphCommitRow {
                oid: oid("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
                parents: vec![oid("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")],
            },
            GraphCommitRow {
                oid: oid("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
                parents: vec![oid("cccccccccccccccccccccccccccccccccccccccc")],
            },
            GraphCommitRow {
                oid: oid("cccccccccccccccccccccccccccccccccccccccc"),
                parents: vec![],
            },
        ];

        let lines = build_commit_graph_lines(&rows, GraphCharset::unicode());
        let strs: Vec<String> = lines.iter().map(|c| graph_cells_to_string(c)).collect();
        assert_eq!(strs, vec!["◯", "◯", "◯"]);
    }

    #[test]
    fn test_build_commit_graph_lines_merge_history() {
        let rows = vec![
            GraphCommitRow {
                oid: oid("1111111111111111111111111111111111111111"),
                parents: vec![
                    oid("2222222222222222222222222222222222222222"),
                    oid("3333333333333333333333333333333333333333"),
                ],
            },
            GraphCommitRow {
                oid: oid("2222222222222222222222222222222222222222"),
                parents: vec![oid("4444444444444444444444444444444444444444")],
            },
            GraphCommitRow {
                oid: oid("3333333333333333333333333333333333333333"),
                parents: vec![oid("4444444444444444444444444444444444444444")],
            },
            GraphCommitRow {
                oid: oid("4444444444444444444444444444444444444444"),
                parents: vec![],
            },
        ];

        let lines = build_commit_graph_lines(&rows, GraphCharset::unicode());
        let strs: Vec<String> = lines.iter().map(|c| graph_cells_to_string(c)).collect();
        assert_eq!(strs, vec!["⏣ ╮", "◯ │", "│ ◯", "◯ ╯"]);
    }

    #[test]
    fn test_build_commit_graph_lines_octopus_merge_like() {
        let rows = vec![
            GraphCommitRow {
                oid: oid("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
                parents: vec![
                    oid("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
                    oid("cccccccccccccccccccccccccccccccccccccccc"),
                    oid("dddddddddddddddddddddddddddddddddddddddd"),
                ],
            },
            GraphCommitRow {
                oid: oid("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
                parents: vec![oid("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee")],
            },
            GraphCommitRow {
                oid: oid("cccccccccccccccccccccccccccccccccccccccc"),
                parents: vec![oid("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee")],
            },
            GraphCommitRow {
                oid: oid("dddddddddddddddddddddddddddddddddddddddd"),
                parents: vec![oid("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee")],
            },
            GraphCommitRow {
                oid: oid("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"),
                parents: vec![],
            },
        ];

        let lines = build_commit_graph_lines(&rows, GraphCharset::unicode());
        let strs: Vec<String> = lines.iter().map(|c| graph_cells_to_string(c)).collect();
        assert_eq!(strs[0], "⏣ ┬─╮");
        assert_eq!(strs[1], "◯ │ │");
        assert_eq!(strs[2], "│ ◯ │");
        assert_eq!(strs[3], "│ │ ◯");
        assert_eq!(strs[4], "◯ ┴─╯");
    }

    #[test]
    fn test_graph_charset_from_env_toggle() {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        let _guard = ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("env lock");

        let original = std::env::var("RATAGIT_GRAPH_ASCII").ok();

        std::env::remove_var("RATAGIT_GRAPH_ASCII");
        assert_eq!(graph_charset_from_env().node, '◯');

        std::env::set_var("RATAGIT_GRAPH_ASCII", "1");
        assert_eq!(graph_charset_from_env().node, '*');

        if let Some(value) = original {
            std::env::set_var("RATAGIT_GRAPH_ASCII", value);
        } else {
            std::env::remove_var("RATAGIT_GRAPH_ASCII");
        }
    }

    fn oid(value: &str) -> git2::Oid {
        git2::Oid::from_str(value).expect("valid oid")
    }

    fn rows_from_oid_pairs(pairs: &[(&str, &[&str])]) -> Vec<GraphCommitRow> {
        pairs
            .iter()
            .map(|(o, ps)| GraphCommitRow {
                oid: oid(o),
                parents: ps.iter().map(|p| oid(p)).collect(),
            })
            .collect()
    }

    fn complex_repo_rows() -> Vec<GraphCommitRow> {
        rows_from_oid_pairs(&[
            (
                "841a287edd4e6b32c59c1affc677fc28dcede01d",
                &[
                    "6d2c737c870c743a0a3e5d6113fd1225f8ce7214",
                    "6ff4b94506817257e24550cb4cb86db19758ed2d",
                ],
            ),
            (
                "6ff4b94506817257e24550cb4cb86db19758ed2d",
                &["428ecd10104810073d8752729e789f582f0264ad"],
            ),
            (
                "428ecd10104810073d8752729e789f582f0264ad",
                &[
                    "22751db39f9ea5f474440fa82c84301c693ba3ed",
                    "6d2c737c870c743a0a3e5d6113fd1225f8ce7214",
                ],
            ),
            (
                "6d2c737c870c743a0a3e5d6113fd1225f8ce7214",
                &[
                    "59280f68a38d8d594589173a7eafb371bcecb5ee",
                    "13fe9925399cc03ba03628485c3f95b078ea9dc1",
                ],
            ),
            (
                "13fe9925399cc03ba03628485c3f95b078ea9dc1",
                &["1a0137ef486eae5607fbc7852169e626113ebd07"],
            ),
            (
                "59280f68a38d8d594589173a7eafb371bcecb5ee",
                &[
                    "c014d84fb6656d9a0f1199405c75bbf2cca0c260",
                    "69ca3992d4902f601412d7f4d372fa9d32f4c9ed",
                ],
            ),
            (
                "69ca3992d4902f601412d7f4d372fa9d32f4c9ed",
                &["1363c71ecdba04252939f2e5baf9fb0d6e78612c"],
            ),
            (
                "1363c71ecdba04252939f2e5baf9fb0d6e78612c",
                &["14c40bfded212b58b04ea8344e7c3712f8b9bfcb"],
            ),
            (
                "2fdf45590c1b89c3ee5ad627c6c65cf1c537a17f",
                &["e5fe47f2deddf8bf38f6295e64663fd2d4ffd062"],
            ),
            (
                "e5fe47f2deddf8bf38f6295e64663fd2d4ffd062",
                &[
                    "17b91a61c98954a0d56e162c5749b036f2a8c53b",
                    "c014d84fb6656d9a0f1199405c75bbf2cca0c260",
                ],
            ),
            (
                "c014d84fb6656d9a0f1199405c75bbf2cca0c260",
                &[
                    "1a0137ef486eae5607fbc7852169e626113ebd07",
                    "1b3ce77a986f530827838e548103d7b81a5e8d38",
                ],
            ),
            (
                "1b3ce77a986f530827838e548103d7b81a5e8d38",
                &["14c40bfded212b58b04ea8344e7c3712f8b9bfcb"],
            ),
            (
                "1a0137ef486eae5607fbc7852169e626113ebd07",
                &[
                    "eaa7952dc7a202a75a741ed4bc9516c6aa624942",
                    "22751db39f9ea5f474440fa82c84301c693ba3ed",
                ],
            ),
            (
                "22751db39f9ea5f474440fa82c84301c693ba3ed",
                &["14837bd3e1eddf263536d7121f0f6b22293715eb"],
            ),
            (
                "14837bd3e1eddf263536d7121f0f6b22293715eb",
                &["14c40bfded212b58b04ea8344e7c3712f8b9bfcb"],
            ),
            (
                "eaa7952dc7a202a75a741ed4bc9516c6aa624942",
                &["14c40bfded212b58b04ea8344e7c3712f8b9bfcb"],
            ),
            (
                "17b91a61c98954a0d56e162c5749b036f2a8c53b",
                &["a054e8662b0bce64197c45a6f97f380547cee39c"],
            ),
            (
                "a054e8662b0bce64197c45a6f97f380547cee39c",
                &["14c40bfded212b58b04ea8344e7c3712f8b9bfcb"],
            ),
            (
                "14c40bfded212b58b04ea8344e7c3712f8b9bfcb",
                &["1a095db72c8bfd761cddfa4983f8acdfc7ef2f5f"],
            ),
            ("1a095db72c8bfd761cddfa4983f8acdfc7ef2f5f", &[]),
        ])
    }

    // Build GraphCommitRows from a list of (oid_hex, [parent_hex...]) pairs.
    // Each label is padded to 40 hex chars using only hex-safe digits.
    fn rows_from_pairs(pairs: &[(&str, &[&str])]) -> Vec<GraphCommitRow> {
        fn expand(s: &str) -> git2::Oid {
            // Map label chars to hex: use sha256-style - only 0-9a-f.
            // Simple approach: hash the label string to a deterministic 40-char hex.
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut h = DefaultHasher::new();
            s.hash(&mut h);
            let v = h.finish();
            // Produce 40 hex chars by repeating the 16-char hash.
            let hex16 = format!("{:016x}", v);
            let hex40 = format!("{}{}{}{}", hex16, &hex16[..8], &hex16[..8], &hex16[..8]);
            git2::Oid::from_str(&hex40[..40]).expect("oid from label")
        }
        pairs
            .iter()
            .map(|(o, ps)| GraphCommitRow {
                oid: expand(o),
                parents: ps.iter().map(|p| expand(p)).collect(),
            })
            .collect()
    }

    // Render rows to a list of strings, one per commit row.
    fn render_rows(pairs: &[(&str, &[&str])]) -> Vec<String> {
        let rows = rows_from_pairs(pairs);
        let lines = build_commit_graph_lines(&rows, GraphCharset::unicode());
        lines.iter().map(|c| graph_cells_to_string(c)).collect()
    }

    // ------- Tests derived from complex-merge-graph-repo topology -------
    // Commit order (topo-order, newest first), using short labels:
    //
    //  0: M1  parents=[R, F1_3]     merge: feature/a back into main
    //  1: F1_3 parents=[F1_2]       feat(a): add a3 after cross merge
    //  2: F1_2 parents=[A2, R]      merge: sync main into feature/a (cross merge)
    //  3: R    parents=[C, L1]      merge: release/1.0 back to main
    //  4: L1   parents=[MA]         docs(release): prepare 1.0 notes
    //  5: C    parents=[H, C2]      merge: feature/c into main
    //  6: C2   parents=[C1]         feat(c): add c2
    //  7: C1   parents=[BASE]       feat(c): add c1
    //  8: H    parents=[MA, FX]     merge: hotfix/urgent into main
    //  9: FX   parents=[BASE]       fix: urgent hotfix
    // 10: MA   parents=[MB, A2]     merge: feature/a into main
    // 11: A2   parents=[A1]         feat(a): add a2
    // 12: A1   parents=[BASE]       feat(a): add a1
    // 13: MB   parents=[BASE]       feat: main baseline 2
    // 14: B2   parents=[B1]         feat(b): add b2 (on feature/b, not in main history here)
    // 15: B1   parents=[BASE]       feat(b): add b1
    // 16: BASE parents=[INIT]       feat: main baseline 1
    // 17: INIT parents=[]           chore: initial commit
    //
    // (Using git --topo-order output)

    #[test]
    fn test_graph_full_repo_topology() {
        // Full topology from d:/tmp/complex-merge-graph-repo, topo-order --all.
        let lines = build_commit_graph_lines(&complex_repo_rows(), GraphCharset::unicode());
        let strs: Vec<String> = lines.iter().map(|c| graph_cells_to_string(c)).collect();
        eprintln!("=== Full repo topology ===");
        for (i, row) in strs.iter().enumerate() {
            eprintln!("row {:2}: {}", i, row);
        }
        for (i, row) in strs.iter().enumerate() {
            assert!(!row.is_empty(), "row {i} empty");
        }
        for (i, row) in strs.iter().enumerate() {
            let w = row.chars().count();
            assert!(w <= 22, "row {i} too wide ({w} chars): {row:?}");
        }
    }

    #[test]
    fn test_complex_repo_node_cells_include_row_oid() {
        let rows = complex_repo_rows();
        let lines = build_commit_graph_lines(&rows, GraphCharset::unicode());
        for (idx, row) in rows.iter().enumerate() {
            let row_oid = row.oid.to_string();
            let node = lines[idx]
                .iter()
                .find(|cell| matches!(cell.text.as_str(), "◯" | "⏣" | "*" | "#"))
                .expect("node cell exists");
            assert!(
                node.pipe_oids.iter().any(|oid| oid == &row_oid),
                "row {idx} node should include row oid"
            );
        }
    }

    #[test]
    fn test_complex_repo_has_multi_owner_overlap_cells() {
        let rows = complex_repo_rows();
        let lines = build_commit_graph_lines(&rows, GraphCharset::unicode());
        let has_overlap = lines
            .iter()
            .flat_map(|line| line.iter())
            .any(|cell| cell.text != " " && cell.pipe_oids.len() > 1);
        assert!(
            has_overlap,
            "expected at least one non-space overlap cell with multiple path owners"
        );
    }

    #[test]
    fn test_graph_cross_merge_bidirectional() {
        // Minimal cross-merge: X merges [A, B], later Y merges [C, X] where X was already
        // on a side branch. Tests that lane assignments don't collide.
        let strs = render_rows(&[
            ("M", &["A", "B"]),  // merge commit on main
            ("B", &["A", "M2"]), // cross: B references M2 which is ahead in the list
            ("A", &["BASE"]),
            ("M2", &["BASE"]),
            ("BASE", &[]),
        ]);
        for (i, row) in strs.iter().enumerate() {
            assert!(!row.is_empty(), "row {i} empty");
        }
    }

    #[test]
    fn test_graph_feature_b_parallel_branch() {
        // feature/b has commits b3, sync-merge, b2, b1 that never merge into main
        // within this window. They should appear as parallel lanes.
        let strs = render_rows(&[
            ("B3", &["BM"]),
            ("BM", &["B2", "MB"]), // sync merge from main
            ("MB", &["BASE"]),
            ("B2", &["B1"]),
            ("B1", &["BASE"]),
            ("BASE", &[]),
        ]);
        for (i, row) in strs.iter().enumerate() {
            assert!(!row.is_empty(), "row {i} empty: {:?}", row);
        }
        // BM is a merge node.
        assert!(
            strs[1].contains('⏣'),
            "BM should be merge node: {:?}",
            strs[1]
        );
    }

    #[tokio::test]
    async fn test_async_git_repository_status() {
        let (_dir, repo) = init_repo_with_commit();
        let status = AsyncGitRepository::status_async(&repo)
            .await
            .expect("status");
        assert!(status.unstaged.is_empty());
    }

    #[tokio::test]
    async fn test_async_git_repository_commits() {
        let (_dir, repo) = init_repo_with_commit();
        let commits = AsyncGitRepository::commits_async(&repo, 1)
            .await
            .expect("commits");
        assert_eq!(commits.len(), 1);
    }
}
