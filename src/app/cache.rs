use crate::backend::git_ops::{BranchEntry, CommitEntry, StashEntry, StatusEntry};

#[derive(Default)]
pub struct CachedData {
    pub files: Vec<StatusEntry>,
    pub branches: Vec<BranchEntry>,
    pub commits: Vec<CommitEntry>,
    pub stashes: Vec<StashEntry>,
    pub current_diff: Option<(String, String)>,
    pub log_entries: Vec<String>,
}
