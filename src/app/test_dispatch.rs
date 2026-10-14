use super::{App, Command};
use crate::flux::action::{Action, DomainAction};

pub fn dispatch_test_action(app: &mut App, action: DomainAction) -> Option<Command> {
    crate::flux::test_runtime::drive_actions(app, [Action::Domain(action)])
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
