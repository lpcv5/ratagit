use std::fmt;
use std::path::{Component, Path};

mod cli;
mod hybrid;
mod mock;
mod status_cli;
mod untracked_diff;

use ratagit_core::{
    BranchDeleteMode, Command, CommitEntry, CommitFileDiffTarget, CommitFileEntry, GitResult,
    RepoSnapshot, ResetMode, StashEntry,
};

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
    fn load_more_commits(
        &mut self,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<CommitEntry>, GitError>;
    fn files_details_diff(&mut self, paths: &[String]) -> Result<String, GitError>;
    fn branch_details_log(&mut self, branch: &str, max_count: usize) -> Result<String, GitError>;
    fn commit_details_diff(&mut self, commit_id: &str) -> Result<String, GitError>;
    fn commit_files(&mut self, commit_id: &str) -> Result<Vec<CommitFileEntry>, GitError>;
    fn commit_file_diff(&mut self, target: &CommitFileDiffTarget) -> Result<String, GitError>;
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
    fn create_branch(&mut self, name: &str, start_point: &str) -> Result<(), GitError>;
    fn checkout_branch(&mut self, name: &str, auto_stash: bool) -> Result<(), GitError>;
    fn delete_branch(
        &mut self,
        name: &str,
        mode: BranchDeleteMode,
        force: bool,
    ) -> Result<(), GitError>;
    fn rebase_branch(
        &mut self,
        target: &str,
        interactive: bool,
        auto_stash: bool,
    ) -> Result<(), GitError>;
    fn squash_commits(&mut self, commit_ids: &[String]) -> Result<(), GitError>;
    fn fixup_commits(&mut self, commit_ids: &[String]) -> Result<(), GitError>;
    fn reword_commit(&mut self, commit_id: &str, message: &str) -> Result<(), GitError>;
    fn delete_commits(&mut self, commit_ids: &[String]) -> Result<(), GitError>;
    fn checkout_commit_detached(
        &mut self,
        commit_id: &str,
        auto_stash: bool,
    ) -> Result<(), GitError>;
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
        Command::LoadMoreCommits {
            offset,
            limit,
            epoch,
        } => GitResult::CommitsPage {
            offset,
            limit,
            epoch,
            result: backend
                .load_more_commits(offset, limit)
                .map_err(|error| error.message),
        },
        Command::RefreshFilesDetailsDiff { paths } => GitResult::FilesDetailsDiff {
            paths: paths.clone(),
            result: backend
                .files_details_diff(&paths)
                .map_err(|error| error.message),
        },
        Command::RefreshBranchDetailsLog { branch, max_count } => GitResult::BranchDetailsLog {
            branch: branch.clone(),
            result: backend
                .branch_details_log(&branch, max_count)
                .map_err(|error| error.message),
        },
        Command::RefreshCommitDetailsDiff { commit_id } => GitResult::CommitDetailsDiff {
            commit_id: commit_id.clone(),
            result: backend
                .commit_details_diff(&commit_id)
                .map_err(|error| error.message),
        },
        Command::RefreshCommitFiles { commit_id } => GitResult::CommitFiles {
            commit_id: commit_id.clone(),
            result: backend
                .commit_files(&commit_id)
                .map_err(|error| error.message),
        },
        Command::RefreshCommitFileDiff { target } => GitResult::CommitFileDiff {
            target: target.clone(),
            result: backend
                .commit_file_diff(&target)
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
        Command::CreateBranch { name, start_point } => GitResult::CreateBranch {
            name: name.clone(),
            start_point: start_point.clone(),
            result: backend
                .create_branch(&name, &start_point)
                .map_err(|error| error.message),
        },
        Command::CheckoutBranch { name, auto_stash } => GitResult::CheckoutBranch {
            name: name.clone(),
            auto_stash,
            result: backend
                .checkout_branch(&name, auto_stash)
                .map_err(|error| error.message),
        },
        Command::DeleteBranch { name, mode, force } => GitResult::DeleteBranch {
            name: name.clone(),
            mode,
            force,
            result: backend
                .delete_branch(&name, mode, force)
                .map_err(|error| error.message),
        },
        Command::RebaseBranch {
            target,
            interactive,
            auto_stash,
        } => GitResult::RebaseBranch {
            target: target.clone(),
            interactive,
            auto_stash,
            result: backend
                .rebase_branch(&target, interactive, auto_stash)
                .map_err(|error| error.message),
        },
        Command::SquashCommits { commit_ids } => GitResult::SquashCommits {
            commit_ids: commit_ids.clone(),
            result: backend
                .squash_commits(&commit_ids)
                .map_err(|error| error.message),
        },
        Command::FixupCommits { commit_ids } => GitResult::FixupCommits {
            commit_ids: commit_ids.clone(),
            result: backend
                .fixup_commits(&commit_ids)
                .map_err(|error| error.message),
        },
        Command::RewordCommit { commit_id, message } => GitResult::RewordCommit {
            commit_id: commit_id.clone(),
            message: message.clone(),
            result: backend
                .reword_commit(&commit_id, &message)
                .map_err(|error| error.message),
        },
        Command::DeleteCommits { commit_ids } => GitResult::DeleteCommits {
            commit_ids: commit_ids.clone(),
            result: backend
                .delete_commits(&commit_ids)
                .map_err(|error| error.message),
        },
        Command::CheckoutCommitDetached {
            commit_id,
            auto_stash,
        } => GitResult::CheckoutCommitDetached {
            commit_id: commit_id.clone(),
            auto_stash,
            result: backend
                .checkout_commit_detached(&commit_id, auto_stash)
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
    use ratagit_core::{
        BranchDeleteMode, BranchEntry, COMMITS_PAGE_SIZE, CommitEntry, CommitHashStatus, FileEntry,
        ResetMode,
    };

    use super::*;

    fn test_commit(id: &str, summary: &str) -> CommitEntry {
        CommitEntry {
            id: id.to_string(),
            full_id: id.to_string(),
            summary: summary.to_string(),
            message: summary.to_string(),
            author_name: "ratagit-tests".to_string(),
            graph: "●".to_string(),
            hash_status: CommitHashStatus::Unpushed,
            is_merge: false,
        }
    }

    fn test_snapshot_with_commits(commits: Vec<CommitEntry>) -> RepoSnapshot {
        RepoSnapshot {
            status_summary: "clean".to_string(),
            current_branch: "main".to_string(),
            detached_head: false,
            files: Vec::new(),
            commits,
            branches: vec![BranchEntry {
                name: "main".to_string(),
                is_current: true,
            }],
            stashes: Vec::new(),
        }
    }

    fn test_commits(count: usize) -> Vec<CommitEntry> {
        (0..count)
            .map(|index| test_commit(&format!("{index:07x}"), &format!("commit {index}")))
            .collect()
    }

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
            .create_branch("feature/mvp", "main")
            .expect("create branch should work");
        backend
            .checkout_branch("feature/mvp", false)
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
    fn mock_branch_operations_cover_start_point_delete_rebase_and_auto_stash() {
        let mut backend = MockGitBackend::new(RepoSnapshot {
            status_summary: "dirty".to_string(),
            current_branch: "main".to_string(),
            detached_head: false,
            files: Vec::new(),
            commits: Vec::new(),
            branches: vec![
                BranchEntry {
                    name: "main".to_string(),
                    is_current: true,
                },
                BranchEntry {
                    name: "feature/base".to_string(),
                    is_current: false,
                },
            ],
            stashes: Vec::new(),
        });

        backend
            .create_branch("feature/new", "feature/base")
            .expect("create branch from start point should work");
        backend
            .checkout_branch("feature/new", true)
            .expect("auto-stash checkout should work");
        backend
            .rebase_branch("origin/main", false, true)
            .expect("auto-stash rebase should work");
        backend
            .delete_branch("feature/base", BranchDeleteMode::Both, false)
            .expect("delete both should work");

        assert_eq!(
            backend.operations(),
            &[
                "create-branch:feature/new:feature/base".to_string(),
                "auto-stash-push".to_string(),
                "checkout-branch:feature/new".to_string(),
                "auto-stash-pop".to_string(),
                "auto-stash-push".to_string(),
                "rebase:simple:origin/main".to_string(),
                "auto-stash-pop".to_string(),
                "delete-local:feature/base".to_string(),
                "delete-remote:origin/feature/base".to_string(),
            ]
        );
        assert_eq!(backend.snapshot().current_branch, "feature/new");
        assert!(
            !backend
                .snapshot()
                .branches
                .iter()
                .any(|branch| branch.name == "feature/base")
        );
    }

    #[test]
    fn mock_commit_operations_update_snapshot_and_trace() {
        let mut backend = MockGitBackend::new(RepoSnapshot {
            status_summary: "clean".to_string(),
            current_branch: "main".to_string(),
            detached_head: false,
            files: Vec::new(),
            commits: vec![
                test_commit("aaa1111", "head"),
                test_commit("bbb2222", "middle"),
                test_commit("ccc3333", "base"),
            ],
            branches: vec![BranchEntry {
                name: "main".to_string(),
                is_current: true,
            }],
            stashes: Vec::new(),
        });

        backend
            .fixup_commits(&["aaa1111".to_string()])
            .expect("fixup should work");
        assert_eq!(backend.snapshot().commits[0].summary, "middle");
        backend
            .reword_commit("bbb2222", "middle reworded")
            .expect("reword should work");
        assert_eq!(backend.snapshot().commits[0].summary, "middle reworded");
        backend
            .checkout_commit_detached("bbb2222", true)
            .expect("detached checkout should work");
        assert!(backend.snapshot().detached_head);
        assert_eq!(
            backend.operations(),
            &[
                "fixup:aaa1111".to_string(),
                "reword:bbb2222:middle reworded".to_string(),
                "auto-stash-push".to_string(),
                "checkout-detached:bbb2222".to_string(),
                "auto-stash-pop".to_string(),
            ]
        );
    }

    #[test]
    fn mock_refresh_and_commit_pages_use_page_size() {
        let mut backend = MockGitBackend::new(test_snapshot_with_commits(test_commits(
            COMMITS_PAGE_SIZE + 25,
        )));

        let snapshot = backend.refresh_snapshot().expect("refresh should work");
        assert_eq!(snapshot.commits.len(), COMMITS_PAGE_SIZE);

        let page = backend
            .load_more_commits(COMMITS_PAGE_SIZE, COMMITS_PAGE_SIZE)
            .expect("page should load");
        assert_eq!(page.len(), 25);
        assert_eq!(page[0].summary, "commit 100");
        assert_eq!(
            backend.operations(),
            &[
                "refresh".to_string(),
                format!("commits-page:{}:{}", COMMITS_PAGE_SIZE, COMMITS_PAGE_SIZE),
            ]
        );
    }

    #[test]
    fn mock_commit_rewrites_reject_public_merge_and_root_parent_cases() {
        let mut public = test_commit("aaa1111", "public");
        public.hash_status = CommitHashStatus::Pushed;
        let mut backend = MockGitBackend::new(test_snapshot_with_commits(vec![
            public,
            test_commit("bbb2222", "base"),
            test_commit("ccc3333", "root"),
        ]));

        let error = backend
            .delete_commits(&["aaa1111".to_string()])
            .expect_err("public commits should not be rewritten");
        assert!(error.message.contains("not private"));

        let mut merge = test_commit("ddd4444", "merge");
        merge.is_merge = true;
        let mut backend = MockGitBackend::new(test_snapshot_with_commits(vec![
            merge,
            test_commit("eee5555", "base"),
            test_commit("fff6666", "root"),
        ]));

        let error = backend
            .reword_commit("ddd4444", "merge reworded")
            .expect_err("merge commits should not be rewritten");
        assert!(error.message.contains("merge commits"));

        let mut backend = MockGitBackend::new(test_snapshot_with_commits(vec![
            test_commit("ggg7777", "head"),
            test_commit("hhh8888", "root"),
        ]));

        let error = backend
            .fixup_commits(&["ggg7777".to_string()])
            .expect_err("fixup into root should be rejected");
        assert!(error.message.contains("cannot squash or fixup into root"));
    }

    #[test]
    fn mock_delete_current_branch_reports_error() {
        let mut backend = MockGitBackend::new(RepoSnapshot {
            status_summary: "clean".to_string(),
            current_branch: "main".to_string(),
            detached_head: false,
            files: Vec::new(),
            commits: Vec::new(),
            branches: vec![BranchEntry {
                name: "main".to_string(),
                is_current: true,
            }],
            stashes: Vec::new(),
        });

        let error = backend
            .delete_branch("main", BranchDeleteMode::Local, false)
            .expect_err("current branch delete should fail");

        assert!(error.message.contains("cannot delete current branch"));
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
            commits: vec![test_commit("mock-0001", "initial")],
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
    fn execute_command_refresh_branch_details_log_uses_backend_output() {
        let mut backend = MockGitBackend::new(RepoSnapshot {
            status_summary: "clean".to_string(),
            current_branch: "main".to_string(),
            detached_head: false,
            files: Vec::new(),
            commits: vec![test_commit("abc1234", "initial")],
            branches: vec![BranchEntry {
                name: "main".to_string(),
                is_current: true,
            }],
            stashes: Vec::new(),
        });

        let result = execute_command(
            &mut backend,
            Command::RefreshBranchDetailsLog {
                branch: "main".to_string(),
                max_count: 50,
            },
        );

        match result {
            GitResult::BranchDetailsLog { branch, result } => {
                assert_eq!(branch, "main");
                let graph = result.expect("mock graph should succeed");
                assert!(graph.contains("\u{1b}[33m*"));
                assert!(graph.contains("commit abc1234"));
                assert_eq!(backend.operations(), &["branch-log:main:50".to_string()]);
            }
            other => panic!("unexpected git result: {other:?}"),
        }
    }

    #[test]
    fn execute_command_refresh_commit_details_diff_uses_backend_output() {
        let mut backend = MockGitBackend::new(RepoSnapshot {
            status_summary: "clean".to_string(),
            current_branch: "main".to_string(),
            detached_head: false,
            files: Vec::new(),
            commits: vec![test_commit("abc1234", "initial")],
            branches: vec![BranchEntry {
                name: "main".to_string(),
                is_current: true,
            }],
            stashes: Vec::new(),
        });

        let result = execute_command(
            &mut backend,
            Command::RefreshCommitDetailsDiff {
                commit_id: "abc1234".to_string(),
            },
        );

        match result {
            GitResult::CommitDetailsDiff { commit_id, result } => {
                assert_eq!(commit_id, "abc1234");
                let diff = result.expect("mock commit diff should succeed");
                assert!(diff.contains("commit abc1234"));
                assert!(diff.contains("diff --git a/commit.txt b/commit.txt"));
                assert_eq!(backend.operations(), &["commit-diff:abc1234".to_string()]);
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
