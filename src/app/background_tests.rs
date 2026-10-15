//! Production completion events retain payload identity through narrow state adapters.
use super::*;
use crate::background::{TaskPriority, TaskRequest};
use crate::flux::action::{Action, SystemAction};
use crate::flux::command_output::CommandOutput;
use crate::flux::dispatcher::Dispatcher;
use crate::flux::snapshot::measurement::{fixture, measure, Allocations};
use crate::flux::snapshot::AppStateSnapshotOwned;
use crate::git::{FileEntry, FileStatus, GitStatus};
use crate::shared::Shared;
use std::sync::Arc;

fn register(app: &mut App, load: BackgroundLoad) -> TaskRequest {
    let mut output = CommandOutput::default();
    app.queue_background(load, TaskPriority::High, &mut output);
    let Command::Effect(crate::flux::effects::EffectRequest::LoadBackground { request }) =
        output.drain().remove(0)
    else {
        panic!("background effect required")
    };
    request
}

fn complete(app: &mut App, completed: BackgroundCompleted) {
    let mut dispatcher = Dispatcher::with_default_stores();
    let envelope = dispatcher.next_envelope(Action::System(SystemAction::BackgroundCompleted(
        Box::new(completed),
    )));
    dispatcher.dispatch(&mut app.state, envelope);
}

fn files() -> Shared<Vec<FileEntry>> {
    vec![FileEntry {
        path: "dir/file.rs".into(),
        status: FileStatus::Modified,
    }]
    .into()
}

#[test]
fn test_history_completion_clones_and_dispatches_without_deep_payload_copy() {
    let mut measured = Vec::new();
    for large in [false, true] {
        let mut app = fixture(large);
        let items = app.revisions.view().commits.items.clone();
        let request = register(
            &mut app,
            BackgroundLoad::Commits {
                limit: items.len(),
                fast: false,
            },
        );
        let completed = BackgroundCompleted {
            request,
            result: Ok(Arc::new(BackgroundPayload::Commits(items.clone()))),
        };
        let (cloned, cloned_counts) = measure(|| completed.clone());
        assert_eq!(cloned_counts, Allocations::default());
        assert!(Arc::ptr_eq(
            completed.result.as_ref().unwrap(),
            cloned.result.as_ref().unwrap()
        ));
        let mut dispatcher = Dispatcher::with_default_stores();
        let envelope = dispatcher.next_envelope(Action::System(SystemAction::BackgroundCompleted(
            Box::new(cloned),
        )));
        let (_, accepted_counts) = measure(|| dispatcher.dispatch(&mut app.state, envelope));
        assert!(items.ptr_eq(&app.revisions.view().commits.items));
        let snapshot = AppStateSnapshotOwned::from_app(&app.state);
        assert!(std::ptr::eq(
            items.as_ptr(),
            snapshot.commits.view().items.as_ptr()
        ));
        assert!(!app.tasks.has_task(&crate::background::TaskKey::Commits));
        measured.push(accepted_counts);
    }
    assert_eq!(measured[0], measured[1]);
    assert!(
        measured[1].calls <= 16 && measured[1].bytes <= 2048,
        "{measured:?}"
    );
}

#[test]
fn test_cancelled_and_superseded_large_history_results_leave_current_payload_unchanged() {
    let mut app = fixture(true);
    let items = app.revisions.view().commits.items.clone();
    let old = register(
        &mut app,
        BackgroundLoad::Commits {
            limit: 30,
            fast: false,
        },
    );
    let current = register(
        &mut app,
        BackgroundLoad::Commits {
            limit: 70,
            fast: false,
        },
    );
    let obsolete = BackgroundCompleted {
        request: old,
        result: Ok(Arc::new(BackgroundPayload::Commits(items.clone()))),
    };
    let mut dispatcher = Dispatcher::with_default_stores();
    let envelope = dispatcher.next_envelope(Action::System(SystemAction::BackgroundCompleted(
        Box::new(obsolete),
    )));
    let (_, counts) = measure(|| dispatcher.dispatch(&mut app.state, envelope));
    assert_eq!(counts, Allocations::default());
    assert!(items.ptr_eq(&app.revisions.view().commits.items));
    assert!(app.tasks.has_task(&current.key));
    let mut output = CommandOutput::default();
    app.cancel_background_task(&current.key, &mut output);
    let logs = app.command_log.len();
    complete(
        &mut app,
        BackgroundCompleted {
            request: current,
            result: Err("cancelled result must be silent".into()),
        },
    );
    assert_eq!(app.command_log.len(), logs);
    assert!(items.ptr_eq(&app.revisions.view().commits.items));
    assert!(app.tasks.is_empty());
}

