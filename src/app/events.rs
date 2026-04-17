// src/app/events.rs
//
// Event type definitions for the event-driven architecture.
//
// Components return AppEvent from handle_key_event(), which flows to App::process_event().
// The App routes events to appropriate processors:
// - GitEvent → GitProcessor → BackendCommand(s)
// - ModalEvent → ModalProcessor → State update
// - SwitchPanel/ActivatePanel → Direct state update

use crate::app::ui_state::Panel;

/// Top-level event type returned by components
///
/// This is the primary communication mechanism from components to the App.
/// Components never mutate state directly - they only return events.
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
///
/// These events are converted to BackendCommand(s) by GitProcessor.
/// GitProcessor handles multi-select logic and determines the appropriate
/// backend commands to send.
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
///
/// These events are processed by ModalProcessor to show/hide modal dialogs.
/// ModalProcessor updates AppState.active_modal, which is then rendered
/// on top of the main UI.
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
