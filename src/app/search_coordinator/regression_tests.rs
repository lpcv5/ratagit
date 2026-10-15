//! Coordinator regressions use the production dispatcher and existing feature/navigation adapters.
use super::*;
use crate::app::{Command, InputMode};
use crate::file_tree::FileTreeNodeStatus;
use crate::flux::action::{Action, DomainAction, RuntimeEvent, SystemAction};
use crate::flux::dispatcher::Dispatcher;
use crate::flux::snapshot::AppStateSnapshotOwned;
use crate::flux::stores::test_support::MockRepo;
use crate::git::{BranchInfo, CommitInfo, CommitSyncState, FileStatus, StashInfo};

fn node(path: &str, is_dir: bool) -> FileTreeNode {
    FileTreeNode {
        path: path.into(),
        status: if is_dir {
            FileTreeNodeStatus::Directory
        } else {
            FileTreeNodeStatus::Unstaged(FileStatus::Modified)
        },
        depth: 0,
        is_dir,
        is_expanded: true,
    }
}

fn commit(oid: &str, message: &str) -> CommitInfo {
    CommitInfo {
        oid: oid.into(),
        message: message.into(),
        author: "Ada Lovelace".into(),
        graph: Vec::new(),
        time: "2026-10-15".into(),
        parent_count: 0,
        parent_oids: Vec::new(),
        sync_state: CommitSyncState::LocalOnly,
    }
}

fn fixture() -> App {
    let mut app = App::from_repo(Box::new(MockRepo)).unwrap();
    app.files.test_view_mut().tree_nodes = vec![
        node("parent", true),
        node("parent/leaf.rs", false),
        node("hidden", true),
    ]
    .into();
    app.revisions.branches.items = vec![BranchInfo {
        name: "feature/ui".into(),
        is_current: true,
    }]
    .into();
    app.revisions.commits.items = vec![
        commit("0123456789", "root message"),
        commit("second", "other"),
    ]
    .into();
    app.revisions.branches.commits_subview.items = vec![commit("abc", "branch message")].into();
    app.revisions.commits.tree_mode.nodes = vec![node("committed/file.rs", false)].into();
    app.revisions.stash.items = vec![StashInfo {
        index: 12,
        message: "stash message".into(),
    }]
    .into();
    app.revisions.stash.tree_mode.nodes = vec![node("stashed/file.rs", false)].into();
    app
}

fn dispatch(app: &mut App, dispatcher: &mut Dispatcher, action: Action) -> Vec<Command> {
    let envelope = dispatcher.next_envelope(action);
    dispatcher.dispatch(&mut app.state, envelope).commands
}

#[test]
fn test_data_adapter_preserves_all_panel_and_subview_search_text() {
    let mut app = fixture();
    for (panel, expected) in [
        (SidePanel::Files, vec!["parent/", "leaf.rs", "hidden/"]),
        (SidePanel::LocalBranches, vec!["feature/ui"]),
        (
            SidePanel::Commits,
            vec![
                "0123456 Ada Lovelace root message",
                "second Ada Lovelace other",
            ],
        ),
        (SidePanel::Stash, vec!["stash@{12} stash message"]),
    ] {
        app.ui.active_panel = panel;
        assert_eq!(app.searchable_items_for_scope(), expected);
    }
    app.ui.active_panel = SidePanel::LocalBranches;
    app.revisions.branches.commits_subview_active = true;
    assert_eq!(
        app.searchable_items_for_scope(),
        ["abc Ada Lovelace branch message"]
    );
    app.ui.active_panel = SidePanel::Commits;
    app.revisions.commits.tree_mode.active = true;
    assert_eq!(app.searchable_items_for_scope(), ["file.rs"]);
    app.ui.active_panel = SidePanel::Stash;
    app.revisions.stash.tree_mode.active = true;
    assert_eq!(app.searchable_items_for_scope(), ["file.rs"]);
}

#[test]
fn test_files_search_uses_visible_names_directory_suffix_and_excludes_parent_paths() {
    let mut app = fixture();
    assert_eq!(app.apply_search_query("parent/leaf".into()), 0);
    assert_eq!(app.apply_search_query("leaf.rs".into()), 1);
    assert_eq!(app.search.view().matches, &[1]);
    assert_eq!(app.apply_search_query("parent/".into()), 1);
    assert_eq!(app.search.view().matches, &[0]);
    // Children absent from the visible tree remain absent from the search dataset.
    assert_eq!(app.apply_search_query("hidden/child".into()), 0);
}

#[test]
fn test_input_query_navigation_and_diff_reload_follow_existing_action_roundtrip() {
    let mut app = fixture();
    let mut dispatcher = Dispatcher::with_default_stores();
    dispatch(
        &mut app,
        &mut dispatcher,
        Action::Domain(DomainAction::StartSearchInput),
    );
    let commands = dispatch(
        &mut app,
        &mut dispatcher,
        Action::Domain(DomainAction::InputChar('l')),
    );
    assert_eq!(app.input.buffer, "l");
    assert_eq!(app.search.view().query, "");
    assert!(
        matches!(commands.as_slice(), [Command::Sync(DomainAction::SearchSetQuery(query))] if query == "l")
    );
    let Command::Sync(query) = commands.into_iter().next().unwrap() else {
        unreachable!()
    };
    let commands = dispatch(&mut app, &mut dispatcher, Action::Domain(query));
    assert_eq!(app.search.view().query, "l");
    assert_eq!(app.files.view().panel.selected(), Some(1));
    assert!(matches!(
        commands.as_slice(),
        [Command::Sync(DomainAction::Runtime(
            RuntimeEvent::ReloadDiffNow
        ))]
    ));
    let Command::Sync(reload) = commands.into_iter().next().unwrap() else {
        unreachable!()
    };
    let commands = dispatch(&mut app, &mut dispatcher, Action::Domain(reload));
    assert!(
        matches!(commands.as_slice(), [Command::Effect(crate::flux::effects::EffectRequest::LoadBackground { request })]
        if matches!(request.load, crate::background::BackgroundLoad::Diff { .. }))
    );
}

