//! Exercise provenance through the production dispatcher, executor and renderer.
use super::*;
use crate::file_tree::{FileTreeNode, FileTreeNodeStatus};
use crate::flux::action::{DomainAction, RuntimeEvent, SystemAction};
use crate::flux::effects::{self, EffectRequest};
use crate::flux::stores::test_support::MockRepo;
use crate::git::{FileStatus, Git2Repository};
use crate::latency::{tests::Capture, Recorder, Trace};
use ratatui::{backend::TestBackend, Terminal};
use std::time::Duration;

fn fixture() -> App {
    let mut app = App::from_repo(Box::new(MockRepo)).unwrap();
    app.initial_output = Default::default();
    app.ui.dirty.clear();
    app.files.test_view_mut().tree_nodes = ["a.txt", "b.txt"]
        .into_iter()
        .map(|path| FileTreeNode {
            path: path.into(),
            status: FileTreeNodeStatus::Unstaged(FileStatus::Modified),
            depth: 0,
            is_dir: false,
            is_expanded: false,
        })
        .collect();
    app.files.test_view_mut().panel.select(Some(0));
    app
}

struct Harness {
    channels: UiChannels,
    commands: mpsc::UnboundedReceiver<Tracked<app::Command>>,
    handle: tokio::task::JoinHandle<()>,
}

impl Harness {
    fn start(app: App, recorder: Option<Arc<Recorder>>) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let action_tx = ActionSender::new(tx, recorder);
        let (command_tx, commands) = mpsc::unbounded_channel();
        let (version_tx, state_version_rx) = watch::channel(0);
        let initial = PublishedSnapshot::new(AppStateSnapshotOwned::from_app(&app.state), 0, None);
        let (snapshot_tx, snapshot_rx) = watch::channel(Arc::new(initial));
        let (shutdown_tx, _) = broadcast::channel(1);
        let handle = tokio::task::spawn_local(dispatch_loop(
            (app.state, app.initial_output),
            Arc::default(),
            rx,
            command_tx,
            version_tx,
            snapshot_tx,
            shutdown_tx.subscribe(),
        ));
        Self {
            channels: UiChannels {
                editor_rx: mpsc::unbounded_channel().1,
                action_tx,
                state_version_rx,
                snapshot_rx,
                shutdown_tx,
            },
            commands,
            handle,
        }
    }

    async fn version(&mut self, version: u64) {
        tokio::time::timeout(Duration::from_secs(3), async {
            while *self.channels.state_version_rx.borrow_and_update() < version {
                self.channels.state_version_rx.changed().await.unwrap();
            }
        })
        .await
        .unwrap();
    }

    async fn command(&mut self) -> Tracked<app::Command> {
        tokio::time::timeout(Duration::from_secs(3), self.commands.recv())
            .await
            .unwrap()
            .unwrap()
    }

    fn draw(&mut self) {
        let snapshot = self.channels.snapshot_rx.borrow_and_update().clone();
        ui::draw_snapshot(
            &mut Terminal::new(TestBackend::new(100, 30)).unwrap(),
            &snapshot,
            &mut Default::default(),
        )
        .unwrap();
    }

    async fn stop(self) {
        self.channels.shutdown_tx.send(()).unwrap();
        self.handle.await.unwrap();
    }
}

