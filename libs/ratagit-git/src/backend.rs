use std::fmt;

use ratagit_core::{
    BranchDeleteMode, CommitEntry, CommitFileDiffTarget, CommitFileEntry, FileDiffTarget,
    FilesSnapshot, GitErrorKind, GitFailure, RepoSnapshot, ResetMode, StashEntry,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitError {
    pub kind: GitErrorKind,
    pub message: String,
}

impl GitError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self::with_kind(GitErrorKind::Unknown, message)
    }

    pub(crate) fn with_kind(kind: GitErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub(crate) fn io(message: impl Into<String>) -> Self {
        Self::with_kind(GitErrorKind::Io, message)
    }

    pub(crate) fn cli(kind: GitErrorKind, message: impl Into<String>) -> Self {
        Self::with_kind(kind, message)
    }

    pub(crate) fn into_failure(self) -> GitFailure {
        GitFailure::new(self.kind, self.message)
    }
}

impl From<git2::Error> for GitError {
    fn from(error: git2::Error) -> Self {
        Self::with_kind(GitErrorKind::Git2, error.message().to_string())
    }
}

impl fmt::Display for GitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(formatter)
    }
}

impl std::error::Error for GitError {}

pub trait GitBackendRead {
    fn refresh_snapshot(&mut self) -> Result<RepoSnapshot, GitError>;
    fn refresh_files(&mut self) -> Result<FilesSnapshot, GitError> {
        let snapshot = self.refresh_snapshot()?;
        Ok(FilesSnapshot {
            status_summary: snapshot.status_summary,
            current_branch: snapshot.current_branch,
            detached_head: snapshot.detached_head,
            index_entry_count: snapshot.files.len(),
            files: snapshot.files,
            large_repo_mode: false,
            status_truncated: false,
            status_scan_skipped: false,
            untracked_scan_skipped: false,
        })
    }
    fn refresh_branches(&mut self) -> Result<Vec<ratagit_core::BranchEntry>, GitError> {
        self.refresh_snapshot().map(|snapshot| snapshot.branches)
    }
    fn refresh_commits(&mut self) -> Result<Vec<CommitEntry>, GitError> {
        self.refresh_snapshot().map(|snapshot| snapshot.commits)
    }
    fn branch_commits(&mut self, branch: &str) -> Result<Vec<CommitEntry>, GitError>;
    fn refresh_stashes(&mut self) -> Result<Vec<StashEntry>, GitError> {
        self.refresh_snapshot().map(|snapshot| snapshot.stashes)
    }
    fn load_more_commits(
        &mut self,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<CommitEntry>, GitError>;
    fn files_details_diff(&mut self, targets: &[FileDiffTarget]) -> Result<String, GitError>;
    fn branch_details_log(&mut self, branch: &str, max_count: usize) -> Result<String, GitError>;
    fn commit_details_diff(&mut self, commit_id: &str) -> Result<String, GitError>;
    fn commit_files(&mut self, commit_id: &str) -> Result<Vec<CommitFileEntry>, GitError>;
    fn commit_file_diff(&mut self, target: &CommitFileDiffTarget) -> Result<String, GitError>;
}

pub trait GitBackendWrite {
    fn stage_file(&mut self, path: &str) -> Result<(), GitError>;
    fn unstage_file(&mut self, path: &str) -> Result<(), GitError>;
    fn stage_files(&mut self, paths: &[String]) -> Result<(), GitError> {
        for path in paths {
            self.stage_file(path)?;
        }
        Ok(())
    }
    fn unstage_files(&mut self, paths: &[String]) -> Result<(), GitError> {
        for path in paths {
            self.unstage_file(path)?;
        }
        Ok(())
    }
    fn create_commit(&mut self, message: &str) -> Result<(), GitError>;
    fn pull(&mut self) -> Result<(), GitError>;
    fn push(&mut self, force: bool) -> Result<(), GitError>;
    fn create_branch(&mut self, name: &str, start_point: &str) -> Result<(), GitError>;
    fn checkout_branch(&mut self, name: &str, auto_stash: bool) -> Result<(), GitError>;
    fn delete_branch(
        &mut self,
        name: &str,
        mode: BranchDeleteMode,
        force: bool,
    ) -> Result<(), GitError>;
    fn checkout_commit_detached(
        &mut self,
        commit_id: &str,
        auto_stash: bool,
    ) -> Result<(), GitError>;
    fn stash_push(&mut self, message: &str) -> Result<(), GitError>;
    fn stash_files(&mut self, message: &str, paths: &[String]) -> Result<(), GitError>;
    fn stash_pop(&mut self, stash_id: &str) -> Result<(), GitError>;
    fn reset(&mut self, mode: ResetMode) -> Result<(), GitError>;
    fn nuke(&mut self) -> Result<(), GitError>;
    fn discard_files(&mut self, paths: &[String]) -> Result<(), GitError>;
}

pub trait GitBackendHistoryRewrite {
    fn rebase_branch(
        &mut self,
        target: &str,
        interactive: bool,
        auto_stash: bool,
    ) -> Result<(), GitError>;
    fn squash_commits(&mut self, commit_ids: &[String]) -> Result<(), GitError>;
    fn fixup_commits(&mut self, commit_ids: &[String]) -> Result<(), GitError>;
    fn reword_commit(&mut self, commit_id: &str, message: &str) -> Result<(), GitError>;
    fn delete_commits(&mut self, commit_ids: &[String]) -> Result<(), GitError>;
}

pub trait GitBackend: GitBackendRead + GitBackendWrite + GitBackendHistoryRewrite {}

impl<T> GitBackend for T where
    T: GitBackendRead + GitBackendWrite + GitBackendHistoryRewrite + ?Sized
{
}

macro_rules! delegate_boxed_backend {
    ($($method:ident($($arg:ident: $arg_ty:ty),*) -> $ret:ty;)*) => {
        $(
            fn $method(&mut self, $($arg: $arg_ty),*) -> $ret {
                (**self).$method($($arg),*)
            }
        )*
    };
}

impl<T: GitBackendRead + ?Sized> GitBackendRead for Box<T> {
    delegate_boxed_backend! {
        refresh_snapshot() -> Result<RepoSnapshot, GitError>;
        refresh_files() -> Result<FilesSnapshot, GitError>;
        refresh_branches() -> Result<Vec<ratagit_core::BranchEntry>, GitError>;
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

impl<T: GitBackendWrite + ?Sized> GitBackendWrite for Box<T> {
    delegate_boxed_backend! {
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

impl<T: GitBackendHistoryRewrite + ?Sized> GitBackendHistoryRewrite for Box<T> {
    delegate_boxed_backend! {
        rebase_branch(target: &str, interactive: bool, auto_stash: bool) -> Result<(), GitError>;
        squash_commits(commit_ids: &[String]) -> Result<(), GitError>;
        fixup_commits(commit_ids: &[String]) -> Result<(), GitError>;
        reword_commit(commit_id: &str, message: &str) -> Result<(), GitError>;
        delete_commits(commit_ids: &[String]) -> Result<(), GitError>;
    }
}