#[test]
fn test_confirmation_keeps_query_and_cursor_while_clear_keeps_other_scopes() {
    let mut app = fixture();
    app.ui.active_panel = SidePanel::LocalBranches;
    app.apply_search_query("feature".into());
    app.ui.active_panel = SidePanel::Files;
    app.restore_search_for_active_scope();
    app.start_search_input();
    app.input.buffer = "leaf".into();
    app.apply_search_query("leaf".into());
    app.files.select_index(0);
    let mut dispatcher = Dispatcher::with_default_stores();
    let commands = dispatch(
        &mut app,
        &mut dispatcher,
        Action::Domain(DomainAction::InputEnter),
    );
    assert!(app.input.mode.is_none());
    assert!(app.input.buffer.is_empty());
    assert_eq!(app.search.view().query, "leaf");
    assert!(matches!(
        commands.as_slice(),
        [Command::Sync(DomainAction::SearchConfirm)]
    ));
    let commands = dispatch(
        &mut app,
        &mut dispatcher,
        Action::Domain(DomainAction::SearchConfirm),
    );
    assert_eq!(app.files.view().panel.selected(), Some(0));
    assert_eq!(
        app.command_log.last().unwrap().command,
        "search match 1: leaf"
    );
    assert!(matches!(
        commands.as_slice(),
        [Command::Sync(DomainAction::Runtime(
            RuntimeEvent::ReloadDiffNow
        ))]
    ));
    dispatch(
        &mut app,
        &mut dispatcher,
        Action::Domain(DomainAction::SearchClear),
    );
    assert!(app.search.view().query.is_empty());
    app.ui.active_panel = SidePanel::LocalBranches;
    app.restore_search_for_active_scope();
    assert_eq!(app.search.view().query, "feature");
}

#[test]
fn test_refresh_restores_query_against_new_data_without_repositioning_cursor() {
    let mut app = fixture();
    app.ui.active_panel = SidePanel::Commits;
    app.apply_search_query("message".into());
    app.revisions.commits.panel.select(Some(1));
    app.refresh_render_cache();
    let retained = AppStateSnapshotOwned::from_app(&app.state);
    let mut output = crate::flux::command_output::CommandOutput::default();
    app.queue_background(
        crate::background::BackgroundLoad::Commits {
            limit: 3,
            fast: false,
        },
        crate::background::TaskPriority::Normal,
        &mut output,
    );
    let Command::Effect(crate::flux::effects::EffectRequest::LoadBackground { request }) =
        output.drain().remove(0)
    else {
        unreachable!()
    };
    let payload = crate::background::BackgroundPayload::Commits(
        vec![
            commit("one", "message"),
            commit("two", "other"),
            commit("three", "message"),
        ]
        .into(),
    );
    let action = Action::System(SystemAction::BackgroundCompleted(Box::new(
        crate::background::BackgroundCompleted {
            request,
            result: Ok(std::sync::Arc::new(payload)),
        },
    )));
    dispatch(&mut app, &mut Dispatcher::with_default_stores(), action);
    assert_eq!(app.search.view().query, "message");
    assert_eq!(app.search.view().matches, &[0, 2]);
    assert_eq!(app.revisions.view().commits.panel.selected(), Some(1));
    app.refresh_render_cache();
    let current = AppStateSnapshotOwned::from_app(&app.state);
    assert_eq!(
        retained.render_cache.commits_search_summary.as_deref(),
        Some("/message 1/1")
    );
    assert_eq!(
        current.render_cache.commits_search_summary.as_deref(),
        Some("/message 1/2")
    );
}

#[test]
fn test_search_navigation_applies_to_revision_subview_cursor_only() {
    let mut app = fixture();
    app.ui.active_panel = SidePanel::LocalBranches;
    app.revisions.branches.commits_subview_active = true;
    app.revisions.branches.commits_subview_source = Some("feature/ui".into());
    app.revisions.branches.commits_subview.items = vec![
        commit("one", "match"),
        commit("two", "other"),
        commit("three", "match"),
    ]
    .into();
    app.apply_search_query("match".into());
    app.search_select_initial_match();
    assert!(app.search_jump_next());
    assert_eq!(
        app.revisions
            .view()
            .branches
            .commits_subview
            .panel
            .selected(),
        Some(2)
    );
    assert_eq!(app.revisions.view().branches.panel.selected(), Some(0));
    assert!(app.search_jump_next());
    assert_eq!(
        app.revisions
            .view()
            .branches
            .commits_subview
            .panel
            .selected(),
        Some(0)
    );
}

#[test]
fn test_open_input_preserves_capture_order_and_empty_match_jump_is_a_noop() {
    let mut app = fixture();
    app.apply_search_query("leaf".into());
    // Direct scope change exercises the legacy capture behavior without panel-switch restoration.
    app.ui.active_panel = SidePanel::LocalBranches;
    app.start_search_input();
    assert_eq!(app.input.mode, Some(InputMode::Search));
    assert_eq!(app.input.buffer, "leaf");
    assert_eq!(app.search.view().query, "leaf");
    assert_eq!(app.search.view().scope, &ViewContext::Branches);
    app.apply_search_query("not present".into());
    let mut dispatcher = Dispatcher::with_default_stores();
    for action in [DomainAction::SearchNext, DomainAction::SearchPrev] {
        assert!(dispatch(&mut app, &mut dispatcher, Action::Domain(action)).is_empty());
    }
}
