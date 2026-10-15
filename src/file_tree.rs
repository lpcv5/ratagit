//! File-tree construction and selection, independent of terminal rendering.
use crate::git::{FileEntry, FileStatus};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FileTreeNodeStatus {
    Unstaged(FileStatus),
    Staged(FileStatus),
    Untracked,
    Directory,
}

#[derive(Debug, Clone)]
pub struct FileTreeNode {
    pub path: PathBuf,
    pub status: FileTreeNodeStatus,
    pub depth: usize,
    pub is_dir: bool,
    pub is_expanded: bool,
}

pub fn collect_dirs<'a>(paths: impl IntoIterator<Item = &'a Path>) -> HashSet<PathBuf> {
    let mut dirs = HashSet::new();
    for path in paths {
        let mut parent = path.parent();
        while let Some(path) = parent.filter(|path| !path.as_os_str().is_empty()) {
            if !dirs.insert(path.to_path_buf()) {
                break;
            }
            parent = path.parent();
        }
    }
    dirs
}

#[derive(Default)]
struct Children {
    dirs: Vec<PathBuf>,
    files: Vec<(PathBuf, FileTreeNodeStatus)>,
}

pub fn build_file_tree(
    unstaged: &[FileEntry],
    untracked: &[FileEntry],
    staged: &[FileEntry],
    expanded: &HashSet<PathBuf>,
) -> Vec<FileTreeNode> {
    let dirs = collect_dirs(
        unstaged
            .iter()
            .chain(untracked)
            .chain(staged)
            .map(|entry| entry.path.as_path()),
    );
    let mut children: BTreeMap<PathBuf, Children> = BTreeMap::new();
    // Index each edge once instead of scanning every file for every directory.
    for dir in dirs.into_iter().collect::<BTreeSet<_>>() {
        if let Some(parent) = dir.parent() {
            children
                .entry(parent.to_path_buf())
                .or_default()
                .dirs
                .push(dir);
        }
    }
    let entries = unstaged
        .iter()
        .map(|entry| (entry, FileTreeNodeStatus::Unstaged(entry.status.clone())))
        .chain(
            untracked
                .iter()
                .map(|entry| (entry, FileTreeNodeStatus::Untracked)),
        )
        .chain(
            staged
                .iter()
                .map(|entry| (entry, FileTreeNodeStatus::Staged(entry.status.clone()))),
        );
    for (entry, status) in entries {
        if let Some(parent) = entry.path.parent() {
            children
                .entry(parent.to_path_buf())
                .or_default()
                .files
                .push((entry.path.clone(), status));
        }
    }
    for group in children.values_mut() {
        // Stable sorting preserves unstaged/staged duplicates of the same path.
        group.files.sort_by(|a, b| a.0.cmp(&b.0));
    }

    let mut nodes = Vec::new();
    let mut pending = Vec::new();
    push_children(&children, Path::new(""), 0, &mut pending);
    // Iteration also avoids recursion limits for deeply nested trees.
    while let Some((path, depth, status)) = pending.pop() {
        let is_dir = status.is_none();
        let is_expanded = is_dir && expanded.contains(path);
        nodes.push(FileTreeNode {
            path: path.to_path_buf(),
            status: status.cloned().unwrap_or(FileTreeNodeStatus::Directory),
            depth,
            is_dir,
            is_expanded,
        });
        if is_expanded {
            push_children(&children, path, depth + 1, &mut pending);
        }
    }
    nodes
}

fn push_children<'a>(
    children: &'a BTreeMap<PathBuf, Children>,
    path: &Path,
    depth: usize,
    pending: &mut Vec<(&'a Path, usize, Option<&'a FileTreeNodeStatus>)>,
) {
    if let Some(group) = children.get(path) {
        pending.extend(
            group
                .files
                .iter()
                .rev()
                .map(|(path, status)| (path.as_path(), depth, Some(status))),
        );
        pending.extend(
            group
                .dirs
                .iter()
                .rev()
                .map(|path| (path.as_path(), depth, None)),
        );
    }
}

/// A cursor is identified by path and index/worktree side, not its row number.
pub struct TreeCursor {
    node: Option<FileTreeNode>,
    index: Option<usize>,
}

impl TreeCursor {
    pub fn capture(nodes: &[FileTreeNode], index: Option<usize>) -> Self {
        Self {
            node: index.and_then(|index| nodes.get(index)).cloned(),
            index,
        }
    }

