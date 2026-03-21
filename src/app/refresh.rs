use super::app::PanelState;
use crate::git::GitStatus;
use crate::ui::widgets::file_tree::{FileTree, FileTreeNode, FileTreeNodeStatus};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Clone, PartialEq)]
struct TreeSelectionKey {
    path: PathBuf,
    status: FileTreeNodeStatus,
    is_dir: bool,
}

pub(super) fn collect_all_dirs(status: &GitStatus) -> HashSet<PathBuf> {
    let mut dirs = HashSet::new();
    let all_files = status
        .unstaged
        .iter()
        .map(|f| &f.path)
        .chain(status.untracked.iter().map(|f| &f.path))
        .chain(status.staged.iter().map(|f| &f.path));

    for path in all_files {
        let mut p = path.as_path();
        while let Some(parent) = p.parent() {
            if parent == std::path::Path::new("") {
                break;
            }
            dirs.insert(parent.to_path_buf());
            p = parent;
        }
    }
    dirs
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
    // Remember the selected node identity so we can restore selection after rebuild.
    let selected_node = files_panel
        .list_state
        .selected()
        .and_then(|i| file_tree_nodes.get(i))
        .map(tree_selection_key);
    let selected_idx = files_panel.list_state.selected();

    *file_tree_nodes = FileTree::from_git_status_with_expanded(
        &status.unstaged,
        &status.untracked,
        &status.staged,
        expanded_dirs,
    );

    let count = file_tree_nodes.len();
    if count == 0 {
        files_panel.list_state.select(None);
        *files_visual_anchor = None;
    } else {
        // Try to find the same node in the new tree; fall back to clamped index.
        let new_idx = selected_node
            .as_ref()
            .and_then(|selected| {
                file_tree_nodes
                    .iter()
                    .position(|node| tree_selection_key(node) == *selected)
            })
            .unwrap_or_else(|| selected_idx.unwrap_or(0).min(count - 1));
        files_panel.list_state.select(Some(new_idx));
        if let Some(anchor) = *files_visual_anchor {
            *files_visual_anchor = Some(anchor.min(count - 1));
        }
    }
}

fn tree_selection_key(node: &FileTreeNode) -> TreeSelectionKey {
    TreeSelectionKey {
        path: node.path.clone(),
        status: node.status.clone(),
        is_dir: node.is_dir,
    }
}
