use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use ratagit_core::{
    BranchDeleteMode, CommitFileDiffTarget, CommitFileEntry, CommitFileStatus, FileEntry,
    GitErrorKind, ResetMode, StatusMode,
};

use crate::status_cli::parse_porcelain_v1_z_limited;
use crate::{GitError, validate_repo_relative_path};

pub(crate) const STATUS_ENTRY_LIMIT: usize = 50_000;
pub(crate) const STATUS_OUTPUT_LIMIT_BYTES: usize = 64 * 1024 * 1024;
pub(crate) const COMMIT_DETAILS_DIFF_OUTPUT_LIMIT_BYTES: usize = 1024 * 1024;
pub(crate) const FILES_DETAILS_DIFF_OUTPUT_LIMIT_BYTES: usize = 1024 * 1024;

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
            StatusMode::LargeRepoFast | StatusMode::HugeRepoMetadataOnly => "--untracked-files=no",
        },
        "--ignored=no",
        "--ignore-submodules=all",
    ]
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GitCommandOptions {
    optional_locks_disabled: bool,
    stdout_limit: Option<usize>,
    capture_stderr: bool,
    timeout: Option<Duration>,
}

impl GitCommandOptions {
    fn new(optional_locks_disabled: bool) -> Self {
        Self {
            optional_locks_disabled,
            stdout_limit: None,
            capture_stderr: true,
            timeout: None,
        }
    }

    fn with_stdout_limit(mut self, stdout_limit: usize) -> Self {
        self.stdout_limit = Some(stdout_limit);
        self
    }
}

#[derive(Debug)]
struct GitCommandOutput {
    stdout: Vec<u8>,
    stdout_truncated: bool,
}

struct RawGitCommandOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    stdout_truncated: bool,
    elapsed_ms: u128,
}

struct GitCommandRunner<'repo> {
    repo_path: &'repo Path,
}

impl<'repo> GitCommandRunner<'repo> {
    fn new(repo_path: &'repo Path) -> Self {
        Self { repo_path }
    }

