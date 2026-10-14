use crate::flux::action::{Action, ActionEnvelope, DomainAction};
use crate::flux::stores::{ReduceCtx, ReduceOutput, Store, UiInvalidation};

use super::state_access::LifecycleAccess;

pub struct QuitStore;

impl QuitStore {
    pub fn new() -> Self {
        Self
    }
}

impl<S: LifecycleAccess + ?Sized> Store<S> for QuitStore {
    fn reduce(&mut self, action: &ActionEnvelope, ctx: &mut ReduceCtx<'_, S>) -> ReduceOutput {
        if !matches!(action.action, Action::Domain(DomainAction::Quit)) {
            return ReduceOutput::none();
        }
        ctx.state.set_running(false);
        ReduceOutput::none().with_invalidation(UiInvalidation::all())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flux::action::{Action, DomainAction};
    use crate::flux::stores::test_support::make_envelope;

    // Deliberately implements no other state capabilities.
    struct LifecycleState {
        running: bool,
    }

    impl LifecycleAccess for LifecycleState {
        fn set_running(&mut self, running: bool) {
            self.running = running;
        }
    }

    #[test]
    fn test_quit_sets_running_false() {
        let mut store = QuitStore::new();
        let mut state = LifecycleState { running: true };
        let env = make_envelope(Action::Domain(DomainAction::Quit));
        let mut ctx = ReduceCtx { state: &mut state };
        let output = store.reduce(&env, &mut ctx);
        assert!(!state.running);
        assert!(output.commands.is_empty());
        assert_eq!(output.invalidation, UiInvalidation::all());
    }

    #[test]
    fn test_non_quit_action_does_nothing() {
        let mut store = QuitStore::new();
        let mut state = LifecycleState { running: true };
        let env = make_envelope(Action::Domain(DomainAction::PanelNext));
        let mut ctx = ReduceCtx { state: &mut state };
        let output = store.reduce(&env, &mut ctx);
        assert!(state.running);
        assert!(output.commands.is_empty());
        assert_eq!(output.invalidation, UiInvalidation::none());
    }
}
