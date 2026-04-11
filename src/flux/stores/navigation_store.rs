use crate::app::Command;
use crate::app::SidePanel;
use crate::flux::action::{Action, ActionEnvelope, DomainAction};
use crate::flux::branch_backend::BranchBackendCommand;
use crate::flux::effects::EffectRequest;
use crate::flux::files_backend::FilesBackendCommand;
use crate::flux::stores::{ReduceCtx, ReduceOutput, Store, UiInvalidation};

pub struct NavigationStore;

impl NavigationStore {
    pub fn new() -> Self {
        Self
    }

    fn recompute_commit_highlight(ctx: &mut ReduceCtx<'_>) {
        ctx.state.recompute_commit_highlight();
    }

    /// Common post-processing after panel switch operations.
    fn after_panel_switch(ctx: &mut ReduceCtx<'_>) -> ReduceOutput {
        ctx.state.restore_search_for_active_scope();
        let mut output = ReduceOutput::from_command(Command::Effect(
            EffectRequest::EnsureCommitsLoadedForActivePanel,
        ))
        .with_invalidation(UiInvalidation::all());
        if ctx.state.active_panel() == SidePanel::LocalBranches {
            let branch_name = ctx.state.selected_branch_name();
            output
                .commands
                .push(Command::Effect(EffectRequest::BranchesBackend(
                    BranchBackendCommand::LoadBranchGraph { branch_name },
                )));
        } else {
            ctx.state.schedule_diff_reload();
        }
        output
    }

    /// Common post-processing after list navigation operations.
    fn after_list_nav(ctx: &mut ReduceCtx<'_>) -> ReduceOutput {
        Self::recompute_commit_highlight(ctx);
        let mut output = ReduceOutput::none().with_invalidation(UiInvalidation::all());
        if ctx.state.active_panel() == SidePanel::LocalBranches {
            let branch_name = ctx.state.selected_branch_name();
            output
                .commands
                .push(Command::Effect(EffectRequest::BranchesBackend(
                    BranchBackendCommand::LoadBranchGraph { branch_name },
                )));
        } else {
            ctx.state.schedule_diff_reload();
        }
        output
    }
}

