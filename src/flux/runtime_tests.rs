use super::action::{Action, DomainAction, SystemAction};
use super::dispatcher::Dispatcher;
use super::effects::{self, EffectRequest, RuntimeEvent};
use super::runtime::EffectExecutor;
use super::snapshot::AppStateSnapshotOwned;
use super::stores::test_support::MockRepo;
use crate::app::{App, AppState, Command, RefreshKind, SidePanel};
use crate::git::{
    BranchInfo, CommitInfo, DiffLine, DiffLineKind, FileEntry, GitError, GitStatus, StashInfo,
};
use crate::ui::widgets::file_tree::{FileTreeNode, FileTreeNodeStatus};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Notify;

type Gate<T> = Mutex<Option<Receiver<Result<T, GitError>>>>;

#[derive(Default)]
struct ControlledRepo {
    fetch: Gate<String>,
    stage: Gate<()>,
    fetch_started: Arc<Notify>,
    stage_started: Arc<Notify>,
    writes: Arc<Mutex<Vec<String>>>,
    diff_version: AtomicUsize,
    panic_on_stage: bool,
    fail_diff: bool,
}

macro_rules! delegate {
    ($($name:ident($($arg:ident: $ty:ty),*) -> $result:ty;)*) => {
        $(fn $name(&self, $($arg: $ty),*) -> $result {
            MockRepo.$name($($arg),*)
        })*
    };
}

impl crate::git::WorktreeRepository for ControlledRepo {
    delegate! { status() -> Result<GitStatus, GitError>; stage_paths(paths: &[PathBuf]) -> Result<(), GitError>; unstage_paths(paths: &[PathBuf]) -> Result<(), GitError>; discard_paths(paths: &[PathBuf]) -> Result<(), GitError>; }
    fn stage(&self, path: &Path) -> Result<(), GitError> {
        assert!(!self.panic_on_stage, "controlled Git failure");
        self.writes
            .lock()
            .unwrap()
            .push(format!("stage:{}", path.display()));
        self.stage_started.notify_one();
        let gate = self.stage.lock().unwrap().take();
        if let Some(rx) = gate {
            rx.recv().map_err(|e| GitError::Git2(e.to_string()))?
        } else {
            Ok(())
        }
    }

    fn unstage(&self, path: &Path) -> Result<(), GitError> {
        self.writes
            .lock()
            .unwrap()
            .push(format!("unstage:{}", path.display()));
        Ok(())
    }
}

impl crate::git::PatchRepository for ControlledRepo {
    delegate! { diff_staged(path: &Path) -> Result<Vec<DiffLine>, GitError>; diff_untracked(path: &Path) -> Result<Vec<DiffLine>, GitError>; commit_diff_scoped(oid: &str, path: Option<&Path>) -> Result<Vec<DiffLine>, GitError>; }
    fn diff_unstaged(&self, path: &Path) -> Result<Vec<DiffLine>, GitError> {
        if self.fail_diff {
            return Err(GitError::InvalidState);
        }
        let version = self.diff_version.fetch_add(1, Ordering::SeqCst) + 1;
        Ok(vec![DiffLine {
            kind: DiffLineKind::Context,
            content: format!("{}:{version}", path.display()),
        }])
    }
}

impl crate::git::HistoryRepository for ControlledRepo {
    delegate! { commits(limit: usize) -> Result<Vec<CommitInfo>, GitError>; commit_files(oid: &str) -> Result<Vec<FileEntry>, GitError>; }
}

impl crate::git::ReferenceRepository for ControlledRepo {
    delegate! { branches() -> Result<Vec<BranchInfo>, GitError>; create_branch(name: &str) -> Result<(), GitError>; checkout_branch(name: &str) -> Result<(), GitError>; delete_branch(name: &str) -> Result<(), GitError>; }
    fn fetch_default_async(&self) -> Result<Receiver<Result<String, GitError>>, GitError> {
        self.fetch_started.notify_one();
        self.fetch
            .lock()
            .unwrap()
            .take()
            .ok_or(GitError::InvalidState)
    }
}

