use crate::app::Command;
use crate::flux::action::{Action, ActionEnvelope, DomainAction};
use crate::flux::effects::EffectRequest;
use crate::flux::stores::{ReduceCtx, ReduceOutput, Store, UiInvalidation};

pub struct OverlayStore;

impl OverlayStore {
    pub fn new() -> Self {
        Self
    }
}

impl Store for OverlayStore {
    fn reduce(&mut self, action: &ActionEnvelope, ctx: &mut ReduceCtx<'_>) -> ReduceOutput {
        let Action::Domain(domain) = &action.action else {
            return ReduceOutput::none();
        };
        match domain {
            DomainAction::StartCommitInput => {
                return ReduceOutput::from_command(Command::Effect(
                    EffectRequest::StartCommitEditorGuarded,
                ));
            }
            DomainAction::StartCommandPalette => {
                ctx.app.start_command_palette();
                ctx.app
                    .push_log("command palette: type command and press Enter", true);
                return ReduceOutput::none().with_invalidation(UiInvalidation::log_and_overlay());
            }
            DomainAction::StartBranchCreateInput => {
                ctx.app.start_branch_create_input();
                ctx.app
                    .push_log("branch create: enter name and press Enter", true);
                return ReduceOutput::none().with_invalidation(UiInvalidation::all());
            }
            DomainAction::StartStashInput => {
                let targets = ctx.app.prepare_stash_targets_from_selection();
                if targets.is_empty() {
                    ctx.app.push_log("stash blocked: no selected items", false);
                    return ReduceOutput::none();
                }
                ctx.app.start_stash_editor(targets);
                ctx.app.push_log("stash: enter title and press Enter", true);
                return ReduceOutput::none().with_invalidation(UiInvalidation::all());
            }
            DomainAction::StartSearchInput => {
                ctx.app.start_search_input();
                ctx.app
                    .push_log("search: type query, Enter confirm, Esc cancel", true);
                return ReduceOutput::none().with_invalidation(UiInvalidation::log_and_overlay());
            }
            _ => return ReduceOutput::none(),
        }
    }
}
