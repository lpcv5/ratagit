use super::states::PanelState;
use crate::file_tree::{build_file_tree, collect_dirs, FileTreeNode, TreeCursor};
use crate::git::GitStatus;
use std::collections::HashSet;
use std::path::PathBuf;

pub(super) fn collect_all_dirs(status: &GitStatus) -> HashSet<PathBuf> {
    collect_dirs(
        status
            .unstaged
            .iter()
            .chain(&status.untracked)
            .chain(&status.staged)
            .map(|entry| entry.path.as_path()),
    )
}
pub(super) fn toggle_selected_dir(
    expanded_dirs: &mut HashSet<PathBuf>,
    selected_dir_path: Option<PathBuf>,
) {
    let Some(path) = selected_dir_path else {
        return;
    };
    if expanded_dirs.contains(&path) {
        expanded_dirs.remove(&path);
    } else {
        expanded_dirs.insert(path);
    }
}

pub(super) fn collapse_all(expanded_dirs: &mut HashSet<PathBuf>) {
    expanded_dirs.clear();
}

pub(super) fn expand_all(expanded_dirs: &mut HashSet<PathBuf>, status: &GitStatus) {
    *expanded_dirs = collect_all_dirs(status);
}

pub(super) fn rebuild_tree(
    status: &GitStatus,
    expanded_dirs: &HashSet<PathBuf>,
    file_tree_nodes: &mut Vec<FileTreeNode>,
    files_panel: &mut PanelState,
    files_visual_anchor: &mut Option<usize>,
) {
    let cursor = TreeCursor::capture(file_tree_nodes, files_panel.list_state.selected());
    let anchor = files_visual_anchor.map(|index| TreeCursor::capture(file_tree_nodes, Some(index)));
    *file_tree_nodes = build_file_tree(
        &status.unstaged,
        &status.untracked,
        &status.staged,
        expanded_dirs,
    );
    files_panel
        .list_state
        .select(cursor.restore(file_tree_nodes));
    *files_visual_anchor = anchor.and_then(|cursor| cursor.restore(file_tree_nodes));
}
