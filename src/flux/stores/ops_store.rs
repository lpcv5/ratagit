use crate::app::{Command, RefreshKind};
use crate::flux::action::{Action, ActionEnvelope, SystemAction};
use crate::flux::effects::EffectRequest;
use crate::flux::stores::{ReduceCtx, ReduceOutput, Store, UiInvalidation};

pub struct OpsStore;

impl OpsStore {
    pub fn new() -> Self {
        Self
    }
}

impl Store for OpsStore {
    fn reduce(&mut self, action: &ActionEnvelope, ctx: &mut ReduceCtx<'_>) -> ReduceOutput {
        match &action.action {
            Action::System(SystemAction::Tick) => {
                return ReduceOutput::from_command(Command::Effect(
                    EffectRequest::ProcessBackgroundLoads,
                ));
            }
            Action::System(SystemAction::AutoRefresh) => {}
            Action::System(SystemAction::Resize { .. }) => {
                return ReduceOutput::none().with_invalidation(UiInvalidation::all());
            }
            _ => return ReduceOutput::none(),
        }

        ctx.app.request_refresh(RefreshKind::Full);
        ReduceOutput::from_command(Command::Effect(EffectRequest::ProcessBackgroundLoads))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flux::action::{Action, SystemAction};
    use crate::flux::stores::test_support::{make_envelope, mock_app};
    use pretty_assertions::assert_eq;

    fn reduce(store: &mut OpsStore, app: &mut crate::app::App, action: Action) -> ReduceOutput {
        let env = make_envelope(action);
        let mut ctx = ReduceCtx { app };
        store.reduce(&env, &mut ctx)
    }

    #[test]
    fn test_tick_emits_flush_effect() {
        let mut store = OpsStore::new();
        let mut app = mock_app();
        let output = reduce(&mut store, &mut app, Action::System(SystemAction::Tick));
        assert!(!output.commands.is_empty());
    }

    #[test]
    fn test_auto_refresh_queues_full_refresh() {
        let mut store = OpsStore::new();
        let mut app = mock_app();
        let output = reduce(
            &mut store,
            &mut app,
            Action::System(SystemAction::AutoRefresh),
        );
        assert!(!output.commands.is_empty());
    }

    #[test]
    fn test_resize_invalidates_all() {
        let mut store = OpsStore::new();
        let mut app = mock_app();
        let output = reduce(
            &mut store,
            &mut app,
            Action::System(SystemAction::Resize {
                width: 100,
                height: 50,
            }),
        );
        assert_eq!(output.invalidation, UiInvalidation::all());
    }
}
