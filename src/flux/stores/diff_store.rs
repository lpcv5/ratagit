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
