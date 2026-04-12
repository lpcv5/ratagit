use crate::backend::git_ops::{BranchEntry, CommitEntry, StashEntry, StatusEntry};
use crate::components::core::GitFileStatus;

#[derive(Debug)]
pub enum FrontendEvent {
    FilesUpdated {
        files: Vec<StatusEntry>,
    },
    BranchesUpdated {
        branches: Vec<BranchEntry>,
    },
    CommitsUpdated {
        commits: Vec<CommitEntry>,
    },
    StashesUpdated {
        stashes: Vec<StashEntry>,
    },
    DiffLoaded {
        #[allow(dead_code)]
        request_id: u64,
        file_path: String,
        diff: String,
    },
    CommitFilesLoaded {
        #[allow(dead_code)]
        request_id: u64,
        commit_id: String,
        files: Vec<(String, GitFileStatus)>,
    },
    BranchGraphLoaded {
        #[allow(dead_code)]
        request_id: u64,
        branch_name: String,
        graph: String,
    },
    BranchCommitsLoaded {
        #[allow(dead_code)]
        request_id: u64,
        branch_name: String,
        commits: Vec<crate::backend::git_ops::CommitEntry>,
    },
    ActionSucceeded {
        #[allow(dead_code)]
        request_id: u64,
        message: String,
    },
    #[allow(dead_code)]
    ActionFailed {
        #[allow(dead_code)]
        request_id: u64,
        message: String,
    },
    Error {
        #[allow(dead_code)]
        request_id: Option<u64>,
        message: String,
    },
}

/// 带请求 ID 的事件信封
#[derive(Debug)]
pub struct EventEnvelope {
    pub request_id: Option<u64>,
    pub event: FrontendEvent,
}

impl EventEnvelope {
    pub fn new(request_id: Option<u64>, event: FrontendEvent) -> Self {
        Self { request_id, event }
    }
}
