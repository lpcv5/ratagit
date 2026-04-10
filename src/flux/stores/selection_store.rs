use crate::app::{Command, RefreshKind, SidePanel};
use crate::flux::action::{Action, ActionEnvelope, DomainAction};
use crate::flux::effects::EffectRequest;
use crate::flux::files_backend::FilesBackendCommand;
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
            | DomainAction::ToggleStageSelectionFinished { .. }
            | DomainAction::DiscardSelection
            | DomainAction::DiscardPathsFinished { .. }
            | DomainAction::PrepareCommitFromSelection
            | DomainAction::PrepareCommitFromSelectionFinished { .. } => {}
            _ => return ReduceOutput::none(),
        }

        if ctx.state.active_panel() != SidePanel::Files {
            return ReduceOutput::none();
        }

        match domain {
            DomainAction::ToggleVisualSelectMode => {
                ctx.state.toggle_visual_select_mode();
                return ReduceOutput::from_command(Command::Effect(EffectRequest::FilesBackend(
                    FilesBackendCommand::ReloadDiff,
                )))
                .with_invalidation(UiInvalidation::all());
            }
            DomainAction::ToggleStageSelection => {
                return ReduceOutput::from_command(Command::Effect(
                    EffectRequest::ToggleStageSelection,
                ));
            }
            DomainAction::ToggleStageSelectionFinished { result } => match result {
                Ok((staged, unstaged)) => {
                    ctx.state.push_log(
                        format!(
                            "selection toggled: staged {}, unstaged {}",
                            staged, unstaged
                        ),
                        true,
                    );
                    ctx.state.request_refresh(RefreshKind::StatusOnly);
                    return ReduceOutput::none().with_invalidation(UiInvalidation::all());
                }
                Err(e) => {
                    ctx.state
                        .push_log(format!("selection toggle failed: {}", e), false);
                    return ReduceOutput::none();
                }
            },
            DomainAction::DiscardSelection => {
                let paths = ctx.state.prepare_discard_targets_from_selection();
                if paths.is_empty() {
                    ctx.state.push_log(
                        "discard blocked: no discardable selected items".to_string(),
                        false,
                    );
                    return ReduceOutput::none();
                }
                return ReduceOutput::from_command(Command::Effect(EffectRequest::FilesBackend(
                    FilesBackendCommand::DiscardPaths(paths),
                )));
            }
            DomainAction::DiscardPathsFinished { result, .. } => {
                if result.is_ok() {
                    ctx.state.clear_files_visual_selection();
                    return ReduceOutput::none().with_invalidation(UiInvalidation::all());
                }
            }
            DomainAction::PrepareCommitFromSelection => {
                return ReduceOutput::from_command(Command::Effect(
                    EffectRequest::PrepareCommitFromVisualSelection,
                ));
            }
            DomainAction::PrepareCommitFromSelectionFinished { result } => match result {
                Ok(count) if *count == 0 => {
                    ctx.state
                        .push_log("commit blocked: no selected items".to_string(), false);
                    return ReduceOutput::none();
                }
                Ok(count) => {
                    if ctx.state.start_commit_editor_guarded() {
                        ctx.state.push_log(
                            format!(
                                "commit: {} selected target(s) staged; edit message/description",
                                count
                            ),
                            true,
                        );
                        return ReduceOutput::none().with_invalidation(UiInvalidation::all());
                    }
                    return ReduceOutput::none();
                }
                Err(e) => {
                    ctx.state
                        .push_log(format!("prepare commit failed: {}", e), false);
                    return ReduceOutput::none();
                }
            },
            _ => return ReduceOutput::none(),
        }
        ReduceOutput::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flux::action::{Action, DomainAction};
    use crate::flux::effects::EffectRequest;
    use crate::flux::files_backend::FilesBackendCommand;
    use crate::flux::stores::test_support::{mock_app, reduce_action as reduce};

    #[test]
    fn test_toggle_visual_mode_in_files_panel() {
        let mut store = SelectionStore::new();
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        assert!(!app.ui.files.visual_mode);
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::ToggleVisualSelectMode),
        );
        assert!(app.ui.files.visual_mode);
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
        assert!(!app.ui.files.visual_mode);
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
        app.ui.files.visual_mode = true;
        app.ui.files.visual_anchor = Some(0);
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::DiscardPathsFinished {
                paths: vec![],
                result: Ok(()),
            }),
        );
        assert!(!app.ui.files.visual_mode);
        assert!(app.ui.files.visual_anchor.is_none());
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

    #[test]
    fn test_toggle_visual_mode_emits_files_backend_reload_diff() {
        let mut store = SelectionStore::new();
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;

        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::ToggleVisualSelectMode),
        );

        assert!(matches!(
            output.commands.as_slice(),
            [Command::Effect(EffectRequest::FilesBackend(
                FilesBackendCommand::ReloadDiff
            ))]
        ));
    }
}
