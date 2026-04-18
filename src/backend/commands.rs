#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiffTarget {
    pub path: String,
    pub is_dir: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Used in match arms and handlers
pub enum BackendCommand {
    RefreshStatus,
    RefreshBranches,
    RefreshCommits {
        limit: usize,
    },
    RefreshStashes,
    GetDiff {
        file_path: String,
    },
    GetDiffBatch {
        targets: Vec<DiffTarget>,
    },
    GetCommitFiles {
        commit_id: String,
    },
    GetCommitDiff {
        commit_id: String,
        path: String,
        is_dir: bool,
    },
    GetCommitDiffBatch {
        commit_id: String,
        targets: Vec<DiffTarget>,
    },
    GetBranchGraph {
        branch_name: String,
        limit: usize,
    },
    GetBranchCommits {
        branch_name: String,
        limit: usize,
    },
    StageFile {
        file_path: String,
    },
    StageFiles {
        file_paths: Vec<String>,
    },
    UnstageFile {
        file_path: String,
    },
    UnstageFiles {
        file_paths: Vec<String>,
    },
    StageAll,
    Commit {
        message: String,
    },
    DiscardFiles {
        paths: Vec<String>,
    },
    StashFiles {
        paths: Vec<String>,
        message: Option<String>,
    },
    #[allow(dead_code)]
    AmendCommit {
        message: String,
    },
    #[allow(dead_code)]
    GetCommitMessage {
        commit_id: String,
    },
    AmendCommitWithFiles {
        commit_id: String,
        message: String,
        paths: Vec<String>,
    },
    ResetHard {
        target: String,
    },
    ResetMixed {
        target: String,
    },
    ResetSoft {
        target: String,
    },
    IgnoreFiles {
        paths: Vec<String>,
    },
    Quit,
}

/// 带请求 ID 的命令信封
#[derive(Debug, Clone)]
pub struct CommandEnvelope {
    pub request_id: u64,
    pub command: BackendCommand,
}

impl CommandEnvelope {
    pub fn new(request_id: u64, command: BackendCommand) -> Self {
        Self {
            request_id,
            command,
        }
    }
}
