use crate::app::{App, CommitFieldFocus, InputMode, RefreshKind};
use std::path::PathBuf;

impl App {
    pub fn start_commit_editor(&mut self) {
        self.input_mode = Some(InputMode::CommitEditor);
        self.commit_message_buffer.clear();
        self.commit_description_buffer.clear();
        self.commit_focus = CommitFieldFocus::Message;
    }

    pub fn start_commit_editor_guarded(&mut self) -> bool {
        if self.status.staged.is_empty() {
            if self.pending_refresh_kind().is_some() {
                self.request_refresh(RefreshKind::StatusOnly);
                if let Err(e) = self.flush_pending_refresh() {
                    self.push_log(format!("refresh failed: {}", e), false);
                    return false;
                }
            }
            if self.status.staged.is_empty() {
                // Check if there are any files to commit
                let total_files = self.status.unstaged.len() + self.status.untracked.len();
                if total_files == 0 {
                    self.push_log("nothing to commit", false);
                    return false;
                }
                // Enter confirmation mode
                self.input_mode = Some(InputMode::CommitAllConfirm);
                self.push_log("commit all: confirm to stage all files and commit", true);
                return true;
            }
        }
        self.start_commit_editor();
        true
    }

    pub fn start_branch_create_input(&mut self) {
        self.input_mode = Some(InputMode::CreateBranch);
        self.input_buffer.clear();
    }

    pub fn start_command_palette(&mut self) {
        self.input_mode = Some(InputMode::CommandPalette);
        self.input_buffer.clear();
    }

    pub fn start_stash_editor(&mut self, targets: Vec<PathBuf>) {
        self.input_mode = Some(InputMode::StashEditor);
        self.stash_targets = targets;
        self.stash_message_buffer.clear();
    }

    pub fn cancel_input(&mut self) {
        self.input_mode = None;
        self.input_buffer.clear();
        self.commit_message_buffer.clear();
        self.commit_description_buffer.clear();
        self.commit_focus = CommitFieldFocus::Message;
        self.stash_message_buffer.clear();
        self.stash_targets.clear();
        self.branch_switch_target = None;
    }

    pub(crate) fn resolve_command_palette_command(
        &self,
        input: &str,
    ) -> Option<crate::flux::action::DomainAction> {
        use crate::flux::action::DomainAction;
        let normalized = input.trim().to_lowercase();
        match normalized.as_str() {
            "q" | "quit" | "exit" => Some(DomainAction::Quit),
            "c" | "commit" => Some(DomainAction::StartCommitInput),
            "/" | "search" => Some(DomainAction::StartSearchInput),
            "stash" | "stash push" => Some(DomainAction::StartStashInput),
            "branch" | "branch create" | "branch new" => Some(DomainAction::StartBranchCreateInput),
            "fetch" | "fetch remote" => Some(DomainAction::FetchRemote),
            "panel 1" | "panel files" | "files" => Some(DomainAction::PanelGoto(1)),
            "panel 2" | "panel branches" | "branches" => Some(DomainAction::PanelGoto(2)),
            "panel 3" | "panel commits" | "commits" => Some(DomainAction::PanelGoto(3)),
            "panel 4" | "panel stash" | "stash panel" => Some(DomainAction::PanelGoto(4)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flux::stores::test_support::MockRepo;
    use pretty_assertions::assert_eq;

    fn mock_app() -> App {
        App::from_repo(Box::new(MockRepo)).expect("app")
    }

    #[test]
    fn test_start_commit_editor_sets_mode() {
        let mut app = mock_app();
        assert!(app.input_mode.is_none());
        app.start_commit_editor();
        assert_eq!(app.input_mode, Some(InputMode::CommitEditor));
        assert!(app.commit_message_buffer.is_empty());
    }

    #[test]
    fn test_start_commit_editor_guarded_with_staged_files() {
        let mut app = mock_app();
        app.status.staged.push(crate::git::FileEntry {
            path: "foo.txt".into(),
            status: crate::git::FileStatus::Modified,
        });
        let result = app.start_commit_editor_guarded();
        assert!(result);
        assert_eq!(app.input_mode, Some(InputMode::CommitEditor));
    }

    #[test]
    fn test_start_commit_editor_guarded_no_files_returns_false() {
        let mut app = mock_app();
        let result = app.start_commit_editor_guarded();
        assert!(!result);
        assert!(app.input_mode.is_none());
    }

    #[test]
    fn test_start_branch_create_input_sets_mode() {
        let mut app = mock_app();
        app.start_branch_create_input();
        assert_eq!(app.input_mode, Some(InputMode::CreateBranch));
        assert!(app.input_buffer.is_empty());
    }

    #[test]
    fn test_start_command_palette_sets_mode() {
        let mut app = mock_app();
        app.start_command_palette();
        assert_eq!(app.input_mode, Some(InputMode::CommandPalette));
    }

    #[test]
    fn test_start_stash_editor_sets_mode_and_targets() {
        let mut app = mock_app();
        let targets = vec!["foo.txt".into(), "bar.txt".into()];
        app.start_stash_editor(targets.clone());
        assert_eq!(app.input_mode, Some(InputMode::StashEditor));
        assert_eq!(app.stash_targets, targets);
    }

    #[test]
    fn test_cancel_input_clears_all_state() {
        let mut app = mock_app();
        app.input_mode = Some(InputMode::CommitEditor);
        app.input_buffer = "test".to_string();
        app.commit_message_buffer = "msg".to_string();
        app.commit_description_buffer = "desc".to_string();
        app.stash_message_buffer = "stash".to_string();
        app.stash_targets = vec!["foo.txt".into()];
        app.branch_switch_target = Some("main".to_string());
        app.cancel_input();
        assert!(app.input_mode.is_none());
        assert!(app.input_buffer.is_empty());
        assert!(app.commit_message_buffer.is_empty());
        assert!(app.stash_targets.is_empty());
        assert!(app.branch_switch_target.is_none());
    }

    #[test]
    fn test_resolve_command_palette_quit() {
        let app = mock_app();
        assert!(matches!(
            app.resolve_command_palette_command("quit"),
            Some(crate::flux::action::DomainAction::Quit)
        ));
    }

    #[test]
    fn test_resolve_command_palette_commit() {
        let app = mock_app();
        assert!(matches!(
            app.resolve_command_palette_command("commit"),
            Some(crate::flux::action::DomainAction::StartCommitInput)
        ));
    }

    #[test]
    fn test_resolve_command_palette_panel_goto() {
        let app = mock_app();
        assert!(matches!(
            app.resolve_command_palette_command("files"),
            Some(crate::flux::action::DomainAction::PanelGoto(1))
        ));
    }

    #[test]
    fn test_resolve_command_palette_case_insensitive() {
        let app = mock_app();
        assert!(matches!(
            app.resolve_command_palette_command("QUIT"),
            Some(crate::flux::action::DomainAction::Quit)
        ));
    }

    #[test]
    fn test_resolve_command_palette_unknown() {
        let app = mock_app();
        assert!(app
            .resolve_command_palette_command("unknown_command")
            .is_none());
    }
}