impl Store for NavigationStore {
    fn reduce(&mut self, action: &ActionEnvelope, ctx: &mut ReduceCtx<'_>) -> ReduceOutput {
        let Action::Domain(domain) = &action.action else {
            return ReduceOutput::none();
        };
        match domain {
            DomainAction::PanelNext => {
                let next = match ctx.state.active_panel() {
                    SidePanel::Files => SidePanel::LocalBranches,
                    SidePanel::LocalBranches => SidePanel::Commits,
                    SidePanel::Commits => SidePanel::Stash,
                    SidePanel::Stash => SidePanel::Files,
                };
                ctx.state.set_active_panel(next);
                Self::after_panel_switch(ctx)
            }
            DomainAction::PanelPrev => {
                let prev = match ctx.state.active_panel() {
                    SidePanel::Files => SidePanel::Stash,
                    SidePanel::LocalBranches => SidePanel::Files,
                    SidePanel::Commits => SidePanel::LocalBranches,
                    SidePanel::Stash => SidePanel::Commits,
                };
                ctx.state.set_active_panel(prev);
                Self::after_panel_switch(ctx)
            }
            DomainAction::PanelGoto(n) => {
                let panel = match n {
                    1 => SidePanel::Files,
                    2 => SidePanel::LocalBranches,
                    3 => SidePanel::Commits,
                    4 => SidePanel::Stash,
                    _ => ctx.state.active_panel(),
                };
                ctx.state.set_active_panel(panel);
                Self::after_panel_switch(ctx)
            }
            DomainAction::ListDown => {
                ctx.state.list_down();
                Self::after_list_nav(ctx)
            }
            DomainAction::ListUp => {
                ctx.state.list_up();
                Self::after_list_nav(ctx)
            }
            DomainAction::ToggleDir => ReduceOutput::from_command(Command::Effect(
                EffectRequest::FilesBackend(FilesBackendCommand::ToggleSelectedDir),
            ))
            .with_invalidation(UiInvalidation::all()),
            DomainAction::CollapseAll => ReduceOutput::from_command(Command::Effect(
                EffectRequest::FilesBackend(FilesBackendCommand::CollapseAll),
            ))
            .with_invalidation(UiInvalidation::all()),
            DomainAction::ExpandAll => ReduceOutput::from_command(Command::Effect(
                EffectRequest::FilesBackend(FilesBackendCommand::ExpandAll),
            ))
            .with_invalidation(UiInvalidation::all()),
            DomainAction::DiffScrollUp => {
                ctx.state.diff_scroll_up();
                ReduceOutput::none().with_invalidation(UiInvalidation::diff())
            }
            DomainAction::DiffScrollDown => {
                ctx.state.diff_scroll_down();
                ReduceOutput::none().with_invalidation(UiInvalidation::diff())
            }
            _ => ReduceOutput::none(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flux::action::{Action, DomainAction};
    use crate::flux::effects::EffectRequest;
    use crate::flux::files_backend::FilesBackendCommand;
    use crate::flux::stores::test_support::{mock_app, reduce_action as reduce};
    use pretty_assertions::assert_eq;

    #[test]
    fn test_panel_next_cycles_forward() {
        let mut store = NavigationStore::new();
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::PanelNext),
        );
        assert_eq!(app.ui.active_panel, SidePanel::LocalBranches);
    }

    #[test]
    fn test_panel_prev_cycles_backward() {
        let mut store = NavigationStore::new();
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::LocalBranches;
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::PanelPrev),
        );
        assert_eq!(app.ui.active_panel, SidePanel::Files);
    }

    #[test]
    fn test_panel_goto_jumps_to_correct_panel() {
        let mut store = NavigationStore::new();
        let mut app = mock_app();
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::PanelGoto(3)),
        );
        assert_eq!(app.ui.active_panel, SidePanel::Commits);
    }

    #[test]
    fn test_diff_scroll_down_invalidates_diff() {
        let mut store = NavigationStore::new();
        let mut app = mock_app();
        let initial_scroll = app.ui.diff_scroll;
        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::DiffScrollDown),
        );
        assert!(
            app.ui.diff_scroll > initial_scroll || output.invalidation != UiInvalidation::none()
        );
    }

    #[test]
    fn test_unknown_action_returns_none() {
        let mut store = NavigationStore::new();
        let mut app = mock_app();
        let output = reduce(&mut store, &mut app, Action::Domain(DomainAction::Quit));
        assert!(output.commands.is_empty());
        assert_eq!(output.invalidation, UiInvalidation::none());
    }

    #[test]
    fn test_toggle_dir_emits_files_backend_effect() {
        let mut store = NavigationStore::new();
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;

        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::ToggleDir),
        );

        assert!(matches!(
            output.commands.as_slice(),
            [Command::Effect(EffectRequest::FilesBackend(
                FilesBackendCommand::ToggleSelectedDir
            ))]
        ));
    }

    #[test]
    fn test_collapse_all_emits_files_backend_effect() {
        let mut store = NavigationStore::new();
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;

        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::CollapseAll),
        );

        assert!(matches!(
            output.commands.as_slice(),
            [Command::Effect(EffectRequest::FilesBackend(
                FilesBackendCommand::CollapseAll
            ))]
        ));
    }

    #[test]
    fn test_expand_all_emits_files_backend_effect() {
        let mut store = NavigationStore::new();
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;

        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::ExpandAll),
        );

        assert!(matches!(
            output.commands.as_slice(),
            [Command::Effect(EffectRequest::FilesBackend(
                FilesBackendCommand::ExpandAll
            ))]
        ));
    }
}
