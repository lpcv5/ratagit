use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

mod mock;

use ratagit_core::{
    BranchEntry, Command, CommitEntry, FileEntry, GitResult, RepoSnapshot, StashEntry,
};

pub use mock::MockGitBackend;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitError {
    pub message: String,
}

impl GitError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

pub trait GitBackend {
    fn refresh_snapshot(&mut self) -> Result<RepoSnapshot, GitError>;
    fn files_details_diff(&mut self, paths: &[String]) -> Result<String, GitError>;
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
    fn create_branch(&mut self, name: &str) -> Result<(), GitError>;
    fn checkout_branch(&mut self, name: &str) -> Result<(), GitError>;
    fn stash_push(&mut self, message: &str) -> Result<(), GitError>;
    fn stash_files(&mut self, message: &str, paths: &[String]) -> Result<(), GitError>;
    fn stash_pop(&mut self, stash_id: &str) -> Result<(), GitError>;
    fn discard_files(&mut self, paths: &[String]) -> Result<(), GitError>;
}

pub(crate) fn resequence_stashes(stashes: &mut [StashEntry]) {
    for (index, stash) in stashes.iter_mut().enumerate() {
        stash.id = format!("stash@{{{index}}}");
    }
}

fn remove_untracked_path(repo_path: &Path, relative_path: &str) -> Result<(), GitError> {
    let target = repo_path.join(relative_path);
    let repo = repo_path
        .canonicalize()
        .map_err(|err| GitError::new(format!("failed to resolve repo path: {err}")))?;
    let parent = target
        .parent()
        .ok_or_else(|| GitError::new(format!("invalid path: {relative_path}")))?;
    let resolved_parent = parent
        .canonicalize()
        .map_err(|err| GitError::new(format!("failed to resolve parent path: {err}")))?;
    if !resolved_parent.starts_with(&repo) {
        return Err(GitError::new(format!(
            "refusing to remove path outside repo: {relative_path}"
        )));
    }
    if target.is_dir() {
        std::fs::remove_dir_all(&target)
            .map_err(|err| GitError::new(format!("failed to remove {relative_path}: {err}")))?;
    } else {
        std::fs::remove_file(&target)
            .map_err(|err| GitError::new(format!("failed to remove {relative_path}: {err}")))?;
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct CliGitBackend {
    repo_path: PathBuf,
}

impl CliGitBackend {
    pub fn new(repo_path: impl Into<PathBuf>) -> Self {
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
}

impl GitBackend for CliGitBackend {
    fn refresh_snapshot(&mut self) -> Result<RepoSnapshot, GitError> {
        let status_output =
            self.run_git(&["status", "--short", "--branch", "--untracked-files=all"])?;
        let log_output = self.run_git(&["log", "--oneline", "-n", "10"])?;
        let branch_output = self.run_git(&["branch", "--list"])?;
        let stash_output = self.run_git(&["stash", "list"])?;

        let (current_branch, detached_head, status_summary, files) =
            parse_status_output(&status_output);
        let commits = parse_log_output(&log_output);
        let branches = parse_branches_output(&branch_output, &current_branch);
        let stashes = parse_stash_output(&stash_output);

        Ok(RepoSnapshot {
            status_summary,
            current_branch,
            detached_head,
            files,
            commits,
            branches,
            stashes,
        })
    }

    fn files_details_diff(&mut self, paths: &[String]) -> Result<String, GitError> {
        if paths.is_empty() {
            return Ok(String::new());
        }

        let mut unstaged_args = vec![
            "diff".to_string(),
            "--no-ext-diff".to_string(),
            "--".to_string(),
        ];
        unstaged_args.extend(paths.iter().cloned());
        let unstaged = self.run_git_owned(unstaged_args)?;

        let mut staged_args = vec![
            "diff".to_string(),
            "--cached".to_string(),
            "--no-ext-diff".to_string(),
            "--".to_string(),
        ];
        staged_args.extend(paths.iter().cloned());
        let staged = self.run_git_owned(staged_args)?;

        let mut sections = Vec::new();
        if !unstaged.trim().is_empty() {
            sections.push("### unstaged".to_string());
            sections.push(unstaged.trim_end().to_string());
        }
        if !staged.trim().is_empty() {
            if !sections.is_empty() {
                sections.push(String::new());
            }
            sections.push("### staged".to_string());
            sections.push(staged.trim_end().to_string());
        }

        Ok(sections.join("\n"))
    }

    fn stage_file(&mut self, path: &str) -> Result<(), GitError> {
        self.run_git(&["add", "--", path]).map(|_| ())
    }

    fn unstage_file(&mut self, path: &str) -> Result<(), GitError> {
        self.run_git(&["restore", "--staged", "--", path])
            .map(|_| ())
    }

    fn stage_files(&mut self, paths: &[String]) -> Result<(), GitError> {
        if paths.is_empty() {
            return Ok(());
        }
        let mut args = vec!["add".to_string(), "--".to_string()];
        args.extend(paths.iter().cloned());
        self.run_git_owned(args).map(|_| ())
    }

    fn unstage_files(&mut self, paths: &[String]) -> Result<(), GitError> {
        if paths.is_empty() {
            return Ok(());
        }
        let mut args = vec![
            "restore".to_string(),
            "--staged".to_string(),
            "--".to_string(),
        ];
        args.extend(paths.iter().cloned());
        self.run_git_owned(args).map(|_| ())
    }

    fn create_commit(&mut self, message: &str) -> Result<(), GitError> {
        self.run_git(&["commit", "-m", message]).map(|_| ())
    }

    fn create_branch(&mut self, name: &str) -> Result<(), GitError> {
        self.run_git(&["branch", name]).map(|_| ())
    }

    fn checkout_branch(&mut self, name: &str) -> Result<(), GitError> {
        self.run_git(&["checkout", name]).map(|_| ())
    }

    fn stash_push(&mut self, message: &str) -> Result<(), GitError> {
        if message.trim().is_empty() {
            self.run_git(&["stash", "push"]).map(|_| ())
        } else {
            self.run_git(&["stash", "push", "-m", message]).map(|_| ())
        }
    }

    fn stash_files(&mut self, message: &str, paths: &[String]) -> Result<(), GitError> {
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

    fn stash_pop(&mut self, stash_id: &str) -> Result<(), GitError> {
        self.run_git(&["stash", "pop", stash_id]).map(|_| ())
    }

    fn discard_files(&mut self, paths: &[String]) -> Result<(), GitError> {
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

pub fn execute_command(backend: &mut dyn GitBackend, command: Command) -> GitResult {
    match command {
        Command::RefreshAll => match backend.refresh_snapshot() {
            Ok(snapshot) => GitResult::Refreshed(snapshot),
            Err(error) => GitResult::RefreshFailed {
                error: error.message,
            },
        },
        Command::RefreshFilesDetailsDiff { paths } => GitResult::FilesDetailsDiff {
            paths: paths.clone(),
            result: backend
                .files_details_diff(&paths)
                .map_err(|error| error.message),
        },
        Command::StageFiles { paths } => GitResult::StageFiles {
            paths: paths.clone(),
            result: backend.stage_files(&paths).map_err(|error| error.message),
        },
        Command::UnstageFiles { paths } => GitResult::UnstageFiles {
            paths: paths.clone(),
            result: backend.unstage_files(&paths).map_err(|error| error.message),
        },
        Command::StashFiles { message, paths } => GitResult::StashFiles {
            message: message.clone(),
            paths: paths.clone(),
            result: backend
                .stash_files(&message, &paths)
                .map_err(|error| error.message),
        },
        Command::DiscardFiles { paths } => GitResult::DiscardFiles {
            paths: paths.clone(),
            result: backend.discard_files(&paths).map_err(|error| error.message),
        },
        Command::CreateCommit { message } => GitResult::CreateCommit {
            message: message.clone(),
            result: backend
                .create_commit(&message)
                .map_err(|error| error.message),
        },
        Command::CreateBranch { name } => GitResult::CreateBranch {
            name: name.clone(),
            result: backend.create_branch(&name).map_err(|error| error.message),
        },
        Command::CheckoutBranch { name } => GitResult::CheckoutBranch {
            name: name.clone(),
            result: backend
                .checkout_branch(&name)
                .map_err(|error| error.message),
        },
        Command::StashPush { message } => GitResult::StashPush {
            message: message.clone(),
            result: backend.stash_push(&message).map_err(|error| error.message),
        },
        Command::StashPop { stash_id } => GitResult::StashPop {
            stash_id: stash_id.clone(),
            result: backend.stash_pop(&stash_id).map_err(|error| error.message),
        },
    }
}

fn parse_status_output(output: &str) -> (String, bool, String, Vec<FileEntry>) {
    let mut branch = "unknown".to_string();
    let mut detached = false;
    let mut files = Vec::new();
    let mut staged = 0usize;
    let mut unstaged = 0usize;

    for (index, line) in output.lines().enumerate() {
        if index == 0 && line.starts_with("## ") {
            let header = line.trim_start_matches("## ").trim();
            let branch_part = header.split("...").next().unwrap_or(header).trim();
            branch = branch_part.to_string();
            detached = branch_part.starts_with("HEAD");
            continue;
        }

        if line.trim().is_empty() {
            continue;
        }

        let status_code = line.get(0..2).unwrap_or("  ");
        let path = line.get(3..).unwrap_or("").trim().to_string();
        let is_untracked = status_code == "??";
        let is_staged = !is_untracked && status_code.chars().next().unwrap_or(' ') != ' ';
        if is_staged {
            staged = staged.saturating_add(1);
        } else {
            unstaged = unstaged.saturating_add(1);
        }
        files.push(FileEntry {
            path,
            staged: is_staged,
            untracked: is_untracked,
        });
    }

    let summary = format!("staged: {staged}, unstaged: {unstaged}");
    (branch, detached, summary, files)
}

fn parse_log_output(output: &str) -> Vec<CommitEntry> {
    output
        .lines()
        .filter_map(|line| {
            let mut split = line.splitn(2, ' ');
            let id = split.next()?.trim();
            let summary = split.next().unwrap_or("").trim();
            if id.is_empty() || summary.is_empty() {
                return None;
            }
            Some(CommitEntry {
                id: id.to_string(),
                summary: summary.to_string(),
            })
        })
        .collect()
}

fn parse_branches_output(output: &str, current_branch: &str) -> Vec<BranchEntry> {
    output
        .lines()
        .filter_map(|line| {
            let clean = line.trim();
            if clean.is_empty() {
                return None;
            }
            let (marker, name) = if clean.starts_with('*') {
                ("*", clean.trim_start_matches('*').trim())
            } else {
                (" ", clean)
            };
            Some(BranchEntry {
                name: name.to_string(),
                is_current: marker == "*" || name == current_branch,
            })
        })
        .collect()
}

fn parse_stash_output(output: &str) -> Vec<StashEntry> {
    output
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(2, ':');
            let id = parts.next()?.trim().to_string();
            let summary = parts.next().unwrap_or("").trim().to_string();
            if id.is_empty() {
                return None;
            }
            Some(StashEntry { id, summary })
        })
        .collect()
}

pub fn is_git_repo(path: &Path) -> bool {
    path.join(".git").exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_backend_mutates_state() {
        let mut backend = MockGitBackend::new(RepoSnapshot {
            status_summary: "dirty".to_string(),
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
            stashes: Vec::new(),
        });

        backend.stage_file("a.txt").expect("stage should work");
        backend
            .create_commit("first")
            .expect("create commit should work");
        backend
            .create_branch("feature/mvp")
            .expect("create branch should work");
        backend
            .checkout_branch("feature/mvp")
            .expect("checkout should work");
        backend
            .stash_push("checkpoint")
            .expect("stash push should work");
        let stash_id = backend.snapshot().stashes[0].id.clone();
        backend.stash_pop(&stash_id).expect("stash pop should work");

        assert!(backend.snapshot().files.is_empty());
        assert_eq!(backend.snapshot().current_branch, "feature/mvp");
        assert!(backend.snapshot().stashes.is_empty());
    }

    #[test]
    fn parse_status_output_extracts_branch_and_files() {
        let output = "## main...origin/main\n M src/lib.rs\nA  src/app.rs\n?? notes.txt\n";
        let (branch, detached, summary, files) = parse_status_output(output);
        assert_eq!(branch, "main");
        assert!(!detached);
        assert_eq!(summary, "staged: 1, unstaged: 2");
        assert_eq!(files.len(), 3);
        assert!(!files[0].staged);
        assert!(files[1].staged);
        assert!(!files[0].untracked);
        assert!(files[2].untracked);
    }

    #[test]
    fn parse_status_output_preserves_untracked_directory_marker_path() {
        let output = "## main\n?? libs/ratagit-git/tests/\n";
        let (_branch, _detached, _summary, files) = parse_status_output(output);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "libs/ratagit-git/tests/");
        assert!(files[0].untracked);
        assert!(!files[0].staged);
    }

    #[test]
    fn parse_status_output_keeps_untracked_nested_file_path() {
        let output = "## main\n?? libs/ratagit-git/tests/cli_tmp.rs\n";
        let (_branch, _detached, summary, files) = parse_status_output(output);
        assert_eq!(summary, "staged: 0, unstaged: 1");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "libs/ratagit-git/tests/cli_tmp.rs");
        assert!(files[0].untracked);
        assert!(!files[0].staged);
    }

    #[test]
    fn parse_log_output_extracts_commits() {
        let output = "abc1234 first commit\ndef5678 second commit\n";
        let commits = parse_log_output(output);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].id, "abc1234");
        assert_eq!(commits[1].summary, "second commit");
    }

    #[test]
    fn execute_command_refresh_files_details_diff_uses_backend_output() {
        let mut backend = MockGitBackend::new(RepoSnapshot {
            status_summary: "dirty".to_string(),
            current_branch: "main".to_string(),
            detached_head: false,
            files: vec![
                FileEntry {
                    path: "src/lib.rs".to_string(),
                    staged: false,
                    untracked: false,
                },
                FileEntry {
                    path: "src/main.rs".to_string(),
                    staged: true,
                    untracked: false,
                },
            ],
            commits: Vec::new(),
            branches: vec![BranchEntry {
                name: "main".to_string(),
                is_current: true,
            }],
            stashes: Vec::new(),
        });

        let result = execute_command(
            &mut backend,
            Command::RefreshFilesDetailsDiff {
                paths: vec!["src/lib.rs".to_string(), "src/main.rs".to_string()],
            },
        );

        match result {
            GitResult::FilesDetailsDiff { paths, result } => {
                assert_eq!(
                    paths,
                    vec!["src/lib.rs".to_string(), "src/main.rs".to_string()]
                );
                let diff = result.expect("mock diff should succeed");
                assert!(diff.contains("### unstaged"));
                assert!(diff.contains("### staged"));
                assert!(diff.contains("diff --git a/src/lib.rs b/src/lib.rs"));
                assert!(diff.contains("diff --git a/src/main.rs b/src/main.rs"));
            }
            other => panic!("unexpected git result: {other:?}"),
        }
    }
}
