// src/app/events.rs

use crate::app::ui_state::Panel;

/// Top-level event type returned by components
#[derive(Debug, Clone, PartialEq)]
pub enum AppEvent {
    /// Git operation event
    Git(GitEvent),
    /// Modal/dialog event
    Modal(ModalEvent),
    /// Switch active panel
    SwitchPanel(Panel),
    /// Activate current panel (Enter key behavior)
    ActivatePanel,
    /// Selection changed, refresh main view
    SelectionChanged,
    /// Event handled internally by component
    None,
}

/// Git operation events
#[derive(Debug, Clone, PartialEq)]
pub enum GitEvent {
    ToggleStageFile,
    StageAll,
    CommitWithMessage(String),
    DiscardSelected,
    StashSelected,
    AmendCommit,
    ExecuteReset(usize),
    IgnoreSelected,
    RenameFile(String),
}

/// Modal/dialog events
#[derive(Debug, Clone, PartialEq)]
pub enum ModalEvent {
    ShowHelp,
    ShowCommitDialog,
    ShowRenameDialog,
    ShowResetMenu,
    ShowDiscardConfirmation,
    ShowStashConfirmation,
    ShowAmendConfirmation,
    ShowResetConfirmation(usize),
    ShowNukeConfirmation,
    Close,
}
