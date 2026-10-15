use super::action::RuntimeEvent;
use super::action::{Action, DomainAction, SystemAction};
use super::dispatcher::Dispatcher;
use super::effects::{self, EffectRequest};
use super::runtime::{route_command, EffectExecutor};
use super::snapshot::AppStateSnapshotOwned;
use super::stores::test_support::MockRepo;
use crate::app::{App, AppState, Command, RefreshKind, SidePanel};
use crate::file_tree::{FileTreeNode, FileTreeNodeStatus};
use crate::git::{
    BranchInfo, CommitInfo, DiffLine, DiffLineKind, FileEntry, GitError, GitStatus, StashInfo,
};
use std::collections::VecDeque;
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
    fetch_queue: Mutex<VecDeque<Receiver<Result<String, GitError>>>>,
    fetch_count: Arc<AtomicUsize>,
    stage: Gate<()>,
    fetch_started: Arc<Notify>,
    stage_started: Arc<Notify>,
    writes: Arc<Mutex<Vec<String>>>,
    diff_version: AtomicUsize,
    panic_on_stage: bool,
    fail_diff: bool,
    diff_error: Option<GitError>,
    control_error: Option<GitError>,
    reload_error: Option<GitError>,
    control_called: std::sync::atomic::AtomicBool,
    unstage_paths_error: Option<GitError>,
}

macro_rules! delegate {
    ($($name:ident($($arg:ident: $ty:ty),*) -> $result:ty;)*) => {
        $(fn $name(&self, $($arg: $ty),*) -> $result {
            MockRepo.$name($($arg),*)
        })*
    };
}

impl crate::git::WorktreeRepository for ControlledRepo {
    delegate! { status() -> Result<GitStatus, GitError>; discard_paths(paths: &[PathBuf]) -> Result<(), GitError>; }
    fn stage_paths(&self, _: &[PathBuf]) -> Result<(), GitError> {
        self.writes.lock().unwrap().push("stage_paths".into());
        Ok(())
    }

    fn unstage_paths(&self, _: &[PathBuf]) -> Result<(), GitError> {
        self.writes.lock().unwrap().push("unstage_paths".into());
        self.unstage_paths_error.clone().map_or(Ok(()), Err)
    }
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
        if let Some(error) = &self.diff_error {
            return Err(error.clone());
        }
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
        self.fetch_count.fetch_add(1, Ordering::SeqCst);
        self.fetch_started.notify_one();
        if let Some(receiver) = self.fetch_queue.lock().unwrap().pop_front() {
            return Ok(receiver);
        }
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

impl crate::git::OperationRepository for ControlledRepo {
    fn control_operation(
        &self,
        _: crate::git::operation::OperationControl,
    ) -> Result<(), GitError> {
        self.control_called.store(true, Ordering::SeqCst);
        self.control_error.clone().map_or(Ok(()), Err)
    }

