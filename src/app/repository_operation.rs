use super::{AppState, RefreshKind};
use crate::flux::action::DomainAction;
use crate::flux::effects::EffectRequest;

impl AppState {
    pub(crate) fn apply_repository_action(&mut self, action: &DomainAction) {
        self.apply_diff_action(action);
        match action {
            DomainAction::StageHunks { patch, file, hunks } => {
                match patch.validate_hunks(*file, hunks) {
                    Ok(_) => self.effects.push(EffectRequest::StageHunks {
                        patch: patch.clone(),
                        file: *file,
                        hunks: hunks.clone(),
                    }),
                    Err(error) => self.push_log(error.to_string(), false),
                }
            }
            DomainAction::StageHunksFinished { result } => match result {
                Ok(()) => self.push_log("Selected hunks staged", true),
                Err(error) => self.push_log(error.clone(), false),
            },
            DomainAction::ControlRepositoryOperation(control) => {
                match self.git.operation.begin(*control) {
                    Ok(id) => self
                        .effects
                        .push(EffectRequest::ControlRepositoryOperation {
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
                    Err(error) => self.push_log(error.clone(), false),
                }
                // Failed commands may still change the index, HEAD or conflict set.
                self.request_refresh(RefreshKind::Full);
                self.ui.dirty.mark_all();
            }
            _ => {}
        }
    }
}
