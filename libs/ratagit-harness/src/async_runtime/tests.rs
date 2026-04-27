use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ratagit_core::{
    AppContext, BranchDeleteMode, BranchEntry, Command, CommitEntry, CommitFileDiffTarget,
    CommitFileEntry, FileDiffTarget, FilesSnapshot, RepoSnapshot, ResetMode, StashEntry,
};
use ratagit_git::{GitBackend, GitError, MockGitBackend};
use ratagit_testkit::fixture_dirty_repo;
use ratagit_ui::TerminalSize;

use super::{AsyncRuntime, DEFAULT_READ_WORKER_COUNT};

#[derive(Clone)]
struct RecordingFactory {
    next_id: Arc<AtomicUsize>,
    log: Arc<Mutex<Vec<String>>>,
    mutation_active: Arc<AtomicUsize>,
    max_mutation_active: Arc<AtomicUsize>,
    refresh_started: Option<Sender<()>>,
    refresh_release: Option<Arc<Mutex<Receiver<()>>>>,
}

struct RecordingBackend {
    id: usize,
    inner: MockGitBackend,
    log: Arc<Mutex<Vec<String>>>,
    mutation_active: Arc<AtomicUsize>,
    max_mutation_active: Arc<AtomicUsize>,
    refresh_started: Option<Sender<()>>,
    refresh_release: Option<Arc<Mutex<Receiver<()>>>>,
}

impl RecordingFactory {
    fn new() -> Self {
        Self {
            next_id: Arc::new(AtomicUsize::new(0)),
            log: Arc::new(Mutex::new(Vec::new())),
            mutation_active: Arc::new(AtomicUsize::new(0)),
            max_mutation_active: Arc::new(AtomicUsize::new(0)),
            refresh_started: None,
            refresh_release: None,
        }
    }

    fn with_blocking_refresh(refresh_started: Sender<()>, refresh_release: Receiver<()>) -> Self {
        Self {
            refresh_started: Some(refresh_started),
            refresh_release: Some(Arc::new(Mutex::new(refresh_release))),
            ..Self::new()
        }
    }

    fn build(&self) -> RecordingBackend {
        RecordingBackend {
            id: self.next_id.fetch_add(1, Ordering::SeqCst),
            inner: MockGitBackend::new(fixture_dirty_repo()),
            log: Arc::clone(&self.log),
            mutation_active: Arc::clone(&self.mutation_active),
            max_mutation_active: Arc::clone(&self.max_mutation_active),
            refresh_started: self.refresh_started.clone(),
            refresh_release: self.refresh_release.clone(),
        }
    }
}

impl RecordingBackend {
    fn record(&self, entry: impl Into<String>) {
        self.log
            .lock()
            .expect("recording log lock")
            .push(entry.into());
    }

    fn start_mutation(&self, label: &str) {
        self.record(format!("{label}-start:{}", self.id));
        let active = self.mutation_active.fetch_add(1, Ordering::SeqCst) + 1;
        self.max_mutation_active.fetch_max(active, Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(20));
    }

    fn finish_mutation(&self, label: &str) {
        self.mutation_active.fetch_sub(1, Ordering::SeqCst);
        self.record(format!("{label}-end:{}", self.id));
    }
}

macro_rules! delegate_recording_backend {
    ($($method:ident($($arg:ident: $arg_ty:ty),*) -> $ret:ty;)*) => {
        $(
            fn $method(&mut self, $($arg: $arg_ty),*) -> $ret {
                self.inner.$method($($arg),*)
            }
        )*
    };
}

impl GitBackend for RecordingBackend {
    fn refresh_snapshot(&mut self) -> Result<RepoSnapshot, GitError> {
        self.record(format!("refresh:{}", self.id));
        if let Some(started) = &self.refresh_started {
            let _ = started.send(());
        }
        if let Some(release) = &self.refresh_release {
            release
                .lock()
                .expect("refresh release lock")
                .recv_timeout(Duration::from_secs(2))
                .expect("test should release refresh");
        }
        self.inner.refresh_snapshot()
    }

