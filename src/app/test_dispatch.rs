use super::{App, Command};
use crate::flux::action::{Action, DomainAction};
use crate::flux::dispatcher::Dispatcher;
use crate::flux::test_runtime::run_inline_effect;
use std::cell::RefCell;

thread_local! {
    static TEST_DISPATCHER: RefCell<Dispatcher> =
        RefCell::new(Dispatcher::with_default_stores());
}

pub fn dispatch_test_action(app: &mut App, action: DomainAction) -> Option<Command> {
    fn dispatch_action(app: &mut App, action: DomainAction) -> Option<Command> {
        TEST_DISPATCHER.with(|dispatcher| {
            let mut dispatcher = dispatcher.borrow_mut();
            let envelope = dispatcher.next_envelope(Action::Domain(action));
            let commands = dispatcher.dispatch(app, envelope).commands;
            drop(dispatcher);

            for command in commands {
                match command {
                    Command::Effect(request) => {
                        if let Some(follow_ups) = run_inline_effect(app, request.clone()) {
                            for follow_up in follow_ups {
                                if let Some(cmd) = dispatch_action(app, follow_up) {
                                    return Some(cmd);
                                }
                            }
                            continue;
                        }
                        return Some(Command::Effect(request));
                    }
                    Command::None => continue,
                    other => return Some(other),
                }
            }
            None
        })
    }

    dispatch_action(app, action)
}

pub fn map_test_key(app: &App, key: crossterm::event::KeyEvent) -> Option<DomainAction> {
    let snapshot = crate::flux::snapshot::AppStateSnapshot::from_app(app);
    let actions = crate::flux::input_mapper::map_key_to_actions(key, &snapshot);
    actions.into_iter().find_map(|action| match action {
        Action::Domain(domain) => Some(domain),
        Action::System(_) => None,
    })
}

pub fn dispatch_test_key(app: &mut App, key: crossterm::event::KeyEvent) -> Option<Command> {
    let snapshot = crate::flux::snapshot::AppStateSnapshot::from_app(app);
    let actions = crate::flux::input_mapper::map_key_to_actions(key, &snapshot);
    let mut last = None;
    for action in actions {
        if let Action::Domain(domain) = action {
            last = dispatch_test_action(app, domain);
        }
    }
    last
}
