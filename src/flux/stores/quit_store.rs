use crate::flux::action::{Action, ActionEnvelope, DomainAction};
use crate::flux::stores::{ReduceCtx, ReduceOutput, Store};

pub struct QuitStore;

impl QuitStore {
    pub fn new() -> Self {
        Self
    }
}

impl Store for QuitStore {
    fn reduce(&mut self, action: &ActionEnvelope, ctx: &mut ReduceCtx<'_>) -> ReduceOutput {
        if !matches!(action.action, Action::Domain(DomainAction::Quit)) {
            return ReduceOutput::none();
        }
        ctx.state.set_running(false);
        ctx.state.mark_all_dirty();
        ReduceOutput::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flux::action::{Action, DomainAction};
    use crate::flux::stores::test_support::{make_envelope, mock_app};

    #[test]
    fn test_quit_sets_running_false() {
        let mut store = QuitStore::new();
        let mut app = mock_app();
        assert!(app.running);
        let env = make_envelope(Action::Domain(DomainAction::Quit));
        let mut ctx = ReduceCtx { state: &mut app };
        store.reduce(&env, &mut ctx);
        assert!(!app.running);
    }

    #[test]
    fn test_non_quit_action_does_nothing() {
        let mut store = QuitStore::new();
        let mut app = mock_app();
        let env = make_envelope(Action::Domain(DomainAction::PanelNext));
        let mut ctx = ReduceCtx { state: &mut app };
        store.reduce(&env, &mut ctx);
        assert!(app.running);
    }
}