    fn refresh_files(&mut self) -> Result<FilesSnapshot, GitError> {
        self.record(format!("refresh-files:{}", self.id));
        if let Some(started) = &self.refresh_started {
            let _ = started.send(());
        }
        if let Some(release) = &self.refresh_release {
            release
                .lock()
                .expect("refresh release lock")
                .recv_timeout(Duration::from_secs(2))
                .expect("test should release refresh");
        }
        self.inner.refresh_files()
    }

    fn refresh_branches(&mut self) -> Result<Vec<BranchEntry>, GitError> {
        self.record(format!("refresh-branches:{}", self.id));
        self.inner.refresh_branches()
    }

    fn refresh_commits(&mut self) -> Result<Vec<CommitEntry>, GitError> {
        self.record(format!("refresh-commits:{}", self.id));
        self.inner.refresh_commits()
    }

    fn refresh_stashes(&mut self) -> Result<Vec<StashEntry>, GitError> {
        self.record(format!("refresh-stash:{}", self.id));
        self.inner.refresh_stashes()
    }

    fn load_more_commits(
        &mut self,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<CommitEntry>, GitError> {
        self.record(format!("load-more:{}:{offset}", self.id));
        self.inner.load_more_commits(offset, limit)
    }

    fn files_details_diff(&mut self, targets: &[FileDiffTarget]) -> Result<String, GitError> {
        let paths = targets
            .iter()
            .map(|target| target.path.as_str())
            .collect::<Vec<_>>()
            .join(",");
        self.record(format!("details-diff:{}:{paths}", self.id));
        self.inner.files_details_diff(targets)
    }

    fn stage_files(&mut self, paths: &[String]) -> Result<(), GitError> {
        self.start_mutation("stage");
        let result = self.inner.stage_files(paths);
        self.finish_mutation("stage");
        result
    }

    fn unstage_files(&mut self, paths: &[String]) -> Result<(), GitError> {
        self.start_mutation("unstage");
        let result = self.inner.unstage_files(paths);
        self.finish_mutation("unstage");
        result
    }

    delegate_recording_backend! {
        branch_details_log(branch: &str, max_count: usize) -> Result<String, GitError>;
        commit_details_diff(commit_id: &str) -> Result<String, GitError>;
        commit_files(commit_id: &str) -> Result<Vec<CommitFileEntry>, GitError>;
        commit_file_diff(target: &CommitFileDiffTarget) -> Result<String, GitError>;
        stage_file(path: &str) -> Result<(), GitError>;
        unstage_file(path: &str) -> Result<(), GitError>;
        create_commit(message: &str) -> Result<(), GitError>;
        pull() -> Result<(), GitError>;
        push(force: bool) -> Result<(), GitError>;
        create_branch(name: &str, start_point: &str) -> Result<(), GitError>;
        checkout_branch(name: &str, auto_stash: bool) -> Result<(), GitError>;
        delete_branch(name: &str, mode: BranchDeleteMode, force: bool) -> Result<(), GitError>;
        rebase_branch(target: &str, interactive: bool, auto_stash: bool) -> Result<(), GitError>;
        squash_commits(commit_ids: &[String]) -> Result<(), GitError>;
        fixup_commits(commit_ids: &[String]) -> Result<(), GitError>;
        reword_commit(commit_id: &str, message: &str) -> Result<(), GitError>;
        delete_commits(commit_ids: &[String]) -> Result<(), GitError>;
        checkout_commit_detached(commit_id: &str, auto_stash: bool) -> Result<(), GitError>;
        stash_push(message: &str) -> Result<(), GitError>;
        stash_files(message: &str, paths: &[String]) -> Result<(), GitError>;
        stash_pop(stash_id: &str) -> Result<(), GitError>;
        reset(mode: ResetMode) -> Result<(), GitError>;
        nuke() -> Result<(), GitError>;
        discard_files(paths: &[String]) -> Result<(), GitError>;
    }
}

fn file_diff_target(path: &str) -> FileDiffTarget {
    FileDiffTarget {
        path: path.to_string(),
        untracked: false,
        is_directory_marker: path.ends_with('/'),
    }
}

#[test]
fn read_commands_are_distributed_across_read_workers() {
    let factory = RecordingFactory::new();
    let log = Arc::clone(&factory.log);
    let runtime_factory = factory.clone();
    let mut runtime = AsyncRuntime::new(
        AppContext::default(),
        move || runtime_factory.build(),
        terminal_size(),
    );

    runtime.process_commands(
        (0..DEFAULT_READ_WORKER_COUNT)
            .map(|offset| Command::LoadMoreCommits {
                offset,
                limit: 1,
                epoch: 0,
            })
            .collect(),
    );

    let entries = wait_for_log_count(&log, DEFAULT_READ_WORKER_COUNT);
    let worker_ids = entries
        .iter()
        .filter_map(|entry| {
            entry
                .strip_prefix("load-more:")
                .and_then(|rest| rest.split(':').next())
                .and_then(|id| id.parse::<usize>().ok())
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        worker_ids,
        (0..DEFAULT_READ_WORKER_COUNT).collect::<BTreeSet<_>>()
    );
}

#[test]
fn split_refresh_results_apply_while_files_refresh_is_blocked() {
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let factory = RecordingFactory::with_blocking_refresh(started_tx, release_rx);
    let runtime_factory = factory.clone();
    let mut runtime = AsyncRuntime::new(
        AppContext::default(),
        move || runtime_factory.build(),
        terminal_size(),
    );

    runtime.dispatch_ui(ratagit_core::UiAction::RefreshAll);
    started_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("files refresh should start");

    wait_for_state(&mut runtime, |runtime| {
        !runtime.state.repo.branches.items.is_empty()
            && !runtime.state.repo.commits.items.is_empty()
            && !runtime.state.repo.stash.items.is_empty()
    });

    assert!(runtime.state.repo.files.items.is_empty());
    assert_eq!(runtime.state.repo.status.refresh_count, 0);
    assert!(runtime.state.work.refresh_pending);
    release_tx.send(()).expect("files refresh should release");
    wait_for_quiet_tick(&mut runtime);
}

#[test]
fn mutating_commands_are_serialized_on_the_write_worker() {
    let factory = RecordingFactory::new();
    let log = Arc::clone(&factory.log);
    let max_mutation_active = Arc::clone(&factory.max_mutation_active);
    let runtime_factory = factory.clone();
    let mut runtime = AsyncRuntime::new(
        AppContext::default(),
        move || runtime_factory.build(),
        terminal_size(),
    );

    runtime.process_commands(vec![
        Command::StageFiles {
            paths: vec!["src/lib.rs".to_string()],
        },
        Command::UnstageFiles {
            paths: vec!["src/lib.rs".to_string()],
        },
    ]);

    let entries = wait_for_entries(&log, |entries| {
        entries
            .iter()
            .filter(|entry| entry.ends_with(&format!(":{}", DEFAULT_READ_WORKER_COUNT)))
            .count()
            >= 4
    });
    assert!(entries.contains(&format!("stage-start:{DEFAULT_READ_WORKER_COUNT}")));
    assert!(entries.contains(&format!("stage-end:{DEFAULT_READ_WORKER_COUNT}")));
    assert!(entries.contains(&format!("unstage-start:{DEFAULT_READ_WORKER_COUNT}")));
    assert!(entries.contains(&format!("unstage-end:{DEFAULT_READ_WORKER_COUNT}")));
    assert_eq!(max_mutation_active.load(Ordering::SeqCst), 1);
}

#[test]
fn read_commands_are_deferred_while_mutation_is_pending() {
    let factory = RecordingFactory::new();
    let log = Arc::clone(&factory.log);
    let runtime_factory = factory.clone();
    let mut runtime = AsyncRuntime::new(
        AppContext::default(),
        move || runtime_factory.build(),
        terminal_size(),
    );

    runtime.write_commands_in_flight = 1;
    runtime.process_commands(vec![Command::LoadMoreCommits {
        offset: 0,
        limit: 1,
        epoch: 0,
    }]);

    assert_eq!(runtime.deferred_reads.len(), 1);
    assert!(log.lock().expect("recording log lock").is_empty());

    runtime.write_commands_in_flight = 0;
    runtime.flush_deferred_reads_if_unblocked();

    let entries = wait_for_log_count(&log, 1);
    assert_eq!(entries, vec!["load-more:0:0".to_string()]);
}

#[test]
fn stale_read_results_are_dropped_after_a_queued_mutation() {
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let factory = RecordingFactory::with_blocking_refresh(started_tx, release_rx);
    let runtime_factory = factory.clone();
    let mut runtime = AsyncRuntime::new(
        AppContext::default(),
        move || runtime_factory.build(),
        terminal_size(),
    );

    runtime.process_commands(vec![Command::RefreshAll]);
    started_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("refresh should start");

    runtime.process_commands(vec![Command::StageFiles {
        paths: vec!["missing.txt".to_string()],
    }]);
    wait_for_state(&mut runtime, |runtime| {
        runtime.state.repo.status.last_error.is_some()
    });

    assert_eq!(runtime.state.repo.status.refresh_count, 0);
    release_tx.send(()).expect("refresh should release");
    wait_for_quiet_tick(&mut runtime);

    assert_eq!(runtime.state.repo.status.refresh_count, 0);
    assert!(
        runtime
            .state
            .repo
            .status
            .last_error
            .as_deref()
            .is_some_and(|error| error.contains("Failed to stage missing.txt"))
    );
}

#[test]
fn debounce_window_defers_and_coalesces_async_read_commands() {
    let factory = RecordingFactory::new();
    let log = Arc::clone(&factory.log);
    let runtime_factory = factory.clone();
    let mut runtime = AsyncRuntime::new(
        AppContext::default(),
        move || runtime_factory.build(),
        terminal_size(),
    )
    .with_debounce_window(Duration::from_millis(40));

    runtime.process_commands(vec![
        Command::RefreshFilesDetailsDiff {
            targets: vec![file_diff_target("old.txt")],
            truncated_from: None,
        },
        Command::RefreshFilesDetailsDiff {
            targets: vec![file_diff_target("latest.txt")],
            truncated_from: None,
        },
    ]);

    assert!(log.lock().expect("recording log lock").is_empty());
    std::thread::sleep(Duration::from_millis(70));
    runtime.tick();

    let entries = wait_for_log_count(&log, 1);
    assert_eq!(entries, vec!["details-diff:0:latest.txt".to_string()]);
}

#[test]
fn render_smoke_paths_use_current_state_without_dispatching_git() {
    let factory = RecordingFactory::new();
    let log = Arc::clone(&factory.log);
    let runtime_factory = factory.clone();
    let runtime = AsyncRuntime::new(
        AppContext::default(),
        move || runtime_factory.build(),
        terminal_size(),
    );

    let frame = runtime.render();
    let buffer = runtime.render_terminal_buffer();

    assert!(frame.as_text().contains("[1]"));
    assert!(buffer.area.width > 0);
    assert!(log.lock().expect("recording log lock").is_empty());
}

#[test]
fn empty_read_worker_pool_reports_refresh_failure() {
    let factory = RecordingFactory::new();
    let runtime_factory = factory.clone();
    let mut runtime = AsyncRuntime::new(
        AppContext::default(),
        move || runtime_factory.build(),
        terminal_size(),
    );
    runtime.read_command_txs.clear();

    runtime.process_commands(vec![Command::LoadMoreCommits {
        offset: 0,
        limit: 1,
        epoch: 0,
    }]);

    assert_eq!(
        runtime.state.repo.status.last_error.as_deref(),
        Some("Failed to refresh: async git read worker pool is empty")
    );
}

fn terminal_size() -> TerminalSize {
    TerminalSize {
        width: 100,
        height: 30,
    }
}

fn wait_for_log_count(log: &Arc<Mutex<Vec<String>>>, count: usize) -> Vec<String> {
    wait_for_entries(log, |entries| entries.len() >= count)
}

fn wait_for_entries(
    log: &Arc<Mutex<Vec<String>>>,
    done: impl Fn(&[String]) -> bool,
) -> Vec<String> {
    for _ in 0..100 {
        let entries = log.lock().expect("recording log lock").clone();
        if done(&entries) {
            return entries;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("timed out waiting for log entries");
}

fn wait_for_state(
    runtime: &mut AsyncRuntime<RecordingBackend>,
    done: impl Fn(&AsyncRuntime<RecordingBackend>) -> bool,
) {
    for _ in 0..100 {
        runtime.tick();
        if done(runtime) {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("timed out waiting for runtime state");
}

fn wait_for_quiet_tick(runtime: &mut AsyncRuntime<RecordingBackend>) {
    for _ in 0..10 {
        runtime.tick();
        std::thread::sleep(Duration::from_millis(10));
    }
}
