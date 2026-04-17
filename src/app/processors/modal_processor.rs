// src/app/processors/modal_processor.rs
use crate::app::events::{AppEvent, GitEvent, ModalEvent};
use crate::app::state::AppState;
use crate::app::ui_state::Panel;
use crate::components::dialogs::ModalDialogV2;

pub struct ModalProcessor;

impl ModalProcessor {
    pub fn process(&self, event: ModalEvent, state: &mut AppState) {
        match event {
            ModalEvent::ShowHelp => {
                state.active_modal = Some(self.create_help_modal(state.ui_state.active_panel));
            }
            ModalEvent::ShowCommitDialog => {
                state.active_modal = Some(ModalDialogV2::text_input(
                    "Commit".to_string(),
                    "Enter commit message:".to_string(),
                ));
            }
            ModalEvent::ShowRenameDialog => {
                state.active_modal = Some(ModalDialogV2::text_input(
                    "Rename File".to_string(),
                    "Enter new filename:".to_string(),
                ));
            }
            ModalEvent::ShowResetMenu => {
                state.active_modal = Some(self.create_reset_menu());
            }
            ModalEvent::ShowDiscardConfirmation => {
                state.active_modal = Some(ModalDialogV2::confirmation(
                    "Discard Changes".to_string(),
                    "Discard changes to selected file(s)?\nThis cannot be undone.".to_string(),
                    AppEvent::Git(GitEvent::DiscardSelected),
                ));
            }
            ModalEvent::ShowStashConfirmation => {
                state.active_modal = Some(ModalDialogV2::confirmation(
                    "Stash Changes".to_string(),
                    "Stash selected file(s)?".to_string(),
                    AppEvent::Git(GitEvent::StashSelected),
                ));
            }
            ModalEvent::ShowAmendConfirmation => {
                state.active_modal = Some(ModalDialogV2::confirmation(
                    "Amend Commit".to_string(),
                    "Amend the last commit with staged changes?".to_string(),
                    AppEvent::Git(GitEvent::AmendCommit),
                ));
            }
            ModalEvent::ShowResetConfirmation(index) => {
                state.active_modal = Some(self.create_reset_confirmation(index));
            }
            ModalEvent::ShowNukeConfirmation => {
                state.active_modal = Some(ModalDialogV2::confirmation(
                    "NUKE REPOSITORY".to_string(),
                    "Are you ABSOLUTELY SURE?\nThis will DELETE the .git directory.\nThis CANNOT be undone!".to_string(),
                    AppEvent::None, // Placeholder - nuke not implemented yet
                ));
            }
            ModalEvent::Close => {
                state.active_modal = None;
            }
        }
    }

    fn create_help_modal(&self, panel: Panel) -> ModalDialogV2 {
        let items = match panel {
            Panel::Files => vec![
                ("j/k or ↑/↓".to_string(), AppEvent::None),
                ("Space".to_string(), AppEvent::None),
                ("a".to_string(), AppEvent::None),
                ("c".to_string(), AppEvent::None),
                ("d".to_string(), AppEvent::None),
                ("s".to_string(), AppEvent::None),
                ("i".to_string(), AppEvent::None),
                ("r".to_string(), AppEvent::None),
            ],
            Panel::Branches => vec![
                ("j/k or ↑/↓".to_string(), AppEvent::None),
                ("Enter".to_string(), AppEvent::None),
                ("d".to_string(), AppEvent::None),
            ],
            Panel::Commits => vec![
                ("j/k or ↑/↓".to_string(), AppEvent::None),
                ("Enter".to_string(), AppEvent::None),
            ],
            Panel::Stash => vec![
                ("j/k or ↑/↓".to_string(), AppEvent::None),
                ("Space".to_string(), AppEvent::None),
                ("p".to_string(), AppEvent::None),
                ("d".to_string(), AppEvent::None),
            ],
            Panel::MainView | Panel::Log => vec![
                ("q".to_string(), AppEvent::None),
                ("?".to_string(), AppEvent::None),
            ],
        };

        ModalDialogV2::help(format!("{} Keybindings", panel.title()), items)
    }

