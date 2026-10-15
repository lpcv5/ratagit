//! Regressions exercise the effect worker, completion clones and production dispatcher.
use super::*;
use crate::git::operation::OperationControl;
use crate::git::{Git2Repository, GitErrorCategory};

fn native(code: git2::ErrorCode) -> GitError {
    git2::Error::new(code, git2::ErrorClass::Repository, "same diagnostic").into()
}

#[tokio::test]
async fn test_same_diagnostic_retains_category_through_effect_and_completion_clone() {
    for (code, category, hint) in [
        (
            git2::ErrorCode::Conflict,
            GitErrorCategory::Conflict,
            "Resolve conflicts",
        ),
        (
            git2::ErrorCode::Auth,
            GitErrorCategory::Authentication,
            "Check credentials",
        ),
        (
            git2::ErrorCode::Locked,
            GitErrorCategory::Locked,
            "Another Git operation",
        ),
    ] {
        let expected = native(code).context("fetch");
        let (tx, rx) = mpsc::channel();
        tx.send(Err(native(code))).unwrap();
        let mut app = fixture(ControlledRepo {
            fetch: Mutex::new(Some(rx)),
            ..Default::default()
        });
        let actions = effects::run(EffectRequest::FetchRemote, &app.runtime).await;
        let action = actions[0].clone();
        let Action::Domain(DomainAction::FetchRemoteFinished(Err(error))) = &action else {
            panic!("typed fetch failure");
        };
        assert_eq!(error, &expected);
        assert_eq!(error.category(), category);
        let mut dispatcher = Dispatcher::with_default_stores();
        app.ui.dirty.clear();
        assert!(dispatch_commands(&mut dispatcher, &mut app.state, action).is_empty());
        assert!(app.ui.dirty.command_log());
        let log = app.command_log.last().unwrap();
        assert!(!log.success);
        assert!(log.command.contains(hint), "{}", log.command);
        assert!(log.command.contains("same diagnostic"));
        assert!(log.command.contains("code "));
        assert!(log.command.contains("class "));
    }
}

#[tokio::test]
async fn test_background_failure_keeps_identity_category_and_invalidates_log() {
    let error = native(git2::ErrorCode::Locked);
    let mut app = fixture(ControlledRepo {
        diff_error: Some(error.clone()),
        ..Default::default()
    });
    let request = diff_request(&mut app);
    let EffectRequest::LoadBackground { request: expected } = &request else {
        unreachable!()
    };
    let actions = effects::run(request.clone(), &app.runtime).await;
    let Action::System(SystemAction::BackgroundCompleted(completed)) = &actions[0] else {
        panic!("background completion");
    };
    assert_eq!(completed.request.generation, expected.generation);
    assert_eq!(completed.request.key, expected.key);
    assert_eq!(
        completed.result.as_ref().unwrap_err(),
        &error.context("diff")
    );
    let mut dispatcher = Dispatcher::with_default_stores();
    app.ui.dirty.clear();
    dispatch_commands(&mut dispatcher, &mut app.state, actions[0].clone());
    assert!(app.ui.dirty.command_log());
    assert!(!AppStateSnapshotOwned::from_app(&app.state).diff_loading);
    assert!(app
        .command_log
        .last()
        .unwrap()
        .command
        .contains("Another Git operation"));
    let logs = app.command_log.len();
    app.ui.dirty.clear();
    // Duplicate failures have already retired; they must not add a second hint.
    dispatch_commands(&mut dispatcher, &mut app.state, actions[0].clone());
    assert_eq!(app.command_log.len(), logs);
    assert!(!app.ui.dirty.command_log());
}