impl crate::git::StashRepository for ControlledRepo {
    delegate! { stashes() -> Result<Vec<StashInfo>, GitError>; stash_files(index: usize) -> Result<Vec<FileEntry>, GitError>; stash_diff(index: usize, path: Option<&Path>) -> Result<Vec<DiffLine>, GitError>; stash_push_paths(paths: &[PathBuf], message: &str) -> Result<usize, GitError>; stash_apply(index: usize) -> Result<(), GitError>; stash_pop(index: usize) -> Result<(), GitError>; stash_drop(index: usize) -> Result<(), GitError>; }
}

impl crate::git::CommitRepository for ControlledRepo {
    delegate! { commit(message: &str) -> Result<String, GitError>; }
}

impl crate::git::OperationRepository for ControlledRepo {}

fn fixture(repo: ControlledRepo) -> App {
    let mut app = App::from_repo(Box::new(repo)).unwrap();
    app.state.ui.files.tree_nodes = ["a.txt", "b.txt"]
        .into_iter()
        .map(|path| FileTreeNode {
            path: path.into(),
            status: FileTreeNodeStatus::Unstaged(crate::git::FileStatus::Modified),
            depth: 0,
            is_dir: false,
            is_expanded: false,
        })
        .collect();
    app.state.ui.files.panel.list_state.select(Some(0));
    app.state.ui.dirty.clear();
    app
}

fn dispatch(
    dispatcher: &mut Dispatcher,
    state: &mut AppState,
    action: Action,
) -> Vec<EffectRequest> {
    let envelope = dispatcher.next_envelope(action);
    dispatcher
        .dispatch(state, envelope)
        .commands
        .into_iter()
        .filter_map(|command| match command {
            Command::Effect(effect) => Some(effect),
            _ => None,
        })
        .collect()
}

fn diff_request(app: &mut App) -> EffectRequest {
    app.state.reload_diff_now();
    app.state
        .take_effects()
        .into_iter()
        .find(|request| {
            matches!(
                request,
                EffectRequest::LoadBackground {
                    load: crate::app::BackgroundLoad::Diff { .. },
                    ..
                }
            )
        })
        .expect("diff request")
}

async fn completion(executor: &mut EffectExecutor) -> Vec<Action> {
    tokio::time::timeout(Duration::from_secs(3), executor.next_completed())
        .await
        .expect("independent operation must complete while another is blocked")
        .actions
}

#[tokio::test]
async fn test_file_effect_changes_state_only_after_completion_is_dispatched() {
    let mut app = fixture(ControlledRepo::default());
    let mut dispatcher = Dispatcher::with_default_stores();
    let requests = dispatch(
        &mut dispatcher,
        &mut app.state,
        Action::Domain(DomainAction::StageFile("a.txt".into())),
    );
    assert!(app.state.command_log.is_empty());
    assert!(!app.state.ui.dirty.is_dirty());
    assert!(!app.state.has_pending_refresh_work());
    let actions = effects::run(requests.into_iter().next().unwrap(), &app.runtime).await;
    assert!(app.state.command_log.is_empty());
    assert!(!app.state.has_pending_refresh_work());
    for action in actions {
        dispatch(&mut dispatcher, &mut app.state, action);
    }
    assert!(app.state.has_pending_refresh_work());
    let snapshot = AppStateSnapshotOwned::from_app(&app.state);
    assert_eq!(snapshot.command_log.last().unwrap().command, "staged a.txt");
    assert!(app.state.ui.dirty.is_dirty());
}

#[tokio::test]
async fn test_background_diff_changes_snapshot_only_after_completion_event() {
    let mut app = fixture(ControlledRepo::default());
    let mut dispatcher = Dispatcher::with_default_stores();
    let request = diff_request(&mut app);
    let actions = effects::run(request, &app.runtime).await;
    assert!(app.state.git.current_diff.is_empty());
    assert!(app.state.has_pending_refresh_work());
    app.state.ui.dirty.clear();
    for action in actions {
        dispatch(&mut dispatcher, &mut app.state, action);
    }
    assert_eq!(
        AppStateSnapshotOwned::from_app(&app.state).current_diff[0].content,
        "a.txt:1"
    );
    assert!(!app.state.has_pending_refresh_work());
    assert!(app.state.ui.dirty.is_dirty());
    assert!(!app.state.ui.dirty.main_content());
}

