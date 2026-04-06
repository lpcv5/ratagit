use crate::app::Command;
use crate::app::SidePanel;
use crate::flux::action::{Action, ActionEnvelope, DomainAction};
use crate::flux::effects::EffectRequest;
use crate::flux::stores::{ReduceCtx, ReduceOutput, Store, UiInvalidation};

pub struct RevisionStore;

impl RevisionStore {
    pub fn new() -> Self {
        Self
    }
}

impl Store for RevisionStore {
    fn reduce(&mut self, action: &ActionEnvelope, ctx: &mut ReduceCtx<'_>) -> ReduceOutput {
        let Action::Domain(domain) = &action.action else {
            return ReduceOutput::none();
        };
        match domain {
            DomainAction::RevisionOpenTreeOrToggleDir => ReduceOutput::from_command(
                Command::Effect(EffectRequest::RevisionOpenTreeOrToggleDir),
            ),
            DomainAction::RevisionCloseTree => {
                match ctx.state.active_panel() {
                    SidePanel::Stash => ctx.state.stash_close_tree(),
                    SidePanel::Commits => ctx.state.commit_close_tree(),
                    SidePanel::LocalBranches => ctx.state.close_branch_commits_subview(),
                    _ => {}
                }
                ctx.state.restore_search_for_active_scope();
                ReduceOutput::from_command(Command::Effect(EffectRequest::ReloadDiffNow))
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
    fn test_revision_open_tree_emits_effect() {
        let mut store = RevisionStore::new();
        let mut app = mock_app();
        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::RevisionOpenTreeOrToggleDir),
        );
        assert!(!output.commands.is_empty());
    }

    #[test]
    fn test_revision_close_tree_emits_reload_effect() {
        let mut store = RevisionStore::new();
        let mut app = mock_app();
        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::RevisionCloseTree),
        );
        assert!(!output.commands.is_empty());
    }

    #[test]
    fn test_unknown_action_does_nothing() {
        let mut store = RevisionStore::new();
        let mut app = mock_app();
        let output = reduce(&mut store, &mut app, Action::Domain(DomainAction::Quit));
        assert!(output.commands.is_empty());
    }
}