    fn operation_snapshot(&self) -> Result<crate::git::operation::RepositoryOperation, GitError> {
        if self.control_called.load(Ordering::SeqCst) {
            if let Some(error) = &self.reload_error {
                return Err(error.clone());
            }
        }
        Ok(Default::default())
    }
}

#[path = "git_error_tests.rs"]
mod git_error_tests;

fn fixture(repo: ControlledRepo) -> App {
    let mut app = App::from_repo(Box::new(repo)).unwrap();
    app.state.files.test_view_mut().tree_nodes = ["a.txt", "b.txt"]
        .into_iter()
        .map(|path| FileTreeNode {
            path: path.into(),
            status: FileTreeNodeStatus::Unstaged(crate::git::FileStatus::Modified),
            depth: 0,
            is_dir: false,
            is_expanded: false,
        })
        .collect();
    app.state.files.test_view_mut().panel.select(Some(0));
    app.state.ui.dirty.clear();
    app
}

fn dispatch_commands(
    dispatcher: &mut Dispatcher,
    state: &mut AppState,
    action: Action,
) -> Vec<Command> {
    let envelope = dispatcher.next_envelope(action);
    dispatcher.dispatch(state, envelope).commands
}

fn dispatch(
    dispatcher: &mut Dispatcher,
    state: &mut AppState,
    action: Action,
) -> Vec<EffectRequest> {
    dispatch_commands(dispatcher, state, action)
        .into_iter()
        .filter_map(|command| match command {
            Command::Effect(effect) => Some(effect),
            _ => None,
        })
        .collect()
}

fn diff_request(app: &mut App) -> EffectRequest {
    app.reload_diff_now();
    app.take_effects()
        .into_iter()
        .find(|request| {
            matches!(
                request,
                EffectRequest::LoadBackground {
                    request: crate::background::TaskRequest {
                        load: crate::app::BackgroundLoad::Diff { .. },
                        ..
                    }
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
    assert!(app.state.diff.document().lines.is_empty());
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
    assert_eq!(app.state.files.view().panel.selected(), Some(1));
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
    assert!(app.state.revisions.branches.is_fetching_remote);
    fetch_tx.send(Ok("origin".into())).unwrap();
    for action in completion(&mut executor).await {
        dispatch(&mut dispatcher, &mut app.state, action);
    }
    assert!(!app.state.revisions.branches.is_fetching_remote);
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
    let accepted = AppStateSnapshotOwned::from_app(&app.state);
    for action in old {
        dispatch(&mut dispatcher, &mut app.state, action);
    }
    let after_stale = AppStateSnapshotOwned::from_app(&app.state);
    assert!(accepted.current_diff.ptr_eq(&after_stale.current_diff));
    assert_eq!(accepted.current_diff[0].content, "b.txt:2");
    assert_eq!(app.state.diff.document().lines[0].content, "b.txt:2");
    assert!(!app.state.has_pending_refresh_work());
}

#[tokio::test]
async fn test_unified_output_preserves_write_order_and_state_event_roundtrip() {
    let (stage_tx, stage_rx) = mpsc::channel();
    let repo = ControlledRepo {
        stage: Mutex::new(Some(stage_rx)),
        ..Default::default()
    };
    let started = repo.stage_started.clone();
    let writes = repo.writes.clone();
    let mut app = fixture(repo);
    let mut initial = crate::flux::command_output::CommandOutput::default();
    initial.effect(EffectRequest::StageFile("coordinator".into()));
    initial.action(DomainAction::Runtime(RuntimeEvent::ProcessBackgroundLoads));
    let mut dispatcher = Dispatcher::with_initial_output(initial);
    let requests = dispatch_commands(
        &mut dispatcher,
        &mut app.state,
        Action::Domain(DomainAction::StageFile("store".into())),
    );
    assert!(
        matches!(requests.as_slice(), [Command::Effect(EffectRequest::StageFile(store)), Command::Effect(EffectRequest::StageFile(coordinator)), Command::Sync(DomainAction::Runtime(RuntimeEvent::ProcessBackgroundLoads))]
        if store.to_str() == Some("store") && coordinator.to_str() == Some("coordinator"))
    );
    let mut executor = EffectExecutor::new(app.runtime.clone());
    let mut immediate = Vec::new();
    for request in requests {
        immediate.extend(route_command(request, &mut executor));
    }
    assert!(matches!(
        immediate.as_slice(),
        [Action::Domain(DomainAction::Runtime(
            RuntimeEvent::ProcessBackgroundLoads
        ))]
    ));
    // Returning the event does not apply it: the dispatcher is still the only
    // owner of state transitions, and the state event does not wait for a write.
    assert!(!app.state.has_pending_refresh_work());
    tokio::time::timeout(Duration::from_secs(3), started.notified())
        .await
        .unwrap();
    assert_eq!(*writes.lock().unwrap(), ["stage:store"]);
    stage_tx.send(Ok(())).unwrap();
    assert!(
        matches!(completion(&mut executor).await.as_slice(), [Action::Domain(DomainAction::StageFileFinished { path, result: Ok(()) })] if path.to_str() == Some("store"))
    );
    assert!(
        matches!(completion(&mut executor).await.as_slice(), [Action::Domain(DomainAction::StageFileFinished { path, result: Ok(()) })] if path.to_str() == Some("coordinator"))
    );
    assert_eq!(
        *writes.lock().unwrap(),
        ["stage:store", "stage:coordinator"]
    );
}

#[tokio::test]
async fn test_refresh_rejects_old_diff_even_when_path_is_unchanged() {
    let mut app = fixture(ControlledRepo::default());
    let mut dispatcher = Dispatcher::with_default_stores();
    let old = effects::run(diff_request(&mut app), &app.runtime).await;
    app.request_refresh(RefreshKind::StatusOnly);
    let new = effects::run(diff_request(&mut app), &app.runtime).await;
    for action in new {
        dispatch(&mut dispatcher, &mut app.state, action);
    }
    let accepted = AppStateSnapshotOwned::from_app(&app.state);
    for action in old {
        dispatch(&mut dispatcher, &mut app.state, action);
    }
    let after_stale = AppStateSnapshotOwned::from_app(&app.state);
    assert!(accepted.current_diff.ptr_eq(&after_stale.current_diff));
    assert_eq!(accepted.current_diff[0].content, "a.txt:2");
    assert_eq!(app.state.diff.document().lines[0].content, "a.txt:2");
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
    assert!(app.state.diff.document().lines.is_empty());
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
    let actions = route_command(Command::runtime(RuntimeEvent::ReloadDiffNow), &mut executor);
    assert!(matches!(
        actions.as_slice(),
        [Action::Domain(DomainAction::Runtime(
            RuntimeEvent::ReloadDiffNow
        ))]
    ));
    assert!(!executor.has_jobs());
}

#[test]
fn test_all_state_events_route_to_actions_in_order_without_io_jobs() {
    let app = fixture(ControlledRepo::default());
    let events = vec![
        RuntimeEvent::ProcessBackgroundLoads,
        RuntimeEvent::FlushPendingRefresh { log_success: false },
        RuntimeEvent::FlushPendingRefresh { log_success: true },
        RuntimeEvent::FlushPendingDiffReload,
        RuntimeEvent::EnsureCommitsLoadedForActivePanel,
        RuntimeEvent::ReloadDiffNow,
        RuntimeEvent::RevisionOpenTreeOrToggleDir,
    ];
    let mut executor = EffectExecutor::new(app.runtime.clone());
    let actions: Vec<_> = events
        .iter()
        .cloned()
        .flat_map(|event| route_command(Command::runtime(event), &mut executor))
        .collect();
    let received: Vec<_> = actions
        .into_iter()
        .map(|action| {
            let Action::Domain(DomainAction::Runtime(event)) = action else {
                panic!("state commands must return actions")
            };
            event
        })
        .collect();
    assert_eq!(received, events);
    assert!(!executor.has_jobs());
    assert!(
        !app.state.has_pending_refresh_work(),
        "routing must not apply state transitions"
    );
    assert!(!app.state.has_pending_diff_reload());
    assert!(app.state.command_log.is_empty());
}

#[tokio::test]
async fn test_state_action_runs_while_all_read_slots_and_write_lane_are_blocked() {
    let (stage_tx, stage_rx) = mpsc::channel();
    let mut fetch_senders = Vec::new();
    let mut fetch_receivers = VecDeque::new();
    for _ in 0..6 {
        let (tx, rx) = mpsc::channel();
        fetch_senders.push(tx);
        fetch_receivers.push_back(rx);
    }
    let repo = ControlledRepo {
        stage: Mutex::new(Some(stage_rx)),
        fetch_queue: Mutex::new(fetch_receivers),
        ..Default::default()
    };
    let reads_started = repo.fetch_started.clone();
    let read_count = repo.fetch_count.clone();
    let write_started = repo.stage_started.clone();
    let writes = repo.writes.clone();
    let mut app = fixture(repo);
    let mut executor = EffectExecutor::new(app.runtime.clone());
    for _ in 0..6 {
        executor.enqueue(EffectRequest::FetchRemote);
    }
    executor.enqueue(EffectRequest::StageFile("held".into()));
    executor.enqueue(EffectRequest::UnstageFile("queued".into()));
    tokio::time::timeout(Duration::from_secs(3), async {
        while read_count.load(Ordering::SeqCst) < 6 {
            reads_started.notified().await;
        }
        write_started.notified().await;
    })
    .await
    .unwrap();

    let actions = route_command(Command::runtime(RuntimeEvent::ReloadDiffNow), &mut executor);
    assert!(!app.state.has_pending_refresh_work());
    let mut dispatcher = Dispatcher::with_default_stores();
    let commands = dispatch_commands(
        &mut dispatcher,
        &mut app.state,
        actions.into_iter().next().unwrap(),
    );
    assert!(
        app.state.diff.is_loading(),
        "state transition must run without an I/O slot"
    );
    assert!(matches!(
        commands.as_slice(),
        [Command::Effect(EffectRequest::LoadBackground {
            request: crate::background::TaskRequest {
                load: crate::app::BackgroundLoad::Diff { .. },
                ..
            }
        })]
    ));
    for command in commands {
        assert!(route_command(command, &mut executor).is_empty());
    }
    assert!(
        app.state.diff.document().lines.is_empty(),
        "the new read must wait for a slot"
    );
    assert_eq!(*writes.lock().unwrap(), ["stage:held"]);

    for sender in fetch_senders {
        sender.send(Ok("origin".into())).unwrap();
    }
    let mut diff_applied = false;
    for _ in 0..7 {
        for action in completion(&mut executor).await {
            if matches!(action, Action::System(SystemAction::BackgroundCompleted(_))) {
                dispatch(&mut dispatcher, &mut app.state, action);
                diff_applied = true;
            }
        }
    }
    assert!(diff_applied);
    assert_eq!(app.state.diff.document().lines[0].content, "a.txt:1");
    assert_eq!(
        *writes.lock().unwrap(),
        ["stage:held"],
        "reads remain independent of the blocked write"
    );
    stage_tx.send(Ok(())).unwrap();
    assert!(matches!(
        completion(&mut executor).await.as_slice(),
        [Action::Domain(DomainAction::StageFileFinished { .. })]
    ));
    assert!(matches!(
        completion(&mut executor).await.as_slice(),
        [Action::Domain(DomainAction::UnstageFileFinished { .. })]
    ));
    assert_eq!(*writes.lock().unwrap(), ["stage:held", "unstage:queued"]);
    assert!(!executor.has_jobs());
}

#[test]
fn test_state_followups_keep_existing_and_later_actions_in_fifo_order() {
    let mut app = fixture(ControlledRepo::default());
    app.input.mode = Some(crate::app::InputMode::CreateBranch);
    let mut pending = VecDeque::from([Action::Domain(DomainAction::InputChar('a'))]);
    let mut executor = EffectExecutor::new(app.runtime.clone());
    pending.extend(route_command(
        Command::runtime(RuntimeEvent::FlushPendingRefresh { log_success: true }),
        &mut executor,
    ));
    pending.extend(route_command(
        Command::Sync(DomainAction::InputChar('b')),
        &mut executor,
    ));
    assert!(app.input.buffer.is_empty());
    assert!(app.command_log.is_empty());
    let mut dispatcher = Dispatcher::with_default_stores();
    let mut versions = Vec::new();
    while let Some(action) = pending.pop_front() {
        let envelope = dispatcher.next_envelope(action);
        let result = dispatcher.dispatch(&mut app.state, envelope);
        assert!(result.commands.is_empty());
        versions.push(result.state_version);
        if versions.len() == 1 {
            assert_eq!(app.input.buffer, "a");
            assert!(app.command_log.is_empty());
        }
        if versions.len() == 2 {
            assert_eq!(app.input.buffer, "a");
            assert_eq!(app.command_log.last().unwrap().command, "refresh");
        }
    }
    assert_eq!(versions, [1, 2, 3]);
    assert_eq!(app.input.buffer, "ab");
    assert!(!executor.has_jobs());
}

#[test]
fn test_refresh_schedules_io_only_after_its_followup_action_is_dispatched() {
    let mut app = fixture(ControlledRepo::default());
    let mut dispatcher = Dispatcher::with_default_stores();
    let mut executor = EffectExecutor::new(app.runtime.clone());
    let commands = dispatch_commands(
        &mut dispatcher,
        &mut app.state,
        Action::Domain(DomainAction::Refresh),
    );
    assert!(matches!(
        commands.as_slice(),
        [Command::Sync(DomainAction::Runtime(
            RuntimeEvent::ProcessBackgroundLoads
        ))]
    ));
    assert!(!executor.has_jobs());
    let action = route_command(commands.into_iter().next().unwrap(), &mut executor)
        .pop()
        .unwrap();
    assert!(!executor.has_jobs());
    let reads = dispatch_commands(&mut dispatcher, &mut app.state, action);
    assert!(matches!(
        reads.first(),
        Some(Command::Effect(EffectRequest::LoadBackground {
            request: crate::background::TaskRequest {
                load: crate::app::BackgroundLoad::Status { fast: true, .. },
                ..
            }
        }))
    ));
    assert!(reads
        .iter()
        .all(|command| matches!(command, Command::Effect(_))));
    assert!(
        !executor.has_jobs(),
        "only routing the actual read starts I/O"
    );
}

#[test]
fn test_revision_open_remains_a_followup_action_before_its_repository_read() {
    let mut app = branch_fixture();
    let mut dispatcher = Dispatcher::with_default_stores();
    let mut executor = EffectExecutor::new(app.runtime.clone());
    let commands = dispatch_commands(
        &mut dispatcher,
        &mut app.state,
        Action::Domain(DomainAction::RevisionOpenTreeOrToggleDir),
    );
    assert!(matches!(
        commands.as_slice(),
        [Command::Sync(DomainAction::Runtime(
            RuntimeEvent::RevisionOpenTreeOrToggleDir
        ))]
    ));
    assert!(!app.revisions.view().branches.commits_subview_active);
    let action = route_command(commands.into_iter().next().unwrap(), &mut executor)
        .pop()
        .unwrap();
    assert!(!app.revisions.view().branches.commits_subview_active);
    let reads = dispatch_commands(&mut dispatcher, &mut app.state, action);
    assert!(app.revisions.view().branches.commits_subview_active);
    assert!(app.revisions.view().branches.commits_subview_loading);
    assert!(
        matches!(reads.first(), Some(Command::Effect(EffectRequest::LoadBackground { request: crate::background::TaskRequest { load: crate::app::BackgroundLoad::BranchCommits { branch, .. }, .. } })) if branch == "main")
    );
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
        [Action::Domain(DomainAction::StageFileFinished { path, result: Err(error) })]
        if path == Path::new("a.txt") && error.category() == crate::git::GitErrorCategory::Worker));
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
    app.commit_open_tree_or_toggle_dir();
    let request = app.take_effects().pop().unwrap();
    let actions = effects::run(request, &app.runtime).await;
    app.commit_close_tree();
    for action in actions {
        dispatch(&mut dispatcher, &mut app.state, action);
    }
    assert!(!app.state.revisions.commits.tree_mode.active);
    assert!(app
        .state
        .revisions
        .commits
        .tree_mode
        .selected_source
        .is_none());
    assert!(!app.state.has_pending_refresh_work());
}

async fn open_revision_request(app: &mut App, dispatcher: &mut Dispatcher) -> EffectRequest {
    let commands = dispatch_commands(
        dispatcher,
        &mut app.state,
        Action::Domain(DomainAction::RevisionOpenTreeOrToggleDir),
    );
    let mut reads = Vec::new();
    let mut executor = EffectExecutor::new(app.runtime.clone());
    for command in commands {
        for action in route_command(command, &mut executor) {
            reads.extend(dispatch(dispatcher, &mut app.state, action));
        }
    }
    reads
        .into_iter()
        .find(|request| {
            matches!(
                request,
                EffectRequest::LoadBackground {
                    request: crate::background::TaskRequest {
                        load: crate::app::BackgroundLoad::BranchCommits { .. }
                            | crate::app::BackgroundLoad::CommitFiles { .. }
                            | crate::app::BackgroundLoad::StashFiles { .. },
                        ..
                    }
                }
            )
        })
        .expect("revision input must schedule an owned read")
}

fn branch_fixture() -> App {
    let mut app = fixture(ControlledRepo::default());
    app.state.revisions.update(
        crate::app::revisions::RevisionAction::ReplaceBranches(
            vec![BranchInfo {
                name: "main".into(),
                is_current: true,
            }]
            .into(),
        ),
        SidePanel::Files,
    );
    app.state.ui.active_panel = SidePanel::LocalBranches;
    app
}

#[tokio::test]
async fn test_reopened_branch_subview_rejects_superseded_generation_for_same_source() {
    let mut app = branch_fixture();
    let mut dispatcher = Dispatcher::with_default_stores();
    let old = open_revision_request(&mut app, &mut dispatcher).await;
    let old_actions = effects::run(old, &app.runtime).await;
    dispatch(
        &mut dispatcher,
        &mut app.state,
        Action::Domain(DomainAction::RevisionCloseTree),
    );
    let current = open_revision_request(&mut app, &mut dispatcher).await;
    for action in old_actions {
        dispatch(&mut dispatcher, &mut app.state, action);
    }
    let snapshot = AppStateSnapshotOwned::from_app(&app.state);
    assert!(snapshot.branches.view().commits_subview_loading);
    assert!(snapshot.branches.view().commits_subview.items.is_empty());
    for action in effects::run(current, &app.runtime).await {
        dispatch(&mut dispatcher, &mut app.state, action);
    }
    let snapshot = AppStateSnapshotOwned::from_app(&app.state);
    assert!(!snapshot.branches.view().commits_subview_loading);
    assert_eq!(
        snapshot.branches.view().commits_subview.items[0].oid,
        "abc1234567890"
    );
}

#[tokio::test]
async fn test_branch_completion_after_focus_change_is_visible_in_owned_snapshot() {
    let mut app = branch_fixture();
    let mut dispatcher = Dispatcher::with_default_stores();
    let request = open_revision_request(&mut app, &mut dispatcher).await;
    dispatch(
        &mut dispatcher,
        &mut app.state,
        Action::Domain(DomainAction::PanelGoto(1)),
    );
    app.state.ui.dirty.clear();
    for action in effects::run(request, &app.runtime).await {
        dispatch(&mut dispatcher, &mut app.state, action);
    }
    let snapshot = AppStateSnapshotOwned::from_app(&app.state);
    assert_eq!(snapshot.active_panel, SidePanel::Files);
    assert!(!snapshot.branches.view().commits_subview_loading);
    assert_eq!(snapshot.branches.view().commits_subview.items.len(), 1);
    assert!(app.state.ui.dirty.main_content());
}

#[tokio::test]
async fn test_cancelled_stash_read_cannot_reopen_tree_through_dispatcher() {
    let mut app = fixture(ControlledRepo::default());
    app.state.revisions.update(
        crate::app::revisions::RevisionAction::ReplaceStashes(
            vec![StashInfo {
                index: 12,
                message: "saved".into(),
            }]
            .into(),
        ),
        SidePanel::Files,
    );
    app.state.ui.active_panel = SidePanel::Stash;
    let mut dispatcher = Dispatcher::with_default_stores();
    let request = open_revision_request(&mut app, &mut dispatcher).await;
    let actions = effects::run(request, &app.runtime).await;
    dispatch(
        &mut dispatcher,
        &mut app.state,
        Action::Domain(DomainAction::RevisionCloseTree),
    );
    for action in actions {
        dispatch(&mut dispatcher, &mut app.state, action);
    }
    let snapshot = AppStateSnapshotOwned::from_app(&app.state);
    assert!(!snapshot.stash.view().tree_mode.active);
    assert!(snapshot.stash.view().tree_mode.selected_source.is_none());
    assert_eq!(snapshot.stash.view().panel.selected(), Some(0));
}

#[tokio::test]
async fn test_branch_read_failure_clears_loading_and_publishes_failure_log() {
    let mut app = branch_fixture();
    let mut dispatcher = Dispatcher::with_default_stores();
    let EffectRequest::LoadBackground { request, .. } =
        open_revision_request(&mut app, &mut dispatcher).await
    else {
        unreachable!()
    };
    app.state.ui.dirty.clear();
    dispatch(
        &mut dispatcher,
        &mut app.state,
        Action::System(SystemAction::BackgroundCompleted(Box::new(
            crate::app::BackgroundCompleted {
                request,
                result: Err("branch read failed".into()),
            },
        ))),
    );
    let snapshot = AppStateSnapshotOwned::from_app(&app.state);
    assert!(!snapshot.branches.view().commits_subview_loading);
    assert_eq!(
        snapshot.command_log.last().unwrap().command,
        "branch read failed"
    );
    assert!(!snapshot.command_log.last().unwrap().success);
    assert!(app.state.ui.dirty.is_dirty());
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
    assert_eq!(app.state.files.status().unstaged.len(), 1);
    for (request, staged) in [
        (EffectRequest::StageFile("file.txt".into()), true),
        (EffectRequest::UnstageFile("file.txt".into()), false),
    ] {
        let actions = effects::run(request, &app.runtime).await;
        assert_eq!(app.state.files.status().staged.is_empty(), staged);
        super::test_runtime::drive_actions(&mut app, actions);
        super::test_runtime::drive_actions(
            &mut app,
            [Action::Domain(DomainAction::Runtime(
                RuntimeEvent::ProcessBackgroundLoads,
            ))],
        );
        assert_eq!(!app.state.files.status().staged.is_empty(), staged);
        assert_eq!(app.state.files.status().unstaged.is_empty(), staged);
    }
}
