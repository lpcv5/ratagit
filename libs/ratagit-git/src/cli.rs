use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

use ratagit_core::{BranchDeleteMode, FileEntry, ResetMode};

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

    pub(crate) fn branch_details_log(
        &mut self,
        branch: &str,
        max_count: usize,
    ) -> Result<String, GitError> {
        self.run_git_owned(vec![
            "log".to_string(),
            "--graph".to_string(),
            "--color=always".to_string(),
            "-n".to_string(),
            max_count.to_string(),
            branch.to_string(),
        ])
    }

    pub(crate) fn create_commit(&mut self, message: &str) -> Result<(), GitError> {
        self.run_git(&["commit", "-m", message]).map(|_| ())
    }

    pub(crate) fn create_branch(&mut self, name: &str, start_point: &str) -> Result<(), GitError> {
        self.run_git(&["branch", name, start_point]).map(|_| ())
    }

    pub(crate) fn checkout_branch(&mut self, name: &str, auto_stash: bool) -> Result<(), GitError> {
        if auto_stash {
            return self.with_auto_stash("ratagit auto-stash before checkout", |cli| {
                cli.run_git(&["checkout", name]).map(|_| ())
            });
        }
        self.run_git(&["checkout", name]).map(|_| ())
    }

    pub(crate) fn delete_branch(
        &mut self,
        name: &str,
        mode: BranchDeleteMode,
        force: bool,
    ) -> Result<(), GitError> {
        match mode {
            BranchDeleteMode::Local => self.delete_local_branch(name, force),
            BranchDeleteMode::Remote => self.delete_remote_branch(name),
            BranchDeleteMode::Both => {
                self.delete_local_branch(name, force)?;
                self.delete_remote_branch(name)
            }
        }
    }

    pub(crate) fn rebase_branch(
        &mut self,
        target: &str,
        interactive: bool,
        auto_stash: bool,
    ) -> Result<(), GitError> {
        let run_rebase = |cli: &mut Self| {
            if interactive {
                cli.run_git(&["rebase", "-i", target]).map(|_| ())
            } else {
                cli.run_git(&["rebase", target]).map(|_| ())
            }
        };
        if auto_stash {
            return self.with_auto_stash("ratagit auto-stash before rebase", run_rebase);
        }
        run_rebase(self)
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

    fn delete_local_branch(&mut self, name: &str, force: bool) -> Result<(), GitError> {
        if let Some(path) = self.worktree_path_for_branch(name)? {
            return Err(GitError::new(format!(
                "branch is checked out in worktree: {}",
                path.display()
            )));
        }
        let flag = if force { "-D" } else { "-d" };
        self.run_git(&["branch", flag, name]).map(|_| ())
    }

    fn delete_remote_branch(&mut self, name: &str) -> Result<(), GitError> {
        self.run_git(&["push", "origin", "--delete", name])
            .map(|_| ())
    }

    fn with_auto_stash(
        &mut self,
        message: &str,
        operation: impl FnOnce(&mut Self) -> Result<(), GitError>,
    ) -> Result<(), GitError> {
        let before = self.stash_top()?;
        self.stash_push(message)?;
        let after = self.stash_top()?;
        let created_stash = after.is_some() && after != before;
        let result = operation(self);
        if created_stash {
            let stash_id = after.unwrap_or_else(|| "stash@{0}".to_string());
            let pop_result = self.stash_pop(&stash_id);
            if let Err(pop_error) = pop_result {
                return match result {
                    Ok(()) => Err(pop_error),
                    Err(operation_error) => Err(GitError::new(format!(
                        "{}; additionally failed to restore auto-stash: {}",
                        operation_error.message, pop_error.message
                    ))),
                };
            }
        }
        result
    }

    fn stash_top(&self) -> Result<Option<String>, GitError> {
        let output = self.run_git(&["stash", "list", "--format=%gd"])?;
        Ok(output.lines().next().map(str::to_string))
    }

    fn worktree_path_for_branch(&self, branch_name: &str) -> Result<Option<PathBuf>, GitError> {
        let output = self.run_git(&["worktree", "list", "--porcelain"])?;
        let target_ref = format!("refs/heads/{branch_name}");
        let mut current_path: Option<PathBuf> = None;
        for line in output.lines() {
            if let Some(path) = line.strip_prefix("worktree ") {
                current_path = Some(PathBuf::from(path));
            } else if let Some(branch) = line.strip_prefix("branch ") {
                if branch == target_ref {
                    return Ok(current_path.clone());
                }
            } else if line.is_empty() {
                current_path = None;
            }
        }
        Ok(None)
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