#[tokio::test(flavor = "current_thread")]
async fn test_coalesced_inputs_measure_only_first_successful_frame() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let capture = Capture::default();
            let mut app = fixture();
            app.input.mode = Some(crate::app::InputMode::CreateBranch);
            let mut harness = Harness::start(app, Some(Recorder::with_writer(capture.clone())));
            harness
                .channels
                .action_tx
                .send(Action::Domain(DomainAction::InputChar('a')))
                .unwrap();
            harness
                .channels
                .action_tx
                .send(Action::Domain(DomainAction::InputChar('b')))
                .unwrap();
            harness.version(2).await;
            assert_eq!(harness.channels.snapshot_rx.borrow().input_buffer, "ab");
            harness.draw();
            harness.draw();
            let events = capture.events();
            let roots: Vec<_> = events
                .iter()
                .filter(|event| event["stage"] == "request_started")
                .collect();
            let frames: Vec<_> = events
                .iter()
                .filter(|event| event["stage"] == "frame_drawn")
                .collect();
            assert_eq!(roots.len(), 2);
            assert_eq!(frames.len(), 1);
            assert_eq!(frames[0]["request_id"], roots[1]["request_id"]);
            assert_eq!(frames[0]["state_version"], 2);
            assert!(events
                .iter()
                .any(|event| event["stage"] == "snapshot_superseded"
                    && event["request_id"] == roots[0]["request_id"]));
            harness.stop().await;
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn test_deferred_diff_keeps_input_origin_and_rejects_duplicate_completion() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let capture = Capture::default();
            let mut harness =
                Harness::start(fixture(), Some(Recorder::with_writer(capture.clone())));
            harness
                .channels
                .action_tx
                .send(Action::Domain(DomainAction::ListDown))
                .unwrap();
            harness.version(1).await;
            assert!(harness.channels.snapshot_rx.borrow().pending_diff_reload);
            harness.draw();

            // This untraced maintenance action is what a later tick routes. It must
            // inherit the selection's origin rather than start a new request.
            harness
                .channels
                .action_tx
                .send_traced(
                    Action::Domain(DomainAction::Runtime(RuntimeEvent::FlushPendingDiffReload)),
                    None,
                )
                .unwrap();
            harness.version(2).await;
            let command = harness.command().await;
            let app::Command::Effect(request @ EffectRequest::LoadBackground { .. }) =
                command.value
            else {
                panic!("expected diff read");
            };
            let trace = command.trace.expect("deferred diff retains input origin");
            let runtime = app::AppRuntime::new(Box::new(MockRepo));
            let mut executor = crate::flux::runtime::EffectExecutor::new(runtime);
            executor.enqueue_traced(request, Some(trace.clone()));
            let completed = executor.next_completed().await;
            let action = completed.actions.into_iter().next().unwrap();
            harness
                .channels
                .action_tx
                .send_traced(action.clone(), completed.trace)
                .unwrap();
            harness.version(3).await;
            harness.draw();
            harness
                .channels
                .action_tx
                .send_traced(action, Some(trace))
                .unwrap();
            harness.version(4).await;
            harness.draw();
            let events = capture.events();
            let frames: Vec<_> = events
                .iter()
                .filter(|event| event["stage"] == "frame_drawn")
                .collect();
            assert_eq!(
                frames.len(),
                2,
                "input feedback and accepted diff are separate samples"
            );
            assert_eq!(frames[0]["request_id"], frames[1]["request_id"]);
            assert_ne!(frames[0]["span_id"], frames[1]["span_id"]);
            assert_eq!(frames[1]["operation"], "diff");
            assert!(frames[1]["generation"].as_u64().is_some());
            assert!(events
                .iter()
                .any(|event| event["stage"] == "completion_rejected"));
            for stage in [
                "action_dequeued",
                "dispatch_finished",
                "command_queued",
                "effect_started",
                "effect_finished",
                "snapshot_built",
                "snapshot_published",
            ] {
                assert!(
                    events.iter().any(|event| event["stage"] == stage),
                    "missing {stage}"
                );
            }

            // Return to the cached file after publishing navigation feedback.
            // Cache hits need a result sample even if the root's first frame
            // was already drawn before the debounce flush.
            harness
                .channels
                .action_tx
                .send(Action::Domain(DomainAction::ListUp))
                .unwrap();
            harness
                .channels
                .action_tx
                .send(Action::Domain(DomainAction::ListDown))
                .unwrap();
            harness.version(6).await;
            harness.draw();
            harness
                .channels
                .action_tx
                .send_traced(
                    Action::Domain(DomainAction::Runtime(RuntimeEvent::FlushPendingDiffReload)),
                    None,
                )
                .unwrap();
            harness.version(7).await;
            harness.draw();
            assert!(matches!(
                harness.commands.try_recv(),
                Err(mpsc::error::TryRecvError::Empty)
            ));
            let events = capture.events();
            let cached = events
                .iter()
                .find(|event| event["operation"] == "diff_ready" && event["stage"] == "frame_drawn")
                .unwrap();
            let latest_input = events
                .iter()
                .rev()
                .find(|event| event["stage"] == "request_started")
                .unwrap();
            assert_eq!(cached["request_id"], latest_input["request_id"]);
            assert_ne!(cached["span_id"], latest_input["span_id"]);
            harness.stop().await;
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn test_disabled_tracking_keeps_actions_and_snapshots_untraced() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let mut app = fixture();
            app.input.mode = Some(crate::app::InputMode::CreateBranch);
            let mut harness = Harness::start(app, None);
            assert!(harness.channels.action_tx.start("input").is_none());
            harness
                .channels
                .action_tx
                .send(Action::Domain(DomainAction::InputChar('x')))
                .unwrap();
            harness.version(1).await;
            assert_eq!(harness.channels.snapshot_rx.borrow().input_buffer, "x");
            harness.draw();
            harness.stop().await;
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn test_cancelled_failure_is_excluded_but_current_error_feedback_is_measured() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let capture = Capture::default();
            let mut harness =
                Harness::start(fixture(), Some(Recorder::with_writer(capture.clone())));
            let mut reads = Vec::new();
            for (index, action) in [DomainAction::ListDown, DomainAction::ListUp]
                .into_iter()
                .enumerate()
            {
                harness
                    .channels
                    .action_tx
                    .send(Action::Domain(action))
                    .unwrap();
                harness.version((index * 2 + 1) as u64).await;
                harness
                    .channels
                    .action_tx
                    .send_traced(
                        Action::Domain(DomainAction::Runtime(RuntimeEvent::FlushPendingDiffReload)),
                        None,
                    )
                    .unwrap();
                harness.version((index * 2 + 2) as u64).await;
                reads.push(harness.command().await);
            }
            for (index, command) in reads.into_iter().enumerate() {
                let app::Command::Effect(EffectRequest::LoadBackground { request }) = command.value
                else {
                    panic!("expected diff read");
                };
                let failure = if index == 0 {
                    "obsolete error"
                } else {
                    "current error"
                };
                harness
                    .channels
                    .action_tx
                    .send_traced(
                        Action::System(SystemAction::BackgroundCompleted(Box::new(
                            crate::background::BackgroundCompleted {
                                request,
                                result: Err(failure.into()),
                            },
                        ))),
                        command.trace,
                    )
                    .unwrap();
                harness.version(index as u64 + 5).await;
                if index == 0 {
                    assert_eq!(harness.channels.snapshot_rx.borrow().version, 4);
                    assert!(!harness
                        .channels
                        .snapshot_rx
                        .borrow()
                        .command_log
                        .iter()
                        .any(|entry| entry.command == failure));
                } else {
                    harness.draw();
                }
            }
            let events = capture.events();
            let rejected = events
                .iter()
                .find(|event| event["stage"] == "completion_rejected")
                .unwrap();
            let failed = events
                .iter()
                .find(|event| event["stage"] == "completion_failed")
                .unwrap();
            let frames: Vec<_> = events
                .iter()
                .filter(|event| event["stage"] == "frame_drawn")
                .collect();
            assert_eq!(frames.len(), 1);
            assert_eq!(frames[0]["span_id"], failed["span_id"]);
            assert_ne!(frames[0]["span_id"], rejected["span_id"]);
            assert!(!events
                .iter()
                .any(|event| event.to_string().contains("current error")));
            harness.stop().await;
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn test_native_git_worker_timings_keep_effect_and_generation_identity() {
    let directory = tempfile::tempdir().unwrap();
    git2::Repository::init(directory.path()).unwrap();
    let repo = Git2Repository::open(directory.path()).unwrap();
    let runtime = app::AppRuntime::new(Box::new(repo));
    let capture = Capture::default();
    let trace = Recorder::with_writer(capture.clone())
        .start("refresh")
        .child("status_full", Some(17));
    let request = crate::background::TaskRequest {
        key: crate::background::TaskKey::Status,
        generation: crate::background::TaskGeneration(17),
        priority: crate::background::TaskPriority::High,
        load: crate::background::BackgroundLoad::Status {
            fast: false,
            operation_epoch: 0,
        },
    };
    let actions = effects::run_traced(
        EffectRequest::LoadBackground { request },
        &runtime,
        Some(trace),
    )
    .await;
    assert!(
        matches!(&actions[0], Action::System(SystemAction::BackgroundCompleted(result)) if result.result.is_ok())
    );
    let events = capture.events();
    for stage in ["git_queued", "git_started", "git_finished"] {
        let event = events.iter().find(|event| event["stage"] == stage).unwrap();
        assert_eq!(event["request_id"], events[0]["request_id"]);
        assert_eq!(event["generation"], 17);
        assert_eq!(event["operation"], "status_full");
        assert!(event["lane"].as_u64().is_some());
        assert!(!event
            .to_string()
            .contains(directory.path().to_str().unwrap()));
    }
    assert!(Trace::current().is_none());
}