    fn run<I, S>(&self, args: I, options: GitCommandOptions) -> Result<GitCommandOutput, GitError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let args = args
            .into_iter()
            .map(|arg| arg.as_ref().to_string())
            .collect::<Vec<_>>();
        if let Some(stdout_limit) = options.stdout_limit {
            self.run_limited(args, options, stdout_limit)
        } else {
            self.run_full(args, options)
        }
    }

    fn run_full(
        &self,
        args: Vec<String>,
        options: GitCommandOptions,
    ) -> Result<GitCommandOutput, GitError> {
        let started = Instant::now();
        let output = self
            .command(&args, &options)
            .output()
            .map_err(|err| GitError::io(format!("failed to start git {:?}: {err}", args)))?;
        let elapsed_ms = started.elapsed().as_millis();
        let stderr = if options.capture_stderr {
            output.stderr
        } else {
            Vec::new()
        };
        self.finish(
            &args,
            &options,
            RawGitCommandOutput {
                status: output.status,
                stdout: output.stdout,
                stderr,
                stdout_truncated: false,
                elapsed_ms,
            },
        )
    }

    fn run_limited(
        &self,
        args: Vec<String>,
        options: GitCommandOptions,
        stdout_limit: usize,
    ) -> Result<GitCommandOutput, GitError> {
        let started = Instant::now();
        let mut command = self.command(&args, &options);
        command.stdout(Stdio::piped());
        if options.capture_stderr {
            command.stderr(Stdio::piped());
        }
        let mut child = command
            .spawn()
            .map_err(|err| GitError::io(format!("failed to start git {:?}: {err}", args)))?;
        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| GitError::io("failed to capture git stdout"))?;
        let mut bytes = Vec::new();
        let mut truncated = false;
        let mut buffer = [0u8; 8192];
        loop {
            let read = stdout
                .read(&mut buffer)
                .map_err(|err| GitError::io(format!("failed to read git stdout: {err}")))?;
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
        drop(stdout);
        let output = child
            .wait_with_output()
            .map_err(|err| GitError::io(format!("failed to wait for git {:?}: {err}", args)))?;
        let elapsed_ms = started.elapsed().as_millis();
        let stderr = if options.capture_stderr {
            output.stderr
        } else {
            Vec::new()
        };
        self.finish(
            &args,
            &options,
            RawGitCommandOutput {
                status: output.status,
                stdout: bytes,
                stderr,
                stdout_truncated: truncated,
                elapsed_ms,
            },
        )
    }

    fn command(&self, args: &[String], options: &GitCommandOptions) -> ProcessCommand {
        let mut command = ProcessCommand::new("git");
        command.args(args).current_dir(self.repo_path);
        if options.optional_locks_disabled {
            command.env("GIT_OPTIONAL_LOCKS", "0");
        }
        let _timeout_ready = options.timeout;
        command
    }

    fn finish(
        &self,
        args: &[String],
        options: &GitCommandOptions,
        output: RawGitCommandOutput,
    ) -> Result<GitCommandOutput, GitError> {
        let subcommand = args.first().map_or("unknown", String::as_str);
        if !output.stdout_truncated && !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            tracing::warn!(
                target: "ratagit.git.cli",
                git_subcommand = subcommand,
                optional_locks_disabled = options.optional_locks_disabled,
                exit_code = ?output.status.code(),
                elapsed_ms = output.elapsed_ms,
                stdout_bytes = output.stdout.len(),
                stderr = %stderr,
                "git cli command failed"
            );
            let kind = classify_cli_error(args, &stderr);
            return Err(GitError::cli(
                kind,
                format!("git {:?} failed: {}", args, stderr),
            ));
        }

        if output.stdout_truncated {
            tracing::warn!(
                target: "ratagit.git.cli",
                git_subcommand = subcommand,
                optional_locks_disabled = options.optional_locks_disabled,
                elapsed_ms = output.elapsed_ms,
                stdout_bytes = output.stdout.len(),
                stdout_limit = options.stdout_limit,
                "git cli output truncated"
            );
        } else {
            tracing::debug!(
                target: "ratagit.git.cli",
                git_subcommand = subcommand,
                optional_locks_disabled = options.optional_locks_disabled,
                elapsed_ms = output.elapsed_ms,
                stdout_bytes = output.stdout.len(),
                "git cli command completed"
            );
        }

        Ok(GitCommandOutput {
            stdout: output.stdout,
            stdout_truncated: output.stdout_truncated,
        })
    }
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
        GitCommandRunner::new(&self.repo_path)
            .run(args, GitCommandOptions::new(optional_locks_disabled))
            .map(|output| output.stdout)
    }

    fn run_git_read_output_limited<I, S>(
        &self,
        args: I,
        stdout_limit: usize,
    ) -> Result<(Vec<u8>, bool), GitError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let output = GitCommandRunner::new(&self.repo_path).run(
            args,
            GitCommandOptions::new(true).with_stdout_limit(stdout_limit),
        )?;
        Ok((output.stdout, output.stdout_truncated))
    }

    pub(crate) fn status_files(&self, mode: StatusMode) -> Result<StatusFilesResult, GitError> {
        let args = status_args(mode);
        let (mut output, output_truncated) =
            self.run_git_read_output_limited(args, STATUS_OUTPUT_LIMIT_BYTES)?;
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

    pub(crate) fn commit_log_page(&self, offset: usize, limit: usize) -> Result<Vec<u8>, GitError> {
        self.commit_log_page_for_revision(None, offset, limit)
    }

    pub(crate) fn branch_commit_log_page(
        &self,
        branch: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<u8>, GitError> {
        self.commit_log_page_for_revision(Some(branch), offset, limit)
    }

    fn commit_log_page_for_revision(
        &self,
        revision: Option<&str>,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<u8>, GitError> {
        let mut args = vec![
            "log".to_string(),
            format!("--skip={offset}"),
            "-n".to_string(),
            limit.to_string(),
            "--format=%x1e%H%x00%h%x00%P%x00%an%x00%B%x00".to_string(),
        ];
        if let Some(revision) = revision {
            args.push(revision.to_string());
        }
        self.run_git_output_with_options(args, true)
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

    pub(crate) fn files_unstaged_diff(&mut self, paths: &[String]) -> Result<String, GitError> {
        self.files_diff(false, paths)
    }

    pub(crate) fn files_staged_diff(&mut self, paths: &[String]) -> Result<String, GitError> {
        self.files_diff(true, paths)
    }

    fn files_diff(&mut self, staged: bool, paths: &[String]) -> Result<String, GitError> {
        let mut args = vec![
            "diff".to_string(),
            "--color=always".to_string(),
            "--no-ext-diff".to_string(),
            "--no-textconv".to_string(),
        ];
        if staged {
            args.push("--cached".to_string());
        }
        args.push("--".to_string());
        for path in paths {
            args.push(literal_pathspec(path)?);
        }

        let (mut diff, truncated) =
            self.run_git_read_text_limited(args, FILES_DETAILS_DIFF_OUTPUT_LIMIT_BYTES)?;
        if truncated {
            append_diff_truncation_notice(
                &mut diff,
                "files diff",
                FILES_DETAILS_DIFF_OUTPUT_LIMIT_BYTES,
            );
        }
        Ok(diff)
    }

    pub(crate) fn commit_details_diff(&mut self, commit_id: &str) -> Result<String, GitError> {
        let (mut diff, truncated) = self.run_git_read_text_limited(
            vec![
                "show".to_string(),
                "--color=always".to_string(),
                "--no-ext-diff".to_string(),
                "--no-textconv".to_string(),
                "--no-renames".to_string(),
                "--format=fuller".to_string(),
                "--patch".to_string(),
                commit_id.to_string(),
            ],
            COMMIT_DETAILS_DIFF_OUTPUT_LIMIT_BYTES,
        )?;
        if truncated {
            append_diff_truncation_notice(
                &mut diff,
                "commit diff",
                COMMIT_DETAILS_DIFF_OUTPUT_LIMIT_BYTES,
            );
        }
        Ok(diff)
    }

    fn run_git_read_text_limited(
        &self,
        args: Vec<String>,
        stdout_limit: usize,
    ) -> Result<(String, bool), GitError> {
        self.run_git_read_output_limited(args, stdout_limit)
            .map(|(stdout, truncated)| (String::from_utf8_lossy(&stdout).to_string(), truncated))
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
            "--color=always".to_string(),
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
                args.push(literal_pathspec(old_path)?);
            }
            if pushed.insert(path.path.clone()) {
                args.push(literal_pathspec(&path.path)?);
            }
        }
        let (mut diff, truncated) =
            self.run_git_read_text_limited(args, COMMIT_DETAILS_DIFF_OUTPUT_LIMIT_BYTES)?;
        if truncated {
            append_diff_truncation_notice(
                &mut diff,
                "commit diff",
                COMMIT_DETAILS_DIFF_OUTPUT_LIMIT_BYTES,
            );
        }
        Ok(diff)
    }

    pub(crate) fn create_commit(&mut self, message: &str) -> Result<(), GitError> {
        self.run_git(&["commit", "-m", message]).map(|_| ())
    }

    pub(crate) fn amend_staged_changes(&mut self, commit_id: &str) -> Result<(), GitError> {
        self.ensure_only_staged_changes()?;
        let target = self.resolve_commit(commit_id)?;
        let original_head = self.resolve_commit("HEAD")?;
        if target == original_head {
            return self
                .run_git(&["commit", "--amend", "--no-edit"])
                .map(|_| ());
        }

        let history = self.rev_list_reverse_head()?;
        let start = history
            .iter()
            .position(|commit| commit == &target)
            .ok_or_else(|| GitError::new(format!("commit is not reachable from HEAD: {target}")))?;
        if start == 0 {
            return Err(GitError::new("cannot amend root commit"));
        }
        let replay_commits = history[start..].to_vec();
        for commit in &replay_commits {
            if self.parent_count(commit)? > 1 {
                return Err(GitError::new(
                    "commit rewrite does not support merge commits yet",
                ));
            }
        }

        self.run_git(&["commit", "-m", "ratagit amend staged changes"])?;
        let staged_commit = self.resolve_commit("HEAD")?;
        let base = history[start - 1].clone();
        self.run_git_owned(vec!["reset".to_string(), "--hard".to_string(), base])?;
        let result = self.replay_with_staged_amend(&replay_commits, &target, &staged_commit);
        if let Err(error) = result {
            let _ = self.run_git(&["cherry-pick", "--abort"]);
            let _ = self.run_git_owned(vec![
                "reset".to_string(),
                "--hard".to_string(),
                staged_commit,
            ]);
            let _ = self.run_git_owned(vec![
                "reset".to_string(),
                "--soft".to_string(),
                original_head,
            ]);
            return Err(error);
        }
        Ok(())
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

    pub(crate) fn pull(&mut self) -> Result<(), GitError> {
        self.run_git(&["pull"]).map(|_| ())
    }

    pub(crate) fn push(&mut self, force: bool) -> Result<(), GitError> {
        if force {
            self.run_git(&["push", "--force-with-lease"]).map(|_| ())
        } else {
            self.run_git(&["push"]).map(|_| ())
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

    fn replay_with_staged_amend(
        &self,
        replay_commits: &[String],
        target: &str,
        staged_commit: &str,
    ) -> Result<(), GitError> {
        for commit in replay_commits {
            self.run_git_owned(vec!["cherry-pick".to_string(), commit.clone()])?;
            if commit == target {
                self.run_git_owned(vec![
                    "cherry-pick".to_string(),
                    "--no-commit".to_string(),
                    staged_commit.to_string(),
                ])?;
                self.run_git(&["commit", "--amend", "--no-edit"])?;
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

    fn ensure_only_staged_changes(&self) -> Result<(), GitError> {
        let output = self.run_git(&["status", "--porcelain"])?;
        let mut has_staged = false;
        for line in output.lines() {
            let mut chars = line.chars();
            let index_status = chars.next().unwrap_or(' ');
            let worktree_status = chars.next().unwrap_or(' ');
            if index_status == '?' && worktree_status == '?' {
                return Err(GitError::new(
                    "amend requires only staged changes; untracked changes are present",
                ));
            }
            if worktree_status != ' ' {
                return Err(GitError::new(
                    "amend requires only staged changes; unstaged changes are present",
                ));
            }
            if index_status != ' ' {
                has_staged = true;
            }
        }
        if has_staged {
            Ok(())
        } else {
            Err(GitError::new("no staged changes to amend"))
        }
    }

    fn resolve_commit(&self, commit_id: &str) -> Result<String, GitError> {
        let spec = format!("{commit_id}^{{commit}}");
        Ok(self
            .run_git_owned(vec!["rev-parse".to_string(), "--verify".to_string(), spec])?
            .trim()
            .to_string())
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
                    return Ok(current_path);
                }
            } else if line.is_empty() {
                current_path = None;
            }
        }
        Ok(None)
    }
}

pub(crate) fn classify_cli_error(args: &[String], stderr: &str) -> GitErrorKind {
    let lower = stderr.to_ascii_lowercase();
    if args.first().is_some_and(|arg| arg == "push") && is_divergent_push_stderr(&lower) {
        return GitErrorKind::DivergentPush;
    }
    if args.first().is_some_and(|arg| arg == "branch") && is_unmerged_branch_delete_stderr(&lower) {
        return GitErrorKind::UnmergedBranchDelete;
    }
    GitErrorKind::Cli
}

fn is_divergent_push_stderr(lower_stderr: &str) -> bool {
    lower_stderr.contains("non-fast-forward")
        || lower_stderr.contains("fetch first")
        || lower_stderr.contains("rejected") && lower_stderr.contains("fetch")
        || lower_stderr.contains("remote contains work")
        || lower_stderr.contains("failed to push some refs")
}

fn is_unmerged_branch_delete_stderr(lower_stderr: &str) -> bool {
    lower_stderr.contains("not fully merged") || lower_stderr.contains("not merged")
}

fn parse_commit_files(output: &str) -> Result<Vec<CommitFileEntry>, GitError> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(parse_commit_file_line)
        .collect()
}

fn append_diff_truncation_notice(diff: &mut String, label: &str, limit_bytes: usize) {
    if !diff.ends_with('\n') {
        diff.push('\n');
    }
    diff.push('\n');
    diff.push_str(&format!("### {label} truncated at {limit_bytes} bytes\n"));
}

fn literal_pathspec(path: &str) -> Result<String, GitError> {
    validate_repo_relative_path(path)?;
    Ok(format!(":(literal){path}"))
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
        assert!(status_args(StatusMode::HugeRepoMetadataOnly).contains(&"--untracked-files=no"));
    }

    #[test]
    fn git_command_options_keep_lifecycle_defaults_explicit() {
        let options = GitCommandOptions::new(true).with_stdout_limit(1024);

        assert!(options.optional_locks_disabled);
        assert_eq!(options.stdout_limit, Some(1024));
        assert!(options.capture_stderr);
        assert_eq!(options.timeout, None);
    }

    #[test]
    fn git_command_runner_start_failure_is_io() {
        let missing_repo = std::env::temp_dir().join(format!(
            "ratagit-missing-repo-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        let runner = GitCommandRunner::new(&missing_repo);

        let error = runner
            .run(["status"], GitCommandOptions::new(false))
            .expect_err("missing working directory should fail before git exits");

        assert_eq!(error.kind, GitErrorKind::Io);
    }

    #[test]
    fn git_command_runner_limited_output_reports_truncation() {
        if !git_available() {
            eprintln!(
                "git is unavailable, skipping git_command_runner_limited_output_reports_truncation"
            );
            return;
        }
        let runner = GitCommandRunner::new(Path::new("."));

        let output = runner
            .run(
                ["--version"],
                GitCommandOptions::new(false).with_stdout_limit(4),
            )
            .expect("git --version should run");

        assert_eq!(output.stdout.len(), 4);
        assert!(output.stdout_truncated);
    }

    #[test]
    fn git_command_runner_non_zero_exit_uses_cli_classification() {
        if !git_available() {
            eprintln!(
                "git is unavailable, skipping git_command_runner_non_zero_exit_uses_cli_classification"
            );
            return;
        }
        let runner = GitCommandRunner::new(Path::new("."));

        let error = runner
            .run(
                ["definitely-not-a-git-command"],
                GitCommandOptions::new(false),
            )
            .expect_err("unknown git subcommand should fail after process start");

        assert_eq!(error.kind, GitErrorKind::Cli);
    }

    #[test]
    fn append_diff_truncation_notice_starts_new_section() {
        let mut diff = "commit abc\n+partial".to_string();

        append_diff_truncation_notice(&mut diff, "commit diff", 42);

        assert_eq!(
            diff,
            "commit abc\n+partial\n\n### commit diff truncated at 42 bytes\n"
        );
    }

    #[test]
    fn classify_cli_error_detects_divergent_push() {
        let args = vec!["push".to_string()];

        let kind = classify_cli_error(&args, "! [rejected] main -> main (fetch first)");

        assert_eq!(kind, GitErrorKind::DivergentPush);
    }

    #[test]
    fn classify_cli_error_detects_unmerged_branch_delete() {
        let args = vec![
            "branch".to_string(),
            "-d".to_string(),
            "feature".to_string(),
        ];

        let kind = classify_cli_error(&args, "error: The branch 'feature' is not fully merged.");

        assert_eq!(kind, GitErrorKind::UnmergedBranchDelete);
    }

    #[test]
    fn classify_cli_error_keeps_other_cli_errors_generic() {
        let args = vec!["status".to_string()];

        let kind = classify_cli_error(&args, "fatal: not a git repository");

        assert_eq!(kind, GitErrorKind::Cli);
    }

    fn git_available() -> bool {
        ProcessCommand::new("git")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    }
}
