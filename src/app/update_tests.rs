use super::*;
use crate::app::{App, RefreshKind, SidePanel};
use crate::git::{
    BranchInfo, CommitInfo, CommitSyncState, DiffLine, DiffLineKind, FileEntry, FileStatus,
    GitError, GitRepository, GitStatus, StashInfo,
};
use crate::ui::widgets::file_tree::{FileTreeNode, FileTreeNodeStatus};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
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

fn test_app() -> App {
    App::from_repo(Box::new(MockRepo)).expect("app from mock repo")
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
fn test_revision_open_close_for_commits_panel() {
    let mut app = test_app();
    app.active_panel = SidePanel::Commits;
    app.commits.panel.list_state.select(Some(0));

    update(&mut app, Message::RevisionOpenTreeOrToggleDir);
    assert!(app.commits.tree_mode.active);
    assert_eq!(
        app.commits.tree_mode.selected_source.as_deref(),
        Some("abc1234567890")
    );
    assert!(!app.commits.tree_mode.nodes.is_empty());

    update(&mut app, Message::RevisionCloseTree);
    assert!(!app.commits.tree_mode.active);
    assert!(app.commits.tree_mode.selected_source.is_none());
}

#[test]
fn test_revision_open_close_for_stash_panel() {
    let mut app = test_app();
    app.active_panel = SidePanel::Stash;
    app.stash.panel.list_state.select(Some(0));

    update(&mut app, Message::RevisionOpenTreeOrToggleDir);
    assert!(app.stash.tree_mode.active);
    assert_eq!(app.stash.tree_mode.selected_source, Some(0));
    assert!(!app.stash.tree_mode.nodes.is_empty());

    update(&mut app, Message::RevisionCloseTree);
    assert!(!app.stash.tree_mode.active);
    assert!(app.stash.tree_mode.selected_source.is_none());
}

#[test]
fn test_commit_diff_scopes_path_in_tree_mode() {
    let mut app = test_app();
    app.active_panel = SidePanel::Commits;
    app.commits.panel.list_state.select(Some(0));

    app.reload_diff_now();
    assert!(app.current_diff[0].content.contains("<none>"));

    update(&mut app, Message::RevisionOpenTreeOrToggleDir);
    app.reload_diff_now();
    assert!(!app.current_diff[0].content.contains("<none>"));
}

#[test]
fn test_stash_diff_scopes_path_in_tree_mode() {
    let mut app = test_app();
    app.active_panel = SidePanel::Stash;
    app.stash.panel.list_state.select(Some(0));

    app.reload_diff_now();
    assert!(app.current_diff[0].content.contains("<none>"));

    update(&mut app, Message::RevisionOpenTreeOrToggleDir);
    app.reload_diff_now();
    assert!(!app.current_diff[0].content.contains("<none>"));
}

#[test]
fn test_commit_close_tree_restores_list_selection() {
    let mut app = test_app();
    app.active_panel = SidePanel::Commits;
    app.commits.panel.list_state.select(Some(1));

    update(&mut app, Message::RevisionOpenTreeOrToggleDir);
    assert!(app.commits.tree_mode.active);
    assert_eq!(
        app.commits.tree_mode.selected_source.as_deref(),
        Some("def5678901234")
    );

    update(&mut app, Message::RevisionCloseTree);
    assert!(!app.commits.tree_mode.active);
    assert_eq!(app.commits.panel.list_state.selected(), Some(1));
}

#[test]
fn test_stash_close_tree_restores_list_selection() {
    let mut app = test_app();
    app.active_panel = SidePanel::Stash;
    app.stash.panel.list_state.select(Some(1));

    update(&mut app, Message::RevisionOpenTreeOrToggleDir);
    assert!(app.stash.tree_mode.active);
    assert_eq!(app.stash.tree_mode.selected_source, Some(1));

    update(&mut app, Message::RevisionCloseTree);
    assert!(!app.stash.tree_mode.active);
    assert_eq!(app.stash.panel.list_state.selected(), Some(1));
}

#[test]
fn test_search_query_and_vim_navigation_for_commits() {
    let mut app = test_app();
    app.active_panel = SidePanel::Commits;
    app.commits.panel.list_state.select(Some(0));

    update(&mut app, Message::StartSearchInput);
    update(&mut app, Message::SearchSetQuery("test commit".to_string()));
    assert!(app.has_search_for_active_scope());

    update(&mut app, Message::SearchNext);
    assert_eq!(app.commits.panel.list_state.selected(), Some(1));

    update(&mut app, Message::SearchPrev);
    assert_eq!(app.commits.panel.list_state.selected(), Some(0));
}

#[test]
fn test_search_query_matches_commit_author() {
    let mut app = test_app();
    app.active_panel = SidePanel::Commits;
    app.commits.panel.list_state.select(Some(0));
    app.commits.items[0].author = "alice".to_string();
    app.commits.items[1].author = "bob".to_string();

    update(&mut app, Message::StartSearchInput);
    update(&mut app, Message::SearchSetQuery("bob".to_string()));
    app.confirm_search_input();
    update(&mut app, Message::SearchConfirm);
    assert!(app.has_search_for_active_scope());

    update(&mut app, Message::SearchNext);
    assert_eq!(app.commits.panel.list_state.selected(), Some(1));
}

#[test]
fn test_search_keybindings_slash_n_and_shift_n() {
    let mut app = test_app();
    app.active_panel = SidePanel::Commits;

    let msg = app.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
    assert!(matches!(msg, Some(Message::StartSearchInput)));

    update(&mut app, Message::StartSearchInput);
    update(&mut app, Message::SearchSetQuery("test commit".to_string()));
    app.confirm_search_input();
    update(&mut app, Message::SearchConfirm);

    let next = app.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));
    assert!(matches!(next, Some(Message::SearchNext)));

    let prev = app.handle_key(KeyEvent::new(KeyCode::Char('N'), KeyModifiers::SHIFT));
    assert!(matches!(prev, Some(Message::SearchPrev)));
}

