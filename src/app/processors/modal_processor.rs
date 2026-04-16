// src/app/processors/modal_processor.rs
use crate::app::events::ModalEvent;
use crate::app::state::AppState;
use crate::app::ui_state::Panel;
use crate::components::{Intent, ModalDialog};

pub struct ModalProcessor;

impl ModalProcessor {
    pub fn process(&self, event: ModalEvent, state: &mut AppState) {
        match event {
            ModalEvent::ShowHelp => {
                state.active_modal = Some(self.create_help_modal(state.ui_state.active_panel));
            }
            ModalEvent::ShowCommitDialog => {
                state.active_modal = Some(ModalDialog::text_input(
                    "Commit".to_string(),
                    "Enter commit message:".to_string(),
                ));
            }
            ModalEvent::ShowRenameDialog => {
                state.active_modal = Some(ModalDialog::text_input(
                    "Rename File".to_string(),
                    "Enter new filename:".to_string(),
                ));
            }
            ModalEvent::ShowResetMenu => {
                state.active_modal = Some(self.create_reset_menu());
            }
            ModalEvent::ShowDiscardConfirmation => {
                state.active_modal = Some(ModalDialog::confirmation(
                    "Discard Changes".to_string(),
                    "Discard changes to selected file(s)?\nThis cannot be undone.".to_string(),
                    Intent::None, // Will be replaced by event-driven flow
                ));
            }
            ModalEvent::ShowStashConfirmation => {
                state.active_modal = Some(ModalDialog::confirmation(
                    "Stash Changes".to_string(),
                    "Stash selected file(s)?".to_string(),
                    Intent::None, // Will be replaced by event-driven flow
                ));
            }
            ModalEvent::ShowAmendConfirmation => {
                state.active_modal = Some(ModalDialog::confirmation(
                    "Amend Commit".to_string(),
                    "Amend the last commit with staged changes?".to_string(),
                    Intent::None, // Will be replaced by event-driven flow
                ));
            }
            ModalEvent::ShowResetConfirmation(index) => {
                state.active_modal = Some(self.create_reset_confirmation(index));
            }
            ModalEvent::ShowNukeConfirmation => {
                state.active_modal = Some(ModalDialog::confirmation(
                    "NUKE REPOSITORY".to_string(),
                    "Are you ABSOLUTELY SURE?\nThis will DELETE the .git directory.\nThis CANNOT be undone!".to_string(),
                    Intent::None, // Will be replaced by event-driven flow
                ));
            }
            ModalEvent::Close => {
                state.active_modal = None;
            }
        }
    }

    fn create_help_modal(&self, panel: Panel) -> ModalDialog {
        let items = match panel {
            Panel::Files => vec![
                ("j/k or ↑/↓".to_string(), Intent::None),
                ("Space".to_string(), Intent::None),
                ("a".to_string(), Intent::None),
                ("c".to_string(), Intent::None),
                ("d".to_string(), Intent::None),
                ("s".to_string(), Intent::None),
                ("i".to_string(), Intent::None),
                ("r".to_string(), Intent::None),
            ],
            Panel::Branches => vec![
                ("j/k or ↑/↓".to_string(), Intent::None),
                ("Enter".to_string(), Intent::None),
                ("d".to_string(), Intent::None),
            ],
            Panel::Commits => vec![
                ("j/k or ↑/↓".to_string(), Intent::None),
                ("Enter".to_string(), Intent::None),
            ],
            Panel::Stash => vec![
                ("j/k or ↑/↓".to_string(), Intent::None),
                ("Space".to_string(), Intent::None),
                ("p".to_string(), Intent::None),
                ("d".to_string(), Intent::None),
            ],
            Panel::MainView | Panel::Log => vec![
                ("q".to_string(), Intent::None),
                ("?".to_string(), Intent::None),
            ],
        };

        ModalDialog::help(format!("{} Keybindings", panel.title()), items)
    }

    fn create_reset_menu(&self) -> ModalDialog {
        let options = vec![
            "Reset --hard HEAD".to_string(),
            "Reset --mixed HEAD".to_string(),
            "Reset --soft HEAD".to_string(),
            "Reset --hard HEAD~1".to_string(),
            "Reset --soft HEAD~1".to_string(),
            "Nuke repo (delete .git)".to_string(),
        ];
        ModalDialog::selection("Reset Options".to_string(), options)
    }

    fn create_reset_confirmation(&self, index: usize) -> ModalDialog {
        let (target, reset_type) = match index {
            0 => ("HEAD", "hard"),
            1 => ("HEAD", "mixed"),
            2 => ("HEAD", "soft"),
            3 => ("HEAD~1", "hard"),
            4 => ("HEAD~1", "soft"),
            _ => ("HEAD", "hard"),
        };

        ModalDialog::confirmation(
            "Confirm Reset".to_string(),
            format!(
                "Reset {} to {}?\nThis will discard changes.",
                reset_type, target
            ),
            Intent::None, // Will be replaced by event-driven flow
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
        state.active_modal = Some(crate::components::ModalDialog::help(
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
