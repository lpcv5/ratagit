use crate::app::Command;
use crate::app::SidePanel;
use crate::flux::action::{Action, ActionEnvelope, DomainAction};
use crate::flux::effects::EffectRequest;
use crate::flux::stores::{ReduceCtx, ReduceOutput, Store};

pub struct RevisionStore;

impl RevisionStore {
    pub fn new() -> Self {
        Self
    }
}

impl Store for RevisionStore {
    fn reduce(&mut self, action: &ActionEnvelope, ctx: &mut ReduceCtx<'_>) -> ReduceOutput {
        let Action::Domain(domain) = &action.action else {
            return ReduceOutput::none();
        };
        match domain {
            DomainAction::RevisionOpenTreeOrToggleDir => ReduceOutput::from_command(
                Command::Effect(EffectRequest::RevisionOpenTreeOrToggleDir),
            ),
            DomainAction::RevisionCloseTree => {
                match ctx.app.active_panel {
                    SidePanel::Stash => ctx.app.stash_close_tree(),
                    SidePanel::Commits => ctx.app.commit_close_tree(),
                    _ => {}
                }
                ctx.app.restore_search_for_active_scope();
                ctx.app.dirty.mark();
                ReduceOutput::from_command(Command::Effect(EffectRequest::ReloadDiffNow))
            }
            _ => ReduceOutput::none(),
        }
    }
}