#[tokio::test]
async fn test_slow_fetch_does_not_block_navigation_diff_or_file_write() {
    let (fetch_tx, fetch_rx) = mpsc::channel();
    let repo = ControlledRepo {
        fetch: Mutex::new(Some(fetch_rx)),
        ..Default::default()
    };
    let started = repo.fetch_started.clone();
    let mut app = fixture(repo);
    let mut dispatcher = Dispatcher::with_default_stores();
    let mut executor = EffectExecutor::new(app.runtime.clone());
    for request in dispatch(
        &mut dispatcher,
        &mut app.state,
        Action::Domain(DomainAction::FetchRemote),
    ) {
        executor.enqueue(request);
    }
    tokio::time::timeout(Duration::from_secs(3), started.notified())
        .await
        .unwrap();
    dispatch(
        &mut dispatcher,
        &mut app.state,
        Action::Domain(DomainAction::ListDown),
    );
    assert_eq!(app.state.ui.files.panel.list_state.selected(), Some(1));
    executor.enqueue(diff_request(&mut app));
    executor.enqueue(EffectRequest::StageFile("a.txt".into()));
    let mut got_diff = false;
    let mut got_stage = false;
    for _ in 0..2 {
        for action in completion(&mut executor).await {
            got_diff |= matches!(action, Action::System(SystemAction::BackgroundCompleted(_)));
            got_stage |= matches!(
                action,
                Action::Domain(DomainAction::StageFileFinished { .. })
            );
        }
    }
    assert!(got_diff && got_stage);
    assert!(app.state.ui.branches.is_fetching_remote);
    fetch_tx.send(Ok("origin".into())).unwrap();
    for action in completion(&mut executor).await {
        dispatch(&mut dispatcher, &mut app.state, action);
    }
    assert!(!app.state.ui.branches.is_fetching_remote);
}

#[tokio::test]
async fn test_local_writes_are_serial_while_reads_continue() {
    let (stage_tx, stage_rx) = mpsc::channel();
    let repo = ControlledRepo {
        stage: Mutex::new(Some(stage_rx)),
        ..Default::default()
    };
    let started = repo.stage_started.clone();
    let writes = repo.writes.clone();
    let mut app = fixture(repo);
    let mut executor = EffectExecutor::new(app.runtime.clone());
    executor.enqueue(EffectRequest::StageFile("a.txt".into()));
    tokio::time::timeout(Duration::from_secs(3), started.notified())
        .await
        .unwrap();
    executor.enqueue(EffectRequest::UnstageFile("a.txt".into()));
    executor.enqueue(diff_request(&mut app));
    assert!(matches!(
        completion(&mut executor).await.as_slice(),
        [Action::System(SystemAction::BackgroundCompleted(_))]
    ));
    assert_eq!(*writes.lock().unwrap(), ["stage:a.txt"]);
    stage_tx.send(Ok(())).unwrap();
    assert!(matches!(
        completion(&mut executor).await.as_slice(),
        [Action::Domain(DomainAction::StageFileFinished {
            result: Ok(()),
            ..
        })]
    ));
    assert!(matches!(
        completion(&mut executor).await.as_slice(),
        [Action::Domain(DomainAction::UnstageFileFinished {
            result: Ok(()),
            ..
        })]
    ));
    assert_eq!(*writes.lock().unwrap(), ["stage:a.txt", "unstage:a.txt"]);
}

