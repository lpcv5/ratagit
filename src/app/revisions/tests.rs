use super::{RevisionAction as Action, RevisionCancel, RevisionRead, RevisionState};
use crate::app::{SidePanel, ViewContext};
use crate::git::{BranchInfo, CommitInfo, CommitSyncState, FileEntry, FileStatus, StashInfo};

fn commit(oid: &str) -> CommitInfo {
    CommitInfo {
        oid: oid.into(),
        message: oid.into(),
        author: "test".into(),
        graph: Vec::new(),
        time: String::new(),
        parent_count: 0,
        sync_state: CommitSyncState::LocalOnly,
        parent_oids: Vec::new(),
    }
}

fn branch(name: &str) -> BranchInfo {
    BranchInfo {
        name: name.into(),
        is_current: name == "main",
    }
}

fn files() -> Vec<FileEntry> {
    vec![FileEntry {
        path: "src/file.rs".into(),
        status: FileStatus::Modified,
    }]
}

fn fixture() -> RevisionState {
    RevisionState::new(
        vec![branch("main"), branch("feature")],
        vec![commit("new"), commit("old")],
        vec![
            StashInfo {
                index: 7,
                message: "first".into(),
            },
            StashInfo {
                index: 12,
                message: "second".into(),
            },
        ],
        true,
    )
}

#[test]
fn test_branch_subview_accepts_only_current_source_and_restores_parent_identity() {
    let mut state = fixture();
    state.update(Action::SelectIndex(1), SidePanel::LocalBranches);
    let output = state.update(
        Action::OpenBranchCommits { limit: 100 },
        SidePanel::LocalBranches,
    );
    assert_eq!(
        output.read,
        Some(RevisionRead::BranchCommits {
            branch: "feature".into(),
            limit: 100
        })
    );
    assert_eq!(
        state.selection(SidePanel::LocalBranches).context,
        ViewContext::BranchCommits {
            branch: "feature".into()
        }
    );
    assert!(state.view().branches.commits_subview_loading);
    assert!(state
        .selection(SidePanel::LocalBranches)
        .selected_commit_oid()
        .is_none());

    let stale = state.update(
        Action::BranchCommitsLoaded {
            branch: "main".into(),
            items: vec![commit("wrong")].into(),
        },
        SidePanel::Files,
    );
    assert!(!stale.accepted);
    assert!(state.view().branches.commits_subview_loading);
    let accepted = state.update(
        Action::BranchCommitsLoaded {
            branch: "feature".into(),
            items: vec![commit("right")].into(),
        },
        SidePanel::Files,
    );
    assert!(
        accepted.accepted,
        "completion stays valid after focus changes"
    );
    assert_eq!(
        state
            .selection(SidePanel::LocalBranches)
            .selected_commit_oid(),
        Some("right")
    );

    state.update(
        Action::ReplaceBranches(vec![branch("feature"), branch("main")].into()),
        SidePanel::Files,
    );
    state.update(Action::CloseBranchCommits, SidePanel::LocalBranches);
    assert_eq!(
        state
            .selection(SidePanel::LocalBranches)
            .selected_branch_name(),
        Some("feature")
    );
    assert_eq!(
        state.selection(SidePanel::LocalBranches).selected_index(),
        Some(0)
    );
    assert!(state.view().branches.commits_subview.items.is_empty());
    assert!(
        !state
            .update(
                Action::BranchCommitsLoaded {
                    branch: "feature".into(),
                    items: vec![commit("late")].into()
                },
                SidePanel::Files
            )
            .accepted
    );
}

#[test]
fn test_empty_branch_subview_has_no_selection_and_close_handles_removed_source() {
    let mut state = fixture();
    state.update(
        Action::OpenBranchCommits { limit: 10 },
        SidePanel::LocalBranches,
    );
    state.update(
        Action::BranchCommitsLoaded {
            branch: "main".into(),
            items: Vec::new().into(),
        },
        SidePanel::LocalBranches,
    );
    assert_eq!(
        state.selection(SidePanel::LocalBranches).selected_index(),
        None
    );
    assert!(!state.view().branches.commits_subview_loading);
    state.update(
        Action::ReplaceBranches(vec![branch("remaining")].into()),
        SidePanel::Files,
    );
    state.update(Action::CloseBranchCommits, SidePanel::LocalBranches);
    assert_eq!(
        state
            .selection(SidePanel::LocalBranches)
            .selected_branch_name(),
        Some("remaining")
    );
    state.update(
        Action::OpenBranchCommits { limit: 10 },
        SidePanel::LocalBranches,
    );
    state.update(Action::ReplaceBranches(Vec::new().into()), SidePanel::Files);
    state.update(Action::CloseBranchCommits, SidePanel::LocalBranches);
    assert_eq!(
        state.selection(SidePanel::LocalBranches).selected_index(),
        None
    );
}

