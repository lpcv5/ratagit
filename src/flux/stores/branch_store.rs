use crate::app::{Command, RefreshKind};
use crate::flux::action::{Action, ActionEnvelope, DomainAction};
use crate::flux::effects::EffectRequest;
use crate::flux::stores::{ReduceCtx, ReduceOutput, Store, UiInvalidation};

fn flush_refresh() -> Command {
    Command::Effect(EffectRequest::FlushPendingRefresh { log_success: false })
}

pub struct BranchStore;

impl BranchStore {
    pub fn new() -> Self {
        Self
    }
}

impl Store for BranchStore {
    fn reduce(&mut self, action: &ActionEnvelope, ctx: &mut ReduceCtx<'_>) -> ReduceOutput {
        let Action::Domain(domain) = &action.action else {
            return ReduceOutput::none();
        };
        match domain {
            DomainAction::CreateBranch(name) => ReduceOutput::from_command(Command::Effect(
                EffectRequest::CreateBranch(name.clone()),
            )),
            DomainAction::CreateBranchFinished { name, result } => {
                match result {
                    Ok(()) => {
                        ctx.app.push_log(format!("branch created: {}", name), true);
                        return ReduceOutput::none().with_invalidation(UiInvalidation::all());
                    }
                    Err(e) => ctx
                        .app
                        .push_log(format!("create branch failed: {}", e), false),
                }
                ReduceOutput::none()
            }
            DomainAction::CheckoutSelectedBranch => {
                ReduceOutput::from_command(Command::Effect(EffectRequest::CheckoutSelectedBranch))
            }
            DomainAction::BranchSwitchConfirm(auto_stash) => {
                let Some(target) = ctx.app.take_branch_switch_target() else {
                    ctx.app.cancel_input();
                    return ReduceOutput::none().with_invalidation(UiInvalidation::overlay());
                };
                ctx.app.cancel_input();
                if !auto_stash {
                    ctx.app
                        .push_log(format!("switch canceled: {}", target), false);
                    return ReduceOutput::none().with_invalidation(UiInvalidation::all());
                }

                ReduceOutput::from_command(Command::Effect(EffectRequest::CheckoutBranch {
                    name: target,
                    auto_stash: true,
                }))
            }
            DomainAction::CheckoutBranchFinished {
                name,
                auto_stash,
                result,
            } => {
                match result {
                    Ok(()) => {
                        if *auto_stash {
                            ctx.app
                                .push_log(format!("switched with auto stash: {}", name), true);
                        } else {
                            ctx.app.push_log(format!("switched to {}", name), true);
                        }
                        return ReduceOutput {
                            commands: vec![flush_refresh()],
                            invalidation: UiInvalidation::all(),
                        };
                    }
                    Err(e) => {
                        if *auto_stash {
                            ctx.app
                                .push_log(format!("auto-stash switch failed: {}", e), false);
                        } else {
                            ctx.app.push_log(format!("switch failed: {}", e), false);
                        }
                    }
                }
                ReduceOutput::none()
            }
            DomainAction::DeleteSelectedBranch => {
                if let Some(name) = ctx.app.selected_branch_name() {
                    return ReduceOutput::from_command(Command::Effect(
                        EffectRequest::DeleteBranch(name),
                    ));
                }
                ctx.app.push_log("no branch selected", false);
                ReduceOutput::none()
            }
            DomainAction::DeleteBranchFinished { name, result } => {
                match result {
                    Ok(()) => {
                        ctx.app.push_log(format!("deleted branch {}", name), true);
                        return ReduceOutput::none().with_invalidation(UiInvalidation::all());
                    }
                    Err(e) => ctx
                        .app
                        .push_log(format!("delete branch failed: {}", e), false),
                }
                ReduceOutput::none()
            }
            DomainAction::FetchRemote => {
                if ctx.app.branches.is_fetching_remote {
                    ctx.app.push_log("fetch already running", false);
                    return ReduceOutput::none();
                }
                ctx.app.branches.is_fetching_remote = true;
                ctx.app.push_log("fetch started", true);
                ReduceOutput::from_command(Command::Effect(EffectRequest::FetchRemote))
                    .with_invalidation(UiInvalidation::all())
            }
            DomainAction::FetchRemoteFinished(result) => {
                ctx.app.branches.is_fetching_remote = false;
                match result {
                    Ok(remote) => {
                        ctx.app.request_refresh(RefreshKind::Full);
                        ctx.app.push_log(format!("fetched {}", remote), true);
                        return ReduceOutput {
                            commands: vec![flush_refresh()],
                            invalidation: UiInvalidation::all(),
                        };
                    }
                    Err(e) => ctx.app.push_log(format!("fetch failed: {}", e), false),
                }
                ReduceOutput::none()
            }
            _ => ReduceOutput::none(),
        }
    }
}
