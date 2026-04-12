#[derive(Debug, Clone)]
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
    GetCommitFiles {
        commit_id: String,
    },
    GetCommitDiff {
        commit_id: String,
        path: String,
        is_dir: bool,
    },
    StageFile {
        file_path: String,
    },
    UnstageFile {
        file_path: String,
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
