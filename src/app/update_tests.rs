use super::*;
use crate::app::{App, RefreshKind, SidePanel};
use crate::flux::action::DomainAction;
use crate::flux::effects::EffectRequest;
use crate::flux::test_runtime::run_inline_effect;
use crate::git::{
    BranchInfo, CommitInfo, CommitSyncState, DiffLine, DiffLineKind, FileEntry, FileStatus,
    GitError, GitRepository, GitStatus, StashInfo,
};
use crate::ui::widgets::file_tree::{FileTreeNode, FileTreeNodeStatus};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use pretty_assertions::assert_eq;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

struct MockRepo;

impl GitRepository for MockRepo {
    fn status(&self) -> Result<GitStatus, GitError> {
        Ok(GitStatus::default())
    }

    fn stage(&self, _path: &Path) -> Result<(), GitError> {
        Ok(())
    }

    fn stage_paths(&self, _paths: &[PathBuf]) -> Result<(), GitError> {
        Ok(())
    }

    fn unstage(&self, _path: &Path) -> Result<(), GitError> {
        Ok(())
    }

    fn unstage_paths(&self, _paths: &[PathBuf]) -> Result<(), GitError> {
        Ok(())
    }

    fn discard_paths(&self, _paths: &[PathBuf]) -> Result<(), GitError> {
        Ok(())
    }

    fn diff_unstaged(&self, _path: &Path) -> Result<Vec<DiffLine>, GitError> {
        Ok(vec![])
    }

    fn diff_staged(&self, _path: &Path) -> Result<Vec<DiffLine>, GitError> {
        Ok(vec![])
    }

    fn diff_untracked(&self, _path: &Path) -> Result<Vec<DiffLine>, GitError> {
        Ok(vec![])
    }

    fn branches(&self) -> Result<Vec<BranchInfo>, GitError> {
        Ok(vec![])
    }

    fn commits(&self, _limit: usize) -> Result<Vec<CommitInfo>, GitError> {
        Ok(vec![
            CommitInfo {
                short_hash: "abc1234".to_string(),
                oid: "abc1234567890".to_string(),
                message: "test commit".to_string(),
                author: "tester".to_string(),
                graph: vec![crate::git::GraphCell {
                    text: "●".to_string(),
                    lane: 0,
                    pipe_oid: None,
                    pipe_oids: vec![],
                }],
                time: "2026-03-20 00:00".to_string(),
                parent_count: 1,
                sync_state: CommitSyncState::DefaultBranch,
                parent_oids: vec!["def5678901234".to_string()],
            },
            CommitInfo {
                short_hash: "def5678".to_string(),
                oid: "def5678901234".to_string(),
                message: "test commit 2".to_string(),
                author: "tester".to_string(),
                graph: vec![crate::git::GraphCell {
                    text: "●".to_string(),
                    lane: 0,
                    pipe_oid: None,
                    pipe_oids: vec![],
                }],
                time: "2026-03-20 00:01".to_string(),
                parent_count: 1,
                sync_state: CommitSyncState::DefaultBranch,
                parent_oids: vec![],
            },
        ])
    }

    fn commit_files(&self, _oid: &str) -> Result<Vec<FileEntry>, GitError> {
        Ok(vec![FileEntry {
            path: PathBuf::from("src/main.rs"),
            status: FileStatus::Modified,
        }])
    }

    fn stashes(&self) -> Result<Vec<StashInfo>, GitError> {
        Ok(vec![
            StashInfo {
                index: 0,
                message: "stash test".to_string(),
            },
            StashInfo {
                index: 1,
                message: "stash test 2".to_string(),
            },
        ])
    }

    fn stash_files(&self, _index: usize) -> Result<Vec<FileEntry>, GitError> {
        Ok(vec![FileEntry {
            path: PathBuf::from("src/lib.rs"),
            status: FileStatus::Modified,
        }])
    }

    fn stash_diff(&self, _index: usize, path: Option<&Path>) -> Result<Vec<DiffLine>, GitError> {
        let scoped = path
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "<none>".to_string());
        Ok(vec![DiffLine {
            kind: DiffLineKind::Header,
            content: format!("stash diff {}", scoped),
        }])
    }

    fn stash_push_paths(&self, _paths: &[PathBuf], _message: &str) -> Result<usize, GitError> {
        Ok(0)
    }

    fn stash_apply(&self, _index: usize) -> Result<(), GitError> {
        Ok(())
    }

    fn stash_pop(&self, _index: usize) -> Result<(), GitError> {
        Ok(())
    }

    fn stash_drop(&self, _index: usize) -> Result<(), GitError> {
        Ok(())
    }

    fn commit_diff_scoped(
        &self,
        _oid: &str,
        path: Option<&Path>,
    ) -> Result<Vec<DiffLine>, GitError> {
        let scoped = path
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "<none>".to_string());
        Ok(vec![DiffLine {
            kind: DiffLineKind::Header,
            content: format!("commit diff {}", scoped),
        }])
    }

    fn commit(&self, _message: &str) -> Result<String, GitError> {
        Ok("oid".to_string())
    }

    fn create_branch(&self, _name: &str) -> Result<(), GitError> {
        Ok(())
    }

    fn checkout_branch(&self, _name: &str) -> Result<(), GitError> {
        Ok(())
    }

    fn delete_branch(&self, _name: &str) -> Result<(), GitError> {
        Ok(())
    }

    fn fetch_default_async(&self) -> Result<Receiver<Result<String, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(Ok("origin".to_string()));
        Ok(rx)
    }
}

fn mock_app() -> App {
    App::from_repo(Box::new(MockRepo)).expect("app from mock repo")
}

fn first_diff_content(app: &App) -> &str {
    app.current_diff
        .first()
        .map(|line| line.content.as_str())
        .expect("diff should contain at least one line")
}

fn assert_commits_selected(app: &App, expected: Option<usize>) {
    assert_eq!(app.commits.panel.list_state.selected(), expected);
}

fn assert_files_selected(app: &App, expected: Option<usize>) {
    assert_eq!(app.files.panel.list_state.selected(), expected);
}

fn assert_branches_selected(app: &App, expected: Option<usize>) {
    assert_eq!(app.branches.panel.list_state.selected(), expected);
}

fn assert_stash_selected(app: &App, expected: Option<usize>) {
    assert_eq!(app.stash.panel.list_state.selected(), expected);
}

struct CountingRepo {
    commits_calls: Arc<AtomicUsize>,
}

impl CountingRepo {
    fn new(commits_calls: Arc<AtomicUsize>) -> Self {
        Self { commits_calls }
    }
}

impl GitRepository for CountingRepo {
    fn status(&self) -> Result<GitStatus, GitError> {
        Ok(GitStatus::default())
    }

