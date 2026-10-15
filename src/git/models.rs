pub use super::error::GitError;
use std::path::PathBuf;

/// Owned repository data shared by effects and immutable UI snapshots.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FileStatus {
    New,
    Modified,
    Deleted,
    Renamed,
    TypeChange,
}

/// Owned repository data shared by effects and immutable UI snapshots.
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub status: FileStatus,
}

/// Owned repository data shared by effects and immutable UI snapshots.
#[derive(Debug, Clone, Default)]
pub struct GitStatus {
    pub unstaged: Vec<FileEntry>,
    pub staged: Vec<FileEntry>,
    pub untracked: Vec<FileEntry>,
}

/// Owned repository data shared by effects and immutable UI snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffLineKind {
    Added,
    Removed,
    Context,
    Header,
}

/// Owned repository data shared by effects and immutable UI snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub content: String,
}

/// Branch info
#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub is_current: bool,
}

/// Commit sync state for log coloring
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitSyncState {
    DefaultBranch,
    RemoteBranch,
    LocalOnly,
}

/// A single cell in a commit graph line, carrying its display string and lane index for coloring.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphCell {
    pub text: String,
    pub lane: usize,
    /// OID of the commit whose branch line this cell belongs to (for path highlighting).
    pub pipe_oid: Option<String>,
    /// OIDs of all commits whose paths overlap this cell.
    pub pipe_oids: Vec<String>,
}

/// Commit info for log display
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub oid: String,
    pub message: String,
    pub author: String,
    pub graph: Vec<GraphCell>,
    pub time: String,
    pub parent_count: usize,
    pub sync_state: CommitSyncState,
    /// OIDs of this commit's parents (for ancestry tracing).
    pub parent_oids: Vec<String>,
}

/// Stash entry
#[derive(Debug, Clone)]
pub struct StashInfo {
    pub index: usize,
    pub message: String,
}
