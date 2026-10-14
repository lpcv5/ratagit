use super::state_access::LogAccess;
use super::{ReduceOutput, UiInvalidation};
use crate::app::files::{FileWrite, FilesUpdate};
use crate::app::Command;
use crate::flux::effects::EffectRequest;

/// Translate owned feature decisions at the Flux boundary.
pub(super) fn finish_files_update(
    state: &mut (impl LogAccess + ?Sized),
    update: FilesUpdate,
) -> ReduceOutput {
    let mut output = ReduceOutput::none();
    if let Some(write) = update.write {
        let effect = match write {
            FileWrite::Stage(path) => EffectRequest::StageFile(path),
            FileWrite::Unstage(path) => EffectRequest::UnstageFile(path),
            FileWrite::Discard(paths) => EffectRequest::DiscardPaths(paths),
            FileWrite::ToggleSelection { stage, unstage } => {
                EffectRequest::ToggleStageSelection { stage, unstage }
            }
            FileWrite::PrepareCommit(paths) => {
                EffectRequest::PrepareCommitFromVisualSelection { paths }
            }
        };
        output.commands.push(Command::Effect(effect));
    }
    if let Some((message, success)) = update.log {
        state.push_log(message, success);
    }
    if update.reload_diff {
        output
            .commands
            .push(Command::Effect(EffectRequest::ReloadDiffNow));
    }
    if update.redraw {
        output.invalidation = UiInvalidation::all();
    }
    output
}
