//! Stash reads and mutations serialized on the primary worker.

use super::shared::parse_diff;
use super::{Git2RepoInner, Git2Repository};
use crate::git::{DiffLine, FileEntry, FileStatus, GitError, StashInfo};
use std::path::PathBuf;
use std::sync::mpsc::Receiver;

impl Git2RepoInner {
    fn stash_refname(index: usize) -> String {
        if index == 0 {
            "refs/stash".to_string()
        } else {
            format!("refs/stash@{{{}}}", index)
        }
    }

    fn stash_commit(&self, index: usize) -> Result<git2::Commit<'_>, GitError> {
        let refname = Self::stash_refname(index);
        let reference = self.repo.find_reference(&refname)?;
        Ok(reference.peel_to_commit()?)
    }

    fn stashes(&mut self) -> Result<Vec<StashInfo>, GitError> {
        let mut result = Vec::new();
        self.repo.stash_foreach(|index, name, _oid| {
            result.push(StashInfo {
                index,
                message: name.to_string(),
            });
            true
        })?;
        Ok(result)
    }

    fn stash_files(&self, index: usize) -> Result<Vec<FileEntry>, GitError> {
        let mut files = Vec::new();
        let commit = self.stash_commit(index)?;
        let tree = commit.tree()?;
        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };
        let diff = self
            .repo
            .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;
        for delta in diff.deltas() {
            let status = match delta.status() {
                git2::Delta::Added => FileStatus::New,
                git2::Delta::Deleted => FileStatus::Deleted,
                git2::Delta::Renamed => FileStatus::Renamed,
                git2::Delta::Typechange => FileStatus::TypeChange,
                _ => FileStatus::Modified,
            };
            let path = match delta.status() {
                git2::Delta::Deleted => delta.old_file().path(),
                _ => delta.new_file().path().or_else(|| delta.old_file().path()),
            };
            if let Some(path) = path {
                files.push(FileEntry {
                    path: path.to_path_buf(),
                    status,
                });
            }
        }

        Ok(files)
    }

    fn stash_diff(
        &self,
        index: usize,
        path: Option<&std::path::Path>,
    ) -> Result<Vec<DiffLine>, GitError> {
        let commit = self.stash_commit(index)?;
        let tree = commit.tree()?;
        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };
        let mut opts = git2::DiffOptions::new();
        if let Some(path) = path {
            let path_str = Self::path_to_str(path, "stash path")?;
            opts.pathspec(path_str);
        }
        let diff =
            self.repo
                .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut opts))?;
        Ok(parse_diff(&diff))
    }

    fn stash_push_paths(&mut self, paths: &[PathBuf], message: &str) -> Result<usize, GitError> {
        if paths.is_empty() {
            return Err(GitError::Git2("no selected paths for stash".to_string()));
        }

        let before = self.stashes()?.len();
        let mut args = vec![
            "stash".to_string(),
            "push".to_string(),
            "-u".to_string(),
            "-m".to_string(),
            message.to_string(),
            "--".to_string(),
        ];
        for path in paths {
            let path_str = Self::path_to_str(path, "stash path")?;
            args.push(path_str.to_string());
        }

        self.run_git_owned(&args)?;
        let after = self.stashes()?;
        if after.len() <= before {
            return Err(GitError::Git2(
                "no local changes in selected paths to stash".to_string(),
            ));
        }
        Ok(after[0].index)
    }

    fn stash_apply(&mut self, index: usize) -> Result<(), GitError> {
        let mut checkout = git2::build::CheckoutBuilder::new();
        checkout.force();
        let mut apply_opts = git2::StashApplyOptions::new();
        apply_opts.checkout_options(checkout);
        self.repo.stash_apply(index, Some(&mut apply_opts))?;
        Ok(())
    }

    fn stash_pop(&mut self, index: usize) -> Result<(), GitError> {
        let mut checkout = git2::build::CheckoutBuilder::new();
        checkout.force();
        let mut apply_opts = git2::StashApplyOptions::new();
        apply_opts.checkout_options(checkout);

        self.repo.stash_apply(index, Some(&mut apply_opts))?;
        self.repo.stash_drop(index)?;
        Ok(())
    }

    fn stash_drop(&mut self, index: usize) -> Result<(), GitError> {
        self.repo.stash_drop(index)?;
        Ok(())
    }
}