#[tokio::test]
async fn test_old_diff_completion_cannot_overwrite_new_selection() {
    let mut app = fixture(ControlledRepo::default());
    let mut dispatcher = Dispatcher::with_default_stores();
    let old = effects::run(diff_request(&mut app), &app.runtime).await;
    dispatch(
        &mut dispatcher,
        &mut app.state,
        Action::Domain(DomainAction::ListDown),
    );
    let new = effects::run(diff_request(&mut app), &app.runtime).await;
    for action in new {
        dispatch(&mut dispatcher, &mut app.state, action);
    }
    for action in old {
        dispatch(&mut dispatcher, &mut app.state, action);
    }
    assert_eq!(app.state.git.current_diff[0].content, "b.txt:2");
    assert!(!app.state.has_pending_refresh_work());
}

#[tokio::test]
async fn test_refresh_rejects_old_diff_even_when_path_is_unchanged() {
    let mut app = fixture(ControlledRepo::default());
    let mut dispatcher = Dispatcher::with_default_stores();
    let old = effects::run(diff_request(&mut app), &app.runtime).await;
    app.state.request_refresh(RefreshKind::StatusOnly);
    let new = effects::run(diff_request(&mut app), &app.runtime).await;
    for action in new {
        dispatch(&mut dispatcher, &mut app.state, action);
    }
    for action in old {
        dispatch(&mut dispatcher, &mut app.state, action);
    }
    assert_eq!(app.state.git.current_diff[0].content, "a.txt:2");
}

#[tokio::test]
async fn test_cancelled_diff_is_rejected_without_replacement_request() {
    let mut app = fixture(ControlledRepo::default());
    let mut dispatcher = Dispatcher::with_default_stores();
    let old = effects::run(diff_request(&mut app), &app.runtime).await;
    dispatch(
        &mut dispatcher,
        &mut app.state,
        Action::Domain(DomainAction::ListDown),
    );
    for action in old {
        dispatch(&mut dispatcher, &mut app.state, action);
    }
    assert!(app.state.git.current_diff.is_empty());
    assert!(app.state.has_pending_diff_reload());
}

#[tokio::test]
async fn test_selection_completion_is_applied_after_leaving_files_panel() {
    let mut app = fixture(ControlledRepo::default());
    let mut dispatcher = Dispatcher::with_default_stores();
    app.state.ui.active_panel = SidePanel::LocalBranches;
    let actions = effects::run(
        EffectRequest::ToggleStageSelection {
            stage: vec!["a.txt".into()],
            unstage: Vec::new(),
        },
        &app.runtime,
    )
    .await;
    for action in actions {
        dispatch(&mut dispatcher, &mut app.state, action);
    }
    assert!(app.state.has_pending_refresh_work());
    assert!(app
        .state
        .command_log
        .last()
        .unwrap()
        .command
        .contains("selection toggled"));
}

#[test]
fn test_state_event_does_not_wait_for_executor_slots() {
    let app = fixture(ControlledRepo::default());
    let mut executor = EffectExecutor::new(app.runtime);
    let actions = executor.enqueue(EffectRequest::ReloadDiffNow);
    assert!(matches!(
        actions.as_slice(),
        [Action::Domain(DomainAction::Runtime(
            RuntimeEvent::ReloadDiffNow
        ))]
    ));
    assert!(!executor.has_jobs());
}

#[tokio::test]
async fn test_worker_panic_emits_failure_and_releases_write_lane() {
    let app = fixture(ControlledRepo {
        panic_on_stage: true,
        ..Default::default()
    });
    let mut executor = EffectExecutor::new(app.runtime);
    executor.enqueue(EffectRequest::StageFile("a.txt".into()));
    executor.enqueue(EffectRequest::UnstageFile("a.txt".into()));
    assert!(matches!(completion(&mut executor).await.as_slice(),
        [Action::Domain(DomainAction::StageFileFinished { path, result: Err(_) })] if path == Path::new("a.txt")));
    assert!(matches!(
        completion(&mut executor).await.as_slice(),
        [Action::Domain(DomainAction::UnstageFileFinished {
            result: Ok(()),
            ..
        })]
    ));
    assert!(!executor.has_jobs());
}

