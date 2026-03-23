use crate::app::Command;
use crate::flux::action::{Action, ActionEnvelope, SystemAction};
use crate::flux::effects::EffectRequest;
use crate::flux::stores::{ReduceCtx, ReduceOutput, Store};
use std::time::Duration;

pub struct DiffStore;

impl DiffStore {
    pub fn new() -> Self {
        Self
    }
}

impl Store for DiffStore {
    fn reduce(&mut self, action: &ActionEnvelope, ctx: &mut ReduceCtx<'_>) -> ReduceOutput {
        if !matches!(action.action, Action::System(SystemAction::Tick)) {
            return ReduceOutput::none();
        }

        const DIFF_RELOAD_DEBOUNCE: Duration = Duration::from_millis(80);
        if ctx.app.has_pending_diff_reload()
            && ctx.app.diff_reload_debounce_elapsed(DIFF_RELOAD_DEBOUNCE)
        {
            return ReduceOutput::from_command(Command::Effect(
                EffectRequest::FlushPendingDiffReload,
            ));
        }

        ReduceOutput::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flux::action::{Action, DomainAction, SystemAction};
    use crate::flux::stores::test_support::{mock_app, reduce_action as reduce};

    #[test]
    fn test_non_tick_does_nothing() {
        let mut store = DiffStore::new();
        let mut app = mock_app();
        let output = reduce(&mut store, &mut app, Action::Domain(DomainAction::Quit));
        assert!(output.commands.is_empty());
    }

    #[test]
    fn test_tick_without_pending_diff_does_nothing() {
        let mut store = DiffStore::new();
        let mut app = mock_app();
        // No pending diff reload
        let output = reduce(&mut store, &mut app, Action::System(SystemAction::Tick));
        assert!(output.commands.is_empty());
    }
}
