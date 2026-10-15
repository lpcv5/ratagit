//! Independent backend capabilities. The aggregate is only a composition boundary.
use super::models::*;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};

pub trait WorktreeRepository: Send + Sync {
    /// Toggle the entire worktree using fresh status in the serialized worker.
    fn toggle_stage_all(&self) -> Result<bool, GitError> {
        Err(GitError::Git2(
            "stage all is unsupported by this backend".into(),
        ))
    }
    fn ignore_file(&self, _path: &std::path::Path) -> Result<(), GitError> {
        Err(GitError::Git2(
            "ignore is unsupported by this backend".into(),
        ))
    }
    fn prepare_editor(
        &self,
        _path: &std::path::Path,
    ) -> Result<crate::desktop::EditorCommand, GitError> {
        Err(GitError::Git2(
            "editing is unsupported by this backend".into(),
        ))
    }
    fn status(&self) -> Result<GitStatus, GitError>;

    fn status_fast(&self) -> Result<GitStatus, GitError> {
        self.status()
    }

    fn status_async(&self) -> Result<Receiver<Result<GitStatus, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.status());
        Ok(rx)
    }

    fn status_fast_async(&self) -> Result<Receiver<Result<GitStatus, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.status_fast());
        Ok(rx)
    }

    fn stage(&self, path: &std::path::Path) -> Result<(), GitError>;

    fn stage_async(&self, path: PathBuf) -> Result<Receiver<Result<(), GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.stage(&path));
        Ok(rx)
    }

    fn stage_paths(&self, paths: &[PathBuf]) -> Result<(), GitError>;

    fn stage_paths_async(
        &self,
        paths: Vec<PathBuf>,
    ) -> Result<Receiver<Result<(), GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.stage_paths(&paths));
        Ok(rx)
    }

    fn unstage(&self, path: &std::path::Path) -> Result<(), GitError>;

    fn unstage_async(&self, path: PathBuf) -> Result<Receiver<Result<(), GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.unstage(&path));
        Ok(rx)
    }

    fn unstage_paths(&self, paths: &[PathBuf]) -> Result<(), GitError>;

    fn discard_paths(&self, paths: &[PathBuf]) -> Result<(), GitError>;

    fn discard_paths_async(
        &self,
        paths: Vec<PathBuf>,
    ) -> Result<Receiver<Result<(), GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.discard_paths(&paths));
        Ok(rx)
    }
}

pub trait PatchRepository: Send + Sync {
    fn apply_lines(
        &self,
        _patch: &super::patch::Patch,
        _ids: &[super::patch::LineId],
        _direction: super::patch::PatchDirection,
    ) -> Result<(), GitError> {
        Err(GitError::Git2(
            "partial line changes are unsupported by this backend".into(),
        ))
    }
    fn apply_hunks(
        &self,
        _patch: &super::patch::Patch,
        _ids: &[super::patch::HunkId],
        _direction: super::patch::PatchDirection,
    ) -> Result<(), GitError> {
        Err(GitError::Git2(
            "partial index changes are unsupported by this backend".into(),
        ))
    }
    /// Legacy/test backends can explicitly report that structured patches are unavailable.
    fn patch(
        &self,
        _request: super::patch::PatchRequest,
    ) -> Result<Option<super::patch::Patch>, GitError> {
        Ok(None)
    }

    fn stage_hunks(
        &self,
        _patch: &super::patch::Patch,
        _file: usize,
        _hunks: &[usize],
    ) -> Result<(), GitError> {
        Err(GitError::Git2(
            "partial staging is unsupported by this backend".into(),
        ))
    }

    fn diff_unstaged(&self, path: &std::path::Path) -> Result<Vec<DiffLine>, GitError>;

    fn diff_unstaged_async(
        &self,
        path: PathBuf,
    ) -> Result<Receiver<Result<Vec<DiffLine>, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.diff_unstaged(&path));
        Ok(rx)
    }

    fn diff_staged(&self, path: &std::path::Path) -> Result<Vec<DiffLine>, GitError>;

    fn diff_staged_async(
        &self,
        path: PathBuf,
    ) -> Result<Receiver<Result<Vec<DiffLine>, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.diff_staged(&path));
        Ok(rx)
    }

    fn diff_untracked(&self, path: &std::path::Path) -> Result<Vec<DiffLine>, GitError>;

    fn diff_untracked_async(
        &self,
        path: PathBuf,
    ) -> Result<Receiver<Result<Vec<DiffLine>, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.diff_untracked(&path));
        Ok(rx)
    }

    fn diff_directory(&self, _path: &std::path::Path) -> Result<Vec<DiffLine>, GitError> {
        Ok(Vec::new())
    }

    fn diff_directory_async(
        &self,
        path: PathBuf,
    ) -> Result<Receiver<Result<Vec<DiffLine>, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.diff_directory(&path));
        Ok(rx)
    }

    fn commit_diff_scoped(
        &self,
        oid: &str,
        path: Option<&std::path::Path>,
    ) -> Result<Vec<DiffLine>, GitError>;

    fn commit_diff_scoped_async(
        &self,
        oid: String,
        path: Option<PathBuf>,
    ) -> Result<Receiver<Result<Vec<DiffLine>, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.commit_diff_scoped(&oid, path.as_deref()));
        Ok(rx)
    }
}

pub trait HistoryRepository: Send + Sync {
    fn branch_log(&self, _name: &str, _limit: usize) -> Result<Vec<DiffLine>, GitError> {
        Ok(Vec::new())
    }

