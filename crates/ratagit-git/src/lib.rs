use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

use ratagit_core::{
    BranchEntry, Command, CommitEntry, FileEntry, GitResult, RepoSnapshot, StashEntry,
};

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
    fn stage_file(&mut self, path: &str) -> Result<(), GitError>;
    fn unstage_file(&mut self, path: &str) -> Result<(), GitError>;
    fn create_commit(&mut self, message: &str) -> Result<(), GitError>;
    fn create_branch(&mut self, name: &str) -> Result<(), GitError>;
    fn checkout_branch(&mut self, name: &str) -> Result<(), GitError>;
    fn stash_push(&mut self, message: &str) -> Result<(), GitError>;
    fn stash_pop(&mut self, stash_id: &str) -> Result<(), GitError>;
}

#[derive(Debug, Clone)]
pub struct MockGitBackend {
    snapshot: RepoSnapshot,
    operations: Vec<String>,
    commit_sequence: u64,
}

impl MockGitBackend {
    pub fn new(snapshot: RepoSnapshot) -> Self {
        Self {
            snapshot,
            operations: Vec::new(),
            commit_sequence: 1,
        }
    }

    pub fn operations(&self) -> &[String] {
        &self.operations
    }

    pub fn snapshot(&self) -> &RepoSnapshot {
        &self.snapshot
    }
}

impl GitBackend for MockGitBackend {
    fn refresh_snapshot(&mut self) -> Result<RepoSnapshot, GitError> {
        self.operations.push("refresh".to_string());
        Ok(self.snapshot.clone())
    }

    fn stage_file(&mut self, path: &str) -> Result<(), GitError> {
        self.operations.push(format!("stage:{path}"));
        let entry = self
            .snapshot
            .files
            .iter_mut()
            .find(|entry| entry.path == path)
            .ok_or_else(|| GitError::new(format!("file not found: {path}")))?;
        entry.staged = true;
        Ok(())
    }

    fn unstage_file(&mut self, path: &str) -> Result<(), GitError> {
        self.operations.push(format!("unstage:{path}"));
        let entry = self
            .snapshot
            .files
            .iter_mut()
            .find(|entry| entry.path == path)
            .ok_or_else(|| GitError::new(format!("file not found: {path}")))?;
        entry.staged = false;
        Ok(())
    }

    fn create_commit(&mut self, message: &str) -> Result<(), GitError> {
        self.operations.push(format!("commit:{message}"));
        if message.trim().is_empty() {
            return Err(GitError::new("commit message cannot be empty"));
        }
        self.snapshot.commits.insert(
            0,
            CommitEntry {
                id: format!("mock-{:04}", self.commit_sequence),
                summary: message.to_string(),
            },
        );
        self.commit_sequence = self.commit_sequence.saturating_add(1);
        self.snapshot.files.retain(|entry| !entry.staged);
        Ok(())
    }

    fn create_branch(&mut self, name: &str) -> Result<(), GitError> {
        self.operations.push(format!("create-branch:{name}"));
        if self
            .snapshot
            .branches
            .iter()
            .any(|branch| branch.name == name)
        {
            return Err(GitError::new(format!("branch already exists: {name}")));
        }
        self.snapshot.branches.push(BranchEntry {
            name: name.to_string(),
            is_current: false,
        });
        Ok(())
    }

    fn checkout_branch(&mut self, name: &str) -> Result<(), GitError> {
        self.operations.push(format!("checkout-branch:{name}"));
        if !self
            .snapshot
            .branches
            .iter()
            .any(|branch| branch.name == name)
        {
            return Err(GitError::new(format!("branch not found: {name}")));
        }
        for branch in &mut self.snapshot.branches {
            branch.is_current = branch.name == name;
        }
        self.snapshot.current_branch = name.to_string();
        self.snapshot.detached_head = false;
        Ok(())
    }

    fn stash_push(&mut self, message: &str) -> Result<(), GitError> {
        self.operations.push(format!("stash-push:{message}"));
        let clean_message = if message.trim().is_empty() {
            "WIP".to_string()
        } else {
            message.to_string()
        };
        self.snapshot.stashes.insert(
            0,
            StashEntry {
                id: "stash@{0}".to_string(),
                summary: clean_message,
            },
        );
        resequence_stashes(&mut self.snapshot.stashes);
        Ok(())
    }

    fn stash_pop(&mut self, stash_id: &str) -> Result<(), GitError> {
        self.operations.push(format!("stash-pop:{stash_id}"));
        let index = self
            .snapshot
            .stashes
            .iter()
            .position(|entry| entry.id == stash_id)
            .ok_or_else(|| GitError::new(format!("stash not found: {stash_id}")))?;
        self.snapshot.stashes.remove(index);
        resequence_stashes(&mut self.snapshot.stashes);
        Ok(())
    }
}

fn resequence_stashes(stashes: &mut [StashEntry]) {
    for (index, stash) in stashes.iter_mut().enumerate() {
        stash.id = format!("stash@{{{index}}}");
    }
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
}

impl GitBackend for CliGitBackend {
    fn refresh_snapshot(&mut self) -> Result<RepoSnapshot, GitError> {
        let status_output = self.run_git(&["status", "--short", "--branch"])?;
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

    fn stage_file(&mut self, path: &str) -> Result<(), GitError> {
        self.run_git(&["add", "--", path]).map(|_| ())
    }

    fn unstage_file(&mut self, path: &str) -> Result<(), GitError> {
        self.run_git(&["restore", "--staged", "--", path])
            .map(|_| ())
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

    fn stash_pop(&mut self, stash_id: &str) -> Result<(), GitError> {
        self.run_git(&["stash", "pop", stash_id]).map(|_| ())
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
        Command::StageFile { path } => GitResult::StageFile {
            path: path.clone(),
            result: backend.stage_file(&path).map_err(|error| error.message),
        },
        Command::UnstageFile { path } => GitResult::UnstageFile {
            path: path.clone(),
            result: backend.unstage_file(&path).map_err(|error| error.message),
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
        let is_staged = status_code.chars().next().unwrap_or(' ') != ' ';
        if is_staged {
            staged = staged.saturating_add(1);
        } else {
            unstaged = unstaged.saturating_add(1);
        }
        files.push(FileEntry {
            path,
            staged: is_staged,
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
        let output = "## main...origin/main\n M src/lib.rs\nA  src/app.rs\n";
        let (branch, detached, summary, files) = parse_status_output(output);
        assert_eq!(branch, "main");
        assert!(!detached);
        assert_eq!(summary, "staged: 1, unstaged: 1");
        assert_eq!(files.len(), 2);
        assert!(!files[0].staged);
        assert!(files[1].staged);
    }

    #[test]
    fn parse_log_output_extracts_commits() {
        let output = "abc1234 first commit\ndef5678 second commit\n";
        let commits = parse_log_output(output);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].id, "abc1234");
        assert_eq!(commits[1].summary, "second commit");
    }
}
