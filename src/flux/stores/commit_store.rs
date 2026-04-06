use crate::app::Command;
use crate::flux::action::{Action, ActionEnvelope, DomainAction};
use crate::flux::effects::EffectRequest;
use crate::flux::stores::{tick_background_loads, ReduceCtx, ReduceOutput, Store, UiInvalidation};

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
                        ctx.state
                            .push_log(format!("commit {} ({})", message, oid), true);
                        return ReduceOutput {
                            commands: vec![tick_background_loads()],
                            invalidation: UiInvalidation::all(),
                        };
                    }
                    Err(e) => ctx.state.push_log(format!("commit failed: {}", e), false),
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
    use crate::flux::stores::test_support::{mock_app, reduce_action as reduce};

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