    fn create_reset_menu(&self) -> ModalDialogV2 {
        let options = vec![
            "Reset --hard HEAD".to_string(),
            "Reset --mixed HEAD".to_string(),
            "Reset --soft HEAD".to_string(),
            "Reset --hard HEAD~1".to_string(),
            "Reset --soft HEAD~1".to_string(),
            "Nuke repo (delete .git)".to_string(),
        ];
        ModalDialogV2::selection("Reset Options".to_string(), options)
    }

    fn create_reset_confirmation(&self, index: usize) -> ModalDialogV2 {
        let (target, reset_type) = match index {
            0 => ("HEAD", "hard"),
            1 => ("HEAD", "mixed"),
            2 => ("HEAD", "soft"),
            3 => ("HEAD~1", "hard"),
            4 => ("HEAD~1", "soft"),
            _ => ("HEAD", "hard"),
        };

        ModalDialogV2::confirmation(
            "Confirm Reset".to_string(),
            format!(
                "Reset {} to {}?\nThis will discard changes.",
                reset_type, target
            ),
            AppEvent::Git(GitEvent::ExecuteReset(index)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::events::ModalEvent;

    fn mock_state() -> crate::app::state::AppState {
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(4);
        let (_event_tx, event_rx) = tokio::sync::mpsc::channel(4);
        crate::app::state::AppState::new(cmd_tx, event_rx)
    }

    fn mock_state_with_modal() -> crate::app::state::AppState {
        let mut state = mock_state();
        state.active_modal = Some(crate::components::dialogs::ModalDialogV2::help(
            "Test".to_string(),
            vec![],
        ));
        state
    }

    #[test]
    fn test_show_help_modal() {
        let processor = ModalProcessor;
        let mut state = mock_state();
        processor.process(ModalEvent::ShowHelp, &mut state);

        assert!(state.active_modal.is_some());
    }

    #[test]
    fn test_close_modal() {
        let processor = ModalProcessor;
        let mut state = mock_state_with_modal();
        processor.process(ModalEvent::Close, &mut state);

        assert!(state.active_modal.is_none());
    }

    #[test]
    fn test_show_commit_dialog() {
        let processor = ModalProcessor;
        let mut state = mock_state();
        processor.process(ModalEvent::ShowCommitDialog, &mut state);

        assert!(state.active_modal.is_some());
    }

    #[test]
    fn test_show_rename_dialog() {
        let processor = ModalProcessor;
        let mut state = mock_state();
        processor.process(ModalEvent::ShowRenameDialog, &mut state);

        assert!(state.active_modal.is_some());
    }

    #[test]
    fn test_show_reset_menu() {
        let processor = ModalProcessor;
        let mut state = mock_state();
        processor.process(ModalEvent::ShowResetMenu, &mut state);

        assert!(state.active_modal.is_some());
    }

    #[test]
    fn test_show_discard_confirmation() {
        let processor = ModalProcessor;
        let mut state = mock_state();
        processor.process(ModalEvent::ShowDiscardConfirmation, &mut state);

        assert!(state.active_modal.is_some());
    }

    #[test]
    fn test_show_stash_confirmation() {
        let processor = ModalProcessor;
        let mut state = mock_state();
        processor.process(ModalEvent::ShowStashConfirmation, &mut state);

        assert!(state.active_modal.is_some());
    }

    #[test]
    fn test_show_amend_confirmation() {
        let processor = ModalProcessor;
        let mut state = mock_state();
        processor.process(ModalEvent::ShowAmendConfirmation, &mut state);

        assert!(state.active_modal.is_some());
    }

    #[test]
    fn test_show_reset_confirmation() {
        let processor = ModalProcessor;
        let mut state = mock_state();
        processor.process(ModalEvent::ShowResetConfirmation(0), &mut state);

        assert!(state.active_modal.is_some());
    }

    #[test]
    fn test_show_nuke_confirmation() {
        let processor = ModalProcessor;
        let mut state = mock_state();
        processor.process(ModalEvent::ShowNukeConfirmation, &mut state);

        assert!(state.active_modal.is_some());
    }
}
