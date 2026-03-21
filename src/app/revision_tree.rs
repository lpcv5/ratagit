use crate::git::FileEntry;
use crate::ui::widgets::file_tree::{FileTree, FileTreeNode, FileTreeNodeStatus};
use ratatui::widgets::ListState;
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Clone, PartialEq)]
struct TreeSelectionKey {
    path: PathBuf,
    status: FileTreeNodeStatus,
    is_dir: bool,
}

pub struct TreeModeState<'a, Id> {
    pub tree_mode: &'a mut bool,
    pub tree_nodes: &'a mut Vec<FileTreeNode>,
    pub tree_files: &'a mut Vec<FileEntry>,
    pub expanded_dirs: &'a mut HashSet<PathBuf>,
    pub selected_tree_revision: &'a mut Option<Id>,
    pub list_state: &'a mut ListState,
}

pub fn enter_tree_mode<Id: Clone>(
    selected_revision: Id,
    files: Vec<FileEntry>,
    state: TreeModeState<'_, Id>,
) {
    let TreeModeState {
        tree_mode,
        tree_nodes,
        tree_files,
        expanded_dirs,
        selected_tree_revision,
        list_state,
    } = state;

    *tree_files = files;
    *expanded_dirs = collect_dirs_from_entries(tree_files);
    *selected_tree_revision = Some(selected_revision);
    *tree_mode = true;

    rebuild_tree_nodes(tree_files, expanded_dirs, tree_nodes, list_state);
    if tree_nodes.is_empty() {
        list_state.select(None);
    } else {
        list_state.select(Some(0));
    }
}

pub fn toggle_tree_dir(
    selected_dir_path: Option<PathBuf>,
    tree_files: &[FileEntry],
    expanded_dirs: &mut HashSet<PathBuf>,
    tree_nodes: &mut Vec<FileTreeNode>,
    list_state: &mut ListState,
) {
    let Some(path) = selected_dir_path else {
        return;
    };
    if expanded_dirs.contains(&path) {
        expanded_dirs.remove(&path);
    } else {
        expanded_dirs.insert(path);
    }
    rebuild_tree_nodes(tree_files, expanded_dirs, tree_nodes, list_state);
}

pub fn close_tree_mode(
    tree_mode: &mut bool,
    tree_nodes: &mut Vec<FileTreeNode>,
    tree_files: &mut Vec<FileEntry>,
    expanded_dirs: &mut HashSet<PathBuf>,
    list_state: &mut ListState,
    selected_source_index: Option<usize>,
    source_len: usize,
) {
    if !*tree_mode {
        return;
    }

    *tree_mode = false;
    tree_files.clear();
    tree_nodes.clear();
    expanded_dirs.clear();

    if let Some(idx) = selected_source_index {
        list_state.select(Some(idx));
        return;
    }

    if source_len == 0 {
        list_state.select(None);
    } else {
        list_state.select(Some(0));
    }
}

pub fn selected_tree_node<'a>(
    list_state: &ListState,
    tree_nodes: &'a [FileTreeNode],
) -> Option<&'a FileTreeNode> {
    let idx = list_state.selected()?;
    tree_nodes.get(idx)
}

pub fn rebuild_tree_nodes(
    tree_files: &[FileEntry],
    expanded_dirs: &HashSet<PathBuf>,
    tree_nodes: &mut Vec<FileTreeNode>,
    list_state: &mut ListState,
) {
    let selected_node = list_state
        .selected()
        .and_then(|idx| tree_nodes.get(idx))
        .map(tree_selection_key);
    let selected_idx = list_state.selected();
    *tree_nodes = FileTree::from_git_status_with_expanded(tree_files, &[], &[], expanded_dirs);
    let count = tree_nodes.len();
    if count == 0 {
        list_state.select(None);
        return;
    }
    let idx = selected_node
        .as_ref()
        .and_then(|selected| {
            tree_nodes
                .iter()
                .position(|node| tree_selection_key(node) == *selected)
        })
        .unwrap_or_else(|| selected_idx.unwrap_or(0).min(count - 1));
    list_state.select(Some(idx));
}

fn collect_dirs_from_entries(entries: &[FileEntry]) -> HashSet<PathBuf> {
    let mut dirs = HashSet::new();
    for entry in entries {
        let mut p = entry.path.as_path();
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

fn tree_selection_key(node: &FileTreeNode) -> TreeSelectionKey {
    TreeSelectionKey {
        path: node.path.clone(),
        status: node.status.clone(),
        is_dir: node.is_dir,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::{FileEntry, FileStatus};

    #[test]
    fn test_rebuild_tree_nodes_restores_selected_path_when_new_nodes_insert_before_it() {
        let mut tree_nodes = Vec::new();
        let mut list_state = ListState::default();
        let expanded_dirs = HashSet::from([PathBuf::from("b")]);
        let initial_files = vec![FileEntry {
            path: PathBuf::from("b/file.txt"),
            status: FileStatus::Modified,
        }];

        rebuild_tree_nodes(
            &initial_files,
            &expanded_dirs,
            &mut tree_nodes,
            &mut list_state,
        );
        list_state.select(Some(1));

        let updated_expanded_dirs = HashSet::from([PathBuf::from("a"), PathBuf::from("b")]);
        let updated_files = vec![
            FileEntry {
                path: PathBuf::from("a/new.txt"),
                status: FileStatus::Modified,
            },
            FileEntry {
                path: PathBuf::from("b/file.txt"),
                status: FileStatus::Modified,
            },
        ];

        rebuild_tree_nodes(
            &updated_files,
            &updated_expanded_dirs,
            &mut tree_nodes,
            &mut list_state,
        );

        let selected = selected_tree_node(&list_state, &tree_nodes).expect("selected tree node");
        assert_eq!(selected.path, PathBuf::from("b/file.txt"));
    }
}
