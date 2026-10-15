//! Worktree status and index/worktree writes on their existing worker lanes.

use super::{Git2RepoInner, Git2Repository};
use crate::git::{FileEntry, FileStatus, GitError, GitStatus};
use std::path::PathBuf;
use std::sync::mpsc::Receiver;

impl Git2RepoInner {
    /// Documentation comment in English.
    fn convert_status(status: git2::Status) -> FileStatus {
        if status.is_index_new() || status.is_wt_new() {
            FileStatus::New
        } else if status.is_index_modified() || status.is_wt_modified() {
            FileStatus::Modified
        } else if status.is_index_deleted() || status.is_wt_deleted() {
            FileStatus::Deleted
        } else if status.is_index_renamed() || status.is_wt_renamed() {
            FileStatus::Renamed
        } else if status.is_index_typechange() || status.is_wt_typechange() {
            FileStatus::TypeChange
        } else {
            FileStatus::Modified // default
        }
    }

    fn status_impl(&self, include_untracked: bool) -> Result<GitStatus, GitError> {
        self.repo.index()?.read(true)?;
        let mut git_status = GitStatus::default();

        let mut opts = git2::StatusOptions::new();
        opts.include_untracked(include_untracked)
            .include_ignored(false)
            .update_index(false) // Performance: skip index refresh, rely on manual refresh
            .renames_head_to_index(true)
            .renames_index_to_workdir(true)
            .include_unmodified(false)
            .recurse_untracked_dirs(false) // Performance: only show top-level untracked dirs
            .recurse_ignored_dirs(false); // Performance: skip ignored directories entirely

        let statuses = self.repo.statuses(Some(&mut opts))?;

        for entry in statuses.iter() {
            let path = PathBuf::from(entry.path().unwrap_or(""));
            let status = entry.status();

            let file_entry = FileEntry {
                path,
                status: Self::convert_status(status),
            };

            // Comment in English.
            if status.is_wt_new() {
                git_status.untracked.push(file_entry);
            } else {
                if status.is_index_new()
                    || status.is_index_modified()
                    || status.is_index_deleted()
                    || status.is_index_renamed()
                    || status.is_index_typechange()
                {
                    let index_status = status
                        & (git2::Status::INDEX_NEW
                            | git2::Status::INDEX_MODIFIED
                            | git2::Status::INDEX_DELETED
                            | git2::Status::INDEX_RENAMED
                            | git2::Status::INDEX_TYPECHANGE);
                    git_status.staged.push(FileEntry {
                        path: file_entry.path.clone(),
                        status: Self::convert_status(index_status),
                    });
                }
                if status.is_wt_modified()
                    || status.is_wt_deleted()
                    || status.is_wt_renamed()
                    || status.is_wt_typechange()
                    || status.is_conflicted()
                {
                    let worktree_status = status
                        & (git2::Status::WT_MODIFIED
                            | git2::Status::WT_DELETED
                            | git2::Status::WT_RENAMED
                            | git2::Status::WT_TYPECHANGE
                            | git2::Status::CONFLICTED);
                    git_status.unstaged.push(FileEntry {
                        path: file_entry.path,
                        status: Self::convert_status(worktree_status),
                    });
                }
            }
        }

        Ok(git_status)
    }

    pub(super) fn status(&self) -> Result<GitStatus, GitError> {
        self.status_impl(true)
    }

    fn status_fast(&self) -> Result<GitStatus, GitError> {
        self.status_impl(false)
    }

    fn stage(&self, path: &std::path::Path) -> Result<(), GitError> {
        self.stage_paths(&[path.to_path_buf()])
    }

    fn stage_paths(&self, paths: &[PathBuf]) -> Result<(), GitError> {
        if paths.is_empty() {
            return Ok(());
        }

        let mut index = self.repo.index()?;
        let specs: Vec<&str> = paths.iter().filter_map(|p| p.to_str()).collect();
        index.add_all(specs, git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;
        Ok(())
    }

    fn unstage(&self, path: &std::path::Path) -> Result<(), GitError> {
        self.unstage_paths(&[path.to_path_buf()])
    }

    fn unstage_paths(&self, paths: &[PathBuf]) -> Result<(), GitError> {
        if paths.is_empty() {
            return Ok(());
        }

        let head = self.repo.head()?.target().ok_or(GitError::InvalidState)?;
        let commit_obj = self
            .repo
            .find_object(head, Some(git2::ObjectType::Commit))?;
        let specs: Vec<&str> = paths.iter().filter_map(|p| p.to_str()).collect();
        self.repo.reset_default(Some(&commit_obj), specs)?;

        Ok(())
    }

    fn discard_paths(&self, paths: &[PathBuf]) -> Result<(), GitError> {
        if paths.is_empty() {
            return Ok(());
        }

        let mut checkout = git2::build::CheckoutBuilder::new();
        checkout.force();
        for path in paths {
            let path_str = Self::path_to_str(path, "path")?;
            checkout.path(path_str);
        }
        self.repo.checkout_index(None, Some(&mut checkout))?;
        Ok(())
    }
}

impl crate::git::WorktreeRepository for Git2Repository {
    fn status(&self) -> Result<GitStatus, GitError> {
        self.spawn_repo_job("status", |inner| inner.status())?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn status_fast(&self) -> Result<GitStatus, GitError> {
        self.spawn_repo_job_external("status_fast", |inner| inner.status_fast())?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn status_async(&self) -> Result<Receiver<Result<GitStatus, GitError>>, GitError> {
        self.spawn_repo_job("status", |inner| inner.status())
    }

    fn status_fast_async(&self) -> Result<Receiver<Result<GitStatus, GitError>>, GitError> {
        self.spawn_repo_job_external("status_fast", |inner| inner.status_fast())
    }

    fn stage(&self, path: &std::path::Path) -> Result<(), GitError> {
        let path = path.to_path_buf();
        self.spawn_repo_job("stage", move |inner| inner.stage(&path))?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn stage_async(&self, path: PathBuf) -> Result<Receiver<Result<(), GitError>>, GitError> {
        self.spawn_repo_job("stage", move |inner| inner.stage(&path))
    }

    fn stage_paths(&self, paths: &[PathBuf]) -> Result<(), GitError> {
        let paths = paths.to_vec();
        self.spawn_repo_job("stage_paths", move |inner| inner.stage_paths(&paths))?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn unstage(&self, path: &std::path::Path) -> Result<(), GitError> {
        let path = path.to_path_buf();
        self.spawn_repo_job("unstage", move |inner| inner.unstage(&path))?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn unstage_async(&self, path: PathBuf) -> Result<Receiver<Result<(), GitError>>, GitError> {
        self.spawn_repo_job("unstage", move |inner| inner.unstage(&path))
    }

    fn unstage_paths(&self, paths: &[PathBuf]) -> Result<(), GitError> {
        let paths = paths.to_vec();
        self.spawn_repo_job("unstage_paths", move |inner| inner.unstage_paths(&paths))?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn discard_paths(&self, paths: &[PathBuf]) -> Result<(), GitError> {
        let paths = paths.to_vec();
        self.spawn_repo_job("discard_paths", move |inner| inner.discard_paths(&paths))?
            .recv()
            .map_err(|_| GitError::Git2("worker disconnected".to_string()))?
    }

    fn discard_paths_async(
        &self,
        paths: Vec<PathBuf>,
    ) -> Result<Receiver<Result<(), GitError>>, GitError> {
        self.spawn_repo_job("discard_paths", move |inner| inner.discard_paths(&paths))
    }
}