    pub fn restore(&self, nodes: &[FileTreeNode]) -> Option<usize> {
        if nodes.is_empty() {
            return None;
        }
        let find = |predicate: &dyn Fn(&FileTreeNode) -> bool| nodes.iter().position(predicate);
        let matched = self.node.as_ref().and_then(|selected| {
            find(&|node| {
                node.path == selected.path
                    && node.status == selected.status
                    && node.is_dir == selected.is_dir
            })
            .or_else(|| {
                find(&|node| {
                    node.path == selected.path
                        && std::mem::discriminant(&node.status)
                            == std::mem::discriminant(&selected.status)
                })
            })
            // Git represents an untracked directory as a leaf, then as a
            // directory after staging. Its path remains the same.
            .or_else(|| find(&|node| node.path == selected.path))
            .or_else(|| {
                nodes
                    .iter()
                    .enumerate()
                    .filter(|(_, node)| selected.path.starts_with(&node.path))
                    .max_by_key(|(_, node)| node.path.components().count())
                    .map(|(index, _)| index)
            })
        });
        Some(matched.unwrap_or_else(|| self.index.unwrap_or(0).min(nodes.len() - 1)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: impl Into<PathBuf>) -> FileEntry {
        FileEntry {
            path: path.into(),
            status: FileStatus::Modified,
        }
    }

    #[test]
    fn test_file_tree_orders_directories_first_and_preserves_duplicate_sides() {
        let files = [
            entry("z.txt"),
            entry("b/deep/file.txt"),
            entry("a/file.txt"),
        ];
        let expanded = collect_dirs(files.iter().map(|entry| entry.path.as_path()));
        let nodes = build_file_tree(&files, &[entry("new.txt")], &[entry("z.txt")], &expanded);
        let paths: Vec<_> = nodes.iter().map(|node| node.path.clone()).collect();
        assert_eq!(
            paths,
            [
                "a",
                "a/file.txt",
                "b",
                "b/deep",
                "b/deep/file.txt",
                "new.txt",
                "z.txt",
                "z.txt"
            ]
            .map(PathBuf::from)
        );
        assert!(matches!(nodes[6].status, FileTreeNodeStatus::Unstaged(_)));
        assert!(matches!(nodes[7].status, FileTreeNodeStatus::Staged(_)));
        let collapsed = build_file_tree(&files, &[], &[], &HashSet::new());
        assert_eq!(collapsed.len(), 3);
        assert!(collapsed[..2]
            .iter()
            .all(|node| node.is_dir && !node.is_expanded));
    }

    #[test]
    fn test_file_tree_large_dataset_keeps_all_nodes_and_selection() {
        let files: Vec<_> = (0..10_000)
            .rev()
            .map(|index| entry(format!("dir{index:05}/file.txt")))
            .collect();
        let expanded = collect_dirs(files.iter().map(|entry| entry.path.as_path()));
        let nodes = build_file_tree(&files, &[], &[], &expanded);
        assert_eq!(nodes.len(), 20_000);
        assert_eq!(nodes[19_999].path, Path::new("dir09999/file.txt"));
        let cursor = TreeCursor::capture(&nodes, Some(19_999));
        let mut updated = files;
        updated.push(entry("a/new.txt"));
        let expanded = collect_dirs(updated.iter().map(|entry| entry.path.as_path()));
        let nodes = build_file_tree(&updated, &[], &[], &expanded);
        assert_eq!(cursor.restore(&nodes), Some(20_001));
    }

    #[test]
    fn test_file_tree_deep_paths_and_collapse_restore_to_visible_parent() {
        let mut path = PathBuf::new();
        for _ in 0..512 {
            path.push("d");
        }
        path.push("file.txt");
        let files = [entry(path)];
        let expanded = collect_dirs(files.iter().map(|entry| entry.path.as_path()));
        let nodes = build_file_tree(&files, &[], &[], &expanded);
        assert_eq!(nodes.len(), 513);
        assert_eq!(nodes[512].depth, 512);
        let cursor = TreeCursor::capture(&nodes, Some(512));
        let collapsed = build_file_tree(&files, &[], &[], &HashSet::new());
        assert_eq!(cursor.restore(&collapsed), Some(0));
        assert_eq!(cursor.restore(&[]), None);
    }

    #[test]
    fn test_file_tree_cursor_keeps_staged_side_when_status_changes() {
        let files = [entry("file.txt")];
        let nodes = build_file_tree(&files, &[], &files, &HashSet::new());
        let cursor = TreeCursor::capture(&nodes, Some(1));
        let nodes = build_file_tree(
            &files,
            &[],
            &[FileEntry {
                path: "file.txt".into(),
                status: FileStatus::New,
            }],
            &HashSet::new(),
        );
        assert_eq!(cursor.restore(&nodes), Some(1));
    }
}
