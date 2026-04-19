// src/app/processors/modal_processor.rs
//
// ModalProcessor handles modal dialog events.
//
// This processor is responsible for:
// - Creating appropriate modal dialogs (help, confirmation, text input, menu)
// - Updating AppState.active_modal to show/hide modals
// - Configuring modal callbacks (what event to return on confirmation)
//
// ModalProcessor directly mutates AppState (unlike GitProcessor which returns commands).
// This is safe because modal state is purely UI state, not Git state.

use crate::app::events::{AppEvent, BranchDeleteMode, BranchRemoteRef, GitEvent, ModalEvent};
use crate::app::state::AppState;
use crate::app::ui_state::Panel;
use crate::components::dialogs::{ModalDialogV2, SelectionItemV2, TextSubmitAction};

pub struct ModalProcessor;

impl ModalProcessor {
    /// Process a ModalEvent and update AppState
    ///
    /// This directly mutates state.active_modal to show/hide dialogs.
    /// Modals handle their own input and return AppEvent when confirmed/cancelled.
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
            ModalEvent::ShowBranchCreateDialog { from_branch } => {
                state.active_modal = Some(ModalDialogV2::text_input_with_action(
                    "New Branch".to_string(),
                    format!("Create from '{from_branch}':"),
                    TextSubmitAction::CreateBranch { from_branch },
                ));
            }
            ModalEvent::ShowBranchDeleteMenu {
                local_branch,
                is_head,
                upstream,
            } => {
                state.active_modal =
                    Some(self.create_branch_delete_menu(local_branch, is_head, upstream));
            }
            ModalEvent::ShowBranchDeleteConfirm {
                local_branch,
                remote,
                mode,
            } => {
                state.active_modal =
                    Some(self.create_branch_delete_confirm(local_branch, remote, mode));
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
                ("A".to_string(), AppEvent::None),
                ("c".to_string(), AppEvent::None),
                ("d".to_string(), AppEvent::None),
                ("D".to_string(), AppEvent::None),
                ("s".to_string(), AppEvent::None),
                ("i".to_string(), AppEvent::None),
                ("`".to_string(), AppEvent::None),
                ("-".to_string(), AppEvent::None),
                ("=".to_string(), AppEvent::None),
            ],
            Panel::Branches => vec![
                ("j/k or ↑/↓".to_string(), AppEvent::None),
                ("Space".to_string(), AppEvent::None),
                ("n".to_string(), AppEvent::None),
                ("d".to_string(), AppEvent::None),
                ("Enter".to_string(), AppEvent::None),
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
        let items = vec![
            SelectionItemV2 {
                label: "Reset --hard HEAD".to_string(),
                event: AppEvent::Modal(ModalEvent::ShowResetConfirmation(0)),
                enabled: true,
            },
            SelectionItemV2 {
                label: "Reset --mixed HEAD".to_string(),
                event: AppEvent::Modal(ModalEvent::ShowResetConfirmation(1)),
                enabled: true,
            },
            SelectionItemV2 {
                label: "Reset --soft HEAD".to_string(),
                event: AppEvent::Modal(ModalEvent::ShowResetConfirmation(2)),
                enabled: true,
            },
            SelectionItemV2 {
                label: "Reset --hard HEAD~1".to_string(),
                event: AppEvent::Modal(ModalEvent::ShowResetConfirmation(3)),
                enabled: true,
            },
            SelectionItemV2 {
                label: "Reset --soft HEAD~1".to_string(),
                event: AppEvent::Modal(ModalEvent::ShowResetConfirmation(4)),
                enabled: true,
            },
            SelectionItemV2 {
                label: "Nuke repo (delete .git)".to_string(),
                event: AppEvent::Modal(ModalEvent::ShowNukeConfirmation),
                enabled: true,
            },
        ];
        ModalDialogV2::selection("Reset Options".to_string(), items)
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

    fn create_branch_delete_menu(
        &self,
        local_branch: String,
        is_head: bool,
        upstream: Option<String>,
    ) -> ModalDialogV2 {
        let remote = parse_upstream_ref(upstream);
        let can_delete_local = !is_head;
        let can_delete_remote = remote.is_some();
        let can_delete_both = can_delete_local && can_delete_remote;

        let local_label = if can_delete_local {
            "Delete local branch".to_string()
        } else {
            "Delete local branch (current branch)".to_string()
        };
        let remote_label = if can_delete_remote {
            "Delete remote branch".to_string()
        } else {
            "Delete remote branch (no upstream)".to_string()
        };
        let both_label = if can_delete_both {
            "Delete local + remote branch".to_string()
        } else {
            "Delete local + remote branch (unavailable)".to_string()
        };

        let items = vec![
            SelectionItemV2 {
                label: local_label,
                event: AppEvent::Modal(ModalEvent::ShowBranchDeleteConfirm {
                    local_branch: local_branch.clone(),
                    remote: remote.clone(),
                    mode: BranchDeleteMode::Local,
                }),
                enabled: can_delete_local,
            },
            SelectionItemV2 {
                label: remote_label,
                event: AppEvent::Modal(ModalEvent::ShowBranchDeleteConfirm {
                    local_branch: local_branch.clone(),
                    remote: remote.clone(),
                    mode: BranchDeleteMode::Remote,
                }),
                enabled: can_delete_remote,
            },
            SelectionItemV2 {
                label: both_label,
                event: AppEvent::Modal(ModalEvent::ShowBranchDeleteConfirm {
                    local_branch: local_branch.clone(),
                    remote,
                    mode: BranchDeleteMode::LocalAndRemote,
                }),
                enabled: can_delete_both,
            },
            SelectionItemV2 {
                label: "Cancel".to_string(),
                event: AppEvent::Modal(ModalEvent::Close),
                enabled: true,
            },
        ];

        ModalDialogV2::selection(format!("Delete '{local_branch}'"), items)
    }

    fn create_branch_delete_confirm(
        &self,
        local_branch: String,
        remote: Option<BranchRemoteRef>,
        mode: BranchDeleteMode,
    ) -> ModalDialogV2 {
        let (title, message) = match (&mode, remote.as_ref()) {
            (BranchDeleteMode::Local, _) => (
                "Confirm Delete Local Branch".to_string(),
                format!("Delete local branch '{local_branch}'?"),
            ),
            (BranchDeleteMode::Remote, Some(remote_ref)) => (
                "Confirm Delete Remote Branch".to_string(),
                format!(
                    "Delete remote branch '{}/{}'?",
                    remote_ref.remote_name, remote_ref.branch_name
                ),
            ),
            (BranchDeleteMode::LocalAndRemote, Some(remote_ref)) => (
                "Confirm Delete Local + Remote".to_string(),
                format!(
                    "Delete local branch '{local_branch}' and remote branch '{}/{}'?",
                    remote_ref.remote_name, remote_ref.branch_name
                ),
            ),
            (BranchDeleteMode::Remote, None) => (
                "Delete Branch".to_string(),
                "Remote branch is unavailable for this entry.".to_string(),
            ),
            (BranchDeleteMode::LocalAndRemote, None) => (
                "Delete Branch".to_string(),
                "Remote branch is unavailable for this entry.".to_string(),
            ),
        };

        ModalDialogV2::confirmation(
            title,
            message,
            AppEvent::Git(GitEvent::DeleteBranch {
                local_branch,
                remote,
                mode,
            }),
        )
    }
}

fn parse_upstream_ref(upstream: Option<String>) -> Option<BranchRemoteRef> {
    let value = upstream?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let normalized = trimmed.strip_prefix("refs/remotes/").unwrap_or(trimmed);
    let (remote_name, branch_name) = normalized.split_once('/')?;
    if remote_name.is_empty() || branch_name.is_empty() {
        return None;
    }

    Some(BranchRemoteRef {
        remote_name: remote_name.to_string(),
        branch_name: branch_name.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::events::ModalEvent;
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

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

    #[test]
    fn test_branch_delete_menu_disables_unavailable_items() {
        let processor = ModalProcessor;
        let mut state = mock_state();
        processor.process(
            ModalEvent::ShowBranchDeleteMenu {
                local_branch: "main".to_string(),
                is_head: true,
                upstream: None,
            },
            &mut state,
        );

        let modal = state.active_modal.as_mut().expect("modal should exist");

        // First item is disabled (cannot delete checked out branch).
        let event = modal.handle_event_v2(&Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )));
        assert_eq!(event, AppEvent::None);

        // Move to Cancel and ensure it returns close.
        for _ in 0..3 {
            let _ = modal.handle_event_v2(&Event::Key(KeyEvent::new(
                KeyCode::Down,
                KeyModifiers::NONE,
            )));
        }
        let event = modal.handle_event_v2(&Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )));
        assert_eq!(event, AppEvent::Modal(ModalEvent::Close));
    }

    #[test]
    fn test_show_branch_create_dialog_submits_create_branch_event() {
        let processor = ModalProcessor;
        let mut state = mock_state();
        processor.process(
            ModalEvent::ShowBranchCreateDialog {
                from_branch: "main".to_string(),
            },
            &mut state,
        );

        let modal = state.active_modal.as_mut().expect("modal should exist");
        for ch in ['f', 'e', 'a', 't'] {
            let _ = modal.handle_event_v2(&Event::Key(KeyEvent::new(
                KeyCode::Char(ch),
                KeyModifiers::NONE,
            )));
        }

        let event = modal.handle_event_v2(&Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )));
        assert_eq!(
            event,
            AppEvent::Git(GitEvent::CreateBranch {
                new_name: "feat".to_string(),
                from_branch: "main".to_string(),
            })
        );
    }
}
