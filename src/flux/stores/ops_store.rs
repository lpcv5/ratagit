use crate::app::{Command, RefreshKind};
use crate::flux::action::{Action, ActionEnvelope, DomainAction, SystemAction};
use crate::flux::effects::EffectRequest;
use crate::flux::stores::{ReduceCtx, ReduceOutput, Store};

pub struct OpsStore;

impl OpsStore {
    pub fn new() -> Self {
        Self
    }
}

impl Store for OpsStore {
    fn reduce(&mut self, action: &ActionEnvelope, ctx: &mut ReduceCtx<'_>) -> ReduceOutput {
        match &action.action {
            Action::System(SystemAction::Tick) => {
                return ReduceOutput::from_command(Command::Effect(
                    EffectRequest::FlushPendingRefresh { log_success: false },
                ));
            }
            Action::System(SystemAction::Resize { .. }) => {
                ctx.app.dirty.mark_all();
                return ReduceOutput::none();
            }
            Action::Domain(DomainAction::RefreshStatus) => {}
            _ => return ReduceOutput::none(),
        }

        ctx.app.request_refresh(RefreshKind::Full);
        ReduceOutput::from_command(Command::Effect(EffectRequest::FlushPendingRefresh {
            log_success: true,
        }))
    }
}
