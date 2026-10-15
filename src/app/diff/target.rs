use super::{DiffCacheKey, DiffSide};
use crate::file_tree::{FileTreeNode, FileTreeNodeStatus};
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
                limit: 100,
            },
            Self::File { path, status } => DiffCacheKey::File {
                path: path.clone(),
                is_staged: matches!(status, FileTreeNodeStatus::Staged(_)),
            },
            Self::Directory { path } => {
                let files_hash = visible_files
                    .iter()
                    .filter(|node| node.path.starts_with(path))
                    .map(|node| node.path.to_string_lossy().to_string())
                    .collect::<Vec<_>>()
                    .join("|")
                    .bytes()
                    .fold(0u64, |hash, byte| {
                        hash.wrapping_mul(31).wrapping_add(byte as u64)
                    });
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
