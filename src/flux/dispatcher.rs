use crate::app::{AppState, Command};
use crate::flux::action::{Action, ActionEnvelope};
use crate::flux::command_output::CommandOutput;
use crate::flux::stores::{
    BranchStore, CommitStore, DiffStore, FilesStore, InputStore, NavigationStore, OpsStore,
    OverlayStore, QuitStore, ReduceCtx, RevisionStore, RuntimeStore, SearchStore, SelectionStore,
    StashStore, StateAccess, Store, UiInvalidation,
};

pub struct DispatchResult {
    pub commands: Vec<Command>,
    pub state_version: u64,
}

pub struct Dispatcher {
    next_sequence: u64,
    state_version: u64,
    last_sequence: Option<u64>,
    stores: Vec<Box<dyn Store<dyn StateAccess>>>,
    output: CommandOutput,
}

impl Dispatcher {
    #[cfg(test)]
    pub fn with_default_stores() -> Self {
        Self::with_initial_output(CommandOutput::default())
    }

    pub fn with_initial_output(output: CommandOutput) -> Self {
        Self {
            next_sequence: 1,
            state_version: 0,
            last_sequence: None,
            stores: vec![
                Box::new(RuntimeStore),
                Box::new(InputStore::new()),
                Box::new(QuitStore::new()),
                Box::new(OpsStore::new()),
                Box::new(RevisionStore::new()),
                Box::new(NavigationStore::new()),
                Box::new(SelectionStore::new()),
                Box::new(SearchStore::new()),
                Box::new(DiffStore::new()),
                Box::new(OverlayStore::new()),
                Box::new(FilesStore::new()),
                Box::new(BranchStore::new()),
                Box::new(StashStore::new()),
                Box::new(CommitStore::new()),
            ],
            output,
        }
    }

    pub fn next_envelope(&mut self, action: Action) -> ActionEnvelope {
        let sequence = self.next_sequence;
        self.next_sequence += 1;
        ActionEnvelope { sequence, action }
    }

    #[cfg(test)]
    fn with_stores(stores: Vec<Box<dyn Store<dyn StateAccess>>>) -> Self {
        Self {
            next_sequence: 1,
            state_version: 0,
            last_sequence: None,
            stores,
            output: CommandOutput::default(),
        }
    }

    pub fn dispatch(&mut self, app: &mut AppState, action: ActionEnvelope) -> DispatchResult {
        if let Some(last_sequence) = self.last_sequence {
            debug_assert!(
                action.sequence > last_sequence,
                "dispatch sequence must be monotonic: {} <= {}",
                action.sequence,
                last_sequence
            );
        }

        let mut ctx: ReduceCtx<'_, dyn StateAccess> = ReduceCtx {
            state: app,
            commands: &mut self.output,
        };
        let mut invalidation = UiInvalidation::none();
        for store in &mut self.stores {
            let output = store.reduce(&action, &mut ctx);
            invalidation.merge(output.invalidation);
        }
        invalidation.apply(ctx.state);
        let commands = self.output.drain();

        self.last_sequence = Some(action.sequence);
        self.state_version += 1;

