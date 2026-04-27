use std::sync::{Arc, Mutex};

use ratagit_core::{
    BranchDeleteMode, BranchEntry, CommitEntry, CommitFileDiffTarget, CommitFileEntry,
    FileDiffTarget, FilesSnapshot, RepoSnapshot, ResetMode, StashEntry,
};

use crate::{GitBackend, GitError, MockGitBackend};

#[derive(Debug, Clone)]
pub struct SharedMockGitBackend {
    inner: Arc<Mutex<MockGitBackend>>,
}

impl SharedMockGitBackend {
    pub fn new(snapshot: RepoSnapshot) -> Self {
        Self {
            inner: Arc::new(Mutex::new(MockGitBackend::new(snapshot))),
        }
    }

    pub fn operations(&self) -> Vec<String> {
        self.inner
            .lock()
            .expect("shared mock backend lock")
            .operations()
            .to_vec()
    }

    pub fn snapshot(&self) -> RepoSnapshot {
        self.inner
            .lock()
            .expect("shared mock backend lock")
            .snapshot()
            .clone()
    }
}

macro_rules! delegate_shared_backend {
    ($($method:ident($($arg:ident: $arg_ty:ty),*) -> $ret:ty;)*) => {
        $(
            fn $method(&mut self, $($arg: $arg_ty),*) -> $ret {
                self.inner
                    .lock()
                    .expect("shared mock backend lock")
                    .$method($($arg),*)
            }
        )*
    };
}

impl GitBackend for SharedMockGitBackend {
    delegate_shared_backend! {
        refresh_snapshot() -> Result<RepoSnapshot, GitError>;
        refresh_files() -> Result<FilesSnapshot, GitError>;
        refresh_branches() -> Result<Vec<BranchEntry>, GitError>;
        refresh_commits() -> Result<Vec<CommitEntry>, GitError>;
        refresh_stashes() -> Result<Vec<StashEntry>, GitError>;
        load_more_commits(offset: usize, limit: usize) -> Result<Vec<CommitEntry>, GitError>;
        files_details_diff(targets: &[FileDiffTarget]) -> Result<String, GitError>;
        branch_details_log(branch: &str, max_count: usize) -> Result<String, GitError>;
        commit_details_diff(commit_id: &str) -> Result<String, GitError>;
        commit_files(commit_id: &str) -> Result<Vec<CommitFileEntry>, GitError>;
        commit_file_diff(target: &CommitFileDiffTarget) -> Result<String, GitError>;
        stage_file(path: &str) -> Result<(), GitError>;
        unstage_file(path: &str) -> Result<(), GitError>;
        stage_files(paths: &[String]) -> Result<(), GitError>;
        unstage_files(paths: &[String]) -> Result<(), GitError>;
        create_commit(message: &str) -> Result<(), GitError>;
        create_branch(name: &str, start_point: &str) -> Result<(), GitError>;
        checkout_branch(name: &str, auto_stash: bool) -> Result<(), GitError>;
        delete_branch(name: &str, mode: BranchDeleteMode, force: bool) -> Result<(), GitError>;
        rebase_branch(target: &str, interactive: bool, auto_stash: bool) -> Result<(), GitError>;
        squash_commits(commit_ids: &[String]) -> Result<(), GitError>;
        fixup_commits(commit_ids: &[String]) -> Result<(), GitError>;
        reword_commit(commit_id: &str, message: &str) -> Result<(), GitError>;
        delete_commits(commit_ids: &[String]) -> Result<(), GitError>;
        checkout_commit_detached(commit_id: &str, auto_stash: bool) -> Result<(), GitError>;
        stash_push(message: &str) -> Result<(), GitError>;
        stash_files(message: &str, paths: &[String]) -> Result<(), GitError>;
        stash_pop(stash_id: &str) -> Result<(), GitError>;
        reset(mode: ResetMode) -> Result<(), GitError>;
        nuke() -> Result<(), GitError>;
        discard_files(paths: &[String]) -> Result<(), GitError>;
    }
}

#[cfg(test)]
mod tests {
    use ratagit_core::{BranchEntry, FileEntry, StashEntry};

    use super::*;

    fn shared_snapshot() -> RepoSnapshot {
        RepoSnapshot {
            status_summary: "staged: 0, unstaged: 1".to_string(),
            current_branch: "main".to_string(),
            detached_head: false,
            files: vec![FileEntry {
                path: "a.txt".to_string(),
                staged: false,
                untracked: false,
            }],
            commits: Vec::new(),
            branches: vec![BranchEntry {
                name: "main".to_string(),
                is_current: true,
            }],
            stashes: vec![StashEntry {
                id: "stash@{0}".to_string(),
                summary: "savepoint".to_string(),
            }],
        }
    }

    #[test]
    fn clones_share_operations_and_mutated_snapshot() {
        let mut first = SharedMockGitBackend::new(shared_snapshot());
        let mut second = first.clone();

        first
            .stage_files(&["a.txt".to_string()])
            .expect("stage should update shared mock");
        second
            .stash_pop("stash@{0}")
            .expect("stash pop should update shared mock");

        assert_eq!(
            first.operations(),
            vec![
                "stage-files:a.txt".to_string(),
                "stash-pop:stash@{0}".to_string()
            ]
        );
        assert_eq!(second.operations(), first.operations());
        let snapshot = first.snapshot();
        assert!(
            snapshot
                .files
                .iter()
                .any(|entry| entry.path == "a.txt" && entry.staged)
        );
        assert!(snapshot.stashes.is_empty());
    }
}
