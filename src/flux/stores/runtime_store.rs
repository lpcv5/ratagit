use super::{ReduceCtx, ReduceOutput, Store};
use crate::flux::action::{Action, ActionEnvelope, DomainAction, SystemAction};

pub struct RuntimeStore;

impl Store for RuntimeStore {
    fn reduce(&mut self, action: &ActionEnvelope, ctx: &mut ReduceCtx<'_>) -> ReduceOutput {
        if let Action::Domain(domain) = &action.action {
            ctx.state.apply_repository_action(domain);
        }
        match &action.action {
            Action::Domain(DomainAction::Runtime(event)) => ctx.state.apply_runtime_event(event),
            Action::System(SystemAction::BackgroundCompleted(completed)) => {
                ctx.state.apply_background_completed(completed)
            }
            Action::Domain(domain) => {
                use crate::app::RefreshKind;
                let refresh = match domain {
                    DomainAction::StageFileFinished { result: Ok(()), .. }
                    | DomainAction::StageHunksFinished { result: Ok(()), .. }
                    | DomainAction::UnstageFileFinished { result: Ok(()), .. }
                    | DomainAction::DiscardPathsFinished { result: Ok(()), .. }
                    | DomainAction::StagePathsFinished { .. }
                    | DomainAction::ToggleStageSelectionFinished { .. }
                    | DomainAction::PrepareCommitFromSelectionFinished { .. } => {
                        Some(RefreshKind::StatusOnly)
                    }
                    DomainAction::CreateBranchFinished { result: Ok(()), .. }
                    | DomainAction::DeleteBranchFinished { result: Ok(()), .. }
                    | DomainAction::StashPushFinished { result: Ok(_), .. }
                    | DomainAction::StashApplyFinished { result: Ok(()), .. }
                    | DomainAction::StashPopFinished { result: Ok(()), .. }
                    | DomainAction::StashDropFinished { result: Ok(()), .. } => {
                        Some(RefreshKind::StatusAndRefs)
                    }
                    DomainAction::CommitFinished { result: Ok(_), .. }
                    | DomainAction::CheckoutBranchFinished { result: Ok(()), .. } => {
                        Some(RefreshKind::Full)
                    }
                    _ => None,
                };
                if let Some(kind) = refresh {
                    ctx.state.request_refresh(kind);
                }
            }
            _ => {}
        }
        ReduceOutput::none()
    }
}
