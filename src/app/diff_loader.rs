use crate::git::{DiffLine, GitRepository};
use crate::ui::widgets::file_tree::{FileTreeNode, FileTreeNodeStatus};
use std::path::PathBuf;

const MAX_DIR_DIFF_LINES: usize = 2000;

#[derive(Debug, Clone)]
pub enum DiffTarget {
    None,
    File {
        path: PathBuf,
        status: FileTreeNodeStatus,
    },
    Directory {
        path: PathBuf,
    },
    Commit {
        oid: String,
        path: Option<PathBuf>,
    },
    Stash {
        index: usize,
        path: Option<PathBuf>,
    },
}

pub fn load_diff(
    repo: &dyn GitRepository,
    file_tree_nodes: &[FileTreeNode],
    target: DiffTarget,
) -> Vec<DiffLine> {
    match target {
        DiffTarget::None => Vec::new(),
        DiffTarget::File { path, status } => load_file_diff(repo, &path, &status),
        DiffTarget::Directory { path } => load_dir_diff(repo, file_tree_nodes, &path),
        DiffTarget::Commit { oid, path } => repo
            .commit_diff_scoped(&oid, path.as_deref())
            .unwrap_or_default(),
        DiffTarget::Stash { index, path } => repo
            .stash_diff(index, path.as_deref())
            .unwrap_or_default(),
    }
}

fn load_file_diff(
    repo: &dyn GitRepository,
    path: &std::path::Path,
    status: &FileTreeNodeStatus,
) -> Vec<DiffLine> {
    match status {
        FileTreeNodeStatus::Staged(_) => repo.diff_staged(path).unwrap_or_default(),
        FileTreeNodeStatus::Untracked => repo.diff_untracked(path).unwrap_or_default(),
        FileTreeNodeStatus::Unstaged(_) => repo.diff_unstaged(path).unwrap_or_default(),
        FileTreeNodeStatus::Directory => Vec::new(),
    }
}

fn load_dir_diff(
    repo: &dyn GitRepository,
    file_tree_nodes: &[FileTreeNode],
    dir_path: &std::path::Path,
) -> Vec<DiffLine> {
    let mut result = Vec::new();
    for node in file_tree_nodes
        .iter()
        .filter(|n| !n.is_dir && n.path.starts_with(dir_path))
    {
        if result.len() >= MAX_DIR_DIFF_LINES {
            break;
        }
        let lines = load_file_diff(repo, &node.path, &node.status);
        let remaining = MAX_DIR_DIFF_LINES - result.len();
        result.extend(lines.into_iter().take(remaining));
    }
    result
}
