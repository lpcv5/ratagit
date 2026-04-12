use std::collections::HashMap;

use crate::backend::git_ops::{BranchEntry, CommitEntry, StashEntry, StatusEntry};
use crate::components::core::GitFileStatus;

#[derive(Default)]
pub struct CachedData {
    pub files: Vec<StatusEntry>,
    pub branches: Vec<BranchEntry>,
    pub commits: Vec<CommitEntry>,
    pub stashes: Vec<StashEntry>,
    pub current_diff: Option<(String, String)>,
    pub log_entries: Vec<String>,
    /// Commit 文件树缓存：(commit_id, 文件列表)
    pub commit_files: Option<(String, Vec<(String, GitFileStatus)>)>,
    /// 分支提交图缓存：branch_name -> git log --graph output
    pub branch_graphs: HashMap<String, String>,
    /// 进入 branch commits sub-panel 前保存的 commits，退出时恢复
    pub saved_commits: Option<Vec<CommitEntry>>,
}