#[test]
fn test_files_search_matches_tree_display_name_not_full_path() {
    let mut app = test_app();
    app.active_panel = SidePanel::Files;
    app.files.tree_nodes = vec![FileTreeNode {
        path: PathBuf::from("src/main.rs"),
        status: FileTreeNodeStatus::Unstaged(FileStatus::Modified),
        depth: 0,
        is_dir: false,
        is_expanded: false,
    }];
    app.files.panel.list_state.select(Some(0));

    update(&mut app, Message::StartSearchInput);
    update(&mut app, Message::SearchSetQuery("src".to_string()));
    app.confirm_search_input();
    update(&mut app, Message::SearchConfirm);
    assert!(!app.has_search_for_active_scope());

    update(&mut app, Message::StartSearchInput);
    update(&mut app, Message::SearchSetQuery("main".to_string()));
    app.confirm_search_input();
    update(&mut app, Message::SearchConfirm);
    assert!(app.has_search_for_active_scope());
}

#[test]
fn test_space_on_directory_stages_directory_in_files_panel() {
    let mut app = test_app();
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

    let msg = app.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
    assert!(matches!(
        msg,
        Some(Message::StageFile(path)) if path == Path::new("src")
    ));
}

#[test]
fn test_space_on_staged_directory_unstages_directory_in_files_panel() {
    let mut app = test_app();
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

    let msg = app.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
    assert!(matches!(
        msg,
        Some(Message::UnstageFile(path)) if path == Path::new("src")
    ));
}

#[test]
fn test_discard_key_returns_discard_paths_for_selected_file() {
    let mut app = test_app();
    app.active_panel = SidePanel::Files;
    app.files.tree_nodes = vec![FileTreeNode {
        path: PathBuf::from("src/main.rs"),
        status: FileTreeNodeStatus::Unstaged(FileStatus::Modified),
        depth: 0,
        is_dir: false,
        is_expanded: false,
    }];
    app.files.panel.list_state.select(Some(0));

    let msg = app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
    assert!(matches!(
        msg,
        Some(Message::DiscardPaths(paths))
            if paths.len() == 1 && paths[0] == Path::new("src/main.rs")
    ));
}

#[test]
fn test_discard_key_in_visual_mode_returns_discard_selection() {
    let mut app = test_app();
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

    let msg = app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
    assert!(matches!(msg, Some(Message::DiscardSelection)));
}

