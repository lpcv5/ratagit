use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

use ratagit_core::{FileEntry, ResetMode};

use crate::status_cli::parse_porcelain_v1_z;
use crate::{GitError, validate_repo_relative_path};

#[derive(Debug, Clone)]
pub(crate) struct GitCli {
    repo_path: PathBuf,
}

impl GitCli {
    pub(crate) fn new(repo_path: impl Into<PathBuf>) -> Self {
        Self {
            repo_path: repo_path.into(),
        }
    }

    fn run_git(&self, args: &[&str]) -> Result<String, GitError> {
        let output = ProcessCommand::new("git")
            .args(args)
            .current_dir(&self.repo_path)
            .output()
            .map_err(|err| GitError::new(format!("failed to start git {:?}: {err}", args)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(GitError::new(format!("git {:?} failed: {}", args, stderr)));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn run_git_owned(&self, args: Vec<String>) -> Result<String, GitError> {
        let output = ProcessCommand::new("git")
            .args(&args)
            .current_dir(&self.repo_path)
            .output()
            .map_err(|err| GitError::new(format!("failed to start git {:?}: {err}", args)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(GitError::new(format!("git {:?} failed: {}", args, stderr)));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn run_git_bytes(&self, args: &[&str]) -> Result<Vec<u8>, GitError> {
        let output = ProcessCommand::new("git")
            .args(args)
            .current_dir(&self.repo_path)
            .output()
            .map_err(|err| GitError::new(format!("failed to start git {:?}: {err}", args)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(GitError::new(format!("git {:?} failed: {}", args, stderr)));
        }

        Ok(output.stdout)
    }

    pub(crate) fn status_files(&self) -> Result<Vec<FileEntry>, GitError> {
        let output = self.run_git_bytes(&[
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
            "--ignored=no",
            "--ignore-submodules=all",
        ])?;
        parse_porcelain_v1_z(&output)
    }

    pub(crate) fn create_commit(&mut self, message: &str) -> Result<(), GitError> {
        self.run_git(&["commit", "-m", message]).map(|_| ())
    }

    pub(crate) fn create_branch(&mut self, name: &str) -> Result<(), GitError> {
        self.run_git(&["branch", name]).map(|_| ())
    }

    pub(crate) fn checkout_branch(&mut self, name: &str) -> Result<(), GitError> {
        self.run_git(&["checkout", name]).map(|_| ())
    }

    pub(crate) fn stash_push(&mut self, message: &str) -> Result<(), GitError> {
        if message.trim().is_empty() {
            self.run_git(&["stash", "push", "-u"]).map(|_| ())
        } else {
            self.run_git(&["stash", "push", "-u", "-m", message])
                .map(|_| ())
        }
    }

    pub(crate) fn stash_files(&mut self, message: &str, paths: &[String]) -> Result<(), GitError> {
        if paths.is_empty() {
            return Ok(());
        }
        let mut args = vec!["stash".to_string(), "push".to_string(), "-u".to_string()];
        if !message.trim().is_empty() {
            args.push("-m".to_string());
            args.push(message.to_string());
        }
        args.push("--".to_string());
        args.extend(paths.iter().cloned());
        self.run_git_owned(args).map(|_| ())
    }

    pub(crate) fn stash_pop(&mut self, stash_id: &str) -> Result<(), GitError> {
        self.run_git(&["stash", "pop", stash_id]).map(|_| ())
    }

    pub(crate) fn reset(&mut self, mode: ResetMode) -> Result<(), GitError> {
        let mode_arg = match mode {
            ResetMode::Mixed => "--mixed",
            ResetMode::Soft => "--soft",
            ResetMode::Hard => "--hard",
        };
        self.run_git(&["reset", mode_arg, "HEAD"]).map(|_| ())
    }

    pub(crate) fn nuke(&mut self) -> Result<(), GitError> {
        self.reset(ResetMode::Hard)?;
        self.run_git(&["clean", "-fd"]).map(|_| ())
    }

    pub(crate) fn discard_files(&mut self, paths: &[String]) -> Result<(), GitError> {
        for path in paths {
            if self
                .run_git(&["ls-files", "--error-unmatch", "--", path])
                .is_ok()
            {
                self.run_git(&["restore", "--staged", "--worktree", "--", path])?;
            } else {
                remove_untracked_path(&self.repo_path, path)?;
            }
        }
        Ok(())
    }
}

fn remove_untracked_path(repo_path: &Path, relative_path: &str) -> Result<(), GitError> {
    let relative_path = validate_repo_relative_path(relative_path)?;
    let target = repo_path.join(relative_path);
    let repo = repo_path
        .canonicalize()
        .map_err(|err| GitError::new(format!("failed to resolve repo path: {err}")))?;
    let parent = target
        .parent()
        .ok_or_else(|| GitError::new(format!("invalid path: {}", relative_path.display())))?;
    let resolved_parent = parent
        .canonicalize()
        .map_err(|err| GitError::new(format!("failed to resolve parent path: {err}")))?;
    if !resolved_parent.starts_with(&repo) {
        return Err(GitError::new(format!(
            "refusing to remove path outside repo: {}",
            relative_path.display()
        )));
    }
    if target.is_dir() {
        std::fs::remove_dir_all(&target).map_err(|err| {
            GitError::new(format!(
                "failed to remove {}: {err}",
                relative_path.display()
            ))
        })?;
    } else {
        std::fs::remove_file(&target).map_err(|err| {
            GitError::new(format!(
                "failed to remove {}: {err}",
                relative_path.display()
            ))
        })?;
    }
    Ok(())
}
