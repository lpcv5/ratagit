use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, Stdio};
use std::time::Instant;

use ratagit_core::{
    BranchDeleteMode, CommitFileDiffTarget, CommitFileEntry, CommitFileStatus, FileEntry,
    ResetMode, StatusMode,
};

use crate::status_cli::parse_porcelain_v1_z_limited;
use crate::{GitError, validate_repo_relative_path};

pub(crate) const STATUS_ENTRY_LIMIT: usize = 50_000;
pub(crate) const STATUS_OUTPUT_LIMIT_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StatusFilesResult {
    pub(crate) files: Vec<FileEntry>,
    pub(crate) output_truncated: bool,
    pub(crate) entries_truncated: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct GitCli {
    repo_path: PathBuf,
}

fn status_args(mode: StatusMode) -> [&'static str; 6] {
    [
        "status",
        "--porcelain=v1",
        "-z",
        match mode {
            StatusMode::Full => "--untracked-files=all",
            StatusMode::LargeRepoFast => "--untracked-files=no",
        },
        "--ignored=no",
        "--ignore-submodules=all",
    ]
}

impl GitCli {
    pub(crate) fn new(repo_path: impl Into<PathBuf>) -> Self {
        Self {
            repo_path: repo_path.into(),
        }
    }

    fn run_git(&self, args: &[&str]) -> Result<String, GitError> {
        self.run_git_text(args.iter().copied())
    }

    fn run_git_owned(&self, args: Vec<String>) -> Result<String, GitError> {
        self.run_git_text(args)
    }

    fn run_git_read_owned(&self, args: Vec<String>) -> Result<String, GitError> {
        self.run_git_read_text(args)
    }

    fn run_git_text<I, S>(&self, args: I) -> Result<String, GitError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.run_git_output(args)
            .map(|stdout| String::from_utf8_lossy(&stdout).to_string())
    }

    fn run_git_read_text<I, S>(&self, args: I) -> Result<String, GitError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.run_git_output_with_options(args, true)
            .map(|stdout| String::from_utf8_lossy(&stdout).to_string())
    }

    fn run_git_output<I, S>(&self, args: I) -> Result<Vec<u8>, GitError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.run_git_output_with_options(args, false)
    }

    fn run_git_output_with_options<I, S>(
        &self,
        args: I,
        optional_locks_disabled: bool,
    ) -> Result<Vec<u8>, GitError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let args = args
            .into_iter()
            .map(|arg| arg.as_ref().to_string())
            .collect::<Vec<_>>();
        let subcommand = args.first().map_or("unknown", String::as_str);
        let started = Instant::now();
        let mut command = ProcessCommand::new("git");
        command.args(&args).current_dir(&self.repo_path);
        if optional_locks_disabled {
            command.env("GIT_OPTIONAL_LOCKS", "0");
        }
        let output = command
            .output()
            .map_err(|err| GitError::new(format!("failed to start git {:?}: {err}", args)))?;
        let elapsed_ms = started.elapsed().as_millis();

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            tracing::warn!(
                target: "ratagit.git.cli",
                git_subcommand = subcommand,
                optional_locks_disabled,
                exit_code = ?output.status.code(),
                elapsed_ms,
                stdout_bytes = output.stdout.len(),
                stderr = %stderr,
                "git cli command failed"
            );
            return Err(GitError::new(format!("git {:?} failed: {}", args, stderr)));
        }

        tracing::debug!(
            target: "ratagit.git.cli",
            git_subcommand = subcommand,
            optional_locks_disabled,
            elapsed_ms,
            stdout_bytes = output.stdout.len(),
            "git cli command completed"
        );
        Ok(output.stdout)
    }

    fn run_git_read_output_limited(
        &self,
        args: &[&str],
        stdout_limit: usize,
    ) -> Result<(Vec<u8>, bool), GitError> {
        let subcommand = args.first().copied().unwrap_or("unknown");
        let started = Instant::now();
        let mut child = ProcessCommand::new("git")
            .args(args)
            .current_dir(&self.repo_path)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| GitError::new(format!("failed to start git {:?}: {err}", args)))?;
        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| GitError::new("failed to capture git stdout"))?;
        let mut bytes = Vec::new();
        let mut truncated = false;
        let mut buffer = [0u8; 8192];
        loop {
            let read = stdout
                .read(&mut buffer)
                .map_err(|err| GitError::new(format!("failed to read git stdout: {err}")))?;
            if read == 0 {
                break;
            }
            let remaining = stdout_limit.saturating_sub(bytes.len());
            if read > remaining {
                bytes.extend_from_slice(&buffer[..remaining]);
                truncated = true;
                let _ = child.kill();
                break;
            }
            bytes.extend_from_slice(&buffer[..read]);
            if bytes.len() == stdout_limit {
                truncated = true;
                let _ = child.kill();
                break;
            }
        }
        let output = child
            .wait_with_output()
            .map_err(|err| GitError::new(format!("failed to wait for git {:?}: {err}", args)))?;
        let elapsed_ms = started.elapsed().as_millis();
        if !truncated && !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            tracing::warn!(
                target: "ratagit.git.cli",
                git_subcommand = subcommand,
                optional_locks_disabled = true,
                exit_code = ?output.status.code(),
                elapsed_ms,
                stdout_bytes = bytes.len(),
                stderr = %stderr,
                "git cli command failed"
            );
            return Err(GitError::new(format!("git {:?} failed: {}", args, stderr)));
        }
        if truncated {
            tracing::warn!(
                target: "ratagit.git.cli",
                git_subcommand = subcommand,
                optional_locks_disabled = true,
                elapsed_ms,
                stdout_bytes = bytes.len(),
                stdout_limit,
                "git cli output truncated"
            );
        } else {
            tracing::debug!(
                target: "ratagit.git.cli",
                git_subcommand = subcommand,
                optional_locks_disabled = true,
                elapsed_ms,
                stdout_bytes = bytes.len(),
                "git cli command completed"
            );
        }
        Ok((bytes, truncated))
    }

    pub(crate) fn status_files(&self, mode: StatusMode) -> Result<StatusFilesResult, GitError> {
        let args = status_args(mode);
        let (mut output, output_truncated) =
            self.run_git_read_output_limited(&args, STATUS_OUTPUT_LIMIT_BYTES)?;
        let raw_output_bytes = output.len();
        if output_truncated {
            if let Some(last_record_end) = output.iter().rposition(|byte| *byte == 0) {
                output.truncate(last_record_end + 1);
            } else {
                output.clear();
            }
        }
        let started = Instant::now();
        let parsed = parse_porcelain_v1_z_limited(&output, STATUS_ENTRY_LIMIT)?;
        tracing::debug!(
            target: "ratagit.git.status",
            mode = ?mode,
            elapsed_ms = started.elapsed().as_millis(),
            raw_output_bytes,
            parsed_output_bytes = output.len(),
            result_count = parsed.files.len(),
            entries_truncated = parsed.truncated,
            output_truncated,
            "git status porcelain parsed"
        );
        Ok(StatusFilesResult {
            files: parsed.files,
            output_truncated,
            entries_truncated: parsed.truncated,
        })
    }

    pub(crate) fn branch_details_log(
        &mut self,
        branch: &str,
        max_count: usize,
    ) -> Result<String, GitError> {
        self.run_git_read_owned(vec![
            "log".to_string(),
            "--graph".to_string(),
            "--color=always".to_string(),
            "-n".to_string(),
            max_count.to_string(),
            branch.to_string(),
        ])
    }

    pub(crate) fn commit_details_diff(&mut self, commit_id: &str) -> Result<String, GitError> {
        self.run_git_read_owned(vec![
            "show".to_string(),
            "--no-color".to_string(),
            "--format=fuller".to_string(),
            "--patch".to_string(),
            commit_id.to_string(),
        ])
    }

    pub(crate) fn commit_files(
        &mut self,
        commit_id: &str,
    ) -> Result<Vec<CommitFileEntry>, GitError> {
        let output = self.run_git_read_owned(vec![
            "diff-tree".to_string(),
            "--root".to_string(),
            "--no-commit-id".to_string(),
            "--name-status".to_string(),
            "-r".to_string(),
            "-M".to_string(),
            "-C".to_string(),
            commit_id.to_string(),
        ])?;
        parse_commit_files(&output)
    }

    pub(crate) fn commit_file_diff(
        &mut self,
        target: &CommitFileDiffTarget,
    ) -> Result<String, GitError> {
        let mut args = vec![
            "show".to_string(),
            "--no-color".to_string(),
            "--format=".to_string(),
            "--patch".to_string(),
            "--find-renames".to_string(),
            "--find-copies".to_string(),
            target.commit_id.clone(),
            "--".to_string(),
        ];
        let mut pushed = BTreeSet::new();
        for path in &target.paths {
            if let Some(old_path) = &path.old_path
                && pushed.insert(old_path.clone())
            {
                args.push(old_path.clone());
            }
            if pushed.insert(path.path.clone()) {
                args.push(path.path.clone());
            }
        }
        self.run_git_read_owned(args)
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

    pub(crate) fn squash_commits(&mut self, commit_ids: &[String]) -> Result<(), GitError> {
        self.replay_commits(commit_ids, ReplayMode::Squash)
    }

    pub(crate) fn fixup_commits(&mut self, commit_ids: &[String]) -> Result<(), GitError> {
        self.replay_commits(commit_ids, ReplayMode::Fixup)
    }

    pub(crate) fn reword_commit(&mut self, commit_id: &str, message: &str) -> Result<(), GitError> {
        self.replay_commits(
            &[commit_id.to_string()],
            ReplayMode::Reword(message.to_string()),
        )
    }

    pub(crate) fn delete_commits(&mut self, commit_ids: &[String]) -> Result<(), GitError> {
        self.replay_commits(commit_ids, ReplayMode::Delete)
    }

    pub(crate) fn checkout_commit_detached(
        &mut self,
        commit_id: &str,
        auto_stash: bool,
    ) -> Result<(), GitError> {
        if auto_stash {
            return self.with_auto_stash("ratagit auto-stash before detached checkout", |cli| {
                cli.run_git(&["checkout", "--detach", commit_id])
                    .map(|_| ())
            });
        }
        self.run_git(&["checkout", "--detach", commit_id])
            .map(|_| ())
    }

    fn replay_commits(&mut self, commit_ids: &[String], mode: ReplayMode) -> Result<(), GitError> {
        if commit_ids.is_empty() {
            return Err(GitError::new("no commits selected"));
        }
        self.ensure_clean_worktree()?;
        let targets = commit_ids
            .iter()
            .map(|id| self.resolve_commit(id))
            .collect::<Result<Vec<_>, GitError>>()?;
        self.ensure_commits_are_private(&targets)?;
        let history = self.rev_list_reverse_head()?;
        let target_positions = targets
            .iter()
            .map(|target| {
                history
                    .iter()
                    .position(|commit| commit == target)
                    .ok_or_else(|| {
                        GitError::new(format!("commit is not reachable from HEAD: {target}"))
                    })
            })
            .collect::<Result<Vec<_>, GitError>>()?;
        let start = *target_positions
            .iter()
            .min()
            .ok_or_else(|| GitError::new("no commits selected"))?;
        if start == 0 {
            return Err(GitError::new("cannot rewrite root commit"));
        }
        if start == 1 && matches!(mode, ReplayMode::Squash | ReplayMode::Fixup) {
            return Err(GitError::new("cannot squash or fixup into root commit"));
        }
        let replay_commits = history[start..].to_vec();
        for commit in &replay_commits {
            if self.parent_count(commit)? > 1 {
                return Err(GitError::new(
                    "commit rewrite does not support merge commits yet",
                ));
            }
        }
        let base = history[start - 1].clone();
        let original_head = self.resolve_commit("HEAD")?;
        self.run_git_owned(vec!["reset".to_string(), "--hard".to_string(), base])?;
        let result = self.replay_from_targets(&replay_commits, &targets, &mode);
        if let Err(error) = result {
            let _ = self.run_git(&["cherry-pick", "--abort"]);
            let _ = self.run_git_owned(vec![
                "reset".to_string(),
                "--hard".to_string(),
                original_head,
            ]);
            return Err(error);
        }
        Ok(())
    }

    fn replay_from_targets(
        &self,
        replay_commits: &[String],
        targets: &[String],
        mode: &ReplayMode,
    ) -> Result<(), GitError> {
        for commit in replay_commits {
            let targeted = targets.iter().any(|target| target == commit);
            match (targeted, mode) {
                (true, ReplayMode::Delete) => {}
                (true, ReplayMode::Fixup) => {
                    self.run_git_owned(vec![
                        "cherry-pick".to_string(),
                        "--no-commit".to_string(),
                        commit.clone(),
                    ])?;
                    self.run_git(&["commit", "--amend", "--no-edit"])?;
                }
                (true, ReplayMode::Squash) => {
                    let current_message = self.commit_message("HEAD")?;
                    let target_message = self.commit_message(commit)?;
                    let message = combine_squash_messages(&current_message, &target_message);
                    self.run_git_owned(vec![
                        "cherry-pick".to_string(),
                        "--no-commit".to_string(),
                        commit.clone(),
                    ])?;
                    self.amend_message(&message)?;
                }
                (true, ReplayMode::Reword(message)) => {
                    self.run_git_owned(vec!["cherry-pick".to_string(), commit.clone()])?;
                    self.amend_message(message)?;
                }
                (false, _) => {
                    self.run_git_owned(vec!["cherry-pick".to_string(), commit.clone()])?;
                }
            }
        }
        Ok(())
    }

    fn ensure_clean_worktree(&self) -> Result<(), GitError> {
        let output = self.run_git(&["status", "--porcelain"])?;
        if output.trim().is_empty() {
            Ok(())
        } else {
            Err(GitError::new("working tree must be clean"))
        }
    }

    fn ensure_commits_are_private(&self, commit_ids: &[String]) -> Result<(), GitError> {
        let main = self.try_resolve_commit("refs/heads/main");
        let upstream = self.try_resolve_commit("@{upstream}");
        for commit_id in commit_ids {
            if let Some(main) = &main
                && self.commit_is_ancestor_of(commit_id, main)?
            {
                return Err(GitError::new(format!(
                    "commit is already merged to main: {commit_id}"
                )));
            }
            if let Some(upstream) = &upstream
                && self.commit_is_ancestor_of(commit_id, upstream)?
            {
                return Err(GitError::new(format!(
                    "commit is already pushed upstream: {commit_id}"
                )));
            }
        }
        Ok(())
    }

    fn resolve_commit(&self, commit_id: &str) -> Result<String, GitError> {
        let spec = format!("{commit_id}^{{commit}}");
        Ok(self
            .run_git_owned(vec!["rev-parse".to_string(), "--verify".to_string(), spec])?
            .trim()
            .to_string())
    }

    fn try_resolve_commit(&self, commit_id: &str) -> Option<String> {
        self.resolve_commit(commit_id).ok()
    }

    fn commit_is_ancestor_of(&self, commit_id: &str, tip: &str) -> Result<bool, GitError> {
        let output = ProcessCommand::new("git")
            .args(["merge-base", "--is-ancestor", commit_id, tip])
            .current_dir(&self.repo_path)
            .output()
            .map_err(|err| GitError::new(format!("failed to start git merge-base: {err}")))?;
        if output.status.success() {
            return Ok(true);
        }
        if output.status.code() == Some(1) {
            return Ok(false);
        }
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(GitError::new(format!("git merge-base failed: {stderr}")))
    }

    fn rev_list_reverse_head(&self) -> Result<Vec<String>, GitError> {
        Ok(self
            .run_git(&["rev-list", "--reverse", "HEAD"])?
            .lines()
            .map(str::to_string)
            .collect())
    }

    fn parent_count(&self, commit_id: &str) -> Result<usize, GitError> {
        let output = self.run_git_owned(vec![
            "rev-list".to_string(),
            "--parents".to_string(),
            "-n".to_string(),
            "1".to_string(),
            commit_id.to_string(),
        ])?;
        Ok(output.split_whitespace().count().saturating_sub(1))
    }

    fn commit_message(&self, commit_id: &str) -> Result<String, GitError> {
        self.run_git_owned(vec![
            "log".to_string(),
            "-1".to_string(),
            "--format=%B".to_string(),
            commit_id.to_string(),
        ])
    }

    fn amend_message(&self, message: &str) -> Result<(), GitError> {
        self.run_git_owned(vec![
            "commit".to_string(),
            "--amend".to_string(),
            "-m".to_string(),
            message.to_string(),
        ])
        .map(|_| ())
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

fn parse_commit_files(output: &str) -> Result<Vec<CommitFileEntry>, GitError> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(parse_commit_file_line)
        .collect()
}

fn parse_commit_file_line(line: &str) -> Result<CommitFileEntry, GitError> {
    let parts = line.split('\t').collect::<Vec<_>>();
    let Some(raw_status) = parts.first() else {
        return Err(GitError::new("missing commit file status"));
    };
    let status = match raw_status.chars().next().unwrap_or('?') {
        'A' => CommitFileStatus::Added,
        'M' => CommitFileStatus::Modified,
        'D' => CommitFileStatus::Deleted,
        'R' => CommitFileStatus::Renamed,
        'C' => CommitFileStatus::Copied,
        'T' => CommitFileStatus::TypeChanged,
        _ => CommitFileStatus::Unknown,
    };
    let (old_path, path) = match status {
        CommitFileStatus::Renamed | CommitFileStatus::Copied => {
            if parts.len() < 3 {
                return Err(GitError::new(format!("invalid commit file line: {line}")));
            }
            (Some(parts[1].to_string()), parts[2].to_string())
        }
        _ => {
            if parts.len() < 2 {
                return Err(GitError::new(format!("invalid commit file line: {line}")));
            }
            (None, parts[1].to_string())
        }
    };
    Ok(CommitFileEntry {
        path,
        old_path,
        status,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ReplayMode {
    Squash,
    Fixup,
    Reword(String),
    Delete,
}

fn combine_squash_messages(current: &str, target: &str) -> String {
    let current = current.trim_end();
    let target = target.trim_end();
    if target.is_empty() {
        current.to_string()
    } else if current.is_empty() {
        target.to_string()
    } else {
        format!("{current}\n\n{target}")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_args_follow_status_mode() {
        assert!(status_args(StatusMode::Full).contains(&"--untracked-files=all"));
        assert!(status_args(StatusMode::LargeRepoFast).contains(&"--untracked-files=no"));
    }
}
