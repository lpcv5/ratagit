use crate::git::FileEntry;
use crate::ui::widgets::file_tree::{FileTree, FileTreeNode, FileTreeNodeStatus};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Clone, PartialEq)]
struct TreeSelectionKey {
    path: PathBuf,
    status: FileTreeNodeStatus,
    is_dir: bool,
}

pub fn rebuild_tree_nodes(
    tree_files: &[FileEntry],
    expanded_dirs: &HashSet<PathBuf>,
    tree_nodes: &mut Vec<FileTreeNode>,
    list_state: &mut ratatui::widgets::ListState,
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
    use pretty_assertions::assert_eq;
    use ratatui::widgets::ListState;

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

        let selected = tree_nodes
            .get(list_state.selected().expect("selected index"))
            .expect("selected tree node");
        assert_eq!(selected.path, PathBuf::from("b/file.txt"));
    }
}