    fn stage(&self, _path: &Path) -> Result<(), GitError> {
        Ok(())
    }

    fn stage_paths(&self, _paths: &[PathBuf]) -> Result<(), GitError> {
        Ok(())
    }

    fn unstage(&self, _path: &Path) -> Result<(), GitError> {
        Ok(())
    }

    fn unstage_paths(&self, _paths: &[PathBuf]) -> Result<(), GitError> {
        Ok(())
    }

    fn discard_paths(&self, _paths: &[PathBuf]) -> Result<(), GitError> {
        Ok(())
    }

    fn diff_unstaged(&self, _path: &Path) -> Result<Vec<DiffLine>, GitError> {
        Ok(vec![])
    }

    fn diff_staged(&self, _path: &Path) -> Result<Vec<DiffLine>, GitError> {
        Ok(vec![])
    }

    fn diff_untracked(&self, _path: &Path) -> Result<Vec<DiffLine>, GitError> {
        Ok(vec![])
    }

    fn branches(&self) -> Result<Vec<BranchInfo>, GitError> {
        Ok(vec![])
    }

    fn commits(&self, _limit: usize) -> Result<Vec<CommitInfo>, GitError> {
        let calls = self.commits_calls.fetch_add(1, Ordering::SeqCst) + 1;
        Ok(vec![CommitInfo {
            short_hash: format!("c{}", calls),
            oid: format!("oid{}", calls),
            message: format!("commit {}", calls),
            author: "tester".to_string(),
            graph: vec![crate::git::GraphCell {
                text: "●".to_string(),
                lane: 0,
                pipe_oid: None,
                pipe_oids: vec![],
            }],
            time: "2026-03-20 00:00".to_string(),
            parent_count: 1,
            sync_state: CommitSyncState::DefaultBranch,
            parent_oids: vec![],
        }])
    }

    fn commit_files(&self, _oid: &str) -> Result<Vec<FileEntry>, GitError> {
        Ok(vec![])
    }

    fn stashes(&self) -> Result<Vec<StashInfo>, GitError> {
        Ok(vec![])
    }

    fn stash_files(&self, _index: usize) -> Result<Vec<FileEntry>, GitError> {
        Ok(vec![])
    }

    fn stash_diff(&self, _index: usize, _path: Option<&Path>) -> Result<Vec<DiffLine>, GitError> {
        Ok(vec![])
    }

    fn stash_push_paths(&self, _paths: &[PathBuf], _message: &str) -> Result<usize, GitError> {
        Ok(0)
    }

    fn stash_apply(&self, _index: usize) -> Result<(), GitError> {
        Ok(())
    }

    fn stash_pop(&self, _index: usize) -> Result<(), GitError> {
        Ok(())
    }

    fn stash_drop(&self, _index: usize) -> Result<(), GitError> {
        Ok(())
    }

    fn commit_diff_scoped(
        &self,
        _oid: &str,
        _path: Option<&Path>,
    ) -> Result<Vec<DiffLine>, GitError> {
        Ok(vec![])
    }

    fn commit(&self, _message: &str) -> Result<String, GitError> {
        Ok("oid".to_string())
    }

    fn create_branch(&self, _name: &str) -> Result<(), GitError> {
        Ok(())
    }

    fn checkout_branch(&self, _name: &str) -> Result<(), GitError> {
        Ok(())
    }

    fn delete_branch(&self, _name: &str) -> Result<(), GitError> {
        Ok(())
    }

    fn fetch_default_async(&self) -> Result<Receiver<Result<String, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(Ok("origin".to_string()));
        Ok(rx)
    }
}

struct RefreshCountingRepo {
    status_calls: Arc<AtomicUsize>,
    branches_calls: Arc<AtomicUsize>,
    stashes_calls: Arc<AtomicUsize>,
    commits_calls: Arc<AtomicUsize>,
}

struct DuplicateStatusRepo;

impl GitRepository for DuplicateStatusRepo {
    fn status(&self) -> Result<GitStatus, GitError> {
        Ok(GitStatus {
            unstaged: vec![FileEntry {
                path: PathBuf::from("src/main.rs"),
                status: FileStatus::Modified,
            }],
            untracked: vec![],
            staged: vec![FileEntry {
                path: PathBuf::from("src/main.rs"),
                status: FileStatus::Modified,
            }],
        })
    }

    fn stage(&self, _path: &Path) -> Result<(), GitError> {
        Ok(())
    }

    fn stage_paths(&self, _paths: &[PathBuf]) -> Result<(), GitError> {
        Ok(())
    }

    fn unstage(&self, _path: &Path) -> Result<(), GitError> {
        Ok(())
    }

    fn unstage_paths(&self, _paths: &[PathBuf]) -> Result<(), GitError> {
        Ok(())
    }

    fn discard_paths(&self, _paths: &[PathBuf]) -> Result<(), GitError> {
        Ok(())
    }

    fn diff_unstaged(&self, _path: &Path) -> Result<Vec<DiffLine>, GitError> {
        Ok(vec![DiffLine {
            kind: DiffLineKind::Header,
            content: "unstaged".to_string(),
        }])
    }

    fn diff_staged(&self, _path: &Path) -> Result<Vec<DiffLine>, GitError> {
        Ok(vec![DiffLine {
            kind: DiffLineKind::Header,
            content: "staged".to_string(),
        }])
    }

    fn diff_untracked(&self, _path: &Path) -> Result<Vec<DiffLine>, GitError> {
        Ok(vec![DiffLine {
            kind: DiffLineKind::Header,
            content: "untracked".to_string(),
        }])
    }

    fn branches(&self) -> Result<Vec<BranchInfo>, GitError> {
        Ok(vec![])
    }

    fn commits(&self, _limit: usize) -> Result<Vec<CommitInfo>, GitError> {
        Ok(vec![])
    }

    fn commit_files(&self, _oid: &str) -> Result<Vec<FileEntry>, GitError> {
        Ok(vec![])
    }

    fn stashes(&self) -> Result<Vec<StashInfo>, GitError> {
        Ok(vec![])
    }

    fn stash_files(&self, _index: usize) -> Result<Vec<FileEntry>, GitError> {
        Ok(vec![])
    }

    fn stash_diff(&self, _index: usize, _path: Option<&Path>) -> Result<Vec<DiffLine>, GitError> {
        Ok(vec![])
    }

    fn stash_push_paths(&self, _paths: &[PathBuf], _message: &str) -> Result<usize, GitError> {
        Ok(0)
    }

    fn stash_apply(&self, _index: usize) -> Result<(), GitError> {
        Ok(())
    }

    fn stash_pop(&self, _index: usize) -> Result<(), GitError> {
        Ok(())
    }