        DispatchResult {
            commands,
            state_version: self.state_version,
        }
    }

    #[cfg(test)]
    pub fn take_initial_commands(&mut self) -> Vec<Command> {
        self.output.drain()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flux::action::DomainAction;
    use crate::flux::action::RuntimeEvent;
    use crate::flux::action::SystemAction;
    use crate::flux::effects::EffectRequest;
    use crate::flux::stores::state_access::{RevisionAccess, RuntimeAccess};
    use crate::flux::stores::{ReduceOutput, Store};
    use pretty_assertions::assert_eq;
    use std::cell::RefCell;
    use std::rc::Rc;

    struct ProbeStore {
        calls: Rc<RefCell<Vec<&'static str>>>,
        label: &'static str,
    }

    impl ProbeStore {
        fn new(calls: Rc<RefCell<Vec<&'static str>>>, label: &'static str) -> Self {
            Self { calls, label }
        }
    }

    impl<S: ?Sized> Store<S> for ProbeStore {
        fn reduce(&mut self, _action: &ActionEnvelope, ctx: &mut ReduceCtx<'_, S>) -> ReduceOutput {
            self.calls.borrow_mut().push(self.label);
            ctx.command(Command::Sync(DomainAction::SearchSetQuery(
                self.label.to_string(),
            )))
            .with_invalidation(UiInvalidation::all())
        }
    }

    #[test]
    fn test_dispatcher_broadcasts_to_following_stores_after_handled_output() {
        let calls: Rc<RefCell<Vec<&'static str>>> = Rc::new(RefCell::new(Vec::new()));
        let stores: Vec<Box<dyn Store<dyn StateAccess>>> = vec![
            Box::new(ProbeStore::new(calls.clone(), "first")),
            Box::new(ProbeStore::new(calls.clone(), "second")),
            Box::new(ProbeStore::new(calls.clone(), "third")),
        ];

        let mut dispatcher = Dispatcher::with_stores(stores);
        let mut app = crate::flux::stores::test_support::mock_app();
        app.ui.dirty.clear();
        let envelope =
            dispatcher.next_envelope(Action::System(crate::flux::action::SystemAction::Tick));
        let result = dispatcher.dispatch(&mut app, envelope);

        let got = calls.borrow().clone();
        assert_eq!(got, vec!["first", "second", "third"]);
        let commands: Vec<_> = result
            .commands
            .iter()
            .map(|command| match command {
                Command::Sync(DomainAction::SearchSetQuery(label)) => label.as_str(),
                _ => panic!("unexpected command"),
            })
            .collect();
        assert_eq!(commands, got);
        assert_eq!(result.state_version, 1);
        assert!(app.ui.dirty.main_content());
    }

    #[test]
    fn test_dispatcher_applies_quit_invalidation() {
        let mut dispatcher = Dispatcher::with_default_stores();
        let mut app = crate::flux::stores::test_support::mock_app();
        app.ui.dirty.clear();
        let envelope = dispatcher.next_envelope(Action::Domain(DomainAction::Quit));
        let result = dispatcher.dispatch(&mut app, envelope);
        assert!(!app.running);
        assert!(app.ui.dirty.main_content());
        assert!(result.commands.is_empty());
        assert_eq!(result.state_version, 1);
    }

    struct MixedStore(&'static str);

    impl<S: ?Sized> Store<S> for MixedStore {
        fn reduce(&mut self, action: &ActionEnvelope, ctx: &mut ReduceCtx<'_, S>) -> ReduceOutput {
            if !matches!(action.action, Action::System(SystemAction::Tick)) {
                return ReduceOutput::none();
            }
            // Coordinator output can be produced before a store emits its command.
            ctx.commands.effect(EffectRequest::StageFile(self.0.into()));
            ctx.command(Command::Sync(DomainAction::SearchSetQuery(self.0.into())))
                .with_invalidation(UiInvalidation::all())
        }
    }

    #[test]
    fn test_dispatcher_keeps_all_store_commands_before_coordinator_requests() {
        let mut app = crate::flux::stores::test_support::mock_app();
        let mut dispatcher = Dispatcher::with_stores(vec![
            Box::new(MixedStore("first")),
            Box::new(MixedStore("second")),
        ]);
        let envelope = dispatcher.next_envelope(Action::System(SystemAction::Tick));
        let result = dispatcher.dispatch(&mut app, envelope);
        assert!(matches!(result.commands.as_slice(), [
            Command::Sync(DomainAction::SearchSetQuery(first)),
            Command::Sync(DomainAction::SearchSetQuery(second)),
            Command::Effect(EffectRequest::StageFile(first_write)),
            Command::Effect(EffectRequest::StageFile(second_write)),
        ] if first == "first" && second == "second" && first_write.to_str() == Some("first") && second_write.to_str() == Some("second")));
        assert!(
            app.search.view().query.is_empty(),
            "emitting Sync must not dispatch recursively"
        );
        let envelope = dispatcher.next_envelope(Action::System(SystemAction::Resize {
            width: 80,
            height: 24,
        }));
        let result = dispatcher.dispatch(&mut app, envelope);
        assert!(
            result.commands.is_empty(),
            "output must not leak into the next action"
        );
        assert_eq!(result.state_version, 2);
    }

    struct OpenRevision;
    impl<S: RuntimeAccess + ?Sized> Store<S> for OpenRevision {
        fn reduce(&mut self, _: &ActionEnvelope, ctx: &mut ReduceCtx<'_, S>) -> ReduceOutput {
            ctx.state
                .apply_runtime_event(&RuntimeEvent::RevisionOpenTreeOrToggleDir, ctx.commands);
            ctx.command(Command::Effect(EffectRequest::StageFile(
                "store-first".into(),
            )))
        }
    }

    struct CloseRevision;
    impl<S: RevisionAccess + ?Sized> Store<S> for CloseRevision {
        fn reduce(&mut self, _: &ActionEnvelope, ctx: &mut ReduceCtx<'_, S>) -> ReduceOutput {
            ctx.state.commit_close_tree(ctx.commands);
            ctx.command(Command::Effect(EffectRequest::UnstageFile(
                "store-second".into(),
            )))
        }
    }

    #[test]
    fn test_later_store_cancels_unpublished_read_and_retains_other_output() {
        let mut app = crate::flux::stores::test_support::mock_app();
        app.ui.active_panel = crate::app::SidePanel::Commits;
        let mut dispatcher =
            Dispatcher::with_stores(vec![Box::new(OpenRevision), Box::new(CloseRevision)]);
        let envelope = dispatcher.next_envelope(Action::System(SystemAction::Tick));
        let result = dispatcher.dispatch(&mut app, envelope);
        assert!(matches!(
            result.commands.as_slice(),
            [
                Command::Effect(EffectRequest::StageFile(_)),
                Command::Effect(EffectRequest::UnstageFile(_)),
                Command::Effect(EffectRequest::LoadBackground {
                    request: crate::background::TaskRequest {
                        load: crate::app::BackgroundLoad::Diff { .. },
                        ..
                    }
                }),
            ]
        ));
        assert!(app
            .revisions
            .view()
            .commits
            .tree_mode
            .selected_source
            .is_none());
        assert!(!app.revisions.view().commits.tree_mode.active);
    }

    #[test]
    fn test_initial_output_follows_store_commands_and_is_published_once() {
        let mut app = crate::flux::stores::test_support::mock_app();
        let mut initial = CommandOutput::default();
        initial.effect(EffectRequest::StageFile("initial".into()));
        let mut dispatcher = Dispatcher::with_initial_output(initial);
        let envelope =
            dispatcher.next_envelope(Action::Domain(DomainAction::StageFile("store".into())));
        let result = dispatcher.dispatch(&mut app, envelope);
        assert!(matches!(result.commands.as_slice(), [
            Command::Effect(EffectRequest::StageFile(store)),
            Command::Effect(EffectRequest::StageFile(initial)),
        ] if store.to_str() == Some("store") && initial.to_str() == Some("initial")));
        let envelope = dispatcher.next_envelope(Action::System(SystemAction::Resize {
            width: 80,
            height: 24,
        }));
        assert!(dispatcher.dispatch(&mut app, envelope).commands.is_empty());
    }

    #[test]
    fn test_first_action_can_cancel_initial_unpublished_diff_request() {
        let mut app = crate::flux::stores::test_support::mock_app();
        app.ui.active_panel = crate::app::SidePanel::Commits;
        let mut initial = CommandOutput::default();
        app.state.reload_diff_now(&mut initial);
        assert!(app.diff.is_loading());
        let mut dispatcher = Dispatcher::with_initial_output(initial);
        let envelope = dispatcher.next_envelope(Action::Domain(DomainAction::StageFileFinished {
            path: "file".into(),
            result: Ok(()),
        }));
        let result = dispatcher.dispatch(&mut app, envelope);
        assert!(result.commands.is_empty());
        assert!(!app.diff.is_loading());
        assert!(
            app.state.has_pending_refresh_work(),
            "refresh intent must remain after cancelling its old diff read"
        );
    }

    #[test]
    fn test_tick_state_actions_keep_their_order_before_initial_io() {
        let mut app = crate::flux::stores::test_support::mock_app();
        app.ui.active_panel = crate::app::SidePanel::Commits;
        let mut initial = CommandOutput::default();
        app.state.reload_diff_now(&mut initial);
        app.diff
            .schedule_reload(std::time::Instant::now() - std::time::Duration::from_millis(200));
        let mut dispatcher = Dispatcher::with_initial_output(initial);
        let envelope = dispatcher.next_envelope(Action::System(SystemAction::Tick));
        let result = dispatcher.dispatch(&mut app, envelope);
        assert!(matches!(
            result.commands.as_slice(),
            [
                Command::Sync(DomainAction::Runtime(RuntimeEvent::ProcessBackgroundLoads)),
                Command::Sync(DomainAction::Runtime(RuntimeEvent::FlushPendingDiffReload)),
                Command::Effect(EffectRequest::LoadBackground {
                    request: crate::background::TaskRequest {
                        load: crate::app::BackgroundLoad::Diff { .. },
                        ..
                    }
                }),
            ]
        ));
        assert!(
            app.diff.has_pending_reload(),
            "emitted state actions must not reduce recursively"
        );
        assert_eq!(result.state_version, 1);
    }
}
