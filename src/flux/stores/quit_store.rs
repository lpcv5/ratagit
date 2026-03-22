use crate::flux::action::{Action, ActionEnvelope, DomainAction};
use crate::flux::stores::{ReduceCtx, ReduceOutput, Store};

pub struct QuitStore;

impl QuitStore {
    pub fn new() -> Self {
        Self
    }
}

impl Store for QuitStore {
    fn reduce(&mut self, action: &ActionEnvelope, ctx: &mut ReduceCtx<'_>) -> ReduceOutput {
        if !matches!(action.action, Action::Domain(DomainAction::Quit)) {
            return ReduceOutput::none();
        }
        ctx.app.running = false;
        ReduceOutput::none()
    }
}
