use std::sync::{Arc, Mutex};

use ratagit_core::{
    BranchDeleteMode, BranchEntry, CommitEntry, CommitFileDiffTarget, CommitFileEntry,
    FileDiffTarget, FilesSnapshot, RepoSnapshot, ResetMode, StashEntry,
};

use crate::{GitBackendHistoryRewrite, GitBackendRead, GitBackendWrite, GitError, MockGitBackend};

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

impl GitBackendRead for SharedMockGitBackend {
    delegate_shared_backend! {
        refresh_snapshot() -> Result<RepoSnapshot, GitError>;
        refresh_files() -> Result<FilesSnapshot, GitError>;
        refresh_branches() -> Result<Vec<BranchEntry>, GitError>;
        refresh_commits() -> Result<Vec<CommitEntry>, GitError>;
        branch_commits(branch: &str) -> Result<Vec<CommitEntry>, GitError>;
        refresh_stashes() -> Result<Vec<StashEntry>, GitError>;
        load_more_commits(offset: usize, limit: usize) -> Result<Vec<CommitEntry>, GitError>;
        files_details_diff(targets: &[FileDiffTarget]) -> Result<String, GitError>;
        branch_details_log(branch: &str, max_count: usize) -> Result<String, GitError>;
        commit_details_diff(commit_id: &str) -> Result<String, GitError>;
        commit_files(commit_id: &str) -> Result<Vec<CommitFileEntry>, GitError>;
        commit_file_diff(target: &CommitFileDiffTarget) -> Result<String, GitError>;
    }
}

impl GitBackendWrite for SharedMockGitBackend {
    delegate_shared_backend! {
        stage_file(path: &str) -> Result<(), GitError>;
        unstage_file(path: &str) -> Result<(), GitError>;
        stage_files(paths: &[String]) -> Result<(), GitError>;
        unstage_files(paths: &[String]) -> Result<(), GitError>;
        create_commit(message: &str) -> Result<(), GitError>;
        pull() -> Result<(), GitError>;
        push(force: bool) -> Result<(), GitError>;
        create_branch(name: &str, start_point: &str) -> Result<(), GitError>;
        checkout_branch(name: &str, auto_stash: bool) -> Result<(), GitError>;
        delete_branch(name: &str, mode: BranchDeleteMode, force: bool) -> Result<(), GitError>;
        checkout_commit_detached(commit_id: &str, auto_stash: bool) -> Result<(), GitError>;
        stash_push(message: &str) -> Result<(), GitError>;
        stash_files(message: &str, paths: &[String]) -> Result<(), GitError>;
        stash_pop(stash_id: &str) -> Result<(), GitError>;
        reset(mode: ResetMode) -> Result<(), GitError>;
        nuke() -> Result<(), GitError>;
        discard_files(paths: &[String]) -> Result<(), GitError>;
    }
}

impl GitBackendHistoryRewrite for SharedMockGitBackend {
    delegate_shared_backend! {
        rebase_branch(target: &str, interactive: bool, auto_stash: bool) -> Result<(), GitError>;
        squash_commits(commit_ids: &[String]) -> Result<(), GitError>;
        fixup_commits(commit_ids: &[String]) -> Result<(), GitError>;
        reword_commit(commit_id: &str, message: &str) -> Result<(), GitError>;
        delete_commits(commit_ids: &[String]) -> Result<(), GitError>;
    }
}

#[cfg(test)]
mod tests {
    use ratagit_core::{
        BranchEntry, CommitEntry, CommitFileStatus, CommitHashStatus, FileEntry, StashEntry,
    };

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
                status: CommitFileStatus::Modified,
                conflicted: false,
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

    fn shared_snapshot_with_commits() -> RepoSnapshot {
        let mut snapshot = shared_snapshot();
        snapshot.commits = vec![
            CommitEntry {
                id: "aaa1111".to_string(),
                full_id: "aaa1111".to_string(),
                summary: "head".to_string(),
                message: "head".to_string(),
                author_name: "ratagit-tests".to_string(),
                graph: "●".to_string(),
                hash_status: CommitHashStatus::Unpushed,
                is_merge: false,
            },
            CommitEntry {
                id: "bbb2222".to_string(),
                full_id: "bbb2222".to_string(),
                summary: "base".to_string(),
                message: "base".to_string(),
                author_name: "ratagit-tests".to_string(),
                graph: "●".to_string(),
                hash_status: CommitHashStatus::Unpushed,
                is_merge: false,
            },
        ];
        snapshot
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

    #[test]
    fn capabilities_dispatch_through_shared_mock() {
        let mut backend = SharedMockGitBackend::new(shared_snapshot_with_commits());

        let commits = GitBackendRead::refresh_commits(&mut backend)
            .expect("read capability should refresh commits");
        GitBackendWrite::stage_files(&mut backend, &["a.txt".to_string()])
            .expect("write capability should stage files");
        GitBackendHistoryRewrite::reword_commit(&mut backend, "aaa1111", "head reworded")
            .expect("history rewrite capability should reword commits");

        assert_eq!(commits.len(), 2);
        assert_eq!(backend.snapshot().commits[0].summary, "head reworded");
        assert_eq!(
            backend.operations(),
            vec![
                "refresh-commits".to_string(),
                "stage-files:a.txt".to_string(),
                "reword:aaa1111:head reworded".to_string(),
            ]
        );
    }
}