#[test]
fn test_search_input_esc_clears_query_and_highlight() {
    let mut app = test_app();
    app.active_panel = SidePanel::Commits;

    update(&mut app, Message::StartSearchInput);
    update(&mut app, Message::SearchSetQuery("test".to_string()));
    assert_eq!(app.search_query, "test");

    let esc = app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(esc.is_none());
    assert!(app.search_query.is_empty());
    assert!(!app.has_search_query_for_active_scope());
}

#[test]
fn test_commits_tree_esc_clears_search_before_closing_tree() {
    let mut app = test_app();
    app.active_panel = SidePanel::Commits;
    app.commits.panel.list_state.select(Some(0));

    update(&mut app, Message::RevisionOpenTreeOrToggleDir);
    assert!(app.commits.tree_mode.active);

    update(&mut app, Message::StartSearchInput);
    update(&mut app, Message::SearchSetQuery("main".to_string()));
    app.confirm_search_input();
    update(&mut app, Message::SearchConfirm);
    assert!(!app.search_query.is_empty());

    let first_esc = app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(matches!(first_esc, Some(Message::SearchClear)));
    update(&mut app, first_esc.expect("search clear message"));
    assert!(app.commits.tree_mode.active);
    assert!(app.search_query.is_empty());

    let second_esc = app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(matches!(second_esc, Some(Message::RevisionCloseTree)));
}

#[test]
fn test_fetch_remote_returns_async_command_and_finishes() {
    let mut app = test_app();
    app.active_panel = SidePanel::LocalBranches;

    let cmd = update(&mut app, Message::FetchRemote);
    assert!(matches!(cmd, Some(Command::Async(_))));
    assert!(app.branches.is_fetching_remote);

    update(
        &mut app,
        Message::FetchRemoteFinished(Ok("origin".to_string())),
    );
    assert!(!app.branches.is_fetching_remote);
}

#[test]
fn test_full_refresh_defers_commit_reload_until_commits_panel_active() {
    let commits_calls = Arc::new(AtomicUsize::new(0));
    let repo = CountingRepo::new(commits_calls.clone());
    let mut app = App::from_repo(Box::new(repo)).expect("app from counting repo");
    assert_eq!(commits_calls.load(Ordering::SeqCst), 1);
    assert_eq!(app.active_panel, SidePanel::Files);

    app.request_refresh(RefreshKind::Full);
    app.flush_pending_refresh().expect("flush full refresh");
    assert_eq!(commits_calls.load(Ordering::SeqCst), 1);

    update(&mut app, Message::PanelGoto(3));
    assert_eq!(app.active_panel, SidePanel::Commits);
    assert_eq!(commits_calls.load(Ordering::SeqCst), 2);
}