    fn stash_drop(&self, _index: usize) -> Result<(), GitError> {
        Ok(())
    }

    fn commit_diff_scoped(
        &self,
        _oid: &str,
        _path: Option<&Path>,
    ) -> Result<Vec<DiffLine>, GitError> {
        Ok(vec![])
    }

    fn commit(&self, _message: &str) -> Result<String, GitError> {
        Ok("oid".to_string())
    }

    fn create_branch(&self, _name: &str) -> Result<(), GitError> {
        Ok(())
    }

    fn checkout_branch(&self, _name: &str) -> Result<(), GitError> {
        Ok(())
    }

    fn delete_branch(&self, _name: &str) -> Result<(), GitError> {
        Ok(())
    }

    fn fetch_default_async(&self) -> Result<Receiver<Result<String, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(Ok("origin".to_string()));
        Ok(rx)
    }
}

struct NavigationDiffRepo;

impl GitRepository for NavigationDiffRepo {
    fn status(&self) -> Result<GitStatus, GitError> {
        Ok(GitStatus {
            unstaged: vec![
                FileEntry {
                    path: PathBuf::from("a.txt"),
                    status: FileStatus::Modified,
                },
                FileEntry {
                    path: PathBuf::from("b.txt"),
                    status: FileStatus::Modified,
                },
            ],
            untracked: vec![],
            staged: vec![],
        })
    }

    fn stage(&self, _path: &Path) -> Result<(), GitError> {
        Ok(())
    }

    fn stage_paths(&self, _paths: &[PathBuf]) -> Result<(), GitError> {
        Ok(())
    }

    fn unstage(&self, _path: &Path) -> Result<(), GitError> {
        Ok(())
    }

    fn unstage_paths(&self, _paths: &[PathBuf]) -> Result<(), GitError> {
        Ok(())
    }

    fn discard_paths(&self, _paths: &[PathBuf]) -> Result<(), GitError> {
        Ok(())
    }

    fn diff_unstaged(&self, path: &Path) -> Result<Vec<DiffLine>, GitError> {
        Ok(vec![DiffLine {
            kind: DiffLineKind::Header,
            content: format!("file {}", path.display()),
        }])
    }

    fn diff_staged(&self, _path: &Path) -> Result<Vec<DiffLine>, GitError> {
        Ok(vec![])
    }

    fn diff_untracked(&self, _path: &Path) -> Result<Vec<DiffLine>, GitError> {
        Ok(vec![])
    }

    fn branches(&self) -> Result<Vec<BranchInfo>, GitError> {
        Ok(vec![
            BranchInfo {
                name: "main".to_string(),
                is_current: true,
            },
            BranchInfo {
                name: "feature/x".to_string(),
                is_current: false,
            },
        ])
    }

    fn branch_log(&self, name: &str, _limit: usize) -> Result<Vec<DiffLine>, GitError> {
        Ok(vec![DiffLine {
            kind: DiffLineKind::Context,
            content: format!("branch {}", name),
        }])
    }

    fn commits(&self, _limit: usize) -> Result<Vec<CommitInfo>, GitError> {
        Ok(vec![
            CommitInfo {
                short_hash: "1111111".to_string(),
                oid: "oid1".to_string(),
                message: "commit one".to_string(),
                author: "tester".to_string(),
                graph: vec![crate::git::GraphCell {
                    text: "●".to_string(),
                    lane: 0,
                    pipe_oid: None,
                    pipe_oids: vec![],
                }],
                time: "2026-03-20 00:00".to_string(),
                parent_count: 1,
                sync_state: CommitSyncState::DefaultBranch,
                parent_oids: vec!["oid2".to_string()],
            },
            CommitInfo {
                short_hash: "2222222".to_string(),
                oid: "oid2".to_string(),
                message: "commit two".to_string(),
                author: "tester".to_string(),
                graph: vec![crate::git::GraphCell {
                    text: "●".to_string(),
                    lane: 0,
                    pipe_oid: None,
                    pipe_oids: vec![],
                }],
                time: "2026-03-20 00:01".to_string(),
                parent_count: 1,
                sync_state: CommitSyncState::DefaultBranch,
                parent_oids: vec![],
            },
        ])
    }

    fn commit_files(&self, _oid: &str) -> Result<Vec<FileEntry>, GitError> {
        Ok(vec![])
    }

    fn stashes(&self) -> Result<Vec<StashInfo>, GitError> {
        Ok(vec![])
    }

    fn stash_files(&self, _index: usize) -> Result<Vec<FileEntry>, GitError> {
        Ok(vec![])
    }

    fn stash_diff(&self, _index: usize, _path: Option<&Path>) -> Result<Vec<DiffLine>, GitError> {
        Ok(vec![])
    }

    fn stash_push_paths(&self, _paths: &[PathBuf], _message: &str) -> Result<usize, GitError> {
        Ok(0)
    }

    fn stash_apply(&self, _index: usize) -> Result<(), GitError> {
        Ok(())
    }

    fn stash_pop(&self, _index: usize) -> Result<(), GitError> {
        Ok(())
    }

    fn stash_drop(&self, _index: usize) -> Result<(), GitError> {
        Ok(())
    }

    fn commit_diff_scoped(
        &self,
        oid: &str,
        _path: Option<&Path>,
    ) -> Result<Vec<DiffLine>, GitError> {
        Ok(vec![DiffLine {
            kind: DiffLineKind::Header,
            content: format!("commit {}", oid),
        }])
    }

    fn commit(&self, _message: &str) -> Result<String, GitError> {
        Ok("oid".to_string())
    }

    fn create_branch(&self, _name: &str) -> Result<(), GitError> {
        Ok(())
    }

    fn checkout_branch(&self, _name: &str) -> Result<(), GitError> {
        Ok(())
    }

    fn delete_branch(&self, _name: &str) -> Result<(), GitError> {
        Ok(())
    }

    fn fetch_default_async(&self) -> Result<Receiver<Result<String, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(Ok("origin".to_string()));
        Ok(rx)
    }
}

struct BranchSwitchRepo {
    checkout_calls: Arc<AtomicUsize>,
    auto_stash_calls: Arc<AtomicUsize>,
}

impl BranchSwitchRepo {
    fn new(checkout_calls: Arc<AtomicUsize>, auto_stash_calls: Arc<AtomicUsize>) -> Self {
        Self {
            checkout_calls,
            auto_stash_calls,
        }
    }
}

impl GitRepository for BranchSwitchRepo {
    fn status(&self) -> Result<GitStatus, GitError> {
        Ok(GitStatus {
            unstaged: vec![FileEntry {
                path: PathBuf::from("dirty.txt"),
                status: FileStatus::Modified,
            }],
            untracked: vec![],
            staged: vec![],
        })
    }

