use crate::app::Command;
use crate::flux::action::{Action, ActionEnvelope, DomainAction};
use crate::flux::effects::EffectRequest;
use crate::flux::stores::{ReduceCtx, ReduceOutput, Store};

pub struct StashStore;

impl StashStore {
    pub fn new() -> Self {
        Self
    }
}

impl Store for StashStore {
    fn reduce(&mut self, action: &ActionEnvelope, ctx: &mut ReduceCtx<'_>) -> ReduceOutput {
        let Action::Domain(domain) = &action.action else {
            return ReduceOutput::none();
        };
        match domain {
            DomainAction::StashPush { message, paths } => {
                ReduceOutput::from_command(Command::Effect(EffectRequest::StashPush {
                    message: message.clone(),
                    paths: paths.clone(),
                }))
            }
            DomainAction::StashPushFinished { message, result } => {
                match result {
                    Ok(index) => {
                        ctx.app.push_log(
                            format!("stash created stash@{{{}}}: {}", index, message),
                            true,
                        );
                        ctx.app.dirty.mark();
                    }
                    Err(e) => ctx
                        .app
                        .push_log(format!("stash create failed: {}", e), false),
                }
                ReduceOutput::none()
            }
            DomainAction::StashApplySelected => {
                if let Some(index) = ctx.app.selected_stash_index() {
                    return ReduceOutput::from_command(Command::Effect(EffectRequest::StashApply(
                        index,
                    )));
                }
                ctx.app.push_log("no stash selected", false);
                ReduceOutput::none()
            }
            DomainAction::StashApplyFinished { index, result } => {
                match result {
                    Ok(()) => {
                        ctx.app
                            .push_log(format!("stash applied stash@{{{}}}", index), true);
                        ctx.app.dirty.mark();
                    }
                    Err(e) => ctx.app.push_log(
                        format!("stash apply failed stash@{{{}}}: {}", index, e),
                        false,
                    ),
                }
                ReduceOutput::none()
            }
            DomainAction::StashPopSelected => {
                if let Some(index) = ctx.app.selected_stash_index() {
                    return ReduceOutput::from_command(Command::Effect(EffectRequest::StashPop(
                        index,
                    )));
                }
                ctx.app.push_log("no stash selected", false);
                ReduceOutput::none()
            }
            DomainAction::StashPopFinished { index, result } => {
                match result {
                    Ok(()) => {
                        ctx.app
                            .push_log(format!("stash popped stash@{{{}}}", index), true);
                        ctx.app.dirty.mark();
                    }
                    Err(e) => ctx.app.push_log(
                        format!("stash pop failed stash@{{{}}}: {}", index, e),
                        false,
                    ),
                }
                ReduceOutput::none()
            }
            DomainAction::StashDropSelected => {
                if let Some(index) = ctx.app.selected_stash_index() {
                    return ReduceOutput::from_command(Command::Effect(EffectRequest::StashDrop(
                        index,
                    )));
                }
                ctx.app.push_log("no stash selected", false);
                ReduceOutput::none()
            }
            DomainAction::StashDropFinished { index, result } => {
                match result {
                    Ok(()) => {
                        ctx.app
                            .push_log(format!("stash dropped stash@{{{}}}", index), true);
                        ctx.app.dirty.mark();
                    }
                    Err(e) => ctx.app.push_log(
                        format!("stash drop failed stash@{{{}}}: {}", index, e),
                        false,
                    ),
                }
                ReduceOutput::none()
            }
            _ => ReduceOutput::none(),
        }
    }
}
