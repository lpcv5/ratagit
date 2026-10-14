use super::*;
use crate::file_tree::FileTreeNodeStatus;
use crate::git::{FileEntry, FileStatus, GitStatus};
use std::path::{Path, PathBuf};

fn entry(path: &str) -> FileEntry {
    FileEntry {
        path: path.into(),
        status: FileStatus::Modified,
    }
}

fn state(unstaged: &[&str], staged: &[&str], untracked: &[&str]) -> FilesState {
    FilesState::new(
        GitStatus {
            unstaged: unstaged.iter().map(|path| entry(path)).collect(),
            staged: staged.iter().map(|path| entry(path)).collect(),
            untracked: untracked.iter().map(|path| entry(path)).collect(),
        },
        true,
    )
}

fn index(state: &FilesState, path: &str) -> usize {
    state
        .view()
        .tree_nodes
        .iter()
        .position(|node| node.path == Path::new(path))
        .unwrap()
}

fn select(state: &mut FilesState, path: &str) {
    state.select_index(index(state, path));
}

fn selected_path(state: &FilesState) -> &Path {
    &state.selection(true).selected_tree_node().unwrap().path
}

fn paths(names: &[&str]) -> Vec<PathBuf> {
    names.iter().map(PathBuf::from).collect()
}

#[test]
fn test_status_rebuild_restores_cursor_and_visual_anchor_by_path() {
    let mut state = state(&["dir/b.txt", "dir/z.txt"], &[], &[]);
    select(&mut state, "dir/b.txt");
    state.update(FilesAction::ToggleVisual, true);
    select(&mut state, "dir/z.txt");
    state.update(
        FilesAction::ReplaceStatus(GitStatus {
            unstaged: vec![entry("dir/a.txt"), entry("dir/b.txt"), entry("dir/z.txt")],
            ..Default::default()
        }),
        true,
    );
    assert_eq!(selected_path(&state), Path::new("dir/z.txt"));
    assert_eq!(state.view().visual_anchor, Some(index(&state, "dir/b.txt")));
    let selection = state.selection(true);
    assert_eq!(
        selection.collect_commit_targets(&selection.visual_selected_indices()),
        paths(&["dir/b.txt", "dir/z.txt"])
    );
}

#[test]
fn test_status_refresh_keeps_collapsed_dirs_and_expands_only_new_dirs() {
    let mut state = state(&["src/a.txt", "obsolete/a.txt"], &[], &[]);
    state.update(FilesAction::CollapseAll, true);
    state.update(
        FilesAction::ReplaceStatus(GitStatus {
            unstaged: vec![entry("src/b.txt"), entry("new/a.txt")],
            ..Default::default()
        }),
        true,
    );
    assert!(!state.view().expanded_dirs.contains(Path::new("src")));
    assert!(!state.view().expanded_dirs.contains(Path::new("obsolete")));
    assert!(state.view().expanded_dirs.contains(Path::new("new")));
    assert!(state
        .view()
        .tree_nodes
        .iter()
        .any(|node| node.path == Path::new("new/a.txt")));
    assert!(!state
        .view()
        .tree_nodes
        .iter()
        .any(|node| node.path == Path::new("src/b.txt")));
}

#[test]
fn test_collapsing_tree_restores_selection_and_anchor_to_visible_parent() {
    let mut state = state(&["src/deep/file.txt"], &[], &[]);
    select(&mut state, "src/deep/file.txt");
    state.update(FilesAction::ToggleVisual, true);
    state.update(FilesAction::CollapseAll, true);
    assert_eq!(selected_path(&state), Path::new("src"));
    assert_eq!(state.view().visual_anchor, Some(0));
    state.update(FilesAction::ExpandAll, true);
    assert_eq!(selected_path(&state), Path::new("src"));
    assert!(state
        .view()
        .tree_nodes
        .iter()
        .any(|node| node.path == Path::new("src/deep/file.txt")));
}

