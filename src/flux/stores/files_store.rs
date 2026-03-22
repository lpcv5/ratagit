use crate::app::Command;
use crate::flux::action::{Action, ActionEnvelope, DomainAction};
use crate::flux::effects::EffectRequest;
use crate::flux::stores::{ReduceCtx, ReduceOutput, Store, UiInvalidation};

pub struct FilesStore;

impl FilesStore {
    pub fn new() -> Self {
        Self
    }
}

impl Store for FilesStore {
    fn reduce(&mut self, action: &ActionEnvelope, ctx: &mut ReduceCtx<'_>) -> ReduceOutput {
        let Action::Domain(domain) = &action.action else {
            return ReduceOutput::none();
        };
        match domain {
            DomainAction::StageFile(path) => {
                ReduceOutput::from_command(Command::Effect(EffectRequest::StageFile(path.clone())))
            }
            DomainAction::UnstageFile(path) => ReduceOutput::from_command(Command::Effect(
                EffectRequest::UnstageFile(path.clone()),
            )),
            DomainAction::DiscardPaths(paths) => {
                if paths.is_empty() {
                    ctx.app
                        .push_log("discard blocked: no discardable selected items", false);
                    return ReduceOutput::none();
                }
                ReduceOutput::from_command(Command::Effect(EffectRequest::DiscardPaths(
                    paths.clone(),
                )))
            }
            DomainAction::StageFileFinished { path, result } => {
                let display = path.display().to_string();
                match result {
                    Ok(()) => {
                        ctx.app.push_log(format!("staged {}", display), true);
                        return ReduceOutput::none().with_invalidation(UiInvalidation::all());
                    }
                    Err(err) => {
                        ctx.app
                            .push_log(format!("stage failed {}: {}", display, err), false);
                    }
                }
                ReduceOutput::none()
            }
            DomainAction::UnstageFileFinished { path, result } => {
                let display = path.display().to_string();
                match result {
                    Ok(()) => {
                        ctx.app.push_log(format!("unstaged {}", display), true);
                        return ReduceOutput::none().with_invalidation(UiInvalidation::all());
                    }
                    Err(err) => {
                        ctx.app
                            .push_log(format!("unstage failed {}: {}", display, err), false);
                    }
                }
                ReduceOutput::none()
            }
            DomainAction::DiscardPathsFinished { paths, result } => {
                match result {
                    Ok(()) => {
                        if paths.len() == 1 {
                            ctx.app
                                .push_log(format!("discarded {}", paths[0].display()), true);
                        } else {
                            ctx.app
                                .push_log(format!("discarded {} path(s)", paths.len()), true);
                        }
                        return ReduceOutput::none().with_invalidation(UiInvalidation::all());
                    }
                    Err(err) => {
                        ctx.app.push_log(format!("discard failed: {}", err), false);
                    }
                }
                ReduceOutput::none()
            }
            _ => ReduceOutput::none(),
        }
    }
}