#[tokio::test]
async fn test_background_failure_clears_loading_when_event_is_reduced() {
    let mut app = fixture(ControlledRepo {
        fail_diff: true,
        ..Default::default()
    });
    let mut dispatcher = Dispatcher::with_default_stores();
    let request = diff_request(&mut app);
    assert!(AppStateSnapshotOwned::from_app(&app.state).diff_loading);
    let actions = effects::run(request, &app.runtime).await;
    assert!(app.state.has_pending_refresh_work());
    for action in actions {
        dispatch(&mut dispatcher, &mut app.state, action);
    }
    assert!(!app.state.has_pending_refresh_work());
    assert!(!AppStateSnapshotOwned::from_app(&app.state).diff_loading);
    assert!(!app.state.command_log.last().unwrap().success);
}

#[tokio::test]
async fn test_cancelled_revision_load_cannot_reopen_tree() {
    let mut app = fixture(ControlledRepo::default());
    let mut dispatcher = Dispatcher::with_default_stores();
    app.state.ui.active_panel = SidePanel::Commits;
    app.state.commit_open_tree_or_toggle_dir().unwrap();
    let request = app.state.take_effects().pop().unwrap();
    let actions = effects::run(request, &app.runtime).await;
    app.state.commit_close_tree();
    for action in actions {
        dispatch(&mut dispatcher, &mut app.state, action);
    }
    assert!(!app.state.ui.commits.tree_mode.active);
    assert!(app.state.ui.commits.tree_mode.selected_source.is_none());
    assert!(!app.state.has_pending_refresh_work());
}

#[test]
fn test_shutdown_does_not_wait_for_blocked_git_operation() {
    let (stage_tx, stage_rx) = mpsc::channel();
    let repo = ControlledRepo {
        stage: Mutex::new(Some(stage_rx)),
        ..Default::default()
    };
    let started = repo.stage_started.clone();
    let app = fixture(repo);
    let (exited_tx, exited_rx) = mpsc::channel();
    let thread = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let mut executor = EffectExecutor::new(app.runtime);
            executor.enqueue(EffectRequest::StageFile("a.txt".into()));
            tokio::time::timeout(Duration::from_secs(3), started.notified())
                .await
                .unwrap();
            // Dropping the executor aborts completion waits, including queued work.
        });
        drop(runtime);
        exited_tx.send(()).unwrap();
    });
    let exited = exited_rx.recv_timeout(Duration::from_secs(3));
    stage_tx.send(Ok(())).unwrap();
    thread.join().unwrap();
    exited.expect("shutdown must finish before the blocked Git operation is released");
}

#[tokio::test]
async fn test_git_stage_unstage_roundtrip_through_completion_events() {
    let dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();
    let file = dir.path().join("file.txt");
    std::fs::write(&file, "before\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("file.txt")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let signature = git2::Signature::now("Test", "test@example.com").unwrap();
    repo.commit(Some("HEAD"), &signature, &signature, "initial", &tree, &[])
        .unwrap();
    std::fs::write(&file, "after\n").unwrap();
    let mut app = App::from_repo(Box::new(
        crate::git::Git2Repository::open(dir.path()).unwrap(),
    ))
    .unwrap();
    assert_eq!(app.state.git.status.unstaged.len(), 1);
    for (request, staged) in [
        (EffectRequest::StageFile("file.txt".into()), true),
        (EffectRequest::UnstageFile("file.txt".into()), false),
    ] {
        let actions = effects::run(request, &app.runtime).await;
        assert_eq!(app.state.git.status.staged.is_empty(), staged);
        super::test_runtime::drive_actions(&mut app, actions);
        super::test_runtime::drive_actions(
            &mut app,
            [Action::Domain(DomainAction::Runtime(
                RuntimeEvent::ProcessBackgroundLoads,
            ))],
        );
        assert_eq!(!app.state.git.status.staged.is_empty(), staged);
        assert_eq!(app.state.git.status.unstaged.is_empty(), staged);
    }
}
