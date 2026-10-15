use crate::app::Command;
use crate::app::SidePanel;
use crate::flux::action::RuntimeEvent;
use crate::flux::action::{Action, ActionEnvelope, DomainAction};
use crate::flux::stores::{ReduceCtx, ReduceOutput, Store, UiInvalidation};

use super::state_access::{PanelAccess, RevisionAccess, SearchAccess};

pub struct RevisionStore;

impl RevisionStore {
    pub fn new() -> Self {
        Self
    }
}

impl<S: PanelAccess + RevisionAccess + SearchAccess + ?Sized> Store<S> for RevisionStore {
    fn reduce(&mut self, action: &ActionEnvelope, ctx: &mut ReduceCtx<'_, S>) -> ReduceOutput {
        let Action::Domain(domain) = &action.action else {
            return ReduceOutput::none();
        };
        match domain {
            DomainAction::RevisionOpenTreeOrToggleDir => {
                ctx.command(Command::runtime(RuntimeEvent::RevisionOpenTreeOrToggleDir))
            }
            DomainAction::RevisionCloseTree => {
                match ctx.state.active_panel() {
                    SidePanel::Stash => ctx.state.stash_close_tree(ctx.commands),
                    SidePanel::Commits => ctx.state.commit_close_tree(ctx.commands),
                    SidePanel::LocalBranches => {
                        ctx.state.close_branch_commits_subview(ctx.commands)
                    }
                    _ => {}
                }
                ctx.state.restore_search_for_active_scope();
                ctx.command(Command::runtime(RuntimeEvent::ReloadDiffNow))
                    .with_invalidation(UiInvalidation::all())
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
    fn test_revision_open_tree_queues_state_action() {
        let mut store = RevisionStore::new();
        let mut app = mock_app();
        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::RevisionOpenTreeOrToggleDir),
        );
        assert!(matches!(
            output.commands.as_slice(),
            [Command::Sync(DomainAction::Runtime(
                RuntimeEvent::RevisionOpenTreeOrToggleDir
            ))]
        ));
    }

    #[test]
    fn test_revision_close_tree_queues_reload_action() {
        let mut store = RevisionStore::new();
        let mut app = mock_app();
        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::RevisionCloseTree),
        );
        assert!(matches!(
            output.commands.as_slice(),
            [Command::Sync(DomainAction::Runtime(
                RuntimeEvent::ReloadDiffNow
            ))]
        ));
    }

    #[test]
    fn test_unknown_action_does_nothing() {
        let mut store = RevisionStore::new();
        let mut app = mock_app();
        let output = reduce(&mut store, &mut app, Action::Domain(DomainAction::Quit));
        assert!(output.commands.is_empty());
    }
}
