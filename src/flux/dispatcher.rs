use crate::app::{App, Command};
use crate::flux::action::{Action, ActionEnvelope};
use crate::flux::stores::{
    BranchStore, CommitStore, DiffStore, FilesStore, InputStore, NavigationStore, OpsStore,
    OverlayStore, QuitStore, ReduceCtx, RevisionStore, SearchStore, SelectionStore, StashStore,
    Store, UiInvalidation,
};
use crate::flux::task_manager::{TaskManager, TaskResult};

pub struct DispatchResult {
    pub commands: Vec<Command>,
    pub state_version: u64,
}

pub struct Dispatcher {
    next_sequence: u64,
    state_version: u64,
    last_sequence: Option<u64>,
    stores: Vec<Box<dyn Store>>,
}

impl Dispatcher {
    pub fn with_default_stores() -> Self {
        Self {
            next_sequence: 1,
            state_version: 0,
            last_sequence: None,
            stores: vec![
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
        }
    }

    pub fn next_envelope(&mut self, action: Action) -> ActionEnvelope {
        let sequence = self.next_sequence;
        self.next_sequence += 1;
        ActionEnvelope { sequence, action }
    }

    #[cfg(test)]
    fn with_stores(stores: Vec<Box<dyn Store>>) -> Self {
        Self {
            next_sequence: 1,
            state_version: 0,
            last_sequence: None,
            stores,
        }
    }

    pub fn dispatch(&mut self, app: &mut App, action: ActionEnvelope) -> DispatchResult {
        if let Some(last_sequence) = self.last_sequence {
            debug_assert!(
                action.sequence > last_sequence,
                "dispatch sequence must be monotonic: {} <= {}",
                action.sequence,
                last_sequence
            );
        }

        let mut ctx = ReduceCtx { app };
        let mut commands = Vec::new();
        let mut invalidation = UiInvalidation::none();
        for store in &mut self.stores {
            let mut output = store.reduce(&action, &mut ctx);
            commands.append(&mut output.commands);
            invalidation.merge(output.invalidation);
        }
        invalidation.apply(ctx.app);

        self.last_sequence = Some(action.sequence);
        self.state_version += 1;

        DispatchResult {
            commands,
            state_version: self.state_version,
        }
    }

    /// Bridge point for collecting task results from the async task manager.
    /// Integration to Action/Store pipelines can be introduced incrementally.
    #[allow(dead_code)]
    pub fn collect_task_results(&mut self, task_manager: &mut TaskManager) -> Vec<TaskResult> {
        task_manager.collect_ready()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    impl Store for ProbeStore {
        fn reduce(&mut self, _action: &ActionEnvelope, _ctx: &mut ReduceCtx<'_>) -> ReduceOutput {
            self.calls.borrow_mut().push(self.label);
            ReduceOutput::none()
        }
    }

    #[test]
    fn test_dispatcher_broadcasts_to_following_stores_after_handled_output() {
        let calls: Rc<RefCell<Vec<&'static str>>> = Rc::new(RefCell::new(Vec::new()));
        let stores: Vec<Box<dyn Store>> = vec![
            Box::new(ProbeStore::new(calls.clone(), "first")),
            Box::new(ProbeStore::new(calls.clone(), "second")),
            Box::new(ProbeStore::new(calls.clone(), "third")),
        ];

        let mut dispatcher = Dispatcher::with_stores(stores);
        let mut app = App::new().expect("app init");
        let envelope =
            dispatcher.next_envelope(Action::System(crate::flux::action::SystemAction::Tick));
        let _ = dispatcher.dispatch(&mut app, envelope);

        let got = calls.borrow().clone();
        assert_eq!(got, vec!["first", "second", "third"]);
    }
}
