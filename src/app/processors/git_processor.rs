// src/app/processors/git_processor.rs
use crate::app::events::GitEvent;
use crate::app::state::AppState;
use crate::backend::BackendCommand;

pub struct GitProcessor;

impl GitProcessor {
    pub fn process(&self, event: GitEvent, _state: &AppState) -> Vec<BackendCommand> {
        match event {
            GitEvent::ToggleStageFile => vec![],
            GitEvent::StageAll => vec![],
            GitEvent::CommitWithMessage(_) => vec![],
            GitEvent::DiscardSelected => vec![],
            GitEvent::StashSelected => vec![],
            GitEvent::AmendCommit => vec![],
            GitEvent::ExecuteReset(_) => vec![],
            GitEvent::IgnoreSelected => vec![],
            GitEvent::RenameFile(_) => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::events::GitEvent;
    use crate::backend::BackendCommand;

    #[test]
    fn test_git_processor_toggle_stage() {
        // Stub test - just verify it compiles and returns a vec
        // Real logic will be added in Phase 4.1
        let processor = GitProcessor;
        // For now, we just test that the method exists and returns Vec<BackendCommand>
        // We'll add proper state mocking when implementing real logic
        let _processor = processor;
    }
}