#[test]
fn test_refresh_preserves_staged_cursor_for_partially_staged_path() {
    let mut state = state(&["file.txt"], &["file.txt"], &[]);
    state.select_index(1);
    let mut status = state.status().clone();
    status.staged[0].status = FileStatus::New;
    state.update(FilesAction::ReplaceStatus(status), true);
    assert!(matches!(
        state.selection(true).selected_tree_node().unwrap().status,
        FileTreeNodeStatus::Staged(FileStatus::New)
    ));
    assert_eq!(
        state.selection(true).toggle_stage_target(),
        Some(StageTarget::Unstage("file.txt".into()))
    );
}

#[test]
fn test_reverse_visual_range_partitions_stage_and_unstage_targets_in_tree_order() {
    let mut state = state(&["a.txt"], &["b.txt"], &["c.txt"]);
    select(&mut state, "c.txt");
    state.update(FilesAction::ToggleVisual, true);
    select(&mut state, "a.txt");
    let update = state.update(FilesAction::ToggleStageSelection, true);
    assert_eq!(
        update.write,
        Some(FileWrite::ToggleSelection {
            stage: paths(&["a.txt", "c.txt"]),
            unstage: paths(&["b.txt"]),
        })
    );
    let update = state.update(FilesAction::PrepareCommitSelection, true);
    assert_eq!(
        update.write,
        Some(FileWrite::PrepareCommit(paths(&[
            "a.txt", "b.txt", "c.txt"
        ])))
    );
    // Requests describe targets without pretending that Git has completed.
    assert_eq!(state.status().staged[0].path, Path::new("b.txt"));
    assert!(state.view().visual_mode);
}

#[test]
fn test_fully_selected_directory_is_one_target_for_stage_commit_and_stash() {
    let mut state = state(&["dir/a.txt"], &["dir/b.txt"], &[]);
    select(&mut state, "dir");
    state.update(FilesAction::ToggleVisual, true);
    select(&mut state, "dir/b.txt");
    assert_eq!(
        state.update(FilesAction::ToggleStageSelection, true).write,
        Some(FileWrite::ToggleSelection {
            stage: paths(&["dir"]),
            unstage: vec![],
        })
    );
    assert_eq!(
        state
            .update(FilesAction::PrepareCommitSelection, true)
            .write,
        Some(FileWrite::PrepareCommit(paths(&["dir"])))
    );
    assert_eq!(
        state.selection(true).prepare_stash_targets_from_selection(),
        paths(&["dir"])
    );
    assert_eq!(
        state.update(FilesAction::DiscardSelection, true).write,
        Some(FileWrite::Discard(paths(&["dir/a.txt", "dir/b.txt"])))
    );
}

#[test]
fn test_partial_directory_selection_does_not_include_unselected_children() {
    let mut state = state(&["dir/a.txt", "dir/b.txt"], &[], &[]);
    select(&mut state, "dir");
    state.update(FilesAction::ToggleVisual, true);
    select(&mut state, "dir/a.txt");
    assert_eq!(
        state.update(FilesAction::ToggleStageSelection, true).write,
        Some(FileWrite::ToggleSelection {
            stage: paths(&["dir/a.txt"]),
            unstage: vec![],
        })
    );
    assert_eq!(
        state.selection(true).prepare_stash_targets_from_selection(),
        paths(&["dir/a.txt"])
    );
    assert_eq!(
        state.update(FilesAction::DiscardSelection, true).write,
        Some(FileWrite::Discard(paths(&["dir/a.txt"])))
    );
}

#[test]
fn test_fully_staged_directory_selects_unstage_operation() {
    let mut state = state(&[], &["dir/a.txt", "dir/b.txt"], &[]);
    select(&mut state, "dir");
    assert_eq!(
        state.selection(true).toggle_stage_target(),
        Some(StageTarget::Unstage("dir".into()))
    );
    state.update(FilesAction::ToggleVisual, true);
    select(&mut state, "dir/b.txt");
    assert_eq!(
        state.update(FilesAction::ToggleStageSelection, true).write,
        Some(FileWrite::ToggleSelection {
            stage: vec![],
            unstage: paths(&["dir"]),
        })
    );
}

