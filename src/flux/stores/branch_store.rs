use crate::app::{Command, RefreshKind};
use crate::flux::action::{Action, ActionEnvelope, DomainAction};
use crate::flux::effects::EffectRequest;
use crate::flux::stores::{flush_refresh, ReduceCtx, ReduceOutput, Store, UiInvalidation};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flux::action::{Action, DomainAction};
    use crate::flux::stores::test_support::{make_envelope, mock_app};

    fn reduce(store: &mut BranchStore, app: &mut crate::app::App, action: Action) -> ReduceOutput {
        let env = make_envelope(action);
        let mut ctx = ReduceCtx { app };
        store.reduce(&env, &mut ctx)
    }

    #[test]
    fn test_create_branch_emits_effect() {
        let mut store = BranchStore::new();
        let mut app = mock_app();
        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::CreateBranch("feature".to_string())),
        );
        assert!(!output.commands.is_empty());
    }

    #[test]
    fn test_create_branch_finished_ok_logs_success() {
        let mut store = BranchStore::new();
        let mut app = mock_app();
        let logs_before = app.command_log.len();
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::CreateBranchFinished {
                name: "feature".to_string(),
                result: Ok(()),
            }),
        );
        assert!(app.command_log.len() > logs_before);
    }

    #[test]
    fn test_checkout_selected_branch_emits_effect() {
        let mut store = BranchStore::new();
        let mut app = mock_app();
        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::CheckoutSelectedBranch),
        );
        assert!(!output.commands.is_empty());
    }

    #[test]
    fn test_delete_selected_branch_emits_effect() {
        let mut store = BranchStore::new();
        let mut app = mock_app();
        app.active_panel = crate::app::SidePanel::LocalBranches;
        app.branches.items = vec![crate::git::BranchInfo {
            name: "feature".to_string(),
            is_current: false,
        }];
        app.branches.panel.list_state.select(Some(0));
        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::DeleteSelectedBranch),
        );
        assert!(!output.commands.is_empty());
    }

    #[test]
    fn test_fetch_remote_emits_effect() {
        let mut store = BranchStore::new();
        let mut app = mock_app();
        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::FetchRemote),
        );
        assert!(!output.commands.is_empty());
    }

    #[test]
    fn test_fetch_remote_finished_ok_clears_fetching_flag() {
        let mut store = BranchStore::new();
        let mut app = mock_app();
        app.branches.is_fetching_remote = true;
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::FetchRemoteFinished(Ok("origin".to_string()))),
        );
        assert!(!app.branches.is_fetching_remote);
    }

    #[test]
    fn test_unknown_action_does_nothing() {
        let mut store = BranchStore::new();
        let mut app = mock_app();
        let output = reduce(&mut store, &mut app, Action::Domain(DomainAction::Quit));
        assert!(output.commands.is_empty());
    }
}

#[cfg(test)]
mod more_tests {
    use super::*;
    use crate::flux::action::{Action, DomainAction};
    use crate::flux::stores::test_support::{make_envelope, mock_app};

    fn reduce(store: &mut BranchStore, app: &mut crate::app::App, action: Action) -> ReduceOutput {
        let env = make_envelope(action);
        let mut ctx = ReduceCtx { app };
        store.reduce(&env, &mut ctx)
    }

    #[test]
    fn test_checkout_branch_finished_ok_logs_success() {
        let mut store = BranchStore::new();
        let mut app = mock_app();
        let logs_before = app.command_log.len();
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::CheckoutBranchFinished {
                name: "feature".to_string(),
                auto_stash: false,
                result: Ok(()),
            }),
        );
        assert!(app.command_log.len() > logs_before);
    }

    #[test]
    fn test_checkout_branch_finished_err_logs_failure() {
        let mut store = BranchStore::new();
        let mut app = mock_app();
        let logs_before = app.command_log.len();
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::CheckoutBranchFinished {
                name: "feature".to_string(),
                auto_stash: false,
                result: Err("checkout failed".to_string()),
            }),
        );
        assert!(app.command_log.len() > logs_before);
    }

    #[test]
    fn test_delete_branch_finished_ok_logs_success() {
        let mut store = BranchStore::new();
        let mut app = mock_app();
        let logs_before = app.command_log.len();
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::DeleteBranchFinished {
                name: "feature".to_string(),
                result: Ok(()),
            }),
        );
        assert!(app.command_log.len() > logs_before);
    }

    #[test]
    fn test_branch_switch_confirm_false_cancels() {
        let mut store = BranchStore::new();
        let mut app = mock_app();
        app.branch_switch_target = Some("feature".to_string());
        app.input_mode = Some(crate::app::InputMode::BranchSwitchConfirm);
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::BranchSwitchConfirm(false)),
        );
        assert!(app.branch_switch_target.is_none());
        assert!(app.input_mode.is_none());
    }

    #[test]
    fn test_fetch_remote_finished_err_logs_failure() {
        let mut store = BranchStore::new();
        let mut app = mock_app();
        app.branches.is_fetching_remote = true;
        let logs_before = app.command_log.len();
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::FetchRemoteFinished(Err(
                "fetch failed".to_string()
            ))),
        );
        assert!(!app.branches.is_fetching_remote);
        assert!(app.command_log.len() > logs_before);
    }
}
