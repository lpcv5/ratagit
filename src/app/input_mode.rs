#[cfg(test)]
use crate::app::App;
use crate::app::{AppState, CommitFieldFocus, InputMode, RefreshKind};
use crate::flux::command_output::CommandOutput;
use std::path::PathBuf;

impl AppState {
    pub fn start_commit_editor(&mut self) {
        self.input.error = None;
        self.input.mode = Some(InputMode::CommitEditor);
        self.input.commit_message_buffer.clear();
        self.input.commit_description_buffer.clear();
        self.input.commit_focus = CommitFieldFocus::Message;
    }

    pub fn start_commit_editor_guarded(&mut self, output: &mut CommandOutput) -> bool {
        if self.files.status().staged.is_empty() {
            if self.pending_refresh_kind().is_some() {
                self.request_refresh(RefreshKind::StatusOnly, output);
            }
            // Check if there are any files to commit
            let total_files =
                self.files.status().unstaged.len() + self.files.status().untracked.len();
            if total_files == 0 {
                self.push_log("nothing to commit", false);
                return false;
            }
            // Enter confirmation mode
            self.input.mode = Some(InputMode::CommitAllConfirm);
            self.push_log("commit all: confirm to stage all files and commit", true);
            return true;
        }
        self.start_commit_editor();
        true
    }

    pub fn start_branch_create_input(&mut self) {
        self.input.error = None;
        self.input.mode = Some(InputMode::CreateBranch);
        self.input.buffer.clear();
    }

    pub fn start_command_palette(&mut self) {
        self.input.error = None;
        self.input.mode = Some(InputMode::CommandPalette);
        self.input.buffer.clear();
        self.input.palette_selected = 0;
    }

    pub fn start_stash_editor(&mut self, targets: Vec<PathBuf>) {
        self.input.error = None;
        self.input.mode = Some(InputMode::StashEditor);
        self.input.stash_targets = targets;
        self.input.stash_message_buffer.clear();
    }

    pub fn cancel_input(&mut self) {
        self.input.error = None;
        self.input.mode = None;
        self.input.palette_selected = 0;
        self.input.help = super::help::HelpState::default();
        self.input.buffer.clear();
        self.input.commit_message_buffer.clear();
        self.input.commit_description_buffer.clear();
        self.input.commit_focus = CommitFieldFocus::Message;
        self.input.stash_message_buffer.clear();
        self.input.stash_targets.clear();
        self.input.branch_switch_target = None;
    }

    pub(crate) fn resolve_command_palette_command(
        &self,
        input: &str,
    ) -> Option<crate::flux::action::DomainAction> {
        let mut env = self.command_env();
        env.input = None;
        super::commands::resolve_palette(input, &env)
    }

    pub(crate) fn move_palette_selection(&mut self, delta: isize) {
        let commands = self.available_commands();
        let count = super::commands::filter_commands(&self.input.buffer, &commands).len();
        self.input.palette_selected = self
            .input
            .palette_selected
            .saturating_add_signed(delta)
            .min(count.saturating_sub(1));
    }

    pub(crate) fn selected_palette_command(&self) -> Option<crate::flux::action::DomainAction> {
        let commands = self.available_commands();
        let filtered = super::commands::filter_commands(&self.input.buffer, &commands);
        let selected = self
            .input
            .palette_selected
            .min(filtered.len().saturating_sub(1));
        self.resolve_command_palette_command(filtered.get(selected)?.name)
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
        assert!(app.input.mode.is_none());
        app.start_commit_editor();
        assert_eq!(app.input.mode, Some(InputMode::CommitEditor));
        assert!(app.input.commit_message_buffer.is_empty());
    }

    #[test]
    fn test_start_commit_editor_guarded_with_staged_files() {
        let mut app = mock_app();
        app.files
            .test_status_mut()
            .staged
            .push(crate::git::FileEntry {
                path: "foo.txt".into(),
                status: crate::git::FileStatus::Modified,
            });
        let result = app.start_commit_editor_guarded();
        assert!(result);
        assert_eq!(app.input.mode, Some(InputMode::CommitEditor));
    }

    #[test]
    fn test_start_commit_editor_guarded_no_files_returns_false() {
        let mut app = mock_app();
        let result = app.start_commit_editor_guarded();
        assert!(!result);
        assert!(app.input.mode.is_none());
    }

    #[test]
    fn test_start_commit_editor_guarded_with_pending_refresh_requires_confirmation() {
        let mut app = mock_app();
        app.files
            .test_status_mut()
            .untracked
            .push(crate::git::FileEntry {
                path: "new.txt".into(),
                status: crate::git::FileStatus::New,
            });
        let mut output = CommandOutput::default();
        app.state.request_refresh(RefreshKind::Full, &mut output);

        assert!(app.state.start_commit_editor_guarded(&mut output));

        assert_eq!(app.input.mode, Some(InputMode::CommitAllConfirm));
        assert_eq!(app.pending_refresh_kind(), Some(RefreshKind::Full));
        assert!(app.files.status().staged.is_empty());
        assert_eq!(app.files.status().untracked.len(), 1);
    }

    #[test]
    fn test_start_branch_create_input_sets_mode() {
        let mut app = mock_app();
        app.start_branch_create_input();
        assert_eq!(app.input.mode, Some(InputMode::CreateBranch));
        assert!(app.input.buffer.is_empty());
    }

    #[test]
    fn test_start_command_palette_sets_mode() {
        let mut app = mock_app();
        app.start_command_palette();
        assert_eq!(app.input.mode, Some(InputMode::CommandPalette));
    }

    #[test]
    fn test_start_stash_editor_sets_mode_and_targets() {
        let mut app = mock_app();
        let targets = vec!["foo.txt".into(), "bar.txt".into()];
        app.start_stash_editor(targets.clone());
        assert_eq!(app.input.mode, Some(InputMode::StashEditor));
        assert_eq!(app.input.stash_targets, targets);
    }

    #[test]
    fn test_cancel_input_clears_all_state() {
        let mut app = mock_app();
        app.input.mode = Some(InputMode::CommitEditor);
        app.input.buffer = "test".to_string();
        app.input.commit_message_buffer = "msg".to_string();
        app.input.commit_description_buffer = "desc".to_string();
        app.input.stash_message_buffer = "stash".to_string();
        app.input.stash_targets = vec!["foo.txt".into()];
        app.input.branch_switch_target = Some("main".to_string());
        app.cancel_input();
        assert!(app.input.mode.is_none());
        assert!(app.input.buffer.is_empty());
        assert!(app.input.commit_message_buffer.is_empty());
        assert!(app.input.stash_targets.is_empty());
        assert!(app.input.branch_switch_target.is_none());
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
