use crate::app::{Command, SidePanel};
use crate::flux::action::{Action, ActionEnvelope, DomainAction};
use crate::flux::effects::EffectRequest;
use crate::flux::stores::{ReduceCtx, ReduceOutput, Store, UiInvalidation};

pub struct SelectionStore;

impl SelectionStore {
    pub fn new() -> Self {
        Self
    }
}

impl Store for SelectionStore {
    fn reduce(&mut self, action: &ActionEnvelope, ctx: &mut ReduceCtx<'_>) -> ReduceOutput {
        let Action::Domain(domain) = &action.action else {
            return ReduceOutput::none();
        };

        match domain {
            DomainAction::ToggleVisualSelectMode
            | DomainAction::ToggleStageSelection
            | DomainAction::DiscardSelection
            | DomainAction::DiscardPathsFinished { .. }
            | DomainAction::PrepareCommitFromSelection => {}
            _ => return ReduceOutput::none(),
        }

        if ctx.app.active_panel != SidePanel::Files {
            return ReduceOutput::none();
        }

        match domain {
            DomainAction::ToggleVisualSelectMode => {
                ctx.app.toggle_visual_select_mode();
                return ReduceOutput::from_command(Command::Effect(EffectRequest::ReloadDiffNow))
                    .with_invalidation(UiInvalidation::all());
            }
            DomainAction::ToggleStageSelection => {
                return ReduceOutput::from_command(Command::Effect(
                    EffectRequest::ToggleStageSelection,
                ));
            }
            DomainAction::DiscardSelection => {
                let paths = ctx.app.prepare_discard_targets_from_selection();
                if paths.is_empty() {
                    ctx.app
                        .push_log("discard blocked: no discardable selected items", false);
                    return ReduceOutput::none();
                }
                return ReduceOutput::from_command(Command::Effect(EffectRequest::DiscardPaths(
                    paths,
                )));
            }
            DomainAction::DiscardPathsFinished { result, .. } => {
                if result.is_ok() {
                    ctx.app.files.visual_mode = false;
                    ctx.app.files.visual_anchor = None;
                    return ReduceOutput::none().with_invalidation(UiInvalidation::all());
                }
            }
            DomainAction::PrepareCommitFromSelection => {
                return ReduceOutput::from_command(Command::Effect(
                    EffectRequest::PrepareCommitFromVisualSelection,
                ));
            }
            _ => return ReduceOutput::none(),
        }
        ReduceOutput::none()
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
        app.active_panel = SidePanel::Files;
        assert!(!app.files.visual_mode);
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::ToggleVisualSelectMode),
        );
        assert!(app.files.visual_mode);
    }

    #[test]
    fn test_toggle_visual_mode_ignored_in_non_files_panel() {
        let mut store = SelectionStore::new();
        let mut app = mock_app();
        app.active_panel = SidePanel::LocalBranches;
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::ToggleVisualSelectMode),
        );
        assert!(!app.files.visual_mode);
    }

    #[test]
    fn test_toggle_stage_selection_emits_effect() {
        let mut store = SelectionStore::new();
        let mut app = mock_app();
        app.active_panel = SidePanel::Files;
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
        app.active_panel = SidePanel::Files;
        app.files.visual_mode = true;
        app.files.visual_anchor = Some(0);
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::DiscardPathsFinished {
                paths: vec![],
                result: Ok(()),
            }),
        );
        assert!(!app.files.visual_mode);
        assert!(app.files.visual_anchor.is_none());
    }

    #[test]
    fn test_prepare_commit_from_selection_emits_effect() {
        let mut store = SelectionStore::new();
        let mut app = mock_app();
        app.active_panel = SidePanel::Files;
        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::PrepareCommitFromSelection),
        );
        assert!(!output.commands.is_empty());
    }
}
