//! Shared background read protocol; no execution or mutable application state.
use crate::app::diff::DiffCacheKey;
use crate::app::diff::DiffTarget;
use crate::git::{BranchInfo, CommitInfo, FileEntry, GitStatus, StashInfo};
use crate::shared::Shared;
use std::sync::Arc;

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

/// Repository read identity. Diff cancellation keeps one active slot across selections.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TaskKey {
    CommitFiles,
    StashFiles,
    Status,
    Branches,
    Stashes,
    Commits,
    BranchCommits { branch: String },
    Diff { target: String },
}

impl TaskKey {
    pub fn active_diff() -> Self {
        Self::Diff {
            target: "active".into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TaskGeneration(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskPriority {
    High,
    Normal,
    /// Reserved for future admission policies; only exercised in tests.
    #[allow(dead_code)]
    Low,
}

/// A complete read request: execution receives exactly the load registered by state.
#[derive(Debug, Clone)]
pub struct TaskRequest {
    pub key: TaskKey,
    pub generation: TaskGeneration,
    #[cfg_attr(not(test), allow(dead_code))]
    pub priority: TaskPriority,
    pub load: BackgroundLoad,
}

impl BackgroundLoad {
    pub fn task_key(&self) -> TaskKey {
        match self {
            Self::Status { .. } => TaskKey::Status,
            Self::Branches => TaskKey::Branches,
            Self::Stashes => TaskKey::Stashes,
            Self::Commits { .. } => TaskKey::Commits,
            Self::BranchCommits { branch, .. } => TaskKey::BranchCommits {
                branch: branch.clone(),
            },
            Self::Diff { .. } => TaskKey::active_diff(),
            Self::CommitFiles { .. } => TaskKey::CommitFiles,
            Self::StashFiles { .. } => TaskKey::StashFiles,
        }
    }
}

/// Shared completion data reused by the action, features and published snapshots.
#[derive(Debug, Clone)]
pub enum BackgroundPayload {
    Status {
        status: Shared<GitStatus>,
        fast: bool,
        operation_epoch: u64,
        operation: crate::git::operation::RepositoryOperation,
    },
    Branches(Shared<Vec<BranchInfo>>),
    Stashes(Shared<Vec<StashInfo>>),
    Commits(Shared<Vec<CommitInfo>>),
    CommitsFast(Shared<Vec<CommitInfo>>),
    BranchCommits {
        branch: String,
        items: Shared<Vec<CommitInfo>>,
    },
    Diff {
        cache_key: DiffCacheKey,
        diff: crate::git::patch::DiffDocument,
    },
    CommitFiles {
        oid: String,
        files: Shared<Vec<FileEntry>>,
    },
    StashFiles {
        index: usize,
        files: Shared<Vec<FileEntry>>,
    },
}

#[derive(Debug, Clone)]
pub struct BackgroundCompleted {
    pub request: TaskRequest,
    pub result: Result<Arc<BackgroundPayload>, String>,
}
