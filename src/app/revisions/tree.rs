use crate::app::states::PanelState;
use crate::file_tree::{build_file_tree, collect_dirs, FileTreeNode, TreeCursor};
use crate::git::FileEntry;
use crate::shared::Shared;
use std::collections::HashSet;
use std::path::PathBuf;

pub struct TreeModeState<'a, Id> {
    pub tree_mode: &'a mut bool,
    pub tree_nodes: &'a mut Shared<Vec<FileTreeNode>>,
    pub tree_files: &'a mut Shared<Vec<FileEntry>>,
    pub expanded_dirs: &'a mut Shared<HashSet<PathBuf>>,
    pub selected_tree_revision: &'a mut Option<Id>,
    pub panel: &'a mut PanelState,
}

pub fn enter_tree_mode<Id: Clone>(
    selected_revision: Id,
    files: Shared<Vec<FileEntry>>,
    state: TreeModeState<'_, Id>,
) {
    let TreeModeState {
        tree_mode,
        tree_nodes,
        tree_files,
        expanded_dirs,
        selected_tree_revision,
        panel,
    } = state;

    *tree_files = files;
    *expanded_dirs = collect_dirs_from_entries(tree_files).into();
    *selected_tree_revision = Some(selected_revision);
    *tree_mode = true;

    rebuild_tree_nodes(tree_files, expanded_dirs, tree_nodes, panel);
    if tree_nodes.is_empty() {
        panel.select(None);
    } else {
        panel.select(Some(0));
    }
}

pub fn toggle_tree_dir(
    selected_dir_path: Option<PathBuf>,
    tree_files: &[FileEntry],
    expanded_dirs: &mut Shared<HashSet<PathBuf>>,
    tree_nodes: &mut Shared<Vec<FileTreeNode>>,
    panel: &mut PanelState,
) {
    let Some(path) = selected_dir_path else {
        return;
    };
    if expanded_dirs.contains(&path) {
        expanded_dirs.remove(&path);
    } else {
        expanded_dirs.insert(path);
    }
    rebuild_tree_nodes(tree_files, expanded_dirs, tree_nodes, panel);
}

pub fn close_tree_mode(
    tree_mode: &mut bool,
    tree_nodes: &mut Shared<Vec<FileTreeNode>>,
    tree_files: &mut Shared<Vec<FileEntry>>,
    expanded_dirs: &mut Shared<HashSet<PathBuf>>,
    panel: &mut PanelState,
    selected_source_index: Option<usize>,
    source_len: usize,
) {
    if !*tree_mode {
        return;
    }

    *tree_mode = false;
    *tree_files = Shared::default();
    *tree_nodes = Shared::default();
    *expanded_dirs = Shared::default();

    if let Some(idx) = selected_source_index {
        panel.select(Some(idx));
        return;
    }

    if source_len == 0 {
        panel.select(None);
    } else {
        panel.select(Some(0));
    }
}

pub fn rebuild_tree_nodes(
    tree_files: &[FileEntry],
    expanded_dirs: &HashSet<PathBuf>,
    tree_nodes: &mut Shared<Vec<FileTreeNode>>,
    panel: &mut PanelState,
) {
    let cursor = TreeCursor::capture(tree_nodes, panel.selected());
    *tree_nodes = build_file_tree(tree_files, &[], &[], expanded_dirs).into();
    panel.select(cursor.restore(tree_nodes));
}

fn collect_dirs_from_entries(entries: &[FileEntry]) -> HashSet<PathBuf> {
    collect_dirs(entries.iter().map(|entry| entry.path.as_path()))
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::{FileEntry, FileStatus};
    use pretty_assertions::assert_eq;

    #[test]
    fn test_rebuild_tree_nodes_restores_selected_path_when_new_nodes_insert_before_it() {
        let mut tree_nodes = Shared::default();
        let mut panel = PanelState::default();
        let expanded_dirs = HashSet::from([PathBuf::from("b")]);
        let initial_files = vec![FileEntry {
            path: PathBuf::from("b/file.txt"),
            status: FileStatus::Modified,
        }];

        rebuild_tree_nodes(&initial_files, &expanded_dirs, &mut tree_nodes, &mut panel);
        panel.select(Some(1));

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
            &mut panel,
        );

        let selected = &tree_nodes[panel.selected().expect("selected tree node")];
        assert_eq!(selected.path, PathBuf::from("b/file.txt"));
    }
}
