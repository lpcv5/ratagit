use crate::git::{DiffLine, GitRepository};
use crate::ui::widgets::file_tree::FileTreeNodeStatus;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum DiffTarget {
    None,
    Branch {
        name: String,
    },
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

pub fn load_diff(repo: &dyn GitRepository, target: DiffTarget) -> Vec<DiffLine> {
    match target {
        DiffTarget::None => Vec::new(),
        DiffTarget::Branch { name } => repo.branch_log(&name, 100).unwrap_or_default(),
        DiffTarget::File { path, status } => load_file_diff(repo, &path, &status),
        DiffTarget::Directory { path } => repo.diff_directory(&path).unwrap_or_default(),
        DiffTarget::Commit { oid, path } => repo
            .commit_diff_scoped(&oid, path.as_deref())
            .unwrap_or_default(),
        DiffTarget::Stash { index, path } => {
            repo.stash_diff(index, path.as_deref()).unwrap_or_default()
        }
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
