use crate::app::Command;
use crate::flux::action::{Action, ActionEnvelope, DomainAction};
use crate::flux::effects::EffectRequest;
use crate::flux::stores::{flush_refresh, ReduceCtx, ReduceOutput, Store, UiInvalidation};

pub struct CommitStore;

impl CommitStore {
    pub fn new() -> Self {
        Self
    }
}

impl Store for CommitStore {
    fn reduce(&mut self, action: &ActionEnvelope, ctx: &mut ReduceCtx<'_>) -> ReduceOutput {
        let Action::Domain(domain) = &action.action else {
            return ReduceOutput::none();
        };
        match domain {
            DomainAction::Commit(commit_message) => ReduceOutput::from_command(Command::Effect(
                EffectRequest::Commit(commit_message.clone()),
            )),
            DomainAction::CommitFinished { message, result } => {
                match result {
                    Ok(oid) => {
                        ctx.app
                            .push_log(format!("commit {} ({})", message, oid), true);
                        return ReduceOutput {
                            commands: vec![flush_refresh()],
                            invalidation: UiInvalidation::all(),
                        };
                    }
                    Err(e) => ctx.app.push_log(format!("commit failed: {}", e), false),
                }
                ReduceOutput::none()
            }
            _ => ReduceOutput::none(),
        }
    }
}
