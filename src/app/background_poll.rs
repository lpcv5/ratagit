use super::diff_cache;
use crate::flux::task_manager::TaskRequest;
use crate::git::{
    BranchInfo, CommitInfo, DiffLine, GitError, GitStatus, StashInfo,
};
use std::sync::mpsc::Receiver;

pub(super) enum BackgroundReceiver {
    Status {
        fast: bool,
        rx: Receiver<Result<GitStatus, GitError>>,
    },
    Branches(Receiver<Result<Vec<BranchInfo>, GitError>>),
    Stashes(Receiver<Result<Vec<StashInfo>, GitError>>),
    Commits(Receiver<Result<Vec<CommitInfo>, GitError>>),
    CommitsFast(Receiver<Result<Vec<CommitInfo>, GitError>>),
    BranchCommits {
        branch: String,
        rx: Receiver<Result<Vec<CommitInfo>, GitError>>,
    },
    Diff {
        cache_key: diff_cache::DiffCacheKey,
        rx: Receiver<Result<Vec<DiffLine>, GitError>>,
    },
}

pub(super) enum BackgroundPayload {
    Status {
        status: GitStatus,
        fast: bool,
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
        cache_key: diff_cache::DiffCacheKey,
        diff: Vec<DiffLine>,
    },
}

pub(super) struct PendingBackgroundTask {
    pub(super) request: TaskRequest,
    pub(super) receiver: BackgroundReceiver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DiffRefreshSource {
    Status,
    Branches,
    Stashes,
    Commits,
}
