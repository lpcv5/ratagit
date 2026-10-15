use super::*;
use crate::app::{AppRuntime, Command, InputMode};
use crate::flux::action::{DomainAction, RuntimeEvent, SystemAction};
use crate::flux::effects::EffectRequest;
use crate::flux::stores::test_support::MockRepo;
use crate::git::{self, *};
use std::path::Path;
use std::sync::{mpsc as blocking, Mutex};
use std::time::Duration;
use tokio::sync::Notify;

fn fixture() -> App {
    App::from_repo(Box::new(MockRepo)).unwrap()
}

async fn snapshot_until(
    channels: &mut UiChannels,
    predicate: impl Fn(&AppStateSnapshotOwned) -> bool,
) {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if predicate(&channels.snapshot_rx.borrow_and_update()) {
                return;
            }
            channels.snapshot_rx.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn test_runtime_preserves_input_fifo_and_publishes_quit_before_joining() {
    let mut app = fixture();
    app.input.mode = Some(InputMode::CreateBranch);
    run_with_ui(app, |mut channels, _| async move {
        channels
            .action_tx
            .send(Action::Domain(DomainAction::InputChar('a')))
            .unwrap();
        channels
            .action_tx
            .send(Action::Domain(DomainAction::InputChar('b')))
            .unwrap();
        snapshot_until(&mut channels, |snapshot| snapshot.input_buffer == "ab").await;
        assert!(channels.snapshot_rx.borrow().running);
        channels
            .action_tx
            .send(Action::Domain(DomainAction::Quit))
            .unwrap();
        snapshot_until(&mut channels, |snapshot| !snapshot.running).await;
        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn test_ui_error_is_preserved_and_all_runtime_channels_close() {
    let (saved_tx, saved_rx) = tokio::sync::oneshot::channel();
    let error = run_with_ui(fixture(), |channels, _| async move {
        assert!(saved_tx
            .send((
                channels.action_tx.clone(),
                channels.snapshot_rx.clone(),
                channels.state_version_rx.clone(),
            ))
            .is_ok());
        Err(color_eyre::eyre::eyre!("UI read failed"))
    })
    .await
    .unwrap_err();
    assert_eq!(error.to_string(), "UI read failed");
    let (action_tx, snapshots, versions) = saved_rx.await.unwrap();
    assert!(action_tx.send(Action::System(SystemAction::Tick)).is_err());
    assert!(snapshots.has_changed().is_err());
    assert!(versions.has_changed().is_err());
}

#[tokio::test(flavor = "current_thread")]
async fn test_dispatch_publishes_snapshot_and_version_before_followup_command() {
    tokio::task::LocalSet::new().run_until(async {
        let mut app = fixture();
        app.input.mode = Some(InputMode::CreateBranch);
        app.input.buffer = "feature".into();
        let initial = Arc::new(PublishedSnapshot::new(AppStateSnapshotOwned::from_app(&app.state), 0, None));
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        let (command_tx, mut command_rx) = mpsc::unbounded_channel();
        let (version_tx, version_rx) = watch::channel(0);
        let (snapshot_tx, mut snapshot_rx) = watch::channel(initial);
        let (shutdown_tx, _) = broadcast::channel(1);
        let handle = tokio::task::spawn_local(dispatch_loop((app.state, app.initial_output), Arc::default(), action_rx, command_tx, version_tx, snapshot_tx, shutdown_tx.subscribe()));
        action_tx.send(Action::Domain(DomainAction::InputEnter).into()).unwrap();
        let command = tokio::time::timeout(Duration::from_secs(3), command_rx.recv()).await.unwrap().unwrap().value;
        assert!(matches!(command, Command::Sync(DomainAction::CreateBranch(name)) if name == "feature"));
        assert_eq!(*version_rx.borrow(), 1);
        assert!(snapshot_rx.has_changed().unwrap());
        assert!(snapshot_rx.borrow_and_update().input_mode.is_none());
        assert!(snapshot_rx.borrow().input_buffer.is_empty());
        shutdown_tx.send(()).unwrap();
        handle.await.unwrap();
    }).await;
}

#[derive(Default)]
struct PausedRepo {
    stage: Mutex<Option<blocking::Receiver<Result<(), git::GitError>>>>,
    started: Arc<Notify>,
    writes: Arc<Mutex<Vec<String>>>,
}

macro_rules! delegate {
    ($($name:ident($($arg:ident: $ty:ty),*) -> $result:ty;)*) => {
        $(fn $name(&self, $($arg: $ty),*) -> $result { MockRepo.$name($($arg),*) })*
    };
}

impl WorktreeRepository for PausedRepo {
    delegate! { status() -> Result<GitStatus, GitError>; stage_paths(paths: &[std::path::PathBuf]) -> Result<(), GitError>; unstage_paths(paths: &[std::path::PathBuf]) -> Result<(), GitError>; discard_paths(paths: &[std::path::PathBuf]) -> Result<(), GitError>; }
    fn stage(&self, path: &Path) -> Result<(), GitError> {
        self.writes
            .lock()
            .unwrap()
            .push(format!("stage:{}", path.display()));
        self.started.notify_one();
        let gate = self.stage.lock().unwrap().take();
        match gate {
            Some(receiver) => receiver
                .recv()
                .map_err(|error| GitError::Git2(error.to_string()))?,
            None => Ok(()),
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
impl PatchRepository for PausedRepo {
    delegate! { diff_unstaged(path: &Path) -> Result<Vec<DiffLine>, GitError>; diff_staged(path: &Path) -> Result<Vec<DiffLine>, GitError>; diff_untracked(path: &Path) -> Result<Vec<DiffLine>, GitError>; commit_diff_scoped(oid: &str, path: Option<&Path>) -> Result<Vec<DiffLine>, GitError>; }
}
impl HistoryRepository for PausedRepo {
    delegate! { commits(limit: usize) -> Result<Vec<CommitInfo>, GitError>; commit_files(oid: &str) -> Result<Vec<FileEntry>, GitError>; }
}
impl ReferenceRepository for PausedRepo {
    delegate! { branches() -> Result<Vec<BranchInfo>, GitError>; create_branch(name: &str) -> Result<(), GitError>; checkout_branch(name: &str) -> Result<(), GitError>; delete_branch(name: &str) -> Result<(), GitError>; fetch_default_async() -> Result<blocking::Receiver<Result<String, GitError>>, GitError>; }
}
impl StashRepository for PausedRepo {
    delegate! { stashes() -> Result<Vec<StashInfo>, GitError>; stash_files(index: usize) -> Result<Vec<FileEntry>, GitError>; stash_diff(index: usize, path: Option<&Path>) -> Result<Vec<DiffLine>, GitError>; stash_push_paths(paths: &[std::path::PathBuf], message: &str) -> Result<usize, GitError>; stash_apply(index: usize) -> Result<(), GitError>; stash_pop(index: usize) -> Result<(), GitError>; stash_drop(index: usize) -> Result<(), GitError>; }
}
impl CommitRepository for PausedRepo {
    delegate! { commit(message: &str) -> Result<String, GitError>; }
}
impl OperationRepository for PausedRepo {}

async fn next_action(receiver: &mut mpsc::UnboundedReceiver<Tracked<Action>>) -> Action {
    tokio::time::timeout(Duration::from_secs(3), receiver.recv())
        .await
        .unwrap()
        .unwrap()
        .value
}

#[tokio::test(flavor = "current_thread")]
async fn test_command_loop_keeps_fifo_actions_and_writes_while_reads_continue() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let (release, gate) = blocking::channel();
            let repo = PausedRepo {
                stage: Mutex::new(Some(gate)),
                ..Default::default()
            };
            let started = repo.started.clone();
            let writes = repo.writes.clone();
            let (command_tx, command_rx) = mpsc::unbounded_channel();
            let (action_tx, mut action_rx) = mpsc::unbounded_channel();
            let action_tx = ActionSender::new(action_tx, None);
            let (shutdown_tx, _) = broadcast::channel(1);
            let handle = tokio::task::spawn_local(command_loop(
                AppRuntime::new(Box::new(repo)),
                Arc::default(),
                command_rx,
                action_tx,
                shutdown_tx.subscribe(),
            ));
            command_tx
                .send(Command::Effect(EffectRequest::StageFile("first".into())).into())
                .unwrap();
            command_tx
                .send(Command::runtime(RuntimeEvent::FlushPendingDiffReload).into())
                .unwrap();
            command_tx
                .send(Command::Effect(EffectRequest::UnstageFile("second".into())).into())
                .unwrap();
            command_tx
                .send(Command::Effect(EffectRequest::FetchRemote).into())
                .unwrap();
            command_tx
                .send(Command::Sync(DomainAction::InputChar('x')).into())
                .unwrap();
            tokio::time::timeout(Duration::from_secs(3), started.notified())
                .await
                .unwrap();
            let mut follows = Vec::new();
            let mut read_finished = false;
            for _ in 0..3 {
                match next_action(&mut action_rx).await {
                    Action::Domain(DomainAction::Runtime(RuntimeEvent::FlushPendingDiffReload)) => {
                        follows.push("flush")
                    }
                    Action::Domain(DomainAction::InputChar('x')) => follows.push("input"),
                    Action::Domain(DomainAction::FetchRemoteFinished(_)) => read_finished = true,
                    other => panic!("unexpected action before write release: {other:?}"),
                }
            }
            assert_eq!(follows, ["flush", "input"]);
            assert!(read_finished);
            assert_eq!(*writes.lock().unwrap(), ["stage:first"]);
            release.send(Ok(())).unwrap();
            assert!(matches!(
                next_action(&mut action_rx).await,
                Action::Domain(DomainAction::StageFileFinished { .. })
            ));
            assert!(matches!(
                next_action(&mut action_rx).await,
                Action::Domain(DomainAction::UnstageFileFinished { .. })
            ));
            assert_eq!(*writes.lock().unwrap(), ["stage:first", "unstage:second"]);
            shutdown_tx.send(()).unwrap();
            handle.await.unwrap();
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn test_runtime_joins_tasks_after_ui_error_without_waiting_for_blocked_git() {
    let (release, gate) = blocking::channel();
    let repo = PausedRepo {
        stage: Mutex::new(Some(gate)),
        ..Default::default()
    };
    let started = repo.started.clone();
    let app = App::from_repo(Box::new(repo)).unwrap();
    let result = tokio::time::timeout(
        Duration::from_secs(3),
        run_with_ui(app, |mut channels, _| async move {
            channels
                .action_tx
                .send(Action::Domain(DomainAction::StageFile("held".into())))
                .unwrap();
            started.notified().await;
            channels
                .action_tx
                .send(Action::Domain(DomainAction::Quit))
                .unwrap();
            snapshot_until(&mut channels, |snapshot| !snapshot.running).await;
            Err(color_eyre::eyre::eyre!("UI failed after quit"))
        }),
    )
    .await;
    release.send(Ok(())).unwrap();
    assert_eq!(
        result
            .expect("runtime shutdown must not wait for Git")
            .unwrap_err()
            .to_string(),
        "UI failed after quit"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn test_auto_refresh_ticks_immediately_and_shutdown_does_not_wait_for_interval() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let (action_tx, mut action_rx) = mpsc::unbounded_channel();
            let action_tx = ActionSender::new(action_tx, None);
            let (shutdown_tx, _) = broadcast::channel(1);
            let handle =
                tokio::task::spawn_local(auto_refresh_loop(action_tx, shutdown_tx.subscribe()));
            assert!(matches!(
                next_action(&mut action_rx).await,
                Action::System(SystemAction::AutoRefresh)
            ));
            shutdown_tx.send(()).unwrap();
            tokio::time::timeout(Duration::from_secs(1), handle)
                .await
                .unwrap()
                .unwrap();
            assert!(action_rx.recv().await.is_none());
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn test_traced_effect_queue_preserves_identity_while_write_is_blocked() {
    use crate::latency::{tests::Capture, Recorder};
    tokio::task::LocalSet::new()
        .run_until(async {
            let (release, gate) = blocking::channel();
            let repo = PausedRepo {
                stage: Mutex::new(Some(gate)),
                ..Default::default()
            };
            let started = repo.started.clone();
            let capture = Capture::default();
            let recorder = Recorder::with_writer(capture.clone());
            let first = recorder.start("input").child("stage", None);
            let second = recorder.start("input").child("unstage", None);
            let (command_tx, command_rx) = mpsc::unbounded_channel();
            let (action_tx, mut action_rx) = mpsc::unbounded_channel();
            let (shutdown_tx, _) = broadcast::channel(1);
            let handle = tokio::task::spawn_local(command_loop(
                AppRuntime::new(Box::new(repo)),
                Arc::default(),
                command_rx,
                ActionSender::new(action_tx, None),
                shutdown_tx.subscribe(),
            ));
            command_tx
                .send(Tracked::new(
                    Command::Effect(EffectRequest::StageFile("first".into())),
                    Some(first),
                ))
                .unwrap();
            command_tx
                .send(Tracked::new(
                    Command::Effect(EffectRequest::UnstageFile("second".into())),
                    Some(second),
                ))
                .unwrap();
            tokio::time::timeout(Duration::from_secs(3), started.notified())
                .await
                .unwrap();
            assert!(
                !capture
                    .events()
                    .iter()
                    .any(|event| event["operation"] == "unstage"
                        && event["stage"] == "effect_started")
            );
            release.send(Ok(())).unwrap();
            let first_action = tokio::time::timeout(Duration::from_secs(3), action_rx.recv())
                .await
                .unwrap()
                .unwrap();
            let second_action = tokio::time::timeout(Duration::from_secs(3), action_rx.recv())
                .await
                .unwrap()
                .unwrap();
            assert!(matches!(
                first_action.value,
                Action::Domain(DomainAction::StageFileFinished { .. })
            ));
            assert!(matches!(
                second_action.value,
                Action::Domain(DomainAction::UnstageFileFinished { .. })
            ));
            assert!(first_action.trace.is_some() && second_action.trace.is_some());
            let events = capture.events();
            let first_finished = events
                .iter()
                .position(|event| {
                    event["operation"] == "stage" && event["stage"] == "effect_finished"
                })
                .unwrap();
            let second_started = events
                .iter()
                .position(|event| {
                    event["operation"] == "unstage" && event["stage"] == "effect_started"
                })
                .unwrap();
            assert!(first_finished < second_started);
            assert_ne!(
                events[first_finished]["request_id"],
                events[second_started]["request_id"]
            );
            shutdown_tx.send(()).unwrap();
            handle.await.unwrap();
        })
        .await;
}
