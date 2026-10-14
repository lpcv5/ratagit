use crate::app::Command;
use crate::app::SidePanel;
use crate::flux::action::{Action, ActionEnvelope, DomainAction};
use crate::flux::effects::EffectRequest;
use crate::flux::stores::{ReduceCtx, ReduceOutput, Store, UiInvalidation};

use super::state_access::{NavigationAccess, PanelAccess, RefreshAccess, SearchAccess};

pub struct NavigationStore;

impl NavigationStore {
    pub fn new() -> Self {
        Self
    }

    fn recompute_commit_highlight<S: NavigationAccess + ?Sized>(ctx: &mut ReduceCtx<'_, S>) {
        ctx.state.recompute_commit_highlight();
    }

    /// Common post-processing after panel switch operations.
    fn after_panel_switch<S: SearchAccess + RefreshAccess + ?Sized>(
        ctx: &mut ReduceCtx<'_, S>,
    ) -> ReduceOutput {
        ctx.state.restore_search_for_active_scope();
        ctx.state.schedule_diff_reload();
        ReduceOutput::from_command(Command::Effect(
            EffectRequest::EnsureCommitsLoadedForActivePanel,
        ))
        .with_invalidation(UiInvalidation::all())
    }

    /// Common post-processing after list navigation operations.
    fn after_list_nav<S: NavigationAccess + RefreshAccess + ?Sized>(
        ctx: &mut ReduceCtx<'_, S>,
    ) -> ReduceOutput {
        Self::recompute_commit_highlight(ctx);
        ctx.state.schedule_diff_reload();
        ReduceOutput::none().with_invalidation(UiInvalidation::all())
    }

    /// Common post-processing after directory operations.
    fn after_dir_op<S: RefreshAccess + ?Sized>(ctx: &mut ReduceCtx<'_, S>) -> ReduceOutput {
        ctx.state.schedule_diff_reload();
        ReduceOutput::none().with_invalidation(UiInvalidation::all())
    }
}

impl<S: NavigationAccess + PanelAccess + RefreshAccess + SearchAccess + ?Sized> Store<S>
    for NavigationStore
{
    fn reduce(&mut self, action: &ActionEnvelope, ctx: &mut ReduceCtx<'_, S>) -> ReduceOutput {
        let Action::Domain(domain) = &action.action else {
            return ReduceOutput::none();
        };
        match domain {
            DomainAction::PanelNext => {
                let index = SidePanel::ALL
                    .iter()
                    .position(|panel| *panel == ctx.state.active_panel())
                    .unwrap();
                let next = SidePanel::ALL[(index + 1) % SidePanel::ALL.len()];
                ctx.state.set_active_panel(next);
                Self::after_panel_switch(ctx)
            }
            DomainAction::PanelPrev => {
                let index = SidePanel::ALL
                    .iter()
                    .position(|panel| *panel == ctx.state.active_panel())
                    .unwrap();
                let prev =
                    SidePanel::ALL[(index + SidePanel::ALL.len() - 1) % SidePanel::ALL.len()];
                ctx.state.set_active_panel(prev);
                Self::after_panel_switch(ctx)
            }
            DomainAction::PanelGoto(n) => {
                let panel = n
                    .checked_sub(1)
                    .and_then(|index| SidePanel::ALL.get(index))
                    .copied()
                    .unwrap_or(ctx.state.active_panel());
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
            DomainAction::ToggleDir => {
                ctx.state.toggle_selected_dir();
                Self::after_dir_op(ctx)
            }
            DomainAction::CollapseAll => {
                ctx.state.collapse_all();
                Self::after_dir_op(ctx)
            }
            DomainAction::ExpandAll => {
                ctx.state.expand_all();
                Self::after_dir_op(ctx)
            }
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
        let initial_scroll = app.diff.scroll();
        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::DiffScrollDown),
        );
        assert!(
            app.diff.scroll() > initial_scroll || output.invalidation != UiInvalidation::none()
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
}
