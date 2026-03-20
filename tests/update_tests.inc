use super::*;
use crate::app::{App, SidePanel};
use crate::git::{
    BranchInfo, CommitInfo, CommitSyncState, DiffLine, DiffLineKind, FileEntry, FileStatus,
    GitError, GitRepository, GitStatus, StashInfo,
};
use std::path::{Path, PathBuf};

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
                time: "2026-03-20 00:00".to_string(),
                parent_count: 1,
                sync_state: CommitSyncState::Main,
            },
            CommitInfo {
                short_hash: "def5678".to_string(),
                oid: "def5678901234".to_string(),
                message: "test commit 2".to_string(),
                author: "tester".to_string(),
                time: "2026-03-20 00:01".to_string(),
                parent_count: 1,
                sync_state: CommitSyncState::Main,
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

    fn commit_diff_scoped(&self, _oid: &str, path: Option<&Path>) -> Result<Vec<DiffLine>, GitError> {
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

    fn fetch_default(&self) -> Result<String, GitError> {
        Ok("origin".to_string())
    }
}

fn test_app() -> App {
    App::from_repo(Box::new(MockRepo)).expect("app from mock repo")
}

#[test]
fn test_revision_open_close_for_commits_panel() {
    let mut app = test_app();
    app.active_panel = SidePanel::Commits;
    app.commits_panel.list_state.select(Some(0));

    update(&mut app, Message::RevisionOpenTreeOrToggleDir);
    assert!(app.commit_tree_mode);
    assert_eq!(app.commit_tree_commit_oid.as_deref(), Some("abc1234567890"));
    assert!(!app.commit_tree_nodes.is_empty());

    update(&mut app, Message::RevisionCloseTree);
    assert!(!app.commit_tree_mode);
    assert!(app.commit_tree_commit_oid.is_none());
}

#[test]
fn test_revision_open_close_for_stash_panel() {
    let mut app = test_app();
    app.active_panel = SidePanel::Stash;
    app.stash_panel.list_state.select(Some(0));

    update(&mut app, Message::RevisionOpenTreeOrToggleDir);
    assert!(app.stash_tree_mode);
    assert_eq!(app.stash_tree_stash_index, Some(0));
    assert!(!app.stash_tree_nodes.is_empty());

    update(&mut app, Message::RevisionCloseTree);
    assert!(!app.stash_tree_mode);
    assert!(app.stash_tree_stash_index.is_none());
}

#[test]
fn test_commit_diff_scopes_path_in_tree_mode() {
    let mut app = test_app();
    app.active_panel = SidePanel::Commits;
    app.commits_panel.list_state.select(Some(0));

    app.load_diff();
    assert!(app.current_diff[0].content.contains("<none>"));

    update(&mut app, Message::RevisionOpenTreeOrToggleDir);
    app.load_diff();
    assert!(!app.current_diff[0].content.contains("<none>"));
}

#[test]
fn test_stash_diff_scopes_path_in_tree_mode() {
    let mut app = test_app();
    app.active_panel = SidePanel::Stash;
    app.stash_panel.list_state.select(Some(0));

    app.load_diff();
    assert!(app.current_diff[0].content.contains("<none>"));

    update(&mut app, Message::RevisionOpenTreeOrToggleDir);
    app.load_diff();
    assert!(!app.current_diff[0].content.contains("<none>"));
}

#[test]
fn test_commit_close_tree_restores_list_selection() {
    let mut app = test_app();
    app.active_panel = SidePanel::Commits;
    app.commits_panel.list_state.select(Some(1));

    update(&mut app, Message::RevisionOpenTreeOrToggleDir);
    assert!(app.commit_tree_mode);
    assert_eq!(app.commit_tree_commit_oid.as_deref(), Some("def5678901234"));

    update(&mut app, Message::RevisionCloseTree);
    assert!(!app.commit_tree_mode);
    assert_eq!(app.commits_panel.list_state.selected(), Some(1));
}

#[test]
fn test_stash_close_tree_restores_list_selection() {
    let mut app = test_app();
    app.active_panel = SidePanel::Stash;
    app.stash_panel.list_state.select(Some(1));

    update(&mut app, Message::RevisionOpenTreeOrToggleDir);
    assert!(app.stash_tree_mode);
    assert_eq!(app.stash_tree_stash_index, Some(1));

    update(&mut app, Message::RevisionCloseTree);
    assert!(!app.stash_tree_mode);
    assert_eq!(app.stash_panel.list_state.selected(), Some(1));
}