#[tokio::test]
async fn test_real_index_lock_survives_failed_stage_without_retry() {
    let directory = tempfile::tempdir().unwrap();
    let repository = git2::Repository::init(directory.path()).unwrap();
    std::fs::write(directory.path().join("selected.txt"), "content\n").unwrap();
    let mut app =
        App::from_repo(Box::new(Git2Repository::open(directory.path()).unwrap())).unwrap();
    let lock = repository.path().join("index.lock");
    std::fs::write(&lock, "owned by another operation").unwrap();
    let actions = effects::run(
        EffectRequest::StageFile("selected.txt".into()),
        &app.runtime,
    )
    .await;
    let Action::Domain(DomainAction::StageFileFinished {
        path,
        result: Err(error),
    }) = &actions[0]
    else {
        panic!("stage must fail");
    };
    assert_eq!(path, Path::new("selected.txt"));
    assert_eq!(error.category(), GitErrorCategory::Locked);
    let mut dispatcher = Dispatcher::with_default_stores();
    app.ui.dirty.clear();
    assert!(dispatch_commands(&mut dispatcher, &mut app.state, actions[0].clone()).is_empty());
    assert!(app.ui.dirty.command_log());
    assert!(app
        .command_log
        .last()
        .unwrap()
        .command
        .contains("Another Git operation"));
    assert_eq!(
        std::fs::read_to_string(lock).unwrap(),
        "owned by another operation"
    );
    assert!(!repository.path().join("index").exists());
    assert!(repository
        .status_file(Path::new("selected.txt"))
        .unwrap()
        .is_wt_new());
}

#[test]
fn test_operation_and_reload_failures_preserve_both_sources() {
    for primary in [None, Some(native(git2::ErrorCode::Conflict))] {
        let reload = native(git2::ErrorCode::Locked);
        let repo = ControlledRepo {
            control_error: primary.clone(),
            reload_error: Some(reload.clone()),
            ..Default::default()
        };
        let actions = effects::execute(
            EffectRequest::ControlRepositoryOperation {
                id: 42,
                control: OperationControl::Continue,
            },
            &repo,
        );
        let Action::Domain(DomainAction::RepositoryOperationFinished {
            id,
            result: Err(error),
            observed,
        }) = &actions[0]
        else {
            panic!("operation completion");
        };
        assert_eq!(*id, 42);
        assert!(observed.is_none());
        let GitError::Context { operation, source } = error else {
            panic!("operation context")
        };
        assert_eq!(*operation, "control_operation");
        match (primary, source.as_ref()) {
            (None, GitError::Reload { source }) => {
                assert_eq!(source.as_ref(), &reload);
                assert!(crate::app::git_error::message(error).contains("do not repeat"));
            }
            (
                Some(expected),
                GitError::WithReloadFailure {
                    primary,
                    reload: actual,
                },
            ) => {
                assert_eq!(primary.as_ref(), &expected.context("control_operation"));
                assert_eq!(actual.as_ref(), &reload);
            }
            _ => panic!("both original failures must survive"),
        }
    }
}

#[tokio::test]
async fn test_disconnected_receiver_has_worker_category_and_operation_context() {
    let (tx, rx) = mpsc::channel();
    drop(tx);
    let app = fixture(ControlledRepo {
        fetch: Mutex::new(Some(rx)),
        ..Default::default()
    });
    let actions = effects::run(EffectRequest::FetchRemote, &app.runtime).await;
    let Action::Domain(DomainAction::FetchRemoteFinished(Err(error))) = &actions[0] else {
        panic!("failure")
    };
    assert_eq!(error.category(), GitErrorCategory::Worker);
    assert!(matches!(
        error,
        GitError::Context {
            operation: "fetch",
            ..
        }
    ));
    assert!(crate::app::git_error::message(error).contains("outcome may be unknown"));
}

#[tokio::test]
async fn test_selection_partial_write_retains_source_and_never_retries() {
    let error = native(git2::ErrorCode::Locked);
    let writes = Arc::new(Mutex::new(Vec::new()));
    let mut app = fixture(ControlledRepo {
        unstage_paths_error: Some(error.clone()),
        writes: writes.clone(),
        ..Default::default()
    });
    let actions = effects::run(
        EffectRequest::ToggleStageSelection {
            stage: vec!["a.txt".into()],
            unstage: vec!["b.txt".into()],
        },
        &app.runtime,
    )
    .await;
    let Action::Domain(DomainAction::ToggleStageSelectionFinished {
        result: Err(actual),
    }) = &actions[0]
    else {
        panic!("partial failure")
    };
    assert_eq!(
        actual,
        &GitError::PartialWrite {
            source: Box::new(error)
        }
        .context("toggle_stage")
    );
    let mut dispatcher = Dispatcher::with_default_stores();
    assert!(dispatch_commands(&mut dispatcher, &mut app.state, actions[0].clone()).is_empty());
    assert_eq!(*writes.lock().unwrap(), ["stage_paths", "unstage_paths"]);
    assert!(app
        .command_log
        .last()
        .unwrap()
        .command
        .contains("review which paths were staged"));
}