#[test]
fn test_refresh_requests_coalesce_to_highest_priority_with_single_flush() {
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
fn test_fetch_finished_queues_full_refresh_without_immediate_flush() {
    let mut app = test_app();
    app.active_panel = SidePanel::LocalBranches;
    app.branches.is_fetching_remote = true;

    update(
        &mut app,
        Message::FetchRemoteFinished(Ok("origin".to_string())),
    );

    assert!(!app.branches.is_fetching_remote);
    assert_eq!(app.pending_refresh_kind(), Some(RefreshKind::Full));
}

#[test]
fn test_search_query_is_restored_per_panel_scope() {
    let mut app = test_app();

    app.active_panel = SidePanel::Commits;
    update(&mut app, Message::StartSearchInput);
    update(&mut app, Message::SearchSetQuery("test".to_string()));
    app.confirm_search_input();
    update(&mut app, Message::SearchConfirm);
    assert_eq!(app.search_query, "test");

    update(&mut app, Message::PanelGoto(1));
    assert_eq!(app.active_panel, SidePanel::Files);
    assert!(app.search_query.is_empty());

    update(&mut app, Message::StartSearchInput);
    update(&mut app, Message::SearchSetQuery("main".to_string()));
    app.confirm_search_input();
    update(&mut app, Message::SearchConfirm);
    assert_eq!(app.search_query, "main");

    update(&mut app, Message::PanelGoto(3));
    assert_eq!(app.active_panel, SidePanel::Commits);
    assert_eq!(app.search_query, "test");
}

#[test]
fn test_search_query_is_restored_between_commit_list_and_tree_scope() {
    let mut app = test_app();
    app.active_panel = SidePanel::Commits;
    app.commits.panel.list_state.select(Some(0));

    update(&mut app, Message::StartSearchInput);
    update(&mut app, Message::SearchSetQuery("test".to_string()));
    app.confirm_search_input();
    update(&mut app, Message::SearchConfirm);
    assert_eq!(app.search_query, "test");

    update(&mut app, Message::RevisionOpenTreeOrToggleDir);
    assert!(app.commits.tree_mode.active);
    assert!(app.search_query.is_empty());

    update(&mut app, Message::StartSearchInput);
    update(&mut app, Message::SearchSetQuery("main".to_string()));
    app.confirm_search_input();
    update(&mut app, Message::SearchConfirm);
    assert_eq!(app.search_query, "main");

    update(&mut app, Message::RevisionCloseTree);
    assert!(!app.commits.tree_mode.active);
    assert_eq!(app.search_query, "test");
}

#[test]
fn test_refresh_keeps_selected_duplicate_status_entry_and_diff_in_sync() {
    let mut app = App::from_repo(Box::new(DuplicateStatusRepo)).expect("app from duplicate repo");
    app.active_panel = SidePanel::Files;
    assert_eq!(app.files.tree_nodes.len(), 3);

    app.files.panel.list_state.select(Some(2));
    app.reload_diff_now();
    assert_eq!(app.current_diff[0].content, "staged");

    app.request_refresh(RefreshKind::StatusOnly);
    app.flush_pending_refresh().expect("flush refresh");

    assert_eq!(app.files.panel.list_state.selected(), Some(2));
    assert_eq!(app.current_diff[0].content, "staged");
}

#[test]
fn test_list_navigation_debounces_diff_in_files_panel() {
    let mut app = App::from_repo(Box::new(NavigationDiffRepo)).expect("app from navigation repo");
    app.active_panel = SidePanel::Files;
    app.files.panel.list_state.select(Some(0));
    app.reload_diff_now();
    assert_eq!(app.current_diff[0].content, "file a.txt");

    update(&mut app, Message::ListDown);

    assert_eq!(app.files.panel.list_state.selected(), Some(1));
    assert!(app.has_pending_diff_reload());
    assert_eq!(app.current_diff[0].content, "file a.txt");

    app.flush_pending_diff_reload();
    assert_eq!(app.current_diff[0].content, "file b.txt");
    assert!(!app.has_pending_diff_reload());
}

#[test]
fn test_list_navigation_debounces_diff_in_commits_panel() {
    let mut app = App::from_repo(Box::new(NavigationDiffRepo)).expect("app from navigation repo");
    app.active_panel = SidePanel::Commits;
    app.commits.panel.list_state.select(Some(0));
    app.reload_diff_now();
    assert_eq!(app.current_diff[0].content, "commit oid1");

    update(&mut app, Message::ListDown);

    assert_eq!(app.commits.panel.list_state.selected(), Some(1));
    assert!(app.has_pending_diff_reload());
    assert_eq!(app.current_diff[0].content, "commit oid1");

    app.flush_pending_diff_reload();
    assert_eq!(app.current_diff[0].content, "commit oid2");
    assert!(!app.has_pending_diff_reload());
}

#[test]
fn test_list_navigation_debounces_diff_in_local_branches_panel() {
    let mut app = App::from_repo(Box::new(NavigationDiffRepo)).expect("app from navigation repo");
    app.active_panel = SidePanel::LocalBranches;
    app.branches.panel.list_state.select(Some(0));
    app.reload_diff_now();
    assert_eq!(app.current_diff[0].content, "branch main");

    update(&mut app, Message::ListDown);

    assert_eq!(app.branches.panel.list_state.selected(), Some(1));
    assert!(app.has_pending_diff_reload());
    assert_eq!(app.current_diff[0].content, "branch main");

    app.flush_pending_diff_reload();
    assert_eq!(app.current_diff[0].content, "branch feature/x");
    assert!(!app.has_pending_diff_reload());
}