    fn stage(&self, _path: &Path) -> Result<(), GitError> {
        Ok(())
    }

    fn stage_paths(&self, _paths: &[PathBuf]) -> Result<(), GitError> {
        Ok(())
    }

    fn unstage(&self, _path: &Path) -> Result<(), GitError> {
        Ok(())
    }

    fn unstage_paths(&self, _paths: &[PathBuf]) -> Result<(), GitError> {
        Ok(())
    }

    fn discard_paths(&self, _paths: &[PathBuf]) -> Result<(), GitError> {
        Ok(())
    }

    fn diff_unstaged(&self, _path: &Path) -> Result<Vec<DiffLine>, GitError> {
        Ok(vec![])
    }

    fn diff_staged(&self, _path: &Path) -> Result<Vec<DiffLine>, GitError> {
        Ok(vec![])
    }

    fn diff_untracked(&self, _path: &Path) -> Result<Vec<DiffLine>, GitError> {
        Ok(vec![])
    }

    fn branches(&self) -> Result<Vec<BranchInfo>, GitError> {
        Ok(vec![
            BranchInfo {
                name: "main".to_string(),
                is_current: true,
            },
            BranchInfo {
                name: "feature/switch".to_string(),
                is_current: false,
            },
        ])
    }

    fn commits(&self, _limit: usize) -> Result<Vec<CommitInfo>, GitError> {
        Ok(vec![])
    }

    fn commit_files(&self, _oid: &str) -> Result<Vec<FileEntry>, GitError> {
        Ok(vec![])
    }

    fn stashes(&self) -> Result<Vec<StashInfo>, GitError> {
        Ok(vec![])
    }

    fn stash_files(&self, _index: usize) -> Result<Vec<FileEntry>, GitError> {
        Ok(vec![])
    }

    fn stash_diff(&self, _index: usize, _path: Option<&Path>) -> Result<Vec<DiffLine>, GitError> {
        Ok(vec![])
    }

    fn stash_push_paths(&self, _paths: &[PathBuf], _message: &str) -> Result<usize, GitError> {
        Ok(0)
    }

    fn stash_apply(&self, _index: usize) -> Result<(), GitError> {
        Ok(())
    }

    fn stash_pop(&self, _index: usize) -> Result<(), GitError> {
        Ok(())
    }

    fn stash_drop(&self, _index: usize) -> Result<(), GitError> {
        Ok(())
    }

    fn commit_diff_scoped(
        &self,
        _oid: &str,
        _path: Option<&Path>,
    ) -> Result<Vec<DiffLine>, GitError> {
        Ok(vec![])
    }

    fn commit(&self, _message: &str) -> Result<String, GitError> {
        Ok("oid".to_string())
    }

    fn create_branch(&self, _name: &str) -> Result<(), GitError> {
        Ok(())
    }

    fn checkout_branch(&self, _name: &str) -> Result<(), GitError> {
        self.checkout_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn checkout_branch_with_auto_stash(&self, _name: &str) -> Result<(), GitError> {
        self.auto_stash_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn delete_branch(&self, _name: &str) -> Result<(), GitError> {
        Ok(())
    }

    fn fetch_default_async(&self) -> Result<Receiver<Result<String, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(Ok("origin".to_string()));
        Ok(rx)
    }
}

impl RefreshCountingRepo {
    fn new(
        status_calls: Arc<AtomicUsize>,
        branches_calls: Arc<AtomicUsize>,
        stashes_calls: Arc<AtomicUsize>,
        commits_calls: Arc<AtomicUsize>,
    ) -> Self {
        Self {
            status_calls,
            branches_calls,
            stashes_calls,
            commits_calls,
        }
    }
}

impl GitRepository for RefreshCountingRepo {
    fn status(&self) -> Result<GitStatus, GitError> {
        self.status_calls.fetch_add(1, Ordering::SeqCst);
        Ok(GitStatus::default())
    }

    fn stage(&self, _path: &Path) -> Result<(), GitError> {
        Ok(())
    }

    fn stage_paths(&self, _paths: &[PathBuf]) -> Result<(), GitError> {
        Ok(())
    }

    fn unstage(&self, _path: &Path) -> Result<(), GitError> {
        Ok(())
    }

    fn unstage_paths(&self, _paths: &[PathBuf]) -> Result<(), GitError> {
        Ok(())
    }

    fn discard_paths(&self, _paths: &[PathBuf]) -> Result<(), GitError> {
        Ok(())
    }

    fn diff_unstaged(&self, _path: &Path) -> Result<Vec<DiffLine>, GitError> {
        Ok(vec![])
    }

    fn diff_staged(&self, _path: &Path) -> Result<Vec<DiffLine>, GitError> {
        Ok(vec![])
    }

    fn diff_untracked(&self, _path: &Path) -> Result<Vec<DiffLine>, GitError> {
        Ok(vec![])
    }

    fn branches(&self) -> Result<Vec<BranchInfo>, GitError> {
        self.branches_calls.fetch_add(1, Ordering::SeqCst);
        Ok(vec![])
    }

    fn commits(&self, _limit: usize) -> Result<Vec<CommitInfo>, GitError> {
        self.commits_calls.fetch_add(1, Ordering::SeqCst);
        Ok(vec![])
    }

    fn commit_files(&self, _oid: &str) -> Result<Vec<FileEntry>, GitError> {
        Ok(vec![])
    }

    fn stashes(&self) -> Result<Vec<StashInfo>, GitError> {
        self.stashes_calls.fetch_add(1, Ordering::SeqCst);
        Ok(vec![])
    }

    fn stash_files(&self, _index: usize) -> Result<Vec<FileEntry>, GitError> {
        Ok(vec![])
    }

    fn stash_diff(&self, _index: usize, _path: Option<&Path>) -> Result<Vec<DiffLine>, GitError> {
        Ok(vec![])
    }

    fn stash_push_paths(&self, _paths: &[PathBuf], _message: &str) -> Result<usize, GitError> {
        Ok(0)
    }

    fn stash_apply(&self, _index: usize) -> Result<(), GitError> {
        Ok(())
    }

    fn stash_pop(&self, _index: usize) -> Result<(), GitError> {
        Ok(())
    }

    fn stash_drop(&self, _index: usize) -> Result<(), GitError> {
        Ok(())
    }

    fn commit_diff_scoped(
        &self,
        _oid: &str,
        _path: Option<&Path>,
    ) -> Result<Vec<DiffLine>, GitError> {
        Ok(vec![])
    }

    fn commit(&self, _message: &str) -> Result<String, GitError> {
        Ok("oid".to_string())
    }

