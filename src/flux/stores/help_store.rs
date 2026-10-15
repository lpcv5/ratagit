use super::state_access::HelpAccess;
use super::{ReduceCtx, ReduceOutput, Store, UiInvalidation};
use crate::app::Command;
use crate::flux::action::{Action, ActionEnvelope, DomainAction};

pub struct HelpStore;

impl<S: HelpAccess + ?Sized> Store<S> for HelpStore {
    fn reduce(&mut self, action: &ActionEnvelope, ctx: &mut ReduceCtx<'_, S>) -> ReduceOutput {
        let Action::Domain(DomainAction::Help(action)) = &action.action else {
            return ReduceOutput::none();
        };
        if let Some(command) = ctx.state.reduce_help(*action) {
            ctx.command(Command::Sync(command));
        }
        ReduceOutput::none().with_invalidation(UiInvalidation::overlay())
    }
}
