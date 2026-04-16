// src/app/processors/modal_processor.rs
use crate::app::events::ModalEvent;
use crate::app::state::AppState;

pub struct ModalProcessor;

impl ModalProcessor {
    pub fn process(&self, event: ModalEvent, _state: &mut AppState) {
        match event {
            ModalEvent::ShowHelp => {},
            ModalEvent::ShowCommitDialog => {},
            ModalEvent::ShowRenameDialog => {},
            ModalEvent::ShowResetMenu => {},
            ModalEvent::ShowDiscardConfirmation => {},
            ModalEvent::ShowStashConfirmation => {},
            ModalEvent::ShowAmendConfirmation => {},
            ModalEvent::ShowResetConfirmation(_) => {},
            ModalEvent::ShowNukeConfirmation => {},
            ModalEvent::Close => {},
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::events::ModalEvent;

    #[test]
    fn test_modal_processor_show_help() {
        let processor = ModalProcessor;
        processor.process(ModalEvent::ShowHelp, &mut mock_state());
    }

    fn mock_state() -> crate::app::state::AppState {
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(4);
        let (_event_tx, event_rx) = tokio::sync::mpsc::channel(4);
        crate::app::state::AppState::new(cmd_tx, event_rx)
    }
}
