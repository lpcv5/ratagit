use super::measurement::{fixture, measure, Allocations};
use super::*;
use crate::app::revisions::RevisionAction;
use crate::flux::action::{Action, DomainAction};
use crate::flux::dispatcher::Dispatcher;
use crate::git::{FileEntry, FileStatus};

#[test]
fn test_render_view_borrows_all_payloads_without_allocations() {
    let mut app = fixture(false);
    app.ui.active_panel = SidePanel::LocalBranches;
    app.revisions.branches.commits_subview_active = true;
    app.revisions.branches.commits_subview_source = Some("feature/borrowed-view".into());
    app.git.operation.observed.conflicts = vec!["conflicted/file.rs".into()].into();
    app.git.operation.command = crate::git::operation::OperationCommandState::Failed {
        id: 1,
        error: "preserved failure".into(),
    };
    let mut document = app.diff.document().clone();
    document.patch = Some(std::sync::Arc::new(crate::git::patch::Patch {
        request: crate::git::patch::PatchRequest::Worktree("src/file.rs".into()),
        files: Vec::new(),
        truncated: false,
        preview: None,
    }));
    app.diff.set_test_document(document);
    let snapshot = AppStateSnapshotOwned::from_app(&app.state);
    let (view, allocations) = measure(|| snapshot.as_snapshot());
    assert_eq!(allocations, Allocations::default());
    assert!(std::ptr::eq(&*view.context, &snapshot.context));
    assert!(std::ptr::eq(
        view.available_commands.as_ptr(),
        snapshot.available_commands.as_ptr()
    ));
    assert!(std::ptr::eq(
        view.shortcut_hints.as_ptr(),
        snapshot.shortcut_hints.as_ptr()
    ));
    assert!(std::ptr::eq(
        view.command_log.as_ptr(),
        snapshot.command_log.as_ptr()
    ));
    assert!(std::ptr::eq(
        view.stash_targets.as_ptr(),
        snapshot.stash_targets.as_ptr()
    ));
    assert!(std::ptr::eq(view.operation, &snapshot.operation));
    assert!(std::ptr::eq(
        view.current_patch.unwrap(),
        snapshot.current_patch.as_deref().unwrap()
    ));
}

#[test]
fn test_snapshot_allocation_budget_is_independent_of_history_and_diff_size() {
    let small = fixture(false);
    let large = fixture(true);
    let (_, small_counts) = measure(|| AppStateSnapshotOwned::from_app(&small.state));
    let (_, large_counts) = measure(|| AppStateSnapshotOwned::from_app(&large.state));
    assert_eq!(small_counts, large_counts);
    // A budget tolerates small command metadata changes but catches a deep payload clone.
    assert!(large_counts.calls <= 256, "{large_counts:?}");
    assert!(large_counts.bytes <= 16_384, "{large_counts:?}");
}

#[test]
fn test_navigation_keeps_payloads_shared_and_old_cursor_unchanged() {
    let mut app = fixture(false);
    let before = AppStateSnapshotOwned::from_app(&app.state);
    let mut dispatcher = Dispatcher::with_default_stores();
    let envelope = dispatcher.next_envelope(Action::Domain(DomainAction::ListDown));
    dispatcher.dispatch(&mut app.state, envelope);
    app.refresh_render_cache();
    let after = AppStateSnapshotOwned::from_app(&app.state);
    assert_eq!(before.commits.view().panel.selected(), Some(0));
    assert_eq!(after.commits.view().panel.selected(), Some(1));
    assert!(std::ptr::eq(
        before.files.view().tree_nodes.as_ptr(),
        after.files.view().tree_nodes.as_ptr()
    ));
    assert!(std::ptr::eq(
        before.branches.view().items.as_ptr(),
        after.branches.view().items.as_ptr()
    ));
    assert!(std::ptr::eq(
        before.branches.view().commits_subview.items.as_ptr(),
        after.branches.view().commits_subview.items.as_ptr()
    ));
    assert!(std::ptr::eq(
        before.commits.view().items.as_ptr(),
        after.commits.view().items.as_ptr()
    ));
    assert!(std::ptr::eq(
        before.stash.view().items.as_ptr(),
        after.stash.view().items.as_ptr()
    ));
    assert!(before.current_diff.ptr_eq(&after.current_diff));
    assert!(before.keymap.ptr_eq(&after.keymap));
    assert!(before.command_log.ptr_eq(&after.command_log));
}

#[test]
fn test_feature_writes_detach_only_changed_payloads_from_retained_snapshot() {
    let mut app = fixture(false);
    let before = AppStateSnapshotOwned::from_app(&app.state);
    app.revisions.commits.items[0].message = "new message".into();
    app.revisions.branches.items[0].name = "new branch".into();
    app.revisions.stash.items[0].message = "new stash".into();
    app.files.test_view_mut().tree_nodes[0].path = "new/path.rs".into();
    app.push_log("new log", false);
    let after = AppStateSnapshotOwned::from_app(&app.state);
    assert_ne!(
        before.commits.view().items[0].message,
        after.commits.view().items[0].message
    );
    assert_ne!(
        before.branches.view().items[0].name,
        after.branches.view().items[0].name
    );
    assert_ne!(
        before.stash.view().items[0].message,
        after.stash.view().items[0].message
    );
    assert_ne!(
        before.files.view().tree_nodes[0].path,
        after.files.view().tree_nodes[0].path
    );
    assert_eq!(before.command_log.len() + 1, after.command_log.len());
    assert!(!std::ptr::eq(
        before.commits.view().items.as_ptr(),
        after.commits.view().items.as_ptr()
    ));
    assert!(std::ptr::eq(
        before.branches.view().commits_subview.items.as_ptr(),
        after.branches.view().commits_subview.items.as_ptr()
    ));
    assert!(before.current_diff.ptr_eq(&after.current_diff));
    assert!(before.keymap.ptr_eq(&after.keymap));
}

#[test]
fn test_tree_replacement_and_close_preserve_retained_snapshot() {
    let mut app = fixture(false);
    let oid = app.revisions.commits.items[0].oid.clone();
    let opened = app
        .revisions
        .update(RevisionAction::OpenCommitTree, SidePanel::Commits);
    assert!(opened.accepted);
    let loaded = app.revisions.update(
        RevisionAction::CommitFilesLoaded {
            oid: oid.clone(),
            files: vec![FileEntry {
                path: "dir/file.rs".into(),
                status: FileStatus::Modified,
            }]
            .into(),
        },
        SidePanel::Commits,
    );
    assert!(loaded.accepted);
    let before = AppStateSnapshotOwned::from_app(&app.state);
    app.revisions
        .update(RevisionAction::CloseCommitTree, SidePanel::Commits);
    let after = AppStateSnapshotOwned::from_app(&app.state);
    assert!(before.commits.view().tree_mode.active);
    assert!(!before.commits.view().tree_mode.nodes.is_empty());
    assert!(!after.commits.view().tree_mode.active);
    assert!(after.commits.view().tree_mode.nodes.is_empty());
    assert_eq!(
        before
            .commits
            .view()
            .tree_mode
            .selected_source
            .map(String::as_str),
        Some(oid.as_str())
    );
    assert!(std::ptr::eq(
        before.commits.view().items.as_ptr(),
        after.commits.view().items.as_ptr()
    ));
}
