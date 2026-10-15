use super::measurement::{fixture, measure, Allocations};
use super::*;
use crate::app::revisions::RevisionAction;
use crate::app::{dispatch_test_action, App};
use crate::flux::action::DomainAction;
use crate::git::{FileEntry, FileStatus};

fn enter_view(app: &mut App, case: usize) {
    let files = || {
        vec![FileEntry {
            path: "dir/file.rs".into(),
            status: FileStatus::Modified,
        }]
        .into()
    };
    let panel = match case {
        0 => SidePanel::Files,
        1 | 2 => SidePanel::LocalBranches,
        3 | 4 => SidePanel::Commits,
        _ => SidePanel::Stash,
    };
    app.ui.active_panel = panel;
    match case {
        2 => {
            app.revisions
                .update(RevisionAction::OpenBranchCommits { limit: 30 }, panel);
            let branch = app
                .revisions
                .view()
                .branches
                .commits_subview_source
                .clone()
                .unwrap();
            let items = app.revisions.view().commits.items.clone();
            app.revisions
                .update(RevisionAction::BranchCommitsLoaded { branch, items }, panel);
        }
        4 => {
            app.revisions.commits.dirty = false;
            app.revisions.update(RevisionAction::OpenCommitTree, panel);
            let oid = app
                .revisions
                .view()
                .commits
                .tree_mode
                .selected_source
                .clone()
                .unwrap();
            app.revisions.update(
                RevisionAction::CommitFilesLoaded {
                    oid,
                    files: files(),
                },
                panel,
            );
        }
        6 => {
            app.revisions.update(RevisionAction::OpenStashTree, panel);
            let index = app
                .revisions
                .view()
                .stash
                .tree_mode
                .selected_source
                .unwrap();
            app.revisions.update(
                RevisionAction::StashFilesLoaded {
                    index,
                    files: files(),
                },
                panel,
            );
        }
        _ => {}
    }
    app.refresh_render_cache();
}

#[test]
fn test_render_models_and_domain_selectors_agree_through_navigation_in_every_view() {
    for case in 0..7 {
        let mut app = fixture(false);
        enter_view(&mut app, case);
        let retained = AppStateSnapshotOwned::from_app(&app.state);
        let retained_index = retained
            .as_snapshot()
            .command_env()
            .selection
            .selected_index();
        let retained_target = retained
            .as_snapshot()
            .command_env()
            .selection
            .diff_target()
            .cache_key(retained.files.view().tree_nodes);
        for action in [
            DomainAction::ListDown,
            DomainAction::ListDown,
            DomainAction::ListUp,
        ] {
            dispatch_test_action(&mut app, action);
            let published = AppStateSnapshotOwned::from_app(&app.state);
            let view = published.as_snapshot();
            let rendered = view.command_env();
            let domain = app.selection();
            assert_eq!(rendered.selection.context, domain.context, "case={case}");
            assert_eq!(rendered.selection.selected_index(), domain.selected_index());
            assert_eq!(rendered.selection.visible_count(), domain.visible_count());
            assert_eq!(
                rendered
                    .selection
                    .diff_target()
                    .cache_key(view.files.tree_nodes),
                domain
                    .diff_target()
                    .cache_key(app.files.view().render_view().tree_nodes)
            );
            assert_eq!(
                rendered
                    .selection
                    .file_selection()
                    .visual_selected_indices(),
                domain.file_selection().visual_selected_indices()
            );
        }
        assert_eq!(
            retained
                .as_snapshot()
                .command_env()
                .selection
                .selected_index(),
            retained_index
        );
        assert_eq!(
            retained
                .as_snapshot()
                .command_env()
                .selection
                .diff_target()
                .cache_key(retained.files.view().tree_nodes),
            retained_target
        );
    }
}

#[test]
fn test_published_render_models_do_not_retain_domain_tree_bookkeeping() {
    let mut app = fixture(false);
    let entries: Vec<_> = (0..100)
        .map(|i| FileEntry {
            path: format!("dir_{i}/file.rs").into(),
            status: FileStatus::Modified,
        })
        .collect();
    // Each internal collection starts uniquely owned. Publishing must not add
    // owners to data that renderers and command selectors never read.
    app.revisions.commits.tree_mode.files = entries.clone().into();
    app.revisions.stash.tree_mode.files = entries.into();
    app.revisions.commits.tree_mode.expanded_dirs =
        (0..100).map(|i| format!("dir_{i}").into()).collect();
    app.revisions.stash.tree_mode.expanded_dirs =
        (0..100).map(|i| format!("dir_{i}").into()).collect();
    let snapshot = AppStateSnapshotOwned::from_app(&app.state);
    let (_, allocations) = measure(|| {
        app.files.test_view_mut().expanded_dirs.retain(|_| true);
        app.revisions
            .commits
            .tree_mode
            .expanded_dirs
            .retain(|_| true);
        app.revisions.stash.tree_mode.expanded_dirs.retain(|_| true);
        app.revisions.commits.tree_mode.files[0].status = FileStatus::Deleted;
        app.revisions.stash.tree_mode.files[0].status = FileStatus::Deleted;
    });
    assert_eq!(allocations, Allocations::default());
    assert_eq!(
        snapshot.files.view().tree_nodes[1].path,
        app.files.view().tree_nodes[1].path
    );
}

#[test]
fn test_retained_branch_subview_owns_source_loading_cursor_and_highlights() {
    let mut app = fixture(false);
    enter_view(&mut app, 2);
    app.revisions.branches.commits_subview.panel.select(Some(3));
    app.revisions
        .branches
        .commits_subview
        .highlighted_oids
        .insert("old-highlight".into());
    let retained = AppStateSnapshotOwned::from_app(&app.state);
    let source = retained
        .branches
        .view()
        .commits_subview_source
        .unwrap()
        .clone();
    app.revisions
        .update(RevisionAction::CloseBranchCommits, SidePanel::LocalBranches);
    app.revisions.branches.panel.select(Some(1));
    app.revisions.update(
        RevisionAction::OpenBranchCommits { limit: 30 },
        SidePanel::LocalBranches,
    );
    let current = AppStateSnapshotOwned::from_app(&app.state);
    assert!(current.branches.view().commits_subview_loading);
    assert_eq!(
        current.branches.view().commits_subview.panel.selected(),
        None
    );
    assert_ne!(
        current.branches.view().commits_subview_source,
        Some(&source)
    );
    assert!(!retained.branches.view().commits_subview_loading);
    assert_eq!(
        retained.branches.view().commits_subview.panel.selected(),
        Some(3)
    );
    assert_eq!(
        retained.branches.view().commits_subview_source,
        Some(&source)
    );
    assert!(retained
        .branches
        .view()
        .commits_subview
        .highlighted_oids
        .contains("old-highlight"));
    assert!(!current
        .branches
        .view()
        .commits_subview
        .highlighted_oids
        .contains("old-highlight"));
}

#[test]
fn test_feature_render_views_borrow_without_allocation_in_every_view() {
    for case in 0..7 {
        let mut app = fixture(false);
        enter_view(&mut app, case);
        let snapshot = AppStateSnapshotOwned::from_app(&app.state);
        let (_, allocations) = measure(|| {
            std::hint::black_box(app.files.view().render_view());
            std::hint::black_box(app.revisions.view().branches.render_view());
            std::hint::black_box(app.revisions.view().commits.render_view());
            std::hint::black_box(app.revisions.view().stash.render_view());
            std::hint::black_box(snapshot.as_snapshot());
        });
        assert_eq!(allocations, Allocations::default(), "case={case}");
    }
}