#[test]
fn test_status_completion_reuses_status_and_conflicts_and_keeps_epoch_rejection() {
    use crate::git::operation::{OperationKind, RepositoryOperation};
    let mut app = fixture(false);
    let status: Shared<GitStatus> = GitStatus {
        unstaged: files().to_vec(),
        ..Default::default()
    }
    .into();
    let conflicts: Shared<Vec<_>> = vec!["dir/conflicted.rs".into()].into();
    let request = register(
        &mut app,
        BackgroundLoad::Status {
            fast: false,
            operation_epoch: 0,
        },
    );
    let completed = BackgroundCompleted {
        request,
        result: Ok(Arc::new(BackgroundPayload::Status {
            status: status.clone(),
            fast: false,
            operation_epoch: 0,
            operation: RepositoryOperation {
                kind: Some(OperationKind::Merge),
                conflicts: conflicts.clone(),
                ..Default::default()
            },
        })),
    };
    let (cloned, counts) = measure(|| completed.clone());
    assert_eq!(counts, Allocations::default());
    complete(&mut app, cloned);
    assert!(std::ptr::eq(app.files.status(), &*status));
    assert!(conflicts.ptr_eq(&app.git.operation.observed.conflicts));
    let retained = AppStateSnapshotOwned::from_app(&app.state);

    app.git.operation.epoch = 1;
    let request = register(
        &mut app,
        BackgroundLoad::Status {
            fast: false,
            operation_epoch: 0,
        },
    );
    complete(
        &mut app,
        BackgroundCompleted {
            request,
            result: Ok(Arc::new(BackgroundPayload::Status {
                status: status.clone(),
                fast: false,
                operation_epoch: 0,
                operation: RepositoryOperation::default(),
            })),
        },
    );
    assert_eq!(app.git.operation.observed.kind, Some(OperationKind::Merge));
    assert!(conflicts.ptr_eq(&app.git.operation.observed.conflicts));
    assert!(conflicts.ptr_eq(&retained.operation.observed.conflicts));
}

#[test]
fn test_current_revision_generation_still_rejects_obsolete_feature_sources() {
    let mut app = fixture(false);
    app.revisions.branches.commits_subview_active = true;
    app.revisions.branches.commits_subview_source = Some("current-branch".into());
    app.revisions.commits.tree_mode.selected_source = Some("current-commit".into());
    app.revisions.stash.tree_mode.selected_source = Some(12);
    let commit_files_before = app.revisions.view().commits.tree_mode.files.clone();
    let stash_files_before = app.revisions.view().stash.tree_mode.files.clone();
    let before = AppStateSnapshotOwned::from_app(&app.state);
    let cases = [
        (
            BackgroundLoad::BranchCommits {
                branch: "old-branch".into(),
                limit: 100,
            },
            BackgroundPayload::BranchCommits {
                branch: "old-branch".into(),
                items: app.revisions.view().commits.items.clone(),
            },
        ),
        (
            BackgroundLoad::CommitFiles {
                oid: "old-commit".into(),
            },
            BackgroundPayload::CommitFiles {
                oid: "old-commit".into(),
                files: files(),
            },
        ),
        (
            BackgroundLoad::StashFiles { index: 1 },
            BackgroundPayload::StashFiles {
                index: 1,
                files: files(),
            },
        ),
    ];
    for (load, payload) in cases {
        let request = register(&mut app, load);
        complete(
            &mut app,
            BackgroundCompleted {
                request,
                result: Ok(Arc::new(payload)),
            },
        );
    }
    assert!(std::ptr::eq(
        before.branches.view().commits_subview.items.as_ptr(),
        app.revisions.view().branches.commits_subview.items.as_ptr()
    ));
    assert!(commit_files_before.ptr_eq(&app.revisions.view().commits.tree_mode.files));
    assert!(stash_files_before.ptr_eq(&app.revisions.view().stash.tree_mode.files));
    assert!(app.tasks.is_empty());
}

#[test]
fn test_current_revision_files_adopt_shared_payload_and_keep_retained_snapshot() {
    let mut app = fixture(false);
    app.revisions.commits.tree_mode.selected_source = Some("current-commit".into());
    let files_before = app.revisions.view().commits.tree_mode.files.clone();
    let before = AppStateSnapshotOwned::from_app(&app.state);
    let files = files();
    let request = register(
        &mut app,
        BackgroundLoad::CommitFiles {
            oid: "current-commit".into(),
        },
    );
    complete(
        &mut app,
        BackgroundCompleted {
            request,
            result: Ok(Arc::new(BackgroundPayload::CommitFiles {
                oid: "current-commit".into(),
                files: files.clone(),
            })),
        },
    );
    assert!(files.ptr_eq(&app.revisions.view().commits.tree_mode.files));
    assert!(files_before.is_empty());
    assert!(before.commits.view().tree_mode.nodes.is_empty());
    assert!(std::ptr::eq(
        before.commits.view().items.as_ptr(),
        app.revisions.view().commits.items.as_ptr()
    ));
    assert!(app.revisions.view().commits.tree_mode.active);
}

#[test]
fn test_failure_completion_logs_once_and_releases_registration_before_retry() {
    let mut app = fixture(false);
    let request = register(&mut app, BackgroundLoad::Branches);
    let completed = BackgroundCompleted {
        request,
        result: Err("controlled read failure".into()),
    };
    let logs = app.command_log.len();
    complete(&mut app, completed.clone());
    complete(&mut app, completed);
    assert_eq!(app.command_log.len(), logs + 1);
    assert_eq!(
        app.command_log.last().unwrap().command,
        "controlled read failure"
    );
    assert!(app.tasks.is_empty());
    let request = register(&mut app, BackgroundLoad::Branches);
    complete(
        &mut app,
        BackgroundCompleted {
            request,
            result: Ok(Arc::new(BackgroundPayload::Branches(Default::default()))),
        },
    );
    assert!(app.revisions.view().branches.items.is_empty());
    assert!(app.tasks.is_empty());
}