#[test]
fn test_failed_draw_and_unobserved_snapshot_do_not_produce_frame_sample() {
    use ratatui::backend::CrosstermBackend;
    use std::io::{self, Write};
    struct Broken;
    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("controlled write failure"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("controlled flush failure"))
        }
    }
    let capture = Capture::default();
    let recorder = Recorder::with_writer(capture.clone());
    let snapshot = PublishedSnapshot::new(
        AppStateSnapshotOwned::from_app(&fixture().state),
        7,
        Some(recorder.start("input")),
    );
    let mut terminal = Terminal::with_options(
        CrosstermBackend::new(Broken),
        ratatui::TerminalOptions {
            viewport: ratatui::Viewport::Fixed(ratatui::layout::Rect::new(0, 0, 100, 30)),
        },
    )
    .unwrap();
    assert!(ui::draw_snapshot(&mut terminal, &snapshot, &mut Default::default()).is_err());
    snapshot.finish("frame_unobserved", Duration::ZERO);
    let events = capture.events();
    assert!(events.iter().any(|event| event["stage"] == "draw_failed"));
    assert!(!events.iter().any(|event| event["stage"] == "frame_drawn"));
}

#[tokio::test(flavor = "current_thread")]
async fn test_explicit_recording_enables_counters_and_closes_unobserved_frame() {
    let capture = Capture::default();
    let mut app = fixture();
    app.input.mode = Some(crate::app::InputMode::CreateBranch);
    run_with_ui_recording(
        app,
        Some(Recorder::with_writer(capture.clone())),
        |mut channels, perf| async move {
            assert!(perf.profiling);
            assert!(
                perf.logging_enabled(),
                "explicit recording also works without debug assertions"
            );
            channels
                .action_tx
                .send(Action::Domain(DomainAction::InputChar('x')))
                .unwrap();
            tokio::time::timeout(Duration::from_secs(3), async {
                loop {
                    if channels.snapshot_rx.borrow_and_update().input_buffer == "x" {
                        break;
                    }
                    channels.snapshot_rx.changed().await.unwrap();
                }
            })
            .await
            .unwrap();
            Ok(())
        },
    )
    .await
    .unwrap();
    let events = capture.events();
    assert!(events
        .iter()
        .any(|event| event["stage"] == "frame_unobserved"));
    assert!(!events.iter().any(|event| event["stage"] == "frame_drawn"));
}
