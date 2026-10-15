use super::*;
use crate::app::App;
use crate::file_tree::{FileTreeNode, FileTreeNodeStatus};
use crate::flux::snapshot::AppStateSnapshotOwned;
use crate::flux::stores::test_support::MockRepo;
use crate::git::{CommitInfo, GraphCell};

fn app() -> App {
    App::from_repo(Box::new(MockRepo)).unwrap()
}

fn commit(graph: Vec<GraphCell>) -> CommitInfo {
    CommitInfo {
        oid: "123456789abcdef".into(),
        message: "Provided commit".into(),
        author: "Ada Lovelace".into(),
        graph,
        time: String::new(),
        parent_count: 0,
        parent_oids: Vec::new(),
        sync_state: CommitSyncState::RemoteBranch,
    }
}

fn tree() -> Vec<FileTreeNode> {
    vec![FileTreeNode {
        path: "provided.rs".into(),
        status: FileTreeNodeStatus::Untracked,
        depth: 0,
        is_dir: false,
        is_expanded: false,
    }]
}

#[test]
fn test_branch_history_focus_follows_its_container_and_diff_focus() {
    let mut app = app();
    app.revisions.branches.commits_subview_active = true;
    app.revisions.branches.commits_subview_source = Some("feature/ui".into());
    app.revisions.branches.commits_subview_loading = true;
    let mut snapshot = AppStateSnapshotOwned::from_app(&app);
    for active_panel in [
        SidePanel::Files,
        SidePanel::LocalBranches,
        SidePanel::Commits,
        SidePanel::Stash,
    ] {
        for diff_focused in [false, true] {
            snapshot.active_panel = active_panel;
            snapshot.context.diff_focused = diff_focused;
            let view = snapshot.as_snapshot();
            let BranchesPanelProps::Commits { props, view_id } = view.branches_panel_props() else {
                panic!("branch history must be projected as commits");
            };
            assert_eq!(view_id, Some("feature/ui"));
            assert_eq!(
                props.header.is_active,
                active_panel == SidePanel::LocalBranches && !diff_focused
            );
            assert_eq!(
                props.header.title,
                "Branch Commits: feature/ui [Loading...] [Esc Back]"
            );
        }
    }
}

#[test]
fn test_graph_emphasis_resolves_overlapping_owners_and_legacy_fallback() {
    let mut app = app();
    let cell = |text: &str, primary: Option<&str>, owners: &[&str]| GraphCell {
        text: text.into(),
        lane: 0,
        pipe_oid: primary.map(str::to_owned),
        pipe_oids: owners.iter().map(|oid| (*oid).to_owned()).collect(),
    };
    app.revisions.commits.items = vec![commit(vec![
        cell("a", Some("other"), &["other", "focus"]),
        cell("b", Some("focus"), &["other"]),
        cell("c", Some("focus"), &[]),
        cell("d", None, &[]),
    ])]
    .into();
    app.revisions
        .commits
        .highlighted_oids
        .insert("focus".into());
    let retained = AppStateSnapshotOwned::from_app(&app);
    let view = retained.as_snapshot();
    let props = view.commits_panel_props();
    let RevisionContent::List(rows) = props.content else {
        panic!("expected commit rows")
    };
    assert!(matches!(rows[0].tone, Tone::Warning));
    assert_eq!(
        rows[0]
            .graph
            .iter()
            .map(|cell| cell.dimmed)
            .collect::<Vec<_>>(),
        vec![false, true, false, true]
    );

    app.revisions.commits.highlighted_oids.clear();
    let current = AppStateSnapshot::from_app(&app);
    let RevisionContent::List(rows) = current.commits_panel_props().content else {
        panic!("expected commit rows")
    };
    assert!(rows[0].graph.iter().all(|cell| !cell.dimmed));
    let RevisionContent::List(retained_rows) = view.commits_panel_props().content else {
        panic!("expected retained rows")
    };
    assert!(retained_rows[0].graph[1].dimmed);
}

#[test]
fn test_tree_props_keep_scope_query_title_and_opaque_viewport_identity() {
    let mut app = app();
    app.revisions.commits.items = vec![commit(Vec::new()); 3].into();
    app.revisions.commits.tree_mode.active = true;
    app.revisions.commits.tree_mode.selected_source = Some("123456789abcdef".into());
    app.revisions.commits.tree_mode.nodes = tree().into();
    app.revisions.stash.tree_mode.active = true;
    app.revisions.stash.tree_mode.selected_source = Some(42);
    app.revisions.stash.tree_mode.nodes = tree().into();
    app.ui.active_panel = SidePanel::Commits;
    let mut retained = AppStateSnapshotOwned::from_app(&app);
    retained.commits_search_query = Some("provided".into());
    retained.stash_search_query = Some("stash query".into());
    let view = retained.as_snapshot();
    let props = view.commits_panel_props();
    assert_eq!(props.header.title, "Commit Files 1234567 [Esc Back]");
    assert_eq!(props.header.search_query, Some("provided"));
    assert_eq!(view.side_panel_layout_props().focus_item_count, 3);
    let RevisionContent::Tree { nodes, view_id } = props.content else {
        panic!("expected file tree")
    };
    assert_eq!(view_id.as_deref(), Some("123456789abcdef"));
    assert!(std::ptr::eq(
        nodes.as_ptr(),
        view.commits.tree_mode.nodes.as_ptr()
    ));
    let props = view.stash_panel_props();
    assert_eq!(props.header.title, "Stash Files stash@{42} [Esc Back]");
    assert_eq!(props.header.search_query, Some("stash query"));
    assert!(!props.header.is_active);
    let RevisionContent::Tree { nodes, view_id } = props.content else {
        panic!("expected stash tree")
    };
    assert_eq!(view_id.as_deref(), Some("42"));
    assert!(std::ptr::eq(
        nodes.as_ptr(),
        view.stash.tree_mode.nodes.as_ptr()
    ));
}
