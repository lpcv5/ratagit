use super::files_output::finish_files_update;
use super::state_access::{FilesAccess, LogAccess, OverlayAccess, RefreshAccess};
use crate::app::files::FilesAction;
use crate::app::RefreshKind;
#[cfg(test)]
use crate::app::SidePanel;
use crate::flux::action::{Action, ActionEnvelope, DomainAction};
use crate::flux::stores::{ReduceCtx, ReduceOutput, Store};

pub struct SelectionStore;

impl SelectionStore {
    pub fn new() -> Self {
        Self
    }
}

impl<S: FilesAccess + LogAccess + OverlayAccess + RefreshAccess + ?Sized> Store<S>
    for SelectionStore
{
    fn reduce(&mut self, action: &ActionEnvelope, ctx: &mut ReduceCtx<'_, S>) -> ReduceOutput {
        let Action::Domain(domain) = &action.action else {
            return ReduceOutput::none();
        };
        let action = match domain {
            DomainAction::ToggleVisualSelectMode => FilesAction::ToggleVisual,
            DomainAction::ToggleStageSelection => FilesAction::ToggleStageSelection,
            DomainAction::ToggleStageSelectionFinished { result } => {
                FilesAction::ToggleStageSelectionFinished(result)
            }
            DomainAction::DiscardSelection => FilesAction::DiscardSelection,
            DomainAction::DiscardPathsFinished { result, .. } => {
                FilesAction::DiscardSelectionFinished(result)
            }
            DomainAction::PrepareCommitFromSelection => FilesAction::PrepareCommitSelection,
            DomainAction::PrepareCommitFromSelectionFinished { result } => {
                FilesAction::PrepareCommitFinished(result)
            }
            _ => return ReduceOutput::none(),
        };
        let update = ctx.state.reduce_files(action);
        if update.refresh_status {
            ctx.state.request_refresh(RefreshKind::StatusOnly);
        }
        if update.open_commit_editor {
            ctx.state.start_commit_editor();
        }
        finish_files_update(ctx.state, update)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flux::action::{Action, DomainAction};
    use crate::flux::stores::test_support::{mock_app, reduce_action as reduce};

    #[test]
    fn test_toggle_visual_mode_in_files_panel() {
        let mut store = SelectionStore::new();
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        assert!(!app.files.view().visual_mode);
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::ToggleVisualSelectMode),
        );
        assert!(app.files.view().visual_mode);
    }

    #[test]
    fn test_toggle_visual_mode_ignored_in_non_files_panel() {
        let mut store = SelectionStore::new();
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::LocalBranches;
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::ToggleVisualSelectMode),
        );
        assert!(!app.files.view().visual_mode);
    }

    #[test]
    fn test_toggle_stage_selection_emits_effect() {
        let mut store = SelectionStore::new();
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::ToggleStageSelection),
        );
        assert!(!output.commands.is_empty());
    }

    #[test]
    fn test_discard_paths_finished_ok_clears_visual_mode() {
        let mut store = SelectionStore::new();
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        app.files.test_view_mut().visual_mode = true;
        app.files.test_view_mut().visual_anchor = Some(0);
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::DiscardPathsFinished {
                paths: vec![],
                result: Ok(()),
            }),
        );
        assert!(!app.files.view().visual_mode);
        assert!(app.files.view().visual_anchor.is_none());
    }

    #[test]
    fn test_prepare_commit_from_selection_emits_effect() {
        let mut store = SelectionStore::new();
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::PrepareCommitFromSelection),
        );
        assert!(!output.commands.is_empty());
    }
}
