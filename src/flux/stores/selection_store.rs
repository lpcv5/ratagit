use crate::app::{Command, SidePanel};
use crate::flux::action::{Action, ActionEnvelope, DomainAction};
use crate::flux::effects::EffectRequest;
use crate::flux::stores::{ReduceCtx, ReduceOutput, Store};

pub struct SelectionStore;

impl SelectionStore {
    pub fn new() -> Self {
        Self
    }
}

impl Store for SelectionStore {
    fn reduce(&mut self, action: &ActionEnvelope, ctx: &mut ReduceCtx<'_>) -> ReduceOutput {
        let Action::Domain(domain) = &action.action else {
            return ReduceOutput::none();
        };

        match domain {
            DomainAction::ToggleVisualSelectMode
            | DomainAction::ToggleStageSelection
            | DomainAction::DiscardSelection
            | DomainAction::DiscardPathsFinished { .. }
            | DomainAction::PrepareCommitFromSelection => {}
            _ => return ReduceOutput::none(),
        }

        if ctx.app.active_panel != SidePanel::Files {
            return ReduceOutput::none();
        }

        match domain {
            DomainAction::ToggleVisualSelectMode => {
                ctx.app.toggle_visual_select_mode();
                ctx.app.dirty.mark();
                return ReduceOutput::from_command(Command::Effect(EffectRequest::ReloadDiffNow));
            }
            DomainAction::ToggleStageSelection => {
                return ReduceOutput::from_command(Command::Effect(
                    EffectRequest::ToggleStageSelection,
                ));
            }
            DomainAction::DiscardSelection => {
                let paths = ctx.app.prepare_discard_targets_from_selection();
                if paths.is_empty() {
                    ctx.app
                        .push_log("discard blocked: no discardable selected items", false);
                    return ReduceOutput::none();
                }
                return ReduceOutput::from_command(Command::Effect(EffectRequest::DiscardPaths(
                    paths,
                )));
            }
            DomainAction::DiscardPathsFinished { result, .. } => {
                if result.is_ok() {
                    ctx.app.files.visual_mode = false;
                    ctx.app.files.visual_anchor = None;
                    ctx.app.dirty.mark();
                }
            }
            DomainAction::PrepareCommitFromSelection => {
                return ReduceOutput::from_command(Command::Effect(
                    EffectRequest::PrepareCommitFromVisualSelection,
                ));
            }
            _ => return ReduceOutput::none(),
        }
        ReduceOutput::none()
    }
}
