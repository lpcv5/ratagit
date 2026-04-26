use std::fmt;
use std::path::{Component, Path};

mod cli;
mod hybrid;
mod mock;
mod untracked_diff;

use ratagit_core::{Command, GitResult, RepoSnapshot, ResetMode, StashEntry};

pub use hybrid::HybridGitBackend;
pub use mock::MockGitBackend;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitError {
    pub message: String,
}

impl GitError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl From<git2::Error> for GitError {
    fn from(error: git2::Error) -> Self {
        Self::new(error.message().to_string())
    }
}

impl fmt::Display for GitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(formatter)
    }
}

impl std::error::Error for GitError {}

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
    fn reset(&mut self, mode: ResetMode) -> Result<(), GitError>;
    fn nuke(&mut self) -> Result<(), GitError>;
    fn discard_files(&mut self, paths: &[String]) -> Result<(), GitError>;
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
        Command::Reset { mode } => GitResult::Reset {
            mode,
            result: backend.reset(mode).map_err(|error| error.message),
        },
        Command::Nuke => GitResult::Nuke {
            result: backend.nuke().map_err(|error| error.message),
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

pub fn is_git_repo(path: &Path) -> bool {
    git2::Repository::discover(path).is_ok()
}

pub(crate) fn resequence_stashes(stashes: &mut [StashEntry]) {
    for (index, stash) in stashes.iter_mut().enumerate() {
        stash.id = format!("stash@{{{index}}}");
    }
}

pub(crate) fn validate_repo_relative_path(path: &str) -> Result<&Path, GitError> {
    let repo_path = Path::new(path);
    if repo_path.as_os_str().is_empty() {
        return Err(GitError::new("path cannot be empty"));
    }
    if repo_path.is_absolute() {
        return Err(GitError::new(format!(
            "path must be relative to repo: {path}"
        )));
    }
    if repo_path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(GitError::new(format!("invalid repo-relative path: {path}")));
    }
    Ok(repo_path)
}

#[cfg(test)]
mod tests {
    use ratagit_core::{BranchEntry, CommitEntry, FileEntry, ResetMode};

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
            commits: vec![CommitEntry {
                id: "mock-0001".to_string(),
                summary: "initial".to_string(),
            }],
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

    #[test]
    fn validate_repo_relative_path_rejects_unsafe_paths() {
        assert!(validate_repo_relative_path("src/lib.rs").is_ok());
        assert!(validate_repo_relative_path("").is_err());
        assert!(validate_repo_relative_path("../outside.txt").is_err());
        assert!(validate_repo_relative_path("src/../outside.txt").is_err());
        assert!(validate_repo_relative_path("/tmp/outside.txt").is_err());
    }

    #[test]
    fn mock_reset_modes_apply_expected_status_changes() {
        let snapshot = RepoSnapshot {
            status_summary: "dirty".to_string(),
            current_branch: "main".to_string(),
            detached_head: false,
            files: vec![
                FileEntry {
                    path: "staged.txt".to_string(),
                    staged: true,
                    untracked: false,
                },
                FileEntry {
                    path: "unstaged.txt".to_string(),
                    staged: false,
                    untracked: false,
                },
                FileEntry {
                    path: "new.txt".to_string(),
                    staged: false,
                    untracked: true,
                },
            ],
            commits: Vec::new(),
            branches: Vec::new(),
            stashes: Vec::new(),
        };

        let mut soft = MockGitBackend::new(snapshot.clone());
        soft.reset(ResetMode::Soft).expect("soft reset should work");
        assert_eq!(soft.snapshot().files, snapshot.files);

        let mut mixed = MockGitBackend::new(snapshot.clone());
        mixed
            .reset(ResetMode::Mixed)
            .expect("mixed reset should work");
        assert!(mixed.snapshot().files.iter().all(|entry| !entry.staged));
        assert!(mixed.snapshot().files.iter().any(|entry| entry.untracked));

        let mut hard = MockGitBackend::new(snapshot.clone());
        hard.reset(ResetMode::Hard).expect("hard reset should work");
        assert_eq!(hard.snapshot().files.len(), 1);
        assert_eq!(hard.snapshot().files[0].path, "new.txt");

        let mut nuke = MockGitBackend::new(snapshot);
        nuke.nuke().expect("nuke should work");
        assert!(nuke.snapshot().files.is_empty());
        assert_eq!(nuke.operations(), &["nuke".to_string()]);
    }
}