    fn create_branch(&self, _name: &str) -> Result<(), GitError> {
        Ok(())
    }

    fn checkout_branch(&self, _name: &str) -> Result<(), GitError> {
        Ok(())
    }

    fn delete_branch(&self, _name: &str) -> Result<(), GitError> {
        Ok(())
    }

    fn fetch_default_async(&self) -> Result<Receiver<Result<String, GitError>>, GitError> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(Ok("origin".to_string()));
        Ok(rx)
    }
}

#[test]
fn revision_tree_toggle_in_commits_panel_opens_then_closes() {
    let mut app = mock_app();
    app.active_panel = SidePanel::Commits;
    app.commits.panel.list_state.select(Some(0));

    dispatch_test_action(&mut app, DomainAction::RevisionOpenTreeOrToggleDir);
    assert!(app.commits.tree_mode.active);
    assert_eq!(
        app.commits.tree_mode.selected_source.as_deref(),
        Some("abc1234567890")
    );
    assert!(!app.commits.tree_mode.nodes.is_empty());

    dispatch_test_action(&mut app, DomainAction::RevisionCloseTree);
    assert!(!app.commits.tree_mode.active);
    assert!(app.commits.tree_mode.selected_source.is_none());
}

#[test]
fn revision_tree_toggle_in_stash_panel_opens_then_closes() {
    let mut app = mock_app();
    app.active_panel = SidePanel::Stash;
    app.stash.panel.list_state.select(Some(0));

    dispatch_test_action(&mut app, DomainAction::RevisionOpenTreeOrToggleDir);
    assert!(app.stash.tree_mode.active);
    assert_eq!(app.stash.tree_mode.selected_source, Some(0));
    assert!(!app.stash.tree_mode.nodes.is_empty());

    dispatch_test_action(&mut app, DomainAction::RevisionCloseTree);
    assert!(!app.stash.tree_mode.active);
    assert!(app.stash.tree_mode.selected_source.is_none());
}

#[test]
fn commit_diff_in_tree_mode_scopes_to_selected_path() {
    let mut app = mock_app();
    app.active_panel = SidePanel::Commits;
    app.commits.panel.list_state.select(Some(0));

    app.reload_diff_now();
    assert!(first_diff_content(&app).contains("<none>"));

    dispatch_test_action(&mut app, DomainAction::RevisionOpenTreeOrToggleDir);
    app.reload_diff_now();
    assert!(!first_diff_content(&app).contains("<none>"));
}

#[test]
fn stash_diff_in_tree_mode_scopes_to_selected_path() {
    let mut app = mock_app();
    app.active_panel = SidePanel::Stash;
    app.stash.panel.list_state.select(Some(0));

    app.reload_diff_now();
    assert!(first_diff_content(&app).contains("<none>"));

    dispatch_test_action(&mut app, DomainAction::RevisionOpenTreeOrToggleDir);
    app.reload_diff_now();
    assert!(!first_diff_content(&app).contains("<none>"));
}

#[test]
fn commit_tree_close_restores_previous_list_selection() {
    let mut app = mock_app();
    app.active_panel = SidePanel::Commits;
    app.commits.panel.list_state.select(Some(1));

    dispatch_test_action(&mut app, DomainAction::RevisionOpenTreeOrToggleDir);
    assert!(app.commits.tree_mode.active);
    assert_eq!(
        app.commits.tree_mode.selected_source.as_deref(),
        Some("def5678901234")
    );

    dispatch_test_action(&mut app, DomainAction::RevisionCloseTree);
    assert!(!app.commits.tree_mode.active);
    assert_commits_selected(&app, Some(1));
}

#[test]
fn stash_tree_close_restores_previous_list_selection() {
    let mut app = mock_app();
    app.active_panel = SidePanel::Stash;
    app.stash.panel.list_state.select(Some(1));

    dispatch_test_action(&mut app, DomainAction::RevisionOpenTreeOrToggleDir);
    assert!(app.stash.tree_mode.active);
    assert_eq!(app.stash.tree_mode.selected_source, Some(1));

    dispatch_test_action(&mut app, DomainAction::RevisionCloseTree);
    assert!(!app.stash.tree_mode.active);
    assert_stash_selected(&app, Some(1));
}

#[test]
fn commit_search_query_supports_vim_next_prev_navigation() {
    let mut app = mock_app();
    app.active_panel = SidePanel::Commits;
    app.commits.panel.list_state.select(Some(0));

    dispatch_test_action(&mut app, DomainAction::StartSearchInput);
    dispatch_test_action(
        &mut app,
        DomainAction::SearchSetQuery("test commit".to_string()),
    );
    assert!(app.has_search_for_active_scope());

    dispatch_test_action(&mut app, DomainAction::SearchNext);
    assert_commits_selected(&app, Some(1));

    dispatch_test_action(&mut app, DomainAction::SearchPrev);
    assert_commits_selected(&app, Some(0));
}

#[test]
fn commit_search_query_matches_author_field() {
    let mut app = mock_app();
    app.active_panel = SidePanel::Commits;
    app.commits.panel.list_state.select(Some(0));
    app.commits.items[0].author = "alice".to_string();
    app.commits.items[1].author = "bob".to_string();

    dispatch_test_action(&mut app, DomainAction::StartSearchInput);
    dispatch_test_action(&mut app, DomainAction::SearchSetQuery("bob".to_string()));
    app.confirm_search_input();
    dispatch_test_action(&mut app, DomainAction::SearchConfirm);
    assert!(app.has_search_for_active_scope());

    dispatch_test_action(&mut app, DomainAction::SearchNext);
    assert_commits_selected(&app, Some(1));
}

#[test]
fn commit_search_keybindings_map_slash_n_shift_n_to_search_actions() {
    let mut app = mock_app();
    app.active_panel = SidePanel::Commits;

    let msg = map_test_key(&app, KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
    assert!(matches!(msg, Some(DomainAction::StartSearchInput)));

    dispatch_test_action(&mut app, DomainAction::StartSearchInput);
    dispatch_test_action(
        &mut app,
        DomainAction::SearchSetQuery("test commit".to_string()),
    );
    app.confirm_search_input();
    dispatch_test_action(&mut app, DomainAction::SearchConfirm);

    let next = map_test_key(&app, KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));
    assert!(matches!(next, Some(DomainAction::SearchNext)));

    let prev = map_test_key(&app, KeyEvent::new(KeyCode::Char('N'), KeyModifiers::SHIFT));
    assert!(matches!(prev, Some(DomainAction::SearchPrev)));
}

