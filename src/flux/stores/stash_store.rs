use crate::app::Command;
use crate::flux::action::{Action, ActionEnvelope, DomainAction};
use crate::flux::effects::EffectRequest;
use crate::flux::stores::{ReduceCtx, ReduceOutput, Store, UiInvalidation};

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
                        return ReduceOutput::none().with_invalidation(UiInvalidation::all());
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
                        return ReduceOutput::none().with_invalidation(UiInvalidation::all());
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
                        return ReduceOutput::none().with_invalidation(UiInvalidation::all());
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
                        return ReduceOutput::none().with_invalidation(UiInvalidation::all());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flux::action::{Action, DomainAction};
    use crate::flux::stores::test_support::{make_envelope, mock_app};
    use std::path::PathBuf;

    fn reduce(store: &mut StashStore, app: &mut crate::app::App, action: Action) -> ReduceOutput {
        let env = make_envelope(action);
        let mut ctx = ReduceCtx { app };
        store.reduce(&env, &mut ctx)
    }

    #[test]
    fn test_stash_push_emits_effect() {
        let mut store = StashStore::new();
        let mut app = mock_app();
        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::StashPush {
                message: "wip".to_string(),
                paths: vec![PathBuf::from("foo.txt")],
            }),
        );
        assert!(!output.commands.is_empty());
    }

    #[test]
    fn test_stash_push_finished_ok_logs_success() {
        let mut store = StashStore::new();
        let mut app = mock_app();
        let logs_before = app.command_log.len();
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::StashPushFinished {
                message: "wip".to_string(),
                result: Ok(0),
            }),
        );
        assert!(app.command_log.len() > logs_before);
    }

    #[test]
    fn test_stash_apply_selected_emits_effect() {
        let mut store = StashStore::new();
        let mut app = mock_app();
        app.active_panel = crate::app::SidePanel::Stash;
        app.stash.items = vec![crate::git::StashInfo {
            index: 0,
            message: "wip".to_string(),
        }];
        app.stash.panel.list_state.select(Some(0));
        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::StashApplySelected),
        );
        assert!(!output.commands.is_empty());
    }

    #[test]
    fn test_stash_drop_finished_ok_logs_success() {
        let mut store = StashStore::new();
        let mut app = mock_app();
        let logs_before = app.command_log.len();
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::StashDropFinished {
                index: 0,
                result: Ok(()),
            }),
        );
        assert!(app.command_log.len() > logs_before);
    }

    #[test]
    fn test_unknown_action_does_nothing() {
        let mut store = StashStore::new();
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

    fn reduce(store: &mut StashStore, app: &mut crate::app::App, action: Action) -> ReduceOutput {
        let env = make_envelope(action);
        let mut ctx = ReduceCtx { app };
        store.reduce(&env, &mut ctx)
    }

    #[test]
    fn test_stash_pop_selected_emits_effect() {
        let mut store = StashStore::new();
        let mut app = mock_app();
        app.active_panel = crate::app::SidePanel::Stash;
        app.stash.items = vec![crate::git::StashInfo {
            index: 0,
            message: "wip".to_string(),
        }];
        app.stash.panel.list_state.select(Some(0));
        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::StashPopSelected),
        );
        assert!(!output.commands.is_empty());
    }

    #[test]
    fn test_stash_drop_selected_emits_effect() {
        let mut store = StashStore::new();
        let mut app = mock_app();
        app.active_panel = crate::app::SidePanel::Stash;
        app.stash.items = vec![crate::git::StashInfo {
            index: 0,
            message: "wip".to_string(),
        }];
        app.stash.panel.list_state.select(Some(0));
        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::StashDropSelected),
        );
        assert!(!output.commands.is_empty());
    }

    #[test]
    fn test_stash_pop_finished_ok_logs_success() {
        let mut store = StashStore::new();
        let mut app = mock_app();
        let logs_before = app.command_log.len();
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::StashPopFinished {
                index: 0,
                result: Ok(()),
            }),
        );
        assert!(app.command_log.len() > logs_before);
    }

    #[test]
    fn test_stash_apply_finished_ok_logs_success() {
        let mut store = StashStore::new();
        let mut app = mock_app();
        let logs_before = app.command_log.len();
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::StashApplyFinished {
                index: 0,
                result: Ok(()),
            }),
        );
        assert!(app.command_log.len() > logs_before);
    }

    #[test]
    fn test_stash_push_finished_err_logs_failure() {
        let mut store = StashStore::new();
        let mut app = mock_app();
        let logs_before = app.command_log.len();
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::StashPushFinished {
                message: "wip".to_string(),
                result: Err("push failed".to_string()),
            }),
        );
        assert!(app.command_log.len() > logs_before);
    }
}