#[test]
fn test_discard_excludes_untracked_paths_and_preserves_selection_on_failure() {
    let mut state = state(&["a.txt"], &["b.txt"], &["c.txt"]);
    select(&mut state, "a.txt");
    state.update(FilesAction::ToggleVisual, true);
    select(&mut state, "c.txt");
    assert_eq!(
        state.update(FilesAction::DiscardSelection, true).write,
        Some(FileWrite::Discard(paths(&["a.txt", "b.txt"])))
    );
    state.update(
        FilesAction::DiscardSelectionFinished(&Err("locked".into())),
        false,
    );
    assert!(state.view().visual_mode);
    assert_eq!(state.view().visual_anchor, Some(0));
    state.update(FilesAction::DiscardSelectionFinished(&Ok(())), false);
    assert!(!state.view().visual_mode);
    assert!(state.view().visual_anchor.is_none());
    let update = state.update(FilesAction::DiscardSelection, true);
    assert!(update.write.is_none());
    assert_eq!(
        update.log,
        Some((
            "discard blocked: no discardable selected items".into(),
            false
        ))
    );
}

#[test]
fn test_selection_requests_are_inactive_outside_files_but_completions_are_accepted() {
    let mut state = state(&["file.txt"], &[], &[]);
    for action in [
        FilesAction::ToggleVisual,
        FilesAction::ToggleStageSelection,
        FilesAction::DiscardSelection,
        FilesAction::PrepareCommitSelection,
    ] {
        let update = state.update(action, false);
        assert!(update.write.is_none());
        assert!(update.log.is_none());
        assert!(!update.reload_diff && !update.redraw);
    }
    assert!(!state.selection(false).is_visual_mode());
    assert!(state.selection(false).selected_tree_node().is_none());
    state.update(FilesAction::ToggleVisual, true);
    let update = state.update(FilesAction::PrepareCommitFinished(&Ok(1)), false);
    assert!(!state.view().visual_mode);
    assert!(update.refresh_status && update.open_commit_editor && update.redraw);
    assert_eq!(
        update.log,
        Some((
            "commit: 1 selected target(s) staged; edit message/description".into(),
            true
        ))
    );
}

#[test]
fn test_file_writes_and_failure_logs_leave_observed_status_unchanged() {
    let mut state = state(&["file.txt"], &[], &[]);
    let update = state.update(FilesAction::StagePath(Path::new("file.txt")), false);
    assert_eq!(update.write, Some(FileWrite::Stage("file.txt".into())));
    let update = state.update(
        FilesAction::StageFinished {
            path: Path::new("file.txt"),
            result: &Ok(()),
        },
        false,
    );
    assert_eq!(update.log, Some(("staged file.txt".into(), true)));
    assert!(update.redraw);
    assert!(state.status().staged.is_empty());
    assert_eq!(state.status().unstaged[0].path, Path::new("file.txt"));
    let update = state.update(
        FilesAction::UnstageFinished {
            path: Path::new("file.txt"),
            result: &Err("index locked".into()),
        },
        false,
    );
    assert_eq!(
        update.log,
        Some(("unstage failed file.txt: index locked".into(), false))
    );
    assert!(!update.redraw && !update.refresh_status);
    assert!(update.write.is_none());
}

#[test]
fn test_navigation_bounds_and_empty_commit_preparation() {
    let mut state = state(&["a.txt", "b.txt"], &[], &[]);
    state.update(FilesAction::Previous, true);
    assert_eq!(selected_path(&state), Path::new("a.txt"));
    state.update(FilesAction::Next, true);
    state.update(FilesAction::Next, true);
    assert_eq!(selected_path(&state), Path::new("b.txt"));
    let mut empty = FilesState::default();
    empty.update(FilesAction::Next, true);
    empty.update(FilesAction::Previous, true);
    assert!(empty.selection(true).selected_tree_node().is_none());
    assert_eq!(
        empty
            .update(FilesAction::PrepareCommitSelection, true)
            .write,
        Some(FileWrite::PrepareCommit(vec![]))
    );
    let update = empty.update(FilesAction::PrepareCommitFinished(&Ok(0)), true);
    assert!(!update.open_commit_editor && !update.refresh_status);
    assert_eq!(
        update.log,
        Some(("commit blocked: no selected items".into(), false))
    );
}