    fn branch_log_async(
        &self,
        name: String,
        limit: usize,
    ) -> Result<Receiver<Result<Vec<DiffLine>, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.branch_log(&name, limit));
        Ok(rx)
    }

    fn commits(&self, limit: usize) -> Result<Vec<CommitInfo>, GitError>;

    fn commits_fast(&self, limit: usize) -> Result<Vec<CommitInfo>, GitError> {
        self.commits(limit)
    }

    fn commits_async(
        &self,
        limit: usize,
    ) -> Result<Receiver<Result<Vec<CommitInfo>, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.commits(limit));
        Ok(rx)
    }

    fn commits_fast_async(
        &self,
        limit: usize,
    ) -> Result<Receiver<Result<Vec<CommitInfo>, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.commits_fast(limit));
        Ok(rx)
    }

    fn commits_for_branch(&self, _name: &str, limit: usize) -> Result<Vec<CommitInfo>, GitError> {
        self.commits(limit)
    }

    fn commits_for_branch_async(
        &self,
        name: &str,
        limit: usize,
    ) -> Result<Receiver<Result<Vec<CommitInfo>, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.commits_for_branch(name, limit));
        Ok(rx)
    }

    fn commit_files(&self, oid: &str) -> Result<Vec<FileEntry>, GitError>;
}

pub trait ReferenceRepository: Send + Sync {
    fn branches(&self) -> Result<Vec<BranchInfo>, GitError>;

    fn branches_async(&self) -> Result<Receiver<Result<Vec<BranchInfo>, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.branches());
        Ok(rx)
    }

    fn create_branch(&self, name: &str) -> Result<(), GitError>;

    fn create_branch_async(
        &self,
        name: String,
    ) -> Result<Receiver<Result<(), GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.create_branch(&name));
        Ok(rx)
    }

    fn checkout_branch(&self, name: &str) -> Result<(), GitError>;

    fn checkout_branch_async(
        &self,
        name: String,
    ) -> Result<Receiver<Result<(), GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.checkout_branch(&name));
        Ok(rx)
    }

    fn checkout_branch_with_auto_stash(&self, name: &str) -> Result<(), GitError> {
        self.checkout_branch(name)
    }

    fn checkout_branch_with_auto_stash_async(
        &self,
        name: String,
    ) -> Result<Receiver<Result<(), GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.checkout_branch_with_auto_stash(&name));
        Ok(rx)
    }

    fn delete_branch(&self, name: &str) -> Result<(), GitError>;

    fn delete_branch_async(
        &self,
        name: String,
    ) -> Result<Receiver<Result<(), GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.delete_branch(&name));
        Ok(rx)
    }

    fn fetch_default_async(&self) -> Result<Receiver<Result<String, GitError>>, GitError>;
}

pub trait StashRepository: Send + Sync {
    fn stashes(&self) -> Result<Vec<StashInfo>, GitError>;

    fn stashes_async(&self) -> Result<Receiver<Result<Vec<StashInfo>, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.stashes());
        Ok(rx)
    }

    fn stash_files(&self, index: usize) -> Result<Vec<FileEntry>, GitError>;

    fn stash_diff(
        &self,
        index: usize,
        path: Option<&std::path::Path>,
    ) -> Result<Vec<DiffLine>, GitError>;

    fn stash_diff_async(
        &self,
        index: usize,
        path: Option<PathBuf>,
    ) -> Result<Receiver<Result<Vec<DiffLine>, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.stash_diff(index, path.as_deref()));
        Ok(rx)
    }

    fn stash_push_paths(&self, paths: &[PathBuf], message: &str) -> Result<usize, GitError>;

    fn stash_push_paths_async(
        &self,
        paths: Vec<PathBuf>,
        message: String,
    ) -> Result<Receiver<Result<usize, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.stash_push_paths(&paths, &message));
        Ok(rx)
    }

    fn stash_apply(&self, index: usize) -> Result<(), GitError>;

    fn stash_apply_async(&self, index: usize) -> Result<Receiver<Result<(), GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.stash_apply(index));
        Ok(rx)
    }

    fn stash_pop(&self, index: usize) -> Result<(), GitError>;

    fn stash_pop_async(&self, index: usize) -> Result<Receiver<Result<(), GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.stash_pop(index));
        Ok(rx)
    }

    fn stash_drop(&self, index: usize) -> Result<(), GitError>;

    fn stash_drop_async(&self, index: usize) -> Result<Receiver<Result<(), GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.stash_drop(index));
        Ok(rx)
    }
}

pub trait CommitRepository: Send + Sync {
    fn amend_head(&self) -> Result<String, GitError> {
        Err(GitError::Git2(
            "amend is unsupported by this backend".into(),
        ))
    }
    fn commit(&self, message: &str) -> Result<String, GitError>;

    fn commit_async(
        &self,
        message: String,
    ) -> Result<Receiver<Result<String, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.commit(&message));
        Ok(rx)
    }
}

pub trait OperationRepository: Send + Sync {
    fn operation_snapshot(&self) -> Result<super::operation::RepositoryOperation, GitError> {
        Ok(Default::default())
    }

    fn control_operation(
        &self,
        _control: super::operation::OperationControl,
    ) -> Result<(), GitError> {
        Err(GitError::Git2(
            "operation control is unsupported by this backend".into(),
        ))
    }
}

pub trait GitRepository:
    WorktreeRepository
    + PatchRepository
    + HistoryRepository
    + ReferenceRepository
    + StashRepository
    + CommitRepository
    + OperationRepository
{
}
impl<
        T: WorktreeRepository
            + PatchRepository
            + HistoryRepository
            + ReferenceRepository
            + StashRepository
            + CommitRepository
            + OperationRepository,
    > GitRepository for T
{
}