#[test]
fn test_commit_tree_requires_current_history_and_matching_completion_source() {
    let mut state = fixture();
    state.update(Action::MarkCommitsDirty, SidePanel::Files);
    let output = state.update(Action::OpenCommitTree, SidePanel::Commits);
    assert_eq!(output.read, Some(RevisionRead::Commits { limit: 30 }));
    assert!(state.view().commits.tree_mode.selected_source.is_none());
    state.update(
        Action::ReplaceCommits {
            items: vec![commit("old")].into(),
            fast: false,
        },
        SidePanel::Files,
    );
    let output = state.update(Action::OpenCommitTree, SidePanel::Commits);
    assert_eq!(
        output.read,
        Some(RevisionRead::CommitFiles { oid: "old".into() })
    );
    assert!(
        !state.view().commits.tree_mode.active,
        "tree opens on completion, not on request"
    );
    assert!(
        !state
            .update(
                Action::CommitFilesLoaded {
                    oid: "new".into(),
                    files: files().into()
                },
                SidePanel::Commits
            )
            .accepted
    );
    assert!(
        state
            .update(
                Action::CommitFilesLoaded {
                    oid: "old".into(),
                    files: files().into()
                },
                SidePanel::Files
            )
            .accepted
    );
    assert!(state.view().commits.tree_mode.active);
    assert_eq!(state.selection(SidePanel::Commits).visible_count(), 2);
}

#[test]
fn test_commit_tree_toggle_keeps_source_and_close_restores_oid_after_reorder() {
    let mut state = fixture();
    state.update(Action::SelectIndex(1), SidePanel::Commits);
    state.update(Action::OpenCommitTree, SidePanel::Commits);
    state.update(
        Action::CommitFilesLoaded {
            oid: "old".into(),
            files: files().into(),
        },
        SidePanel::Commits,
    );
    let output = state.update(Action::OpenCommitTree, SidePanel::Commits);
    assert!(
        output.read.is_none(),
        "Enter on a directory toggles it without Git I/O"
    );
    assert_eq!(state.selection(SidePanel::Commits).visible_count(), 1);
    assert_eq!(
        state.selection(SidePanel::Commits).selected_commit_oid(),
        Some("old")
    );
    state.update(
        Action::ReplaceCommits {
            items: vec![commit("old"), commit("new")].into(),
            fast: false,
        },
        SidePanel::Commits,
    );
    let output = state.update(Action::CloseCommitTree, SidePanel::Commits);
    assert_eq!(output.cancel, Some(RevisionCancel::CommitFiles));
    assert_eq!(
        state.selection(SidePanel::Commits).selected_index(),
        Some(0)
    );
    assert!(state.view().commits.tree_mode.nodes.is_empty());
    assert!(state.view().commits.tree_mode.files.is_empty());
    assert!(state.view().commits.tree_mode.expanded_dirs.is_empty());
    assert!(state.view().commits.tree_mode.selected_source.is_none());
}

#[test]
fn test_closing_pending_commit_tree_rejects_late_completion_without_moving_cursor() {
    let mut state = fixture();
    state.update(Action::SelectIndex(1), SidePanel::Commits);
    state.update(Action::OpenCommitTree, SidePanel::Commits);
    let output = state.update(Action::CloseCommitTree, SidePanel::Files);
    assert_eq!(output.cancel, Some(RevisionCancel::CommitFiles));
    assert_eq!(
        state.selection(SidePanel::Commits).selected_index(),
        Some(1)
    );
    assert!(
        !state
            .update(
                Action::CommitFilesLoaded {
                    oid: "old".into(),
                    files: files().into()
                },
                SidePanel::Files
            )
            .accepted
    );
    assert!(!state.view().commits.tree_mode.active);
}

#[test]
fn test_stash_tree_uses_stash_identity_instead_of_list_offset() {
    let mut state = fixture();
    state.update(Action::SelectIndex(1), SidePanel::Stash);
    let output = state.update(Action::OpenStashTree, SidePanel::Stash);
    assert_eq!(output.read, Some(RevisionRead::StashFiles { index: 12 }));
    assert!(
        !state
            .update(
                Action::StashFilesLoaded {
                    index: 1,
                    files: files().into()
                },
                SidePanel::Stash
            )
            .accepted
    );
    assert!(
        state
            .update(
                Action::StashFilesLoaded {
                    index: 12,
                    files: files().into()
                },
                SidePanel::Files
            )
            .accepted
    );
    assert_eq!(
        state.selection(SidePanel::Stash).selected_stash_index(),
        Some(12)
    );
    state.update(
        Action::ReplaceStashes(
            vec![StashInfo {
                index: 12,
                message: "moved".into(),
            }]
            .into(),
        ),
        SidePanel::Files,
    );
    let output = state.update(Action::CloseStashTree, SidePanel::Stash);
    assert_eq!(output.cancel, Some(RevisionCancel::StashFiles));
    assert_eq!(state.selection(SidePanel::Stash).selected_index(), Some(0));
    assert!(
        !state
            .update(
                Action::StashFilesLoaded {
                    index: 12,
                    files: files().into()
                },
                SidePanel::Stash
            )
            .accepted
    );
}

