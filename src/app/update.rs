use super::{App, Message, Command, SidePanel};

/// Documentation comment in English.
pub fn update(app: &mut App, msg: Message) -> Option<Command> {
    match msg {
        Message::Quit => {
            app.running = false;
            None
        }

        Message::PanelNext => {
            app.active_panel = match app.active_panel {
                SidePanel::Files => SidePanel::LocalBranches,
                SidePanel::LocalBranches => SidePanel::Commits,
                SidePanel::Commits => SidePanel::Stash,
                SidePanel::Stash => SidePanel::Files,
            };
            app.load_diff();
            None
        }

        Message::PanelPrev => {
            app.active_panel = match app.active_panel {
                SidePanel::Files => SidePanel::Stash,
                SidePanel::LocalBranches => SidePanel::Files,
                SidePanel::Commits => SidePanel::LocalBranches,
                SidePanel::Stash => SidePanel::Commits,
            };
            app.load_diff();
            None
        }

        Message::PanelGoto(n) => {
            app.active_panel = match n {
                1 => SidePanel::Files,
                2 => SidePanel::LocalBranches,
                3 => SidePanel::Commits,
                4 => SidePanel::Stash,
                _ => app.active_panel,
            };
            app.load_diff();
            None
        }

        Message::ListDown => {
            app.list_down();
            app.load_diff();
            None
        }

        Message::ListUp => {
            app.list_up();
            app.load_diff();
            None
        }

        Message::ToggleDir => {
            app.toggle_selected_dir();
            app.load_diff();
            None
        }

        Message::ToggleVisualSelectMode => {
            app.toggle_visual_select_mode();
            app.load_diff();
            None
        }

        Message::CollapseAll => {
            app.collapse_all();
            app.load_diff();
            None
        }

        Message::ExpandAll => {
            app.expand_all();
            app.load_diff();
            None
        }

        Message::DiffScrollUp => {
            app.diff_scroll_up();
            None
        }

        Message::DiffScrollDown => {
            app.diff_scroll_down();
            None
        }

        Message::RefreshStatus => {
            if let Err(e) = app.refresh_status() {
                app.push_log(format!("refresh failed: {}", e), false);
            } else {
                app.push_log("refresh", true);
                app.load_diff();
            }
            None
        }

        Message::StartCommitInput => {
            if app.start_commit_editor_guarded() {
                app.push_log("commit: edit message/description then press Enter on message", true);
            }
            None
        }

        Message::PrepareCommitFromSelection => {
            match app.prepare_commit_from_visual_selection() {
                Ok(count) => {
                    if count == 0 {
                        app.push_log("commit blocked: no selected items", false);
                        return None;
                    }
                    if app.start_commit_editor_guarded() {
                        app.push_log(
                            format!("commit: {} selected target(s) staged; edit message/description", count),
                            true,
                        );
                    }
                }
                Err(e) => app.push_log(format!("prepare commit failed: {}", e), false),
            }
            None
        }

        Message::ToggleStageSelection => {
            match app.toggle_stage_visual_selection() {
                Ok((staged, unstaged)) => {
                    app.push_log(
                        format!("selection toggled: staged {}, unstaged {}", staged, unstaged),
                        true,
                    );
                    app.load_diff();
                }
                Err(e) => app.push_log(format!("selection toggle failed: {}", e), false),
            }
            None
        }

        Message::StartStashInput => {
            let targets = app.prepare_stash_targets_from_selection();
            if targets.is_empty() {
                app.push_log("stash blocked: no selected items", false);
                return None;
            }
            app.start_stash_editor(targets);
            app.push_log("stash: enter title and press Enter", true);
            None
        }

        Message::StashPush { message, paths } => {
            match app.stash_push(&paths, &message) {
                Ok(index) => app.push_log(format!("stash created stash@{{{}}}: {}", index, message), true),
                Err(e) => app.push_log(format!("stash create failed: {}", e), false),
            }
            None
        }

        Message::RevisionOpenTreeOrToggleDir => {
            let result = match app.active_panel {
                SidePanel::Stash => app.stash_open_tree_or_toggle_dir(),
                SidePanel::Commits => app.commit_open_tree_or_toggle_dir(),
                _ => Ok(()),
            };
            match result {
                Ok(()) => app.load_diff(),
                Err(e) => app.push_log(format!("revision files failed: {}", e), false),
            }
            None
        }

        Message::RevisionCloseTree => {
            match app.active_panel {
                SidePanel::Stash => app.stash_close_tree(),
                SidePanel::Commits => app.commit_close_tree(),
                _ => {}
            }
            app.load_diff();
            None
        }

        Message::StashApplySelected => {
            if let Some(index) = app.selected_stash_index() {
                match app.stash_apply(index) {
                    Ok(()) => app.push_log(format!("stash applied stash@{{{}}}", index), true),
                    Err(e) => app.push_log(format!("stash apply failed stash@{{{}}}: {}", index, e), false),
                }
            } else {
                app.push_log("no stash selected", false);
            }
            None
        }

        Message::StashPopSelected => {
            if let Some(index) = app.selected_stash_index() {
                match app.stash_pop(index) {
                    Ok(()) => app.push_log(format!("stash popped stash@{{{}}}", index), true),
                    Err(e) => app.push_log(format!("stash pop failed stash@{{{}}}: {}", index, e), false),
                }
            } else {
                app.push_log("no stash selected", false);
            }
            None
        }

        Message::StashDropSelected => {
            if let Some(index) = app.selected_stash_index() {
                match app.stash_drop(index) {
                    Ok(()) => app.push_log(format!("stash dropped stash@{{{}}}", index), true),
                    Err(e) => app.push_log(format!("stash drop failed stash@{{{}}}: {}", index, e), false),
                }
            } else {
                app.push_log("no stash selected", false);
            }
            None
        }

        Message::StartBranchCreateInput => {
            app.start_branch_create_input();
            app.push_log("branch create: enter name and press Enter", true);
            None
        }

        Message::Commit(message) => {
            match app.commit(&message) {
                Ok(oid) => app.push_log(format!("commit {} ({})", message, oid), true),
                Err(e) => app.push_log(format!("commit failed: {}", e), false),
            }
            None
        }

        Message::CreateBranch(name) => {
            match app.create_branch(&name) {
                Ok(()) => app.push_log(format!("branch created: {}", name), true),
                Err(e) => app.push_log(format!("create branch failed: {}", e), false),
            }
            None
        }

        Message::CheckoutSelectedBranch => {
            if let Some(name) = app.selected_branch_name() {
                match app.checkout_branch(&name) {
                    Ok(()) => app.push_log(format!("checked out {}", name), true),
                    Err(e) => app.push_log(format!("checkout failed: {}", e), false),
                }
            } else {
                app.push_log("no branch selected", false);
            }
            None
        }

        Message::DeleteSelectedBranch => {
            if let Some(name) = app.selected_branch_name() {
                match app.delete_branch(&name) {
                    Ok(()) => app.push_log(format!("deleted branch {}", name), true),
                    Err(e) => app.push_log(format!("delete branch failed: {}", e), false),
                }
            } else {
                app.push_log("no branch selected", false);
            }
            None
        }

        Message::FetchRemote => {
            match app.fetch_remote() {
                Ok(remote) => app.push_log(format!("fetched {}", remote), true),
                Err(e) => app.push_log(format!("fetch failed: {}", e), false),
            }
            None
        }

        Message::StageFile(path) => {
            let display = path.display().to_string();
            if let Err(e) = app.stage_file(path) {
                app.push_log(format!("stage failed {}: {}", display, e), false);
            } else {
                app.push_log(format!("staged {}", display), true);
            }
            None
        }

        Message::UnstageFile(path) => {
            let display = path.display().to_string();
            if let Err(e) = app.unstage_file(path) {
                app.push_log(format!("unstage failed {}: {}", display, e), false);
            } else {
                app.push_log(format!("unstaged {}", display), true);
            }
            None
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
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
            Ok(vec![CommitInfo {
                short_hash: "abc1234".to_string(),
                oid: "abc1234567890".to_string(),
                message: "test commit".to_string(),
                author: "tester".to_string(),
                time: "2026-03-20 00:00".to_string(),
                parent_count: 1,
                sync_state: CommitSyncState::Main,
            }])
        }

        fn commit_files(&self, _oid: &str) -> Result<Vec<FileEntry>, GitError> {
            Ok(vec![FileEntry {
                path: PathBuf::from("src/main.rs"),
                status: FileStatus::Modified,
            }])
        }

        fn stashes(&self) -> Result<Vec<StashInfo>, GitError> {
            Ok(vec![StashInfo {
                index: 0,
                message: "stash test".to_string(),
            }])
        }

        fn stash_files(&self, _index: usize) -> Result<Vec<FileEntry>, GitError> {
            Ok(vec![FileEntry {
                path: PathBuf::from("src/lib.rs"),
                status: FileStatus::Modified,
            }])
        }

        fn stash_diff(&self, _index: usize, _path: Option<&Path>) -> Result<Vec<DiffLine>, GitError> {
            Ok(vec![DiffLine {
                kind: DiffLineKind::Header,
                content: "stash diff".to_string(),
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

        fn commit_diff_scoped(&self, _oid: &str, _path: Option<&Path>) -> Result<Vec<DiffLine>, GitError> {
            Ok(vec![DiffLine {
                kind: DiffLineKind::Header,
                content: "commit diff".to_string(),
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
}