#[test]
fn files_search_matches_display_name_not_full_path() {
    let mut app = mock_app();
    app.active_panel = SidePanel::Files;
    app.files.tree_nodes = vec![FileTreeNode {
        path: PathBuf::from("src/main.rs"),
        status: FileTreeNodeStatus::Unstaged(FileStatus::Modified),
        depth: 0,
        is_dir: false,
        is_expanded: false,
    }];
    app.files.panel.list_state.select(Some(0));

    dispatch_test_action(&mut app, DomainAction::StartSearchInput);
    dispatch_test_action(&mut app, DomainAction::SearchSetQuery("src".to_string()));
    app.confirm_search_input();
    dispatch_test_action(&mut app, DomainAction::SearchConfirm);
    assert!(!app.has_search_for_active_scope());

    dispatch_test_action(&mut app, DomainAction::StartSearchInput);
    dispatch_test_action(&mut app, DomainAction::SearchSetQuery("main".to_string()));
    app.confirm_search_input();
    dispatch_test_action(&mut app, DomainAction::SearchConfirm);
    assert!(app.has_search_for_active_scope());
}

#[test]
fn files_panel_space_on_unstaged_directory_emits_stage_action() {
    let mut app = mock_app();
    app.active_panel = SidePanel::Files;
    app.files.tree_nodes = vec![
        FileTreeNode {
            path: PathBuf::from("src"),
            status: FileTreeNodeStatus::Directory,
            depth: 0,
            is_dir: true,
            is_expanded: true,
        },
        FileTreeNode {
            path: PathBuf::from("src/main.rs"),
            status: FileTreeNodeStatus::Unstaged(FileStatus::Modified),
            depth: 1,
            is_dir: false,
            is_expanded: false,
        },
    ];
    app.files.panel.list_state.select(Some(0));

    let msg = map_test_key(&app, KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
    assert!(matches!(
        msg,
        Some(DomainAction::StageFile(path)) if path == Path::new("src")
    ));
}

#[test]
fn files_panel_space_on_staged_directory_emits_unstage_action() {
    let mut app = mock_app();
    app.active_panel = SidePanel::Files;
    app.files.tree_nodes = vec![
        FileTreeNode {
            path: PathBuf::from("src"),
            status: FileTreeNodeStatus::Directory,
            depth: 0,
            is_dir: true,
            is_expanded: true,
        },
        FileTreeNode {
            path: PathBuf::from("src/main.rs"),
            status: FileTreeNodeStatus::Staged(FileStatus::Modified),
            depth: 1,
            is_dir: false,
            is_expanded: false,
        },
    ];
    app.files.panel.list_state.select(Some(0));

    let msg = map_test_key(&app, KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
    assert!(matches!(
        msg,
        Some(DomainAction::UnstageFile(path)) if path == Path::new("src")
    ));
}

#[test]
fn files_panel_discard_key_on_selected_file_emits_discard_paths() {
    let mut app = mock_app();
    app.active_panel = SidePanel::Files;
    app.files.tree_nodes = vec![FileTreeNode {
        path: PathBuf::from("src/main.rs"),
        status: FileTreeNodeStatus::Unstaged(FileStatus::Modified),
        depth: 0,
        is_dir: false,
        is_expanded: false,
    }];
    app.files.panel.list_state.select(Some(0));

    let msg = map_test_key(&app, KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
    assert!(matches!(
        msg,
        Some(DomainAction::DiscardPaths(paths))
            if paths.len() == 1 && paths[0] == Path::new("src/main.rs")
    ));
}

#[test]
fn files_panel_discard_key_in_visual_mode_emits_discard_selection() {
    let mut app = mock_app();
    app.active_panel = SidePanel::Files;
    app.files.visual_mode = true;
    app.files.tree_nodes = vec![FileTreeNode {
        path: PathBuf::from("src/main.rs"),
        status: FileTreeNodeStatus::Unstaged(FileStatus::Modified),
        depth: 0,
        is_dir: false,
        is_expanded: false,
    }];
    app.files.panel.list_state.select(Some(0));
    app.files.visual_anchor = Some(0);

    let msg = map_test_key(&app, KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
    assert!(matches!(msg, Some(DomainAction::DiscardSelection)));
}

#[test]
fn search_input_escape_clears_query_and_highlight_state() {
    let mut app = mock_app();
    app.active_panel = SidePanel::Commits;

    dispatch_test_action(&mut app, DomainAction::StartSearchInput);
    dispatch_test_action(&mut app, DomainAction::SearchSetQuery("test".to_string()));
    assert_eq!(app.search_query, "test");

    let _ = dispatch_test_key(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert!(app.search_query.is_empty());
    assert!(!app.has_search_query_for_active_scope());
}

#[test]
fn commits_tree_escape_clears_search_before_closing_tree() {
    let mut app = mock_app();
    app.active_panel = SidePanel::Commits;
    app.commits.panel.list_state.select(Some(0));

    dispatch_test_action(&mut app, DomainAction::RevisionOpenTreeOrToggleDir);
    assert!(app.commits.tree_mode.active);

    dispatch_test_action(&mut app, DomainAction::StartSearchInput);
    dispatch_test_action(&mut app, DomainAction::SearchSetQuery("main".to_string()));
    app.confirm_search_input();
    dispatch_test_action(&mut app, DomainAction::SearchConfirm);
    assert!(!app.search_query.is_empty());

    let first_esc = map_test_key(&app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(matches!(first_esc, Some(DomainAction::SearchClear)));
    dispatch_test_action(&mut app, first_esc.expect("search clear message"));
    assert!(app.commits.tree_mode.active);
    assert!(app.search_query.is_empty());

    let second_esc = map_test_key(&app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(matches!(second_esc, Some(DomainAction::RevisionCloseTree)));
}

#[test]
fn fetch_remote_action_emits_effect_and_clears_fetching_on_finish() {
    let mut app = mock_app();
    app.active_panel = SidePanel::LocalBranches;

    let cmd = dispatch_test_action(&mut app, DomainAction::FetchRemote);
    assert!(matches!(cmd, Some(Command::Effect(_))));
    assert!(app.branches.is_fetching_remote);

    dispatch_test_action(
        &mut app,
        DomainAction::FetchRemoteFinished(Ok("origin".to_string())),
    );
    assert!(!app.branches.is_fetching_remote);
}

#[test]
fn full_refresh_flush_loads_commits_immediately() {
    let commits_calls = Arc::new(AtomicUsize::new(0));
    let repo = CountingRepo::new(commits_calls.clone());
    let mut app = App::from_repo(Box::new(repo)).expect("app from counting repo");
    assert_eq!(commits_calls.load(Ordering::SeqCst), 1);
    assert_eq!(app.active_panel, SidePanel::Files);

    app.request_refresh(RefreshKind::Full);
    app.flush_pending_refresh().expect("flush full refresh");
    // Full refresh now always loads commits immediately (not deferred)
    assert_eq!(commits_calls.load(Ordering::SeqCst), 2);
}

#[test]
fn refresh_requests_coalesce_to_highest_priority_on_single_flush() {
    let status_calls = Arc::new(AtomicUsize::new(0));
    let branches_calls = Arc::new(AtomicUsize::new(0));
    let stashes_calls = Arc::new(AtomicUsize::new(0));
    let commits_calls = Arc::new(AtomicUsize::new(0));
    let repo = RefreshCountingRepo::new(
        status_calls.clone(),
        branches_calls.clone(),
        stashes_calls.clone(),
        commits_calls.clone(),
    );
    let mut app = App::from_repo(Box::new(repo)).expect("app from refresh counting repo");

    // from_repo initial load happens once
    assert_eq!(status_calls.load(Ordering::SeqCst), 1);
    assert_eq!(branches_calls.load(Ordering::SeqCst), 1);
    assert_eq!(stashes_calls.load(Ordering::SeqCst), 1);
    assert_eq!(commits_calls.load(Ordering::SeqCst), 1);

    app.request_refresh(RefreshKind::StatusOnly);
    app.request_refresh(RefreshKind::StatusAndRefs);
    app.flush_pending_refresh()
        .expect("flush coalesced refresh");

    // single flush: status + refs once, commits unchanged
    assert_eq!(status_calls.load(Ordering::SeqCst), 2);
    assert_eq!(branches_calls.load(Ordering::SeqCst), 2);
    assert_eq!(stashes_calls.load(Ordering::SeqCst), 2);
    assert_eq!(commits_calls.load(Ordering::SeqCst), 1);
}

#[test]
fn fetch_finished_queues_and_flushes_full_refresh() {
    let mut app = mock_app();
    app.active_panel = SidePanel::LocalBranches;
    app.branches.is_fetching_remote = true;

    dispatch_test_action(
        &mut app,
        DomainAction::FetchRemoteFinished(Ok("origin".to_string())),
    );

    // After FetchRemoteFinished, the store queues a Full refresh and immediately flushes it
    assert!(!app.branches.is_fetching_remote);
    // The flush happens inline via the test dispatcher, so pending_refresh is now None
    assert_eq!(app.pending_refresh_kind(), None);
}

#[test]
fn flush_pending_refresh_without_log_success_marks_ui_dirty() {
    let mut app = mock_app();
    app.active_panel = SidePanel::Commits;
    app.dirty.clear();
    app.request_refresh(RefreshKind::Full);

    let result = run_inline_effect(
        &mut app,
        EffectRequest::FlushPendingRefresh { log_success: false },
    );

    assert!(matches!(result, Some(actions) if actions.is_empty()));
    assert_eq!(app.pending_refresh_kind(), None);
    assert!(app.dirty.is_dirty());
}

#[test]
fn search_query_is_restored_per_panel_scope_after_panel_switch() {
    let mut app = mock_app();

    app.active_panel = SidePanel::Commits;
    dispatch_test_action(&mut app, DomainAction::StartSearchInput);
    dispatch_test_action(&mut app, DomainAction::SearchSetQuery("test".to_string()));
    app.confirm_search_input();
    dispatch_test_action(&mut app, DomainAction::SearchConfirm);
    assert_eq!(app.search_query, "test");

    dispatch_test_action(&mut app, DomainAction::PanelGoto(1));
    assert_eq!(app.active_panel, SidePanel::Files);
    assert!(app.search_query.is_empty());

    dispatch_test_action(&mut app, DomainAction::StartSearchInput);
    dispatch_test_action(&mut app, DomainAction::SearchSetQuery("main".to_string()));
    app.confirm_search_input();
    dispatch_test_action(&mut app, DomainAction::SearchConfirm);
    assert_eq!(app.search_query, "main");

    dispatch_test_action(&mut app, DomainAction::PanelGoto(3));
    assert_eq!(app.active_panel, SidePanel::Commits);
    assert_eq!(app.search_query, "test");
}

#[test]
fn search_query_is_restored_between_commit_list_and_tree_scopes() {
    let mut app = mock_app();
    app.active_panel = SidePanel::Commits;
    app.commits.panel.list_state.select(Some(0));

    dispatch_test_action(&mut app, DomainAction::StartSearchInput);
    dispatch_test_action(&mut app, DomainAction::SearchSetQuery("test".to_string()));
    app.confirm_search_input();
    dispatch_test_action(&mut app, DomainAction::SearchConfirm);
    assert_eq!(app.search_query, "test");

    dispatch_test_action(&mut app, DomainAction::RevisionOpenTreeOrToggleDir);
    assert!(app.commits.tree_mode.active);
    assert!(app.search_query.is_empty());

    dispatch_test_action(&mut app, DomainAction::StartSearchInput);
    dispatch_test_action(&mut app, DomainAction::SearchSetQuery("main".to_string()));
    app.confirm_search_input();
    dispatch_test_action(&mut app, DomainAction::SearchConfirm);
    assert_eq!(app.search_query, "main");

    dispatch_test_action(&mut app, DomainAction::RevisionCloseTree);
    assert!(!app.commits.tree_mode.active);
    assert_eq!(app.search_query, "test");
}

#[test]
fn status_refresh_preserves_duplicate_entry_selection_and_diff_sync() {
    let mut app = App::from_repo(Box::new(DuplicateStatusRepo)).expect("app from duplicate repo");
    app.active_panel = SidePanel::Files;
    assert_eq!(app.files.tree_nodes.len(), 3);

    app.files.panel.list_state.select(Some(2));
    app.reload_diff_now();
    assert_eq!(first_diff_content(&app), "staged");

    app.request_refresh(RefreshKind::StatusOnly);
    app.flush_pending_refresh().expect("flush refresh");

    assert_files_selected(&app, Some(2));
    assert_eq!(first_diff_content(&app), "staged");
}

#[test]
fn files_panel_list_navigation_keeps_diff_until_pending_reload_is_flushed() {
    let mut app = App::from_repo(Box::new(NavigationDiffRepo)).expect("app from navigation repo");
    app.active_panel = SidePanel::Files;
    app.files.panel.list_state.select(Some(0));
    app.reload_diff_now();
    assert_eq!(first_diff_content(&app), "file a.txt");

    dispatch_test_action(&mut app, DomainAction::ListDown);

    assert_files_selected(&app, Some(1));
    assert!(app.has_pending_diff_reload());
    assert_eq!(first_diff_content(&app), "file a.txt");

    app.flush_pending_diff_reload();
    assert_eq!(first_diff_content(&app), "file b.txt");
    assert!(!app.has_pending_diff_reload());
}

#[test]
fn commits_panel_list_navigation_keeps_diff_until_pending_reload_is_flushed() {
    let mut app = App::from_repo(Box::new(NavigationDiffRepo)).expect("app from navigation repo");
    app.active_panel = SidePanel::Commits;
    app.commits.panel.list_state.select(Some(0));
    app.reload_diff_now();
    assert_eq!(first_diff_content(&app), "commit oid1");

    dispatch_test_action(&mut app, DomainAction::ListDown);

    assert_commits_selected(&app, Some(1));
    assert!(app.has_pending_diff_reload());
    assert_eq!(first_diff_content(&app), "commit oid1");

    app.flush_pending_diff_reload();
    assert_eq!(first_diff_content(&app), "commit oid2");
    assert!(!app.has_pending_diff_reload());
}

#[test]
fn branches_panel_list_navigation_keeps_diff_until_pending_reload_is_flushed() {
    let mut app = App::from_repo(Box::new(NavigationDiffRepo)).expect("app from navigation repo");
    app.active_panel = SidePanel::LocalBranches;
    app.branches.panel.list_state.select(Some(0));
    app.reload_diff_now();
    assert_eq!(first_diff_content(&app), "branch main");

    dispatch_test_action(&mut app, DomainAction::ListDown);

    assert_branches_selected(&app, Some(1));
    assert!(app.has_pending_diff_reload());
    assert_eq!(first_diff_content(&app), "branch main");

    app.flush_pending_diff_reload();
    assert_eq!(first_diff_content(&app), "branch feature/x");
    assert!(!app.has_pending_diff_reload());
}

#[test]
fn branches_panel_space_key_emits_checkout_selected_branch() {
    let mut app = App::from_repo(Box::new(NavigationDiffRepo)).expect("app from navigation repo");
    app.active_panel = SidePanel::LocalBranches;
    app.branches.panel.list_state.select(Some(0));

    let msg = map_test_key(&app, KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
    assert!(matches!(msg, Some(DomainAction::CheckoutSelectedBranch)));
}

#[test]
fn checkout_selected_branch_with_dirty_changes_opens_confirmation_input() {
    let checkout_calls = Arc::new(AtomicUsize::new(0));
    let auto_stash_calls = Arc::new(AtomicUsize::new(0));
    let repo = BranchSwitchRepo::new(checkout_calls, auto_stash_calls);
    let mut app = App::from_repo(Box::new(repo)).expect("app from branch switch repo");
    app.active_panel = SidePanel::LocalBranches;
    app.branches.panel.list_state.select(Some(1));

    dispatch_test_action(&mut app, DomainAction::CheckoutSelectedBranch);

    assert_eq!(
        app.input_mode,
        Some(crate::app::InputMode::BranchSwitchConfirm)
    );
    assert_eq!(app.pending_branch_switch_target(), Some("feature/switch"));
}

#[test]
fn branch_switch_confirm_yes_emits_checkout_with_auto_stash() {
    let checkout_calls = Arc::new(AtomicUsize::new(0));
    let auto_stash_calls = Arc::new(AtomicUsize::new(0));
    let repo = BranchSwitchRepo::new(checkout_calls.clone(), auto_stash_calls.clone());
    let mut app = App::from_repo(Box::new(repo)).expect("app from branch switch repo");
    app.active_panel = SidePanel::LocalBranches;
    app.branches.panel.list_state.select(Some(1));

    dispatch_test_action(&mut app, DomainAction::CheckoutSelectedBranch);
    let cmd = dispatch_test_action(&mut app, DomainAction::BranchSwitchConfirm(true));
    assert!(matches!(
        cmd,
        Some(Command::Effect(
            crate::flux::effects::EffectRequest::CheckoutBranch {
                auto_stash: true,
                ..
            }
        ))
    ));

    dispatch_test_action(
        &mut app,
        DomainAction::CheckoutBranchFinished {
            name: "feature/switch".to_string(),
            auto_stash: true,
            result: Ok(()),
        },
    );

    assert_eq!(app.input_mode, None);
    assert!(app.pending_branch_switch_target().is_none());
    assert_eq!(checkout_calls.load(Ordering::SeqCst), 0);
    assert_eq!(auto_stash_calls.load(Ordering::SeqCst), 0);
}

#[test]
fn branch_switch_confirm_no_clears_pending_target_without_checkout() {
    let checkout_calls = Arc::new(AtomicUsize::new(0));
    let auto_stash_calls = Arc::new(AtomicUsize::new(0));
    let repo = BranchSwitchRepo::new(checkout_calls.clone(), auto_stash_calls.clone());
    let mut app = App::from_repo(Box::new(repo)).expect("app from branch switch repo");
    app.active_panel = SidePanel::LocalBranches;
    app.branches.panel.list_state.select(Some(1));

    dispatch_test_action(&mut app, DomainAction::CheckoutSelectedBranch);
    dispatch_test_action(&mut app, DomainAction::BranchSwitchConfirm(false));

    assert_eq!(app.input_mode, None);
    assert!(app.pending_branch_switch_target().is_none());
    assert_eq!(checkout_calls.load(Ordering::SeqCst), 0);
    assert_eq!(auto_stash_calls.load(Ordering::SeqCst), 0);
}

#[test]
fn command_palette_keybinding_executes_refresh_command_and_closes_palette() {
    let mut app = mock_app();

    let open = map_test_key(&app, KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE));
    assert!(matches!(open, Some(DomainAction::StartCommandPalette)));
    dispatch_test_action(&mut app, open.expect("start command palette"));
    assert_eq!(app.input_mode, Some(crate::app::InputMode::CommandPalette));

    let _ = dispatch_test_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE),
    );
    let _ = dispatch_test_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE),
    );
    let _ = dispatch_test_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE),
    );
    let _ = dispatch_test_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE),
    );
    let _ = dispatch_test_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE),
    );
    let _ = dispatch_test_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE),
    );
    let _ = dispatch_test_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE),
    );

    let _ = dispatch_test_key(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(app.input_mode, None);
}

#[test]
fn interaction_trace_replay_preserves_expected_panel_search_and_selection_state() {
    let mut app = mock_app();

    crate::app::trace::replay_actions(
        &mut app,
        &[
            DomainAction::PanelGoto(3),
            DomainAction::StartSearchInput,
            DomainAction::SearchSetQuery("test".to_string()),
            DomainAction::SearchConfirm,
            DomainAction::SearchNext,
            DomainAction::PanelGoto(1),
            DomainAction::ToggleVisualSelectMode,
            DomainAction::ListDown,
        ],
    );

    assert_eq!(app.active_panel, SidePanel::Files);
    assert!(app.files.visual_mode);
    assert_eq!(app.files.visual_anchor, Some(0));
    assert_files_selected(&app, Some(0));
}