#[test]
fn test_empty_tree_and_removed_source_close_to_empty_parent() {
    let mut state = fixture();
    state.update(Action::OpenStashTree, SidePanel::Stash);
    state.update(
        Action::StashFilesLoaded {
            index: 7,
            files: Vec::new().into(),
        },
        SidePanel::Stash,
    );
    assert!(state.view().stash.tree_mode.active);
    assert_eq!(state.selection(SidePanel::Stash).selected_index(), None);
    state.update(Action::ReplaceStashes(Vec::new().into()), SidePanel::Stash);
    state.update(Action::CloseStashTree, SidePanel::Stash);
    assert_eq!(state.selection(SidePanel::Stash).selected_index(), None);
    assert!(state.view().stash.tree_mode.selected_source.is_none());
}

#[test]
fn test_revision_open_requests_are_scoped_to_their_panels() {
    let mut state = fixture();
    for action in [
        Action::OpenBranchCommits { limit: 100 },
        Action::OpenCommitTree,
        Action::OpenStashTree,
    ] {
        let output = state.update(action, SidePanel::Files);
        assert!(output.read.is_none());
        assert!(!output.accepted);
    }
    assert!(state.view().commits.tree_mode.selected_source.is_none());
    assert!(state.view().stash.tree_mode.selected_source.is_none());
}

#[test]
fn test_navigation_in_branch_subview_does_not_move_parent_cursor() {
    let mut state = fixture();
    state.update(Action::SelectIndex(1), SidePanel::LocalBranches);
    state.update(
        Action::OpenBranchCommits { limit: 100 },
        SidePanel::LocalBranches,
    );
    state.update(
        Action::BranchCommitsLoaded {
            branch: "feature".into(),
            items: vec![commit("one"), commit("two")].into(),
        },
        SidePanel::LocalBranches,
    );
    state.update(Action::SelectIndex(1), SidePanel::LocalBranches);
    assert_eq!(
        state
            .selection(SidePanel::LocalBranches)
            .selected_commit_oid(),
        Some("two")
    );
    assert_eq!(state.view().branches.panel.selected(), Some(1));
}

#[test]
fn test_commit_pagination_respects_threshold_loading_dirty_and_tree_mode() {
    let mut state = fixture();
    state.update(
        Action::ReplaceCommits {
            items: (0..30).map(|index| commit(&index.to_string())).collect(),
            fast: false,
        },
        SidePanel::Commits,
    );
    state.update(Action::SelectIndex(21), SidePanel::Commits);
    state.update(Action::ExtendCommits { loading: false }, SidePanel::Commits);
    assert_eq!(state.commits_requested_limit(), 30);
    state.update(Action::SelectIndex(22), SidePanel::Commits);
    state.update(Action::ExtendCommits { loading: true }, SidePanel::Commits);
    state.update(Action::ExtendCommits { loading: false }, SidePanel::Files);
    assert_eq!(state.commits_requested_limit(), 30);
    state.update(Action::ExtendCommits { loading: false }, SidePanel::Commits);
    assert_eq!(state.commits_requested_limit(), 70);
    assert!(state.view().commits.dirty);
    state.update(Action::ExtendCommits { loading: false }, SidePanel::Commits);
    assert_eq!(
        state.commits_requested_limit(),
        70,
        "dirty history must not extend twice"
    );
    state.update(Action::TrackCommitsLoad(10), SidePanel::Commits);
    assert_eq!(
        state.commits_requested_limit(),
        70,
        "requested limit never shrinks"
    );
    state.update(
        Action::ReplaceCommits {
            items: vec![commit("only")].into(),
            fast: false,
        },
        SidePanel::Commits,
    );
    state.update(Action::SelectIndex(0), SidePanel::Commits);
    state.update(Action::OpenCommitTree, SidePanel::Commits);
    state.update(
        Action::CommitFilesLoaded {
            oid: "only".into(),
            files: files().into(),
        },
        SidePanel::Commits,
    );
    state.update(Action::ExtendCommits { loading: false }, SidePanel::Commits);
    assert_eq!(state.commits_requested_limit(), 70);
}

#[test]
fn test_fast_history_requires_full_load_and_highlight_is_cleared_on_focus_change() {
    let mut state = fixture();
    state.update(
        Action::ReplaceCommits {
            items: vec![commit("fast")].into(),
            fast: true,
        },
        SidePanel::Commits,
    );
    assert!(state.view().commits.dirty);
    assert_eq!(
        state
            .update(Action::OpenCommitTree, SidePanel::Commits)
            .read,
        Some(RevisionRead::Commits { limit: 30 })
    );
    state.update(
        Action::ReplaceCommits {
            items: vec![commit("full")].into(),
            fast: false,
        },
        SidePanel::Commits,
    );
    state.update(Action::RecomputeHighlight, SidePanel::Commits);
    assert!(state.view().commits.highlighted_oids.contains("full"));
    state.update(Action::RecomputeHighlight, SidePanel::Files);
    assert!(state.view().commits.highlighted_oids.is_empty());
}
