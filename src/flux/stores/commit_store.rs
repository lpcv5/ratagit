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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flux::action::{Action, DomainAction};
    use crate::flux::stores::test_support::{make_envelope, mock_app};

    fn reduce(store: &mut CommitStore, app: &mut crate::app::App, action: Action) -> ReduceOutput {
        let env = make_envelope(action);
        let mut ctx = ReduceCtx { app };
        store.reduce(&env, &mut ctx)
    }

    #[test]
    fn test_commit_emits_effect() {
        let mut store = CommitStore::new();
        let mut app = mock_app();
        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::Commit("fix: test".to_string())),
        );
        assert!(!output.commands.is_empty());
    }

    #[test]
    fn test_commit_finished_ok_logs_success() {
        let mut store = CommitStore::new();
        let mut app = mock_app();
        let logs_before = app.command_log.len();
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::CommitFinished {
                message: "fix: test".to_string(),
                result: Ok("abc1234".to_string()),
            }),
        );
        assert!(app.command_log.len() > logs_before);
    }

    #[test]
    fn test_commit_finished_err_logs_failure() {
        let mut store = CommitStore::new();
        let mut app = mock_app();
        let logs_before = app.command_log.len();
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::CommitFinished {
                message: "fix: test".to_string(),
                result: Err("commit failed".to_string()),
            }),
        );
        assert!(app.command_log.len() > logs_before);
    }
}
