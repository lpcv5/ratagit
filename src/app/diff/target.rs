use super::{DiffCacheKey, DiffSide, FileDiffSource};
use crate::file_tree::{FileTreeNode, FileTreeNodeStatus};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

pub(crate) const BRANCH_DIFF_LIMIT: usize = 20;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffRefreshSource {
    Status,
    Branches,
    Stashes,
    Commits,
}

impl DiffTarget {
    /// Apply the explicit staged/unstaged choice to a tracked file only.
    pub fn with_side(self, side: Option<DiffSide>) -> Self {
        match (self, side) {
            (
                Self::File {
                    path,
                    status:
                        FileTreeNodeStatus::Staged(status) | FileTreeNodeStatus::Unstaged(status),
                },
                Some(side),
            ) => Self::File {
                path,
                status: if side == DiffSide::Staged {
                    FileTreeNodeStatus::Staged(status)
                } else {
                    FileTreeNodeStatus::Unstaged(status)
                },
            },
            (target, _) => target,
        }
    }

    pub fn affected_by(&self, source: DiffRefreshSource) -> bool {
        matches!(
            (source, self),
            (
                DiffRefreshSource::Status,
                Self::File { .. } | Self::Directory { .. }
            ) | (DiffRefreshSource::Branches, Self::Branch { .. })
                | (DiffRefreshSource::Stashes, Self::Stash { .. })
                | (DiffRefreshSource::Commits, Self::Commit { .. })
        )
    }

    pub fn cache_key(&self, visible_files: &[FileTreeNode]) -> DiffCacheKey {
        match self {
            Self::Branch { name } => DiffCacheKey::Branch {
                name: name.clone(),
                limit: BRANCH_DIFF_LIMIT,
            },
            Self::File { path, status } => DiffCacheKey::File {
                path: path.clone(),
                source: match status {
                    FileTreeNodeStatus::Staged(_) | FileTreeNodeStatus::Unstaged(_) => {
                        FileDiffSource::Combined
                    }
                    FileTreeNodeStatus::Untracked => FileDiffSource::Untracked,
                    FileTreeNodeStatus::Directory => FileDiffSource::Directory,
                },
            },
            Self::Directory { path } => {
                let mut hash = DefaultHasher::new();
                for node in visible_files
                    .iter()
                    .filter(|node| node.path.starts_with(path))
                {
                    (&node.path, &node.status, node.is_dir).hash(&mut hash);
                }
                let files_hash = hash.finish();
                DiffCacheKey::Directory {
                    path: path.clone(),
                    files_hash,
                }
            }
            Self::Commit { oid, path } => DiffCacheKey::Commit {
                oid: oid.clone(),
                path: path.clone(),
            },
            Self::Stash { index, path } => DiffCacheKey::Stash {
                index: *index,
                path: path.clone(),
            },
            Self::None => DiffCacheKey::None,
        }
    }
}
