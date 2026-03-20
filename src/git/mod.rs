mod repository;

#[allow(unused_imports)]
pub use repository::{
    GitError, GitRepository, GitStatus, Git2Repository,
    FileStatus, FileEntry, DiffLine, DiffLineKind,
    BranchInfo, CommitInfo, CommitSyncState, StashInfo,
};