impl crate::git::StashRepository for Git2Repository {
    fn stashes(&self) -> Result<Vec<StashInfo>, GitError> {
        self.spawn_repo_job("stashes", |inner| inner.stashes())?
            .recv()
            .map_err(|_| GitError::Worker("worker disconnected".to_string()))?
    }

    fn stashes_async(&self) -> Result<Receiver<Result<Vec<StashInfo>, GitError>>, GitError> {
        self.spawn_repo_job("stashes", |inner| inner.stashes())
    }

    fn stash_files(&self, index: usize) -> Result<Vec<FileEntry>, GitError> {
        self.spawn_repo_job("stash_files", move |inner| inner.stash_files(index))?
            .recv()
            .map_err(|_| GitError::Worker("worker disconnected".to_string()))?
    }

    fn stash_diff(
        &self,
        index: usize,
        path: Option<&std::path::Path>,
    ) -> Result<Vec<DiffLine>, GitError> {
        let path = path.map(|p| p.to_path_buf());
        self.spawn_repo_job("stash_diff", move |inner| {
            inner.stash_diff(index, path.as_deref())
        })?
        .recv()
        .map_err(|_| GitError::Worker("worker disconnected".to_string()))?
    }

    fn stash_diff_async(
        &self,
        index: usize,
        path: Option<PathBuf>,
    ) -> Result<Receiver<Result<Vec<DiffLine>, GitError>>, GitError> {
        self.spawn_repo_job("stash_diff", move |inner| {
            inner.stash_diff(index, path.as_deref())
        })
    }

    fn stash_push_paths(&self, paths: &[PathBuf], message: &str) -> Result<usize, GitError> {
        let paths = paths.to_vec();
        let message = message.to_string();
        self.spawn_repo_job("stash_push_paths", move |inner| {
            inner.stash_push_paths(&paths, &message)
        })?
        .recv()
        .map_err(|_| GitError::Worker("worker disconnected".to_string()))?
    }

    fn stash_push_paths_async(
        &self,
        paths: Vec<PathBuf>,
        message: String,
    ) -> Result<Receiver<Result<usize, GitError>>, GitError> {
        self.spawn_repo_job("stash_push_paths", move |inner| {
            inner.stash_push_paths(&paths, &message)
        })
    }

    fn stash_apply(&self, index: usize) -> Result<(), GitError> {
        self.spawn_repo_job("stash_apply", move |inner| inner.stash_apply(index))?
            .recv()
            .map_err(|_| GitError::Worker("worker disconnected".to_string()))?
    }

    fn stash_apply_async(&self, index: usize) -> Result<Receiver<Result<(), GitError>>, GitError> {
        self.spawn_repo_job("stash_apply", move |inner| inner.stash_apply(index))
    }

    fn stash_pop(&self, index: usize) -> Result<(), GitError> {
        self.spawn_repo_job("stash_pop", move |inner| inner.stash_pop(index))?
            .recv()
            .map_err(|_| GitError::Worker("worker disconnected".to_string()))?
    }

    fn stash_pop_async(&self, index: usize) -> Result<Receiver<Result<(), GitError>>, GitError> {
        self.spawn_repo_job("stash_pop", move |inner| inner.stash_pop(index))
    }

    fn stash_drop(&self, index: usize) -> Result<(), GitError> {
        self.spawn_repo_job("stash_drop", move |inner| inner.stash_drop(index))?
            .recv()
            .map_err(|_| GitError::Worker("worker disconnected".to_string()))?
    }

    fn stash_drop_async(&self, index: usize) -> Result<Receiver<Result<(), GitError>>, GitError> {
        self.spawn_repo_job("stash_drop", move |inner| inner.stash_drop(index))
    }
}
