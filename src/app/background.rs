use super::diff_cache::DiffCacheKey;
use super::diff_loader::DiffTarget;
use crate::flux::task_manager::TaskRequest;
use crate::git::{BranchInfo, CommitInfo, FileEntry, GitStatus, StashInfo};

/// Owned read requests; none of these carry a reference to mutable state.
#[derive(Debug, Clone)]
pub enum BackgroundLoad {
    Status {
        fast: bool,
        operation_epoch: u64,
    },
    Branches,
    Stashes,
    Commits {
        limit: usize,
        fast: bool,
    },
    BranchCommits {
        branch: String,
        limit: usize,
    },
    Diff {
        cache_key: DiffCacheKey,
        target: DiffTarget,
    },
    CommitFiles {
        oid: String,
    },
    StashFiles {
        index: usize,
    },
}

#[derive(Debug, Clone)]
pub enum BackgroundPayload {
    Status {
        status: GitStatus,
        fast: bool,
        operation_epoch: u64,
        operation: crate::git::operation::RepositoryOperation,
    },
    Branches(Vec<BranchInfo>),
    Stashes(Vec<StashInfo>),
    Commits(Vec<CommitInfo>),
    CommitsFast(Vec<CommitInfo>),
    BranchCommits {
        branch: String,
        items: Vec<CommitInfo>,
    },
    Diff {
        cache_key: DiffCacheKey,
        diff: crate::git::patch::DiffDocument,
    },
    CommitFiles {
        oid: String,
        files: Vec<FileEntry>,
    },
    StashFiles {
        index: usize,
        files: Vec<FileEntry>,
    },
}

#[derive(Debug, Clone)]
pub struct BackgroundCompleted {
    pub request: TaskRequest,
    pub result: Result<BackgroundPayload, String>,
}
