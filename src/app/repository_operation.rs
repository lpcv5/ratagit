use super::{AppState, RefreshKind};
use crate::flux::action::DomainAction;
use crate::flux::command_output::CommandOutput;
use crate::flux::effects::EffectRequest;

impl AppState {
    pub(crate) fn apply_repository_action(
        &mut self,
        action: &DomainAction,
        output: &mut CommandOutput,
    ) {
        self.apply_diff_action(action, output);
        match action {
            DomainAction::ControlRepositoryOperation(control) => {
                match self.git.operation.begin(*control) {
                    Ok(id) => output.effect(EffectRequest::ControlRepositoryOperation {
                        id,
                        control: *control,
                    }),
                    Err(error) => self.push_log(error, false),
                }
                self.ui.dirty.mark_all();
            }
            DomainAction::RepositoryOperationFinished {
                id,
                result,
                observed,
            } if self
                .git
                .operation
                .finish(*id, result.clone(), observed.clone()) =>
            {
                match result {
                    Ok(()) => self.push_log("Repository operation completed", true),
                    Err(error) => self.push_log(crate::app::git_error::message(error), false),
                }
                // Failed commands may still change the index, HEAD or conflict set.
                self.request_refresh(RefreshKind::Full, output);
                self.ui.dirty.mark_all();
            }
            _ => {}
        }
    }
}
